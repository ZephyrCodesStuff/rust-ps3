//! Public DECI3 calls for supported development systems.

use core::ffi::c_void;

define_syscalls! {
    /// Opens a DECI3 session on a debugging station or reference tool.
    /// A non-null option must match the selected protocol.
    pub unsafe fn sys_deci3_open(
        protocol: u32, port: u8, option: *mut c_void, session: *mut i32,
    ) -> i32 = SYS_DECI3_OPEN;

    pub unsafe fn sys_deci3_create_event_path(
        session: i32, buffer_size: u32, event_queue: u32,
    ) -> i32 = SYS_DECI3_CREATE_EVENT_PATH;

    pub unsafe fn sys_deci3_close(session: i32) -> i32 = SYS_DECI3_CLOSE;

    pub unsafe fn sys_deci3_send(
        session: i32, buffer: *const u8, size: u32,
    ) -> i32 = SYS_DECI3_SEND;

    pub unsafe fn sys_deci3_receive(
        session: i32, buffer: *mut u8, size: u32,
    ) -> i32 = SYS_DECI3_RECEIVE;
}

define_syscall_actions! {}
