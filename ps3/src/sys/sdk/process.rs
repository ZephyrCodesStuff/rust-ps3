//! Firmware process and system services.

use core::ffi::c_void;

#[cfg(feature = "sdk-370")]
pub const RANDOM_NUMBER_MAX_SIZE: usize = 4096;

extern "C" {
    pub fn sys_process_exit(status: i32);
    pub fn sys_process_is_stack(ptr: *const c_void) -> i32;

    /// Each argument and environment array stores 32-bit effective addresses.
    pub fn sys_game_process_exitspawn(
        path: *const u8,
        argv: *const u32,
        envp: *const u32,
        data: u32,
        data_size: u32,
        priority: i32,
        flags: u64,
    );
    /// Each argument and environment array stores 32-bit effective addresses.
    pub fn sys_game_process_exitspawn2(
        path: *const u8,
        argv: *const u32,
        envp: *const u32,
        data: u32,
        data_size: u32,
        priority: i32,
        flags: u64,
    );

    pub fn sys_time_get_system_time() -> i64;
    #[cfg(feature = "sdk-370")]
    pub fn sys_get_random_number(buffer: *mut c_void, size: u64) -> i32;
    pub fn console_putc(c: i32);
    pub fn console_write(data: *mut u8, len: u32) -> i32;
    pub fn console_getc() -> i32;
}
