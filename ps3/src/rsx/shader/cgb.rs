//! Configuration and microcode from Cell CGB shader binaries.

use super::{be16, be32, ShaderError, INSTRUCTION_SIZE};
use crate::rsx::commands::packet;
use crate::rsx::{Context, Error, Location};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct VertexConfiguration {
    pub instruction_slot: u16,
    pub instruction_count: u16,
    pub attribute_input_mask: u16,
    pub registers: u8,
    pub reserved: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct FragmentConfiguration {
    pub offset: u32,
    pub attribute_input_mask: u32,
    pub texcoords: u16,
    pub texcoords_2d: u16,
    pub texcoords_centroid: u16,
    pub reserved0: u16,
    pub control: u32,
    pub registers: u8,
    pub reserved1: u8,
    pub reserved2: u16,
}

const _: () = {
    use core::mem::{align_of, offset_of, size_of};
    assert!(size_of::<VertexConfiguration>() == 8);
    assert!(align_of::<VertexConfiguration>() == 2);
    assert!(offset_of!(VertexConfiguration, registers) == 6);
    assert!(size_of::<FragmentConfiguration>() == 24);
    assert!(align_of::<FragmentConfiguration>() == 4);
    assert!(offset_of!(FragmentConfiguration, control) == 16);
    assert!(offset_of!(FragmentConfiguration, registers) == 20);
};

pub struct VertexProgram<'a> {
    pub config: VertexConfiguration,
    pub attribute_output_mask: u32,
    pub user_clip_mask: u32,
    code: &'a [u8],
    constant_slots: &'a [u8],
    constant_values: &'a [u8],
}

impl<'a> VertexProgram<'a> {
    pub fn parse(binary: &'a [u8]) -> Result<Self, ShaderError> {
        let program = Program::parse(binary, 0)?;
        let block = program.constant_table;
        let count = if block.is_empty() {
            0
        } else {
            usize::from(be16(block, 2))
        };
        let (constant_slots, constant_values) = if count == 0 {
            (&[][..], &[][..])
        } else {
            let values_start = (4 + count * 2).next_multiple_of(16);
            (
                block.get(4..4 + count * 2).ok_or(ShaderError)?,
                block
                    .get(values_start..values_start + count * 16)
                    .ok_or(ShaderError)?,
            )
        };
        let config = VertexConfiguration {
            instruction_slot: be16(binary, 24),
            instruction_count: (program.code.len() / INSTRUCTION_SIZE) as u16,
            attribute_input_mask: be16(binary, 12),
            registers: binary[14],
            reserved: 0,
        };
        Ok(Self {
            config,
            attribute_output_mask: be32(binary, 16),
            user_clip_mask: be32(binary, 20),
            code: program.code,
            constant_slots,
            constant_values,
        })
    }

    pub fn microcode(&self) -> &'a [u8] {
        self.code
    }

    pub fn initial_constants(&self) -> impl ExactSizeIterator<Item = (u16, [f32; 4])> + '_ {
        self.constant_slots
            .chunks_exact(2)
            .zip(self.constant_values.chunks_exact(16))
            .map(|(slot, value)| {
                (
                    be16(slot, 0),
                    core::array::from_fn(|i| f32::from_bits(be32(value, 4 * i))),
                )
            })
    }
}

pub struct FragmentProgram<'a> {
    config: FragmentConfiguration,
    code: &'a [u8],
}

impl<'a> FragmentProgram<'a> {
    pub fn parse(binary: &'a [u8]) -> Result<Self, ShaderError> {
        let program = Program::parse(binary, 1)?;
        let config = FragmentConfiguration {
            attribute_input_mask: be32(binary, 12),
            texcoords: be16(binary, 16),
            texcoords_2d: be16(binary, 18),
            texcoords_centroid: be16(binary, 20),
            control: be32(binary, 24),
            registers: binary[28],
            ..Default::default()
        };
        Ok(Self {
            config,
            code: program.code,
        })
    }

    pub fn microcode(&self) -> &'a [u8] {
        self.code
    }

    pub fn configuration_at(&self, offset: u32) -> FragmentConfiguration {
        FragmentConfiguration {
            offset,
            ..self.config
        }
    }

    /// Preserves the vertex shader's user clip outputs in the fragment input
    /// mask. The SDK's `userclip_gcm` sample copies vertex output bits 6–11
    /// into this mask because GCM derives the vertex output mask from it.
    pub fn configuration_at_with_vertex_output_mask(
        &self,
        offset: u32,
        vertex_output_mask: u32,
    ) -> FragmentConfiguration {
        let mut config = self.configuration_at(offset);
        config.attribute_input_mask |= vertex_output_mask & (0x3f << 6);
        config
    }
}

struct Program<'a> {
    code: &'a [u8],
    constant_table: &'a [u8],
}

impl<'a> Program<'a> {
    fn parse(binary: &'a [u8], profile: u8) -> Result<Self, ShaderError> {
        if binary.len() < 32
            || &binary[..4] != b"CGB\0"
            || be16(binary, 4) != 0
            || binary[10] != profile
        {
            return Err(ShaderError);
        }
        let code_size = usize::from(be16(binary, 8));
        if code_size == 0 || code_size % INSTRUCTION_SIZE != 0 {
            return Err(ShaderError);
        }
        let code_end = 32 + code_size;
        let code = binary.get(32..code_end).ok_or(ShaderError)?;
        let constant_table = if binary[11] & 1 != 0 {
            let block = binary.get(code_end..).ok_or(ShaderError)?;
            if block.len() < 4 {
                return Err(ShaderError);
            }
            let block_size = usize::from(be16(block, 0));
            if block_size < 4 {
                return Err(ShaderError);
            }
            block.get(..block_size).ok_or(ShaderError)?
        } else {
            &[]
        };
        Ok(Self {
            code,
            constant_table,
        })
    }
}

impl Context {
    /// Loads a Cell CGB vertex microcode blob using its GCM configuration.
    ///
    /// # Safety
    ///
    /// The code must be valid big-endian RSX microcode. Initialize all
    /// shader input attributes and constant registers before drawing.
    pub unsafe fn load_cgb_vertex_configuration(
        &mut self,
        config: &VertexConfiguration,
        microcode: &[u8],
    ) -> Result<(), Error> {
        let start = usize::from(config.instruction_slot);
        let count = usize::from(config.instruction_count);
        if count == 0 || start + count > 512 || microcode.len() != count * INSTRUCTION_SIZE {
            return Err(Error::InvalidArgument);
        }
        let timeout = if config.registers <= 32 {
            0x0020_ffff
        } else {
            0x0030_ffff
        };
        unsafe {
            self.upload_vertex_microcode(start as u32, microcode)?;
            self.write_words(&[&[
                packet(0x1ff0, 1),
                u32::from(config.attribute_input_mask),
                packet(0x1ef8, 1),
                timeout,
            ]])
        }
    }

    /// Loads the shader and its stored default values.
    ///
    /// # Safety
    ///
    /// Use valid RSX microcode and bind the required vertex attributes and
    /// remaining constants before drawing.
    pub unsafe fn load_cgb_vertex_program(
        &mut self,
        shader: &VertexProgram<'_>,
    ) -> Result<(), Error> {
        if shader
            .initial_constants()
            .any(|(register, _)| register >= 468)
        {
            return Err(Error::InvalidArgument);
        }
        unsafe { self.load_cgb_vertex_configuration(&shader.config, shader.microcode())? };
        for (register, value) in shader.initial_constants() {
            self.set_vertex_constants(register, &[value])?;
        }
        Ok(())
    }

    /// Loads the GCM fragment configuration for a CGB shader in GPU memory.
    ///
    /// # Safety
    ///
    /// The configured address must designate valid RSX fragment microcode
    /// in the selected memory space. Keep it mapped until the GPU finishes.
    pub unsafe fn load_cgb_fragment_program(
        &mut self,
        config: &FragmentConfiguration,
        location: Location,
    ) -> Result<(), Error> {
        unsafe { self.emit_fragment_config(config, location, true) }
    }
}
