//! Reads revision-6 big-endian Cg binaries from the Cell SDK.

use core::ffi::CStr;

use super::cgb::{FragmentConfiguration, VertexConfiguration};
use super::{
    be16, be32, embedded_indices, fragment_address, offset_table, patch_embedded, ShaderError,
    INSTRUCTION_SIZE,
};
use crate::rsx::commands::packet;
use crate::rsx::transfer::TransferMemory;
use crate::rsx::{Context, Error, Location};

const PARAMETER_BYTES: usize = 48;
const CG_CONSTANT: u32 = 4103;
// Values from Cell SDK 3.70's Cg/NV/cg_datatypes.h (CG_TYPE_START_ENUM=1024).
const CG_FLOAT: u32 = 1044;
const CG_FLOAT1: u32 = 1090;
const CG_FLOAT2: u32 = 1045;
const CG_FLOAT3: u32 = 1046;
const CG_FLOAT4: u32 = 1047;
const CG_FLOAT3X3: u32 = 1058;
const CG_FLOAT3X4: u32 = 1059;
const CG_FLOAT4X3: u32 = 1062;
const CG_FLOAT4X4: u32 = 1063;
const CG_BOOL: u32 = 1113;
const CG_BOOL1: u32 = 1114;
const CG_BOOL2: u32 = 1115;
const CG_BOOL3: u32 = 1116;
const CG_BOOL4: u32 = 1117;
const CG_BOOL3X3: u32 = 1128;
const CG_BOOL3X4: u32 = 1129;
const CG_BOOL4X3: u32 = 1132;
const CG_BOOL4X4: u32 = 1133;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Profile {
    Vertex,
    Fragment,
}

/// A decoded parameter from a Cg binary.
pub struct Parameter<'a> {
    image: &'a [u8],
    embedded_constants: u32,
    parameter_offset: usize,
    parameter_end: usize,
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
        self.parameters
            .chunks_exact(PARAMETER_BYTES)
            .enumerate()
            .map(|(index, p)| {
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
                    parameter_offset: index * PARAMETER_BYTES + be32(self.image, 16) as usize,
                    parameter_end: be32(self.image, 16) as usize + self.parameters.len(),
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
    /// Queues an update of one reflected Cell SDK Cg fragment constant row.
    /// Call `refresh_fragment_program` after uploading the desired rows.
    ///
    /// # Safety
    /// The offset must identify the parameter's live fragment microcode
    /// in GPU memory, with accesses synchronized around the transfer.
    pub unsafe fn upload_cg_fragment_constant(
        &mut self,
        parameter: &Parameter<'_>,
        location: Location,
        offset: u32,
        value: &[f32],
    ) -> Result<(), Error> {
        if be32(parameter.image, 0) != 7004 {
            return Err(Error::InvalidArgument);
        }
        let size = be32(parameter.image, 24) as usize;
        let offsets = parameter
            .offset_bytes()
            .map_err(|_| Error::InvalidArgument)?;
        unsafe { self.upload_embedded_constants(location, offset, size, offsets, value) }
    }

    /// Copies a Cell SDK `SetFragmentProgramParameterPointer` source into the
    /// embedded copies of a fragment parameter. Source words must already
    /// have the SDK-required 16-bit-half swap. Matrix values use the SDK
    /// binary table's consecutive row parameters. `source.pitch` and
    /// `destination.pitch` are ignored; rows advance by the SDK byte width.
    ///
    /// # Safety
    /// Both memory ranges must be live GPU allocations, synchronized with CPU
    /// and GPU access, and the source range must contain the parameter data.
    pub unsafe fn copy_cg_fragment_parameter_pointer(
        &mut self,
        shader: &Program<'_>,
        parameter: &Parameter<'_>,
        destination: TransferMemory,
        source: TransferMemory,
    ) -> Result<(), Error> {
        if shader.profile != Profile::Fragment
            || be32(parameter.image, 0) != 7004
            || !core::ptr::eq(shader.image.as_ptr(), parameter.image.as_ptr())
            || shader.image.len() != parameter.image.len()
        {
            return Err(Error::InvalidArgument);
        }

        let (rows, row_bytes, first_parameter) = match parameter.data_type {
            CG_FLOAT | CG_BOOL | CG_FLOAT1 | CG_BOOL1 => (1usize, 4u32, parameter.parameter_offset),
            CG_FLOAT2 | CG_BOOL2 => (1, 8, parameter.parameter_offset),
            CG_FLOAT3 | CG_BOOL3 => (1, 12, parameter.parameter_offset),
            CG_FLOAT4 | CG_BOOL4 => (1, 16, parameter.parameter_offset),
            CG_FLOAT3X3 | CG_BOOL3X3 => (3, 12, parameter.parameter_offset + PARAMETER_BYTES),
            CG_FLOAT4X3 | CG_BOOL4X3 => (4, 12, parameter.parameter_offset + PARAMETER_BYTES),
            CG_FLOAT3X4 | CG_BOOL3X4 => (3, 16, parameter.parameter_offset + PARAMETER_BYTES),
            CG_FLOAT4X4 | CG_BOOL4X4 => (4, 16, parameter.parameter_offset + PARAMETER_BYTES),
            _ => return Err(Error::InvalidArgument),
        };
        if rows > 1
            && first_parameter
                .checked_add(rows * PARAMETER_BYTES)
                .is_none_or(|end| end > parameter.parameter_end)
        {
            return Err(Error::InvalidArgument);
        }
        let code_size = be32(parameter.image, 24) as usize;
        fragment_address(destination.location, destination.offset)?;
        if code_size > (0x2000_0000 - destination.offset) as usize {
            return Err(Error::InvalidArgument);
        }
        if source.offset % 4 != 0 {
            return Err(Error::InvalidArgument);
        }
        // Validate and retain the row tables before queuing any transfer.
        let mut row_offsets: [&[u8]; 4] = [&[]; 4];
        for row in 0..rows {
            let record = if rows == 1 {
                region(parameter.image, first_parameter as u32, PARAMETER_BYTES)
            } else {
                region(
                    parameter.image,
                    (first_parameter + row * PARAMETER_BYTES) as u32,
                    PARAMETER_BYTES,
                )
            }
            .map_err(|_| Error::InvalidArgument)?;
            let offsets = if be32(record, 24) == 0 {
                &[][..]
            } else {
                offset_table(parameter.image, be32(record, 24))
                    .map_err(|_| Error::InvalidArgument)?
            };
            let indices = embedded_indices(offsets, code_size, (row_bytes / 4) as usize)
                .map_err(|_| Error::InvalidArgument)?;
            row_offsets[row] = offsets;
            let source_offset = source
                .offset
                .checked_add(
                    (row as u32)
                        .checked_mul(row_bytes)
                        .ok_or(Error::InvalidArgument)?,
                )
                .ok_or(Error::InvalidArgument)?;
            for index in indices {
                let index = u32::try_from(index).map_err(|_| Error::InvalidArgument)?;
                destination
                    .offset
                    .checked_add(index)
                    .ok_or(Error::InvalidArgument)?;
            }
            source_offset
                .checked_add(row_bytes - 1)
                .ok_or(Error::InvalidArgument)?;
        }
        for row in 0..rows {
            let offsets = row_offsets[row];
            let indices = embedded_indices(offsets, code_size, (row_bytes / 4) as usize)
                .map_err(|_| Error::InvalidArgument)?;
            let source_offset = source.offset + row as u32 * row_bytes;
            for index in indices {
                let destination_offset = destination.offset + index as u32;
                unsafe {
                    self.copy_rows(
                        TransferMemory {
                            offset: source_offset,
                            pitch: row_bytes as i32,
                            ..source
                        },
                        TransferMemory {
                            offset: destination_offset,
                            pitch: row_bytes as i32,
                            ..destination
                        },
                        row_bytes,
                        1,
                    )?;
                }
            }
        }
        Ok(())
    }

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
