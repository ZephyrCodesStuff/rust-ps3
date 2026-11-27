//! Reads revision-6 big-endian Cg binaries from the Cell SDK.

use core::ffi::CStr;

use super::cgb::{FragmentConfiguration, VertexConfiguration};
use super::{be16, be32, offset_table, patch_embedded, ShaderError, INSTRUCTION_SIZE};
use crate::rsx::commands::packet;
use crate::rsx::{Context, Error, Location};

const PARAMETER_BYTES: usize = 48;
const CG_CONSTANT: u32 = 4103;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Profile {
    Vertex,
    Fragment,
}

/// A decoded parameter from a Cg binary.
pub struct Parameter<'a> {
    image: &'a [u8],
    embedded_constants: u32,
    pub name: Option<&'a CStr>,
    pub semantic: Option<&'a CStr>,
    pub data_type: u32,
    pub resource: u32,
    pub resource_index: i32,
    pub variability: u32,
    pub direction: u32,
    pub ordinal: i32,
    pub referenced: bool,
    pub shared: bool,
    pub default_value: Option<[f32; 4]>,
}

impl Parameter<'_> {
    fn offset_bytes(&self) -> Result<&[u8], ShaderError> {
        if self.embedded_constants == 0 {
            Ok(&[])
        } else {
            offset_table(self.image, self.embedded_constants)
        }
    }

    /// Returns the byte offsets of inlined fragment constants in the microcode.
    pub fn embedded_offsets(&self) -> Result<impl ExactSizeIterator<Item = u32> + '_, ShaderError> {
        Ok(self
            .offset_bytes()?
            .chunks_exact(4)
            .map(|offset| be32(offset, 0)))
    }

    /// Updates the embedded copies of a fragment parameter. RSX fragment
    /// instructions store each float with the 16-bit halves swapped.
    pub fn patch_fragment_constant(
        &self,
        microcode: &mut [u8],
        value: &[f32],
    ) -> Result<(), ShaderError> {
        patch_embedded(self.offset_bytes()?, microcode, value)
    }
}

pub struct Program<'a> {
    image: &'a [u8],
    profile: Profile,
    code: &'a [u8],
    configuration: &'a [u8],
    parameters: &'a [u8],
}

fn region(image: &[u8], offset: u32, length: usize) -> Result<&[u8], ShaderError> {
    let start = offset as usize;
    let end = start.checked_add(length).ok_or(ShaderError)?;
    image.get(start..end).ok_or(ShaderError)
}

fn name(image: &[u8], offset: u32) -> Option<&CStr> {
    if offset == 0 {
        return None;
    }
    CStr::from_bytes_until_nul(image.get(offset as usize..)?).ok()
}

impl<'a> Program<'a> {
    pub fn parse(binary: &'a [u8]) -> Result<Self, ShaderError> {
        if binary.len() < 32 || be32(binary, 4) != 6 {
            return Err(ShaderError);
        }
        let profile = match be32(binary, 0) {
            7003 => Profile::Vertex,
            7004 => Profile::Fragment,
            _ => return Err(ShaderError),
        };
        let image = region(binary, 0, be32(binary, 8) as usize)?;
        if image.len() < 32 {
            return Err(ShaderError);
        }
        let code_size = be32(image, 24) as usize;
        if code_size == 0 || code_size % INSTRUCTION_SIZE != 0 {
            return Err(ShaderError);
        }
        let parameter_count = be32(image, 12) as usize;
        let parameter_size = parameter_count
            .checked_mul(PARAMETER_BYTES)
            .ok_or(ShaderError)?;
        Ok(Self {
            image,
            profile,
            code: region(image, be32(image, 28), code_size)?,
            configuration: region(image, be32(image, 20), 24)?,
            parameters: region(image, be32(image, 16), parameter_size)?,
        })
    }

    pub fn profile(&self) -> Profile {
        self.profile
    }

    pub fn microcode(&self) -> &'a [u8] {
        self.code
    }

    pub fn parameters(&self) -> impl ExactSizeIterator<Item = Parameter<'a>> + '_ {
        self.parameters.chunks_exact(PARAMETER_BYTES).map(|p| {
            let default_value = be32(p, 20);
            let default_value = if default_value == 0 {
                None
            } else {
                region(self.image, default_value, 16).ok().map(|value| {
                    core::array::from_fn(|index| f32::from_bits(be32(value, index * 4)))
                })
            };
            Parameter {
                image: self.image,
                embedded_constants: be32(p, 24),
                data_type: be32(p, 0),
                resource: be32(p, 4),
                variability: be32(p, 8),
                resource_index: be32(p, 12) as i32,
                name: name(self.image, be32(p, 16)),
                semantic: name(self.image, be32(p, 28)),
                direction: be32(p, 32),
                ordinal: be32(p, 36) as i32,
                referenced: be32(p, 40) != 0,
                shared: be32(p, 44) != 0,
                default_value,
            }
        })
    }

    pub fn vertex_configuration(&self) -> Result<VertexConfiguration, ShaderError> {
        if self.profile != Profile::Vertex {
            return Err(ShaderError);
        }
        let info = self.configuration;
        Ok(VertexConfiguration {
            instruction_count: u16::try_from(be32(info, 0)).map_err(|_| ShaderError)?,
            instruction_slot: u16::try_from(be32(info, 4)).map_err(|_| ShaderError)?,
            registers: u8::try_from(be32(info, 8)).map_err(|_| ShaderError)?,
            attribute_input_mask: u16::try_from(be32(info, 12)).map_err(|_| ShaderError)?,
            reserved: 0,
        })
    }

    pub fn fragment_configuration(
        &self,
        offset: u32,
    ) -> Result<FragmentConfiguration, ShaderError> {
        if self.profile != Profile::Fragment {
            return Err(ShaderError);
        }
        let info = self.configuration;
        let mut control = 1 << 10;
        if info[19] == 0 {
            control |= 0x40;
        }
        if info[20] != 0 {
            control |= 0x0e;
        }
        if info[21] != 0 {
            control |= 0x80;
        }
        Ok(FragmentConfiguration {
            offset,
            attribute_input_mask: be32(info, 4) | (1 << 5),
            texcoords: be16(info, 12),
            texcoords_2d: be16(info, 14),
            texcoords_centroid: be16(info, 16),
            registers: info[18],
            control,
            ..Default::default()
        })
    }
}

impl Context {
    /// Uploads a Cell SDK Cg vertex program and its internal defaults.
    ///
    /// # Safety
    ///
    /// The program must be valid RSX code. The caller must supply all
    /// shader inputs and any remaining constants before issuing draws.
    pub unsafe fn load_cg_vertex_program(&mut self, shader: &Program<'_>) -> Result<(), Error> {
        let config = shader
            .vertex_configuration()
            .map_err(|_| Error::InvalidArgument)?;
        unsafe {
            self.load_cgb_vertex_configuration(&config, shader.microcode())?;
            self.write_words(&[&[packet(0x1ff4, 1), be32(shader.configuration, 16) | (1 << 5)]])?;
        }
        for parameter in shader.parameters() {
            if parameter.variability == CG_CONSTANT && (0..468).contains(&parameter.resource_index)
            {
                if let Some(value) = parameter.default_value {
                    self.set_vertex_constants(parameter.resource_index as u16, &[value])?;
                }
            }
        }
        Ok(())
    }

    /// Binds Cell SDK Cg fragment instructions already stored in GPU memory.
    ///
    /// # Safety
    ///
    /// The address must hold live and valid fragment microcode for this
    /// program. Keep it accessible until the last dependent draw completes.
    pub unsafe fn load_cg_fragment_program(
        &mut self,
        shader: &Program<'_>,
        location: Location,
        offset: u32,
    ) -> Result<(), Error> {
        let config = shader
            .fragment_configuration(offset)
            .map_err(|_| Error::InvalidArgument)?;
        unsafe { self.load_cgb_fragment_program(&config, location) }
    }
}
