#![no_std]
#![no_main]

use core::arch::global_asm;
use ps3::sys::error::{Lv2Error, CELL_EINVAL};
use ps3::sys::{self, raw::*};

global_asm!(
    r#"
    .pushsection .entrydesc,"a"
    .p2align 2
    .globl _start
_start:
    .long .Lstart, __toc
    .globl lv2_smoke_thread
lv2_smoke_thread:
    .long .Lthread, __toc
    .popsection
    .text
.Lstart:
    b {start}
.Lthread:
    b {thread}
"#,
    start = sym start,
    thread = sym thread,
);

#[cfg(feature = "sdk-imports")]
global_asm!(
    r#"
    .pushsection .entrydesc,"a"
    .p2align 2
    .globl lv2_sdk_thread
lv2_sdk_thread:
    .long .Lsdk_thread, __toc
    .popsection
    .text
.Lsdk_thread:
    b {sdk_thread}
"#,
    sdk_thread = sym sdk_thread,
);

extern "C" {
    static lv2_smoke_thread: PpuFunctionDescriptor;
    #[cfg(feature = "sdk-imports")]
    static lv2_sdk_thread: PpuFunctionDescriptor;
}

fn check(condition: bool, test: &str) {
    if !condition {
        sys::sys_tty_write("LV2 FAIL ");
        sys::sys_tty_write(test);
        sys::sys_tty_write("\n");
        sys::sys_process_exit(1);
    }
}

extern "C" fn start() -> ! {
    unsafe {
        let pid = sys_process_getpid();
        check(pid > 0, "getpid");
        let mut seconds = 0;
        let mut nanoseconds = 0;
        check(
            sys_time_get_current_time(&mut seconds, &mut nanoseconds) == 0,
            "time",
        );
        check(
            seconds > 0 && (0..1_000_000_000).contains(&nanoseconds),
            "time values",
        );
        check(sys_time_get_timebase_frequency() > 0, "timebase");
        let before = sys::sys_time_get_system_time();
        check(sys_timer_usleep(1000) == 0, "usleep");
        check(
            sys::sys_time_get_system_time() >= before + 1000,
            "system time",
        );

        memory();
        mmapper();
        events();
        timers();
        synchronization();
        threads();
        #[cfg(feature = "sdk-imports")]
        sdk_imports();
    }

    sys::sys_tty_write("LV2 ABI smoke passed\n");
    sys::sys_process_exit(0);
}

unsafe fn memory() {
    let mut info = SysMemoryInfo::default();
    check(
        sys_memory_get_user_memory_size(&mut info) == 0,
        "memory size",
    );
    check(
        info.total_user_memory > 0 && info.available_user_memory <= info.total_user_memory,
        "memory info layout",
    );

    let mut addr = 0;
    check(
        sys_memory_allocate(0x10000, sys::SYS_MEMORY_PAGE_SIZE_64K, &mut addr) == 0,
        "allocate",
    );
    let mut attr = SysPageAttribute::default();
    check(
        sys_memory_get_page_attribute(addr, &mut attr) == 0,
        "page attributes",
    );
    check(attr.page_size == 0x10000, "page attributes layout");
    check(sys_memory_free(addr) == 0, "free");
    let status = sys_memory_free(0);
    check(
        Lv2Error::check(status).map_err(Lv2Error::code) == Err(CELL_EINVAL),
        "free status bits",
    );

    let mut container = 0;
    check(
        sys_memory_container_create(&mut container, 0x100000) == 0,
        "container create",
    );
    check(
        sys_memory_container_get_size(&mut info, container) == 0,
        "container size",
    );
    check(info.total_user_memory == 0x100000, "container size layout");
    check(
        sys_memory_container_destroy(container) == 0,
        "container destroy",
    );

    check(
        sys::portable::sys_memory_container_create(&mut container, 0x100000) == 0,
        "portable container create",
    );
    check(
        sys_memory_container_get_size(&mut info, container) == 0
            && info.total_user_memory == 0x100000,
        "portable container size",
    );
    check(
        sys::portable::sys_memory_container_destroy(container) == 0,
        "portable container destroy",
    );
}

unsafe fn mmapper() {
    let page_size = SYS_MEMORY_PAGE_SIZE_64K;
    let mut addr = 0;
    check(
        sys_mmapper_allocate_address(0x1000_0000, page_size, 0x1000_0000, &mut addr) == 0,
        "mmapper reserve",
    );
    let mut mem_id = 0;
    check(
        sys::sys_mmapper_allocate_memory(0x10000, page_size, &mut mem_id) == 0,
        "mmapper allocate",
    );
    check(
        sys::sys_mmapper_map_memory(addr, mem_id, SYS_MEMORY_ATTR_READ_WRITE) == 0,
        "mmapper map",
    );
    let word = addr as usize as *mut u32;
    word.write_volatile(0x0123_4567);
    check(
        word.read_volatile() == 0x0123_4567,
        "mmapper read and write",
    );
    let mut unmapped = 0;
    check(
        sys::sys_mmapper_unmap_memory(addr, &mut unmapped) == 0 && unmapped == mem_id,
        "mmapper unmap",
    );
    check(
        sys::sys_mmapper_free_memory(mem_id) == 0,
        "mmapper free memory",
    );
    check(sys_mmapper_free_address(addr) == 0, "mmapper free address");

    let mut container = 0;
    check(
        sys_memory_container_create(&mut container, 0x100000) == 0,
        "mmapper container create",
    );
    check(
        sys::sys_mmapper_allocate_memory_from_container(0x10000, container, page_size, &mut mem_id)
            == 0,
        "mmapper allocate from container",
    );
    check(
        sys::sys_mmapper_free_memory(mem_id) == 0,
        "mmapper free from container",
    );
    check(
        sys_memory_container_destroy(container) == 0,
        "mmapper container destroy",
    );
}

unsafe fn events() {
    let attr = SysEventQueueAttribute {
        protocol: SYS_SYNC_FIFO,
        kind: SYS_PPU_QUEUE,
        name: *b"lv2-smok",
    };
    let mut queue = 0;
    check(
        sys_event_queue_create(&mut queue, &attr, SYS_EVENT_QUEUE_LOCAL, 8) == 0,
        "event queue create",
    );
    let mut port = 0;
    let expected = SysEvent {
        source: 0x1234_5678_1234_5678,
        data1: 0xabcd_ef01_2345_6789,
        data2: 0x0123_4567_89ab_cdef,
        data3: 0xfedc_ba98_7654_3210,
    };
    check(
        sys_event_port_create(&mut port, SYS_EVENT_PORT_LOCAL, expected.source) == 0,
        "event port create",
    );
    check(
        sys_event_port_connect_local(port, queue) == 0,
        "event connect",
    );
    check(
        sys_event_port_send(port, expected.data1, expected.data2, expected.data3) == 0,
        "event send",
    );
    let mut event = SysEvent::default();
    check(
        sys_event_queue_receive(queue, &mut event, 1_000_000) == 0,
        "event receive",
    );
    check(event == expected, "event return registers");

    let mut count = 1;
    check(
        sys_event_queue_tryreceive(queue, &mut event, 1, &mut count) == 0,
        "event tryreceive",
    );
    check(count == 0, "event tryreceive count");
    check(sys_event_port_disconnect(port) == 0, "event disconnect");
    check(sys_event_port_destroy(port) == 0, "event port destroy");
    check(
        sys_event_queue_destroy(queue, 0) == 0,
        "event queue destroy",
    );
}

unsafe fn timers() {
    let attr = SysEventQueueAttribute {
        protocol: SYS_SYNC_FIFO,
        kind: SYS_PPU_QUEUE,
        name: *b"lv2-time",
    };
    let mut queue = 0;
    check(
        sys_event_queue_create(&mut queue, &attr, SYS_EVENT_QUEUE_LOCAL, 4) == 0,
        "timer queue create",
    );
    let mut timer = 0;
    check(sys_timer_create(&mut timer) == 0, "timer create");
    check(
        sys_timer_connect_event_queue(timer, queue, 7, 8, 9) == 0,
        "timer connect",
    );

    let now = sys::sys_time_get_system_time() as i64;
    check(
        sys::sys_timer_start_oneshot(timer, now + 20_000) == 0,
        "timer oneshot",
    );
    let mut event = SysEvent::default();
    check(
        sys_event_queue_receive(queue, &mut event, 1_000_000) == 0
            && (event.source, event.data1, event.data2) == (7, 8, 9),
        "timer event",
    );

    let mut info = SysTimerInformation::default();
    check(
        sys::sys_timer_start_periodic(timer, 1_000_000) == 0
            && sys_timer_get_information(timer, &mut info) == 0
            && info.period == 1_000_000,
        "timer periodic",
    );
    check(sys_timer_stop(timer) == 0, "timer stop");
    let base_time = sys::sys_time_get_system_time() as i64 + 1_000_000;
    check(
        sys::sys_timer_start_periodic_absolute(timer, base_time, 2_000_000) == 0
            && sys_timer_get_information(timer, &mut info) == 0
            && info.next_expire == base_time + 2_000_000
            && info.period == 2_000_000,
        "timer periodic absolute",
    );
    check(sys_timer_stop(timer) == 0, "timer absolute stop");
    check(
        sys_timer_disconnect_event_queue(timer) == 0,
        "timer disconnect",
    );
    check(sys_timer_destroy(timer) == 0, "timer destroy");
    check(
        sys_event_queue_destroy(queue, 0) == 0,
        "timer queue destroy",
    );
}

unsafe fn synchronization() {
    let attr = SysSemaphoreAttribute {
        protocol: SYS_SYNC_FIFO,
        pshared: SYS_SYNC_NOT_PROCESS_SHARED,
        ipc_key: 0,
        flags: 0,
        reserved: 0,
        name: *b"lv2-sem\0",
    };
    let mut id = 0;
    check(
        sys_semaphore_create(&mut id, &attr, 0, 1) == 0,
        "semaphore create",
    );
    check(sys_semaphore_post(id, 1) == 0, "semaphore post");
    check(sys_semaphore_wait(id, 1_000_000) == 0, "semaphore wait");
    let mut value = -1;
    check(
        sys_semaphore_get_value(id, &mut value) == 0 && value == 0,
        "semaphore value",
    );
    check(sys_semaphore_destroy(id) == 0, "semaphore destroy");

    let attr = SysRwlockAttribute {
        protocol: SYS_SYNC_FIFO,
        pshared: SYS_SYNC_NOT_PROCESS_SHARED,
        ipc_key: 0,
        flags: 0,
        reserved: 0,
        name: *b"lv2-rw\0\0",
    };
    check(sys_rwlock_create(&mut id, &attr) == 0, "rwlock create");
    check(sys_rwlock_trywlock(id) == 0, "rwlock try-write");
    check(sys_rwlock_wunlock(id) == 0, "rwlock write unlock");
    check(
        sys::portable::sys_rwlock_trywlock(id) == 0,
        "portable rwlock try-write",
    );
    check(sys_rwlock_wunlock(id) == 0, "portable rwlock write unlock");
    check(sys_rwlock_destroy(id) == 0, "rwlock destroy");
}

#[cfg(feature = "sdk-imports")]
extern "C" fn sdk_thread(value: u64) -> ! {
    unsafe { sys::sdk::thread::sys_ppu_thread_exit(value + 1) };
    loop {
        core::hint::spin_loop();
    }
}

extern "C" fn thread(value: u64) -> ! {
    unsafe { sys::sys_ppu_thread_exit(value + 1) };
    loop {
        core::hint::spin_loop();
    }
}

unsafe fn threads() {
    let mut stack = SysPpuThreadStack::default();
    check(
        sys_ppu_thread_get_stack_information(&mut stack) == 0
            && stack.address != 0
            && stack.size >= 4096,
        "thread stack information",
    );
    let mut thread_id = 0;
    let entry = (&raw const lv2_smoke_thread) as usize as u64;
    let argument = 0x0123_4567_89ab_cdef;
    check(
        sys::sys_ppu_thread_create(
            &mut thread_id,
            entry,
            argument,
            1000,
            0x10000,
            SYS_PPU_THREAD_CREATE_JOINABLE,
            b"rust-lv2\0".as_ptr(),
        ) == 0,
        "thread create and start",
    );
    let mut result = 0;
    check(
        sys_ppu_thread_join(thread_id, &mut result) == 0 && result == argument + 1,
        "thread join",
    );
}

#[cfg(feature = "sdk-imports")]
unsafe fn sdk_imports() {
    use core::mem::MaybeUninit;
    use ps3::sys::sdk::{memory::*, process, sync::*, thread};

    let mut thread_id = 0;
    check(
        thread::sys_ppu_thread_get_id(&mut thread_id) == 0 && thread_id != 0,
        "SDK current thread",
    );
    check(
        process::sys_process_is_stack((&raw const thread_id).cast()) != 0,
        "SDK stack address",
    );

    let mut worker_id = 0;
    check(
        thread::sys_ppu_thread_create(
            &mut worker_id,
            &raw const lv2_sdk_thread,
            42,
            1000,
            0x10000,
            SYS_PPU_THREAD_CREATE_JOINABLE,
            b"rust-sdk\0".as_ptr(),
        ) == 0,
        "SDK thread create",
    );
    let mut worker_status = 0;
    check(
        sys_ppu_thread_join(worker_id, &mut worker_status) == 0 && worker_status == 43,
        "SDK thread exit and join",
    );

    let mut lock = 0;
    sys_spinlock_initialize(&mut lock);
    check(sys_spinlock_trylock(&mut lock) == 0, "SDK spinlock");
    sys_spinlock_unlock(&mut lock);

    let mut mutex = MaybeUninit::<SysLwmutex>::uninit();
    let mut mutex_attr = SysLwmutexAttribute::default();
    let mut cond = MaybeUninit::<SysLwcond>::uninit();
    let mut cond_attr = SysLwcondAttribute::default();
    check(
        sys_lwmutex_create(mutex.as_mut_ptr(), &mut mutex_attr) == 0,
        "SDK lightweight mutex create",
    );
    check(
        sys_lwcond_create(cond.as_mut_ptr(), mutex.as_mut_ptr(), &mut cond_attr) == 0,
        "SDK lightweight condition create",
    );
    check(
        sys_lwmutex_lock(mutex.as_mut_ptr(), 0) == 0,
        "SDK lightweight mutex lock",
    );
    check(
        sys_lwcond_wait(cond.as_mut_ptr(), 1000) == ps3::sys::error::CELL_ETIMEDOUT,
        "SDK lightweight condition timeout",
    );
    check(
        sys_lwmutex_unlock(mutex.as_mut_ptr()) == 0,
        "SDK lightweight mutex unlock",
    );
    check(
        sys_lwcond_destroy(cond.as_mut_ptr()) == 0,
        "SDK lightweight condition destroy",
    );
    check(
        sys_lwmutex_destroy(mutex.as_mut_ptr()) == 0,
        "SDK lightweight mutex destroy",
    );

    #[repr(align(128))]
    struct PoolBacking([u8; 4096]);
    let mut backing = PoolBacking([0; 4096]);
    let mut pool = SysMempool::default();
    check(
        sys_mempool_create(&mut pool, backing.0.as_mut_ptr().cast(), 4096, 256, 128) == 0,
        "SDK memory pool create",
    );
    let count = sys_mempool_get_count(pool);
    let block = sys_mempool_allocate_block(pool);
    check(
        count > 0 && !block.is_null() && sys_mempool_get_count(pool) == count - 1,
        "SDK memory pool allocate",
    );
    check(
        sys_mempool_free_block(pool, block) == 0 && sys_mempool_get_count(pool) == count,
        "SDK memory pool free",
    );
    sys_mempool_destroy(pool);

    prx();
    #[cfg(feature = "sdk-debug")]
    debug_query(thread_id);
}

#[cfg(feature = "sdk-imports")]
unsafe fn prx() {
    use core::ptr::{null, null_mut};
    use ps3::sys::sdk::prx::*;

    let path = b"/dev_flash/sys/external/libfiber.sprx\0";
    let id = sys_prx_load_module(path.as_ptr(), 0, null_mut());
    check(id > 0, "SDK PRX load");
    let mut ids = [0i32; 16];
    let mut list = SysPrxModuleList {
        capacity: ids.len() as u32,
        ids_addr: ids.as_mut_ptr() as usize as u32,
        ..Default::default()
    };
    check(
        sys_prx_get_module_list(0, &mut list) == 0 && ids.contains(&id),
        "SDK PRX list",
    );

    let mut filename = [0u8; SYS_PRX_MODULE_FILENAME_SIZE];
    let mut segments = [SysPrxSegmentInfo::default(); 8];
    let mut info = SysPrxModuleInfo {
        filename_addr: filename.as_mut_ptr() as usize as u32,
        filename_size: filename.len() as u32,
        segments_addr: segments.as_mut_ptr() as usize as u32,
        segments_count: segments.len() as u32,
        ..Default::default()
    };
    check(
        sys_prx_get_module_info(id, 0, &mut info) == 0
            && filename.windows(13).any(|bytes| bytes == b"libfiber.sprx")
            && info.total_segments > 0,
        "SDK PRX info",
    );
    check(
        sys_prx_get_module_id_by_address(segments[0].base as usize as *mut _) == id,
        "SDK PRX lookup by address",
    );

    let mut result = i32::MIN;
    check(
        sys_prx_start_module(id, 0, null_mut(), &mut result, 0, null_mut()) == 0
            && result == SYS_PRX_RESIDENT,
        "SDK PRX start",
    );
    check(
        sys_prx_stop_module(id, 0, null_mut(), &mut result, 0, null_mut()) == 0
            && result == SYS_PRX_STOP_SUCCESS,
        "SDK PRX stop",
    );
    check(sys_prx_unload_module(id, 0, null()) == 0, "SDK PRX unload");
    #[cfg(feature = "sdk-370")]
    check(sys::sys_prx_get_ppu_guid(id) == 0, "SDK unloaded PRX GUID");
}

#[cfg(feature = "sdk-debug")]
unsafe fn debug_query(thread_id: u64) {
    use ps3::sys::sdk::debug::{sys_dbg_get_ppu_thread_status, SysDbgPpuThreadStatus};

    let mut status = SysDbgPpuThreadStatus(u32::MAX);
    let result = sys_dbg_get_ppu_thread_status(thread_id, &mut status);
    if result == 0 && status.0 != u32::MAX {
        check(
            status.0 <= SysDbgPpuThreadStatus::UNKNOWN.0,
            "SDK debug thread status",
        );
    } else {
        sys::sys_tty_write("SDK debug query returned without thread status\n");
    }
}
