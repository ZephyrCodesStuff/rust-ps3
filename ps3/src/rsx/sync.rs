use core::ptr::NonNull;

use crate::sys::sys_timer_usleep;

use super::commands::packet;
use super::{raw, Context, Error};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LabelStage {
    Command,
    Texture,
    Backend,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReportKind {
    ZPassPixels = 1,
    ZCullStats = 2,
    ZCullStats1 = 3,
    ZCullStats2 = 4,
    ZCullStats3 = 5,
}

/// A 32-bit GCM semaphore in GPU-visible memory.
pub struct Label {
    index: u8,
    address: NonNull<u32>,
}

impl Label {
    pub fn index(&self) -> u8 {
        self.index
    }

    pub fn read(&self) -> u32 {
        unsafe { self.address.as_ptr().read_volatile() }
    }

    /// Waits for an exact label value with a bounded number of polls.
    pub fn wait(&self, value: u32, polls: u32) -> Result<(), Error> {
        for _ in 0..polls {
            if self.read() == value {
                return Ok(());
            }
            sys_timer_usleep(100);
        }
        Err(Error::Timeout)
    }

    /// Signals the GPU from the CPU.
    ///
    /// # Safety
    ///
    /// No other writer may use this label concurrently. Observe the
    /// ordering requirements of any resource the label makes visible.
    pub unsafe fn write(&mut self, value: u32) {
        unsafe {
            core::arch::asm!("sync", options(nostack, preserves_flags));
            self.address.as_ptr().write_volatile(value);
        }
    }
}

impl Context {
    /// Obtains a label slot allocated by the firmware.
    ///
    /// # Safety
    ///
    /// The index must be available to the application. The GCM label
    /// allocation must stay mapped while this handle is in use.
    pub unsafe fn label(&self, index: u8) -> Result<Label, Error> {
        let address =
            NonNull::new(unsafe { raw::get_label_address(index) }).ok_or(Error::InvalidContext)?;
        Ok(Label { index, address })
    }

    /// Releases a label from a selected stage of the GPU pipeline.
    ///
    /// # Safety
    ///
    /// The label must be owned by the application, accessible to this GPU
    /// context and live until the release completes.
    pub unsafe fn signal_label(
        &mut self,
        label: &Label,
        stage: LabelStage,
        value: u32,
    ) -> Result<(), Error> {
        let (offset, release, value) = match stage {
            LabelStage::Command => (0x0064, 0x006c, value),
            LabelStage::Texture => (0x1d6c, 0x1d74, value),
            LabelStage::Backend => {
                let [a, r, g, b] = value.to_be_bytes();
                (0x1d6c, 0x1d70, u32::from_be_bytes([a, b, g, r]))
            }
        };
        unsafe {
            self.write_words(&[&[
                packet(offset, 1),
                u32::from(label.index) << 4,
                packet(release, 1),
                value,
            ]])
        }
    }

    /// Blocks FIFO execution until the label holds `value`.
    ///
    /// # Safety
    ///
    /// Keep the label live and ensure the write can complete independently
    /// of any commands queued after this wait.
    pub unsafe fn wait_label(&mut self, label: &Label, value: u32) -> Result<(), Error> {
        unsafe {
            self.write_words(&[&[
                packet(0x0064, 1),
                u32::from(label.index) << 4,
                packet(0x0068, 1),
                value,
            ]])
        }
    }

    /// Enables or disables the Z-pass query counter.
    pub fn set_pixel_count_enabled(&mut self, enable: bool) -> Result<(), Error> {
        unsafe { self.write_words(&[&[packet(0x17cc, 1), enable.into()]]) }
    }

    pub fn clear_report(&mut self, kind: ReportKind) -> Result<(), Error> {
        unsafe { self.write_words(&[&[packet(0x17c8, 1), kind as u32]]) }
    }

    /// Schedules a write to an RSX report slot.
    ///
    /// # Safety
    ///
    /// The report's memory location and index must be mapped and available
    /// to the GPU. Read it only after the report command has completed.
    pub unsafe fn report(&mut self, kind: ReportKind, index: u32) -> Result<(), Error> {
        if index > 0x000f_ffff {
            return Err(Error::InvalidArgument);
        }
        unsafe { self.write_words(&[&[packet(0x1800, 1), (kind as u32) << 24 | (index << 4)]]) }
    }
}
