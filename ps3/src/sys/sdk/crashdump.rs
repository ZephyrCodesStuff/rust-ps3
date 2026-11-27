//! Application log areas for the firmware's crash dump facility.

pub const SYS_CRASH_DUMP_MAX_LABEL_SIZE: usize = 16;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SysCrashDumpLogAreaInfo {
    pub label: [u8; SYS_CRASH_DUMP_MAX_LABEL_SIZE],
    /// Effective address of a readable area that remains valid while registered.
    pub addr: u32,
    pub size: u32,
}

const _: () = {
    use core::mem::{align_of, offset_of, size_of};
    assert!(size_of::<SysCrashDumpLogAreaInfo>() == 24);
    assert!(align_of::<SysCrashDumpLogAreaInfo>() == 4);
    assert!(offset_of!(SysCrashDumpLogAreaInfo, addr) == 16);
    assert!(offset_of!(SysCrashDumpLogAreaInfo, size) == 20);
};

extern "C" {
    /// Writes the log area registered at `index` to `entry`.
    pub fn sys_crash_dump_get_user_log_area(index: u8, entry: *mut SysCrashDumpLogAreaInfo) -> i32;
    /// Registers an area for inclusion in application crash dumps.
    pub fn sys_crash_dump_set_user_log_area(index: u8, entry: *mut SysCrashDumpLogAreaInfo) -> i32;
}
