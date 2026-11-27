#!/usr/bin/env python3
"""Compare the public Rust LV2 bindings with a local PPU SDK header tree."""

import argparse
from dataclasses import dataclass
from pathlib import Path
import re

SYSCALLS = Path(__file__).resolve().parents[1] / "ps3/src/sys/syscalls"
SDK_DEFINE = re.compile(r"^\s*#\s*define\s+(_?SYS_[A-Z0-9_]+)\s+(\d+)\b", re.M)
SDK_INLINE = re.compile(
    r"\b(?:inline|__inline__)\s+([\w ]+?)\s+(\w+)\s*\(([^;{}()]*)\)\s*"
    r"\{([^{}]*(?:\{[^{}]*\}[^{}]*)*)\}"
)
SDK_CALL = re.compile(r"\bsystem_call_([0-8])\s*\(\s*(_?SYS_[A-Z0-9_]+)\b")
PSL1GHT_CALL = re.compile(r"\blv2syscall([0-8])\s*\(\s*(\d+)\s*[,)]")
RUST_NUMBER = re.compile(r"^pub const (_?SYS_[A-Z0-9_]+): u64 = (\w+);", re.M)
RUST_BINDING = re.compile(
    r"\bpub unsafe fn (\w+)\(([^{}()]*)\)\s*(?:->\s*(\w+)\s*)?"
    r"(?:=\s*(_?SYS_[A-Z0-9_]+);|\{(?P<body>.*?)^\})",
    re.M | re.S,
)
RETURNS = {"void": "()", "int": "i32", "uint64_t": "u64", "sys_pid_t": "u32", "sys_addr_t": "u32"}
EXIT_COMPAT_CONSTANTS = {"SYS_PROCESS_EXIT", "SYS_PPU_THREAD_EXIT"}


@dataclass(frozen=True)
class Binding:
    selector: str
    number: int
    arity: int
    return_type: str


def header_source(path):
    source = path.read_text(encoding="utf-8", errors="replace")
    return re.sub(r"/\*.*?\*/|//[^\n]*", "", source, flags=re.S)


def parameter_count(args):
    args = args.strip().rstrip(",")
    return 0 if args in ("", "void") else args.count(",") + 1


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
            if selector not in numbers or int(arity) != parameter_count(args):
                raise ValueError(f"{path}: cannot resolve the direct ABI of {name}")
            if name in bindings:
                raise ValueError(f"{path}: duplicate inline syscall wrapper {name}")
            bindings[name] = Binding(selector, int(numbers[selector]), int(arity), RETURNS[ret.strip()])
    if not bindings:
        raise ValueError(f"no inline syscall wrappers found in {include}")
    missed = observed - {item.selector for item in bindings.values()}
    if missed:
        raise ValueError(f"unrecognized inline SDK selectors: {', '.join(sorted(missed))}")
    return bindings


def psl1ght_selectors(include):
    if not (include / "ppu-lv2.h").is_file():
        raise ValueError(f"missing PSL1GHT PPU header: {include / 'ppu-lv2.h'}")
    selectors = {}
    for path in include.rglob("*.h"):
        for arity, number in PSL1GHT_CALL.findall(header_source(path)):
            selectors.setdefault(int(number), set()).add(int(arity))
    if not selectors:
        raise ValueError(f"no inline PSL1GHT syscalls found in {include}")
    return selectors


def rust_source(path, abi):
    source = path.read_text(encoding="utf-8")
    return re.sub(
        r'^([ \t]*)#\[cfg\(feature = "(sdk-\d+)"\)\]\n((?:[^\n]+\n)*?[^\n]*?[;{])',
        lambda match: match[3] if match[2] == abi else "",
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
            bindings[name] = Binding(selector, int(numbers[selector]), parameter_count(args), ret or "()")
    referenced = {item.selector for item in bindings.values()}
    extras = numbers.keys() - referenced - EXIT_COMPAT_CONSTANTS
    if extras:
        raise ValueError(f"public selector constants without wrappers: {', '.join(sorted(extras))}")
    return bindings


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--public-sdk-headers", type=Path, required=True, metavar="INCLUDE_DIR")
    parser.add_argument("--sdk-abi", choices=("sdk-370", "sdk-192"), default="sdk-370")
    parser.add_argument("--psl1ght-headers", type=Path, metavar="INCLUDE_DIR")
    args = parser.parse_args()
    try:
        sdk = sdk_bindings(args.public_sdk_headers)
        rust = rust_bindings(args.sdk_abi)
        psl1ght = psl1ght_selectors(args.psl1ght_headers) if args.psl1ght_headers else None
    except (OSError, KeyError, ValueError) as error:
        parser.exit(1, f"{error}\n")

    errors = []
    errors.extend(f"missing Rust wrapper: {name}" for name in sorted(sdk.keys() - rust.keys()))
    errors.extend(f"extra Rust wrapper: {name}" for name in sorted(rust.keys() - sdk.keys()))
    for name in sorted(rust.keys() & sdk.keys()):
        if sdk[name] != rust[name]:
            errors.append(f"{name}: SDK {sdk[name]}, Rust {rust[name]}")
    if errors:
        parser.exit(1, "\n".join(errors) + "\n")
    print(f"{args.sdk_abi}: all {len(sdk)} public inline wrappers match by name, selector, arity and return type")
    if psl1ght is not None:
        by_number = {binding.number: (name, binding) for name, binding in rust.items()}
        shared = by_number.keys() & psl1ght.keys()
        print(f"PSL1GHT: {len(shared)} shared inline selectors, "
              f"{len(by_number.keys() - psl1ght.keys())} only in the official profile, "
              f"{len(psl1ght.keys() - by_number.keys())} only in PSL1GHT")
        for number in sorted(shared):
            name, binding = by_number[number]
            if psl1ght[number] != {binding.arity}:
                print(f"  {number} ({name}): official arity {binding.arity}, "
                      f"PSL1GHT arities {sorted(psl1ght[number])}")


if __name__ == "__main__":
    main()
