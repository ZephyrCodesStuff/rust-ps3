//! Official `sys_lv2dbg` imports for firmware that provides the debug library.
//!
//! Addresses inside debug output records use 32-bit effective addresses.
//! Query calls that fill arrays require writable storage of the capacity
//! supplied in the corresponding input field or argument.

#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
use super::sync::SysLwcondAttribute;
use super::sync::SysLwmutexAttribute;
#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
use crate::sys::raw::SysEventFlagAttribute;
use crate::sys::raw::{
    PpuFunctionDescriptor, SysCondAttribute, SysEventQueueAttribute, SysMutexAttribute,
    SysRwlockAttribute, SysSemaphoreAttribute,
};

pub const SYS_DBG_PPU_THREAD_STOP: u64 = 0x1;
pub const SYS_DBG_SPU_THREAD_GROUP_STOP: u64 = 0x2;
#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
pub const SYS_DBG_SYSTEM_PPU_THREAD_NOT_STOP: u64 = 0x4;
#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
pub const SYS_DBG_SYSTEM_SPU_THREAD_GROUP_NOT_STOP: u64 = 0x8;
#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
pub const SYS_DBG_NOT_EXE_CTRL_BY_COREDUMP_EVENT: u64 = 0x10;
pub const SYS_DBG_PPU_EXCEPTION_TRAP: u64 = 0x1000000;
pub const SYS_DBG_PPU_EXCEPTION_PREV_INST: u64 = 0x2000000;
pub const SYS_DBG_PPU_EXCEPTION_ILL_INST: u64 = 0x4000000;
pub const SYS_DBG_PPU_EXCEPTION_TEXT_HTAB_MISS: u64 = 0x8000000;
pub const SYS_DBG_PPU_EXCEPTION_TEXT_SLB_MISS: u64 = 0x10000000;
pub const SYS_DBG_PPU_EXCEPTION_DATA_HTAB_MISS: u64 = 0x20000000;
pub const SYS_DBG_PPU_EXCEPTION_DATA_SLB_MISS: u64 = 0x40000000;
pub const SYS_DBG_PPU_EXCEPTION_FLOAT: u64 = 0x80000000;
pub const SYS_DBG_PPU_EXCEPTION_DABR_MATCH: u64 = 0x100000000;
pub const SYS_DBG_PPU_EXCEPTION_ALIGNMENT: u64 = 0x200000000;
#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
pub const SYS_DBG_PPU_EXCEPTION_DATA_MAT: u64 = 0x2000000000;
#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
pub const SYS_DBG_EVENT_CORE_DUMPED: u64 = 0x1000;
#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
pub const SYS_DBG_EVENT_PPU_EXCEPTION_HANDLER_SIGNALED: u64 = 0x2000;
#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
pub const SYS_VM_STATE_LOCKED: u64 = 0x8;
#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
pub const SYS_VM_STATE_DIRTY: u64 = 0x10;
#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
pub const SYS_DBG_DABR_CTRL_READ: u64 = 0x5;
#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
pub const SYS_DBG_DABR_CTRL_WRITE: u64 = 0x6;
#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
pub const SYS_DBG_DABR_CTRL_CLEAR: u64 = 0x0;
#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
pub const SYS_DBG_MAT_TRANSPARENT: u64 = 0x1;
#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
pub const SYS_DBG_MAT_WRITE: u64 = 0x2;
#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
pub const SYS_DBG_MAT_READ_WRITE: u64 = 0x4;
#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
pub const SYS_MAT_GRANULARITY: u32 = 0x1000;

/// The 16-byte register union used by the PPU and SPU debug contexts.
#[repr(C)]
#[derive(Clone, Copy)]
pub union SysDbgVector {
    pub bytes: [u8; 16],
    pub halfwords: [u16; 8],
    pub words: [u32; 4],
    pub doublewords: [u64; 2],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SysDbgPpuThreadContext {
    pub gpr: [u64; 32],
    pub cr: u32,
    pub xer: u64,
    pub lr: u64,
    pub ctr: u64,
    pub pc: u64,
    pub fpr: [u64; 32],
    pub fpscr: u32,
    pub vr: [SysDbgVector; 32],
    pub vscr: SysDbgVector,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SysDbgSpuThreadContext {
    pub gpr: [SysDbgVector; 128],
    pub npc: u32,
    pub fpscr: u32,
    pub srr0: u32,
    pub status: u32,
    pub configuration: u64,
    pub mailbox_status: u32,
    pub ppu_mailbox: u32,
    pub spu_mailbox: [u32; 4],
    pub decrementer: u32,
    pub mfc_command_queue: [u64; 96],
}

#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SysDbgSpuThreadContext2 {
    pub gpr: [SysDbgVector; 128],
    pub npc: u32,
    pub fpscr: SysDbgVector,
    pub srr0: u32,
    pub status: u32,
    pub configuration: u64,
    pub mailbox_status: u32,
    pub ppu_mailbox: u32,
    pub spu_mailbox: [u32; 4],
    pub decrementer: u32,
    pub mfc_command_queue: [u64; 96],
}

/// Caller-supplied storage and returned counts for waiting PPU thread IDs.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SysDbgWaitList {
    pub ids_addr: u32,
    pub count: u32,
    pub total_count: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SysDbgMutexInformation {
    pub attribute: SysMutexAttribute,
    pub owner: u64,
    pub lock_count: i32,
    pub cond_ref_count: i32,
    pub cond_id: u32,
    pub waiters: SysDbgWaitList,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SysDbgCondInformation {
    pub attribute: SysCondAttribute,
    pub mutex_id: u32,
    pub waiters: SysDbgWaitList,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SysDbgRwlockInformation {
    pub attribute: SysRwlockAttribute,
    pub owner: u64,
    pub readers: SysDbgWaitList,
    pub writers: SysDbgWaitList,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SysDbgEventQueueInformation {
    pub attribute: SysEventQueueAttribute,
    pub key: u64,
    pub queue_size: i32,
    pub waiters: SysDbgWaitList,
    pub events_addr: u32,
    pub events_count: u32,
    pub total_events_count: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SysDbgSemaphoreInformation {
    pub attribute: SysSemaphoreAttribute,
    pub max_value: i32,
    pub current_value: i32,
    pub waiters: SysDbgWaitList,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SysDbgLwmutexInformation {
    pub attribute: SysLwmutexAttribute,
    pub owner: u64,
    pub lock_count: i32,
    pub waiters: SysDbgWaitList,
}

#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SysDbgLwcondInformation {
    pub attribute: SysLwcondAttribute,
    pub mutex_addr: u32,
    pub waiters: SysDbgWaitList,
}

#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SysDbgEventFlagWaitInformation {
    pub pattern: u64,
    pub mode: u32,
}

#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SysDbgEventFlagInformation {
    pub attribute: SysEventFlagAttribute,
    pub pattern: u64,
    pub waiter_ids_addr: u32,
    pub wait_info_addr: u32,
    pub waiter_count: u32,
    pub total_waiter_count: u32,
}

#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SysVmPageInformation {
    pub state: u64,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SysDbgPpuThreadStatus(pub u32);

impl SysDbgPpuThreadStatus {
    pub const IDLE: Self = Self(0);
    pub const RUNNABLE: Self = Self(1);
    pub const ON_PROCESSOR: Self = Self(2);
    pub const SLEEPING: Self = Self(3);
    pub const STOPPED: Self = Self(4);
    pub const ZOMBIE: Self = Self(5);
    pub const DELETED: Self = Self(6);
    pub const UNKNOWN: Self = Self(7);
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SysDbgSpuThreadGroupStatus(pub u32);

impl SysDbgSpuThreadGroupStatus {
    pub const NOT_INITIALIZED: Self = Self(0);
    pub const INITIALIZED: Self = Self(1);
    pub const READY: Self = Self(2);
    pub const WAITING: Self = Self(3);
    pub const SUSPENDED: Self = Self(4);
    pub const WAITING_AND_SUSPENDED: Self = Self(5);
    pub const RUNNING: Self = Self(6);
    pub const STOPPED: Self = Self(7);
    pub const UNKNOWN: Self = Self(8);
}

#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SysDbgCoredumpParameter(pub u32);

#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
impl SysDbgCoredumpParameter {
    pub const OFF: Self = Self(0);
    pub const APP_HOME: Self = Self(1);
    pub const MEMORY_STICK: Self = Self(2);
    pub const USB: Self = Self(3);
    pub const HDD0: Self = Self(4);
}

assert_layout!(SysDbgVector, 16, 8);
assert_layout!(
    SysDbgPpuThreadContext,
    1088,
    8,
    xer = 264,
    fpr = 296,
    vr = 560,
    vscr = 1072
);
assert_layout!(
    SysDbgSpuThreadContext,
    2872,
    8,
    npc = 2048,
    configuration = 2064,
    mfc_command_queue = 2104
);
#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
assert_layout!(
    SysDbgSpuThreadContext2,
    2888,
    8,
    npc = 2048,
    fpscr = 2056,
    mfc_command_queue = 2120
);
assert_layout!(SysDbgWaitList, 12, 4);
assert_layout!(SysDbgMutexInformation, 72, 8, owner = 40, waiters = 60);
assert_layout!(SysDbgCondInformation, 40, 8, mutex_id = 24, waiters = 28);
assert_layout!(
    SysDbgRwlockInformation,
    64,
    8,
    owner = 32,
    readers = 40,
    writers = 52
);
assert_layout!(
    SysDbgEventQueueInformation,
    56,
    8,
    waiters = 28,
    events_addr = 40
);
assert_layout!(SysDbgSemaphoreInformation, 56, 8, waiters = 40);
assert_layout!(SysDbgLwmutexInformation, 40, 8, owner = 16, waiters = 28);
#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
assert_layout!(SysDbgLwcondInformation, 24, 4, waiters = 12);
#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
assert_layout!(SysDbgEventFlagWaitInformation, 16, 8, mode = 8);
#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
assert_layout!(
    SysDbgEventFlagInformation,
    56,
    8,
    pattern = 32,
    wait_info_addr = 44
);
#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
assert_layout!(SysVmPageInformation, 8, 8);
assert_layout!(SysDbgPpuThreadStatus, 4, 4);
assert_layout!(SysDbgSpuThreadGroupStatus, 4, 4);
#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
assert_layout!(SysDbgCoredumpParameter, 4, 4);

extern "C" {
    pub fn sys_dbg_read_ppu_thread_context(id: u64, context: *mut SysDbgPpuThreadContext) -> i32;
    pub fn sys_dbg_read_spu_thread_context(id: u32, context: *mut SysDbgSpuThreadContext) -> i32;
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub fn sys_dbg_read_spu_thread_context2(id: u32, context: *mut SysDbgSpuThreadContext2) -> i32;
    pub fn sys_dbg_set_stacksize_ppu_exception_handler(size: u32) -> i32;
    pub fn sys_dbg_initialize_ppu_exception_handler(priority: i32) -> i32;
    pub fn sys_dbg_finalize_ppu_exception_handler() -> i32;
    /// The callback takes three 64-bit values. SDKs 2.70 and 3.70 describe the
    /// order as exception type, thread ID, and data address. SDK 1.92
    /// documents the thread ID before the exception type.
    pub fn sys_dbg_register_ppu_exception_handler(
        callback: *const PpuFunctionDescriptor,
        flags: u64,
    ) -> i32;
    pub fn sys_dbg_unregister_ppu_exception_handler() -> i32;
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub fn sys_dbg_signal_to_ppu_exception_handler(flags: u64) -> i32;
    pub fn sys_dbg_get_mutex_information(id: u32, info: *mut SysDbgMutexInformation) -> i32;
    pub fn sys_dbg_get_cond_information(id: u32, info: *mut SysDbgCondInformation) -> i32;
    pub fn sys_dbg_get_rwlock_information(id: u32, info: *mut SysDbgRwlockInformation) -> i32;
    pub fn sys_dbg_get_event_queue_information(
        id: u32,
        info: *mut SysDbgEventQueueInformation,
    ) -> i32;
    pub fn sys_dbg_get_semaphore_information(id: u32, info: *mut SysDbgSemaphoreInformation)
        -> i32;
    pub fn sys_dbg_get_lwmutex_information(id: u32, info: *mut SysDbgLwmutexInformation) -> i32;
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub fn sys_dbg_get_lwcond_information(id: u32, info: *mut SysDbgLwcondInformation) -> i32;
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub fn sys_dbg_get_event_flag_information(
        id: u32,
        info: *mut SysDbgEventFlagInformation,
    ) -> i32;
    pub fn sys_dbg_get_ppu_thread_ids(ids: *mut u64, count: *mut u64, total: *mut u64) -> i32;
    pub fn sys_dbg_get_spu_thread_group_ids(ids: *mut u32, count: *mut u64, total: *mut u64)
        -> i32;
    pub fn sys_dbg_get_spu_thread_ids(
        group: u32,
        ids: *mut u32,
        count: *mut u64,
        total: *mut u64,
    ) -> i32;
    pub fn sys_dbg_get_ppu_thread_name(id: u64, name: *mut u8) -> i32;
    pub fn sys_dbg_get_spu_thread_name(id: u32, name: *mut u8) -> i32;
    pub fn sys_dbg_get_spu_thread_group_name(id: u32, name: *mut u8) -> i32;
    pub fn sys_dbg_get_ppu_thread_status(id: u64, status: *mut SysDbgPpuThreadStatus) -> i32;
    pub fn sys_dbg_get_spu_thread_group_status(
        id: u32,
        status: *mut SysDbgSpuThreadGroupStatus,
    ) -> i32;
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub fn sys_dbg_enable_floating_point_enabled_exception(
        id: u64,
        flags: u64,
        opt1: u64,
        opt2: u64,
    ) -> i32;
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub fn sys_dbg_disable_floating_point_enabled_exception(
        id: u64,
        flags: u64,
        opt1: u64,
        opt2: u64,
    ) -> i32;
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub fn sys_dbg_vm_get_page_information(
        addr: u32,
        count: u32,
        info: *mut SysVmPageInformation,
    ) -> i32;
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub fn sys_dbg_set_address_to_dabr(addr: u64, flags: u64) -> i32;
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub fn sys_dbg_get_address_from_dabr(addr: *mut u64, flags: *mut u64) -> i32;
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub fn sys_dbg_signal_to_coredump_handler(data1: u64, data2: u64, data3: u64) -> i32;
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub fn sys_dbg_mat_set_condition(addr: u32, condition: u64) -> i32;
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub fn sys_dbg_mat_get_condition(addr: u32, condition: *mut u64) -> i32;
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub fn sys_dbg_get_coredump_params(params: *mut SysDbgCoredumpParameter) -> i32;
    #[cfg(feature = "sdk-370")]
    pub fn sys_dbg_set_mask_to_ppu_exception_handler(mask: u64, flags: u64) -> i32;
}
