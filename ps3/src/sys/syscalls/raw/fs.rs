define_syscalls! {
    #[cfg(feature = "sdk-370")]
    pub unsafe fn sys_fsw_connect_event(queue: u32) -> i32 = SYS_FSW_CONNECT_EVENT;
    #[cfg(feature = "sdk-370")]
    pub unsafe fn sys_fsw_disconnect_event() -> i32 = SYS_FSW_DISCONNECT_EVENT;
    pub unsafe fn sys_tty_read(ch: u32, buf: *mut u8, len: u32, preadlen: *mut u32) -> i32 = SYS_TTY_READ;
    pub unsafe fn sys_tty_write(ch: u32, buf: *const u8, len: u32, pwritelen: *mut u32) -> i32 = SYS_TTY_WRITE;
}
