//! Older public selector variants that coexist with the SDK 3.70 variants on
//! later firmware. These calls can be used with either SDK feature.

const MEMORY_CONTAINER_CREATE: u64 = 341;
const MEMORY_CONTAINER_DESTROY: u64 = 342;
const RWLOCK_TRYWLOCK: u64 = 126;

/// Creates a memory container using the public SDK 1.92 selector.
///
/// # Safety
///
/// The firmware must implement selector 341. `container` must be writable.
/// Destroy the resulting container with [`sys_memory_container_destroy`].
#[inline]
pub unsafe fn sys_memory_container_create(container: *mut u32, size: u32) -> i32 {
    invoke!(MEMORY_CONTAINER_CREATE, container, size) as i32
}

/// Destroys a memory container created with [`sys_memory_container_create`].
///
/// # Safety
///
/// The firmware must implement selector 342. The container must remain
/// valid throughout the call and have no outstanding allocations.
#[inline]
pub unsafe fn sys_memory_container_destroy(container: u32) -> i32 {
    invoke!(MEMORY_CONTAINER_DESTROY, container) as i32
}

/// Attempts to acquire a write lock using the public SDK 1.92 selector.
///
/// # Safety
///
/// The firmware must implement selector 126 and `lock` must identify a
/// valid read/write lock for the duration of the call.
#[inline]
pub unsafe fn sys_rwlock_trywlock(lock: u32) -> i32 {
    invoke!(RWLOCK_TRYWLOCK, lock) as i32
}
