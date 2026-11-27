#![no_std]
#![no_main]

use core::ptr::addr_of_mut;

use ps3::println;
use ps3::rsx::{
    shader::{FragmentProgram, VertexProgram},
    video, Blend, Buffer, ClearMask, Compare, Context, Error, FaceWinding, FlipMode, IndexType,
    IoBuffer, LabelStage, PolygonMode, Primitive, Sampler, Surface, Texture, TransferMemory,
    VertexAttribute, VertexType, Viewport,
};
use ps3::sys::sys_timer_usleep;

const MIB: usize = 1024 * 1024;
const IO_SIZE: usize = MIB;
const COLOR: u32 = 0xff35_78c8;
const TEXEL: u32 = 0xff00_dc00;
const TEX_PITCH: u32 = 64;
const VERTICES: [[f32; 8]; 6] = [
    [-0.5, 0.7, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0],
    [-0.9, -0.6, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0],
    [-0.1, -0.6, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0],
    [0.5, 0.7, 0.0, 1.0, 0.5, 0.0, 0.0, 1.0],
    [0.1, -0.6, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0],
    [0.9, -0.6, 0.0, 1.0, 1.0, 1.0, 0.0, 1.0],
];

static mut IO: IoBuffer<IO_SIZE> = IoBuffer([0; IO_SIZE]);
static mut TEXTURE_UPLOAD: IoBuffer<IO_SIZE> = IoBuffer([0; IO_SIZE]);

#[ps3::main]
fn main() {
    if let Err(error) = run() {
        panic!("RSX example failed: {:?}", error);
    }
}

fn run() -> Result<(), Error> {
    let mut context = unsafe { Context::init(64 * 1024, &mut (*addr_of_mut!(IO)).0)? };
    let config = context.configuration();
    let output = video::Output::Primary;
    let mode = output.state(0)?.display_mode;
    let size = video::resolution(mode.resolution_id)?;
    let pitch = (u32::from(size.width) * 4).next_multiple_of(64);
    let color_bytes = pitch * u32::from(size.height);
    let vertex = VertexProgram::parse(include_bytes!("../shaders/triangle.vpo"))
        .map_err(|_| Error::InvalidArgument)?;
    let fragment = FragmentProgram::parse(include_bytes!("../shaders/triangle.fpo"))
        .map_err(|_| Error::InvalidArgument)?;
    let textured_vertex = VertexProgram::parse(include_bytes!("../shaders/texture.vpo"))
        .map_err(|_| Error::InvalidArgument)?;
    let textured_fragment = FragmentProgram::parse(include_bytes!("../shaders/texture.fpo"))
        .map_err(|_| Error::InvalidArgument)?;
    // This app owns the entire local-memory range supplied by GCM.
    let mut vram = unsafe { context.local_arena(0..config.local_size)? };
    let framebuffer = vram.allocate(color_bytes, 64)?;
    let depth = vram.allocate(color_bytes, 64)?;
    let mut vertices = vram.allocate(size_of_val(&VERTICES) as u32, 128)?;
    let mut fragment_code = vram.allocate(fragment.microcode().len() as u32, 128)?;
    let mut textured_code = vram.allocate(textured_fragment.microcode().len() as u32, 128)?;
    let mut indices = vram.allocate(6, 128)?;
    let texels = vram.allocate(TEX_PITCH * 4, 128)?;
    if context.address_to_offset(framebuffer.address())? != framebuffer.offset() {
        return Err(Error::InvalidContext);
    }
    let mut upload =
        unsafe { context.map_main(addr_of_mut!(TEXTURE_UPLOAD).cast(), IO_SIZE as u32)? };
    let mut draw_done = unsafe { context.label(128)? };

    output.configure(
        mode.resolution_id,
        video::Format::Xrgb,
        mode.aspect,
        pitch,
        false,
    )?;
    context.set_flip_mode(FlipMode::VSync);

    let surface = Surface {
        kind: 1,         // Linear framebuffer
        color_format: 8, // A8R8G8B8
        color_target: 1, // Color target 0
        color_offset: [framebuffer.offset(), 0, 0, 0],
        color_pitch: [pitch, 64, 64, 64],
        depth_format: 2, // Z24S8
        depth_offset: depth.offset(),
        depth_pitch: pitch,
        width: size.width,
        height: size.height,
        ..Default::default()
    };
    unsafe {
        let vertex_bytes =
            core::slice::from_raw_parts(VERTICES.as_ptr().cast(), size_of_val(&VERTICES));
        vertices.write(0, vertex_bytes)?;
        fragment_code.write(0, fragment.microcode())?;
        textured_code.write(0, textured_fragment.microcode())?;
        indices.write(0, &[0, 3, 0, 4, 0, 5])?;
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

        context.set_display_buffer(
            0,
            framebuffer.offset(),
            pitch,
            size.width.into(),
            size.height.into(),
        )?;
        context.set_surface(&surface)?;
        let viewport = Viewport::for_surface(size.width, size.height);
        context.set_viewport(&viewport)?;
        context.set_scissor(viewport.rect)?;
        context.set_depth_state(false, false)?;
        context.set_depth_compare(Compare::LessEqual)?;
        context.set_alpha_test(Some((Compare::Greater, 0)))?;
        context.set_blend(Some(Blend::ALPHA))?;
        context.set_culling(None, FaceWinding::CounterClockwise)?;
        context.set_polygon_mode(PolygonMode::Fill, PolygonMode::Fill)?;
        context.set_color_mask([true; 4])?;
        context.set_clear_color(COLOR)?;
        context.clear(ClearMask::COLOR)?;
        context.load_vertex_program(&vertex)?;
        context.load_fragment_program(
            &fragment,
            fragment_code.location(),
            fragment_code.offset(),
        )?;
        let vertex_offset = vertices.offset();
        for (slot, offset) in [
            (0, vertex_offset),
            (3, vertex_offset + 16),
            (8, vertex_offset + 16),
        ] {
            context.set_vertex_attribute(VertexAttribute {
                slot,
                components: 4,
                format: VertexType::F32,
                stride: 32,
                frequency: 0,
                location: vertices.location(),
                offset,
            })?;
        }
        context.draw_arrays(Primitive::Triangles, 0, 3)?;
        context.load_vertex_program(&textured_vertex)?;
        context.load_fragment_program(
            &textured_fragment,
            textured_code.location(),
            textured_code.offset(),
        )?;
        let texture = Texture::linear_argb8(texels.location(), texels.offset(), 4, 4, TEX_PITCH)?;
        context.bind_texture(0, &texture, &Sampler::default())?;
        context.draw_indexed(
            Primitive::Triangles,
            indices.location(),
            IndexType::U16,
            indices.offset(),
            3,
        )?;
        draw_done.write(0);
        context.signal_label(&draw_done, LabelStage::Backend, 0x0123_4567)?;
        context.wait_label(&draw_done, 0x0123_4567)?;
        context.reset_flip_status();
        context.flip(0)?;
        context.finish(0x1234_5678, 30_000)?;
    }
    draw_done.wait(0x0123_4567, 30_000)?;
    context.wait_for_display_flip(30_000)?;
    #[cfg(feature = "verify-framebuffer")]
    unsafe {
        let pixel = |x, y| {
            framebuffer
                .read_word(pitch * y + x * 4)
                .map(|word| word & 0x00ff_ffff)
        };
        let (width, height) = (u32::from(size.width), u32::from(size.height));
        let corner = pixel(0, 0)?;
        let left = pixel(width / 4, height / 2)?;
        let right = pixel(width * 3 / 4, height / 2)?;
        println!(
            "RSX framebuffer readback: corner={:06x}, left={:06x}, right={:06x}",
            corner, left, right
        );
        assert_eq!(corner, COLOR & 0x00ff_ffff);
        assert_eq!(left, 0xff_0000);
        assert_eq!(right, TEXEL & 0x00ff_ffff);
    }

    // Let the first flip become visible before exiting the example.
    sys_timer_usleep(500_000);
    println!(
        "RSX triangle and flip passed ({}x{})",
        size.width, size.height
    );
    Ok(())
}
