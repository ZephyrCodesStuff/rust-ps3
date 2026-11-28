//! User-space synchronization and lightweight LV2 synchronization objects.

use crate::sys::raw::{SysLwcond, SysLwmutex, SYS_SYNC_NOT_RECURSIVE, SYS_SYNC_PRIORITY};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SysLwmutexAttribute {
    pub protocol: u32,
    pub recursive: u32,
    pub name: [u8; 8],
}

impl Default for SysLwmutexAttribute {
    fn default() -> Self {
        Self {
            protocol: SYS_SYNC_PRIORITY,
            recursive: SYS_SYNC_NOT_RECURSIVE,
            name: [0; 8],
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SysLwcondAttribute {
    pub name: [u8; 8],
}

const _: () = {
    use core::mem::{align_of, offset_of, size_of};
    assert!(size_of::<SysLwmutexAttribute>() == 16);
    assert!(align_of::<SysLwmutexAttribute>() == 4);
    assert!(offset_of!(SysLwmutexAttribute, name) == 8);
    assert!(size_of::<SysLwcondAttribute>() == 8);
    assert!(align_of::<SysLwcondAttribute>() == 1);
};

extern "C" {
    pub fn sys_lwmutex_create(mutex: *mut SysLwmutex, attr: *mut SysLwmutexAttribute) -> i32;
    pub fn sys_lwmutex_destroy(mutex: *mut SysLwmutex) -> i32;
    pub fn sys_lwmutex_lock(mutex: *mut SysLwmutex, timeout: u64) -> i32;
    pub fn sys_lwmutex_trylock(mutex: *mut SysLwmutex) -> i32;
    pub fn sys_lwmutex_unlock(mutex: *mut SysLwmutex) -> i32;

    pub fn sys_lwcond_create(
        cond: *mut SysLwcond,
        mutex: *mut SysLwmutex,
        attr: *mut SysLwcondAttribute,
    ) -> i32;
    pub fn sys_lwcond_destroy(cond: *mut SysLwcond) -> i32;
    pub fn sys_lwcond_wait(cond: *mut SysLwcond, timeout: u64) -> i32;
    pub fn sys_lwcond_signal(cond: *mut SysLwcond) -> i32;
    pub fn sys_lwcond_signal_all(cond: *mut SysLwcond) -> i32;
    pub fn sys_lwcond_signal_to(cond: *mut SysLwcond, thread: u64) -> i32;

    pub fn sys_spinlock_initialize(lock: *mut i32);
    pub fn sys_spinlock_lock(lock: *mut i32);
    pub fn sys_spinlock_trylock(lock: *mut i32) -> i32;
    pub fn sys_spinlock_unlock(lock: *mut i32);
}
