<h1 align="center">rust-ps3</h1>

<p align="center">
    <a href="https://www.rust-lang.org/">
        <img src="https://img.shields.io/badge/rust-nightly-orange.svg?style=flat-square">
    </a>
    <a href="https://github.com/rui314/mold">
        <img src="https://img.shields.io/badge/linker-mold-blue.svg?style=flat-square">
    </a>
    <a href="LICENSE">
        <img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg?style=flat-square">
    </a>
    <a href="#">
        <img src="https://img.shields.io/badge/platform-PlayStation%203%20(CellOS)-purple.svg?style=flat-square">
    </a>
</p>
<p align="center">
    A Rust runtime, SDK, and toolchain for the Sony PlayStation 3 (Cell Broadband Engine) <i>(formerly <code>hello-cell</code>)</i>.
</p>

```rust
#![no_std]
#![no_main]

use ps3::alloc::boxed::Box;
use ps3::alloc::format;
use ps3::allocator;
use ps3::println;
use ps3::sys::syscalls;

#[ps3::main]
fn main() -> i32 {
    println!("Hello PlayStation 3 from Rust!");

    let boxed_val = Box::new(0x1337_4242_u64);
    println!("Boxed Value: {:#X}", *boxed_val);

    let stats = allocator::get_heap_stats();
    println!("Active Allocs: {}, Active Bytes: {} bytes", stats.active_allocations, stats.active_bytes);

    0
}
```

See `examples/` directory for sample programs (`examples/hello-world` and `examples/http-server`).

## Features / Roadmap

- [x] `core` & `alloc` support (`talc` fast allocator backed by `sys_memory_allocate` / `sys_memory_free`)
- [x] PowerPC64 ELFv1 ABI support (Big-Endian Cell PPU)
- [x] LV2 Syscall dispatcher (direct inline assembly syscalls via `sc 11`)
- [x] TTY / Standard I/O output via `sys_tty_write`
- [x] `moldier`: ELF post-linker and OPD function descriptor patcher for `mold`
- [x] Automated SPRX dynamic stub generator (`sprx.toml`) and linker binding
- [x] Network socket layer (`sys_net` / `cellSysmodule` stubs, TCP listener & streams)
- [x] Direct compilation via Rust `json-target-spec` without legacy cross-compiler toolchains
- [ ] SPU (Synergistic Processing Unit) toolchain & inter-processor communication (MFC DMA / mailboxes)
- [ ] RSX / libgcm 3D graphics hardware acceleration
- [ ] `std` support
- [ ] Automated self / `EBOOT.BIN` packaging

### What about PSL1GHT / Cell SDK?

Traditional PS3 homebrew relies on the legacy GCC-based **PSL1GHT** SDK or the official Sony Cell SDK. 

`rust-ps3` (formerly `hello-cell`) is a **standalone, modern toolchain and runtime** built directly on top of upstream LLVM/rustc:
- Targets 64-bit PowerPC ELFv1 (`powerpc64-unknown-linux-gnu` reference target spec, customized to `powerpc64-sony-ps3` for CellOS and PPU).
- Links binaries using the modern high-performance [`mold`](https://github.com/rui314/mold) linker.
- Patches the final binary with **`moldier`**, a small(ish) post-linker tool that generates SPRX import stubs from TOML definitions and patches Sony LV2 Official Procedure Descriptors (OPD) and TOC relocations directly into the final ELF.

## Dependencies

To compile for the PS3, you will need a Rust **nightly** toolchain with the `rust-src` component:

```sh
$ rustup default nightly && rustup component add rust-src
```

You also need the [`mold`](https://github.com/rui314/mold) linker installed and available in your `PATH`:

```sh
# macOS (Homebrew)
$ brew install mold

# Arch Linux
$ pacman -S mold

# Ubuntu / Debian
$ apt install mold
```

*(Optional)* If you want to automatically generate signed `EBOOT.BIN` files for retail/CFW PS3s, install `make_fself` into your `PATH`.

## Building & Running Examples

To build one of the examples (e.g. `hello-world` or `http-server`), run:

```sh
# Linux / macOS
$ ./scripts/build.sh hello-world
# or
$ ./scripts/build.sh http-server

# Windows (PowerShell)
$ .\scripts\build.ps1 -TargetExample hello-world
# or
$ .\scripts\build.ps1 -TargetExample http-server
```

The build script will:
1. Compile the host tool `moldier`.
2. Generate assembly stubs (`ps3/src/sys/sprx.s`) from `ps3/sprx.toml`.
3. Compile and link the chosen example package using `rustc` and `mold`.
4. Run `moldier patch` on the output ELF to resolve LV2 OPD descriptors and SPRX imports.
5. Produce `EBOOT.ELF` (and `EBOOT.BIN` if `make_fself` is available) in the project root.

## Running

### RPCS3 Emulator

You can run `EBOOT.ELF` directly in [RPCS3](https://rpcs3.net/).

### Real Hardware (PS3 with CFW / PS3HEN)

1. Place `EBOOT.BIN` inside a game directory on your PS3 internal HDD or USB drive:
   ```
   /dev_hdd0/game/HELLOCELL/USRDIR/EBOOT.BIN
   ```
2. Alternatively, mount and run directly via `webMAN MOD` or `/app_home/PS3_GAME/`.
3. Even more alternatively, use a Target Manager to upload the binary to your PS3 and run it.

## LV2 syscall bindings

`ps3::sys::raw` contains the direct LV2 calls defined inline by the official
PPU SDK, with Rust types for their arguments and results. The target uses
64-bit registers and Rust pointers. Embedded SDK pointers and `size_t`
fields retain their 32-bit CellOS representation. Compile-time assertions
check the sizes, alignments and field offsets of the ABI records.

```rust,ignore
use ps3::sys::error::Lv2Error;
use ps3::sys::raw::{sys_memory_get_user_memory_size, SysMemoryInfo};

let mut info = SysMemoryInfo::default();
Lv2Error::check(unsafe { sys_memory_get_user_memory_size(&mut info) })?;
let available = info.available_user_memory;
```

The default feature `sdk-370` selects the SDK 3.70 ABI and also matches the
public LV2 function signatures and selectors in SDKs 4.60 and 4.75. For an
older ABI, select `sdk-270` or `sdk-192` when declaring the dependency.

```toml
ps3 = { version = "0.1", default-features = false, features = ["sdk-192"] }
```

| Call | SDK 1.92 | SDK 2.70 | SDK 3.70 / 4.60 / 4.75 |
| --- | ---: | ---: | ---: |
| `sys_memory_container_create` | 341 | 324 | 324 |
| `sys_memory_container_destroy` | 342 | 325 | 325 |
| `sys_rwlock_trywlock` | 126 | 126 | 148 |

Select one feature across the dependency graph. The selection is made at
compile time. Check the running firmware and process permissions before
using an entry point. Some firmware revisions expose the two memory
container selector pairs under different permissions. See the
[firmware and PSL1GHT comparison](docs/lv2-compatibility.md) for the
versions and evidence behind these differences.

Public SDK helpers for memory mapping and timer scheduling are available
under `ps3::sys`, alongside the compatibility helpers for process exit,
thread creation and PRX start and stop. Cell APIs such as file I/O and
sockets use their SDK library imports.

Enable the `sdk-imports` feature to access `ps3::sys::sdk`. It contains typed
firmware calls for PRX management, PPU threads, lightweight synchronization,
SPU images, memory pools and related system services. These functions use the
running firmware's `sysPrxForUser` library. The build adds the SDK import IDs
to the application's SPRX import table, which `moldier patch` finalizes.
The selected ABI controls which symbols are imported. Enable `sdk-debug`
to also expose `ps3::sys::sdk::debug` on firmware with the `sys_lv2dbg`
library and the required debug permissions.

Enable `sdk-crashdump` to use the public crash dump log area functions under
`ps3::sys::sdk::crashdump`. They use the firmware's `sys_crashdump` library.
The firmware and executable configuration determine whether a process can
produce crash dump files.

The `sdk-370` profile also provides `SysPrxModuleInfoV2` and the associated
import and export records from SDKs 4.60 and 4.75. Query the extended record
using `sys_prx_get_module_info_v2` on supporting firmware.

Use the audit script to compare the direct selectors and ABI signatures
against local SDK headers.

```sh
python3 scripts/audit-lv2.py --public-sdk-headers INCLUDE_DIR --sdk-abi sdk-370
```

Add `--psl1ght-headers "$PS3DEV/ppu/include"` to compare inline selector
coverage and argument counts with a local PSL1GHT installation. Pass
`--lv2-stub-archive PATH_TO_LIBLV2_STUB_A --ppu-bin "$PS3DEV/ppu/bin"` to
also compare the public library signatures and import IDs. The debug and
crash dump libraries use `--lv2-debug-stub-archive PATH_TO_LIBLV2DBG_STUB_A`
and `--crashdump-stub-archive PATH_TO_LIBCRASHDUMP_STUB_A`. The archive checks
use Python 3.11 or later.

To build the smoke program with a PS3DEV GNU linker:

```sh
CARGO_ENCODED_RUSTFLAGS='' cargo -Z unstable-options -Z build-std=core,alloc \
    -Z build-std-features=compiler-builtins-mem rustc --locked --release -p lv2-smoke -- \
    -C "linker=$PS3DEV/ppu/bin/ppu-ld" -C link-arg=-T \
    -C "link-arg=$PWD/tests/lv2-smoke/link.ld" -C link-arg=--no-opd-optimize \
    -C link-arg=--no-toc-optimize -C link-arg=--no-multi-toc
```

For `sdk-192` or `sdk-270`, pass `--no-default-features` and the selected
`--features` value before the `--`. Boot
`target/powerpc64-sony-ps3/release/lv2-smoke.ELF` in
RPCS3. The program prints `LV2 ABI smoke passed` after checking process and
time calls, memory allocation, mapped memory and containers, event queues,
timers, semaphores, read/write locks, and thread creation and join.

For the firmware import tests, build the smoke program with `--features
sdk-imports` before the `--` and add `-C link-arg=-u -C
link-arg=sys_proc_prx_param` after it. Use
`--no-default-features --features sdk-192,sdk-imports` for the 1.92 variant.
Run `moldier patch` on the resulting ELF before booting it. This variant
also checks the firmware thread wrappers, lightweight mutex and condition
variable, memory pool and PRX lifecycle using the firmware's `libfiber.sprx`.
Use `sdk-debug` in place of `sdk-imports` to try a thread status query.
RPCS3 can resolve the debug import even when its implementation leaves the
output record untouched.

## SPRX Dynamic Imports (`sprx.toml`)

`rust-ps3` allows defining Sony PRX module imports cleanly in `sprx.toml`. `moldier` parses this file, generates the appropriate assembly stubs, and binds them to the binary:

```toml
[libraries.cellSysmodule]
module_id = 0x0009
functions = [
    { name = "sysModuleLoad", fnid = 0x32267A31 },
    { name = "sysModuleUnload", fnid = 0x112A5EE9 },
]

[libraries.sys_net]
module_id = 0x0009
functions = [
    { name = "netInitializeNetworkEx", fnid = 0x139a9e9b },
    { name = "netSocket", fnid = 0x9c056962 },
    { name = "netConnect", fnid = 0x64f66d35 },
    { name = "netBind", fnid = 0xb0a59804 },
    { name = "netListen", fnid = 0x28e208bb },
    { name = "netAccept", fnid = 0xc94f6939 },
]
```

## Debugging

Debug symbols are preserved in `EBOOT.ELF`. You can inspect and debug binaries using:
- **RPCS3 Built-in Debugger / GDB Server**: Enable GDB debugging in RPCS3 settings and connect `gdb-multiarch` or `powerpc64-linux-gnu-gdb`.
- **TTY Output**: View real-time standard output and panic messages via `sys_tty_write` in the RPCS3 log window or PS3 Net Server.

### Useful tools

- [OpenTM](https://github.com/sagemono/opentm) - A fully open-source Target Manager for the PlayStation 3.
- [scetool](https://github.com/naehrwert/scetool) - A tool for signing and verifying PS3 EBOOTs and SPRXs.

## License

See [LICENSE](LICENSE).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
