use super::commands::packet;
use super::methods::nv4097;
use super::{Context, Error};

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FogMode {
    Linear = 0x2601,
    Exponential = 0x0800,
    ExponentialSquared = 0x0801,
    ExponentialAbsolute = 0x0802,
    ExponentialSquaredAbsolute = 0x0803,
    LinearAbsolute = 0x0804,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PointSpriteR {
    Zero = 0,
    FromR = 1,
    FromS = 2,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DepthEncoding {
    Fixed = 0,
    Float = 1,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Compare {
    Never = 0x0200,
    Less = 0x0201,
    Equal = 0x0202,
    LessEqual = 0x0203,
    Greater = 0x0204,
    NotEqual = 0x0205,
    GreaterEqual = 0x0206,
    Always = 0x0207,
}

#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlendFactor {
    Zero = 0,
    One = 1,
    SourceColor = 0x0300,
    OneMinusSourceColor = 0x0301,
    SourceAlpha = 0x0302,
    OneMinusSourceAlpha = 0x0303,
    DestinationAlpha = 0x0304,
    OneMinusDestinationAlpha = 0x0305,
    DestinationColor = 0x0306,
    OneMinusDestinationColor = 0x0307,
    SourceAlphaSaturate = 0x0308,
    ConstantColor = 0x8001,
    OneMinusConstantColor = 0x8002,
    ConstantAlpha = 0x8003,
    OneMinusConstantAlpha = 0x8004,
}

#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlendEquation {
    Add = 0x8006,
    Minimum = 0x8007,
    Maximum = 0x8008,
    Subtract = 0x800a,
    ReverseSubtract = 0x800b,
}

#[derive(Clone, Copy, Debug)]
pub struct BlendChannel {
    pub source: BlendFactor,
    pub destination: BlendFactor,
    pub equation: BlendEquation,
}

#[derive(Clone, Copy, Debug)]
pub struct Blend {
    pub color: BlendChannel,
    pub alpha: BlendChannel,
    /// Primary packed blend color.
    pub constant: u32,
    /// Additional constant components for wider color formats.
    pub constant_ext: u32,
}

impl Blend {
    pub const ALPHA: Self = Self {
        color: BlendChannel {
            source: BlendFactor::SourceAlpha,
            destination: BlendFactor::OneMinusSourceAlpha,
            equation: BlendEquation::Add,
        },
        alpha: BlendChannel {
            source: BlendFactor::One,
            destination: BlendFactor::OneMinusSourceAlpha,
            equation: BlendEquation::Add,
        },
        constant: 0,
        constant_ext: 0,
    };
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StencilAction {
    Zero = 0,
    Keep = 0x1e00,
    Replace = 0x1e01,
    Increment = 0x1e02,
    Decrement = 0x1e03,
    Invert = 0x150a,
    IncrementWrap = 0x8507,
    DecrementWrap = 0x8508,
}

#[derive(Clone, Copy, Debug)]
pub struct StencilFace {
    pub comparison: Compare,
    pub reference: u8,
    pub compare_mask: u8,
    pub write_mask: u8,
    pub fail: StencilAction,
    pub depth_fail: StencilAction,
    pub pass: StencilAction,
}

impl Default for StencilFace {
    fn default() -> Self {
        Self {
            comparison: Compare::Always,
            reference: 0,
            compare_mask: 0xff,
            write_mask: 0xff,
            fail: StencilAction::Keep,
            depth_fail: StencilAction::Keep,
            pass: StencilAction::Keep,
        }
    }
}

impl StencilFace {
    fn words(self) -> [u32; 8] {
        [
            1,
            self.write_mask.into(),
            self.comparison as u32,
            self.reference.into(),
            self.compare_mask.into(),
            self.fail as u32,
            self.depth_fail as u32,
            self.pass as u32,
        ]
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CullFace {
    Front = 0x0404,
    Back = 0x0405,
    FrontAndBack = 0x0408,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FaceWinding {
    Clockwise = 0x0900,
    CounterClockwise = 0x0901,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PolygonMode {
    Point = 0x1b00,
    Line = 0x1b01,
    Fill = 0x1b02,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShadeModel {
    Flat = 0x1d00,
    Smooth = 0x1d01,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClipPlane {
    Disabled = 0,
    LessThanZero = 1,
    GreaterThanOrEqualToZero = 2,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogicOp {
    Clear = 0x1500,
    And = 0x1501,
    AndReverse = 0x1502,
    Copy = 0x1503,
    AndInverted = 0x1504,
    Noop = 0x1505,
    Xor = 0x1506,
    Or = 0x1507,
    Nor = 0x1508,
    Equiv = 0x1509,
    Invert = 0x150a,
    OrReverse = 0x150b,
    CopyInverted = 0x150c,
    OrInverted = 0x150d,
    Nand = 0x150e,
    Set = 0x150f,
}

const fn pack_blend_pair(color: u16, alpha: u16) -> u32 {
    color as u32 | (alpha as u32) << 16
}

impl Context {
    /// Sets the color and alpha source/destination blend factors independently.
    ///
    /// This updates only the two factor registers, preserving the blend equation,
    /// constants, and enable state. Arguments follow the SDK's `SetBlendFunc`
    /// order: source color, destination color, source alpha, destination alpha.
    /// The packet stores color in the low halfword and alpha in the high halfword
    /// of each factor.
    pub fn set_blend_functions(
        &mut self,
        source_color: BlendFactor,
        destination_color: BlendFactor,
        source_alpha: BlendFactor,
        destination_alpha: BlendFactor,
    ) -> Result<(), Error> {
        unsafe {
            self.write_words(&[&[
                nv4097::SET_BLEND_FUNC_SFACTOR.header(2, true),
                pack_blend_pair(source_color as u16, source_alpha as u16),
                pack_blend_pair(destination_color as u16, destination_alpha as u16),
            ]])
        }
    }

    /// Sets the color and alpha blend equations without changing their factors.
    pub fn set_blend_equations(
        &mut self,
        color: BlendEquation,
        alpha: BlendEquation,
    ) -> Result<(), Error> {
        unsafe {
            self.write_words(&[&[
                nv4097::SET_BLEND_EQUATION.header(1, true),
                pack_blend_pair(color as u16, alpha as u16),
            ]])
        }
    }

    /// Sets both SDK blend constants. The second constant is used by wider
    /// color formats; both values are passed through as packed color words.
    pub fn set_blend_colors(&mut self, primary: u32, extended: u32) -> Result<(), Error> {
        unsafe {
            self.write_words(&[&[
                nv4097::SET_BLEND_COLOR.header(1, true),
                primary,
                nv4097::SET_BLEND_COLOR2.header(1, true),
                extended,
            ]])
        }
    }

    /// Sets multisample antialiasing and packs the four SDK control fields.
    ///
    /// The 16-bit sample mask occupies bits 16–31; the three flags occupy bits 0, 4,
    /// and 8 respectively, matching `SetAntiAliasingControl` in the SDK.
    pub fn set_antialiasing(
        &mut self,
        enabled: bool,
        alpha_to_coverage: bool,
        alpha_to_one: bool,
        sample_mask: u16,
    ) -> Result<(), Error> {
        let control = u32::from(enabled)
            | (u32::from(alpha_to_coverage) << 4)
            | (u32::from(alpha_to_one) << 8)
            | (u32::from(sample_mask) << 16);
        unsafe { self.write_method(nv4097::SET_ANTI_ALIASING_CONTROL, true, &[control]) }
    }

    /// Enables or disables two-sided lighting without changing other lighting state.
    pub fn set_two_sided_lighting_enabled(&mut self, enabled: bool) -> Result<(), Error> {
        unsafe {
            self.write_method(
                nv4097::SET_TWO_SIDE_LIGHT_EN,
                true,
                &[u32::from(enabled)],
            )
        }
    }

    /// Sets the width in eighths of a pixel and selects line smoothing.
    pub fn set_line_rasterization(
        &mut self,
        width_eighths: u16,
        smooth: bool,
    ) -> Result<(), Error> {
        if width_eighths > 0x1ff {
            return Err(Error::InvalidArgument);
        }
        unsafe {
            self.write_method(
                nv4097::SET_LINE_WIDTH,
                true,
                &[width_eighths.into(), smooth.into()],
            )
        }
    }

    /// Selects a 16-bit line pattern and its hardware repeat factor.
    pub fn set_line_stipple(&mut self, pattern: Option<(u16, u8)>) -> Result<(), Error> {
        unsafe {
            if let Some((bits, factor)) = pattern {
                let value = u32::from(bits) << 16 | u32::from(factor);
                self.write_method(nv4097::SET_LINE_STIPPLE, true, &[1, value])
            } else {
                self.write_method(nv4097::SET_LINE_STIPPLE, true, &[0])
            }
        }
    }

    /// Selects a 32×32 polygon stipple pattern, one word per row.
    pub fn set_polygon_stipple(&mut self, pattern: Option<&[u32; 32]>) -> Result<(), Error> {
        unsafe {
            if let Some(pattern) = pattern {
                self.write_words(&[&[nv4097::SET_POLYGON_STIPPLE.header(33, true), 1], pattern])
            } else {
                self.write_method(nv4097::SET_POLYGON_STIPPLE, true, &[0])
            }
        }
    }

    pub fn set_polygon_smooth(&mut self, enabled: bool) -> Result<(), Error> {
        unsafe { self.write_method(nv4097::SET_POLY_SMOOTH_ENABLE, true, &[enabled.into()]) }
    }

    pub fn set_point_size(&mut self, pixels: f32) -> Result<(), Error> {
        unsafe { self.write_method(nv4097::SET_POINT_SIZE, true, &[pixels.to_bits()]) }
    }

    /// Selects point sprites. The coordinate mask uses bits 0–9 for TEX0–TEX9.
    pub fn set_point_sprite(&mut self, sprite: Option<(PointSpriteR, u16)>) -> Result<(), Error> {
        let (enabled, control) = match sprite {
            Some((r, mask)) if mask <= 0x03ff => (1, 1 | (r as u32) << 1 | u32::from(mask) << 8),
            Some(_) => return Err(Error::InvalidArgument),
            None => (0, 0),
        };
        unsafe { self.write_method(nv4097::SET_POINT_PARAMS_ENABLE, true, &[enabled, control]) }
    }

    /// Sets the fog equation and its two shader parameters.
    pub fn set_fog(&mut self, mode: FogMode, parameters: [f32; 2]) -> Result<(), Error> {
        unsafe {
            self.write_method(
                nv4097::SET_FOG_MODE,
                true,
                &[
                    mode as u32,
                    parameters[0].to_bits(),
                    parameters[1].to_bits(),
                ],
            )
        }
    }

    /// Selects the numeric representation of the active depth surface.
    pub fn set_depth_encoding(&mut self, encoding: DepthEncoding) -> Result<(), Error> {
        unsafe {
            self.write_method(
                nv4097::SET_CONTROL0,
                true,
                &[0x0010_0000 | (encoding as u32) << 12],
            )
        }
    }

    pub fn set_depth_clip(
        &mut self,
        cull_near_far: bool,
        clamp: bool,
        ignore_w: bool,
    ) -> Result<(), Error> {
        let value = u32::from(cull_near_far) | u32::from(clamp) << 4 | u32::from(ignore_w) << 8;
        unsafe { self.write_method(nv4097::SET_ZMIN_MAX_CONTROL, true, &[value]) }
    }

    pub fn set_blend_optimization(&mut self, enabled: bool) -> Result<(), Error> {
        unsafe { self.write_method(nv4097::SET_REDUCE_DST_COLOR, true, &[enabled.into()]) }
    }

    pub fn set_depth_compare(&mut self, compare: Compare) -> Result<(), Error> {
        unsafe { self.write_words(&[&[packet(0x0a6c, 1), compare as u32]]) }
    }

    pub fn set_depth_bounds(&mut self, bounds: Option<[f32; 2]>) -> Result<(), Error> {
        match bounds {
            Some([min, max]) if min >= 0.0 && min <= max && max <= 1.0 => unsafe {
                self.write_words(&[&[packet(0x0380, 3), 1, min.to_bits(), max.to_bits()]])
            },
            Some(_) => Err(Error::InvalidArgument),
            None => unsafe { self.write_words(&[&[packet(0x0380, 1), 0]]) },
        }
    }

    pub fn set_alpha_test(&mut self, compare: Option<(Compare, u8)>) -> Result<(), Error> {
        match compare {
            Some((compare, reference)) => unsafe {
                self.write_words(&[&[packet(0x0304, 3), 1, compare as u32, reference.into()]])
            },
            None => unsafe { self.write_words(&[&[packet(0x0304, 1), 0]]) },
        }
    }

    pub fn set_blend(&mut self, blend: Option<Blend>) -> Result<(), Error> {
        let Some(blend) = blend else {
            return self.set_blend_enabled(false);
        };
        unsafe {
            self.write_words(&[&[
                packet(0x0310, 5),
                1,
                pack_blend_pair(blend.color.source as u16, blend.alpha.source as u16),
                pack_blend_pair(
                    blend.color.destination as u16,
                    blend.alpha.destination as u16,
                ),
                blend.constant,
                pack_blend_pair(blend.color.equation as u16, blend.alpha.equation as u16),
                packet(0x037c, 1),
                blend.constant_ext,
            ]])
        }
    }

    /// Configures front-facing stencil state and optionally distinct back-face state.
    pub fn set_stencil(
        &mut self,
        front: Option<StencilFace>,
        back: Option<StencilFace>,
    ) -> Result<(), Error> {
        unsafe {
            match (front, back) {
                (Some(front), Some(back)) => self.write_words(&[
                    &[packet(0x0328, 8)],
                    &front.words(),
                    &[packet(0x0348, 8)],
                    &back.words(),
                ]),
                (Some(front), None) => self.write_words(&[
                    &[packet(0x0328, 8)],
                    &front.words(),
                    &[packet(0x0348, 1), 0],
                ]),
                (None, None) => self.write_words(&[&[packet(0x0328, 1), 0, packet(0x0348, 1), 0]]),
                (None, Some(_)) => Err(Error::InvalidArgument),
            }
        }
    }

    pub fn set_culling(
        &mut self,
        face: Option<CullFace>,
        winding: FaceWinding,
    ) -> Result<(), Error> {
        let mode = face.unwrap_or(CullFace::Back);
        unsafe {
            self.write_words(&[&[
                packet(0x1830, 2),
                mode as u32,
                winding as u32,
                packet(0x183c, 1),
                u32::from(face.is_some()),
            ]])
        }
    }

    pub fn set_polygon_mode(&mut self, front: PolygonMode, back: PolygonMode) -> Result<(), Error> {
        unsafe { self.write_words(&[&[packet(0x1828, 2), front as u32, back as u32]]) }
    }

    pub fn set_shading(&mut self, mode: ShadeModel) -> Result<(), Error> {
        unsafe { self.write_words(&[&[packet(0x0368, 1), mode as u32]]) }
    }

    pub fn set_dither_enabled(&mut self, enable: bool) -> Result<(), Error> {
        unsafe { self.write_words(&[&[packet(0x0300, 1), enable.into()]]) }
    }

    pub fn set_logic_op(&mut self, operation: Option<LogicOp>) -> Result<(), Error> {
        let words = match operation {
            Some(op) => [packet(0x0374, 2), 1, op as u32],
            None => [packet(0x0374, 2), 0, LogicOp::Copy as u32],
        };
        unsafe { self.write_words(&[&words]) }
    }

    /// Sets blending and RGBA write masks for the three additional color targets.
    pub fn set_multiple_targets(
        &mut self,
        blend: [bool; 3],
        masks: [[bool; 4]; 3],
    ) -> Result<(), Error> {
        let mut blend_word = 0;
        let mut color_mask = 0;
        for (i, (blend, mask)) in blend.into_iter().zip(masks).enumerate() {
            let [r, g, b, a] = mask.map(u32::from);
            blend_word |= u32::from(blend) << (i + 1);
            color_mask |= (a | (r << 1) | (g << 2) | (b << 3)) << ((i + 1) * 4);
        }
        unsafe { self.write_words(&[&[packet(0x036c, 2), blend_word, color_mask]]) }
    }

    /// Sets polygon offset for points, lines, and triangles in that order.
    pub fn set_polygon_offset(
        &mut self,
        enable: [bool; 3],
        factor: f32,
        bias: f32,
    ) -> Result<(), Error> {
        let [point, line, fill] = enable.map(u32::from);
        unsafe {
            self.write_words(&[&[
                packet(0x0a60, 3),
                point,
                line,
                fill,
                packet(0x0a78, 2),
                factor.to_bits(),
                bias.to_bits(),
            ]])
        }
    }

    /// Enables the restart index for indexed strip and fan draws.
    pub fn set_primitive_restart(&mut self, index: Option<u32>) -> Result<(), Error> {
        unsafe {
            self.write_words(&[&[
                packet(0x1dac, 2),
                u32::from(index.is_some()),
                index.unwrap_or(0),
            ]])
        }
    }

    /// Sets the modulo operation mask for vertex attribute frequency division.
    /// A clear bit selects division for the corresponding attribute.
    pub fn set_vertex_frequency_modulo(&mut self, attributes: u16) -> Result<(), Error> {
        unsafe { self.write_words(&[&[packet(0x1fc0, 1), attributes.into()]]) }
    }

    /// Invalidates RSX texture caches after updates to texture data.
    pub fn invalidate_texture_cache(&mut self, vertex: bool, fragment: bool) -> Result<(), Error> {
        use super::methods::nv4097::INVALIDATE_L2;
        let mask = u32::from(fragment) | (u32::from(vertex) << 1);
        unsafe { self.write_method(INVALIDATE_L2, true, &[mask]) }
    }

    pub fn set_user_clip_planes(&mut self, planes: [ClipPlane; 6]) -> Result<(), Error> {
        use super::methods::nv4097::SET_USER_CLIP_PLANE_CONTROL;
        let word = planes
            .into_iter()
            .enumerate()
            .fold(0, |mask, (i, plane)| mask | ((plane as u32) << (i * 4)));
        unsafe { self.write_method(SET_USER_CLIP_PLANE_CONTROL, true, &[word]) }
    }
}
