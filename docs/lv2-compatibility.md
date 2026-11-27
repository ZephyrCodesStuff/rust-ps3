# LV2 compatibility evidence

SDK features select the calling conventions in the headers and stub archives
of the named SDK. The firmware version and console type determine which
calls and imports are available to a process. The same firmware can provide
more than one selector for an operation.

## Reference material

The public PPU headers and stub libraries from these SDK releases were
compared directly with the Rust bindings. The audit compares the normalized
parameter and return types, plus the direct selectors and firmware import
IDs. Separate C layout checks cover the PRX, memory pool, lightweight
synchronization, and debug records.

| SDK | Rust profile | Inline calls | `sysPrxForUser` imports | `sys_lv2dbg` imports |
| --- | --- | ---: | ---: | ---: |
| 1.92 | `sdk-192` | 137 | 58 | 21 |
| 2.70 | `sdk-270` | 162 | 60 | 34 |
| 3.70 | `sdk-370` | 165 | 62 | 35 |
| 4.60 | `sdk-370` | 165 | 62 | 35 |
| 4.75 | `sdk-370` | 165 | 62 | 35 |

SDK 2.70 uses selector 126 for the RW lock try-write operation and 324/325
for memory containers. It has the same signatures as 3.70 for the functions
they share. SDKs 4.60 and 4.75 provide a larger PRX module information record
whose size selects the extended result through the existing firmware import.

SDK 4.75 also provides both public log area functions in
`libcrashdump_stub.a`. The Rust `sdk-crashdump` feature exposes those
functions and their 24-byte log area record.

The PS3 Developer Wiki indexes further SDK releases and documents LV2
selectors and permission classes. [SDK index](https://www.psdevwiki.com/ps3/SCEI_PS3_SDK),
[LV2 selector table](https://www.psdevwiki.com/ps3/LV2_Functions_and_Syscalls)

PSL1GHT is a separate, useful reference for homebrew behavior. These snapshots
use the same IDs for the eight inline memory operations in `sys/memory.h`,
including 324 and 325 for memory containers.

| PSL1GHT snapshot | Distinct inline selectors | Shared with SDK 3.70 |
| --- | ---: | ---: |
| [2011](https://github.com/ps3dev/PSL1GHT/tree/a8bb7c803ebd458ae1fb3932ad697a30e5632ba0/ppu/include) | 125 | 95 |
| [2020](https://github.com/ps3dev/PSL1GHT/tree/6e565a70e927f55813babe86dc6f64149535f908/ppu/include) | 131 | 95 |
| [2026](https://github.com/ps3dev/PSL1GHT/tree/dd4e8c9be744e3273d665c393fa6d97a87402e2e/ppu/include) | 137 | 95 |

The argument count of each shared selector matches SDK 3.70 in all three
snapshots. PSL1GHT also exposes some underlying filesystem, socket and USB
traps that the official SDK accesses through libraries. A selector match
alone does not establish a structure layout.

## Firmware dispatch tables

The table records whether selected entries route to a handler in the
locally examined stock firmware binaries. A handler entry alone does not
establish process permission, full behavior, or binary compatibility.

| Firmware | RW lock 126 | RW lock 148 | Container 324/325 | Container 341/342 | FSW 633/634 |
| --- | --- | --- | --- | --- | --- |
| DECR 1.10 | Handler | ENOSYS | ENOSYS | Handler | ENOSYS |
| DECR 2.60 | Handler | ENOSYS | Handler | Handler | Handler |
| DEX 3.55 | Handler | Handler | Handler | Handler | ENOSYS |
| DEX 4.81 | Handler | Handler | Handler | Handler | ENOSYS |
| DECR 4.81 | Handler | Handler | Handler | Handler | Handler |
| CEX 4.93 | Handler | Handler | Handler | Handler | ENOSYS |

The SDK 1.92 RW lock selector 126 is present in these six tables. The SDK
3.70 selector 148 is absent in both early DECR samples. The CEX 4.93 table
has both pairs of memory container selectors. The 324/325 pair has a debug
application flag requirement, as documented in the [LV2 selector table](https://www.psdevwiki.com/ps3/LV2_Functions_and_Syscalls).
A broad firmware version check alone would miss these differences.

RPCS3 smoke tests exercise all three Cargo profiles on the same emulator
setup, including the extended PRX query under `sdk-370`. The firmware table
comparison is a separate static check. Debug thread
status currently reaches an RPCS3 implementation that does not fill the
result, so the test exercises the import without validating the returned
status. RPCS3 also resolves both crash dump imports and rejects an invalid
log area index. Dump file generation remains unverified.

## Selecting the older supported calls

Applications that target a range of firmware releases can use
`ps3::sys::portable` for these three operations. It selects 126 for the
try-write lock and 341/342 for memory containers, matching the public SDK
1.92 calling conventions. These entries are present in all six tables
above. The rest of the crate still uses the selected SDK feature.

Use the portable create and destroy functions as a pair. The wrappers
issue one syscall and report its status directly. They require a firmware
and process that support the selected call.

## Firmware library exports

These counts compare the Rust `sysPrxForUser` import IDs against the actual
export tables in the locally examined `liblv2.sprx` binaries. They concern the
optional `sdk-imports` feature.

| Firmware | SDK 1.92 exports found | SDK 2.70 exports found | SDK 3.70 exports found |
| --- | ---: | ---: | ---: |
| DECR 1.10 | 49 / 58 | 49 / 60 | 49 / 62 |
| DECR 2.60 | 58 / 58 | 60 / 60 | 60 / 62 |
| DEX 4.81 | 58 / 58 | 60 / 60 | 62 / 62 |
| DECR 4.81 | 58 / 58 | 60 / 60 | 62 / 62 |
| CEX 4.93 | 58 / 58 | 60 / 60 | 62 / 62 |

The DECR 2.60 sample lacks the 3.70 imports for `sys_get_random_number`
and `sys_prx_load_module_on_memcontainer_by_fd`. On DECR 1.10, even some
1.92 imports are absent, including the lightweight condition-variable
functions and file descriptor based PRX loading.

The same comparison for the optional `sys_lv2dbg` imports uses
`liblv2dbg_for_dex.sprx` on the test and development firmware and
`liblv2dbg_for_cex.sprx` on retail firmware. A matching export establishes
linkage availability. Debug permissions and returned data depend on the
process and firmware.

| Firmware | SDK 1.92 debug exports found | SDK 2.70 debug exports found | SDK 3.70 debug exports found |
| --- | ---: | ---: | ---: |
| DECR 1.10 | 6 / 21 | 6 / 34 | 6 / 35 |
| DECR 2.60 | 21 / 21 | 32 / 34 | 32 / 35 |
| DEX 4.81 | 21 / 21 | 34 / 34 | 35 / 35 |
| DECR 4.81 | 21 / 21 | 34 / 34 | 35 / 35 |
| CEX 4.93 | 21 / 21 | 34 / 34 | 35 / 35 |

Both `sys_crashdump` NIDs are present in `liblv2coredump.sprx` on
DEX 4.81, DECR 4.81, and CEX 4.93.

The archive audit covers public declarations with function symbols in the
SDK's `liblv2_stub.a`, `liblv2dbg_stub.a`, and `libcrashdump_stub.a`.
`sys_prx_register_module` and `sys_spu_thread_elf_loader` are declared in
the 3.70 and 4.75 headers. A symbol scan of the PPU libraries found only
the underscore-prefixed PRX registration trap in `libsyscall.a`, which
has a different argument list.
