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
- `TextureFormat`, `TextureLayout`, and `TextureRemap` build texture
  descriptor words. Fragment bindings can select a border source,
  depth comparison, signed-channel interpretation, alpha kill, and
  anisotropic bias. Vertex texture bindings validate the two supported
  linear, normalized floating-point formats and their alignment.
- Vertex formats and offsets can be updated independently. The API also
  exposes shared vertex array bases, all eight immediate attribute
  formats, and inline 16-bit and 32-bit index draws.
- DMA commands support pitched copies, inline word transfers, scaling,
  and swizzled destinations.
  `Context::copy_image` copies a rectangle into a pitched surface or a
  power-of-two Morton-order texture. It supports 16- and 32-bit linear
  pixels and 16-, 32-, 64-, and 128-bit swizzled pixels.
  Labels, FIFO references, GCM events, and reports expose synchronization
  and feedback from the hardware.
- `ReportSlot` selects the local or main-memory report table. Use
  `map_main_reports` before submitting reports to main memory, and
  `map_notifications` before issuing notification commands. The API
  supports conditional draws, report DMA readback, and Z-cull statistics
  and stencil-cull settings.
- `shader::cg` reads big-endian Cell SDK Cg revision 6. `shader::cgb` reads
  CGB vertex and fragment configurations and vertex default constants. Both
  readers can update embedded fragment constants in a writable microcode copy
  before upload. They can also queue constant writes to an uploaded program
  and refresh its address before drawing.
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

The example exercises a native RSX triangle draw, framebuffer setup, clear
and flip, main-memory mapping, row copies, scaling, swizzled copies, a firmware
FIFO refill, a user callback, tile and Z-cull setup, and alternate display
buffers. The example samples a 4x4 ARGB texture uploaded through the transfer
path, then draws the original color triangle on the other display buffer. The
texture program and its
matching position/TEXCOORD0 vertex program use the official SDK 4.75
ImageProcessing/demosaic standalone binaries. Shader microcode words are
stored in big-endian order and remain in local memory for the lifetime of the
draw. With `verify-framebuffer` and **Write Color Buffers** enabled, it checks
the clear color outside the textured triangle, the sampled texel inside it,
the original color shader's output, and transfer results.

The standalone SDK occlusion smoke follows the SDK 4.75
`report_to_main_memory` query sequence with a depth-tested Z24S8 surface:
enable Z-pass pixel counting, leave Z-cull disabled, clear depth and the
counter, draw query geometry, then issue the report. Its conditional draw
checks use separate local report slots for positive and zero results. In
RPCS3, the positive report produced 158,976 pixels and enabled a visible draw;
a zero report suppressed its draw and left the framebuffer center at the clear
color. Reusing one report slot for both conditional checks was not reliable in
that emulator. This is an RPCS3 result, not a console validation claim.

Cg and CGB shader parsing supports the official SDK formats. GTF decoding
was checked with single-texture and multi-texture SDK samples. The triangle
demonstrates native shader upload and draw submission; broader shader and
rendering combinations still need runtime coverage. Transfer coverage
includes pitched subrectangle copies, a swizzled copy crossing a 1024-pixel
boundary, 128-bit pixels, and narrow mip levels. Physical console tests and one-for-one typed
wrappers for the remaining GCM command helpers are still open.
Local and main-memory timestamps, local-report DMA readback, main-memory
notifications, and the SDK-like conditional occlusion smoke were checked in
RPCS3. On one unchanged render target, the sampler smoke produced the expected
red/blue-remapped center value `0x563412`, then rebound the same texture
allocation and slot with identity remapping and produced `0x123456`. This
confirms the remap state updates on rebind in RPCS3. A separate local Vulkan
run tested the same outside UV (`S = 1.5`) with `ClampToEdge` and
`ClampToBorder`: the edge control returned `0x123456`, and opaque white border
color returned `0xffffff`. A custom `0xffc02200` border returned black. The
local Vulkan device did not advertise `VK_EXT_custom_border_color`; the run did
not verify that the missing extension caused the black result. The isolated
alpha-kill smoke passed in Legacy Recompiler mode: with a transparent texel
(A=0, RGB=`0x123456`), alpha kill off returned `0x123456`, while alpha kill on
left the clear RGB `0x3578c8`. This confirms alpha-kill behavior in that mode.

The PPU packet check in RPCS3 confirmed the Cell GCM command words for the
no-paranoid fetch mask, alpha-kill control, and linear R32F vertex texture
binding. It also checked descriptor construction for RGBA32F.

With `verify-framebuffer`, the `rsx-triangle` example reproduces all nine
NV0039 input/output stride pairs. Each case copies three rows with distinct
source and destination pitches and checks that skipped bytes and row padding
retain their sentinel value; it also checks that a 61-byte output span that
would overflow the u32 address range is rejected. RPCS3 readback and public
PS3 hardware results confirm that the line length counts copied bytes and each
format selects a byte step. [NV0039 hardware results](https://github.com/RPCS3/rpcs3/pull/19190)

A vertex texture smoke used the original SDK 4.75 `fw_vertex_texture` Cg
shader pair compiled for RSX. In separate RPCS3 Legacy Recompiler runs, an
RGBA32F vertex texture produced the expected framebuffer center values
`0xff0000` and `0x00ff00`. A same-target rebind retained the old red pixel.
The async shader interpreter reported that it does not support vertex
textures. Separate R32F runs used the sampled height to move the triangle
apex. A height of +0.65 produced center RGB `0x336619`. A height of -0.65
left the center at its pre-draw value. Same-target rebinding still needs
verification.

A PPU packet check in RPCS3 verified the independent Cell GCM blend factor,
equation, and constant setters, as well as independent stencil and culling
state. A separate FIFO check verified direct vertex constant uploads, including
eight-register packet splitting and rejection of out-of-range register writes.
