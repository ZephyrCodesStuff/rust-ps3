#![no_std]
#![no_main]

use core::ptr::addr_of_mut;
#[cfg(feature = "verify-framebuffer")]
use core::sync::atomic::{AtomicU32, Ordering};

use ps3::println;
use ps3::rsx::{
    video, Buffer, ClearMask, Compare, Context, Error, FlipMode, IoBuffer, LabelStage, Location,
    MagFilter, Method, MinFilter, Primitive, Rect, Sampler, ScaleTarget, Swapchain, Texture,
    TransferMemory, TransferScale, TransferSurface, VertexAttribute, VertexType, Viewport,
    WrapMode,
};
#[cfg(feature = "verify-framebuffer")]
use ps3::rsx::{Event, EventCallback, ImageCopyTarget, ImagePixelSize, TransferDataFormat};
use ps3::sys::sys_timer_usleep;

const MIB: usize = 1024 * 1024;
const IO_SIZE: usize = MIB;
const COLOR: u32 = 0xff35_78c8;
const TEXEL: u32 = 0xff00_dc00;
const TEX_PITCH: u32 = 64;
const VERTEX_SHADER: [u32; 20] = [
    0x401f_9c6c,
    0x01d0_300d,
    0x8106_c0c3,
    0x6040_3f80,
    0x401f_9c6c,
    0x01d0_200d,
    0x8106_c0c3,
    0x6040_5f80,
    0x401f_9c6c,
    0x01d0_100d,
    0x8106_c0c3,
    0x6040_9f80,
    0x401f_9c6c,
    0x01d0_000d,
    0x8106_c0c3,
    0x6041_1f80,
    0x401f_9c6c,
    0x0040_030d,
    0x8106_c083,
    0x6041_ff85,
];
// Exact one-instruction pair from the SDK 4.75 ImageProcessing/demosaic
// standalone shaders. The VP passes input slots 0 and 8 to position and
// TEXCOORD0; the FP samples texture0 at TEXCOORD0 and writes color0.
const TEXTURE_VERTEX_SHADER: [u32; 8] = [
    0x401f_9c6c,
    0x0040_000d,
    0x8106_c083,
    0x6041_ff80,
    0x401f_9c6c,
    0x0040_0808,
    0x0106_c083,
    0x6041_9f9d,
];
const TEXTURE_FRAGMENT_SHADER: [u32; 4] = [0x9e01_1700, 0xc801_1c9d, 0xc800_0001, 0xc800_3fe1];
const COLOR_FRAGMENT_SHADER: [u32; 4] = [0x3e01_0100, 0xc801_1c9d, 0xc800_0001, 0xc800_3fe1];
const TRIANGLE: [f32; 21] = [
    0.0, 0.65, 0.0, 1.0, 0.5, 0.5, 0.0, -0.6, -0.5, 0.0, 1.0, 0.0, 1.0, 0.0, 0.6, -0.5, 0.0, 1.0,
    1.0, 1.0, 0.0,
];
const COLOR_TRIANGLE: [f32; 21] = [
    0.0, 0.65, 0.0, 1.0, 1.0, 0.1, 0.1, -0.6, -0.5, 0.0, 1.0, 0.1, 1.0, 0.1, 0.6, -0.5, 0.0, 1.0,
    0.1, 0.2, 1.0,
];
static mut IO: IoBuffer<IO_SIZE> = IoBuffer([0; IO_SIZE]);
static mut TEXTURE_UPLOAD: IoBuffer<IO_SIZE> = IoBuffer([0; IO_SIZE]);
#[cfg(feature = "verify-framebuffer")]
static USER_EVENT: AtomicU32 = AtomicU32::new(0);

#[cfg(feature = "verify-framebuffer")]
extern "C" fn on_user_event(cause: u32) {
    USER_EVENT.store(cause, Ordering::Release);
}

/// Round-trips NV0039 byte selections and verifies skipped bytes and row padding.
#[cfg(feature = "verify-framebuffer")]
fn verify_formatted_copies(
    context: &mut Context,
    vram: &mut ps3::rsx::LocalArena,
    upload: &mut Buffer,
) -> Result<(), Error> {
    const COLUMNS: u32 = 16;
    const ROWS: u32 = 3;
    const SENTINEL: u8 = 0xa5;
    const SOURCE_PAD: u8 = 0xe7;
    let formats = [
        TransferDataFormat::Bytes1,
        TransferDataFormat::Bytes2,
        TransferDataFormat::Bytes4,
    ];
    let pitches = [24u32, 36, 68];
    let output_pitches = [32u32, 40, 72];
    let mut source = vram.allocate(68 * ROWS, 128)?;

    for (input_index, input_format) in formats.iter().copied().enumerate() {
        for (output_index, output_format) in formats.iter().copied().enumerate() {
            let case_index = input_index * formats.len() + output_index;
            let source_pitch = pitches[input_index];
            let destination_pitch = output_pitches[output_index];
            let destination_offset = upload.offset() + 0x1000 + case_index as u32 * 0x1000;
            let destination_bytes = destination_pitch * ROWS;
            let input_stride = input_format as u8 as u32;
            let output_stride = output_format as u8 as u32;

            unsafe {
                let sentinels = [SENTINEL; 216];
                upload.write(
                    destination_offset - upload.offset(),
                    &sentinels[..destination_bytes as usize],
                )?;
                for row in 0..ROWS {
                    let mut source_row = [SOURCE_PAD; 68];
                    for column in 0..COLUMNS {
                        source_row[(column * input_stride) as usize] =
                            m2mf_sample(case_index as u8, row as u8, column as u8);
                    }
                    source.write(row * source_pitch, &source_row[..source_pitch as usize])?;
                }
                context.copy_rows_formatted(
                    TransferMemory {
                        location: source.location(),
                        offset: source.offset(),
                        pitch: source_pitch as i32,
                    },
                    TransferMemory {
                        location: upload.location(),
                        offset: destination_offset,
                        pitch: destination_pitch as i32,
                    },
                    COLUMNS,
                    ROWS,
                    input_format,
                    output_format,
                )?;
                context.finish(0x1234_5680 + case_index as u32, 30_000)?;

                for row in 0..ROWS {
                    let row_start = destination_offset - upload.offset() + row * destination_pitch;
                    let mut offset = 0;
                    while offset < destination_pitch {
                        let mut expected = [SENTINEL; 4];
                        for byte in 0..4u32 {
                            let column = offset + byte;
                            if column % output_stride == 0 {
                                let selected = column / output_stride;
                                if selected < COLUMNS {
                                    expected[byte as usize] =
                                        m2mf_sample(case_index as u8, row as u8, selected as u8);
                                }
                            }
                        }
                        assert_eq!(
                            upload.read_word(row_start + offset)?,
                            u32::from_be_bytes(expected),
                            "input {:?}, output {:?}, row {row}, byte {offset}",
                            input_format,
                            output_format,
                        );
                        offset += 4;
                    }
                }
            }
        }
    }

    // The 16-byte input span fits at this offset, while the 61-byte output
    // span for Bytes4 would overflow the u32 address range.
    let invalid_destination = TransferMemory {
        location: upload.location(),
        offset: u32::MAX - 15,
        pitch: 0,
    };
    let invalid = unsafe {
        context.copy_rows_formatted(
            TransferMemory {
                location: source.location(),
                offset: source.offset(),
                pitch: 0,
            },
            invalid_destination,
            16,
            1,
            TransferDataFormat::Bytes1,
            TransferDataFormat::Bytes4,
        )
    };
    assert!(matches!(invalid, Err(Error::InvalidArgument)));
    Ok(())
}

#[cfg(feature = "verify-framebuffer")]
fn m2mf_sample(case: u8, row: u8, column: u8) -> u8 {
    case.wrapping_mul(53) ^ row.wrapping_mul(37) ^ column.wrapping_mul(11)
}

#[ps3::main]
fn main() {
    if let Err(error) = run() {
        panic!("RSX example failed: {:?}", error);
    }
}

fn run() -> Result<(), Error> {
    let mut context = unsafe { Context::init(64 * 1024, &mut (*addr_of_mut!(IO)).0)? };
    #[cfg(feature = "verify-framebuffer")]
    unsafe {
        // Fill the first command segment and trigger the firmware refill.
        let fifo = context.as_raw();
        let old_end = (*fifo).end;
        let remaining_words = (old_end - (*fifo).current) / 4;
        context.nop(remaining_words as usize + 32)?;
        assert_ne!((*fifo).end, old_end);
        context.finish(0x1234_5675, 30_000)?;

        let callback = EventCallback::new(on_user_event)?;
        context.set_event_handler(Event::User, Some(callback));
        context.user_event(0x0001_4321)?;
        context.finish(0x1234_5673, 30_000)?;
        for _ in 0..10_000 {
            if USER_EVENT.load(Ordering::Acquire) != 0 {
                break;
            }
            sys_timer_usleep(100);
        }
        context.set_event_handler(Event::User, None);
        assert_eq!(USER_EVENT.load(Ordering::Acquire), 0x0001_4321);
    }
    let config = context.configuration();
    let output = video::Output::Primary;
    let mode = output.state(0)?.display_mode;
    // This app owns the entire local-memory range supplied by GCM.
    let mut vram = unsafe { context.local_arena(0..config.local_size)? };
    #[cfg(feature = "verify-framebuffer")]
    verify_tiling(&mut context, &mut vram)?;
    let mut display = unsafe { Swapchain::new(&mut context, &mut vram, output, mode, [0, 1])? };
    let back = display.back_buffer().ok_or(Error::InvalidContext)?;
    let size = back.resolution();
    let depth = vram.allocate(back.storage().size(), 64)?;
    let surface = back.surface(&depth)?;
    let first_offset = back.storage().offset();
    let texels = vram.allocate(TEX_PITCH * 4, 128)?;
    let scaled = vram.allocate(TEX_PITCH * 8, 128)?;
    let mut vertices = vram.allocate(core::mem::size_of_val(&TRIANGLE) as u32, 16)?;
    let mut fragment_code =
        vram.allocate(core::mem::size_of_val(&TEXTURE_FRAGMENT_SHADER) as u32, 64)?;
    #[cfg(feature = "verify-framebuffer")]
    let mut swizzled = vram.allocate(8 * 8 * 4, 128)?;
    if context.address_to_offset(back.storage().address())? != first_offset {
        return Err(Error::InvalidContext);
    }
    let mut upload =
        unsafe { context.map_main(addr_of_mut!(TEXTURE_UPLOAD).cast(), IO_SIZE as u32)? };
    #[cfg(feature = "verify-framebuffer")]
    verify_formatted_copies(&mut context, &mut vram, &mut upload)?;
    #[cfg(feature = "verify-framebuffer")]
    unsafe {
        let words = core::array::from_fn::<_, 513, _>(|i| 0x1020_3040 + i as u32);
        let guard = 0xface_cafeu32;
        let tail = 4 + size_of_val(&words) as u32;
        upload.write(0, &guard.to_be_bytes())?;
        upload.write(tail, &guard.to_be_bytes())?;
        context.inline_transfer(upload.location(), upload.offset() + 4, &words)?;
        context.finish(0x1234_5672, 30_000)?;
        assert_eq!(upload.read_word(0)?, guard);
        for (i, word) in words.into_iter().enumerate() {
            assert_eq!(upload.read_word(4 + i as u32 * 4)?, word);
        }
        assert_eq!(upload.read_word(tail)?, guard);
    }
    let mut draw_done = unsafe { context.label(128)? };

    context.set_flip_mode(FlipMode::VSync);
    unsafe {
        for row in 0..4 {
            upload.write(row * TEX_PITCH, [TEXEL.to_be_bytes(); 4].as_flattened())?;
        }
        let region = |buffer: &Buffer| TransferMemory {
            location: buffer.location(),
            offset: buffer.offset(),
            pitch: TEX_PITCH as i32,
        };
        context.copy_rows(region(&upload), region(&texels), 16, 4)?;
        context.finish(0x1234_5677, 30_000)?;
        upload.unmap_main()?;
        let scaler = TransferScale::argb8(
            region(&texels),
            [4, 4],
            Rect {
                x: 0,
                y: 0,
                width: 8,
                height: 8,
            },
            false,
        )?;
        context.scale_image(
            texels.location(),
            scaled.location(),
            &scaler,
            ScaleTarget::Linear(TransferSurface::argb8(scaled.offset(), TEX_PITCH as u16)),
        )?;
        context.finish(0x1234_5676, 30_000)?;
        #[cfg(feature = "verify-framebuffer")]
        {
            context.scale_image(
                texels.location(),
                swizzled.location(),
                &scaler,
                ScaleTarget::Swizzled(ps3::rsx::TransferSwizzle {
                    format: 0x0a,
                    width: 3,
                    height: 3,
                    offset: swizzled.offset(),
                }),
            )?;
            context.finish(0x1234_5674, 30_000)?;
            assert_eq!(swizzled.read_word(0)?, TEXEL);
            assert_eq!(swizzled.read_word(swizzled.size() - 4)?, TEXEL);
            // Neighboring texels (1, 1) and (2, 1) occupy Morton indices 3 and 6.
            swizzled.write(3 * 4, &COLOR.to_be_bytes())?;
            swizzled.write(6 * 4, &COLOR.to_be_bytes())?;
            context.copy_image(
                region(&texels),
                Rect {
                    x: 1,
                    y: 1,
                    width: 1,
                    height: 1,
                },
                ImageCopyTarget::Swizzled {
                    location: swizzled.location(),
                    offset: swizzled.offset(),
                    extent: [8, 8],
                },
                [1, 1],
                ImagePixelSize::Bits32,
            )?;
            context.finish(0x1234_5671, 30_000)?;
            assert_eq!(swizzled.read_word(3 * 4)?, TEXEL);
            assert_eq!(swizzled.read_word(6 * 4)?, COLOR);
        }

        vertices.write(0, bytes_of(&TRIANGLE))?;
        fragment_code.write(0, bytes_of(&TEXTURE_FRAGMENT_SHADER))?;
        let vertex_config = ps3::rsx::shader::cgb::VertexConfiguration {
            instruction_slot: 0,
            instruction_count: 2,
            // The paired SDK VP consumes position at 0 and TEXCOORD0 at 8.
            attribute_input_mask: 0x0101,
            registers: 1,
            reserved: 0,
        };
        context.load_cgb_vertex_configuration(&vertex_config, bytes_of(&TEXTURE_VERTEX_SHADER))?;
        // The SDK VP's declared output mask maps its TEXCOORD0 varying.
        context.write_method(Method::new(0x1ff4).unwrap(), true, &[0x4000])?;
        let fragment_config = ps3::rsx::shader::cgb::FragmentConfiguration {
            offset: fragment_code.offset(),
            attribute_input_mask: 0x4000,
            texcoords: 0x01,
            texcoords_2d: 0x01,
            // Cg fragment flags at config bytes 19..22 map to 0x440.
            control: 0x440,
            registers: 2,
            ..Default::default()
        };
        context.load_cgb_fragment_program(&fragment_config, Location::Local)?;
        let sampler = Sampler {
            wrap: [WrapMode::ClampToEdge; 3],
            min_filter: MinFilter::Nearest,
            mag_filter: MagFilter::Nearest,
            ..Sampler::default()
        };
        let texture = Texture::linear_argb8(texels.location(), texels.offset(), 4, 4, TEX_PITCH)?;
        context.bind_texture(0, &texture, &sampler)?;
        context.set_vertex_attribute(VertexAttribute {
            slot: 0,
            components: 4,
            format: VertexType::F32,
            stride: 28,
            frequency: 0,
            location: Location::Local,
            offset: vertices.offset(),
        })?;
        context.set_vertex_attribute(VertexAttribute {
            slot: 8,
            components: 2,
            format: VertexType::F32,
            stride: 28,
            frequency: 0,
            location: Location::Local,
            offset: vertices.offset() + 16,
        })?;
        context.set_surface(&surface)?;
        let viewport = Viewport::for_surface(size.width, size.height);
        context.set_viewport(&viewport)?;
        context.set_scissor(viewport.rect)?;
        context.set_depth_state(false, false)?;
        context.set_depth_compare(Compare::LessEqual)?;
        context.set_color_mask([true; 4])?;
        context.set_clear_color(COLOR)?;
        context.clear(ClearMask::COLOR)?;
        context.draw_arrays(Primitive::Triangles, 0, 3)?;
        draw_done.write(0);
        context.signal_label(&draw_done, LabelStage::Backend, 0x0123_4567)?;
        context.wait_label(&draw_done, 0x0123_4567)?;
        display.present(&mut context)?;
        context.finish(0x1234_5678, 30_000)?;
    }
    draw_done.wait(0x0123_4567, 30_000)?;
    display.wait(&context, 30_000)?;
    #[cfg(feature = "verify-framebuffer")]
    unsafe {
        let front = display.front_buffer().ok_or(Error::InvalidContext)?;
        let pitch = front.pitch();
        let framebuffer = front.storage();
        let pixel = |x, y| {
            framebuffer
                .read_word(pitch * y + x * 4)
                .map(|word| word & 0x00ff_ffff)
        };
        let (width, height) = (u32::from(size.width), u32::from(size.height));
        let corner = pixel(0, 0)?;
        let left = pixel(width / 4, height / 2)?;
        let center = pixel(width / 2, height / 2)?;
        let right = pixel(width * 3 / 4, height / 2)?;
        println!(
            "RSX framebuffer readback: corner={:06x}, left={:06x}, center={:06x}, right={:06x}",
            corner, left, center, right
        );
        assert_eq!(corner, COLOR & 0x00ff_ffff);
        assert_eq!(left, COLOR & 0x00ff_ffff);
        assert_eq!(center, TEXEL & 0x00ff_ffff);
        assert_eq!(right, COLOR & 0x00ff_ffff);
    }

    // Keep the clear visible before confirming that the buffers alternate.
    sys_timer_usleep(500_000);
    let next = display.back_buffer().ok_or(Error::InvalidContext)?;
    let next_offset = next.storage().offset();
    assert_ne!(next_offset, first_offset);
    unsafe {
        vertices.write(0, bytes_of(&COLOR_TRIANGLE))?;
        fragment_code.write(0, bytes_of(&COLOR_FRAGMENT_SHADER))?;
        let vertex_config = ps3::rsx::shader::cgb::VertexConfiguration {
            instruction_slot: 0,
            instruction_count: 5,
            attribute_input_mask: 0x0009,
            registers: 1,
            reserved: 0,
        };
        context.load_cgb_vertex_configuration(&vertex_config, bytes_of(&VERTEX_SHADER))?;
        context.write_method(
            Method::new(0x1efc).unwrap(),
            true,
            &[
                256,
                1.0f32.to_bits(),
                0,
                0,
                0,
                0,
                1.0f32.to_bits(),
                0,
                0,
                0,
                0,
                1.0f32.to_bits(),
                0,
                0,
                0,
                0,
                1.0f32.to_bits(),
            ],
        )?;
        context.write_method(Method::new(0x1ff4).unwrap(), true, &[0x21])?;
        let fragment_config = ps3::rsx::shader::cgb::FragmentConfiguration {
            offset: fragment_code.offset(),
            attribute_input_mask: 0x25,
            texcoords_2d: 0xffff,
            control: 0x440,
            registers: 2,
            ..Default::default()
        };
        context.load_cgb_fragment_program(&fragment_config, Location::Local)?;
        context.set_vertex_attribute(VertexAttribute {
            slot: 0,
            components: 4,
            format: VertexType::F32,
            stride: 28,
            frequency: 0,
            location: Location::Local,
            offset: vertices.offset(),
        })?;
        context.set_vertex_attribute(VertexAttribute {
            slot: 3,
            components: 3,
            format: VertexType::F32,
            stride: 28,
            frequency: 0,
            location: Location::Local,
            offset: vertices.offset() + 16,
        })?;
        context.set_surface(&next.surface(&depth)?)?;
        let viewport = Viewport::for_surface(size.width, size.height);
        context.set_viewport(&viewport)?;
        context.set_scissor(viewport.rect)?;
        context.set_depth_state(false, false)?;
        context.set_depth_compare(Compare::LessEqual)?;
        context.set_color_mask([true; 4])?;
        context.set_clear_color(COLOR)?;
        context.clear(ClearMask::COLOR)?;
        context.draw_arrays(Primitive::Triangles, 0, 3)?;
        display.present(&mut context)?;
    }
    display.wait(&context, 30_000)?;
    assert_eq!(
        display
            .front_buffer()
            .ok_or(Error::InvalidContext)?
            .storage()
            .offset(),
        next_offset,
    );
    #[cfg(feature = "verify-framebuffer")]
    unsafe {
        let front = display.front_buffer().ok_or(Error::InvalidContext)?;
        let center = front.storage().read_word(
            front.pitch() * (u32::from(size.height) / 2) + (u32::from(size.width) / 2) * 4,
        )? & 0x00ff_ffff;
        assert_ne!(center, COLOR & 0x00ff_ffff);
        println!("RSX original color shader readback: center={:06x}", center);
    }
    println!(
        "RSX triangle and flip passed ({}x{})",
        size.width, size.height
    );
    Ok(())
}

fn bytes_of<T>(values: &[T]) -> &[u8] {
    unsafe { core::slice::from_raw_parts(values.as_ptr().cast(), core::mem::size_of_val(values)) }
}

#[cfg(feature = "verify-framebuffer")]
fn verify_tiling(context: &mut Context, arena: &mut ps3::rsx::LocalArena) -> Result<(), Error> {
    use ps3::rsx::{
        raw, Antialias, DepthFormat, Tile, TileCompression, ZCull, ZCullDirection, ZCullFormat,
    };

    let scratch = arena.allocate(0x10000, 0x10000)?;
    unsafe {
        context.bind_tile(
            0,
            &Tile {
                location: scratch.location(),
                offset: scratch.offset(),
                size: scratch.size(),
                pitch: raw::get_tiled_pitch_size(256),
                compression: TileCompression::Disabled,
                tag_base: 0,
                bank: 0,
            },
        )?;
        context.unbind_tile(0)?;

        context.bind_zcull(
            0,
            &ZCull {
                offset: scratch.offset(),
                width: 64,
                height: 64,
                start: 0,
                depth: DepthFormat::Z24S8,
                antialias: Antialias::Single,
                direction: ZCullDirection::Less,
                format: ZCullFormat::LeadingOnes,
                stencil_compare: Compare::NotEqual,
                stencil_reference: 0xab,
                stencil_mask: 0xe7,
            },
        )?;
        #[cfg(feature = "sdk-370")]
        {
            let info = raw::get_zcull_info();
            if info.is_null() {
                return Err(Error::InvalidContext);
            }
            // The compare mode, reference and mask arrive via ELFv1 stack arguments.
            let status = core::ptr::addr_of!((*info).status0).read_volatile();
            assert_eq!(status >> 12, 0xe7ab5);
        }
        context.unbind_zcull(0)?;
    }
    Ok(())
}
