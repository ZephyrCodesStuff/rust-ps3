pub const SYS_PPU_THREAD_CREATE_JOINABLE: u64 = 0x1;
pub const SYS_PPU_THREAD_CREATE_INTERRUPT: u64 = 0x2;

/// A CellOS function descriptor. Its two words hold code and r2 (TOC).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PpuFunctionDescriptor {
    pub entry: u32,
    pub toc: u32,
}

/// Parameters consumed by LV2 syscall 52. `entry` is the 32-bit address of
/// a CellOS descriptor. `tls` is the initial r13 address.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct SysPpuThreadParams {
    pub entry: u32,
    pub tls: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SysPpuThreadStack {
    pub address: u32,
    pub size: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SysPpuThreadInterruptContext {
    pub gpr: [u64; 32],
    pub cr: u32,
    pub reserved: u32,
    pub xer: u64,
    pub lr: u64,
    pub ctr: u64,
    pub pc: u64,
}

assert_layout!(PpuFunctionDescriptor, 8, 4);
assert_layout!(SysPpuThreadParams, 8, 4, tls = 4);
assert_layout!(SysPpuThreadStack, 8, 4);
assert_layout!(SysPpuThreadInterruptContext, 296, 8, xer = 264, pc = 288);

define_syscalls! {
    pub unsafe fn sys_ppu_thread_join(thread_id: u64, vptr: *mut u64) -> i32 = SYS_PPU_THREAD_JOIN;
    pub unsafe fn sys_ppu_thread_detach(thread_id: u64) -> i32 = SYS_PPU_THREAD_DETACH;
    pub unsafe fn sys_ppu_thread_set_priority(thread_id: u64, prio: i32) -> i32 = SYS_PPU_THREAD_SET_PRIORITY;
    pub unsafe fn sys_ppu_thread_get_priority(thread_id: u64, priop: *mut i32) -> i32 = SYS_PPU_THREAD_GET_PRIORITY;
    pub unsafe fn sys_ppu_thread_get_stack_information(sp: *mut SysPpuThreadStack) -> i32 = SYS_PPU_THREAD_GET_STACK_INFORMATION;
    pub(crate) unsafe fn _sys_ppu_thread_create(thread_id: *mut u64, param: *mut SysPpuThreadParams, arg: u64, arg4: u64, prio: i32, stacksize: u64, flags: u64, threadname: *const u8) -> i32 = _SYS_PPU_THREAD_CREATE;
    pub(crate) unsafe fn sys_ppu_thread_start(thread_id: u64) -> i32 = SYS_PPU_THREAD_START;
    pub unsafe fn sys_ppu_thread_rename(thread_id: u64, name: *const u8) -> i32 = SYS_PPU_THREAD_RENAME;
    pub unsafe fn sys_ppu_thread_recover_page_fault(thread_id: u64) -> i32 = SYS_PPU_THREAD_RECOVER_PAGE_FAULT;
    #[cfg(feature = "sdk-370")]
    pub unsafe fn sys_ppu_thread_get_page_fault_context(thread_id: u64, ctxt: *mut SysPpuThreadInterruptContext) -> i32 = SYS_PPU_THREAD_GET_PAGE_FAULT_CONTEXT;
    pub unsafe fn sys_interrupt_tag_create(intrtag: *mut u32, irq_outlet: u64, hw_thread: u32) -> i32 = SYS_INTERRUPT_TAG_CREATE;
    pub unsafe fn sys_interrupt_tag_destroy(intrtag: u32) -> i32 = SYS_INTERRUPT_TAG_DESTROY;
    pub unsafe fn _sys_interrupt_thread_establish(ih: *mut u32, intrtag: u32, intrthread: u64, arg1: u64, arg2: u64) -> i32 = _SYS_INTERRUPT_THREAD_ESTABLISH;
    pub unsafe fn _sys_interrupt_thread_disestablish(ih: u32, r13: *mut u64) -> i32 = _SYS_INTERRUPT_THREAD_DISESTABLISH;
}

define_syscall_actions! {
    pub unsafe fn sys_ppu_thread_yield() = SYS_PPU_THREAD_YIELD;
    pub unsafe fn sys_ppu_thread_get_join_state(isjoinable: *mut i32) = SYS_PPU_THREAD_GET_JOIN_STATE;
    pub unsafe fn sys_interrupt_thread_eoi() = SYS_INTERRUPT_THREAD_EOI;
}
