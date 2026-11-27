//! LV2 overlay entry points and their 32-bit SDK records.

/// A segment record written by the overlay information calls.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SysOverlaySegment {
    pub base: u32,
    pub memory_size: u32,
    pub index: u32,
    pub reserved: u32,
}

/// Buffer addresses are 32-bit effective addresses in the requesting process.
/// The name length is written without its terminating NUL.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SysOverlayModuleInfo {
    pub size: u64,
    /// Address of the file-name buffer. Allow space for the terminating NUL.
    pub file_name_addr: u32,
    pub file_name_len: u32,
    /// Number of writable records at `segments_addr`.
    pub segment_count: u32,
    pub segments_addr: u32,
}

impl Default for SysOverlayModuleInfo {
    fn default() -> Self {
        Self {
            size: core::mem::size_of::<Self>() as u64,
            file_name_addr: 0,
            file_name_len: 0,
            segment_count: 0,
            segments_addr: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SysOverlayModuleInfo2 {
    pub module: SysOverlayModuleInfo,
    /// Address of the module-name buffer. Allow space for the terminating NUL.
    pub name_addr: u32,
    pub name_len: u32,
}

impl Default for SysOverlayModuleInfo2 {
    fn default() -> Self {
        Self {
            module: SysOverlayModuleInfo {
                size: core::mem::size_of::<Self>() as u64,
                ..Default::default()
            },
            name_addr: 0,
            name_len: 0,
        }
    }
}

assert_layout!(SysOverlaySegment, 16, 4, base = 0, index = 8);
assert_layout!(
    SysOverlayModuleInfo,
    24,
    8,
    file_name_addr = 8,
    file_name_len = 12,
    segments_addr = 20
);
assert_layout!(SysOverlayModuleInfo2, 32, 8, name_addr = 24, name_len = 28);

define_syscalls! {
    /// Loads a fixed-address overlay for the current process.
    pub unsafe fn sys_overlay_load_module(
        module_id: *mut u32, path: *const u8, flags: u64, entry: *mut u32,
    ) -> i32 = SYS_OVERLAY_LOAD_MODULE;

    /// Unloads a fixed-address overlay.
    pub unsafe fn sys_overlay_unload_module(module_id: u32) -> i32 = SYS_OVERLAY_UNLOAD_MODULE;

    /// Writes up to `capacity` overlay IDs and a 32-bit result count.
    pub unsafe fn sys_overlay_get_module_list(
        process_id: u32, capacity: u32, modules: *mut u32, count: *mut u32,
    ) -> i32 = SYS_OVERLAY_GET_MODULE_LIST;

    /// Queries an overlay's file name and segment records.
    pub unsafe fn sys_overlay_get_module_info(
        process_id: u32, module_id: u32, info: *mut SysOverlayModuleInfo,
    ) -> i32 = SYS_OVERLAY_GET_MODULE_INFO;

    /// Loads a fixed-address overlay from a nonnegative file offset.
    pub unsafe fn sys_overlay_load_module_by_fd(
        module_id: *mut u32, fd: i32, offset: i64, flags: u64, entry: *mut u32,
    ) -> i32 = SYS_OVERLAY_LOAD_MODULE_BY_FD;

    /// Queries overlay information using the extended name buffer.
    pub unsafe fn sys_overlay_get_module_info2(
        process_id: u32, module_id: u32, info: *mut SysOverlayModuleInfo2,
    ) -> i32 = SYS_OVERLAY_GET_MODULE_INFO2;
}
