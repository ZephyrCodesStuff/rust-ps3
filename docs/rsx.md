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
  CGB vertex and fragment configurations and vertex default constants. Cg
  vertex parameters can be updated directly from their reflected records. Both
  readers can update embedded fragment constants in a writable microcode copy
  before upload. They can also queue constant writes to an uploaded program
  and refresh its address before drawing.
- `gtf::File` reads the texture descriptors and mip chains in Cell SDK
  GTF assets. Copy an image's bytes with `Buffer::write` and obtain the
  relocated texture descriptor with `Image::descriptor_at`.
- `rsx::raw` exposes direct GCM firmware calls. `rsx::methods` names
  the hardware methods from the SDK, and `Context::write_method` emits
  incrementing and non-incrementing command packets. `Context` and
  `CommandRecorder` also provide `write_method_with_payload` for filling a
  zero-initialized packet payload in place. The method header and cursor are
  committed after the fill closure succeeds; an error leaves the packet
  uncommitted, and insufficient FIFO or recorder capacity returns
  `CommandBufferFull` without running the closure. The aligned inline draw,
  vertex constant, and inline transfer variants provide 16-byte-aligned
  payload slices. Alignment uses the FIFO address returned after reservation
  and any refill.

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
binding. It also checked descriptor construction for RGBA32F. A separate
packet smoke checked successful in-place payload fills, that closure errors
leave packets uncommitted for both `Context` and `CommandRecorder`, and that
insufficient capacity returns `CommandBufferFull` without invoking the fill
closure. Standalone sampler filter and address/remap setters and the NV3062
transfer destination selector also matched their expected FIFO packets. The
texture optimization setter now rejects slopes above 31, matching the SDK's
five-bit field.

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
`0xff0000` and `0x00ff00`. A corrected same-target diagnostic also passed in
Legacy Recompiler mode: all 64 active green RGBA32F lanes matched immediately
after DMA (digest `0x9fde6ac5`); the first draw read red `0xff0000`; a clear to
sentinel RGB `0xa53c91` was read back before rebinding; then the green rebind
read center RGB `0x00ff00` while the untouched corner remained `0xa53c91`.
This verifies RGBA32F same-target rebinding in that mode. A separate R32F
same-target high-to-low run first drew the high sample and read center RGB
`0x336619`, then cleared and read sentinel RGB `0xa53c91`; after rebinding the
low sample, the center retained `0xa53c91` while the triangle region at y=78%
read `0x336619`. This confirms the low sample moved the triangle away from
the center on the same target. The diagnostic does not isolate which
individual change resolved the earlier retained-red result. The async shader
interpreter reported that it does not support vertex textures. The R32F test
also passed when the shader matrix and animation parameters were updated by
name using `set_cg_vertex_parameter`. The reflected FIFO check covered the
original matrix, its sparse and noncontiguous variants, and unallocated rows.

A PPU packet check in RPCS3 verified the independent Cell GCM blend factor,
equation, and constant setters, as well as independent stencil and culling
state. A separate FIFO check verified direct vertex constant uploads, including
eight-register packet splitting and rejection of out-of-range register writes.
The scalar component setter emits only the supplied lanes of the final
register. The FIFO check covered partial writes and the 32-word boundary.
The R32F example also passed after a one-component write to its animation
register, preserving both components that control height displacement.
The typed `set_vertex_constants_with` fill was checked against its FIFO packet,
and its uploaded values were exercised by rendering. FIFO checks also covered
the three inline draw fill variants; rendering verified inline `u32` vertex
words, `u32` indices, and packed `u16` index words. Inline transfer fills
passed three-word and 513-word DMA readbacks with padding guards. When the
second callback returned an error, the first 512 words still transferred
and the destination at the failed block was preserved. Aligned versions
passed vertex, 32-bit index and packed 16-bit index rendering as well as
the R32F draw with aligned vertex constants. The inline transfer check covered
all four starting word offsets. A separate aligned transfer reservation
triggered a live FIFO refill and verified the resulting payload address and
DMA destination words.

The `verify-framebuffer` example tested four face-culling settings on the
same back buffer. Changing the culled face or front-face winding switched
the center between the textured triangle and the clear color. All three
SDK profiles passed in RPCS3.

The vertex branch condition word and the per-slot vertex frequency modulo
mask also matched their Cell GCM FIFO packets.

The tracked `verify-framebuffer` example now tests stencil in four separate
scissored strips after a seed pass. On all three SDK profiles, `Never` and
`Equal` with reference `0x5b` against stencil value `0x5a` retained the clear
color (`0x3578c8`); `Always` and matching `Equal` produced the sampled texel
(`0x00dc00`). These results supersede the earlier inconclusive multi-draw
probes without establishing their root cause.

The fragment control FIFO check confirmed the TXP toggle, minimum register
count, and rejection of counts above 48.

An SDK packet review confirmed the RGB565 transfer command words. In RPCS3's
forced CPU-blit path, big-endian red, green, blue, and white source words read
back as `0xff001cc6`, `0xffe70039`, `0xff18e300`, and `0xffffffff`. Feeding the
byte-swapped words through scratch storage produced the expected colors. The
current Vulkan-default probe produced no readback. This records an observed
backend byte-order discrepancy; Rust code was not changed.

The SDK Cg datatype enum check corrected the numeric IDs used by fragment
parameter DMA. All 18 supported float and boolean type cases passed. The
original Cg fragment `texSize` update copied two floats and preserved the
third microcode word at each embedded copy.

The SDK 2.70 and 3.70 profiles passed the default FIFO reset and resize
check on CEX 4.93 in RPCS3. The test advanced the command read pointer,
reset it to the SDK initialization offset `0x1000`, and reduced the segment
from 8,192 to 4,096 words within the 16,384-word command buffer. Submitting
through the next segment advanced the FIFO boundary and completed a new
reference command.

The standalone depth clipping bounds setter matched the SDK FIFO packet,
including the exact float bit patterns. In the local render probe the
narrower bounds `[0.75, 1.0]` left the test triangle visible. The render
semantics remain unverified.
