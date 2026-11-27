#![no_std]
#![no_main]

use core::ptr::addr_of_mut;

use ps3::println;
use ps3::rsx::{video, ClearMask, Context, Error, FlipMode, Surface};
use ps3::sys::sys_timer_usleep;

const MIB: usize = 1024 * 1024;
const IO_SIZE: usize = MIB;
const COLOR: u32 = 0xff35_78c8;

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
    if config.local_address == 0
        || depth_offset + color_bytes > config.local_size
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
        context.set_display_buffer(0, 0, pitch, size.width.into(), size.height.into())?;
        context.set_surface(&surface)?;
        context.set_clear_color(COLOR)?;
        context.clear(ClearMask::COLOR)?;
        context.reset_flip_status();
        context.flip(0)?;
        context.finish(0x1234_5678, 30_000)?;
    }
    context.wait_for_display_flip(30_000)?;

    // Let the first flip become visible before exiting the example.
    sys_timer_usleep(500_000);
    println!("RSX clear and flip passed ({}x{})", size.width, size.height);
    Ok(())
}
