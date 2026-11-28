use core::marker::PhantomData;
use core::ops::Range;

use super::context::IO_ALIGN;
use super::{raw, Context, Error, Location};

/// Main memory data aligned to a GCM I/O page. Declare it as a static or
/// allocate it on the heap. `N` must be a multiple of 1 MiB for GCM mapping.
#[repr(C, align(1048576))]
pub struct IoBuffer<const N: usize>(pub [u8; N]);

/// A region of GPU-addressable memory. The GPU offset is relative to the
/// memory space indicated by [`Self::location`].
pub struct Buffer {
    location: Location,
    address: u32,
    offset: u32,
    size: u32,
    exclusive: PhantomData<*mut u8>,
}

impl Buffer {
    pub fn location(&self) -> Location {
        self.location
    }

    pub fn address(&self) -> u32 {
        self.address
    }

    pub fn offset(&self) -> u32 {
        self.offset
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    /// Copies bytes into this buffer from the CPU.
    ///
    /// # Safety
    ///
    /// The buffer must still be mapped and have no concurrent GPU or CPU
    /// access to the destination range. Source and destination must be
    /// disjoint. Submit a GPU command buffer or issue the appropriate
    /// memory barrier before the GPU reads the data.
    pub unsafe fn write(&mut self, start: u32, bytes: &[u8]) -> Result<(), Error> {
        let count = u32::try_from(bytes.len()).map_err(|_| Error::InvalidArgument)?;
        if start.checked_add(count).is_none_or(|end| end > self.size) {
            return Err(Error::InvalidArgument);
        }
        if !bytes.is_empty() {
            let dest = self.address as usize as *mut u8;
            unsafe {
                dest.add(start as usize)
                    .copy_from_nonoverlapping(bytes.as_ptr(), bytes.len())
            };
        }
        Ok(())
    }

    /// Reads an aligned 32-bit word after all GPU writes have completed.
    ///
    /// # Safety
    ///
    /// The buffer must still be mapped, and neither GPU nor CPU may write
    /// to this word concurrently.
    pub unsafe fn read_word(&self, start: u32) -> Result<u32, Error> {
        if start.checked_add(4).is_none_or(|end| end > self.size) || (self.address + start) % 4 != 0
        {
            return Err(Error::InvalidArgument);
        }
        let address = (self.address + start) as usize as *const u32;
        Ok(unsafe { address.read_volatile() })
    }

    /// Removes a main-memory mapping.
    ///
    /// # Safety
    ///
    /// The GPU must have finished all work that references this buffer.
    /// The caller must release or invalidate any other views of the range.
    pub unsafe fn unmap_main(self) -> Result<(), Error> {
        if self.location != Location::Main {
            return Err(Error::InvalidArgument);
        }
        unsafe { Error::status(raw::unmap_ea_io_address(self.address as usize as *const _)) }
    }
}

/// Allocates disjoint, aligned regions from an application-owned VRAM range.
/// Individual allocations stay reserved until the arena is reset.
pub struct LocalArena {
    base: u32,
    start: u32,
    next: u32,
    end: u32,
    exclusive: PhantomData<*mut u8>,
}

impl LocalArena {
    pub fn allocate(&mut self, size: u32, alignment: u32) -> Result<Buffer, Error> {
        if !alignment.is_power_of_two() || size == 0 {
            return Err(Error::InvalidArgument);
        }
        let align_mask = alignment - 1;
        let offset = self
            .next
            .checked_add(align_mask)
            .ok_or(Error::OutOfMemory)?
            & !align_mask;
        let next = offset.checked_add(size).ok_or(Error::OutOfMemory)?;
        if next > self.end {
            return Err(Error::OutOfMemory);
        }
        self.next = next;
        Ok(Buffer {
            location: Location::Local,
            address: self.base + offset,
            offset,
            size,
            exclusive: PhantomData,
        })
    }

    pub fn unused_bytes(&self) -> u32 {
        self.end - self.next
    }

    /// Reclaims the entire arena.
    ///
    /// # Safety
    ///
    /// All previous buffer handles, scanout registrations, and outstanding
    /// GPU or CPU accesses to this arena must have been released.
    pub unsafe fn reset(&mut self) {
        self.next = self.start;
    }
}

impl Context {
    /// Reserves an application-owned subrange of local RSX memory.
    ///
    /// # Safety
    ///
    /// The caller must own the range exclusively, with no other arenas or
    /// allocations covering these offsets. Maintain that ownership for as
    /// long as any allocated buffer is live.
    pub unsafe fn local_arena(&self, range: Range<u32>) -> Result<LocalArena, Error> {
        let config = self.configuration();
        if config.local_address == 0
            || range.start >= range.end
            || range.end > config.local_size
            || config.local_address.checked_add(range.end - 1).is_none()
        {
            return Err(Error::InvalidArgument);
        }
        Ok(LocalArena {
            base: config.local_address,
            start: range.start,
            next: range.start,
            end: range.end,
            exclusive: PhantomData,
        })
    }

    /// Maps an application allocation into the RSX I/O address space.
    ///
    /// # Safety
    ///
    /// The entire range must be valid, aligned and unused by any other I/O
    /// mapping. Keep the underlying allocation at a stable address until
    /// the GPU is done and the mapping has been successfully removed.
    pub unsafe fn map_main(&mut self, memory: *mut u8, size: u32) -> Result<Buffer, Error> {
        let address = mapping_address(memory, size)?;
        let mut offset = 0;
        unsafe { Error::status(raw::map_main_memory(memory.cast(), size, &mut offset))? };
        Ok(Buffer {
            location: Location::Main,
            address,
            offset,
            size,
            exclusive: PhantomData,
        })
    }

    /// Maps an application allocation at a caller-selected RSX I/O offset.
    ///
    /// `offset` and `size` must describe a range accepted by GCM and must not
    /// overlap another mapping. The offset must satisfy the GCM I/O alignment
    /// requirements. The returned buffer's offset is the requested value.
    ///
    /// # Safety
    ///
    /// The allocation must be valid, aligned, exclusively mapped at this I/O
    /// range, and remain at a stable address until the mapping is removed and
    /// all GPU work using it has completed.
    pub unsafe fn map_main_at(
        &mut self,
        memory: *mut u8,
        size: u32,
        offset: u32,
    ) -> Result<Buffer, Error> {
        self.map_main_at_impl(memory, size, offset, None)
    }

    /// Maps an allocation at a caller-selected RSX I/O offset with GCM user
    /// mapping flags.
    ///
    /// The meaning of `user_flags` is defined by the SDK/runtime contract.
    /// The returned buffer's offset is the requested value.
    ///
    /// # Safety
    ///
    /// The allocation must be valid, aligned, exclusively mapped at this I/O
    /// range, and remain at a stable address until the mapping is removed and
    /// all GPU work using it has completed. `user_flags` must be valid for
    /// the target SDK and mapping.
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub unsafe fn map_main_at_with_flags(
        &mut self,
        memory: *mut u8,
        size: u32,
        offset: u32,
        user_flags: u32,
    ) -> Result<Buffer, Error> {
        self.map_main_at_impl(memory, size, offset, Some(user_flags))
    }

    /// Returns the effective address corresponding to a mapped RSX I/O
    /// offset.
    pub fn io_offset_to_address(&self, offset: u32) -> Result<u32, Error> {
        let mut address = 0;
        unsafe { Error::status(raw::io_offset_to_address(offset, &mut address))? };
        Ok(address)
    }

    /// Reserves additional RSX I/O mapping space from GCM's mapping pool.
    ///
    /// Available with the SDK 3.70 ABI.
    ///
    /// # Safety
    ///
    /// Coordinate reservations with every other user of the process GCM
    /// mapping pool. Do not reserve space already allocated or reserved.
    #[cfg(feature = "sdk-370")]
    pub unsafe fn reserve_io_map_size(&mut self, size: u32) -> Result<(), Error> {
        unsafe { Error::status(raw::reserve_io_map_size(size)) }
    }

    /// Releases a previous reservation from GCM's RSX I/O mapping pool.
    ///
    /// # Safety
    ///
    /// `size` must match space previously reserved by this process and must
    /// not include space still used by an I/O mapping.
    #[cfg(feature = "sdk-370")]
    pub unsafe fn unreserve_io_map_size(&mut self, size: u32) -> Result<(), Error> {
        unsafe { Error::status(raw::unreserve_io_map_size(size)) }
    }

    /// Returns the maximum RSX I/O mapping size supported by GCM, in bytes.
    #[cfg(feature = "sdk-370")]
    pub fn max_io_map_size(&self) -> u32 {
        unsafe { raw::get_max_io_map_size() }
    }

    /// Maps an application allocation at the RSX main report table's fixed
    /// I/O address. Each report consumes 16 bytes of the mapped range.
    ///
    /// # Safety
    /// Follow the same allocation and lifetime requirements as
    /// [`Self::map_main`]. Reserve space for any debugger or performance
    /// tools that use the report table, and keep the mapping live until the
    /// last report and conditional draw has completed.
    pub unsafe fn map_main_reports(&mut self, memory: *mut u8, size: u32) -> Result<Buffer, Error> {
        const REPORT_IO_BASE: u32 = 0x0e00_0000;
        const REPORT_IO_SIZE: u32 = 0x0100_0000;
        if size > REPORT_IO_SIZE {
            return Err(Error::InvalidArgument);
        }
        unsafe { self.map_main_at(memory, size, REPORT_IO_BASE) }
    }

    /// Maps an aligned I/O page for the eight GCM notification slots.
    ///
    /// # Safety
    /// Follow the same allocation and lifetime requirements as
    /// [`Self::map_main`] for the full 1 MiB page. Keep it mapped until
    /// all queued notifications have completed.
    pub unsafe fn map_notifications(&mut self, memory: *mut u8) -> Result<Buffer, Error> {
        const NOTIFY_IO_BASE: u32 = 0x0f10_0000;
        unsafe { self.map_main_at(memory, IO_ALIGN as u32, NOTIFY_IO_BASE) }
    }

    unsafe fn map_main_at_impl(
        &mut self,
        memory: *mut u8,
        size: u32,
        offset: u32,
        user_flags: Option<u32>,
    ) -> Result<Buffer, Error> {
        let address = mapping_address(memory, size)?;
        if offset as usize % IO_ALIGN != 0 || offset.checked_add(size - 1).is_none() {
            return Err(Error::InvalidArgument);
        }
        match user_flags {
            #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
            Some(flags) => unsafe {
                Error::status(raw::map_ea_io_address_with_flags(
                    memory.cast(),
                    offset,
                    size,
                    flags,
                ))?
            },
            _ => unsafe { Error::status(raw::map_ea_io_address(memory.cast(), offset, size))? },
        }
        Ok(Buffer {
            location: Location::Main,
            address,
            offset,
            size,
            exclusive: PhantomData,
        })
    }
}

fn mapping_address(memory: *mut u8, size: u32) -> Result<u32, Error> {
    let address = u32::try_from(memory as usize).map_err(|_| Error::InvalidArgument)?;
    if address == 0
        || size == 0
        || address as usize % IO_ALIGN != 0
        || size as usize % IO_ALIGN != 0
        || address.checked_add(size - 1).is_none()
    {
        return Err(Error::InvalidArgument);
    }
    Ok(address)
}
