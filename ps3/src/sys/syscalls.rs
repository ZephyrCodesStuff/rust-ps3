//! Low-level PlayStation 3 Level 2 (LV2) Kernel Syscalls.
//!
//! Every syscall preserves the TOC register (`r2`) and creates a standard
//! parameter/linkage stack frame below the compiler’s red zone.

#[cfg(any(
    all(feature = "sdk-192", feature = "sdk-270"),
    all(feature = "sdk-192", feature = "sdk-370"),
    all(feature = "sdk-270", feature = "sdk-370"),
    not(any(feature = "sdk-192", feature = "sdk-270", feature = "sdk-370")),
))]
compile_error!("select exactly one PS3 syscall SDK ABI: `sdk-192`, `sdk-270`, or `sdk-370`");

pub mod nr;

// The LV2 trap does not make a PPU user-mode function call. In particular,
// leave FPRs and vector registers out of the asm clobber set.
macro_rules! lv2_trap {
    ($nr:expr; $($operands:tt)*) => {
        // LLVM may use the 288 bytes below r1 in a leaf function. Reserve
        // space for that red zone above this trap's 128-byte linkage frame.
        core::arch::asm!(
            "stdu 1, -416(1)",
            "std 2, 40(1)",
            "sc",
            "ld 2, 40(1)",
            "addi 1, 1, 416",
            inlateout("r11") $nr => _,
            $($operands)*
            lateout("r0") _,
            lateout("r12") _,
            lateout("lr") _,
            lateout("ctr") _,
            lateout("xer") _,
            lateout("cr0") _,
            lateout("cr1") _,
            lateout("cr5") _,
            lateout("cr6") _,
            lateout("cr7") _,
        )
    };
}

// Bind only the arguments a typed LV2 entry uses. Using explicit outputs makes
// the compiler preserve live volatile registers across the trap.
macro_rules! invoke {
    ($nr:expr) => {{
        let result: i64;
        invoke!(@args $nr; ["r4" "r5" "r6" "r7" "r8" "r9" "r10"]
            [lateout("r3") result,];);
        result
    }};
    ($nr:expr, $a0:expr $(, $arg:expr)*) => {{
        let result: i64;
        invoke!(@args $nr; ["r4" "r5" "r6" "r7" "r8" "r9" "r10"]
            [inlateout("r3") ($a0 as usize as u64) => result,]; $($arg),*);
        result
    }};
    (@args $nr:expr; [$($register:tt)*] [$($operands:tt)*];) => {
        unsafe { lv2_trap!($nr; $($operands)* $(lateout($register) _,)*) }
    };
    (@args $nr:expr; [$register:tt $($remaining:tt)*] [$($operands:tt)*];
        $value:expr $(, $arg:expr)*) => {
        invoke!(@args $nr; [$($remaining)*]
            [$($operands)* inlateout($register) ($value as usize as u64) => _,]; $($arg),*)
    };
}

// -----------------------------------------------------------------------------
// Declarative Syscall Definition DSL
// -----------------------------------------------------------------------------

macro_rules! define_syscalls {
    ($(
        $(#[$meta:meta])*
        $vis:vis unsafe fn $name:ident($($arg:ident: $ty:ty),* $(,)?) -> $ret:ty = $nr:ident;
    )*) => {
        $(
            $(#[$meta])*
            #[doc = "# Safety"]
            #[doc = "The caller must meet the LV2 contract for all addresses, lengths, handles and flags."]
            #[doc = "The running firmware must implement this selector with this register layout."]
            #[inline]
            $vis unsafe fn $name($($arg: $ty),*) -> $ret {
                invoke!($crate::sys::syscalls::nr::$nr $(, $arg)*) as $ret
            }
        )*
    };
}

macro_rules! define_syscall_actions {
    ($(
        $(#[$meta:meta])*
        $vis:vis unsafe fn $name:ident($($arg:ident: $ty:ty),* $(,)?) = $nr:ident;
    )*) => {
        $(
            $(#[$meta])*
            #[doc = "# Safety"]
            #[doc = "The caller must meet the LV2 contract for all addresses, lengths, handles and flags."]
            #[doc = "The running firmware must implement this selector with this register layout."]
            #[inline]
            $vis unsafe fn $name($($arg: $ty),*) {
                let _ = invoke!($crate::sys::syscalls::nr::$nr $(, $arg)*);
            }
        )*
    };
}

mod compat;
#[cfg(feature = "rsx")]
pub(crate) use compat::call_compact;
mod mmapper;
pub mod portable;
pub mod raw;

#[cfg(any(feature = "sdk-270", feature = "sdk-370"))]
pub use compat::sys_prx_get_ppu_guid;
pub use compat::{
    sys_interrupt_thread_establish, sys_ppu_thread_create, sys_prx_start_module_raw,
    sys_prx_stop_module, sys_timer_start_oneshot, sys_timer_start_periodic,
    sys_timer_start_periodic_absolute,
};

pub use mmapper::{
    sys_mmapper_allocate_memory, sys_mmapper_allocate_memory_from_container,
    sys_mmapper_free_memory, sys_mmapper_map_memory, sys_mmapper_unmap_memory,
};

define_syscalls! {
    /// Terminates the current process.
    pub unsafe fn sys_process_exit_raw(status: i32) -> i32 = SYS_PROCESS_EXIT;
    /// Terminates the current PPU thread.
    pub unsafe fn sys_ppu_thread_exit(val: u64) -> i32 = SYS_PPU_THREAD_EXIT;
}

/// Writes a string slice to TTY channel 0.
pub fn sys_tty_write(msg: &str) {
    unsafe {
        sys_tty_write_raw(0, msg.as_ptr(), msg.len());
    }
}

/// Writes bytes to a TTY channel.
///
/// # Safety
///
/// `ptr` must be valid for reads of `len` bytes for the duration of the call.
pub unsafe fn sys_tty_write_raw(channel: u64, ptr: *const u8, len: usize) -> i32 {
    let (Ok(channel), Ok(len)) = (u32::try_from(channel), u32::try_from(len)) else {
        return super::error::CELL_EINVAL;
    };
    let mut written = 0;
    unsafe { raw::sys_tty_write(channel, ptr, len, &mut written) }
}

/// Reads bytes from a TTY channel.
///
/// # Safety
///
/// `buf` must be writable for `len` bytes and `read_len` for one `u32`.
pub unsafe fn sys_tty_read(channel: u64, buf: *mut u8, len: usize, read_len: *mut u32) -> i32 {
    let (Ok(channel), Ok(len)) = (u32::try_from(channel), u32::try_from(len)) else {
        return super::error::CELL_EINVAL;
    };
    unsafe { raw::sys_tty_read(channel, buf, len, read_len) }
}

/// Sleeps the current PPU thread and returns the LV2 status.
#[inline]
pub unsafe fn sys_timer_usleep_raw(usecs: u64) -> i32 {
    unsafe { raw::sys_timer_usleep(usecs) }
}

/// Sleeps the current PPU thread for the given number of microseconds.
pub fn sys_timer_usleep(usecs: u64) {
    unsafe { raw::sys_timer_usleep(usecs) };
}

/// Creates a memory container using the selected SDK ABI.
///
/// # Safety
///
/// `out_container` must be writable for one `u32`.
pub unsafe fn sys_memory_container_create_raw(out_container: *mut u32, size: usize) -> i32 {
    let Ok(size) = u32::try_from(size) else {
        return super::error::CELL_EINVAL;
    };
    unsafe { raw::sys_memory_container_create(out_container, size) }
}

pub use raw::sys_memory_container_destroy;

/// Allocates memory and writes its 32-bit address to `out_addr`.
///
/// # Safety
///
/// `out_addr` must be writable for one `u32`. The size and flags must satisfy
/// the requirements of the requested page size.
pub unsafe fn sys_memory_allocate_raw(size: usize, flags: u64, out_addr: *mut u32) -> i32 {
    let Ok(size) = u32::try_from(size) else {
        return super::error::CELL_EINVAL;
    };
    unsafe { raw::sys_memory_allocate(size, flags, out_addr) }
}

/// Frees an LV2 allocation.
///
/// # Safety
///
/// `start_addr` must be the base of a live allocation from LV2. No reference
/// to the allocation may be used once it has been freed.
pub unsafe fn sys_memory_free(start_addr: *mut u8) -> i32 {
    let Ok(addr) = u32::try_from(start_addr as usize) else {
        return super::error::CELL_EINVAL;
    };
    unsafe { raw::sys_memory_free(addr) }
}

/// Writes the total and available user memory, in bytes.
///
/// # Safety
///
/// `info` must be writable for two `u32` values.
pub unsafe fn sys_memory_get_user_memory_size_raw(info: *mut [u32; 2]) -> i32 {
    unsafe { raw::sys_memory_get_user_memory_size(info.cast()) }
}

/// Reads the PPU timebase counter.
#[inline]
pub fn sys_time_get_timebase() -> u64 {
    let ticks: u64;
    unsafe {
        core::arch::asm!("mftb {ticks}", ticks = out(reg) ticks, options(nomem, nostack, preserves_flags));
    }
    ticks
}

/// Returns system time in microseconds using the PPU timebase.
///
/// This SDK operation uses a counter reading and the frequency from LV2.
/// Callers polling at high frequency can cache the frequency and read the
/// timebase directly with [`sys_time_get_timebase`].
#[inline]
pub fn sys_time_get_system_time() -> u64 {
    let hz = unsafe { raw::sys_time_get_timebase_frequency() };
    let ticks = sys_time_get_timebase();
    const MICROSECONDS_PER_SECOND: u64 = 1_000_000;
    (ticks / hz) * MICROSECONDS_PER_SECOND + (ticks % hz) * MICROSECONDS_PER_SECOND / hz
}

/// Allocates virtual memory pages from the LV2 kernel.
pub unsafe fn sys_memory_allocate(size: usize, flags: u64) -> Result<*mut u8, i32> {
    let mut out_addr: u32 = 0;
    let ret = sys_memory_allocate_raw(size, flags, &mut out_addr);

    if ret == 0 {
        Ok(out_addr as usize as *mut u8)
    } else {
        Err(ret)
    }
}

/// Queries total and available user memory.
pub unsafe fn sys_memory_get_user_memory_size() -> Result<(u32, u32), i32> {
    let mut mem_info: [u32; 2] = [0, 0];
    let ret = sys_memory_get_user_memory_size_raw(&mut mem_info);

    if ret == 0 {
        Ok((mem_info[0], mem_info[1]))
    } else {
        Err(ret)
    }
}

/// Terminates the current process (Syscall 3: SYS_PROCESS_EXIT).
pub fn sys_process_exit(status: i32) -> ! {
    unsafe {
        sys_process_exit_raw(status);
        loop {}
    }
}
