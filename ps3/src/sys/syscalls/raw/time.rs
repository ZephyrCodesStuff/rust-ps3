#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SysTimerInformation {
    pub next_expire: i64,
    pub period: u64,
    pub state: u32,
    pub reserved: u32,
}

assert_layout!(SysTimerInformation, 24, 8, state = 16);

define_syscalls! {
    pub unsafe fn sys_time_get_current_time(sec: *mut i64, nsec: *mut i64) -> i32 = SYS_TIME_GET_CURRENT_TIME;
    pub unsafe fn sys_time_get_timebase_frequency() -> u64 = SYS_TIME_GET_TIMEBASE_FREQUENCY;
    pub unsafe fn sys_timer_create(timer_id: *mut u32) -> i32 = SYS_TIMER_CREATE;
    pub unsafe fn sys_timer_destroy(timer_id: u32) -> i32 = SYS_TIMER_DESTROY;
    pub unsafe fn sys_timer_get_information(timer_id: u32, info: *mut SysTimerInformation) -> i32 = SYS_TIMER_GET_INFORMATION;
    pub unsafe fn _sys_timer_start(timer_id: u32, basetime: i64, period: u64) -> i32 = _SYS_TIMER_START;
    pub unsafe fn sys_timer_stop(timer_id: u32) -> i32 = SYS_TIMER_STOP;
    pub unsafe fn sys_timer_connect_event_queue(timer_id: u32, queue_id: u32, name: u64, data1: u64, data2: u64) -> i32 = SYS_TIMER_CONNECT_EVENT_QUEUE;
    pub unsafe fn sys_timer_disconnect_event_queue(timer_id: u32) -> i32 = SYS_TIMER_DISCONNECT_EVENT_QUEUE;
    pub unsafe fn sys_timer_sleep(sleep_time: u32) -> i32 = SYS_TIMER_SLEEP;
    pub unsafe fn sys_timer_usleep(sleep_time: u64) -> i32 = SYS_TIMER_USLEEP;
}
