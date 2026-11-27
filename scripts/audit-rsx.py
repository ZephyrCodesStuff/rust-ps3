#!/usr/bin/env python3
"""Compare the Rust graphics ABI with a local PPU SDK installation."""

import argparse
from pathlib import Path
import re
import runpy
import tomllib


ROOT = Path(__file__).resolve().parents[1]
RSX = ROOT / "ps3/src/rsx"
IMPORTS = re.compile(
    r'#\[link_name = "(\w+)"\]\s+pub(?:\(crate\))? fn \w+'
    r'\(([^()]*)\)(?:\s*->\s*([^;]+))?;'
)
METHODS = re.compile(
    r"^#define\s+CELL_GCM_NV([0-9A-F]+)_(\w+)\s+\(0x([0-9a-fA-F]+)\)", re.M
)
PROTOTYPES = re.compile(
    r"^([A-Za-z_]\w*(?:[ \t]+\w+)?[ \t*]+?)(cellGcm\w+)\s*"
    r"\(((?:[^();]|\([^()]*\))*)\);",
    re.M,
)
# These are required by the SDK's static init and presentation wrappers.
WRAPPER_IMPORTS = {
    "_cellGcmInitBody",
    "_cellGcmSetFlipCommand",
    "_cellGcmSetFlipCommandWithWaitLabel",
    "cellGcmSetPrepareFlip",
}


def sdk_type(declaration):
    base = declaration.replace("const", "").replace("*", "").strip()
    depth = declaration.count("*")
    if base == "void":
        return {0: "()", 1: "*c_void", 2: "*u32"}[depth]
    aliases = {
        "uint8_t": "u8",
        "uint16_t": "u16",
        "uint32_t": "u32",
        "uint64_t": "u64",
        "int32_t": "i32",
        "system_time_t": "i64",
    }
    base = base.removeprefix("CellGcm") if base.startswith("CellGcm") else aliases[base]
    return "*" * depth + base


def sdk_argument(argument):
    if "(*handler)" in argument:
        return "*PpuFunctionDescriptor"
    declaration, count = re.subn(r"\b\w+$", "", argument.strip())
    if count != 1:
        raise ValueError(f"unrecognized GCM argument {argument}")
    return sdk_type(declaration)


def check_imports(sdk, tool_dir):
    common = runpy.run_path(str(ROOT / "scripts/audit-lv2.py"))
    manifest = tomllib.loads((ROOT / "ps3/sdk-imports.toml").read_text())["libraries"]
    imports = {row["name"]: row for row in manifest["cellGcmSys"]["functions"]}
    sdk_ids = common["sdk_stub_ids"](sdk / "lib/libgcm_sys_stub.a", tool_dir)
    rust = {
        name: common["rust_signature"](ret, args)
        for name, args, ret in IMPORTS.findall((RSX / "raw.rs").read_text())
    }
    sdk_header = sdk / "include/cell/gcm.h"
    signatures = {}
    for result, name, args in PROTOTYPES.findall(common["header_source"](sdk_header)):
        if name in sdk_ids:
            signatures[name] = (
                sdk_type(result), tuple(map(sdk_argument, common["parameters"](args)))
            )

    expected = signatures.keys() | WRAPPER_IMPORTS
    if expected != imports.keys() or expected != rust.keys():
        raise ValueError(
            f"GCM coverage differs: missing imports {sorted(expected - imports.keys())}, "
            f"extra imports {sorted(imports.keys() - expected)}, "
            f"declarations without matching import {sorted(rust.keys() ^ imports.keys())}"
        )
    for name, row in imports.items():
        if sdk_ids.get(name) != row["fnid"]:
            raise ValueError(f"{name}: SDK FNID {sdk_ids.get(name)}, Rust FNID {row['fnid']}")
        if name in signatures and signatures[name] != rust[name]:
            raise ValueError(f"{name}: SDK signature {signatures[name]}, Rust signature {rust[name]}")
        words = max(0, len(rust[name][1]) - 8)
        if row.get("stack_words", 0) != words:
            raise ValueError(f"{name}: must forward {words} stack arguments")
    print(f"{len(signatures)} GCM signatures and {len(imports)} GCM FNIDs match")

    sdk_video = common["sdk_stub_ids"](sdk / "lib/libsysutil_stub.a", tool_dir)
    video = manifest["cellSysutil"]["functions"]
    for row in video:
        if sdk_video.get(row["name"]) != row["fnid"]:
            raise ValueError(f"SDK video-out FNID differs for {row['name']}")
    print(f"{len(video)} video-out FNIDs match")


def check_methods(include):
    header = include / "cell/gcm/gcm_macros.h"
    sdk = {(obj.lower(), name): int(value, 16) for obj, name, value in METHODS.findall(header.read_text())}
    source = (RSX / "methods.rs").read_text()
    rust = {
        (obj, name): int(value, 16)
        for obj, body in re.findall(r"^pub mod nv([0-9a-f]+) \{(.*?)^\}", source, re.M | re.S)
        for name, value in re.findall(r"pub const (\w+): Method = method\(0x([0-9a-f]+)\);", body)
    }
    if not sdk or rust != sdk:
        differences = sorted(name for name in sdk.keys() | rust.keys() if sdk.get(name) != rust.get(name))
        raise ValueError(f"RSX method catalogue differs: {differences}")
    print(f"{len(sdk)} RSX methods match")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("sdk_ppu", type=Path, help="SDK target/ppu directory")
    parser.add_argument("--ppu-bin", type=Path, help="directory containing the GNU PPU binary tools")
    args = parser.parse_args()
    try:
        check_imports(args.sdk_ppu, args.ppu_bin)
        check_methods(args.sdk_ppu / "include")
    except (OSError, KeyError, ValueError) as error:
        parser.exit(1, f"{error}\n")


if __name__ == "__main__":
    main()
