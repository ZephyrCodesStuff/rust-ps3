//! Public memory mapper functions from the SDK's sysPrxForUser library.

const FREE_SHARED_MEMORY: u64 = 329;
const ALLOCATE_SHARED_MEMORY: u64 = 332;
const MAP_SHARED_MEMORY: u64 = 334;
const UNMAP_SHARED_MEMORY: u64 = 335;
const ALLOCATE_SHARED_MEMORY_FROM_CONTAINER: u64 = 362;

// The SDK wrappers use the private allocation key for process-local objects.
const PRIVATE_MEMORY_KEY: u64 = 0xffff_0000_0000_0000;

/// Allocates a memory object that can be mapped into a reserved address range.
///
/// # Safety
///
/// `mem_id` must be writable. `size` and `flags` must describe a valid heap
/// page allocation.
pub unsafe fn sys_mmapper_allocate_memory(size: u32, flags: u64, mem_id: *mut u32) -> i32 {
    invoke!(
        ALLOCATE_SHARED_MEMORY,
        PRIVATE_MEMORY_KEY,
        size,
        flags,
        mem_id
    ) as i32
}

/// Allocates a mappable memory object from a memory container.
///
/// # Safety
///
/// `container` must identify a valid memory container. `mem_id` must be
/// writable, and `size` and `flags` must satisfy the page requirements.
pub unsafe fn sys_mmapper_allocate_memory_from_container(
    size: u32,
    container: u32,
    flags: u64,
    mem_id: *mut u32,
) -> i32 {
    invoke!(
        ALLOCATE_SHARED_MEMORY_FROM_CONTAINER,
        PRIVATE_MEMORY_KEY,
        size,
        container,
        flags,
        mem_id
    ) as i32
}

/// Frees an unmapped memory object.
///
/// # Safety
///
/// `mem_id` must identify an owned, live memory object with no active maps.
pub unsafe fn sys_mmapper_free_memory(mem_id: u32) -> i32 {
    invoke!(FREE_SHARED_MEMORY, mem_id) as i32
}

/// Maps a memory object at the base of a reserved range.
///
/// # Safety
///
/// `start_addr` must describe a reserved range large enough for `mem_id`.
/// The caller must serialize updates to the mapping and uphold Rust aliasing
/// requirements for any references created through the new mapping.
pub unsafe fn sys_mmapper_map_memory(start_addr: u32, mem_id: u32, flags: u64) -> i32 {
    invoke!(MAP_SHARED_MEMORY, start_addr, mem_id, flags) as i32
}

/// Removes a mapping and returns the mapped memory object's ID.
///
/// # Safety
///
/// `start_addr` must identify a live mapping and `mem_id` must be writable.
/// No references may be used through the range after it is unmapped.
pub unsafe fn sys_mmapper_unmap_memory(start_addr: u32, mem_id: *mut u32) -> i32 {
    invoke!(UNMAP_SHARED_MEMORY, start_addr, mem_id) as i32
}
