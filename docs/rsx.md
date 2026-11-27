# RSX graphics support

Build with `ps3/rsx` to use `ps3::rsx`. GCM initializes one runtime
context per application. The application supplies an aligned
`IoBuffer` that stays mapped while the GPU uses it. The resulting
`Context` owns command submission. Finish the executable with
`moldier patch` to install its firmware import table and compact
function descriptors.

- `LocalArena` allocates disjoint VRAM ranges. `Context::map_main`
  maps additional main-memory buffers into the RSX I/O address space.
- `Swapchain` allocates and registers two framebuffers, submits the
  active back buffer, and releases the other buffer after scanout.
- Draw calls support vertex and index buffers, inline data, programmable
  shaders, render targets, depth and stencil testing, blending, and
  vertex and fragment textures.
- DMA commands support pitched copies, scaling, and swizzled destinations.
  Labels, FIFO references, GCM events, and reports expose synchronization
  and feedback from the hardware.
- `shader` reads PSL1GHT output and big-endian Cell SDK Cg revision 6.
  `shader::cgb` reads CGB vertex and fragment configurations and vertex
  default constants. The PSL1GHT and Cg readers can update embedded
  fragment constants in a writable copy of the microcode before upload.
- `gtf::File` reads the texture descriptors and mip chains in Cell SDK
  GTF assets. Copy an image's bytes with `Buffer::write` and obtain the
  relocated texture descriptor with `Image::descriptor_at`.
- `rsx::raw` exposes direct GCM firmware calls. `rsx::methods` names
  the hardware methods from the SDK, and `Context::write_method` emits
  incrementing and non-incrementing command packets.

Several operations are unsafe because the GPU accesses mapped memory
asynchronously. Keep backing storage live while it is mapped or registered.
Wait for the appropriate fence before writing or reusing GPU resources.

## Verification

Run the ABI audit with a local SDK 4.75 PPU installation and GNU PPU binary
tools.

```sh
python3 scripts/audit-rsx.py /path/to/target/ppu --ppu-bin "$PS3DEV/ppu/bin"
```

The audit checks 70 public GCM signatures, 74 GCM function IDs, four
video output function IDs, and 277 methods against the original SDK.
The 15 GCM records have matching PPU C and Rust layout checks.

The Cargo SDK profiles also select the GCM imports. SDK 1.92 uses the
`cellGcmSetFlip` import, and SDK 2.70 and later use
`_cellGcmSetFlipCommand`. The import signatures and function IDs were
checked against SDKs 1.92, 2.70, 3.70, 4.60, and 4.75. To audit only an
older SDK's imports, add `--imports-only` with its `--sdk-abi` value.

| Stock firmware | `sdk-192` GCM exports | `sdk-270` GCM exports | `sdk-370` GCM exports |
| --- | ---: | ---: | ---: |
| DECR 1.10 | 35 / 49 | 34 / 60 | 34 / 74 |
| DECR 2.60 | 49 / 49 | 59 / 60 | 59 / 74 |
| DECR 4.81 | 49 / 49 | 60 / 60 | 74 / 74 |
| DEX 4.81 | 49 / 49 | 60 / 60 | 74 / 74 |
| CEX 4.93 | 49 / 49 | 60 / 60 | 74 / 74 |

The `rsx-triangle` example passed in RPCS3 at 1280 × 720 under all three
Cargo profiles. With
`verify-framebuffer` and **Write Color Buffers** enabled, it checks
the clear color, two independent draw paths, GPU copies and scaling,
swizzle output, a firmware FIFO refill, a user callback, and alternate
display buffers. It also checks tile and Z-cull setup, including the
stencil values passed on the ELFv1 stack.

The Cg reader was tested with vertex and fragment binaries shipped
with SDK 4.75, including a fragment parameter with five embedded
locations. GTF decoding was checked with single-texture and
multi-texture SDK samples. The Cg and CGB vertex upload paths were
exercised in RPCS3. Physical console tests and one-for-one typed
wrappers for the remaining GCM command helpers are still open.
