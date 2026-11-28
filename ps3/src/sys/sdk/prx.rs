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
#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
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
#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
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

/// Extended module metadata from SDK 4.60.
#[cfg(feature = "sdk-370")]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SysPrxModuleInfoV2 {
    pub module: SysPrxModuleInfo,
    /// Effective address of the module's `.lib.ent` section.
    pub exports_addr: u32,
    pub exports_size: u32,
    /// Effective address of the module's `.lib.stub` section.
    pub imports_addr: u32,
    pub imports_size: u32,
}

#[cfg(feature = "sdk-370")]
impl Default for SysPrxModuleInfoV2 {
    fn default() -> Self {
        Self {
            module: SysPrxModuleInfo {
                size: core::mem::size_of::<Self>() as u64,
                ..Default::default()
            },
            exports_addr: 0,
            exports_size: 0,
            imports_addr: 0,
            imports_size: 0,
        }
    }
}

/// ELF32 library export record in a module's `.lib.ent` section.
#[cfg(feature = "sdk-370")]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SysPrxLibraryExport {
    pub size: u8,
    pub reserved1: u8,
    pub version: u16,
    pub attributes: u16,
    pub function_count: u16,
    pub variable_count: u16,
    pub tls_count: u16,
    pub hash_info: u8,
    pub tls_hash_info: u8,
    pub reserved2: u8,
    pub alternate_nid_sets: u8,
    pub name_addr: u32,
    pub nid_table_addr: u32,
    pub address_table_addr: u32,
}

/// ELF32 library import record in a module's `.lib.stub` section.
#[cfg(feature = "sdk-370")]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SysPrxLibraryImport {
    pub size: u8,
    pub reserved1: u8,
    pub version: u16,
    pub attributes: u16,
    pub function_count: u16,
    pub variable_count: u16,
    pub tls_count: u16,
    pub reserved2: [u8; 4],
    pub name_addr: u32,
    pub function_nid_table_addr: u32,
    pub function_table_addr: u32,
    pub variable_nid_table_addr: u32,
    pub variable_table_addr: u32,
    pub tls_nid_table_addr: u32,
    pub tls_table_addr: u32,
}

/// ELF32 relocation entry for an imported variable.
#[cfg(feature = "sdk-370")]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SysPrxVariableRelocation {
    pub kind: u32,
    pub address: u32,
    pub addend: u32,
}

#[cfg(feature = "sdk-370")]
const _: () = {
    use core::mem::{align_of, offset_of, size_of};
    assert!(size_of::<SysPrxModuleInfoV2>() == 88);
    assert!(align_of::<SysPrxModuleInfoV2>() == 8);
    assert!(offset_of!(SysPrxModuleInfoV2, exports_addr) == 72);
    assert!(offset_of!(SysPrxModuleInfoV2, imports_addr) == 80);
    assert!(size_of::<SysPrxLibraryExport>() == 28);
    assert!(align_of::<SysPrxLibraryExport>() == 4);
    assert!(offset_of!(SysPrxLibraryExport, name_addr) == 16);
    assert!(size_of::<SysPrxLibraryImport>() == 44);
    assert!(align_of::<SysPrxLibraryImport>() == 4);
    assert!(offset_of!(SysPrxLibraryImport, name_addr) == 16);
    assert!(size_of::<SysPrxVariableRelocation>() == 12);
    assert!(align_of::<SysPrxVariableRelocation>() == 4);
};

/// Queries the extended PRX module information on supporting firmware.
///
/// # Safety
///
/// The effective addresses and lengths in `info.module` must describe valid
/// writable buffers or omit the corresponding output. Any returned section
/// addresses are valid only while the queried module remains loaded.
#[cfg(feature = "sdk-370")]
pub unsafe fn sys_prx_get_module_info_v2(
    id: SysPrxId,
    flags: u64,
    info: &mut SysPrxModuleInfoV2,
) -> i32 {
    info.module.size = core::mem::size_of::<SysPrxModuleInfoV2>() as u64;
    sys_prx_get_module_info(id, flags, core::ptr::from_mut(info).cast())
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
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub fn sys_prx_load_module_list(
        count: i32,
        paths_addr: *const u32,
        flags: u64,
        option: *mut SysPrxLoadModuleListOption,
        ids: *mut SysPrxId,
    ) -> i32;
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
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
