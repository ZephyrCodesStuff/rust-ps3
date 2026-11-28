use super::commands::packet;
use super::methods::nv4097;
use super::{Context, Error, Method};

const BEGIN_END: u16 = 0x1808;
const DRAW_ARRAYS: u16 = 0x1814;
const INDEX_BUFFER: u16 = 0x181c;
const DRAW_INDEXED: u16 = 0x1824;
const INVALIDATE_VERTEX_CACHE: u16 = 0x1714;
const MAX_DRAW_INDEX: u32 = 0x00ff_ffff;
const MAX_INLINE_INDEX_COUNT: usize = 0x000f_ffff;
const MAX_METHOD_COUNT: usize = 0x7ff;

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

/// A vertex input encoded in the FIFO with the selected GCM format.
#[derive(Clone, Copy, Debug)]
pub enum ImmediateVertex {
    Float1(f32),
    Float2([f32; 2]),
    Float3([f32; 3]),
    Float4([f32; 4]),
    Short2([i16; 2]),
    Short4([i16; 4]),
    ScaledShort4([i16; 4]),
    NormalizedByte4([u8; 4]),
}

fn pack_shorts(low: i16, high: i16) -> u32 {
    u32::from(low as u16) | (u32::from(high as u16) << 16)
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
        let format = vertex_format(
            attribute.format,
            attribute.components,
            attribute.stride,
            attribute.frequency,
        );
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

    /// Updates the format of an array independently of its current address.
    /// A zero component count selects the slot's immediate value.
    pub fn set_vertex_attribute_format(
        &mut self,
        slot: u8,
        format: VertexType,
        components: u8,
        stride: u8,
        frequency: u16,
    ) -> Result<(), Error> {
        if slot >= 16 || components > 4 {
            return Err(Error::InvalidArgument);
        }
        let value = vertex_format(format, components, stride, frequency);
        unsafe { self.write_words(&[&[packet(0x1740 + u16::from(slot) * 4, 1), value]]) }
    }

    /// Changes the address of an array independently of its format.
    ///
    /// # Safety
    /// The address and active format must describe a live vertex range
    /// large enough for every dependent draw.
    pub unsafe fn set_vertex_attribute_offset(
        &mut self,
        slot: u8,
        location: Location,
        offset: u32,
    ) -> Result<(), Error> {
        if slot >= 16 || offset >= (1 << 31) {
            return Err(Error::InvalidArgument);
        }
        let address = (u32::from(location as u8) << 31) | offset;
        unsafe { self.write_words(&[&[packet(0x1680 + u16::from(slot) * 4, 1), address]]) }
    }

    /// Sets the shared byte offset and index base for vertex array fetches.
    ///
    /// # Safety
    /// Every enabled attribute array must cover the addresses selected by
    /// subsequent draws with these base values.
    pub unsafe fn set_vertex_data_base(&mut self, offset: u32, index: u32) -> Result<(), Error> {
        unsafe {
            self.write_words(&[&[
                nv4097::SET_VERTEX_DATA_BASE_OFFSET.header(2, true),
                offset,
                index,
            ]])
        }
    }

    /// Disables a buffered vertex input so the current immediate value is used.
    pub fn disable_vertex_attribute(&mut self, slot: u8) -> Result<(), Error> {
        self.set_vertex_attribute_format(slot, VertexType::F32, 0, 0, 0)
    }

    /// Sets the immediate four-component value of a vertex attribute.
    /// Disable its buffered array before using this value in a draw.
    pub fn set_vertex_attribute_constant(
        &mut self,
        slot: u8,
        value: [f32; 4],
    ) -> Result<(), Error> {
        self.set_immediate_vertex(slot, ImmediateVertex::Float4(value))
    }

    /// Queues an immediate attribute value. Disable buffered input for
    /// the slot to use the immediate value across a buffered draw.
    pub fn set_immediate_vertex(&mut self, slot: u8, value: ImmediateVertex) -> Result<(), Error> {
        use nv4097::*;

        if slot >= 16 {
            return Err(Error::InvalidArgument);
        }
        let (base, stride, words, count) = match value {
            ImmediateVertex::Float1(x) => (SET_VERTEX_DATA1F_M, 4, [x.to_bits(), 0, 0, 0], 1),
            ImmediateVertex::Float2([x, y]) => {
                (SET_VERTEX_DATA2F_M, 8, [x.to_bits(), y.to_bits(), 0, 0], 2)
            }
            ImmediateVertex::Float3([x, y, z]) => (
                SET_VERTEX_DATA3F_M,
                16,
                [x.to_bits(), y.to_bits(), z.to_bits(), 0],
                3,
            ),
            ImmediateVertex::Float4(value) => (SET_VERTEX_DATA4F_M, 16, value.map(f32::to_bits), 4),
            ImmediateVertex::Short2([x, y]) => {
                (SET_VERTEX_DATA2S_M, 4, [pack_shorts(x, y), 0, 0, 0], 1)
            }
            ImmediateVertex::Short4([x, y, z, w]) => (
                SET_VERTEX_DATA4S_M,
                8,
                [pack_shorts(x, y), pack_shorts(z, w), 0, 0],
                2,
            ),
            ImmediateVertex::ScaledShort4([x, y, z, w]) => (
                SET_VERTEX_DATA_SCALED4S_M,
                8,
                [pack_shorts(x, y), pack_shorts(z, w), 0, 0],
                2,
            ),
            ImmediateVertex::NormalizedByte4(value) => (
                SET_VERTEX_DATA4UB_M,
                4,
                [u32::from_le_bytes(value), 0, 0, 0],
                1,
            ),
        };
        let method = base.address() + u16::from(slot) * stride;
        unsafe { self.write_words(&[&[packet(method, count)], &words[..count]]) }
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

    /// Draws using 16-bit indices embedded in the command stream.
    ///
    /// # Safety
    /// Every index must refer to live bound vertex data. A failed refill
    /// can leave an incomplete primitive in the stream.
    pub unsafe fn draw_inline_indices_u16(
        &mut self,
        primitive: Primitive,
        mut indices: &[u16],
    ) -> Result<(), Error> {
        if indices.is_empty() {
            return Ok(());
        }
        if indices.len() > MAX_INLINE_INDEX_COUNT {
            return Err(Error::InvalidArgument);
        }
        unsafe {
            self.begin_draw(primitive)?;
            if indices.len() % 2 != 0 {
                self.write_method(nv4097::ARRAY_ELEMENT32, false, &[u32::from(indices[0])])?;
                indices = &indices[1..];
            }
            let mut packed = [0; 256];
            for block in indices.chunks(512) {
                for (word, pair) in packed.iter_mut().zip(block.chunks_exact(2)) {
                    *word = u32::from(pair[0]) | (u32::from(pair[1]) << 16);
                }
                self.write_method(nv4097::ARRAY_ELEMENT16, false, &packed[..block.len() / 2])?;
            }
            self.write_words(&[&[packet(BEGIN_END, 1), 0]])
        }
    }

    /// Reserves one SDK-style inline array payload in the FIFO and lets the
    /// caller fill its words in place. The draw packet, payload and end marker
    /// are committed together, so an error from `fill` cannot leave an open
    /// primitive in the command stream.
    ///
    /// `word_count` is the number of 32-bit FIFO words (not vertices). It is
    /// limited to one RSX method packet, matching GCM's
    /// `SetDrawInlineArrayPointer` helper.
    ///
    /// # Safety
    /// The words written by `fill` must match the active vertex formats and
    /// encode a whole number of vertices.
    pub unsafe fn draw_inline_with(
        &mut self,
        primitive: Primitive,
        word_count: usize,
        fill: impl FnOnce(&mut [u32]) -> Result<(), Error>,
    ) -> Result<(), Error> {
        unsafe { self.draw_inline_payload_with(primitive, nv4097::INLINE_ARRAY, word_count, fill) }
    }

    /// Reserves one SDK-style 32-bit inline index payload in the FIFO and
    /// lets the caller fill it in place. The draw and its end marker commit
    /// together only after `fill` succeeds.
    ///
    /// # Safety
    /// Every index written by `fill` must refer to live bound vertex data.
    pub unsafe fn draw_inline_indices_with(
        &mut self,
        primitive: Primitive,
        index_count: usize,
        fill: impl FnOnce(&mut [u32]) -> Result<(), Error>,
    ) -> Result<(), Error> {
        unsafe {
            self.draw_inline_payload_with(primitive, nv4097::ARRAY_ELEMENT32, index_count, fill)
        }
    }

    /// Reserves one SDK-style 16-bit inline index payload in the FIFO. Each
    /// payload word holds two indices, with the first index in its low half.
    /// The closure therefore fills packed 32-bit words, matching the FIFO
    /// representation used by `SetDrawInlineIndexArray16Pointer`.
    ///
    /// # Safety
    /// Every index encoded by `fill` must refer to live bound vertex data.
    pub unsafe fn draw_inline_indices_u16_words_with(
        &mut self,
        primitive: Primitive,
        packed_word_count: usize,
        fill: impl FnOnce(&mut [u32]) -> Result<(), Error>,
    ) -> Result<(), Error> {
        unsafe {
            self.draw_inline_payload_with(
                primitive,
                nv4097::ARRAY_ELEMENT16,
                packed_word_count,
                fill,
            )
        }
    }

    /// Reserves a 16-byte-aligned inline array payload, matching GCM's
    /// `SetDrawInlineArrayAlignedPointer` count and alignment rules.
    ///
    /// `word_count` counts 32-bit words and must be a multiple of four. The
    /// payload is limited to one RSX method packet.
    ///
    /// # Safety
    /// The words written by `fill` must match the active vertex formats and
    /// encode a whole number of vertices.
    pub unsafe fn draw_inline_aligned_with(
        &mut self,
        primitive: Primitive,
        word_count: usize,
        fill: impl FnOnce(&mut [u32]) -> Result<(), Error>,
    ) -> Result<(), Error> {
        unsafe {
            self.draw_inline_aligned_payload_with(primitive, nv4097::INLINE_ARRAY, word_count, fill)
        }
    }

    /// Reserves a 16-byte-aligned 32-bit inline index payload, matching
    /// GCM's `SetDrawInlineIndexArray32AlignedPointer` restrictions.
    /// `index_count` must be a multiple of four and fit one RSX method packet.
    ///
    /// # Safety
    /// Every index written by `fill` must refer to live bound vertex data.
    pub unsafe fn draw_inline_indices_aligned_with(
        &mut self,
        primitive: Primitive,
        index_count: usize,
        fill: impl FnOnce(&mut [u32]) -> Result<(), Error>,
    ) -> Result<(), Error> {
        unsafe {
            self.draw_inline_aligned_payload_with(
                primitive,
                nv4097::ARRAY_ELEMENT32,
                index_count,
                fill,
            )
        }
    }

    /// Reserves a 16-byte-aligned packed 16-bit index payload, matching
    /// GCM's `SetDrawInlineIndexArray16AlignedPointer` restrictions. The
    /// count is in 32-bit payload words, each holding two indices with the
    /// first index in its low half; it must be a multiple of four.
    ///
    /// # Safety
    /// Every index encoded by `fill` must refer to live bound vertex data.
    pub unsafe fn draw_inline_indices_u16_words_aligned_with(
        &mut self,
        primitive: Primitive,
        packed_word_count: usize,
        fill: impl FnOnce(&mut [u32]) -> Result<(), Error>,
    ) -> Result<(), Error> {
        unsafe {
            self.draw_inline_aligned_payload_with(
                primitive,
                nv4097::ARRAY_ELEMENT16,
                packed_word_count,
                fill,
            )
        }
    }

    unsafe fn draw_inline_payload_with(
        &mut self,
        primitive: Primitive,
        method: Method,
        payload_words: usize,
        fill: impl FnOnce(&mut [u32]) -> Result<(), Error>,
    ) -> Result<(), Error> {
        if payload_words == 0 {
            return Ok(());
        }
        if payload_words > MAX_METHOD_COUNT {
            return Err(Error::InvalidArgument);
        }

        // Six words begin the draw (three invalidates plus begin/end), then
        // one method header and the caller's data, followed by the end marker.
        let total_words = 9 + payload_words;
        unsafe {
            self.write_words_with_payload(total_words, |stream| {
                fill(&mut stream[7..7 + payload_words])?;
                stream[0] = Method::new(INVALIDATE_VERTEX_CACHE)
                    .unwrap()
                    .header(3, false);
                stream[4] = packet(BEGIN_END, 1);
                stream[5] = primitive as u32;
                stream[6] = method.header(payload_words, false);
                stream[7 + payload_words] = packet(BEGIN_END, 1);
                stream[8 + payload_words] = 0;
                Ok(())
            })
        }
    }

    unsafe fn draw_inline_aligned_payload_with(
        &mut self,
        primitive: Primitive,
        method: Method,
        payload_words: usize,
        fill: impl FnOnce(&mut [u32]) -> Result<(), Error>,
    ) -> Result<(), Error> {
        if payload_words == 0 {
            return Ok(());
        }
        if payload_words > MAX_METHOD_COUNT || payload_words % 4 != 0 {
            return Err(Error::InvalidArgument);
        }

        // GCM's aligned pointer helper requires the payload count to be a
        // multiple of four and emits NOP padding after begin and before the
        // method header so the data payload begins at a 16-byte boundary.
        unsafe {
            self.write_aligned_words_with_payload(
                6,
                1,
                payload_words,
                2,
                |leading, prefix, payload, suffix| {
                    fill(payload)?;
                    leading[0] = Method::new(INVALIDATE_VERTEX_CACHE)
                        .unwrap()
                        .header(3, false);
                    leading[4] = packet(BEGIN_END, 1);
                    leading[5] = primitive as u32;
                    prefix[0] = method.header(payload_words, false);
                    suffix[0] = packet(BEGIN_END, 1);
                    suffix[1] = 0;
                    Ok(())
                },
            )
        }
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

fn vertex_format(format: VertexType, components: u8, stride: u8, frequency: u16) -> u32 {
    u32::from(frequency) << 16
        | u32::from(stride) << 8
        | u32::from(components) << 4
        | u32::from(format as u8)
}
