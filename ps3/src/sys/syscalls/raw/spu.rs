use core::ffi::c_void;

pub const SYS_SPU_THREAD_GROUP_TYPE_NORMAL: i32 = 0;
pub const SYS_SPU_THREAD_GROUP_TYPE_SEQUENTIAL: i32 = 0x01;
pub const SYS_SPU_THREAD_GROUP_TYPE_SYSTEM: i32 = 0x02;
pub const SYS_SPU_THREAD_GROUP_TYPE_MEMORY_FROM_CONTAINER: i32 = 0x04;
pub const SYS_SPU_THREAD_GROUP_TYPE_NON_CONTEXT: i32 = 0x08;
pub const SYS_SPU_THREAD_GROUP_TYPE_EXCLUSIVE_NON_CONTEXT: i32 = 0x18;
pub const SYS_SPU_THREAD_GROUP_TYPE_COOPERATE_WITH_SYSTEM: i32 = 0x20;

pub const SYS_SPU_THREAD_GROUP_JOIN_GROUP_EXIT: i32 = 1;
pub const SYS_SPU_THREAD_GROUP_JOIN_ALL_THREADS_EXIT: i32 = 2;
pub const SYS_SPU_THREAD_GROUP_JOIN_TERMINATED: i32 = 4;

pub const SYS_SPU_THREAD_GROUP_EVENT_RUN: u32 = 1;
pub const SYS_SPU_THREAD_GROUP_EVENT_EXCEPTION: u32 = 2;
pub const SYS_SPU_THREAD_GROUP_EVENT_SYSTEM_MODULE: u32 = 4;
pub const SYS_SPU_THREAD_GROUP_EVENT_RUN_KEY: u64 = 0xffff_ffff_5350_5500;
pub const SYS_SPU_THREAD_GROUP_EVENT_EXCEPTION_KEY: u64 = 0xffff_ffff_5350_5503;
pub const SYS_SPU_THREAD_GROUP_EVENT_SYSTEM_MODULE_KEY: u64 = 0xffff_ffff_5350_5504;

pub const SYS_SPU_THREAD_OPTION_NONE: u32 = 0;
pub const SYS_SPU_THREAD_OPTION_ASYNC_INTR_ENABLE: u32 = 1;
pub const SYS_SPU_THREAD_OPTION_DEC_SYNC_TB_ENABLE: u32 = 2;

pub const SYS_SPU_SEGMENT_TYPE_COPY: i32 = 1;
pub const SYS_SPU_SEGMENT_TYPE_FILL: i32 = 2;
pub const SYS_SPU_SEGMENT_TYPE_INFO: i32 = 4;
pub const SYS_SPU_IMAGE_TYPE_USER: u32 = 0;
pub const SYS_SPU_IMAGE_TYPE_KERNEL: u32 = 1;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SysSpuImage {
    pub kind: u32,
    pub entry_point: u32,
    pub segments_addr: u32,
    pub segment_count: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union SysSpuSegmentSource {
    /// Effective address used by COPY segments.
    pub address: u32,
    /// Word used by FILL segments.
    pub value: u32,
    pub reserved: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SysSpuSegment {
    pub kind: i32,
    pub local_storage_addr: u32,
    pub size: i32,
    pub source: SysSpuSegmentSource,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SysSpuThreadAttribute {
    pub name_addr: u32,
    pub name_len: u32,
    pub option: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SysSpuThreadArgument {
    pub arg1: u64,
    pub arg2: u64,
    pub arg3: u64,
    pub arg4: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SysSpuThreadGroupAttribute {
    /// Name length including the terminating NUL.
    pub name_len: u32,
    pub name_addr: u32,
    pub kind: i32,
    /// Read by LV2 when `kind` includes `SYS_SPU_THREAD_GROUP_TYPE_MEMORY_FROM_CONTAINER`.
    pub memory_container: u32,
}

assert_layout!(
    SysSpuImage,
    16,
    4,
    entry_point = 4,
    segments_addr = 8,
    segment_count = 12
);
assert_layout!(SysSpuSegmentSource, 8, 8);
assert_layout!(SysSpuSegment, 24, 8, source = 16);
assert_layout!(SysSpuThreadAttribute, 12, 4, name_len = 4, option = 8);
assert_layout!(SysSpuThreadArgument, 32, 8, arg4 = 24);
assert_layout!(
    SysSpuThreadGroupAttribute,
    16,
    4,
    name_addr = 4,
    kind = 8,
    memory_container = 12
);

define_syscalls! {
    pub unsafe fn sys_spu_initialize(max_usable_spu: u32, max_raw_spu: u32) -> i32 = SYS_SPU_INITIALIZE;
    pub unsafe fn sys_spu_image_open(img: *mut SysSpuImage, path: *const u8) -> i32 = SYS_SPU_IMAGE_OPEN;
    pub unsafe fn sys_spu_image_open_by_fd(img: *mut SysSpuImage, fd: i32, offset: i64) -> i32 = SYS_SPU_IMAGE_OPEN_BY_FD;
    pub unsafe fn sys_spu_thread_initialize(thread: *mut u32, group: u32, spu_num: u32, image: *const SysSpuImage, attr: *const SysSpuThreadAttribute, args: *const SysSpuThreadArgument) -> i32 = SYS_SPU_THREAD_INITIALIZE;
    pub unsafe fn sys_spu_thread_set_argument(id: u32, arg: *const SysSpuThreadArgument) -> i32 = SYS_SPU_THREAD_SET_ARGUMENT;
    pub unsafe fn sys_spu_thread_group_create(id: *mut u32, num: u32, prio: i32, attr: *const SysSpuThreadGroupAttribute) -> i32 = SYS_SPU_THREAD_GROUP_CREATE;
    pub unsafe fn sys_spu_thread_group_destroy(id: u32) -> i32 = SYS_SPU_THREAD_GROUP_DESTROY;
    pub unsafe fn sys_spu_thread_group_start(id: u32) -> i32 = SYS_SPU_THREAD_GROUP_START;
    /// Starts `number_of_groups` SPU thread groups after the current group exits.
    /// `groups` must point to that many group IDs.
    pub unsafe fn sys_spu_thread_group_start_on_exit(
        id: u32, number_of_groups: i32, groups: *const u32,
    ) -> i32 = SYS_SPU_THREAD_GROUP_START_ON_EXIT;
    pub unsafe fn sys_spu_thread_group_suspend(id: u32) -> i32 = SYS_SPU_THREAD_GROUP_SUSPEND;
    pub unsafe fn sys_spu_thread_group_resume(id: u32) -> i32 = SYS_SPU_THREAD_GROUP_RESUME;
    pub unsafe fn sys_spu_thread_group_yield(id: u32) -> i32 = SYS_SPU_THREAD_GROUP_YIELD;
    pub unsafe fn sys_spu_thread_group_terminate(id: u32, value: i32) -> i32 = SYS_SPU_THREAD_GROUP_TERMINATE;
    pub unsafe fn sys_spu_thread_group_join(id: u32, cause: *mut i32, status: *mut i32) -> i32 = SYS_SPU_THREAD_GROUP_JOIN;
    pub unsafe fn sys_spu_thread_group_set_priority(id: u32, priority: i32) -> i32 = SYS_SPU_THREAD_GROUP_SET_PRIORITY;
    pub unsafe fn sys_spu_thread_group_get_priority(id: u32, priority: *mut i32) -> i32 = SYS_SPU_THREAD_GROUP_GET_PRIORITY;
    pub unsafe fn sys_spu_thread_group_connect_event(id: u32, eq: u32, et: u32) -> i32 = SYS_SPU_THREAD_GROUP_CONNECT_EVENT;
    pub unsafe fn sys_spu_thread_group_disconnect_event(id: u32, et: u32) -> i32 = SYS_SPU_THREAD_GROUP_DISCONNECT_EVENT;
    pub unsafe fn sys_spu_thread_group_connect_event_all_threads(id: u32, eq_id: u32, req: u64, spup: *mut u8) -> i32 = SYS_SPU_THREAD_GROUP_CONNECT_EVENT_ALL_THREADS;
    pub unsafe fn sys_spu_thread_group_disconnect_event_all_threads(id: u32, spup: u8) -> i32 = SYS_SPU_THREAD_GROUP_DISCONNECT_EVENT_ALL_THREADS;
    #[cfg(feature = "sdk-370")]
    pub unsafe fn sys_spu_thread_group_set_cooperative_victims(id: u32, threads_mask: u32) -> i32 = SYS_SPU_THREAD_GROUP_SET_COOPERATIVE_VICTIMS;
    #[cfg(feature = "sdk-370")]
    pub unsafe fn sys_spu_thread_group_log(command: i32, stat: *mut i32) -> i32 = SYS_SPU_THREAD_GROUP_LOG;
    pub unsafe fn sys_spu_thread_write_ls(id: u32, lsa: u32, value: u64, r#type: u32) -> i32 = SYS_SPU_THREAD_WRITE_LS;
    pub unsafe fn sys_spu_thread_read_ls(id: u32, lsa: u32, value: *mut u64, r#type: u32) -> i32 = SYS_SPU_THREAD_READ_LS;
    pub unsafe fn sys_spu_thread_write_spu_mb(id: u32, value: u32) -> i32 = SYS_SPU_THREAD_WRITE_SPU_MB;
    pub unsafe fn sys_spu_thread_set_spu_cfg(id: u32, value: u64) -> i32 = SYS_SPU_THREAD_SET_SPU_CFG;
    pub unsafe fn sys_spu_thread_get_spu_cfg(id: u32, value: *mut u64) -> i32 = SYS_SPU_THREAD_GET_SPU_CFG;
    pub unsafe fn sys_spu_thread_write_snr(id: u32, number: i32, value: u32) -> i32 = SYS_SPU_THREAD_WRITE_SNR;
    pub unsafe fn sys_spu_thread_connect_event(id: u32, eq: u32, et: u32, spup: u8) -> i32 = SYS_SPU_THREAD_CONNECT_EVENT;
    pub unsafe fn sys_spu_thread_disconnect_event(id: u32, et: u32, spup: u8) -> i32 = SYS_SPU_THREAD_DISCONNECT_EVENT;
    pub unsafe fn sys_spu_thread_bind_queue(id: u32, spuq: u32, spuq_num: u32) -> i32 = SYS_SPU_THREAD_BIND_QUEUE;
    pub unsafe fn sys_spu_thread_unbind_queue(id: u32, spuq_num: u32) -> i32 = SYS_SPU_THREAD_UNBIND_QUEUE;
    pub unsafe fn sys_spu_thread_get_exit_status(id: u32, status: *mut i32) -> i32 = SYS_SPU_THREAD_GET_EXIT_STATUS;
    pub unsafe fn sys_spu_thread_recover_page_fault(id: u32) -> i32 = SYS_SPU_THREAD_RECOVER_PAGE_FAULT;
    pub unsafe fn sys_raw_spu_create(id: *mut i32, attr: *mut c_void) -> i32 = SYS_RAW_SPU_CREATE;
    pub unsafe fn sys_raw_spu_destroy(id: i32) -> i32 = SYS_RAW_SPU_DESTROY;
    pub unsafe fn sys_raw_spu_create_interrupt_tag(id: i32, class_id: u32, hwthread: u32, intrtag: *mut u32) -> i32 = SYS_RAW_SPU_CREATE_INTERRUPT_TAG;
    pub unsafe fn sys_raw_spu_set_int_mask(id: i32, class_id: u32, mask: u64) -> i32 = SYS_RAW_SPU_SET_INT_MASK;
    pub unsafe fn sys_raw_spu_get_int_mask(id: i32, class_id: u32, mask: *mut u64) -> i32 = SYS_RAW_SPU_GET_INT_MASK;
    pub unsafe fn sys_raw_spu_set_int_stat(id: i32, class_id: u32, stat: u64) -> i32 = SYS_RAW_SPU_SET_INT_STAT;
    pub unsafe fn sys_raw_spu_get_int_stat(id: i32, class_id: u32, stat: *mut u64) -> i32 = SYS_RAW_SPU_GET_INT_STAT;
    pub unsafe fn sys_raw_spu_read_puint_mb(id: i32, value: *mut u32) -> i32 = SYS_RAW_SPU_READ_PUINT_MB;
    pub unsafe fn sys_raw_spu_set_spu_cfg(id: i32, value: u32) -> i32 = SYS_RAW_SPU_SET_SPU_CFG;
    pub unsafe fn sys_raw_spu_get_spu_cfg(id: i32, value: *mut u32) -> i32 = SYS_RAW_SPU_GET_SPU_CFG;
    pub unsafe fn sys_raw_spu_recover_page_fault(id: i32) -> i32 = SYS_RAW_SPU_RECOVER_PAGE_FAULT;
}
