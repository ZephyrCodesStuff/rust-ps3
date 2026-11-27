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

extern "C" {
    static lv2_smoke_thread: PpuFunctionDescriptor;
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
        events();
        synchronization();
        threads();
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
    check(sys_rwlock_destroy(id) == 0, "rwlock destroy");
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
