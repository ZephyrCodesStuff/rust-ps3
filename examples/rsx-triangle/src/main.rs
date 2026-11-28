#![no_std]
#![no_main]

use core::ptr::addr_of_mut;
#[cfg(feature = "verify-framebuffer")]
use core::sync::atomic::{AtomicU32, Ordering};

use ps3::println;
use ps3::rsx::{
    video, Buffer, ClearMask, Compare, Context, Error, FlipMode, IoBuffer, LabelStage, Rect,
    ScaleTarget, Swapchain, TransferMemory, TransferScale, TransferSurface, Viewport,
};
#[cfg(feature = "verify-framebuffer")]
use ps3::rsx::{Event, EventCallback, ImageCopyTarget, ImagePixelSize};
use ps3::sys::sys_timer_usleep;

const MIB: usize = 1024 * 1024;
const IO_SIZE: usize = MIB;
const COLOR: u32 = 0xff35_78c8;
const TEXEL: u32 = 0xff00_dc00;
const TEX_PITCH: u32 = 64;
static mut IO: IoBuffer<IO_SIZE> = IoBuffer([0; IO_SIZE]);
static mut TEXTURE_UPLOAD: IoBuffer<IO_SIZE> = IoBuffer([0; IO_SIZE]);
#[cfg(feature = "verify-framebuffer")]
static USER_EVENT: AtomicU32 = AtomicU32::new(0);

#[cfg(feature = "verify-framebuffer")]
extern "C" fn on_user_event(cause: u32) {
    USER_EVENT.store(cause, Ordering::Release);
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
    #[cfg(feature = "verify-framebuffer")]
    let mut swizzled = vram.allocate(8 * 8 * 4, 128)?;
    if context.address_to_offset(back.storage().address())? != first_offset {
        return Err(Error::InvalidContext);
    }
    let mut upload =
        unsafe { context.map_main(addr_of_mut!(TEXTURE_UPLOAD).cast(), IO_SIZE as u32)? };
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

        context.set_surface(&surface)?;
        let viewport = Viewport::for_surface(size.width, size.height);
        context.set_viewport(&viewport)?;
        context.set_scissor(viewport.rect)?;
        context.set_depth_state(false, false)?;
        context.set_depth_compare(Compare::LessEqual)?;
        context.set_color_mask([true; 4])?;
        context.set_clear_color(COLOR)?;
        context.clear(ClearMask::COLOR)?;
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
        let right = pixel(width * 3 / 4, height / 2)?;
        println!(
            "RSX framebuffer readback: corner={:06x}, left={:06x}, right={:06x}",
            corner, left, right
        );
        assert_eq!(corner, COLOR & 0x00ff_ffff);
        assert_eq!(left, COLOR & 0x00ff_ffff);
        assert_eq!(right, COLOR & 0x00ff_ffff);
    }

    // Keep the clear visible before confirming that the buffers alternate.
    sys_timer_usleep(500_000);
    let next = display.back_buffer().ok_or(Error::InvalidContext)?;
    let next_offset = next.storage().offset();
    assert_ne!(next_offset, first_offset);
    unsafe {
        context.set_surface(&next.surface(&depth)?)?;
        context.set_clear_color(COLOR)?;
        context.clear(ClearMask::COLOR)?;
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
    println!("RSX clear and flip passed ({}x{})", size.width, size.height);
    Ok(())
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
