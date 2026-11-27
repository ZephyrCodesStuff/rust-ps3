#![no_std]
#![no_main]

use core::ptr::addr_of_mut;

use ps3::println;
use ps3::rsx::{
    shader::{FragmentProgram, VertexProgram},
    video, ClearMask, Context, Error, FlipMode, IndexType, Location, Primitive, Sampler, Surface,
    Texture, TransferMemory, VertexAttribute, VertexType, Viewport,
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

#[repr(C, align(1048576))]
struct IoBuffer([u8; IO_SIZE]);

static mut IO: IoBuffer = IoBuffer([0; IO_SIZE]);

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
    let depth_offset = color_bytes.next_multiple_of(64);
    let vertex_offset = (depth_offset + color_bytes).next_multiple_of(128);
    let fragment_offset = (vertex_offset + size_of_val(&VERTICES) as u32).next_multiple_of(128);
    let vertex = VertexProgram::parse(include_bytes!("../shaders/triangle.vpo"))
        .map_err(|_| Error::InvalidArgument)?;
    let fragment = FragmentProgram::parse(include_bytes!("../shaders/triangle.fpo"))
        .map_err(|_| Error::InvalidArgument)?;
    let textured_vertex = VertexProgram::parse(include_bytes!("../shaders/texture.vpo"))
        .map_err(|_| Error::InvalidArgument)?;
    let textured_fragment = FragmentProgram::parse(include_bytes!("../shaders/texture.fpo"))
        .map_err(|_| Error::InvalidArgument)?;
    let textured_offset =
        (fragment_offset + fragment.microcode().len() as u32).next_multiple_of(128);
    let index_offset =
        (textured_offset + textured_fragment.microcode().len() as u32).next_multiple_of(128);
    let texture_source = (index_offset + 6).next_multiple_of(128);
    let texture_dest = texture_source + TEX_PITCH * 4;
    if config.local_address == 0
        || texture_dest + TEX_PITCH * 4 > config.local_size
        || context.address_to_offset(config.local_address)? != 0
    {
        return Err(Error::InvalidContext);
    }

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
        color_pitch: [pitch, 64, 64, 64],
        depth_format: 2, // Z24S8
        depth_offset,
        depth_pitch: pitch,
        width: size.width,
        height: size.height,
        ..Default::default()
    };
    unsafe {
        // These local-memory allocations do not overlap either render target.
        let vram = config.local_address as usize as *mut u8;
        vram.add(vertex_offset as usize)
            .cast::<[[f32; 8]; 6]>()
            .write(VERTICES);
        vram.add(fragment_offset as usize)
            .copy_from_nonoverlapping(fragment.microcode().as_ptr(), fragment.microcode().len());
        vram.add(textured_offset as usize).copy_from_nonoverlapping(
            textured_fragment.microcode().as_ptr(),
            textured_fragment.microcode().len(),
        );
        vram.add(index_offset as usize)
            .cast::<[u16; 3]>()
            .write([3, 4, 5]);
        for row in 0..4 {
            vram.add((texture_source + row * TEX_PITCH) as usize)
                .cast::<[u32; 4]>()
                .write([TEXEL; 4]);
        }
        let region = |offset| TransferMemory {
            location: Location::Local,
            offset,
            pitch: TEX_PITCH as i32,
        };
        context.copy_rows(region(texture_source), region(texture_dest), 16, 4)?;
        context.finish(0x1234_5677, 30_000)?;

        context.set_display_buffer(0, 0, pitch, size.width.into(), size.height.into())?;
        context.set_surface(&surface)?;
        let viewport = Viewport::for_surface(size.width, size.height);
        context.set_viewport(&viewport)?;
        context.set_scissor(viewport.rect)?;
        context.set_depth_state(false, false)?;
        context.set_blend_enabled(false)?;
        context.set_color_mask([true; 4])?;
        context.set_clear_color(COLOR)?;
        context.clear(ClearMask::COLOR)?;
        context.load_vertex_program(&vertex)?;
        context.load_fragment_program(&fragment, Location::Local, fragment_offset)?;
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
                location: Location::Local,
                offset,
            })?;
        }
        context.draw_arrays(Primitive::Triangles, 0, 3)?;
        context.load_vertex_program(&textured_vertex)?;
        context.load_fragment_program(&textured_fragment, Location::Local, textured_offset)?;
        let texture = Texture::linear_argb8(Location::Local, texture_dest, 4, 4, TEX_PITCH)?;
        context.bind_texture(0, &texture, &Sampler::default())?;
        context.draw_indexed(
            Primitive::Triangles,
            Location::Local,
            IndexType::U16,
            index_offset,
            3,
        )?;
        context.reset_flip_status();
        context.flip(0)?;
        context.finish(0x1234_5678, 30_000)?;
    }
    context.wait_for_display_flip(30_000)?;
    #[cfg(feature = "verify-framebuffer")]
    unsafe {
        let vram = config.local_address as usize as *const u8;
        let pixel = |x, y| {
            vram.add((pitch * y + x * 4) as usize)
                .cast::<u32>()
                .read_volatile()
                & 0x00ff_ffff
        };
        let (width, height) = (u32::from(size.width), u32::from(size.height));
        let corner = pixel(0, 0);
        let left = pixel(width / 4, height / 2);
        let right = pixel(width * 3 / 4, height / 2);
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
