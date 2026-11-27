use super::commands::packet;
use super::{Context, Error, Method};

const BEGIN_END: u16 = 0x1808;
const DRAW_ARRAYS: u16 = 0x1814;
const INDEX_BUFFER: u16 = 0x181c;
const DRAW_INDEXED: u16 = 0x1824;
const INVALIDATE_VERTEX_CACHE: u16 = 0x1714;
const MAX_DRAW_INDEX: u32 = 0x00ff_ffff;

/// The GPU memory space containing a resource.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Location {
    Local = 0,
    Main = 1,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Primitive {
    Points = 1,
    Lines = 2,
    LineLoop = 3,
    LineStrip = 4,
    Triangles = 5,
    TriangleStrip = 6,
    TriangleFan = 7,
    Quads = 8,
    QuadStrip = 9,
    Polygon = 10,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VertexType {
    I16Normalized = 1,
    F32 = 2,
    F16 = 3,
    U8Normalized = 4,
    I16 = 5,
    PackedI11I11I10 = 6,
    U8 = 7,
}

#[derive(Clone, Copy, Debug)]
pub struct VertexAttribute {
    pub slot: u8,
    pub components: u8,
    pub format: VertexType,
    pub stride: u8,
    pub frequency: u16,
    pub location: Location,
    pub offset: u32,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IndexType {
    U32 = 0,
    U16 = 1,
}

impl IndexType {
    const fn bytes(self) -> u32 {
        match self {
            Self::U32 => 4,
            Self::U16 => 2,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    fn commands(self) -> [u32; 2] {
        [
            u32::from(self.x) | (u32::from(self.width) << 16),
            u32::from(self.y) | (u32::from(self.height) << 16),
        ]
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Viewport {
    pub rect: Rect,
    pub depth_min: f32,
    pub depth_max: f32,
    pub offset: [f32; 4],
    pub scale: [f32; 4],
}

impl Viewport {
    /// Builds a viewport for clip-space coordinates with a top-left origin.
    pub fn for_surface(width: u16, height: u16) -> Self {
        let half_width = f32::from(width) * 0.5;
        let half_height = f32::from(height) * 0.5;
        Self {
            rect: Rect {
                x: 0,
                y: 0,
                width,
                height,
            },
            depth_min: 0.0,
            depth_max: 1.0,
            offset: [half_width, half_height, 0.5, 0.0],
            scale: [half_width, -half_height, 0.5, 0.0],
        }
    }
}

impl Context {
    pub fn set_viewport(&mut self, viewport: &Viewport) -> Result<(), Error> {
        let transform = [
            packet(0x0a20, 8),
            viewport.offset[0].to_bits(),
            viewport.offset[1].to_bits(),
            viewport.offset[2].to_bits(),
            viewport.offset[3].to_bits(),
            viewport.scale[0].to_bits(),
            viewport.scale[1].to_bits(),
            viewport.scale[2].to_bits(),
            viewport.scale[3].to_bits(),
        ];
        unsafe {
            self.write_words(&[
                &[packet(0x0a00, 2)],
                &viewport.rect.commands(),
                &[
                    packet(0x0394, 2),
                    viewport.depth_min.to_bits(),
                    viewport.depth_max.to_bits(),
                ],
                &transform,
                &transform,
            ])
        }
    }

    pub fn set_scissor(&mut self, rect: Rect) -> Result<(), Error> {
        unsafe { self.write_words(&[&[packet(0x08c0, 2)], &rect.commands()]) }
    }

    pub fn set_depth_state(&mut self, test: bool, write: bool) -> Result<(), Error> {
        unsafe { self.write_words(&[&[packet(0x0a70, 2), write.into(), test.into()]]) }
    }

    pub fn set_blend_enabled(&mut self, enabled: bool) -> Result<(), Error> {
        unsafe { self.write_words(&[&[packet(0x0310, 1), enabled.into()]]) }
    }

    pub fn set_color_mask(&mut self, rgba: [bool; 4]) -> Result<(), Error> {
        let [r, g, b, a] = rgba.map(u32::from);
        unsafe { self.write_words(&[&[packet(0x0324, 1), b | g << 8 | r << 16 | a << 24]]) }
    }

    /// Makes an RSX memory range available as a vertex input.
    ///
    /// # Safety
    ///
    /// Keep all vertices read by subsequent draws mapped and unmodified
    /// until the GPU has completed those draws.
    pub unsafe fn set_vertex_attribute(&mut self, attribute: VertexAttribute) -> Result<(), Error> {
        if attribute.slot >= 16
            || !(1..=4).contains(&attribute.components)
            || attribute.offset >= (1 << 31)
        {
            return Err(Error::InvalidArgument);
        }
        let register = u16::from(attribute.slot) * 4;
        let format = u32::from(attribute.frequency) << 16
            | u32::from(attribute.stride) << 8
            | u32::from(attribute.components) << 4
            | u32::from(attribute.format as u8);
        let address = (u32::from(attribute.location as u8) << 31) | attribute.offset;
        unsafe {
            self.write_words(&[&[
                packet(0x1740 + register, 1),
                format,
                packet(0x1680 + register, 1),
                address,
            ]])
        }
    }

    /// Disables a buffered vertex input so the current immediate value is used.
    pub fn disable_vertex_attribute(&mut self, slot: u8) -> Result<(), Error> {
        if slot >= 16 {
            return Err(Error::InvalidArgument);
        }
        unsafe { self.write_words(&[&[packet(0x1740 + u16::from(slot) * 4, 1), 2]]) }
    }

    /// Sets the immediate four-component value of a vertex attribute.
    /// Disable its buffered array before using this value in a draw.
    pub fn set_vertex_attribute_constant(
        &mut self,
        slot: u8,
        value: [f32; 4],
    ) -> Result<(), Error> {
        if slot >= 16 {
            return Err(Error::InvalidArgument);
        }
        unsafe {
            self.write_words(&[
                &[packet(0x1c00 + u16::from(slot) * 16, 4)],
                &value.map(f32::to_bits),
            ])
        }
    }

    /// Draws interleaved vertex data supplied directly in the command stream.
    ///
    /// # Safety
    ///
    /// The words must match the active vertex formats and together encode
    /// a whole number of vertices. A failed refill can leave an incomplete
    /// primitive in the stream.
    pub unsafe fn draw_inline(&mut self, primitive: Primitive, words: &[u32]) -> Result<(), Error> {
        use super::methods::nv4097::INLINE_ARRAY;
        unsafe { self.draw_inline_words(primitive, INLINE_ARRAY, words) }
    }

    /// Draws using 32-bit indices embedded in the command stream.
    ///
    /// # Safety
    ///
    /// Every index must refer to live bound vertex data. A failed refill
    /// can leave an incomplete primitive in the stream.
    pub unsafe fn draw_inline_indices(
        &mut self,
        primitive: Primitive,
        indices: &[u32],
    ) -> Result<(), Error> {
        use super::methods::nv4097::ARRAY_ELEMENT32;
        unsafe { self.draw_inline_words(primitive, ARRAY_ELEMENT32, indices) }
    }

    unsafe fn draw_inline_words(
        &mut self,
        primitive: Primitive,
        method: Method,
        words: &[u32],
    ) -> Result<(), Error> {
        if words.is_empty() {
            return Ok(());
        }
        unsafe {
            self.begin_draw(primitive)?;
            for chunk in words.chunks(256) {
                self.write_method(method, false, chunk)?;
            }
            self.write_words(&[&[packet(BEGIN_END, 1), 0]])
        }
    }

    unsafe fn begin_draw(&mut self, primitive: Primitive) -> Result<(), Error> {
        unsafe {
            self.write_words(&[&[
                Method::new(INVALIDATE_VERTEX_CACHE)
                    .unwrap()
                    .header(3, false),
                0,
                0,
                0,
                packet(BEGIN_END, 1),
                primitive as u32,
            ]])
        }
    }

    /// Draws vertices in order from the bound attribute arrays.
    ///
    /// # Safety
    ///
    /// The pipeline and selected arrays must be valid for the specified
    /// vertex range. If a refill fails after part of a draw was emitted,
    /// discard the pending stream or close the primitive before drawing again.
    pub unsafe fn draw_arrays(
        &mut self,
        primitive: Primitive,
        first: u32,
        count: u32,
    ) -> Result<(), Error> {
        if count == 0 {
            return Ok(());
        }
        if first
            .checked_add(count - 1)
            .is_none_or(|last| last > MAX_DRAW_INDEX)
        {
            return Err(Error::InvalidArgument);
        }
        unsafe {
            self.begin_draw(primitive)?;
            self.emit_draw_batches(DRAW_ARRAYS, first, count, 256)?;
            self.write_words(&[&[packet(BEGIN_END, 1), 0]])
        }
    }

    /// Draws indexed vertices from an RSX memory range.
    ///
    /// # Safety
    ///
    /// The index and vertex ranges and bound pipeline state must be valid
    /// for the draw and stay live until the GPU has finished. A failed FIFO
    /// refill can leave an open primitive in the stream.
    pub unsafe fn draw_indexed(
        &mut self,
        primitive: Primitive,
        location: Location,
        index_type: IndexType,
        offset: u32,
        count: u32,
    ) -> Result<(), Error> {
        if count == 0 {
            return Ok(());
        }
        let bytes = index_type.bytes();
        if count - 1 > MAX_DRAW_INDEX || offset % bytes != 0 || offset >= 0x2000_0000 {
            return Err(Error::InvalidArgument);
        }
        // End the first batch at a 128-byte boundary if the base is unaligned.
        let first_batch = ((128 - (offset % 128)) % 128) / bytes;
        let first_batch = if first_batch == 0 { 256 } else { first_batch };
        let format = u32::from(index_type as u8) << 4 | u32::from(location as u8);
        unsafe {
            self.write_words(&[&[
                packet(INVALIDATE_VERTEX_CACHE, 1),
                0,
                packet(INDEX_BUFFER, 2),
                offset,
                format,
                packet(BEGIN_END, 1),
                primitive as u32,
            ]])?;
            self.emit_draw_batches(DRAW_INDEXED, 0, count, first_batch)?;
            self.write_words(&[&[packet(BEGIN_END, 1), 0]])
        }
    }

    unsafe fn emit_draw_batches(
        &mut self,
        method: u16,
        mut first: u32,
        mut count: u32,
        mut batch_limit: u32,
    ) -> Result<(), Error> {
        let method = Method::new(method).unwrap();
        let mut batches = [0; 32];
        while count != 0 {
            let mut words = 0;
            for word in &mut batches {
                if count == 0 {
                    break;
                }
                let batch = count.min(batch_limit);
                *word = ((batch - 1) << 24) | first;
                words += 1;
                first += batch;
                count -= batch;
                batch_limit = 256;
            }
            unsafe { self.write_method(method, false, &batches[..words])? };
        }
        Ok(())
    }
}
