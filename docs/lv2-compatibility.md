# LV2 compatibility evidence

SDK features select the calling conventions in the headers and stub archives
of the named SDK. The firmware version and console type determine which
calls and imports are available to a process. The same firmware can provide
more than one selector for an operation.

## Reference material

The [PS3 Developer Wiki SDK index](https://www.psdevwiki.com/ps3/SCEI_PS3_SDK)
lists SDK releases before and after 1.92 and 3.70, including 4.75. Those two
Rust feature names refer to the SDK packages compared directly with this
implementation. The wiki's [LV2 selector table](https://www.psdevwiki.com/ps3/LV2_Functions_and_Syscalls)
is a reference for identifying selectors and permission classes.

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

RPCS3 smoke tests exercise both Cargo profiles on the same emulator setup.
The firmware table comparison is a separate static check. Debug thread
status currently reaches an RPCS3 implementation that does not fill the
result, so the test exercises the import without validating the returned
status.


## Selecting the older supported calls

Applications that target a range of firmware releases can use
`ps3::sys::portable` for these three operations. It selects 126 for the
try-write lock and 341/342 for memory containers, matching the public SDK
1.92 calling conventions. These entries are present in all six tables
above. The rest of the crate still uses the selected SDK feature.

Use the portable create and destroy functions as a pair. The wrappers
issue one syscall and report its status directly. They require a firmware
and process that support the selected call.
