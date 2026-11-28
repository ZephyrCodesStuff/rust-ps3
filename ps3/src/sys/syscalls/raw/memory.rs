pub const SYS_MEMORY_CONTAINER_ID_INVALID: u32 = u32::MAX;
pub const SYS_MEMORY_ACCESS_RIGHT_NONE: u64 = 0xf0;
pub const SYS_MEMORY_ACCESS_RIGHT_ANY: u64 = 0x0f;
pub const SYS_MEMORY_ACCESS_RIGHT_PPU_THR: u64 = 0x08;
pub const SYS_MEMORY_ACCESS_RIGHT_HANDLER: u64 = 0x04;
pub const SYS_MEMORY_ACCESS_RIGHT_SPU_THR: u64 = 0x02;
pub const SYS_MEMORY_ACCESS_RIGHT_RAW_SPU: u64 = 0x01;
pub const SYS_MEMORY_ACCESS_RIGHT_MASK: u64 = 0xff;
pub const SYS_MEMORY_PAGE_SIZE_4K: u64 = 0x100;
pub const SYS_MEMORY_PAGE_SIZE_MASK: u64 = 0xf00;
pub const SYS_MEMORY_ATTR_READ_WRITE: u64 = 0x40000;
pub const SYS_MEMORY_ATTR_READ_ONLY: u64 = 0x80000;
pub const SYS_MEMORY_PROT_MASK: u64 = 0xf0000;

pub use crate::sys::types::{SYS_MEMORY_PAGE_SIZE_1M, SYS_MEMORY_PAGE_SIZE_64K};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SysMemoryInfo {
    pub total_user_memory: u32,
    pub available_user_memory: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SysPageAttribute {
    pub attribute: u64,
    pub access_right: u64,
    pub page_size: u32,
    pub reserved: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SysVmStatistics {
    pub page_fault_ppu: u64,
    pub page_fault_spu: u64,
    pub page_in: u64,
    pub page_out: u64,
    pub pmem_total: u32,
    pub pmem_used: u32,
    pub timestamp: u64,
}

assert_layout!(SysMemoryInfo, 8, 4, available_user_memory = 4);
assert_layout!(SysPageAttribute, 24, 8, page_size = 16);
assert_layout!(SysVmStatistics, 48, 8, timestamp = 40);

define_syscalls! {
    pub unsafe fn sys_memory_allocate(size: u32, flags: u64, alloc_addr: *mut u32) -> i32 = SYS_MEMORY_ALLOCATE;
    pub unsafe fn sys_memory_allocate_from_container(size: u32, cid: u32, flags: u64, alloc_addr: *mut u32) -> i32 = SYS_MEMORY_ALLOCATE_FROM_CONTAINER;
    pub unsafe fn sys_memory_free(start_addr: u32) -> i32 = SYS_MEMORY_FREE;
    pub unsafe fn sys_memory_get_page_attribute(addr: u32, attr: *mut SysPageAttribute) -> i32 = SYS_MEMORY_GET_PAGE_ATTRIBUTE;
    pub unsafe fn sys_memory_get_user_memory_size(mem_info: *mut SysMemoryInfo) -> i32 = SYS_MEMORY_GET_USER_MEMORY_SIZE;
    /// Creates a memory container.
    pub unsafe fn sys_memory_container_create(cid: *mut u32, size: u32) -> i32 = SYS_MEMORY_CONTAINER_CREATE;
    /// Destroys a memory container.
    pub unsafe fn sys_memory_container_destroy(cid: u32) -> i32 = SYS_MEMORY_CONTAINER_DESTROY;
    pub unsafe fn sys_memory_container_get_size(mem_info: *mut SysMemoryInfo, cid: u32) -> i32 = SYS_MEMORY_CONTAINER_GET_SIZE;
    pub unsafe fn sys_mmapper_allocate_address(size: u32, flags: u64, alignment: u32, alloc_addr: *mut u32) -> i32 = SYS_MMAPPER_ALLOCATE_ADDRESS;
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub unsafe fn sys_mmapper_allocate_fixed_address() -> i32 = SYS_MMAPPER_ALLOCATE_FIXED_ADDRESS;
    pub unsafe fn sys_mmapper_change_address_access_right(addr: u32, flags: u64) -> i32 = SYS_MMAPPER_CHANGE_ADDRESS_ACCESS_RIGHT;
    pub unsafe fn sys_mmapper_free_address(addr: u32) -> i32 = SYS_MMAPPER_FREE_ADDRESS;
    pub unsafe fn sys_mmapper_search_and_map(start_addr: u32, mem_id: u32, flags: u64, alloc_addr: *mut u32) -> i32 = SYS_MMAPPER_SEARCH_AND_MAP;
    pub unsafe fn sys_mmapper_enable_page_fault_notification(start_addr: u32, event_queue_id: u32) -> i32 = SYS_MMAPPER_ENABLE_PAGE_FAULT_NOTIFICATION;
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub unsafe fn sys_vm_memory_map(vsize: u32, psize: u32, cid: u32, flag: u64, policy: u64, addr: *mut u32) -> i32 = SYS_VM_MEMORY_MAP;
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub unsafe fn sys_vm_unmap(addr: u32) -> i32 = SYS_VM_UNMAP;
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub unsafe fn sys_vm_append_memory(addr: u32, size: u32) -> i32 = SYS_VM_APPEND_MEMORY;
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub unsafe fn sys_vm_return_memory(addr: u32, size: u32) -> i32 = SYS_VM_RETURN_MEMORY;
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub unsafe fn sys_vm_lock(addr: u32, size: u32) -> i32 = SYS_VM_LOCK;
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub unsafe fn sys_vm_unlock(addr: u32, size: u32) -> i32 = SYS_VM_UNLOCK;
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub unsafe fn sys_vm_touch(addr: u32, size: u32) -> i32 = SYS_VM_TOUCH;
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub unsafe fn sys_vm_flush(addr: u32, size: u32) -> i32 = SYS_VM_FLUSH;
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub unsafe fn sys_vm_invalidate(addr: u32, size: u32) -> i32 = SYS_VM_INVALIDATE;
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub unsafe fn sys_vm_store(addr: u32, size: u32) -> i32 = SYS_VM_STORE;
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub unsafe fn sys_vm_sync(addr: u32, size: u32) -> i32 = SYS_VM_SYNC;
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub unsafe fn sys_vm_test(addr: u32, size: u32, result: *mut u64) -> i32 = SYS_VM_TEST;
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub unsafe fn sys_vm_get_statistics(addr: u32, stat: *mut SysVmStatistics) -> i32 = SYS_VM_GET_STATISTICS;
}
