use super::commands::packet;
use super::methods::nv4097;
use super::{Compare, Context, Error, Location, Texture};

/// The base encodings accepted by a Cell GCM texture descriptor.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextureFormat {
    B8 = 0x81,
    A1R5G5B5 = 0x82,
    A4R4G4B4 = 0x83,
    R5G6B5 = 0x84,
    A8R8G8B8 = 0x85,
    Dxt1 = 0x86,
    Dxt23 = 0x87,
    Dxt45 = 0x88,
    G8B8 = 0x8b,
    R6G5B5 = 0x8f,
    Depth24D8 = 0x90,
    Depth24D8Float = 0x91,
    Depth16 = 0x92,
    Depth16Float = 0x93,
    X16 = 0x94,
    Y16X16 = 0x95,
    R5G5B5A1 = 0x97,
    Hilo8 = 0x98,
    SignedHilo8 = 0x99,
    W16Z16Y16X16Float = 0x9a,
    W32Z32Y32X32Float = 0x9b,
    X32Float = 0x9c,
    D1R5G5B5 = 0x9d,
    D8R8G8B8 = 0x9e,
    Y16X16Float = 0x9f,
    PackedB8R8G8R8 = 0xad,
    PackedR8B8R8G8 = 0xae,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextureLayout {
    Swizzled = 0,
    Linear = 0x20,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextureCoordinates {
    Normalized = 0,
    Texels = 0x40,
}

impl TextureFormat {
    /// Encodes this format for the ABI-facing `Texture::format` field.
    pub const fn encode(self, layout: TextureLayout, coordinates: TextureCoordinates) -> u8 {
        self as u8 | layout as u8 | coordinates as u8
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextureChannel {
    Alpha = 0,
    Red = 1,
    Green = 2,
    Blue = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextureComponent {
    Zero,
    One,
    Channel(TextureChannel),
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextureRemapOrder {
    Xyxy = 0,
    Xxxy = 1,
}

/// Channel shape accepted by the RSX vertex texture fetch path for `f32` data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VertexTextureChannels {
    /// One 32-bit floating-point channel (`X32_FLOAT`).
    R,
    /// Four 32-bit floating-point channels (`W32_Z32_Y32_X32_FLOAT`).
    Rgba,
}

/// A GCM channel remap word that can be stored in `Texture::remap`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextureRemap(u32);

impl TextureRemap {
    pub const IDENTITY: Self = Self(0xaae4);

    /// Builds a remap from the A, R, G, and B output components.
    pub const fn new(output: [TextureComponent; 4], order: TextureRemapOrder) -> Self {
        let mut word = (order as u32) << 16;
        let mut index = 0;
        while index < output.len() {
            let (input, source) = match output[index] {
                TextureComponent::Zero => (0, 0),
                TextureComponent::One => (0, 1),
                TextureComponent::Channel(channel) => (channel as u32, 2),
            };
            word |= (input | source << 8) << (index * 2);
            index += 1;
        }
        Self(word)
    }

    pub const fn bits(self) -> u32 {
        self.0
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BorderSource {
    Texture = 0,
    #[default]
    Color = 1,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SignedRemap {
    #[default]
    Normal = 0,
    Clamped = 3,
}

/// Additional fragment sampling controls for a texture binding.
#[derive(Clone, Copy, Debug, Default)]
pub struct TextureOptions {
    pub border_source: BorderSource,
    pub alpha_kill: bool,
    /// Raw four-bit GCM anisotropic bias selection.
    pub anisotropic_bias: u8,
    pub signed_remap: SignedRemap,
    /// Interpret the texture's A, R, G, and B channels as signed.
    pub signed_channels: [bool; 4],
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnisotropicSpacing {
    Texels0_50 = 0,
    Texels1_00 = 1,
    Texels1_125 = 2,
    Texels1_25 = 3,
    Texels1_375 = 4,
    Texels1_50 = 5,
    Texels1_75 = 6,
    Texels2_00 = 7,
}

#[derive(Clone, Copy, Debug)]
pub struct AnisotropicSpread {
    pub spacing: AnisotropicSpacing,
    pub reduce_samples: bool,
}

impl AnisotropicSpread {
    fn bits(self) -> u32 {
        self.spacing as u32 | u32::from(self.reduce_samples) << 4
    }
}

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

impl Sampler {
    /// Sets the comparison used when the shader samples a depth texture.
    pub fn with_depth_compare(mut self, compare: Compare) -> Self {
        self.compare = (compare as u32 - Compare::Never as u32) as u8;
        self
    }
}

fn texture_address_word(sampler: &Sampler, options: TextureOptions) -> u32 {
    let [s, t, r] = sampler.wrap.map(|coordinate| coordinate as u32);
    s | (u32::from(options.anisotropic_bias) << 4)
        | (t << 8)
        | (r << 16)
        | (u32::from(sampler.unsigned_remap) << 12)
        | (u32::from(sampler.gamma) << 20)
        | ((options.signed_remap as u32) << 24)
        | (u32::from(sampler.compare) << 28)
}

fn texture_filter_word(sampler: &Sampler, signed_channels: [bool; 4]) -> u32 {
    let [a, red, green, blue] = signed_channels.map(u32::from);
    (a << 28)
        | (red << 29)
        | (green << 30)
        | (blue << 31)
        | ((sampler.mag_filter as u32) << 24)
        | ((sampler.min_filter as u32) << 16)
        | (u32::from(sampler.convolution) << 13)
        | (u32::from(sampler.lod_bias as u16) & 0x1fff)
}

impl Texture {
    fn linear_2d(
        location: Location,
        offset: u32,
        width: u16,
        height: u16,
        pitch: u32,
        format: TextureFormat,
        bytes_per_texel: u32,
    ) -> Result<Self, Error> {
        if width == 0
            || height == 0
            || pitch < u32::from(width) * bytes_per_texel
            || pitch > 0x000f_ffff
        {
            return Err(Error::InvalidArgument);
        }
        Ok(Self {
            format: format.encode(TextureLayout::Linear, TextureCoordinates::Normalized),
            mipmaps: 1,
            dimension: 2,
            cubemap: 0,
            remap: TextureRemap::IDENTITY.bits(),
            width,
            height,
            depth: 1,
            location: location as u8,
            reserved: 0,
            pitch,
            offset,
        })
    }

    /// Describes one level of linear A8R8G8B8 data with normalized coordinates.
    pub fn linear_argb8(
        location: Location,
        offset: u32,
        width: u16,
        height: u16,
        pitch: u32,
    ) -> Result<Self, Error> {
        Self::linear_2d(
            location,
            offset,
            width,
            height,
            pitch,
            TextureFormat::A8R8G8B8,
            4,
        )
    }

    /// Describes one level of linear R5G6B5 data with normalized coordinates.
    /// `pitch` is measured in bytes and must hold one row of 16-bit texels.
    pub fn linear_rgb565(
        location: Location,
        offset: u32,
        width: u16,
        height: u16,
        pitch: u32,
    ) -> Result<Self, Error> {
        Self::linear_2d(
            location,
            offset,
            width,
            height,
            pitch,
            TextureFormat::R5G6B5,
            2,
        )
    }

    /// Describes one level of linear A1R5G5B5 data with normalized coordinates.
    /// `pitch` is measured in bytes and must hold one row of 16-bit texels.
    pub fn linear_argb1555(
        location: Location,
        offset: u32,
        width: u16,
        height: u16,
        pitch: u32,
    ) -> Result<Self, Error> {
        Self::linear_2d(
            location,
            offset,
            width,
            height,
            pitch,
            TextureFormat::A1R5G5B5,
            2,
        )
    }

    /// Describes one level of linear, normalized-coordinate vertex `f32` data.
    ///
    /// Vertex textures support either one channel (`X32_FLOAT`) or four
    /// channels (`W32_Z32_Y32_X32_FLOAT`). The base offset must be 128-byte
    /// aligned, and `pitch` must hold at least one row and be a whole number
    /// of texels.
    pub fn linear_vertex_f32(
        location: Location,
        offset: u32,
        width: u16,
        height: u16,
        channels: VertexTextureChannels,
        pitch: u32,
    ) -> Result<Self, Error> {
        let (format, bytes_per_texel) = match channels {
            VertexTextureChannels::R => (TextureFormat::X32Float, 4),
            VertexTextureChannels::Rgba => (TextureFormat::W32Z32Y32X32Float, 16),
        };
        if width == 0
            || height == 0
            || offset % 128 != 0
            || pitch % bytes_per_texel != 0
            || pitch < u32::from(width) * bytes_per_texel
            || pitch > 0x000f_ffff
        {
            return Err(Error::InvalidArgument);
        }
        Ok(Self {
            format: format.encode(TextureLayout::Linear, TextureCoordinates::Normalized),
            mipmaps: 1,
            dimension: 2,
            cubemap: 0,
            remap: TextureRemap::IDENTITY.bits(),
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

fn texture_control_word(
    enabled: bool,
    min_lod: u16,
    max_lod: u16,
    anisotropy: u8,
    alpha_kill: bool,
) -> u32 {
    (u32::from(enabled) << 31)
        | (u32::from(min_lod) << 19)
        | (u32::from(max_lod) << 7)
        | (u32::from(anisotropy) << 4)
        | (u32::from(alpha_kill) << 2)
}

impl Context {
    /// Enables no-paranoid texture fetches for the fragment samplers selected
    /// by `sampler_mask` (bit *n* selects texture slot *n*).
    ///
    /// This is the standalone Cell GCM `SetNoParanoidTextureFetches` method.
    pub fn set_no_paranoid_texture_fetches(&mut self, sampler_mask: u16) -> Result<(), Error> {
        unsafe {
            self.write_words(&[&[
                packet(nv4097::SET_NO_PARANOID_TEXTURE_FETCHES.address(), 1),
                u32::from(sampler_mask),
            ]])
        }
    }

    /// Sets a fragment sampler's enable, LOD range, anisotropy and alpha-kill
    /// state without changing its other sampler methods.
    ///
    /// LOD values use unsigned 4.8 fixed point, matching [`Sampler::lod`].
    pub fn set_texture_control_alpha_kill(
        &mut self,
        slot: u8,
        enabled: bool,
        min_lod: u16,
        max_lod: u16,
        anisotropy: u8,
        alpha_kill: bool,
    ) -> Result<(), Error> {
        if slot >= 16 || min_lod > max_lod || max_lod > 0x0fff || anisotropy > 7 {
            return Err(Error::InvalidArgument);
        }
        let value = texture_control_word(enabled, min_lod, max_lod, anisotropy, alpha_kill);
        let address = nv4097::SET_TEXTURE_CONTROL0.address() + u16::from(slot) * 32;
        unsafe { self.write_words(&[&[packet(address, 1), value]]) }
    }

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
        unsafe { self.bind_texture_with_options(slot, texture, sampler, TextureOptions::default()) }
    }

    /// Binds a fragment texture with optional extended sampler controls.
    ///
    /// # Safety
    /// Observe the same allocation and lifetime requirements as
    /// [`Self::bind_texture`]. For an embedded border the backing texture
    /// must also contain the border texels required by the hardware format.
    pub unsafe fn bind_texture_with_options(
        &mut self,
        slot: u8,
        texture: &Texture,
        sampler: &Sampler,
        options: TextureOptions,
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
            || options.anisotropic_bias > 15
        {
            return Err(Error::InvalidArgument);
        }
        let wrap = texture_address_word(sampler, options);
        let control = texture_control_word(
            true,
            min_lod,
            max_lod,
            sampler.anisotropy,
            options.alpha_kill,
        );
        let filter = texture_filter_word(sampler, options.signed_channels);
        let base = 0x1a00 + u16::from(slot) * 32;
        unsafe {
            self.write_words(&[&[
                packet(base, 8),
                texture.offset,
                texture.format_word()
                    | (u32::from(texture.cubemap) << 2)
                    | (options.border_source as u32) << 3
                    | 0x8000,
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

    /// Sets a fragment sampler's filter, signed-channel flags, and LOD bias
    /// without changing its texture binding or other sampler methods.
    pub fn set_texture_filter_signed(
        &mut self,
        slot: u8,
        sampler: &Sampler,
        signed_channels: [bool; 4],
    ) -> Result<(), Error> {
        if slot >= 16
            || !(-4096..=4095).contains(&sampler.lod_bias)
            || sampler.convolution > 3
        {
            return Err(Error::InvalidArgument);
        }
        let value = texture_filter_word(sampler, signed_channels);
        let address = nv4097::SET_TEXTURE_FILTER.address() + u16::from(slot) * 32;
        unsafe { self.write_words(&[&[packet(address, 1), value]]) }
    }

    /// Sets a fragment sampler's address, unsigned and signed remaps,
    /// comparison, gamma, and anisotropic bias state independently.
    pub fn set_texture_address_aniso_bias_remap(
        &mut self,
        slot: u8,
        sampler: &Sampler,
        options: TextureOptions,
    ) -> Result<(), Error> {
        if slot >= 16
            || sampler.compare > 7
            || sampler.gamma > 15
            || options.anisotropic_bias > 15
        {
            return Err(Error::InvalidArgument);
        }
        let value = texture_address_word(sampler, options);
        let address = nv4097::SET_TEXTURE_ADDRESS.address() + u16::from(slot) * 32;
        unsafe { self.write_words(&[&[packet(address, 1), value]]) }
    }

    /// Configures the quality hints for isotropic and anisotropic sampling.
    pub fn set_texture_optimization(
        &mut self,
        slot: u8,
        slope: u8,
        isotropic_high: bool,
        anisotropic_high: bool,
    ) -> Result<(), Error> {
        if slot >= 16 || slope >= 32 {
            return Err(Error::InvalidArgument);
        }
        let value = 0x2d00
            | u32::from(slope)
            | (u32::from(isotropic_high) << 6)
            | (u32::from(anisotropic_high) << 7);
        let address = nv4097::SET_TEXTURE_CONTROL2.address() + u16::from(slot) * 4;
        unsafe { self.write_words(&[&[packet(address, 1), value]]) }
    }

    /// Sets the general, horizontal, and vertical anisotropic sample spacing.
    pub fn set_anisotropic_spread(
        &mut self,
        slot: u8,
        general: AnisotropicSpread,
        horizontal: AnisotropicSpread,
        vertical: AnisotropicSpread,
    ) -> Result<(), Error> {
        if slot >= 16 {
            return Err(Error::InvalidArgument);
        }
        let value = general.bits() | (horizontal.bits() << 8) | (vertical.bits() << 16);
        let address = nv4097::SET_ANISO_SPREAD.address() + u16::from(slot) * 4;
        unsafe { self.write_words(&[&[packet(address, 1), value]]) }
    }

    /// Sets the U, V, P, and Q cylindrical-wrap mask for texture slots 0–7.
    /// Each slot occupies four bits, starting with slot 0 in the low nibble.
    pub fn set_cylindrical_wrap(&mut self, mask: u32) -> Result<(), Error> {
        unsafe { self.write_method(nv4097::SET_CYLINDRICAL_WRAP, true, &[mask]) }
    }

    /// Updates the fragment sampler's constant A8R8G8B8 border color.
    pub fn set_texture_border_color(&mut self, slot: u8, argb: u32) -> Result<(), Error> {
        if slot >= 16 {
            return Err(Error::InvalidArgument);
        }
        let address = nv4097::SET_TEXTURE_BORDER_COLOR.address() + u16::from(slot) * 32;
        unsafe { self.write_words(&[&[packet(address, 1), argb]]) }
    }

    /// Updates the vertex sampler's constant A8R8G8B8 border color.
    pub fn set_vertex_texture_border_color(&mut self, slot: u8, argb: u32) -> Result<(), Error> {
        if slot >= 4 {
            return Err(Error::InvalidArgument);
        }
        let address = nv4097::SET_VERTEX_TEXTURE_BORDER_COLOR.address() + u16::from(slot) * 32;
        unsafe { self.write_words(&[&[packet(address, 1), argb]]) }
    }

    /// Updates the GCM channel remap word of the bound fragment texture.
    pub fn set_texture_remap(&mut self, slot: u8, remap: u32) -> Result<(), Error> {
        if slot >= 16 {
            return Err(Error::InvalidArgument);
        }
        let address = nv4097::SET_TEXTURE_CONTROL1.address() + u16::from(slot) * 32;
        unsafe { self.write_words(&[&[packet(address, 1), remap]]) }
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
        const RGBA32F: u8 = TextureFormat::W32Z32Y32X32Float
            .encode(TextureLayout::Linear, TextureCoordinates::Normalized);
        const R32F: u8 =
            TextureFormat::X32Float.encode(TextureLayout::Linear, TextureCoordinates::Normalized);
        let bytes_per_pixel = match texture.format {
            RGBA32F => 16,
            R32F => 4,
            _ => return Err(Error::InvalidArgument),
        };
        let [min_lod, max_lod] = sampler.lod;
        if slot >= 4
            || texture.cubemap != 0
            || texture.offset % 128 != 0
            || texture.pitch % bytes_per_pixel != 0
            || texture.pitch < u32::from(texture.width) * bytes_per_pixel
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
