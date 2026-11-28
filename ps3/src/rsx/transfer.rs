use super::commands::packet;
use super::methods::{nv3062, nv308a};
use super::{Context, Error, Location, Rect, TransferScale, TransferSurface, TransferSwizzle};

const DMA_LOCAL: u32 = 0xfeed_0000;
const MEMORY_TO_MEMORY: u16 = 1 << 13;
const MAX_ROW_BYTES: u32 = 0x3f_ffff;
const MAX_ROWS: u32 = 0x7ff;
const LINEAR_SURFACE: u32 = 0x3133_71c3;
const SWIZZLED_SURFACE: u32 = 0x3133_7a73;
const BLIT_BLOCK: u32 = 1024;

/// Storage width of each pixel in an image copy.
#[derive(Clone, Copy, Debug)]
pub enum ImagePixelSize {
    Bits16 = 2,
    Bits32 = 4,
    Bits64 = 8,
    Bits128 = 16,
}

#[derive(Clone, Copy, Debug)]
pub enum ImageCopyTarget {
    Linear(TransferMemory),
    Swizzled {
        location: Location,
        offset: u32,
        /// Power-of-two width and height in pixels.
        extent: [u16; 2],
    },
}

#[derive(Clone, Copy, Debug)]
pub enum ScaleTarget {
    Linear(TransferSurface),
    Swizzled(TransferSwizzle),
}

/// Input pixel encoding accepted by the RSX scaler (Cell GCM transfer format).
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransferScaleFormat {
    A1R5G5B5 = 1,
    X1R5G5B5 = 2,
    A8R8G8B8 = 3,
    X8R8G8B8 = 4,
    Cr8Yb8Cb8Ya8 = 5,
    Yb8Cr8Ya8Cb8 = 6,
    R5G6B5 = 7,
    Y8 = 8,
    Ay8 = 9,
    EyB8EcR8EyA8EcB8 = 0xA,
    EcR8EyB8EcB8EyA8 = 0xB,
    A8B8G8R8 = 0xC,
    X8B8G8R8 = 0xD,
}

impl TransferScaleFormat {
    /// Returns the verified storage size for formats with one value per pixel.
    /// Packed YUV and AY8 are omitted because their pitch unit is not a simple
    /// per-texel byte count in the SDK's scaler interface.
    const fn bytes_per_pixel(self) -> Option<u32> {
        match self {
            Self::A1R5G5B5 | Self::X1R5G5B5 | Self::R5G6B5 => Some(2),
            Self::A8R8G8B8 | Self::X8R8G8B8 | Self::A8B8G8R8 | Self::X8B8G8R8 => Some(4),
            Self::Y8 => Some(1),
            _ => None,
        }
    }
}

/// Operation applied by the Cell GCM scaler.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransferOperation {
    SourceCopyAnd = 0,
    RasterOperationAnd = 1,
    BlendAnd = 2,
    SourceCopy = 3,
    SourceCopyPremultiplied = 4,
    BlendPremultiplied = 5,
}

/// Scaler conversion mode.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransferConversion {
    Dither = 0,
    Truncate = 1,
    SubtractTruncate = 2,
}

/// Input coordinate origin used by the scaler.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransferOrigin {
    Center = 1,
    Corner = 2,
}

/// Scaler interpolation mode.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransferInterpolation {
    ZeroOrderHold = 0,
    FirstOrderHold = 1,
}

/// Output format accepted by a linear transfer surface.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransferSurfaceFormat {
    R5G6B5 = 4,
    A8R8G8B8 = 0xA,
    Y32 = 0xB,
}

// Pin ABI constants to the values in the Cell SDK transfer enums.
const _: () = {
    assert!(TransferScaleFormat::A1R5G5B5 as u32 == 1);
    assert!(TransferScaleFormat::X1R5G5B5 as u32 == 2);
    assert!(TransferScaleFormat::A8R8G8B8 as u32 == 3);
    assert!(TransferScaleFormat::X8R8G8B8 as u32 == 4);
    assert!(TransferScaleFormat::Cr8Yb8Cb8Ya8 as u32 == 5);
    assert!(TransferScaleFormat::Yb8Cr8Ya8Cb8 as u32 == 6);
    assert!(TransferScaleFormat::R5G6B5 as u32 == 7);
    assert!(TransferScaleFormat::Y8 as u32 == 8);
    assert!(TransferScaleFormat::Ay8 as u32 == 9);
    assert!(TransferScaleFormat::EyB8EcR8EyA8EcB8 as u32 == 0xA);
    assert!(TransferScaleFormat::EcR8EyB8EcB8EyA8 as u32 == 0xB);
    assert!(TransferScaleFormat::A8B8G8R8 as u32 == 0xC);
    assert!(TransferScaleFormat::X8B8G8R8 as u32 == 0xD);
    assert!(TransferOperation::SourceCopyAnd as u32 == 0);
    assert!(TransferOperation::RasterOperationAnd as u32 == 1);
    assert!(TransferOperation::BlendAnd as u32 == 2);
    assert!(TransferOperation::SourceCopy as u32 == 3);
    assert!(TransferOperation::SourceCopyPremultiplied as u32 == 4);
    assert!(TransferOperation::BlendPremultiplied as u32 == 5);
    assert!(TransferConversion::Dither as u32 == 0);
    assert!(TransferConversion::Truncate as u32 == 1);
    assert!(TransferConversion::SubtractTruncate as u32 == 2);
    assert!(TransferOrigin::Center as u8 == 1);
    assert!(TransferOrigin::Corner as u8 == 2);
    assert!(TransferInterpolation::ZeroOrderHold as u8 == 0);
    assert!(TransferInterpolation::FirstOrderHold as u8 == 1);
    assert!(TransferSurfaceFormat::R5G6B5 as u32 == 4);
    assert!(TransferSurfaceFormat::A8R8G8B8 as u32 == 0xA);
    assert!(TransferSurfaceFormat::Y32 as u32 == 0xB);
};

#[derive(Clone, Copy, Debug)]
pub struct TransferMemory {
    pub location: Location,
    /// Offset of the first row in the selected memory space.
    pub offset: u32,
    /// Signed byte distance between consecutive rows.
    pub pitch: i32,
}

impl TransferMemory {
    fn row_offset(self, row: u32) -> i64 {
        i64::from(self.offset) + i64::from(self.pitch) * i64::from(row)
    }

    fn valid(self, width: u32, rows: u32) -> bool {
        let first = i64::from(self.offset);
        let last = self.row_offset(rows - 1);
        first.min(last) >= 0 && first.max(last) <= i64::from(u32::MAX - (width - 1))
    }
}

impl TransferScale {
    /// Builds a scaler operation with explicit Cell GCM formats and modes.
    /// The input image is scaled in full to `destination`.
    pub fn configured(
        source: TransferMemory,
        extent: [u16; 2],
        destination: Rect,
        format: TransferScaleFormat,
        operation: TransferOperation,
        conversion: TransferConversion,
        origin: TransferOrigin,
        interpolation: TransferInterpolation,
    ) -> Result<Self, Error> {
        let [width, height] = extent;
        let pitch = u16::try_from(source.pitch).map_err(|_| Error::InvalidArgument)?;
        let row_bytes = format
            .bytes_per_pixel()
            .map(|bytes| u32::from(width) * bytes);
        let x = i16::try_from(destination.x).map_err(|_| Error::InvalidArgument)?;
        let y = i16::try_from(destination.y).map_err(|_| Error::InvalidArgument)?;
        if destination.width == 0
            || destination.height == 0
            || !(2..=2046).contains(&width)
            || width & 1 != 0
            || height == 0
            || height > 2047
            || pitch == 0
            || row_bytes.is_some_and(|bytes| u32::from(pitch) < bytes)
            || row_bytes.is_some_and(|bytes| !source.valid(bytes, u32::from(height)))
        {
            return Err(Error::InvalidArgument);
        }
        Ok(Self {
            conversion: conversion as u32,
            format: format as u32,
            operation: operation as u32,
            clip_x: x,
            clip_y: y,
            clip_width: destination.width,
            clip_height: destination.height,
            output_x: x,
            output_y: y,
            output_width: destination.width,
            output_height: destination.height,
            ratio_x: (i32::from(width) << 20) / i32::from(destination.width),
            ratio_y: (i32::from(height) << 20) / i32::from(destination.height),
            input_width: width,
            input_height: height,
            pitch,
            origin: origin as u8,
            interpolation: interpolation as u8,
            offset: source.offset,
            input_x: 0,
            input_y: 0,
        })
    }

    /// Builds an A8R8G8B8 source-copy operation covering the full input image.
    /// The scaler uses a signed 12.20 step to advance through input pixels.
    pub fn argb8(
        source: TransferMemory,
        extent: [u16; 2],
        destination: Rect,
        linear: bool,
    ) -> Result<Self, Error> {
        if u32::try_from(source.pitch)
            .ok()
            .is_none_or(|pitch| pitch < u32::from(extent[0]) * 4)
        {
            return Err(Error::InvalidArgument);
        }
        Self::configured(
            source,
            extent,
            destination,
            TransferScaleFormat::A8R8G8B8,
            TransferOperation::SourceCopy,
            TransferConversion::Truncate,
            TransferOrigin::Corner,
            if linear {
                TransferInterpolation::FirstOrderHold
            } else {
                TransferInterpolation::ZeroOrderHold
            },
        )
    }
}

impl TransferSurface {
    /// Builds a transfer target surface with a typed Cell GCM output format.
    pub const fn new(format: TransferSurfaceFormat, offset: u32, pitch: u16) -> Self {
        Self {
            format: format as u32,
            pitch,
            reserved: 0,
            offset,
        }
    }

    pub fn argb8(offset: u32, pitch: u16) -> Self {
        Self::new(TransferSurfaceFormat::A8R8G8B8, offset, pitch)
    }
}

impl Context {
    /// Copies an unscaled rectangle from a pitched source image to a pitched
    /// or swizzled destination. Linear targets support 16- and 32-bit pixels.
    /// Swizzled targets also support 64- and 128-bit pixels.
    ///
    /// # Safety
    /// The images must be live, nonoverlapping GPU allocations. Each 2D
    /// blit may read up to the next even input width, with a minimum input
    /// width of 16 elements of the selected 16- or 32-bit transfer format.
    /// Keep this source padding accessible. Synchronize CPU and GPU access
    /// to both allocations until the copy completes.
    pub unsafe fn copy_image(
        &mut self,
        source: TransferMemory,
        rect: Rect,
        destination: ImageCopyTarget,
        origin: [u16; 2],
        pixel_size: ImagePixelSize,
    ) -> Result<(), Error> {
        if rect.width == 0 || rect.height == 0 {
            return Ok(());
        }
        let pixel_bytes = pixel_size as u32;
        let src_pitch = u16::try_from(source.pitch).map_err(|_| Error::InvalidArgument)?;
        let width = u32::from(rect.width);
        let height = u32::from(rect.height);
        let [dst_x, dst_y] = origin.map(u32::from);
        if (u32::from(rect.x) + width) * pixel_bytes > u32::from(src_pitch)
            || source.offset % pixel_bytes.min(4) != 0
        {
            return Err(Error::InvalidArgument);
        }
        let src_offset = checked_offset(
            source.offset,
            u32::from(rect.x) * pixel_bytes,
            u32::from(rect.y),
            u32::from(src_pitch),
        )?;
        let source_region = TransferMemory {
            offset: src_offset,
            ..source
        };
        if !source_region.valid(width * pixel_bytes, height) {
            return Err(Error::InvalidArgument);
        }

        let (target, dst_pitch) = match destination {
            ImageCopyTarget::Linear(target) => {
                let pitch = u16::try_from(target.pitch).map_err(|_| Error::InvalidArgument)?;
                if pixel_bytes > 4
                    || target.offset % 64 != 0
                    || pitch % 64 != 0
                    || (dst_x + width) * pixel_bytes > u32::from(pitch)
                {
                    return Err(Error::InvalidArgument);
                }
                checked_offset(
                    target.offset,
                    (dst_x + width) * pixel_bytes - 1,
                    dst_y + height - 1,
                    u32::from(pitch),
                )?;
                (target, pitch)
            }
            ImageCopyTarget::Swizzled {
                location,
                offset,
                extent,
            } => {
                let [w, h] = extent.map(u32::from);
                if !w.is_power_of_two()
                    || !h.is_power_of_two()
                    || w > 4096
                    || h > 4096
                    || dst_x + width > w
                    || dst_y + height > h
                {
                    return Err(Error::InvalidArgument);
                }
                offset
                    .checked_add(w * h * pixel_bytes - 1)
                    .ok_or(Error::InvalidArgument)?;
                let element_bytes = pixel_bytes.min(4);
                let elements = pixel_bytes / element_bytes;
                let copy = SwizzleCopy {
                    source: source_region,
                    destination: location,
                    element_bytes,
                    start: [dst_x * elements, dst_y],
                    end: [(dst_x + width) * elements, dst_y + height],
                };
                return unsafe { copy.block(self, [0, 0], [w * elements, h], offset) };
            }
        };

        let end_x = dst_x + width;
        let end_y = dst_y + height;
        let (scale_format, surface_format) = image_formats(pixel_bytes);
        let mut y = dst_y;
        while y < end_y {
            let top = y & !(BLIT_BLOCK - 1);
            let rows = end_y.min(top + BLIT_BLOCK) - y;
            let mut x = dst_x;
            while x < end_x {
                let left = x & !(BLIT_BLOCK - 1);
                let columns = end_x.min(left + BLIT_BLOCK) - x;
                let surface = ScaleTarget::Linear(TransferSurface {
                    format: surface_format.into(),
                    pitch: dst_pitch,
                    reserved: 0,
                    offset: target.offset + top * u32::from(dst_pitch) + left * pixel_bytes,
                });
                let input = TransferMemory {
                    offset: src_offset
                        + (y - dst_y) * u32::from(src_pitch)
                        + (x - dst_x) * pixel_bytes,
                    ..source
                };
                let region = Rect {
                    x: (x - left) as u16,
                    y: (y - top) as u16,
                    width: columns as u16,
                    height: rows as u16,
                };
                let scale = image_scale(input, region, scale_format);
                unsafe { self.scale_image(source.location, target.location, &scale, surface)? };
                x += columns;
            }
            y += rows;
        }
        Ok(())
    }

    /// Uploads native-endian words from the FIFO to GPU-visible memory.
    ///
    /// # Safety
    /// The destination must remain mapped and cover the full write. Order
    /// concurrent CPU or GPU accesses to that range around the transfer.
    pub unsafe fn inline_transfer(
        &mut self,
        location: Location,
        offset: u32,
        words: &[u32],
    ) -> Result<(), Error> {
        if words.is_empty() {
            return Ok(());
        }
        let bytes = u32::try_from(words.len())
            .ok()
            .and_then(|count| count.checked_mul(4))
            .ok_or(Error::InvalidArgument)?;
        if offset % 4 != 0 || offset.checked_add(bytes - 1).is_none() {
            return Err(Error::InvalidArgument);
        }

        // Each image is one row of Y32 pixels. The transfer unit requires
        // an aligned surface base and an even number of payload words.
        let mut destination = offset;
        for block in words.chunks(512) {
            let count = block.len();
            let padding = count % 2;
            let extent = (1 << 16) | count as u32;
            let setup = [
                nv3062::SET_CONTEXT_DMA_IMAGE_DESTIN.header(1, true),
                DMA_LOCAL + location as u32,
                nv3062::SET_OFFSET_DESTIN.header(1, true),
                destination & !63,
                nv3062::SET_COLOR_FORMAT.header(2, true),
                0x0b, // Y32
                0x1000_1000,
                nv308a::POINT.header(3, true),
                (destination & 63) / 4,
                extent,
                extent,
                nv308a::COLOR.header(count + padding, true),
            ];
            unsafe { self.write_words(&[&setup, block, &[0][..padding]])? };
            destination = destination.wrapping_add(count as u32 * 4);
        }
        Ok(())
    }

    /// Copies rectangular rows with the RSX memory-to-memory engine.
    ///
    /// # Safety
    ///
    /// Source and destination must be live, nonoverlapping GPU allocations
    /// with sufficient storage for the specified rectangle. Synchronize
    /// accesses by the CPU and other GPU engines until the copy completes.
    pub unsafe fn copy_rows(
        &mut self,
        source: TransferMemory,
        destination: TransferMemory,
        mut row_bytes: u32,
        mut rows: u32,
    ) -> Result<(), Error> {
        if rows == 0 || row_bytes == 0 {
            return Ok(());
        }
        if !source.valid(row_bytes, rows) || !destination.valid(row_bytes, rows) {
            return Err(Error::InvalidArgument);
        }

        let same_contiguous_stride =
            u32::try_from(source.pitch) == Ok(row_bytes) && source.pitch == destination.pitch;
        let (pitch_in, pitch_out) = if same_contiguous_stride {
            row_bytes = row_bytes.checked_mul(rows).ok_or(Error::InvalidArgument)?;
            rows = 1;
            (0, 0)
        } else {
            (source.pitch, destination.pitch)
        };
        let can_batch = i16::try_from(pitch_in).is_ok() && i16::try_from(pitch_out).is_ok();
        let row_limit = if can_batch { MAX_ROWS } else { 1 };

        unsafe {
            self.write_words(&[&[
                packet(MEMORY_TO_MEMORY | 0x0184, 2),
                DMA_LOCAL + source.location as u32,
                DMA_LOCAL + destination.location as u32,
            ]])?;
            let mut row = 0;
            while row < rows {
                let block_rows = (rows - row).min(row_limit);
                let input = source.row_offset(row) as u32;
                let output = destination.row_offset(row) as u32;
                let mut column = 0;
                while column < row_bytes {
                    let width = (row_bytes - column).min(MAX_ROW_BYTES);
                    let pitch_in = if block_rows == 1 { 0 } else { pitch_in as u32 };
                    let pitch_out = if block_rows == 1 { 0 } else { pitch_out as u32 };
                    self.write_words(&[&[
                        packet(MEMORY_TO_MEMORY | 0x030c, 8),
                        input + column,
                        output + column,
                        pitch_in,
                        pitch_out,
                        width,
                        block_rows,
                        0x0101,
                        0,
                    ]])?;
                    column += width;
                }
                row += block_rows;
            }
        }
        Ok(())
    }

    /// Scales or converts a source image using the RSX's 2D engine.
    ///
    /// # Safety
    ///
    /// The source and target descriptions must refer to live, nonoverlapping
    /// GPU allocations with sufficient memory for the transfer and its
    /// selected formats. Synchronize before reading or reusing either region.
    pub unsafe fn scale_image(
        &mut self,
        source: Location,
        destination: Location,
        scale: &TransferScale,
        target: ScaleTarget,
    ) -> Result<(), Error> {
        if !(2..=2046).contains(&scale.input_width)
            || scale.input_width & 1 != 0
            || scale.input_height == 0
            || scale.input_height > 2047
            || scale.pitch == 0
            || scale.output_width == 0
            || scale.output_height == 0
            || !(1..=2).contains(&scale.origin)
            || scale.interpolation > 1
        {
            return Err(Error::InvalidArgument);
        }
        let mut target_words = [0; 7];
        let dma = DMA_LOCAL + destination as u32;
        let (obj, count) = match target {
            ScaleTarget::Linear(surface) => {
                if surface.pitch < 64 || surface.pitch % 64 != 0 || surface.offset % 64 != 0 {
                    return Err(Error::InvalidArgument);
                }
                target_words = [
                    packet(0x6188, 1),
                    dma,
                    packet(0x6300, 4),
                    surface.format,
                    (u32::from(surface.pitch) << 16) | 64,
                    0,
                    surface.offset,
                ];
                (LINEAR_SURFACE, 7)
            }
            ScaleTarget::Swizzled(surface) => {
                if surface.width > 11 || surface.height > 11 || surface.offset % 64 != 0 {
                    return Err(Error::InvalidArgument);
                }
                target_words[..5].copy_from_slice(&[
                    packet(0x8184, 1),
                    dma,
                    packet(0x8300, 2),
                    (u32::from(surface.height) << 24)
                        | (u32::from(surface.width) << 16)
                        | u32::from(surface.format),
                    surface.offset,
                ]);
                (SWIZZLED_SURFACE, 5)
            }
        };
        let xy = |x, y| u32::from(x) | (u32::from(y) << 16);
        unsafe {
            self.write_words(&[
                &[
                    packet(0xc184, 1),
                    DMA_LOCAL + source as u32,
                    packet(0xc198, 1),
                    obj,
                ],
                &target_words[..count],
                &[
                    packet(0xc2fc, 9),
                    scale.conversion,
                    scale.format,
                    scale.operation,
                    xy(scale.clip_x as u16, scale.clip_y as u16),
                    xy(scale.clip_width, scale.clip_height),
                    xy(scale.output_x as u16, scale.output_y as u16),
                    xy(scale.output_width, scale.output_height),
                    scale.ratio_x as u32,
                    scale.ratio_y as u32,
                    packet(0xc400, 4),
                    xy(scale.input_width, scale.input_height),
                    u32::from(scale.pitch)
                        | (u32::from(scale.origin) << 16)
                        | (u32::from(scale.interpolation) << 24),
                    scale.offset,
                    xy(scale.input_x, scale.input_y),
                ],
            ])
        }
    }
}

fn checked_offset(base: u32, x_bytes: u32, y: u32, pitch: u32) -> Result<u32, Error> {
    u32::try_from(u64::from(base) + u64::from(x_bytes) + u64::from(y) * u64::from(pitch))
        .map_err(|_| Error::InvalidArgument)
}

fn image_formats(element_bytes: u32) -> (u32, u16) {
    if element_bytes == 2 {
        (7, 4)
    } else {
        (3, 10)
    }
}

fn image_scale(source: TransferMemory, region: Rect, format: u32) -> TransferScale {
    TransferScale {
        conversion: 1,
        format,
        operation: 3,
        clip_x: region.x as i16,
        clip_y: region.y as i16,
        clip_width: region.width,
        clip_height: region.height,
        output_x: region.x as i16,
        output_y: region.y as i16,
        output_width: region.width,
        output_height: region.height,
        ratio_x: 1 << 20,
        ratio_y: 1 << 20,
        input_width: region.width.max(16).next_multiple_of(2),
        input_height: region.height,
        pitch: source.pitch as u16,
        origin: 2,
        interpolation: 0,
        offset: source.offset,
        input_x: 0,
        input_y: 0,
    }
}

struct SwizzleCopy {
    source: TransferMemory,
    destination: Location,
    element_bytes: u32,
    start: [u32; 2],
    end: [u32; 2],
}

impl SwizzleCopy {
    unsafe fn block(
        &self,
        context: &mut Context,
        [x, y]: [u32; 2],
        [w, h]: [u32; 2],
        offset: u32,
    ) -> Result<(), Error> {
        let left = x.max(self.start[0]);
        let top = y.max(self.start[1]);
        let right = (x + w).min(self.end[0]);
        let bottom = (y + h).min(self.end[1]);
        if left >= right || top >= bottom {
            return Ok(());
        }
        let input = TransferMemory {
            offset: self.source.offset
                + (top - self.start[1]) * self.source.pitch as u32
                + (left - self.start[0]) * self.element_bytes,
            ..self.source
        };
        if w <= 2 || h == 1 {
            // In these dimensions the low Morton bits lay out as pitched rows.
            let output = TransferMemory {
                location: self.destination,
                offset: offset + ((top - y) * w + (left - x)) * self.element_bytes,
                pitch: (w * self.element_bytes) as i32,
            };
            return unsafe {
                context.copy_rows(
                    input,
                    output,
                    (right - left) * self.element_bytes,
                    bottom - top,
                )
            };
        }

        if left == x
            && top == y
            && right == x + w
            && bottom == y + h
            && w <= BLIT_BLOCK
            && h <= BLIT_BLOCK
            && offset % 64 == 0
        {
            let (scale_format, surface_format) = image_formats(self.element_bytes);
            let target = ScaleTarget::Swizzled(TransferSwizzle {
                format: surface_format,
                width: w.ilog2() as u8,
                height: h.ilog2() as u8,
                offset,
            });
            let scale = image_scale(
                input,
                Rect {
                    x: 0,
                    y: 0,
                    width: w as u16,
                    height: h as u16,
                },
                scale_format,
            );
            return unsafe {
                context.scale_image(input.location, self.destination, &scale, target)
            };
        }

        // Remove the highest Morton bit. Equal dimensions interleave Y above
        // X. Both halves occupy contiguous, equal-sized address ranges.
        let second_offset = offset + w * h / 2 * self.element_bytes;
        let (half, second) = if h >= w {
            ([w, h / 2], [x, y + h / 2])
        } else {
            ([w / 2, h], [x + w / 2, y])
        };
        unsafe {
            self.block(context, [x, y], half, offset)?;
            self.block(context, second, half, second_offset)
        }
    }
}
