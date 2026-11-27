use super::commands::packet;
use super::{Context, Error};

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

impl Context {
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
        let pack = |lo, hi| u32::from(lo) | (u32::from(hi) << 16);
        unsafe {
            self.write_words(&[&[
                packet(0x0310, 5),
                1,
                pack(blend.color.source as u16, blend.alpha.source as u16),
                pack(
                    blend.color.destination as u16,
                    blend.alpha.destination as u16,
                ),
                blend.constant,
                pack(blend.color.equation as u16, blend.alpha.equation as u16),
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
}
