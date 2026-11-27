//! RSX shader microcode and reflection for PSL1GHT and Cell SDK formats.

pub mod cg;
pub mod cgb;

use core::ffi::CStr;

use super::commands::packet;
use super::{Context, Error, Location};

const INSTRUCTION_SIZE: usize = 16;
const ATTRIBUTE_SIZE: usize = 12;
const CONSTANT_SIZE: usize = 28;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShaderError;

impl core::fmt::Display for ShaderError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("invalid RSX shader binary or parameter")
    }
}

impl core::error::Error for ShaderError {}

#[derive(Clone, Copy, Debug)]
pub struct Attribute<'a> {
    pub name: Option<&'a CStr>,
    pub index: u32,
    pub kind: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct Constant<'a> {
    pub name: Option<&'a CStr>,
    pub index: u32,
    pub kind: u8,
    pub internal: bool,
    pub count: u8,
    pub initial_value: [u32; 4],
}

struct Program<'a> {
    image: &'a [u8],
    code: &'a [u8],
    attributes: &'a [u8],
    constants: &'a [u8],
    registers: u16,
}

fn be16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes(bytes[offset..offset + 2].try_into().unwrap())
}

fn be32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn offset_table(image: &[u8], start: u32) -> Result<&[u8], ShaderError> {
    let table = image.get(start as usize..).ok_or(ShaderError)?;
    if table.len() < 4 {
        return Err(ShaderError);
    }
    let bytes = (be32(table, 0) as usize)
        .checked_mul(4)
        .ok_or(ShaderError)?;
    let end = bytes.checked_add(4).ok_or(ShaderError)?;
    table.get(4..end).ok_or(ShaderError)
}

fn patch_embedded(offsets: &[u8], microcode: &mut [u8], value: &[f32]) -> Result<(), ShaderError> {
    if value.is_empty() || value.len() > 4 {
        return Err(ShaderError);
    }
    let length = value.len() * 4;
    let indices = offsets.chunks_exact(4).map(|bytes| be32(bytes, 0) as usize);
    if indices.clone().any(|index| {
        index % 4 != 0
            || index
                .checked_add(length)
                .is_none_or(|end| end > microcode.len())
    }) {
        return Err(ShaderError);
    }
    for index in indices {
        for (word, float) in microcode[index..index + length]
            .chunks_exact_mut(4)
            .zip(value)
        {
            word.copy_from_slice(&float.to_bits().rotate_left(16).to_be_bytes());
        }
    }
    Ok(())
}

impl<'a> Program<'a> {
    fn parse(image: &'a [u8], magic: &[u8; 2]) -> Result<Self, ShaderError> {
        if image.len() < 36 || &image[..2] != magic {
            return Err(ShaderError);
        }
        let section = |count, offset, item_bytes| {
            let start = be32(image, offset) as usize;
            let length = usize::from(be16(image, count)) * item_bytes;
            let end = start.checked_add(length).ok_or(ShaderError)?;
            image.get(start..end).ok_or(ShaderError)
        };
        let code = section(10, 20, INSTRUCTION_SIZE)?;
        if code.is_empty() {
            return Err(ShaderError);
        }
        Ok(Self {
            image,
            code,
            attributes: section(6, 12, ATTRIBUTE_SIZE)?,
            constants: section(8, 16, CONSTANT_SIZE)?,
            registers: be16(image, 4),
        })
    }

    fn name(&self, offset: u32) -> Option<&'a CStr> {
        if offset == 0 {
            return None;
        }
        CStr::from_bytes_until_nul(self.image.get(offset as usize..)?).ok()
    }

    fn attributes(&self) -> impl ExactSizeIterator<Item = Attribute<'a>> + '_ {
        self.attributes
            .chunks_exact(ATTRIBUTE_SIZE)
            .map(|attr| Attribute {
                name: self.name(be32(attr, 0)),
                index: be32(attr, 4),
                kind: attr[8],
            })
    }

    fn constants(&self) -> impl ExactSizeIterator<Item = Constant<'a>> + '_ {
        self.constants
            .chunks_exact(CONSTANT_SIZE)
            .map(|constant| Constant {
                name: self.name(be32(constant, 0)),
                index: be32(constant, 4),
                kind: constant[8],
                internal: constant[9] != 0,
                count: constant[10],
                initial_value: core::array::from_fn(|word| be32(constant, 12 + word * 4)),
            })
    }
}

pub struct VertexProgram<'a> {
    program: Program<'a>,
    pub input_mask: u32,
    pub output_mask: u32,
    pub constant_start: u16,
    pub instruction_start: u16,
}

impl<'a> VertexProgram<'a> {
    pub fn parse(image: &'a [u8]) -> Result<Self, ShaderError> {
        let program = Program::parse(image, b"VP")?;
        Ok(Self {
            program,
            input_mask: be32(image, 24),
            output_mask: be32(image, 28),
            constant_start: be16(image, 32),
            instruction_start: be16(image, 34),
        })
    }

    pub fn microcode(&self) -> &'a [u8] {
        self.program.code
    }

    pub fn attributes(&self) -> impl ExactSizeIterator<Item = Attribute<'a>> + '_ {
        self.program.attributes()
    }

    pub fn constants(&self) -> impl ExactSizeIterator<Item = Constant<'a>> + '_ {
        self.program.constants()
    }
}

pub struct FragmentProgram<'a> {
    program: Program<'a>,
    pub control: u32,
    pub texcoords: u16,
    pub texcoords_2d: u16,
    pub texcoords_3d: u16,
}

impl<'a> FragmentProgram<'a> {
    pub fn parse(image: &'a [u8]) -> Result<Self, ShaderError> {
        let program = Program::parse(image, b"FP")?;
        Ok(Self {
            program,
            control: be32(image, 24),
            texcoords: be16(image, 28),
            texcoords_2d: be16(image, 30),
            texcoords_3d: be16(image, 32),
        })
    }

    /// Fragment microcode in the format the RSX expects in GPU memory.
    pub fn microcode(&self) -> &'a [u8] {
        self.program.code
    }

    pub fn attributes(&self) -> impl ExactSizeIterator<Item = Attribute<'a>> + '_ {
        self.program.attributes()
    }

    pub fn constants(&self) -> impl ExactSizeIterator<Item = Constant<'a>> + '_ {
        self.program.constants()
    }

    /// Updates one reflected constant row in a writable copy of the fragment
    /// program's microcode. Matrix rows have separate constant records.
    pub fn patch_constant(
        &self,
        constant: Constant<'_>,
        microcode: &mut [u8],
        value: &[f32],
    ) -> Result<(), ShaderError> {
        if constant.internal || microcode.len() != self.microcode().len() {
            return Err(ShaderError);
        }
        let offsets = if constant.index == u32::MAX {
            &[][..]
        } else {
            offset_table(self.program.image, constant.index)?
        };
        patch_embedded(offsets, microcode, value)
    }
}

impl Context {
    /// Uploads a vertex program and its internal constants.
    ///
    /// # Safety
    ///
    /// The microcode must be valid for the RSX and the application must
    /// supply all vertex inputs and external constants used by the shader.
    pub unsafe fn load_vertex_program(&mut self, shader: &VertexProgram<'_>) -> Result<(), Error> {
        let start = u32::from(shader.instruction_start);
        let instructions = shader.microcode().len() / INSTRUCTION_SIZE;
        if start as usize + instructions > 512
            || shader
                .constants()
                .filter(|value| value.internal)
                .any(|value| {
                    value
                        .index
                        .checked_add(u32::from(shader.constant_start))
                        .is_none_or(|slot| slot >= 468)
                })
        {
            return Err(Error::InvalidArgument);
        }
        unsafe {
            self.upload_vertex_microcode(start, shader.microcode())?;
            self.write_words(&[&[
                packet(0x1ff0, 2),
                shader.input_mask,
                shader.output_mask | (1 << 5),
                packet(0x1ef8, 1),
                if shader.program.registers <= 32 {
                    0x0020_ffff
                } else {
                    0x0030_ffff
                },
            ]])?;
            for value in shader.constants().filter(|value| value.internal) {
                let slot = u32::from(shader.constant_start) + value.index;
                self.write_words(&[&[packet(0x1efc, 5), slot], &value.initial_value])?;
            }
        }
        Ok(())
    }

    /// Uploads complete, validated 16-byte instructions to vertex RAM.
    unsafe fn upload_vertex_microcode(&mut self, start: u32, code: &[u8]) -> Result<(), Error> {
        unsafe { self.write_words(&[&[packet(0x1e9c, 2), start, start]])? };
        let mut words = [0; 32];
        for instructions in code.chunks(128) {
            let count = instructions.len() / 4;
            for (index, word) in words[..count].iter_mut().enumerate() {
                *word = be32(instructions, index * 4);
            }
            unsafe { self.write_words(&[&[packet(0x0b80, count)], &words[..count]])? };
        }
        Ok(())
    }

    /// Updates adjacent four-component vertex constant registers.
    pub fn set_vertex_constants(&mut self, start: u16, values: &[[f32; 4]]) -> Result<(), Error> {
        if usize::from(start)
            .checked_add(values.len())
            .is_none_or(|end| end > 468)
        {
            return Err(Error::InvalidArgument);
        }
        let mut words = [0; 32];
        for (block, values) in values.chunks(8).enumerate() {
            let count = 4 * values.len();
            for (output, component) in words.iter_mut().zip(values.iter().flatten()) {
                *output = component.to_bits();
            }
            let slot = u32::from(start) + block as u32 * 8;
            unsafe { self.write_words(&[&[packet(0x1efc, count + 1), slot], &words[..count]])? };
        }
        Ok(())
    }

    /// Selects fragment microcode already copied into RSX memory.
    ///
    /// # Safety
    ///
    /// The address must designate the unmodified contents of
    /// `shader.microcode()` or valid code with any application constants
    /// patched in. Keep the allocation live until all draws using it finish.
    pub unsafe fn load_fragment_program(
        &mut self,
        shader: &FragmentProgram<'_>,
        location: Location,
        offset: u32,
    ) -> Result<(), Error> {
        if shader.program.registers > 48 {
            return Err(Error::InvalidArgument);
        }
        let config = cgb::FragmentConfiguration {
            offset,
            texcoords: shader.texcoords,
            texcoords_2d: shader.texcoords_2d,
            texcoords_centroid: shader.texcoords_3d,
            control: shader.control | (1 << 10),
            registers: shader.program.registers as u8,
            ..Default::default()
        };
        unsafe { self.emit_fragment_config(&config, location, false) }
    }

    unsafe fn emit_fragment_config(
        &mut self,
        config: &cgb::FragmentConfiguration,
        location: Location,
        set_output_mask: bool,
    ) -> Result<(), Error> {
        let dma_context = u32::from(location as u8) + 1;
        let alignment = 64 * dma_context;
        if config.offset % alignment != 0 || config.offset >= 0x2000_0000 || config.registers > 48 {
            return Err(Error::InvalidArgument);
        }
        let address = config.offset | dma_context;
        let mut words = [0; 38];
        words[0] = packet(0x08e4, 1);
        words[1] = address;
        let mut count = 2;
        if set_output_mask {
            words[2] = packet(0x1ff4, 1);
            words[3] = config.attribute_input_mask;
            count += 2;
        }
        for index in 0..16 {
            if config.texcoords & (1 << index) != 0 {
                words[count] = packet(0x0b40 + index * 4, 1);
                let two_d = u32::from((config.texcoords_2d >> index) & 1);
                let centroid = u32::from((config.texcoords_centroid >> index) & 1);
                words[count + 1] = two_d | (centroid << 4);
                count += 2;
            }
        }
        words[count] = packet(0x1d60, 1);
        words[count + 1] = config.control | (u32::from(config.registers.max(2)) << 24);
        unsafe { self.write_words(&[&words[..count + 2]]) }
    }
}
