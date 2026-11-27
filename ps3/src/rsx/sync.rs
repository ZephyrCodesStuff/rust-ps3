use core::ptr::NonNull;

use crate::sys::sys_timer_usleep;

use super::commands::packet;
use super::methods::{nv0039, nv4097};
use super::{raw, Buffer, Context, Error, Location};
#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
use super::{NotifyData, ReportData};

const LOCAL_REPORT_DMA: u32 = 0x6662_6660;
const MAIN_REPORT_DMA: u32 = 0xbad6_8000;
const MAIN_MEMORY_DMA: u32 = 0xfeed_0001;
const NOTIFY_DMA_ZERO: u32 = 0x6660_420f;
const LOCAL_REPORTS: u32 = 2048;
const MAIN_REPORTS: u32 = 0x0010_0000;
const REPORT_BYTES: u32 = 16;

/// An index in one of the firmware-owned RSX report tables.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReportSlot {
    location: Location,
    index: u32,
}

impl ReportSlot {
    pub const fn new(location: Location, index: u32) -> Result<Self, Error> {
        let count = match location {
            Location::Local => LOCAL_REPORTS,
            Location::Main => MAIN_REPORTS,
        };
        if index >= count {
            return Err(Error::InvalidArgument);
        }
        Ok(Self { location, index })
    }

    pub const fn location(self) -> Location {
        self.location
    }

    pub const fn index(self) -> u32 {
        self.index
    }

    const fn dma(self) -> u32 {
        match self.location {
            Location::Local => LOCAL_REPORT_DMA,
            Location::Main => MAIN_REPORT_DMA,
        }
    }
}

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
        unsafe { self.emit_label(label, stage, value, false) }
    }

    /// Signals a label followed by the idle barrier used by GCM's
    /// conditional-rendering synchronization helpers.
    ///
    /// # Safety
    /// The label must be owned by the application and remain live until
    /// the release completes.
    pub unsafe fn signal_conditional_label(
        &mut self,
        label: &Label,
        stage: LabelStage,
        value: u32,
    ) -> Result<(), Error> {
        if stage == LabelStage::Command {
            return Err(Error::InvalidArgument);
        }
        unsafe { self.emit_label(label, stage, value, true) }
    }

    unsafe fn emit_label(
        &mut self,
        label: &Label,
        stage: LabelStage,
        value: u32,
        idle: bool,
    ) -> Result<(), Error> {
        let (offset, release, value) = match stage {
            LabelStage::Command => (0x0064, 0x006c, value),
            LabelStage::Texture => (0x1d6c, 0x1d74, value),
            LabelStage::Backend => {
                let [a, r, g, b] = value.to_be_bytes();
                (0x1d6c, 0x1d70, u32::from_be_bytes([a, b, g, r]))
            }
        };
        let words = [
            packet(offset, 1),
            u32::from(label.index) << 4,
            packet(release, 1),
            value,
            nv4097::WAIT_FOR_IDLE.header(1, true),
            0,
        ];
        let count = if idle { 6 } else { 4 };
        unsafe { self.write_words(&[&words[..count]]) }
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

    pub fn set_report_location(&mut self, location: Location) -> Result<(), Error> {
        use super::methods::nv4097::SET_CONTEXT_DMA_REPORT;
        let dma = match location {
            Location::Local => LOCAL_REPORT_DMA,
            Location::Main => MAIN_REPORT_DMA,
        };
        unsafe { self.write_method(SET_CONTEXT_DMA_REPORT, true, &[dma]) }
    }

    /// Uses an occlusion report to select whether subsequent draws execute.
    ///
    /// # Safety
    ///
    /// A selected report must be mapped in the current report location and
    /// have been scheduled before this command.
    pub unsafe fn set_conditional_render(&mut self, report: Option<u32>) -> Result<(), Error> {
        match report {
            Some(index) if index <= 0x000f_ffff => unsafe {
                self.write_words(&[&[
                    packet(0x0100, 1),
                    0,
                    packet(0x1e98, 1),
                    0x0200_0000 | index << 4,
                ]])
            },
            Some(_) => Err(Error::InvalidArgument),
            None => unsafe { self.write_words(&[&[packet(0x1e98, 1), 0x0100_0000]]) },
        }
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

    /// Selects the slot's report table and writes the current counter and
    /// timestamp to the slot. Z-pass reports also serve as timestamp queries.
    ///
    /// # Safety
    /// The slot must be available for this context to write. Main slots
    /// must be backed by a live mapping from [`Self::map_main_reports`].
    /// Synchronize with other users of the slot and with subsequent reads.
    pub unsafe fn write_report(&mut self, kind: ReportKind, slot: ReportSlot) -> Result<(), Error> {
        unsafe {
            self.write_words(&[&[
                nv4097::SET_CONTEXT_DMA_REPORT.header(1, true),
                slot.dma(),
                nv4097::GET_REPORT.header(1, true),
                (kind as u32) << 24 | (slot.index * REPORT_BYTES),
            ]])
        }
    }

    /// Selects the slot's report table and gates draws on its Z-pass result.
    ///
    /// # Safety
    /// Schedule the occlusion report before this command and retain its
    /// slot until the last dependent draw has executed. Main slots
    /// require a live [`Self::map_main_reports`] mapping.
    pub unsafe fn render_if_visible(&mut self, slot: ReportSlot) -> Result<(), Error> {
        unsafe {
            self.write_words(&[&[
                nv4097::SET_CONTEXT_DMA_REPORT.header(1, true),
                slot.dma(),
                nv4097::NO_OPERATION.header(1, true),
                0,
                nv4097::SET_RENDER_ENABLE.header(1, true),
                0x0200_0000 | (slot.index * REPORT_BYTES),
            ]])
        }
    }

    /// Reads a local report's timestamp and value through the firmware API.
    ///
    /// # Safety
    /// The slot must be local and the GPU must have finished writing it.
    /// Keep other writers synchronized until the read returns.
    pub unsafe fn read_local_report(
        &self,
        kind: ReportKind,
        slot: ReportSlot,
    ) -> Result<(u64, u32), Error> {
        if slot.location != Location::Local {
            return Err(Error::InvalidArgument);
        }
        unsafe {
            Ok((
                raw::get_time_stamp(slot.index),
                raw::get_report(kind as u32, slot.index),
            ))
        }
    }

    /// Reads a completed entry from the selected report table.
    ///
    /// # Safety
    /// Synchronize with the GPU before reading and keep the slot stable
    /// throughout the read.
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub unsafe fn read_report(&self, slot: ReportSlot) -> Result<ReportData, Error> {
        let address =
            unsafe { raw::get_report_data_address_location(slot.index, slot.location as u32) };
        let address = NonNull::new(address).ok_or(Error::InvalidContext)?;
        Ok(unsafe { address.as_ptr().read_volatile() })
    }

    /// Queues a DMA copy of consecutive local report records to main memory.
    ///
    /// # Safety
    /// The reports and destination must stay live. Order accesses to the
    /// source reports and wait for completion before reading the destination.
    pub unsafe fn copy_local_reports(
        &mut self,
        first: ReportSlot,
        count: u32,
        destination: &Buffer,
        start: u32,
    ) -> Result<(), Error> {
        if count == 0 {
            return Ok(());
        }
        if first.location != Location::Local
            || count > LOCAL_REPORTS - first.index
            || destination.location() != Location::Main
        {
            return Err(Error::InvalidArgument);
        }
        let bytes = count * REPORT_BYTES;
        let offset = destination
            .offset()
            .checked_add(start)
            .ok_or(Error::InvalidArgument)?;
        if offset % REPORT_BYTES != 0
            || start
                .checked_add(bytes)
                .is_none_or(|end| end > destination.size())
        {
            return Err(Error::InvalidArgument);
        }
        unsafe {
            self.write_words(&[&[
                nv0039::SET_CONTEXT_DMA_BUFFER_IN.header(2, true),
                LOCAL_REPORT_DMA,
                MAIN_MEMORY_DMA,
                nv0039::OFFSET_IN.header(8, true),
                first.index * REPORT_BYTES,
                offset,
                0,
                0,
                bytes,
                1,
                0x0101,
                0,
                nv0039::OFFSET_OUT.header(1, true),
                0,
            ]])
        }
    }

    /// Queues a pipeline idle barrier. Call `finish` for a CPU-side wait.
    pub fn wait_for_idle(&mut self) -> Result<(), Error> {
        unsafe { self.write_method(nv4097::WAIT_FOR_IDLE, true, &[0]) }
    }

    /// Triggers the configured RSX performance monitor after an idle barrier.
    pub fn trigger_performance_monitor(&mut self) -> Result<(), Error> {
        unsafe {
            self.write_words(&[&[
                nv4097::WAIT_FOR_IDLE.header(1, true),
                0,
                nv4097::PM_TRIGGER.header(1, true),
                1,
            ]])
        }
    }

    /// Queues a notification to one of the eight GCM main-memory DMA slots.
    ///
    /// # Safety
    /// The slot must be available to the application and backed by a live
    /// [`Self::map_notifications`] mapping. Synchronize access with any
    /// preceding or following users of the same notification.
    pub unsafe fn notify(&mut self, index: u8) -> Result<(), Error> {
        if index >= 8 {
            return Err(Error::InvalidArgument);
        }
        unsafe {
            self.write_words(&[&[
                nv4097::SET_CONTEXT_DMA_NOTIFIES.header(1, true),
                NOTIFY_DMA_ZERO - u32::from(index),
                nv4097::NOTIFY.header(1, true),
                0,
                nv4097::NO_OPERATION.header(1, true),
                0,
            ]])
        }
    }

    /// Reads the completed notification in a GCM main-memory DMA slot.
    ///
    /// # Safety
    /// Keep the slot stable while reading and wait for the GPU to finish
    /// writing it first.
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub unsafe fn read_notification(&self, index: u8) -> Result<NotifyData, Error> {
        if index >= 8 {
            return Err(Error::InvalidArgument);
        }
        let address = NonNull::new(unsafe { raw::get_notify_data_address(u32::from(index)) })
            .ok_or(Error::InvalidContext)?;
        Ok(unsafe { address.as_ptr().read_volatile() })
    }
}
