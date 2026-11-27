use core::ptr::{addr_of_mut, NonNull};

use crate::sys::sys_timer_usleep;
use crate::sys::syscalls::call_compact;

use super::{raw, Config, ContextData, Control};

pub(super) const IO_ALIGN: usize = 1024 * 1024;
const MAX_METHOD_WORDS: usize = 0x7ff;
const NON_INCREMENTING: u32 = 1 << 30;

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

    pub(crate) const fn header(self, count: usize, incrementing: bool) -> u32 {
        let flag = if incrementing { 0 } else { NON_INCREMENTING };
        flag | (count as u32) << 18 | self.0 as u32
    }
}

/// Exclusive access to a GCM command context.
pub struct Context {
    data: NonNull<ContextData>,
}

impl Context {
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
        if words.is_empty() || words.len() > MAX_METHOD_WORDS {
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
