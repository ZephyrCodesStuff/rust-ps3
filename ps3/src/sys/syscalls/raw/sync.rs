use super::super::nr;

pub const SYS_SYNC_FIFO: u32 = 0x1;
pub const SYS_SYNC_PRIORITY: u32 = 0x2;
pub const SYS_SYNC_PRIORITY_INHERIT: u32 = 0x3;
pub const SYS_SYNC_RETRY: u32 = 0x4;
pub const SYS_SYNC_RECURSIVE: u32 = 0x10;
pub const SYS_SYNC_NOT_RECURSIVE: u32 = 0x20;
pub const SYS_SYNC_PROCESS_SHARED: u32 = 0x100;
pub const SYS_SYNC_NOT_PROCESS_SHARED: u32 = 0x200;
pub const SYS_SYNC_NEWLY_CREATED: i32 = 0x1;
pub const SYS_SYNC_NOT_CREATE: i32 = 0x2;
pub const SYS_SYNC_NOT_CARE: i32 = 0x3;
pub const SYS_SYNC_ADAPTIVE: u32 = 0x1000;
pub const SYS_SYNC_NOT_ADAPTIVE: u32 = 0x2000;
pub const SYS_SYNC_WAITER_SINGLE: i32 = 0x10000;
pub const SYS_SYNC_WAITER_MULTIPLE: i32 = 0x20000;
pub const SYS_EVENT_FLAG_WAIT_AND: u32 = 0x01;
pub const SYS_EVENT_FLAG_WAIT_OR: u32 = 0x02;
pub const SYS_EVENT_FLAG_WAIT_CLEAR: u32 = 0x10;
pub const SYS_EVENT_FLAG_WAIT_CLEAR_ALL: u32 = 0x20;
pub const SYS_PPU_QUEUE: i32 = 1;
pub const SYS_SPU_QUEUE: i32 = 2;
pub const SYS_EVENT_QUEUE_DESTROY_FORCE: i32 = 1;
pub const SYS_EVENT_QUEUE_LOCAL: u64 = 0;
pub const SYS_EVENT_PORT_LOCAL: i32 = 1;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SysMutexAttribute {
    pub protocol: u32,
    pub recursive: u32,
    pub pshared: u32,
    pub adaptive: u32,
    pub ipc_key: u64,
    pub flags: i32,
    pub reserved: u32,
    pub name: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SysCondAttribute {
    pub pshared: u32,
    pub flags: i32,
    pub ipc_key: u64,
    pub name: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SysSemaphoreAttribute {
    pub protocol: u32,
    pub pshared: u32,
    pub ipc_key: u64,
    pub flags: i32,
    pub reserved: u32,
    pub name: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SysRwlockAttribute {
    pub protocol: u32,
    pub pshared: u32,
    pub ipc_key: u64,
    pub flags: i32,
    pub reserved: u32,
    pub name: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SysEventFlagAttribute {
    pub protocol: u32,
    pub pshared: u32,
    pub ipc_key: u64,
    pub flags: i32,
    pub kind: i32,
    pub name: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SysEventQueueAttribute {
    pub protocol: u32,
    pub kind: i32,
    pub name: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SysEvent {
    pub source: u64,
    pub data1: u64,
    pub data2: u64,
    pub data3: u64,
}

#[repr(C, align(8))]
pub struct SysLwmutex {
    pub owner: u32,
    pub waiter: u32,
    pub attribute: u32,
    pub recursive_count: u32,
    pub sleep_queue: u32,
    pub reserved: u32,
}

#[repr(C)]
pub struct SysLwcond {
    pub lwmutex_addr: u32,
    pub queue: u32,
}

assert_layout!(SysMutexAttribute, 40, 8, ipc_key = 16, name = 32);
assert_layout!(SysCondAttribute, 24, 8, name = 16);
assert_layout!(SysSemaphoreAttribute, 32, 8, name = 24);
assert_layout!(SysRwlockAttribute, 32, 8, name = 24);
assert_layout!(SysEventFlagAttribute, 32, 8, name = 24);
assert_layout!(SysEventQueueAttribute, 16, 4, name = 8);
assert_layout!(SysEvent, 32, 8);
assert_layout!(SysLwmutex, 24, 8, sleep_queue = 16);
assert_layout!(SysLwcond, 8, 4);

/// Receives one event. LV2 returns the event words in r4 through r7.
///
/// # Safety
///
/// `event` must be writable for one `SysEvent`. `queue` must identify a PPU
/// event queue valid for the duration of the call.
#[inline]
pub unsafe fn sys_event_queue_receive(queue: u32, event: *mut SysEvent, timeout: u64) -> i32 {
    let result: i64;
    let (source, data1, data2, data3);
    unsafe {
        lv2_trap!(nr::SYS_EVENT_QUEUE_RECEIVE;
            inlateout("r3") u64::from(queue) => result,
            inlateout("r4") (event as usize as u64) => source,
            inlateout("r5") timeout => data1,
            lateout("r6") data2,
            lateout("r7") data3,
            lateout("r8") _,
            lateout("r9") _,
            lateout("r10") _,
        );
        if result as i32 == 0 {
            event.write(SysEvent {
                source,
                data1,
                data2,
                data3,
            });
        }
    }
    result as i32
}

define_syscalls! {
    pub unsafe fn sys_mutex_create(mutex_id: *mut u32, attr: *const SysMutexAttribute) -> i32 = SYS_MUTEX_CREATE;
    pub unsafe fn sys_mutex_destroy(mutex_id: u32) -> i32 = SYS_MUTEX_DESTROY;
    pub unsafe fn sys_mutex_lock(mutex_id: u32, timeout: u64) -> i32 = SYS_MUTEX_LOCK;
    pub unsafe fn sys_mutex_trylock(mutex_id: u32) -> i32 = SYS_MUTEX_TRYLOCK;
    pub unsafe fn sys_mutex_unlock(mutex_id: u32) -> i32 = SYS_MUTEX_UNLOCK;
    pub unsafe fn sys_cond_create(cond_id: *mut u32, mutex_id: u32, attr: *const SysCondAttribute) -> i32 = SYS_COND_CREATE;
    pub unsafe fn sys_cond_destroy(cond_id: u32) -> i32 = SYS_COND_DESTROY;
    pub unsafe fn sys_cond_wait(cond_id: u32, timeout: u64) -> i32 = SYS_COND_WAIT;
    pub unsafe fn sys_cond_signal(cond_id: u32) -> i32 = SYS_COND_SIGNAL;
    pub unsafe fn sys_cond_signal_all(cond_id: u32) -> i32 = SYS_COND_SIGNAL_ALL;
    pub unsafe fn sys_cond_signal_to(cond_id: u32, thread_id: u64) -> i32 = SYS_COND_SIGNAL_TO;
    pub unsafe fn sys_rwlock_create(rw_lock_id: *mut u32, attr: *const SysRwlockAttribute) -> i32 = SYS_RWLOCK_CREATE;
    pub unsafe fn sys_rwlock_destroy(rw_lock_id: u32) -> i32 = SYS_RWLOCK_DESTROY;
    pub unsafe fn sys_rwlock_rlock(rw_lock_id: u32, timeout: u64) -> i32 = SYS_RWLOCK_RLOCK;
    pub unsafe fn sys_rwlock_tryrlock(rw_lock_id: u32) -> i32 = SYS_RWLOCK_TRYRLOCK;
    pub unsafe fn sys_rwlock_runlock(rw_lock_id: u32) -> i32 = SYS_RWLOCK_RUNLOCK;
    pub unsafe fn sys_rwlock_wlock(rw_lock_id: u32, timeout: u64) -> i32 = SYS_RWLOCK_WLOCK;
    pub unsafe fn sys_rwlock_trywlock(rw_lock_id: u32) -> i32 = SYS_RWLOCK_TRYWLOCK;
    pub unsafe fn sys_rwlock_wunlock(rw_lock_id: u32) -> i32 = SYS_RWLOCK_WUNLOCK;
    pub unsafe fn sys_semaphore_create(sem_id: *mut u32, attr: *const SysSemaphoreAttribute, initial_val: i32, max_val: i32) -> i32 = SYS_SEMAPHORE_CREATE;
    pub unsafe fn sys_semaphore_destroy(sem_id: u32) -> i32 = SYS_SEMAPHORE_DESTROY;
    pub unsafe fn sys_semaphore_wait(sem_id: u32, timeout: u64) -> i32 = SYS_SEMAPHORE_WAIT;
    pub unsafe fn sys_semaphore_trywait(sem_id: u32) -> i32 = SYS_SEMAPHORE_TRYWAIT;
    pub unsafe fn sys_semaphore_post(sem_id: u32, count: i32) -> i32 = SYS_SEMAPHORE_POST;
    pub unsafe fn sys_semaphore_get_value(sem_id: u32, count: *mut i32) -> i32 = SYS_SEMAPHORE_GET_VALUE;
    pub unsafe fn sys_event_queue_create(equeue_id: *mut u32, attr: *const SysEventQueueAttribute, event_queue_key: u64, size: i32) -> i32 = SYS_EVENT_QUEUE_CREATE;
    pub unsafe fn sys_event_queue_destroy(equeue_id: u32, mode: i32) -> i32 = SYS_EVENT_QUEUE_DESTROY;
    pub unsafe fn sys_event_queue_tryreceive(equeue_id: u32, event_array: *mut SysEvent, size: i32, number: *mut i32) -> i32 = SYS_EVENT_QUEUE_TRYRECEIVE;
    pub unsafe fn sys_event_queue_drain(event_queue_id: u32) -> i32 = SYS_EVENT_QUEUE_DRAIN;
    pub unsafe fn sys_event_port_create(eport_id: *mut u32, port_type: i32, name: u64) -> i32 = SYS_EVENT_PORT_CREATE;
    pub unsafe fn sys_event_port_destroy(eport_id: u32) -> i32 = SYS_EVENT_PORT_DESTROY;
    pub unsafe fn sys_event_port_connect_local(event_port_id: u32, event_queue_id: u32) -> i32 = SYS_EVENT_PORT_CONNECT_LOCAL;
    pub unsafe fn sys_event_port_disconnect(eport_id: u32) -> i32 = SYS_EVENT_PORT_DISCONNECT;
    pub unsafe fn sys_event_port_send(event_port_id: u32, data1: u64, data2: u64, data3: u64) -> i32 = SYS_EVENT_PORT_SEND;
    #[cfg(feature = "sdk-370")]
    pub unsafe fn sys_event_flag_create(id: *mut u32, attr: *const SysEventFlagAttribute, init: u64) -> i32 = SYS_EVENT_FLAG_CREATE;
    #[cfg(feature = "sdk-370")]
    pub unsafe fn sys_event_flag_destroy(id: u32) -> i32 = SYS_EVENT_FLAG_DESTROY;
    #[cfg(feature = "sdk-370")]
    pub unsafe fn sys_event_flag_wait(id: u32, bitptn: u64, mode: u32, result: *mut u64, timeout: u64) -> i32 = SYS_EVENT_FLAG_WAIT;
    #[cfg(feature = "sdk-370")]
    pub unsafe fn sys_event_flag_trywait(id: u32, bitptn: u64, mode: u32, result: *mut u64) -> i32 = SYS_EVENT_FLAG_TRYWAIT;
    #[cfg(feature = "sdk-370")]
    pub unsafe fn sys_event_flag_set(id: u32, bitptn: u64) -> i32 = SYS_EVENT_FLAG_SET;
    #[cfg(feature = "sdk-370")]
    pub unsafe fn sys_event_flag_clear(id: u32, bitptn: u64) -> i32 = SYS_EVENT_FLAG_CLEAR;
    #[cfg(feature = "sdk-370")]
    pub unsafe fn sys_event_flag_cancel(id: u32, num: *mut i32) -> i32 = SYS_EVENT_FLAG_CANCEL;
    #[cfg(feature = "sdk-370")]
    pub unsafe fn sys_event_flag_get(id: u32, flags: *mut u64) -> i32 = SYS_EVENT_FLAG_GET;
}
