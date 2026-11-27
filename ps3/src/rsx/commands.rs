use core::ptr::addr_of;

use crate::sys::sys_timer_usleep;

use super::{Context, Error, Method, Surface};

const NOP: u16 = 0x0100;
const REFERENCE: u16 = 0x0050;
const SEMAPHORE_OFFSET: u16 = 0x0064;
const SEMAPHORE_RELEASE: u16 = 0x006c;
const COLOR_CLEAR: u16 = 0x1d90;
const DEPTH_CLEAR: u16 = 0x1d8c;
const CLEAR: u16 = 0x1d94;
const DMA_LOCAL: u32 = 0xfeed_0000;

const fn packet(address: u16, count: usize) -> u32 {
    // All named methods below are aligned constants.
    Method::new(address).unwrap().header(count, true)
}

/// Selects which parts of a render target a clear affects.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClearMask(u32);

impl ClearMask {
    pub const DEPTH: Self = Self(0x01);
    pub const STENCIL: Self = Self(0x02);
    pub const RED: Self = Self(0x10);
    pub const GREEN: Self = Self(0x20);
    pub const BLUE: Self = Self(0x40);
    pub const ALPHA: Self = Self(0x80);
    pub const COLOR: Self = Self(0xf0);
    pub const ALL: Self = Self(0xf3);
}

impl core::ops::BitOr for ClearMask {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl Context {
    /// Selects the framebuffer and depth buffer for subsequent drawing.
    ///
    /// # Safety
    ///
    /// Active offsets, pitches and formats must describe mapped GPU memory
    /// that stays available until rendering and any display use complete.
    pub unsafe fn set_surface(&mut self, surface: &Surface) -> Result<(), Error> {
        if surface.width == 0
            || surface.height == 0
            || surface.color_location.iter().any(|loc| *loc > 1)
            || surface.depth_location > 1
        {
            return Err(Error::InvalidArgument);
        }
        let format = (surface.height.ilog2() << 24)
            | (surface.width.ilog2() << 16)
            | (u32::from(surface.antialias) << 12)
            | (u32::from(surface.kind) << 8)
            | (u32::from(surface.depth_format) << 5)
            | u32::from(surface.color_format);
        let pack_xy = |x: u16, y: u16| u32::from(x) | (u32::from(y) << 16);
        let local_or_main = |location: u8| DMA_LOCAL + u32::from(location);
        unsafe {
            self.write_words(&[&[
                packet(0x0194, 1), // DMA color 0
                local_or_main(surface.color_location[0]),
                packet(0x018c, 1), // DMA color 1
                local_or_main(surface.color_location[1]),
                packet(0x01b4, 2), // DMA colors 2 and 3
                local_or_main(surface.color_location[2]),
                local_or_main(surface.color_location[3]),
                packet(0x0198, 1), // DMA depth
                local_or_main(surface.depth_location),
                packet(0x0208, 6), // format, color pitches, color and depth offsets
                format,
                surface.color_pitch[0],
                surface.color_offset[0],
                surface.depth_offset,
                surface.color_offset[1],
                surface.color_pitch[1],
                packet(0x022c, 1),
                surface.depth_pitch,
                packet(0x0280, 4),
                surface.color_pitch[2],
                surface.color_pitch[3],
                surface.color_offset[2],
                surface.color_offset[3],
                packet(0x0220, 1),
                surface.color_target.into(),
                packet(0x02b8, 1),
                pack_xy(surface.x, surface.y),
                packet(0x0200, 2),
                pack_xy(surface.x, surface.width),
                pack_xy(surface.y, surface.height),
                packet(0x1d88, 1),
                (1 << 12) | u32::from(surface.height),
            ]])
        }
    }

    /// Loads the clear color in ARGB byte order.
    pub fn set_clear_color(&mut self, argb: u32) -> Result<(), Error> {
        unsafe { self.write_words(&[&[packet(COLOR_CLEAR, 1), argb]]) }
    }

    /// Loads the packed depth and stencil clear value.
    pub fn set_clear_depth_stencil(&mut self, value: u32) -> Result<(), Error> {
        unsafe { self.write_words(&[&[packet(DEPTH_CLEAR, 1), value]]) }
    }

    /// Clears the selected channels of the currently bound surface.
    ///
    /// # Safety
    ///
    /// The target and channels must be valid for writes by the GPU.
    pub unsafe fn clear(&mut self, mask: ClearMask) -> Result<(), Error> {
        unsafe { self.write_words(&[&[packet(CLEAR, 1), mask.0, packet(NOP, 1), 0]]) }
    }

    /// Waits on the semaphore reserved by GCM for display flips.
    pub fn wait_flip(&mut self) -> Result<(), Error> {
        unsafe { self.write_words(&[&[packet(SEMAPHORE_OFFSET, 2), 0x10, 0]]) }
    }

    /// Writes an RSX label from the command processor when this packet executes.
    ///
    /// # Safety
    ///
    /// The label index must be owned by the caller and safe to update
    /// according to the application's synchronization protocol.
    pub unsafe fn write_command_label(&mut self, index: u8, value: u32) -> Result<(), Error> {
        unsafe {
            self.write_words(&[&[
                packet(SEMAPHORE_OFFSET, 1),
                u32::from(index) << 4,
                packet(SEMAPHORE_RELEASE, 1),
                value,
            ]])
        }
    }

    /// Appends a unique reference value and submits the FIFO. Waits at most
    /// `polls` iterations, sleeping 100 microseconds between observations.
    ///
    /// # Safety
    ///
    /// All commands and resources must meet the requirements of
    /// [`Context::flush`]. The reference must differ from the current and any
    /// still pending reference values.
    pub unsafe fn finish(&mut self, reference: u32, polls: u32) -> Result<(), Error> {
        unsafe {
            self.write_words(&[&[packet(REFERENCE, 1), reference]])?;
            self.flush()?;
        }
        let ctrl = self.control()?.as_ptr();
        for _ in 0..polls {
            if unsafe { addr_of!((*ctrl).reference).read_volatile() } == reference {
                return Ok(());
            }
            sys_timer_usleep(100);
        }
        Err(Error::Timeout)
    }
}
