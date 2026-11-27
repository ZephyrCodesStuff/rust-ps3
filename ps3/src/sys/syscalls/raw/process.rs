/// Memory object class used by the process object enumeration calls.
pub const SYS_MEM_OBJECT: u32 = 0x08;
pub const SYS_MUTEX_OBJECT: u32 = 0x85;
pub const SYS_COND_OBJECT: u32 = 0x86;
pub const SYS_RWLOCK_OBJECT: u32 = 0x88;
pub const SYS_INTR_TAG_OBJECT: u32 = 0x0a;
pub const SYS_INTR_SERVICE_HANDLE_OBJECT: u32 = 0x0b;
pub const SYS_EVENT_QUEUE_OBJECT: u32 = 0x8d;
pub const SYS_EVENT_PORT_OBJECT: u32 = 0x0e;
pub const SYS_TRACE_OBJECT: u32 = 0x21;
pub const SYS_SPUIMAGE_OBJECT: u32 = 0x22;
pub const SYS_PRX_OBJECT: u32 = 0x23;
pub const SYS_SPUPORT_OBJECT: u32 = 0x24;
pub const SYS_LWMUTEX_OBJECT: u32 = 0x95;
pub const SYS_TIMER_OBJECT: u32 = 0x11;
pub const SYS_SEMAPHORE_OBJECT: u32 = 0x96;
pub const SYS_FS_FD_OBJECT: u32 = 0x73;
pub const SYS_LWCOND_OBJECT: u32 = 0x97;
pub const SYS_EVENT_FLAG_OBJECT: u32 = 0x98;

pub const SYS_PROCESS_PRIMARY_STACK_SIZE_32K: u64 = 0x10;
pub const SYS_PROCESS_PRIMARY_STACK_SIZE_64K: u64 = 0x20;
pub const SYS_PROCESS_PRIMARY_STACK_SIZE_96K: u64 = 0x30;
pub const SYS_PROCESS_PRIMARY_STACK_SIZE_128K: u64 = 0x40;
pub const SYS_PROCESS_PRIMARY_STACK_SIZE_256K: u64 = 0x50;
pub const SYS_PROCESS_PRIMARY_STACK_SIZE_512K: u64 = 0x60;
pub const SYS_PROCESS_PRIMARY_STACK_SIZE_1M: u64 = 0x70;
pub const SYS_PROCESS_PARAM_MAGIC: u32 = 0x13bc_c5f6;
pub const SYS_PROCESS_PRX_PARAM_MAGIC: u32 = 0x1b43_4cec;

/// Process parameters carried by the executable's `PT_SCE_PROC_PARAM`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SysProcessParam {
    pub size: u32,
    pub magic: u32,
    pub version: u32,
    pub sdk_version: u32,
    pub primary_priority: i32,
    pub primary_stack_size: u32,
    pub malloc_page_size: u32,
    pub ppc_segment: u32,
}

assert_layout!(
    SysProcessParam,
    32,
    4,
    primary_priority = 16,
    ppc_segment = 28
);

/// Process import and export bounds in the executable's `PT_SCE_PROC_PRX_PARAM`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SysProcessPrxParam {
    pub size: u32,
    pub magic: u32,
    pub version: u32,
    pub reserved1: u32,
    pub exports_start: u32,
    pub exports_end: u32,
    pub imports_start: u32,
    pub imports_end: u32,
    pub library_version: u16,
    pub reserved2: u16,
    pub reserved3: u32,
}

assert_layout!(
    SysProcessPrxParam,
    40,
    4,
    exports_start = 16,
    library_version = 32
);

define_syscalls! {
    pub unsafe fn sys_process_getpid() -> u32 = SYS_PROCESS_GETPID;
    pub unsafe fn sys_process_getppid() -> u32 = SYS_PROCESS_GETPPID;
    /// Reads the current process's 32-bit PPU GUID.
    #[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
    pub unsafe fn sys_process_get_ppu_guid() -> u32 = SYS_PROCESS_GET_PPU_GUID;

    pub unsafe fn sys_process_get_number_of_object(object: u32, nump: *mut u32) -> i32 = SYS_PROCESS_GET_NUMBER_OF_OBJECT;
    /// `capacity` is a positive count of writable ID slots in `buffer`.
    pub unsafe fn sys_process_get_id(object: u32, buffer: *mut u32, capacity: u32, set_size: *mut u32) -> i32 = SYS_PROCESS_GET_ID;
    pub unsafe fn sys_process_is_spu_lock_line_reservation_address(addr: u32, flags: u64) -> i32 = SYS_PROCESS_IS_SPU_LOCK_LINE_RESERVATION_ADDRESS;
}

define_syscall_actions! {}
