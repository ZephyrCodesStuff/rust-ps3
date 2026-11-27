//! PRX module loading, lifecycle, and metadata from `sysPrxForUser`.
//!
//! PRX IDs and addresses are 32-bit values. The query records retain
//! 32-bit embedded addresses even when compiled with Rust's 64-bit pointers.

use core::ffi::c_void;

pub type SysPrxId = i32;

pub const SYS_PRX_RESIDENT: i32 = 0;
pub const SYS_PRX_NO_RESIDENT: i32 = 1;
pub const SYS_PRX_STOP_SUCCESS: i32 = 0;
pub const SYS_PRX_STOP_FAILED: i32 = 1;
pub const SYS_PRX_PROCESS_ELF_ID: SysPrxId = 0;
pub const SYS_PRX_MODULE_FILENAME_SIZE: usize = 512;
#[cfg(feature = "sdk-370")]
pub const SYS_PRX_LOAD_MODULE_FLAGS_FIXEDADDR: u64 = 1;

macro_rules! size_option {
    ($($name:ident),+ $(,)?) => {
        $(
            #[repr(C)]
            #[derive(Debug, Clone, Copy)]
            pub struct $name {
                pub size: u64,
            }

            impl Default for $name {
                fn default() -> Self {
                    Self { size: core::mem::size_of::<Self>() as u64 }
                }
            }

            const _: () = {
                assert!(core::mem::size_of::<$name>() == 8);
                assert!(core::mem::align_of::<$name>() == 8);
            };
        )+
    };
}

size_option!(
    SysPrxLoadModuleOption,
    SysPrxStartModuleOption,
    SysPrxStopModuleOption,
    SysPrxUnloadModuleOption,
    SysPrxGetModuleIdByNameOption,
);
#[cfg(feature = "sdk-370")]
size_option!(SysPrxLoadModuleListOption);

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SysPrxSegmentInfo {
    pub base: u64,
    pub file_size: u64,
    pub memory_size: u64,
    pub index: u64,
    pub kind: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SysPrxModuleList {
    pub size: u64,
    pub capacity: u32,
    pub count: u32,
    /// Effective address of a writable array of module IDs.
    pub ids_addr: u32,
    /// Optional effective address for stop-level output.
    pub levels_addr: u32,
}

impl Default for SysPrxModuleList {
    fn default() -> Self {
        Self {
            size: core::mem::size_of::<Self>() as u64,
            capacity: 0,
            count: 0,
            ids_addr: 0,
            levels_addr: 0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SysPrxModuleInfo {
    pub size: u64,
    pub name: [u8; 30],
    pub version: [u8; 2],
    pub attributes: u32,
    pub start_entry: u32,
    pub stop_entry: u32,
    pub total_segments: u32,
    pub filename_addr: u32,
    pub filename_size: u32,
    pub segments_addr: u32,
    pub segments_count: u32,
}

impl Default for SysPrxModuleInfo {
    fn default() -> Self {
        Self {
            size: core::mem::size_of::<Self>() as u64,
            name: [0; 30],
            version: [0; 2],
            attributes: 0,
            start_entry: 0,
            stop_entry: 0,
            total_segments: 0,
            filename_addr: 0,
            filename_size: 0,
            segments_addr: 0,
            segments_count: 0,
        }
    }
}

const _: () = {
    use core::mem::{align_of, offset_of, size_of};
    assert!(size_of::<SysPrxSegmentInfo>() == 40);
    assert!(align_of::<SysPrxSegmentInfo>() == 8);
    assert!(size_of::<SysPrxModuleList>() == 24);
    assert!(align_of::<SysPrxModuleList>() == 8);
    assert!(offset_of!(SysPrxModuleList, ids_addr) == 16);
    assert!(size_of::<SysPrxModuleInfo>() == 72);
    assert!(align_of::<SysPrxModuleInfo>() == 8);
    assert!(offset_of!(SysPrxModuleInfo, name) == 8);
    assert!(offset_of!(SysPrxModuleInfo, attributes) == 40);
    assert!(offset_of!(SysPrxModuleInfo, filename_addr) == 56);
    assert!(offset_of!(SysPrxModuleInfo, segments_addr) == 64);
};

// Each declaration calls the official user-mode wrapper through sysPrxForUser.
// Callers must uphold SDK pointer, ownership, flag, and module state rules.
extern "C" {
    pub fn sys_prx_load_module(
        path: *const u8,
        flags: u64,
        option: *mut SysPrxLoadModuleOption,
    ) -> SysPrxId;
    pub fn sys_prx_load_module_on_memcontainer(
        path: *const u8,
        container: u32,
        flags: u64,
        option: *mut SysPrxLoadModuleOption,
    ) -> SysPrxId;
    pub fn sys_prx_load_module_by_fd(
        fd: i32,
        offset: i64,
        flags: u64,
        option: *mut SysPrxLoadModuleOption,
    ) -> SysPrxId;
    #[cfg(feature = "sdk-370")]
    pub fn sys_prx_load_module_on_memcontainer_by_fd(
        fd: i32,
        offset: i64,
        container: u32,
        flags: u64,
        option: *mut SysPrxLoadModuleOption,
    ) -> SysPrxId;
    #[cfg(feature = "sdk-370")]
    pub fn sys_prx_load_module_list(
        count: i32,
        paths_addr: *const u32,
        flags: u64,
        option: *mut SysPrxLoadModuleListOption,
        ids: *mut SysPrxId,
    ) -> i32;
    #[cfg(feature = "sdk-370")]
    pub fn sys_prx_load_module_list_on_memcontainer(
        count: i32,
        paths_addr: *const u32,
        container: u32,
        flags: u64,
        option: *mut SysPrxLoadModuleListOption,
        ids: *mut SysPrxId,
    ) -> i32;
    pub fn sys_prx_start_module(
        id: SysPrxId,
        args: u32,
        argv: *mut c_void,
        result: *mut i32,
        flags: u64,
        option: *mut SysPrxStartModuleOption,
    ) -> i32;
    pub fn sys_prx_stop_module(
        id: SysPrxId,
        args: u32,
        argv: *mut c_void,
        result: *mut i32,
        flags: u64,
        option: *mut SysPrxStopModuleOption,
    ) -> i32;
    pub fn sys_prx_unload_module(
        id: SysPrxId,
        flags: u64,
        option: *const SysPrxUnloadModuleOption,
    ) -> i32;
    pub fn sys_prx_get_module_list(flags: u64, info: *mut SysPrxModuleList) -> i32;
    pub fn sys_prx_get_module_info(id: SysPrxId, flags: u64, info: *mut SysPrxModuleInfo) -> i32;
    pub fn sys_prx_get_module_id_by_name(
        name: *const u8,
        flags: u64,
        option: *mut SysPrxGetModuleIdByNameOption,
    ) -> SysPrxId;
    pub fn sys_prx_get_module_id_by_address(addr: *mut c_void) -> SysPrxId;
    pub fn sys_prx_get_my_module_id() -> SysPrxId;
    pub fn sys_prx_register_library(libent: *mut c_void) -> i32;
    pub fn sys_prx_unregister_library(libent: *mut c_void) -> i32;
}
