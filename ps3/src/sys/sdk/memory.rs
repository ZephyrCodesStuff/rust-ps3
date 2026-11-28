//! Memory mapping and pool allocation from the system library.

use core::ffi::c_void;

/// ELF32 effective address of a memory pool returned by `sys_mempool_create`.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SysMempool(pub u32);

/// Header stored at the start of a pool's backing memory.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SysMempoolHeader {
    pub magic: u64,
    pub free_count: u64,
    pub chunk_size: u32,
    pub first_free_block_addr: u32,
    pub block_size: u32,
    pub offset: u32,
    pub mutex_id: u32,
    pub cond_id: u32,
}

const _: () = {
    use core::mem::{align_of, offset_of, size_of};
    assert!(size_of::<SysMempool>() == 4);
    assert!(align_of::<SysMempool>() == 4);
    assert!(size_of::<SysMempoolHeader>() == 40);
    assert!(align_of::<SysMempoolHeader>() == 8);
    assert!(offset_of!(SysMempoolHeader, first_free_block_addr) == 20);
    assert!(offset_of!(SysMempoolHeader, mutex_id) == 32);
};

extern "C" {
    pub fn sys_mmapper_allocate_memory(size: u32, flags: u64, mem_id: *mut u32) -> i32;
    pub fn sys_mmapper_allocate_memory_from_container(
        size: u32,
        container: u32,
        flags: u64,
        mem_id: *mut u32,
    ) -> i32;
    pub fn sys_mmapper_map_memory(start_addr: u32, mem_id: u32, flags: u64) -> i32;
    pub fn sys_mmapper_unmap_memory(start_addr: u32, mem_id: *mut u32) -> i32;
    pub fn sys_mmapper_free_memory(mem_id: u32) -> i32;

    pub fn sys_mempool_create(
        pool: *mut SysMempool,
        chunk: *mut c_void,
        chunk_size: u64,
        block_size: u64,
        alignment: u64,
    ) -> i32;
    pub fn sys_mempool_allocate_block(pool: SysMempool) -> *mut c_void;
    pub fn sys_mempool_try_allocate_block(pool: SysMempool) -> *mut c_void;
    pub fn sys_mempool_free_block(pool: SysMempool, block: *mut c_void) -> i32;
    pub fn sys_mempool_destroy(pool: SysMempool);
    pub fn sys_mempool_get_count(pool: SysMempool) -> u64;
}
