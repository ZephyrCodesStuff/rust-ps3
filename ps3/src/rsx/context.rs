use core::ptr::{addr_of_mut, NonNull};

use crate::sys::raw::PpuFunctionDescriptor;
use crate::sys::sys_timer_usleep;
use crate::sys::syscalls::call_compact;

use super::{raw, Config, ContextData, Control};

pub(super) const IO_ALIGN: usize = 1024 * 1024;
const MAX_METHOD_WORDS: usize = 0x7ff;
const NON_INCREMENTING: u32 = 1 << 30;
const RETURN_COMMAND: u32 = 0x0002_0000;

// Internally, GCM retains the address of the slot passed to init_body.
static mut DEFAULT_CONTEXT: u32 = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Firmware(i32),
    InvalidArgument,
    InvalidContext,
    CommandBufferFull,
    OutOfMemory,
    Timeout,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Firmware(status) => write!(f, "GCM returned {:#010x}", *status as u32),
            Self::InvalidArgument => f.write_str("invalid RSX argument"),
            Self::InvalidContext => f.write_str("invalid RSX context or display state"),
            Self::CommandBufferFull => f.write_str("RSX command buffer is full"),
            Self::OutOfMemory => f.write_str("RSX allocation does not fit in the memory arena"),
            Self::Timeout => f.write_str("timed out waiting for the RSX"),
        }
    }
}

impl core::error::Error for Error {}

impl Error {
    pub(crate) fn status(status: i32) -> Result<(), Self> {
        if status == 0 {
            Ok(())
        } else {
            Err(Self::Firmware(status))
        }
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlipMode {
    HSync = 1,
    VSync = 2,
    HSyncWithNoise = 3,
}

/// GCM's pre-initialization command FIFO policy.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DefaultFifoMode {
    Traditional = 0,
    Optimize = 1,
    Conditional = 2,
}

/// GCM's pre-initialization system memory mapping policy.
#[repr(u64)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SystemMode {
    /// Use the firmware's default I/O mapping size.
    Default = 0,
    /// Enable the SDK's 512 MiB I/O map mode.
    IoMap512Mb = 1,
}

/// An aligned RSX method address including its subchannel bits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Method(u16);

impl Method {
    pub const fn new(address: u16) -> Option<Self> {
        if address & 3 == 0 {
            Some(Self(address))
        } else {
            None
        }
    }

    pub const fn address(self) -> u16 {
        self.0
    }

    /// Advances within a method family while preserving its subchannel.
    pub const fn with_offset(self, bytes: u16) -> Option<Self> {
        let Some(address) = self.0.checked_add(bytes) else {
            return None;
        };
        if address & 0xe000 != self.0 & 0xe000 {
            return None;
        }
        Self::new(address)
    }

    pub(crate) const fn header(self, count: usize, incrementing: bool) -> u32 {
        let flag = if incrementing { 0 } else { NON_INCREMENTING };
        flag | (count as u32) << 18 | self.0 as u32
    }
}

/// Records caller-owned RSX command words without submitting them to GCM.
///
/// The recorder never flushes a FIFO or installs a firmware callback. A full
/// buffer returns [`Error::CommandBufferFull`]; callers can then split the
/// recording into multiple buffers themselves.
pub struct CommandRecorder<'a> {
    buffer: &'a mut [u32],
    used: usize,
}

/// An immutable view of the words written by a [`CommandRecorder`].
///
/// Recording alone does not require RSX-mapped storage. Before execution,
/// copy these words to suitably aligned command memory mapped for RSX access,
/// or record directly into that memory while the CPU owns it. Make CPU writes
/// visible to the RSX under the platform's cache-coherency rules before
/// queuing execution. [`Context::flush`] supplies an ordering `sync` barrier,
/// but does not explicitly clean or write back CPU cache lines. Keep the
/// allocation and mapping alive until the GPU completes, and reach it with a
/// valid RSX jump or call target expressed as an I/O offset. A trailing return
/// word is valid only when entered through an RSX call; executing it as
/// ordinary FIFO data is invalid.
pub struct RecordedCommands<'a> {
    words: &'a [u32],
}

impl<'a> CommandRecorder<'a> {
    /// Records a command list in a closure and returns its initialized words.
    ///
    /// The supplied slice is exclusively borrowed for the duration of the
    /// recording and then exposed read-only through the returned view. If the
    /// closure or a write fails, earlier successful writes remain in the
    /// backing slice, but no view is returned.
    pub fn record(
        buffer: &'a mut [u32],
        record: impl FnOnce(&mut Self) -> Result<(), Error>,
    ) -> Result<RecordedCommands<'a>, Error> {
        let mut recorder = Self::new(buffer);
        record(&mut recorder)?;
        Ok(recorder.finish())
    }

    /// Records a command list and appends the SDK return command.
    ///
    /// If the closure, a write, or appending the return command fails, earlier
    /// successful writes remain in the backing slice, but no view is returned.
    pub fn record_with_return(
        buffer: &'a mut [u32],
        record: impl FnOnce(&mut Self) -> Result<(), Error>,
    ) -> Result<RecordedCommands<'a>, Error> {
        let mut recorder = Self::new(buffer);
        record(&mut recorder)?;
        recorder.finish_with_return()
    }

    /// Starts recording into the supplied storage.
    pub fn new(buffer: &'a mut [u32]) -> Self {
        Self { buffer, used: 0 }
    }

    /// Appends already encoded RSX command words.
    ///
    /// # Safety
    ///
    /// The words must be valid RSX commands, and any referenced GPU resources
    /// must remain mapped and alive until the RSX has finished executing them.
    pub unsafe fn write_words(&mut self, words: &[u32]) -> Result<(), Error> {
        let end = self
            .used
            .checked_add(words.len())
            .ok_or(Error::InvalidArgument)?;
        if end > self.buffer.len() {
            return Err(Error::CommandBufferFull);
        }
        self.buffer[self.used..end].copy_from_slice(words);
        self.used = end;
        Ok(())
    }

    /// Appends a method header and its argument words.
    ///
    /// # Safety
    ///
    /// The method, subchannel, arguments and referenced resources must form a
    /// valid RSX operation and remain valid until execution completes.
    pub unsafe fn write_method(
        &mut self,
        method: Method,
        incrementing: bool,
        words: &[u32],
    ) -> Result<(), Error> {
        if words.is_empty()
            || words.len() > MAX_METHOD_WORDS
            || (incrementing && method.with_offset(((words.len() - 1) * 4) as u16).is_none())
        {
            return Err(Error::InvalidArgument);
        }
        let end = self
            .used
            .checked_add(words.len() + 1)
            .ok_or(Error::InvalidArgument)?;
        if end > self.buffer.len() {
            return Err(Error::CommandBufferFull);
        }
        self.buffer[self.used] = method.header(words.len(), incrementing);
        self.buffer[self.used + 1..end].copy_from_slice(words);
        self.used = end;
        Ok(())
    }

    /// Finishes the recording without adding a control-flow command.
    pub fn finish(self) -> RecordedCommands<'a> {
        let Self { buffer, used } = self;
        RecordedCommands {
            words: &buffer[..used],
        }
    }

    /// Finishes the recording with the SDK's one-word return command.
    ///
    /// The command is useful only when the recorded words are reached through
    /// an RSX call command. It does not submit or execute the buffer.
    pub fn finish_with_return(mut self) -> Result<RecordedCommands<'a>, Error> {
        if self.used == self.buffer.len() {
            return Err(Error::CommandBufferFull);
        }
        self.buffer[self.used] = RETURN_COMMAND;
        self.used += 1;
        Ok(self.finish())
    }
}

impl RecordedCommands<'_> {
    /// Returns exactly the initialized command words, excluding unused buffer space.
    pub fn words(&self) -> &[u32] {
        self.words
    }
}

/// Exclusive access to a GCM command context.
pub struct Context {
    data: NonNull<ContextData>,
}

impl ContextData {
    /// Describes a caller-owned command buffer. The final word is held for
    /// a refill callback to install a jump to the next segment.
    pub fn for_buffer(
        buffer: &mut [u32],
        callback: Option<&PpuFunctionDescriptor>,
    ) -> Result<Self, Error> {
        if buffer.len() < 2 {
            return Err(Error::InvalidArgument);
        }
        let begin =
            u32::try_from(buffer.as_mut_ptr() as usize).map_err(|_| Error::InvalidArgument)?;
        let length =
            u32::try_from(core::mem::size_of_val(buffer)).map_err(|_| Error::InvalidArgument)?;
        let end = begin
            .checked_add(length - 4)
            .ok_or(Error::InvalidArgument)?;
        let address = callback.map_or(0, |descriptor| core::ptr::from_ref(descriptor) as usize);
        let callback = u32::try_from(address).map_err(|_| Error::InvalidArgument)?;
        Ok(Self {
            begin,
            end,
            current: begin,
            callback,
        })
    }
}

impl Context {
    /// Selects the default FIFO policy before initializing GCM.
    ///
    /// # Safety
    ///
    /// Call only before GCM initialization, while no other thread can
    /// initialize GCM or change its process-wide FIFO settings.
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub unsafe fn configure_default_fifo_mode(mode: DefaultFifoMode) -> Result<(), Error> {
        unsafe { Error::status(raw::init_default_fifo_mode(mode as i32)) }
    }

    /// Selects GCM's system mapping policy before initializing GCM.
    ///
    /// # Safety
    ///
    /// Call only before GCM initialization, while no other thread can
    /// initialize GCM or change its process-wide system settings.
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub unsafe fn configure_system_mode(mode: SystemMode) -> Result<(), Error> {
        unsafe { Error::status(raw::init_system_mode(mode as u64)) }
    }

    /// Initializes GCM using an application-owned I/O buffer in main memory.
    ///
    /// # Safety
    ///
    /// Call once per process. The I/O allocation must be accessible by the
    /// RSX and stay mapped and exclusively reserved for as long as GCM uses
    /// it. The caller must serialize access to the default context and GPU,
    /// including firmware calls and any callbacks that use either.
    pub unsafe fn init(command_bytes: u32, io: &'static mut [u8]) -> Result<Self, Error> {
        let io_bytes = u32::try_from(io.len()).map_err(|_| Error::InvalidArgument)?;
        if io.as_ptr() as usize % IO_ALIGN != 0
            || io.len() % IO_ALIGN != 0
            || command_bytes == 0
            || command_bytes % 4096 != 0
            || command_bytes > io_bytes
        {
            return Err(Error::InvalidArgument);
        }
        let slot = addr_of_mut!(DEFAULT_CONTEXT);
        unsafe {
            Error::status(raw::init_body(
                slot,
                command_bytes,
                io_bytes,
                io.as_ptr().cast(),
            ))?;
            Self::from_raw(slot.read() as usize as *mut ContextData)
        }
    }

    /// Borrows an existing GCM context for command emission.
    ///
    /// # Safety
    ///
    /// GCM must be active. `data` must refer to an initialized context, its writable command
    /// buffer, and any live compact callback descriptor. The caller must
    /// have exclusive access to the context and buffer while this handle is
    /// in use. Submit commands only to a FIFO with valid GPU mappings.
    pub unsafe fn from_raw(data: *mut ContextData) -> Result<Self, Error> {
        Ok(Self {
            data: NonNull::new(data).ok_or(Error::InvalidContext)?,
        })
    }

    pub fn as_raw(&self) -> *mut ContextData {
        self.data.as_ptr()
    }

    /// Returns the core and local-memory information for the initialized RSX.
    pub fn configuration(&self) -> Config {
        let mut config = Config::default();
        unsafe { raw::get_configuration(&mut config) };
        config
    }

    /// Returns GCM's default command-buffer capacity, in 32-bit words.
    pub fn default_command_buffer_words(&self) -> u32 {
        unsafe { raw::get_default_command_word_size() }
    }

    /// Returns GCM's default command-segment capacity, in 32-bit words.
    pub fn default_segment_words(&self) -> u32 {
        unsafe { raw::get_default_segment_word_size() }
    }

    /// Looks up the RSX offset of a mapped effective address.
    pub fn address_to_offset(&self, address: u32) -> Result<u32, Error> {
        let mut offset = 0;
        unsafe {
            Error::status(raw::address_to_offset(
                address as usize as *const _,
                &mut offset,
            ))?;
        }
        Ok(offset)
    }

    pub fn set_flip_mode(&mut self, mode: FlipMode) {
        unsafe { raw::set_flip_mode(mode as u32) };
    }

    /// Registers a scanout buffer with the firmware.
    ///
    /// # Safety
    ///
    /// The supplied offset, pitch and dimensions must describe a valid local
    /// memory allocation. Keep it available while the display can read it.
    pub unsafe fn set_display_buffer(
        &mut self,
        id: u8,
        offset: u32,
        pitch: u32,
        width: u32,
        height: u32,
    ) -> Result<(), Error> {
        unsafe { Error::status(raw::set_display_buffer(id, offset, pitch, width, height)) }
    }

    /// Enqueues a flip for a previously registered display buffer.
    ///
    /// # Safety
    ///
    /// The buffer must be ready for scanout by the time the GPU reaches this
    /// command. Observe the GCM flip queue and resource synchronization rules.
    pub unsafe fn flip(&mut self, id: u8) -> Result<(), Error> {
        if id >= 8 {
            return Err(Error::InvalidArgument);
        }
        // GCM reserves room for up to 19 words for its flip sequence.
        self.reserve(19)?;
        let status = unsafe { raw::set_flip_command(self.as_raw(), id) };
        if status < 0 {
            Err(Error::Firmware(status))
        } else {
            Ok(())
        }
    }

    /// Marks the display status as pending before the caller queues a flip.
    pub fn reset_flip_status(&mut self) {
        unsafe { raw::reset_flip_status() };
    }

    /// Polls for scanout of a queued buffer.
    pub fn wait_for_display_flip(&self, polls: u32) -> Result<(), Error> {
        for _ in 0..polls {
            if unsafe { raw::get_flip_status() } == 0 {
                return Ok(());
            }
            sys_timer_usleep(100);
        }
        Err(Error::Timeout)
    }

    /// Enqueues a flip followed by the requested firmware label operation.
    ///
    /// # Safety
    ///
    /// The display buffer and label must both be available to the GPU, and
    /// the caller must uphold the ordering and lifetime requirements of
    /// both resources.
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub unsafe fn flip_with_wait_label(
        &mut self,
        id: u8,
        index: u8,
        value: u32,
    ) -> Result<(), Error> {
        if id >= 8 {
            return Err(Error::InvalidArgument);
        }
        self.reserve(19)?;
        let status =
            unsafe { raw::set_flip_command_with_wait_label(self.as_raw(), id, index, value) };
        if status < 0 {
            Err(Error::Firmware(status))
        } else {
            Ok(())
        }
    }

    /// Prepares a pending flip and returns its queue index.
    ///
    /// # Safety
    ///
    /// The display buffer must be configured and remain valid for scanout.
    pub unsafe fn prepare_flip(&mut self, id: u8) -> Result<u32, Error> {
        if id >= 8 {
            return Err(Error::InvalidArgument);
        }
        self.reserve(19)?;
        let status = unsafe { raw::prepare_flip(self.as_raw(), id) };
        u32::try_from(status).map_err(|_| Error::Firmware(status))
    }

    /// Writes a raw method packet. The caller specifies whether the RSX
    /// should advance the method address after each word.
    ///
    /// # Safety
    ///
    /// The method, subchannel and words must form a valid operation for the
    /// current GPU state. Any referenced resources must remain valid until
    /// execution completes.
    pub unsafe fn write_method(
        &mut self,
        method: Method,
        incrementing: bool,
        words: &[u32],
    ) -> Result<(), Error> {
        if words.is_empty()
            || words.len() > MAX_METHOD_WORDS
            || (incrementing && method.with_offset(((words.len() - 1) * 4) as u16).is_none())
        {
            return Err(Error::InvalidArgument);
        }
        unsafe { self.write_words(&[&[method.header(words.len(), incrementing)], words]) }
    }

    /// Publishes the FIFO's DMA put pointer after issuing a CPU memory barrier.
    ///
    /// # Safety
    ///
    /// The current context must target the initialized and mapped RSX command
    /// FIFO. Every pending GPU command and referenced resource must be valid.
    pub unsafe fn flush(&mut self) -> Result<(), Error> {
        let current = unsafe { (*self.as_raw()).current };
        let put = self.address_to_offset(current)?;
        let ctrl = self.control()?;
        unsafe {
            core::arch::asm!("sync", options(nostack, preserves_flags));
            addr_of_mut!((*ctrl.as_ptr()).put).write_volatile(put);
        }
        Ok(())
    }

    pub(crate) fn control(&self) -> Result<NonNull<Control>, Error> {
        NonNull::new(unsafe { raw::get_control_register() }).ok_or(Error::InvalidContext)
    }

    pub(crate) unsafe fn write_words(&mut self, slices: &[&[u32]]) -> Result<(), Error> {
        let words = slices.iter().map(|slice| slice.len()).sum();
        let address = self.reserve(words)?;
        let mut cursor = address as usize as *mut u32;
        for slice in slices {
            unsafe {
                cursor.copy_from_nonoverlapping(slice.as_ptr(), slice.len());
                cursor = cursor.add(slice.len());
            }
        }
        unsafe { (*self.as_raw()).current = cursor as usize as u32 };
        Ok(())
    }

    fn reserve(&mut self, words: usize) -> Result<u32, Error> {
        let words = u32::try_from(words).map_err(|_| Error::InvalidArgument)?;
        let bytes = words.checked_mul(4).ok_or(Error::InvalidArgument)?;
        let fits = |ctx: &ContextData| {
            ctx.current >= ctx.begin && ctx.end.checked_sub(ctx.current).is_some_and(|n| n >= bytes)
        };
        let context = self.as_raw();
        unsafe {
            if !fits(&*context) {
                let callback = (*context).callback;
                if callback == 0 {
                    return Err(Error::CommandBufferFull);
                }
                Error::status(call_compact(
                    u64::from(callback),
                    context as u64,
                    words.into(),
                    0,
                ))?;
                if !fits(&*context) {
                    return Err(Error::CommandBufferFull);
                }
            }
            Ok((*context).current)
        }
    }
}
