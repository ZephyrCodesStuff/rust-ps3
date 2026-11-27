use super::commands::packet;
use super::{Context, Error, Location};

const DMA_LOCAL: u32 = 0xfeed_0000;
const MEMORY_TO_MEMORY: u16 = 1 << 13;
const MAX_ROW_BYTES: u32 = 0x3f_ffff;
const MAX_ROWS: u32 = 0x7ff;

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
}
