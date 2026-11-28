#!/usr/bin/env python3
"""Compare the public Rust LV2 bindings with a local PPU SDK header tree."""

import argparse
from dataclasses import dataclass
from pathlib import Path
import re
import subprocess

PS3 = Path(__file__).resolve().parents[1] / "ps3"
SYSCALLS = PS3 / "src/sys/syscalls"
SDK_DEFINE = re.compile(r"^\s*#\s*define\s+(_?SYS_[A-Z0-9_]+)\s+(\d+)\b", re.M)
SDK_INLINE = re.compile(
    r"\b(?:inline|__inline__)\s+([\w ]+?)\s+(\w+)\s*\(([^;{}()]*)\)\s*"
    r"\{([^{}]*(?:\{[^{}]*\}[^{}]*)*)\}"
)
SDK_CALL = re.compile(r"\bsystem_call_([0-8])\s*\(\s*(_?SYS_[A-Z0-9_]+)\b")
RUST_NUMBER = re.compile(r"^pub const (_?SYS_[A-Z0-9_]+): u64 = (\w+);", re.M)
RUST_BINDING = re.compile(
    r"\bpub unsafe fn (\w+)\(([^{}()]*)\)\s*(?:->\s*(\w+)\s*)?"
    r"(?:=\s*(_?SYS_[A-Z0-9_]+);|\{(?P<body>.*?)^\})",
    re.M | re.S,
)
RUST_IMPORT = re.compile(r"^    pub fn (\w+)\(([^()]*)\)\s*(?:->\s*([^;]+))?;", re.M)
C_TYPES = {
    "void": "c_void",
    "int": "i32",
    "int32_t": "i32",
    "char": "u8",
    "uint8_t": "u8",
    "unsigned int": "u32",
    "unsigned long": "u32",
    "uint32_t": "u32",
    "size_t": "u32",
    "second_t": "u32",
    "uint64_t": "u64",
    "usecond_t": "u64",
    "off64_t": "i64",
    "system_time_t": "i64",
    "sys_time_nsec_t": "i64",
    "sys_time_sec_t": "i64",
    "sys_raw_spu_t": "i32",
    "sys_semaphore_value_t": "i32",
    "sys_deci3_session_t": "i32",
    "sys_deci3_port_t": "u8",
    "sys_raw_spu_attribute_t": "c_void",
    "sys_prx_id_t": "SysPrxId",
    "sys_mempool_t": "SysMempool",
    "dbg_exception_handler_t": "*PpuFunctionDescriptor",
    "CellSsOpenPSID": "CellSsOpenPsid",
    "sys_crash_dump_log_area_info_t": "SysCrashDumpLogAreaInfo",
    "sys_overlay_module_info2_t": "SysOverlayModuleInfo2",
    "sys_page_attr_t": "SysPageAttribute",
    "sys_ppu_thread_icontext_t": "SysPpuThreadInterruptContext",
    "sys_prx_get_module_list_t": "SysPrxModuleList",
    "sys_vm_page_information_t": "SysVmPageInformation",
}
for rust, names in (
    ("u32", "addr class_id cond deci3_protocol event_flag event_port event_queue event_type "
            "hw_thread interrupt_tag interrupt_thread_handle lwcond_pseudo_id lwmutex_pseudo_id "
            "memory_container memory mutex overlay pid rwlock semaphore spu_thread_group spu_thread timer"),
    ("u64", "ipc_key irqoutlet_id ppu_thread prx_flags"),
):
    C_TYPES.update({f"sys_{name}_t": rust for name in names.split()})

for name in (
    "cond_attribute dbg_cond_information dbg_coredump_parameter dbg_event_flag_information "
    "dbg_event_queue_information dbg_lwcond_information dbg_lwmutex_information dbg_mutex_information "
    "dbg_ppu_thread_context dbg_ppu_thread_status dbg_rwlock_information dbg_semaphore_information "
    "dbg_spu_thread_context dbg_spu_thread_context2 dbg_spu_thread_group_status event event_flag_attribute "
    "event_queue_attribute lwcond lwcond_attribute lwmutex lwmutex_attribute memory_info mutex_attribute "
    "overlay_module_info ppu_thread_stack prx_get_module_id_by_name_option prx_load_module_list_option "
    "prx_load_module_option prx_module_info prx_start_module_option prx_stop_module_option "
    "prx_unload_module_option rwlock_attribute semaphore_attribute spu_image spu_segment spu_thread_argument "
    "spu_thread_attribute spu_thread_group_attribute timer_information vm_statistics"
).split():
    C_TYPES[f"sys_{name}_t"] = "Sys" + "".join(part.capitalize() for part in name.split("_"))

EXIT_COMPAT_CONSTANTS = {"SYS_PROCESS_EXIT", "SYS_PPU_THREAD_EXIT"}


@dataclass(frozen=True)
class Binding:
    selector: str
    number: int
    signature: tuple

    @property
    def arity(self):
        return len(self.signature[1])


def header_source(path):
    source = path.read_text(encoding="utf-8", errors="replace")
    return re.sub(r"/\*.*?\*/|//[^\n]*", "", source, flags=re.S)


def parameters(args):
    args = args.strip().rstrip(",")
    if args in ("", "void", "CDECL_VOID"):
        return []
    depth, start = 0, 0
    result = []
    for index, char in enumerate(args):
        if char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
        elif char == "," and depth == 0:
            result.append(args[start:index].strip())
            start = index + 1
    result.append(args[start:].strip())
    return result


def c_type(declaration, *, parameter=False):
    declaration = " ".join(re.sub(r"\b(?:__CSTD|CDECL|extern|const|volatile)\b", "", declaration).split())
    callback = re.fullmatch(r"void\s*\(\s*\*\s*\w+\s*\)\s*\(\s*(void|uint64_t)\s*\)", declaration)
    if callback:
        return "*PpuFunctionDescriptor"
    if re.fullmatch(r"char\s*(?:\*\s*\*\s*\w+|\*\s*\w+\s*\[\s*\])", declaration):
        # CellOS uses arrays of 32-bit effective addresses.
        return "*u32"
    base = declaration.replace("*", " ").strip()
    if parameter and base not in C_TYPES:
        base = re.sub(r"\b\w+$", "", base).strip()
    if base not in C_TYPES:
        raise ValueError(f"unrecognized SDK ABI type: {declaration}")
    if base == "void" and "*" not in declaration:
        return "()"
    return "*" * declaration.count("*") + C_TYPES[base]


def c_signature(ret, args, name):
    try:
        types = tuple(c_type(param, parameter=True) for param in parameters(args))
        if name in {"sys_tty_read", "sys_tty_write"} and types[1] == "*c_void":
            # Rust gives the byte buffer its element type.
            types = types[:1] + ("*u8",) + types[2:]
        return c_type(ret), types
    except ValueError as error:
        raise ValueError(f"{name}: {error}") from error


def rust_signature(ret, args):
    def abi_type(declaration):
        return re.sub(r"\*(?:const|mut)\s+", "*", declaration.strip())

    return abi_type(ret or "()"), tuple(abi_type(param.split(":", 1)[1]) for param in parameters(args))


def sdk_bindings(include):
    header = include / "sys/syscall.h"
    numbers = dict(SDK_DEFINE.findall(header.read_text(encoding="utf-8")))
    bindings = {}
    observed = set()
    for path in include.rglob("*.h"):
        source = header_source(path).replace("__CSTD ", "")
        observed.update(selector for _, selector in SDK_CALL.findall(source))
        for ret, name, args, body in SDK_INLINE.findall(source):
            call = SDK_CALL.search(body)
            if call is None:
                continue
            arity, selector = call.groups()
            signature = c_signature(ret, args, name)
            if selector not in numbers or int(arity) != len(signature[1]):
                raise ValueError(f"{path}: cannot resolve the direct ABI of {name}")
            if name in bindings:
                raise ValueError(f"{path}: duplicate inline syscall wrapper {name}")
            bindings[name] = Binding(selector, int(numbers[selector]), signature)
    if not bindings:
        raise ValueError(f"no inline syscall wrappers found in {include}")
    missed = observed - {item.selector for item in bindings.values()}
    if missed:
        raise ValueError(f"unrecognized inline SDK selectors: {', '.join(sorted(missed))}")
    return bindings


def rust_source(path, abi):
    source = path.read_text(encoding="utf-8")
    return re.sub(
        r'^[ \t]*#\[cfg\((feature = "sdk-\d+"|any\(feature = "sdk-\d+"(?:, feature = "sdk-\d+")+\))\)\]\n'
        r'((?:[^\n]+\n)*?[^\n]*?[;{])',
        lambda match: match[2] if abi in re.findall(r'"(sdk-\d+)"', match[1]) else "",
        source,
        flags=re.M,
    )


def rust_bindings(abi):
    numbers = dict(RUST_NUMBER.findall(rust_source(SYSCALLS / "nr.rs", abi)))
    raw = SYSCALLS / "raw"
    modules = re.findall(r"^pub use (\w+)::\*;", (raw / "mod.rs").read_text(), re.M)
    bindings = {}
    for module in modules:
        source = rust_source(raw / f"{module}.rs", abi)
        for name, args, ret, selector, body in RUST_BINDING.findall(source):
            if not selector:
                trap = re.search(r"\blv2_trap!\(nr::(_?SYS_[A-Z0-9_]+);", body)
                if trap is None:
                    raise ValueError(f"raw::{name} has no typed selector declaration")
                selector = trap[1]
            if name in bindings or selector not in numbers:
                raise ValueError(f"duplicate wrapper or missing public selector for raw::{name}")
            bindings[name] = Binding(selector, int(numbers[selector]), rust_signature(ret, args))
    referenced = {item.selector for item in bindings.values()}
    extras = numbers.keys() - referenced - EXIT_COMPAT_CONSTANTS
    if extras:
        raise ValueError(f"public selector constants without wrappers: {', '.join(sorted(extras))}")
    return bindings


def sdk_stub_ids(archive, ppu_bin):
    """Read Sony archive member names, function symbols and NID records."""
    def tool(name, *args):
        executable = str(ppu_bin / name) if ppu_bin else name
        return subprocess.check_output([executable, *args, str(archive)], text=True)

    symbols = dict(re.findall(
        r"^([^\n:]+\.o):\n[0-9a-f]+ T \.(\w+)$", tool("ppu-nm", "--defined-only"), re.M
    ))
    records = dict(re.findall(
        r"^([^\n:]+\.o):\s*file format[^\n]*\s+Contents of section[^\n]*"
        r"\n 0000 [0-9a-f]{8} [0-9a-f]{8} ([0-9a-f]{8})\b",
        tool("ppu-objdump", "-s", "-j", ".psp_libgen_markfunc"),
        re.M,
    ))
    if not symbols or not records:
        raise ValueError(f"no SDK function NID records found in {archive}")
    if symbols.keys() - records.keys():
        missing = ", ".join(sorted(symbols.keys() - records.keys()))
        raise ValueError(f"missing SDK function NID records in {archive}: {missing}")
    return {symbol: int(records[member], 16) for member, symbol in symbols.items()}


def audit_sdk_library(include, archive, ppu_bin, abi, library):
    """Check the public header/stub intersection against the Rust imports."""
    import tomllib

    stub_ids = sdk_stub_ids(archive, ppu_bin)
    declarations = re.compile(
        r"^[ \t]*(?!return\b)([A-Za-z_][\w \t*]*?)(?<=[ \t*])("
        + "|".join(map(re.escape, stub_ids)) + r")\s*"
        r"\(((?:[^();{}]|\([^()]*\))*)\)\s*(?:__attribute__[^;]*)?;",
        re.M,
    )
    expected = {}
    for path in include.rglob("*.h"):
        # Some SDK headers include declarations for the internal build. The
        # public ABI is the part available with __LV2_EXTERNAL_RELEASE__.
        source = re.sub(
            r"#ifndef __LV2_EXTERNAL_RELEASE__\b.*?#endif\s*/\* __LV2_EXTERNAL_RELEASE__ \*/",
            "", path.read_text(encoding="utf-8", errors="replace"), flags=re.S,
        )
        for ret, name, args in declarations.findall(re.sub(r"/\*.*?\*/|//[^\n]*", "", source, flags=re.S)):
            expected.setdefault(name, set()).add(c_signature(ret, args, name))

    config = tomllib.loads((PS3 / "sdk-imports.toml").read_text())
    imports = {
        item["name"]: item["fnid"]
        for item in config["libraries"][library]["functions"]
        if item.get("since_sdk", 0) <= int(abi.removeprefix("sdk-"))
    }
    rust = {
        symbol: rust_signature(ret, args)
        for path in (PS3 / "src/sys/sdk").glob("*.rs")
        for symbol, args, ret in RUST_IMPORT.findall(rust_source(path, abi))
    }
    failures = []
    failures.extend(f"missing SDK library import: {name}" for name in sorted(expected.keys() - imports.keys()))
    failures.extend(f"extra SDK library import: {name}" for name in sorted(imports.keys() - expected.keys()))
    failures.extend(f"missing Rust SDK declaration: {name}" for name in sorted(imports.keys() - rust.keys()))
    # Declarations in other modules may be added for other firmware libraries.
    failures.extend(f"SDK declaration without import: {name}" for name in sorted((rust.keys() & stub_ids.keys()) - imports.keys()))
    for name in sorted(expected.keys() & imports.keys()):
        if stub_ids[name] != imports[name]:
            failures.append(f"{name}: SDK FNID 0x{stub_ids[name]:08x}, Rust FNID 0x{imports[name]:08x}")
        if name in rust and rust[name] not in expected[name]:
            failures.append(f"{name}: SDK signatures {sorted(expected[name])}, Rust signature {rust[name]}")
    if failures:
        raise ValueError("\n".join(failures))
    print(f"{abi}: {len(expected)} public {library} imports match the SDK signatures and stub FNIDs")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--public-sdk-headers", type=Path, required=True, metavar="INCLUDE_DIR")
    parser.add_argument("--sdk-abi", choices=("sdk-192", "sdk-270", "sdk-370"), default="sdk-370")
    parser.add_argument("--lv2-stub-archive", type=Path, metavar="LIBLV2_STUB_A")
    parser.add_argument("--lv2-debug-stub-archive", type=Path, metavar="LIBLV2DBG_STUB_A")
    parser.add_argument("--crashdump-stub-archive", type=Path, metavar="LIBCRASHDUMP_STUB_A")
    parser.add_argument("--ppu-bin", type=Path, metavar="TOOL_DIRECTORY",
                        help="directory containing ppu-nm and ppu-objdump (defaults to PATH)")
    args = parser.parse_args()
    try:
        sdk = sdk_bindings(args.public_sdk_headers)
        rust = rust_bindings(args.sdk_abi)
    except (OSError, KeyError, ValueError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"{error}\n")

    errors = []
    errors.extend(f"missing Rust wrapper: {name}" for name in sorted(sdk.keys() - rust.keys()))
    errors.extend(f"extra Rust wrapper: {name}" for name in sorted(rust.keys() - sdk.keys()))
    for name in sorted(rust.keys() & sdk.keys()):
        if sdk[name] != rust[name]:
            errors.append(f"{name}: SDK {sdk[name]}, Rust {rust[name]}")
    if errors:
        parser.exit(1, "\n".join(errors) + "\n")
    print(f"{args.sdk_abi}: all {len(sdk)} public inline wrappers match by name, selector and ABI signature")
    for archive, library in ((args.lv2_stub_archive, "sysPrxForUser"),
                             (args.lv2_debug_stub_archive, "sys_lv2dbg"),
                             (args.crashdump_stub_archive, "sys_crashdump")):
        if archive:
            try:
                audit_sdk_library(args.public_sdk_headers, archive, args.ppu_bin, args.sdk_abi, library)
            except (OSError, KeyError, ValueError, subprocess.CalledProcessError) as error:
                parser.exit(1, f"{error}\n")


if __name__ == "__main__":
    main()
