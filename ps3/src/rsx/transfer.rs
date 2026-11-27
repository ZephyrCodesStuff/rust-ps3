use super::commands::packet;
use super::{Context, Error, Location, Rect, TransferScale, TransferSurface, TransferSwizzle};

const DMA_LOCAL: u32 = 0xfeed_0000;
const MEMORY_TO_MEMORY: u16 = 1 << 13;
const MAX_ROW_BYTES: u32 = 0x3f_ffff;
const MAX_ROWS: u32 = 0x7ff;
const LINEAR_SURFACE: u32 = 0x3133_71c3;
const SWIZZLED_SURFACE: u32 = 0x3133_7a73;

#[derive(Clone, Copy, Debug)]
pub enum ScaleTarget {
    Linear(TransferSurface),
    Swizzled(TransferSwizzle),
}

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
    /// Builds an A8R8G8B8 source-copy operation covering the full input image.
    /// The scaler uses a signed 12.20 step to advance through input pixels.
    pub fn argb8(
        source: TransferMemory,
        extent: [u16; 2],
        destination: Rect,
        linear: bool,
    ) -> Result<Self, Error> {
        let [width, height] = extent;
        let pitch = u16::try_from(source.pitch).map_err(|_| Error::InvalidArgument)?;
        let x = i16::try_from(destination.x).map_err(|_| Error::InvalidArgument)?;
        let y = i16::try_from(destination.y).map_err(|_| Error::InvalidArgument)?;
        if destination.width == 0
            || destination.height == 0
            || !(2..=2046).contains(&width)
            || width & 1 != 0
            || height == 0
            || height > 2047
            || u32::from(pitch) < u32::from(width) * 4
        {
            return Err(Error::InvalidArgument);
        }
        Ok(Self {
            conversion: 1,
            format: 3,
            operation: 3,
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
            origin: 2,
            interpolation: u8::from(linear),
            offset: source.offset,
            input_x: 0,
            input_y: 0,
        })
    }
}

impl TransferSurface {
    pub fn argb8(offset: u32, pitch: u16) -> Self {
        Self {
            format: 0x0a,
            pitch,
            reserved: 0,
            offset,
        }
    }
}

impl Context {
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
