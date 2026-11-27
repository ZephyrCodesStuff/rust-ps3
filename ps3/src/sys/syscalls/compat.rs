use super::raw::{self, SysPpuThreadParams, SysPrxStartStopModuleOption};
use crate::sys::error::{CELL_EFAULT, CELL_EINVAL, CELL_OK};

type ModuleCommand = unsafe fn(u32, u64, *mut SysPrxStartStopModuleOption) -> i32;

/// Creates a PPU thread with an initial r13 of zero.
///
/// The entry must be a CellOS function descriptor address.
///
/// # Safety
///
/// The output and name pointers must satisfy LV2's contract. The entry
/// descriptor, code, and any storage referenced by the argument must be valid
/// for the new thread. The entry must be able to run without TLS.
pub unsafe fn sys_ppu_thread_create(
    thread_id: *mut u64,
    entry: u64,
    arg: u64,
    prio: i32,
    stacksize: usize,
    flags: u64,
    threadname: *const u8,
) -> i32 {
    let Ok(entry) = u32::try_from(entry) else {
        return CELL_EINVAL;
    };
    if thread_id.is_null() {
        return CELL_EFAULT;
    }
    let mut params = SysPpuThreadParams { entry, tls: 0 };
    unsafe {
        let result = raw::_sys_ppu_thread_create(
            thread_id,
            &mut params,
            arg,
            0,
            prio,
            stacksize as u64,
            flags,
            threadname,
        );
        if result != 0 || flags & raw::SYS_PPU_THREAD_CREATE_INTERRUPT != 0 {
            return result;
        }
        raw::sys_ppu_thread_start(thread_id.read())
    }
}

/// Starts a loaded PRX and stores its callback return code in `modres`.
///
/// # Safety
///
/// The module must be loaded and eligible to start. The caller must provide
/// writable storage for `modres` and any callback arguments required by the
/// module, and serialize the full operation with other lifecycle calls.
/// `opt` is reserved.
pub unsafe fn sys_prx_start_module_raw(
    prx_id: i32,
    args: u64,
    argp: u64,
    modres: *mut i32,
    flags: u64,
    _opt: u64,
) -> i32 {
    unsafe {
        run_prx_callback(
            raw::_sys_prx_start_module,
            prx_id,
            args,
            argp,
            modres,
            flags,
        )
    }
}

/// Stops a started PRX and stores its callback return code in `modres`.
///
/// # Safety
///
/// The module must be eligible to stop. The caller must provide writable
/// storage for `modres` and any callback arguments required by the module.
/// They must serialize the full operation with other lifecycle calls.
/// `opt` is reserved.
pub unsafe fn sys_prx_stop_module(
    prx_id: i32,
    args: u64,
    argp: u64,
    modres: *mut i32,
    flags: u64,
    _opt: u64,
) -> i32 {
    unsafe { run_prx_callback(raw::_sys_prx_stop_module, prx_id, args, argp, modres, flags) }
}

unsafe fn run_prx_callback(
    syscall: ModuleCommand,
    id: i32,
    args: u64,
    argp: u64,
    modres: *mut i32,
    flags: u64,
) -> i32 {
    let Ok(args) = u32::try_from(args) else {
        return CELL_EINVAL;
    };
    if modres.is_null() {
        return CELL_EINVAL;
    }
    let mut option = SysPrxStartStopModuleOption::begin();
    unsafe {
        let status = syscall(id as u32, flags, &mut option);
        if status < 0 {
            return status;
        }

        let result = if option.entry2 != u64::MAX {
            call_prx_entry(option.entry2, option.entry, u64::from(args), argp)
        } else if option.entry != u64::MAX {
            call_prx_entry(option.entry, u64::from(args), argp, 0)
        } else {
            0
        };

        modres.write(result);
        option.command = 2;
        option.result = i64::from(result) as u64;
        // CellOS reports the callback's status via modres and the kernel
        // applies it during phase 2.
        let _ = syscall(id as u32, flags, &mut option);
        CELL_OK
    }
}

/// Calls an LV2 compact descriptor while keeping the caller's TOC outside
/// the callback's linkage and parameter save areas and the Rust red zone.
unsafe fn call_prx_entry(address: u64, a0: u64, a1: u64, a2: u64) -> i32 {
    let result: i64;
    unsafe {
        core::arch::asm!(
            "stdu 1, -432(1)",
            "std 2, 128(1)",
            "lwz 0, 0(12)",
            "lwz 2, 4(12)",
            "li 11, 0",
            "mtctr 0",
            "bctrl",
            "ld 2, 128(1)",
            "addi 1, 1, 432",
            in("r12") address,
            inlateout("r3") a0 => result,
            in("r4") a1,
            in("r5") a2,
            clobber_abi("C"),
        );
    }
    result as i32
}

/// Starts a one-shot LV2 timer at an absolute system time in microseconds.
///
/// # Safety
///
/// `timer_id` must identify a timer owned by the process. Its event queue
/// and timer must remain valid for the duration of this call.
pub unsafe fn sys_timer_start_oneshot(timer_id: u32, expiration_time: i64) -> i32 {
    unsafe { raw::_sys_timer_start(timer_id, expiration_time, 0) }
}

/// Starts a periodic LV2 timer with a period in microseconds.
///
/// # Safety
///
/// `timer_id` must identify a timer owned by the process. Its event queue
/// and timer must remain valid for the duration of this call.
pub unsafe fn sys_timer_start_periodic(timer_id: u32, period: u64) -> i32 {
    unsafe { raw::_sys_timer_start(timer_id, 0, period) }
}

/// Starts a periodic LV2 timer relative to an absolute base time in microseconds.
///
/// # Safety
///
/// `timer_id` must identify a timer owned by the process. Its event queue
/// and timer must remain valid for the duration of this call.
pub unsafe fn sys_timer_start_periodic_absolute(timer_id: u32, base_time: i64, period: u64) -> i32 {
    unsafe { raw::_sys_timer_start(timer_id, base_time, period) }
}

/// Attaches an interrupt thread to a tag and supplies one handler argument.
///
/// # Safety
///
/// The handle pointer must be writable. The tag and thread must be owned by
/// the process and remain valid for the duration of the call.
pub unsafe fn sys_interrupt_thread_establish(
    handle: *mut u32,
    tag: u32,
    thread: u64,
    arg: u64,
) -> i32 {
    unsafe { raw::_sys_interrupt_thread_establish(handle, tag, thread, arg, 0) }
}

/// Looks up the PPU GUID address for a loaded PRX.
///
/// Returns a zero address if no GUID is available for the module.
///
/// # Safety
///
/// The running firmware must implement the SDK 3.70 PRX GUID selector.
/// A returned nonzero address is valid only while the module remains loaded.
#[cfg(feature = "sdk-370")]
pub unsafe fn sys_prx_get_ppu_guid(module_id: i32) -> u32 {
    const SYS_PRX_GET_PPU_GUID: u64 = 467;
    invoke!(SYS_PRX_GET_PPU_GUID, module_id) as u32
}
