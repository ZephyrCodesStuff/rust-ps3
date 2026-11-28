//! GCM records as laid out by the PPU SDK.

/// DMA FIFO control registers. Access these fields using volatile operations.
#[repr(C)]
pub struct Control {
    pub put: u32,
    pub get: u32,
    pub reference: u32,
}

/// Memory ranges and frequencies supplied by the firmware after GCM init.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Config {
    pub local_address: u32,
    pub io_address: u32,
    pub local_size: u32,
    pub io_size: u32,
    pub memory_frequency: u32,
    pub core_frequency: u32,
}

pub const COLOR_TARGETS: usize = 4;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Surface {
    pub kind: u8,
    pub antialias: u8,
    pub color_format: u8,
    pub color_target: u8,
    pub color_location: [u8; COLOR_TARGETS],
    pub color_offset: [u32; COLOR_TARGETS],
    pub color_pitch: [u32; COLOR_TARGETS],
    pub depth_format: u8,
    pub depth_location: u8,
    pub reserved: [u8; 2],
    pub depth_offset: u32,
    pub depth_pitch: u32,
    pub width: u16,
    pub height: u16,
    pub x: u16,
    pub y: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Texture {
    pub format: u8,
    pub mipmaps: u8,
    pub dimension: u8,
    pub cubemap: u8,
    pub remap: u32,
    pub width: u16,
    pub height: u16,
    pub depth: u16,
    pub location: u8,
    pub reserved: u8,
    pub pitch: u32,
    pub offset: u32,
}

/// Firmware FIFO context. Addresses and the compact callback descriptor
/// address are all 32 bit even though Rust uses ELFv1 64-bit pointers.
#[repr(C)]
#[derive(Debug, Default)]
pub struct ContextData {
    pub begin: u32,
    pub end: u32,
    pub current: u32,
    pub callback: u32,
}

/// Effective addresses of the two GCM translation tables.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct OffsetTable {
    pub effective_to_io: u32,
    pub io_to_effective: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union Cast {
    pub word: u32,
    pub float: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct ReportData {
    pub timer: u64,
    pub value: u32,
    pub reserved: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct NotifyData {
    pub timer: u64,
    pub reserved: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct TransferScale {
    pub conversion: u32,
    pub format: u32,
    pub operation: u32,
    pub clip_x: i16,
    pub clip_y: i16,
    pub clip_width: u16,
    pub clip_height: u16,
    pub output_x: i16,
    pub output_y: i16,
    pub output_width: u16,
    pub output_height: u16,
    pub ratio_x: i32,
    pub ratio_y: i32,
    pub input_width: u16,
    pub input_height: u16,
    pub pitch: u16,
    pub origin: u8,
    pub interpolation: u8,
    pub offset: u32,
    pub input_x: u16,
    pub input_y: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct TransferSurface {
    pub format: u32,
    pub pitch: u16,
    pub reserved: u16,
    pub offset: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct TransferSwizzle {
    pub format: u16,
    /// Base-2 logarithm of the target width in pixels.
    pub width: u8,
    /// Base-2 logarithm of the target height in pixels.
    pub height: u8,
    pub offset: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct TileInfo {
    pub tile: u32,
    pub limit: u32,
    pub pitch: u32,
    pub format: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct ZcullInfo {
    pub region: u32,
    pub size: u32,
    pub start: u32,
    pub offset: u32,
    pub status0: u32,
    pub status1: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct DisplayInfo {
    pub offset: u32,
    pub pitch: u32,
    pub width: u32,
    pub height: u32,
}

const _: () = {
    use core::mem::{align_of, offset_of, size_of};
    assert!(size_of::<Control>() == 12);
    assert!(size_of::<Config>() == 24);
    assert!(size_of::<Surface>() == 60);
    assert!(offset_of!(Surface, depth_offset) == 44);
    assert!(offset_of!(Surface, width) == 52);
    assert!(size_of::<Texture>() == 24);
    assert!(offset_of!(Texture, pitch) == 16);
    assert!(size_of::<ContextData>() == 16);
    assert!(offset_of!(ContextData, callback) == 12);
    assert!(size_of::<OffsetTable>() == 8);
    assert!(size_of::<Cast>() == 4);
    assert!(size_of::<ReportData>() == 16);
    assert!(align_of::<ReportData>() == 8);
    assert!(size_of::<NotifyData>() == 16);
    assert!(size_of::<TransferScale>() == 52);
    assert!(offset_of!(TransferScale, offset) == 44);
    assert!(size_of::<TransferSurface>() == 12);
    assert!(size_of::<TransferSwizzle>() == 8);
    assert!(size_of::<TileInfo>() == 16);
    assert!(size_of::<ZcullInfo>() == 24);
    assert!(size_of::<DisplayInfo>() == 16);
};
