use super::commands::packet;
use super::{Context, Error, Location, Texture};

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WrapMode {
    Repeat = 1,
    MirroredRepeat = 2,
    ClampToEdge = 3,
    ClampToBorder = 4,
    Clamp = 5,
    MirrorOnceClampToEdge = 6,
    MirrorOnceClampToBorder = 7,
    MirrorOnceClamp = 8,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MinFilter {
    Nearest = 1,
    Linear = 2,
    NearestMipmapNearest = 3,
    LinearMipmapNearest = 4,
    NearestMipmapLinear = 5,
    LinearMipmapLinear = 6,
    Convolution = 7,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MagFilter {
    Nearest = 1,
    Linear = 2,
    Convolution = 4,
}

#[derive(Clone, Copy, Debug)]
pub struct Sampler {
    pub wrap: [WrapMode; 3],
    pub min_filter: MinFilter,
    pub mag_filter: MagFilter,
    /// Signed 5.8 fixed-point LOD bias.
    pub lod_bias: i16,
    /// Minimum and maximum LOD as unsigned 4.8 fixed-point values.
    pub lod: [u16; 2],
    /// One of the eight GCM anisotropy levels, starting at 1×.
    pub anisotropy: u8,
    /// GCM convolution kernel index.
    pub convolution: u8,
    /// GCM texture comparison function.
    pub compare: u8,
    /// Four sRGB conversion bits.
    pub gamma: u8,
    pub unsigned_remap: bool,
    pub border_color: u32,
}

impl Default for Sampler {
    fn default() -> Self {
        Self {
            wrap: [WrapMode::ClampToEdge; 3],
            min_filter: MinFilter::Linear,
            mag_filter: MagFilter::Linear,
            lod_bias: 0,
            lod: [0; 2],
            anisotropy: 0,
            convolution: 1,
            compare: 0,
            gamma: 0,
            unsigned_remap: false,
            border_color: 0,
        }
    }
}

impl Texture {
    /// Describes one level of linear A8R8G8B8 data with normalized coordinates.
    pub fn linear_argb8(
        location: Location,
        offset: u32,
        width: u16,
        height: u16,
        pitch: u32,
    ) -> Result<Self, Error> {
        if width == 0 || height == 0 || pitch < u32::from(width) * 4 || pitch > 0x000f_ffff {
            return Err(Error::InvalidArgument);
        }
        Ok(Self {
            format: 0x85 | 0x20,
            mipmaps: 1,
            dimension: 2,
            cubemap: 0,
            remap: 0x0000_aae4, // Each channel uses its corresponding input component.
            width,
            height,
            depth: 1,
            location: location as u8,
            reserved: 0,
            pitch,
            offset,
        })
    }

    fn validate(&self) -> Result<(), Error> {
        if self.location > 1
            || self.cubemap > 1
            || !(1..=3).contains(&self.dimension)
            || !(1..=13).contains(&self.mipmaps)
            || self.width == 0
            || self.height == 0
            || self.depth == 0
            || self.depth > 0x0fff
            || self.pitch > 0x000f_ffff
        {
            return Err(Error::InvalidArgument);
        }
        Ok(())
    }

    fn format_word(&self) -> u32 {
        (u32::from(self.location) + 1)
            | (u32::from(self.dimension) << 4)
            | (u32::from(self.format) << 8)
            | (u32::from(self.mipmaps) << 16)
    }

    fn image_rect(&self) -> u32 {
        (u32::from(self.width) << 16) | u32::from(self.height)
    }
}

impl Context {
    /// Binds texture memory and sampling state to a fragment texture slot.
    ///
    /// # Safety
    ///
    /// The texture must describe allocated and mapped memory for its full
    /// pitch, dimensions, format and mip chain. Keep it alive until the
    /// GPU finishes all commands that use it.
    pub unsafe fn bind_texture(
        &mut self,
        slot: u8,
        texture: &Texture,
        sampler: &Sampler,
    ) -> Result<(), Error> {
        texture.validate()?;
        let [min_lod, max_lod] = sampler.lod;
        if slot >= 16
            || min_lod > max_lod
            || max_lod > 0xfff
            || !(-4096..=4095).contains(&sampler.lod_bias)
            || sampler.anisotropy > 7
            || sampler.convolution > 3
            || sampler.compare > 7
            || sampler.gamma > 15
        {
            return Err(Error::InvalidArgument);
        }
        let [s, t, r] = sampler.wrap.map(|coordinate| coordinate as u32);
        let wrap = s
            | (t << 8)
            | (r << 16)
            | (u32::from(sampler.unsigned_remap) << 12)
            | (u32::from(sampler.gamma) << 20)
            | (u32::from(sampler.compare) << 28);
        let control = 1 << 31
            | (u32::from(min_lod) << 19)
            | (u32::from(max_lod) << 7)
            | (u32::from(sampler.anisotropy) << 4);
        let filter = ((sampler.mag_filter as u32) << 24)
            | ((sampler.min_filter as u32) << 16)
            | (u32::from(sampler.convolution) << 13)
            | (u32::from(sampler.lod_bias as u16) & 0x1fff);
        let base = 0x1a00 + u16::from(slot) * 32;
        unsafe {
            self.write_words(&[&[
                packet(base, 8),
                texture.offset,
                texture.format_word() | (u32::from(texture.cubemap) << 2) | 0x8008,
                wrap,
                control,
                texture.remap,
                filter,
                texture.image_rect(),
                sampler.border_color,
                packet(0x1840 + u16::from(slot) * 4, 1),
                (u32::from(texture.depth) << 20) | texture.pitch,
            ]])
        }
    }

    /// Binds a vertex texture. The RSX vertex path uses a restricted set of
    /// texture formats and supports only the S and T wrap modes.
    ///
    /// # Safety
    ///
    /// The shader and texture must agree on format and dimensions. Keep the
    /// allocation live for every draw that samples it.
    pub unsafe fn bind_vertex_texture(
        &mut self,
        slot: u8,
        texture: &Texture,
        sampler: &Sampler,
    ) -> Result<(), Error> {
        texture.validate()?;
        let [min_lod, max_lod] = sampler.lod;
        if slot >= 4
            || texture.cubemap != 0
            || min_lod > max_lod
            || max_lod > 0xfff
            || !(-4096..=4095).contains(&sampler.lod_bias)
        {
            return Err(Error::InvalidArgument);
        }
        let wrap = sampler.wrap[0] as u32 | ((sampler.wrap[1] as u32) << 8);
        let control = (1 << 31) | (u32::from(min_lod) << 19) | (u32::from(max_lod) << 7);
        unsafe {
            self.write_words(&[&[
                packet(0x0900 + u16::from(slot) * 32, 8),
                texture.offset,
                texture.format_word(),
                wrap,
                control,
                texture.pitch,
                u32::from(sampler.lod_bias as u16) & 0x1fff,
                texture.image_rect(),
                sampler.border_color,
            ]])
        }
    }

    pub fn disable_texture(&mut self, slot: u8) -> Result<(), Error> {
        if slot >= 16 {
            return Err(Error::InvalidArgument);
        }
        unsafe { self.write_words(&[&[packet(0x1a0c + u16::from(slot) * 32, 1), 0]]) }
    }

    pub fn disable_vertex_texture(&mut self, slot: u8) -> Result<(), Error> {
        if slot >= 4 {
            return Err(Error::InvalidArgument);
        }
        unsafe { self.write_words(&[&[packet(0x090c + u16::from(slot) * 32, 1), 0]]) }
    }
}
