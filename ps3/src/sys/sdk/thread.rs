//! SDK PPU thread management using the firmware's thread-local state.

use crate::sys::raw::PpuFunctionDescriptor;

pub const SYS_PPU_THREAD_ONCE_INIT: i32 = 0;
pub const SYS_PPU_THREAD_DONE_INIT: i32 = 1;

extern "C" {
    /// Creates a thread. `entry` points to a compact CellOS function
    /// descriptor. Its entry point receives one `u64` argument.
    pub fn sys_ppu_thread_create(
        id: *mut u64,
        entry: *const PpuFunctionDescriptor,
        arg: u64,
        priority: i32,
        stack_size: u32,
        flags: u64,
        name: *const u8,
    ) -> i32;
    pub fn sys_ppu_thread_exit(value: u64);
    pub fn sys_ppu_thread_get_id(id: *mut u64) -> i32;
    /// The callback is a compact CellOS function descriptor with no arguments.
    pub fn sys_ppu_thread_once(control: *mut i32, init: *const PpuFunctionDescriptor);
    pub fn sys_interrupt_thread_disestablish(handle: u32) -> i32;
}
