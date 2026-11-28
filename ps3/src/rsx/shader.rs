//! RSX shader microcode and reflection for Cell SDK shader formats.

pub mod cg;
pub mod cgb;

use super::commands::packet;
use super::{Context, Error, Location};

const INSTRUCTION_SIZE: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShaderError;

impl core::fmt::Display for ShaderError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("invalid RSX shader binary or parameter")
    }
}

impl core::error::Error for ShaderError {}

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

pub(super) fn embedded_indices(
    offsets: &[u8],
    code_size: usize,
    value_count: usize,
) -> Result<impl Iterator<Item = usize> + use<'_>, ShaderError> {
    if !(1..=4).contains(&value_count) {
        return Err(ShaderError);
    }
    let length = value_count * 4;
    let indices = offsets.chunks_exact(4).map(|bytes| be32(bytes, 0) as usize);
    if indices
        .clone()
        .any(|index| index % 4 != 0 || index.checked_add(length).is_none_or(|end| end > code_size))
    {
        return Err(ShaderError);
    }
    Ok(indices)
}

fn patch_embedded(offsets: &[u8], microcode: &mut [u8], value: &[f32]) -> Result<(), ShaderError> {
    let indices = embedded_indices(offsets, microcode.len(), value.len())?;
    let length = value.len() * 4;
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

impl Context {
    /// Selects the instruction slot at which the vertex program begins.
    pub fn set_vertex_program_start_slot(&mut self, start_slot: u32) -> Result<(), Error> {
        if start_slot >= 512 {
            return Err(Error::InvalidArgument);
        }
        unsafe { self.write_words(&[&[packet(0x1ea0, 1), start_slot]]) }
    }

    /// Selects the instruction slot to receive subsequent vertex microcode.
    pub fn set_vertex_program_load_slot(&mut self, load_slot: u32) -> Result<(), Error> {
        if load_slot >= 512 {
            return Err(Error::InvalidArgument);
        }
        unsafe { self.write_words(&[&[packet(0x1e9c, 1), load_slot]]) }
    }

    /// Sets the vertex shader's 16-bit input attribute mask.
    pub fn set_vertex_attribute_input_mask(&mut self, mask: u16) -> Result<(), Error> {
        unsafe { self.write_words(&[&[packet(0x1ff0, 1), u32::from(mask)]]) }
    }

    /// Sets the vertex shader's 32-bit output attribute mask.
    pub fn set_vertex_attribute_output_mask(&mut self, mask: u32) -> Result<(), Error> {
        unsafe { self.write_words(&[&[packet(0x1ff4, 1), mask]]) }
    }

    /// Selects the transform timeout register budget for a vertex program.
    /// The RSX supports a 32-register budget through 32 used registers and a
    /// 48-register budget above that threshold, as in Cell GCM.
    pub fn set_vertex_program_register_count(&mut self, register_count: u32) -> Result<(), Error> {
        let registers = if register_count <= 32 { 32 } else { 48 };
        let timeout = 0x0000_ffff | (registers << 16);
        unsafe { self.write_words(&[&[packet(0x1ef8, 1), timeout]]) }
    }

    /// Enables or disables fragment shader gamma conversion.
    pub fn set_fragment_program_gamma_enable(&mut self, enable: bool) -> Result<(), Error> {
        unsafe { self.write_words(&[&[packet(0x1fec, 1), u32::from(enable)]]) }
    }

    /// Refreshes the microcode address after queued fragment constant writes.
    ///
    /// # Safety
    /// The address must identify valid, live fragment microcode. Observe
    /// the same memory and synchronization rules as for fragment loading.
    pub unsafe fn refresh_fragment_program(
        &mut self,
        location: Location,
        offset: u32,
    ) -> Result<(), Error> {
        let address = fragment_address(location, offset)?;
        unsafe { self.write_words(&[&[packet(0x08e4, 1), address]]) }
    }

    pub(super) unsafe fn upload_embedded_constants(
        &mut self,
        location: Location,
        offset: u32,
        code_size: usize,
        offsets: &[u8],
        value: &[f32],
    ) -> Result<(), Error> {
        fragment_address(location, offset)?;
        if code_size > (0x2000_0000 - offset) as usize {
            return Err(Error::InvalidArgument);
        }
        let indices = embedded_indices(offsets, code_size, value.len())
            .map_err(|_| Error::InvalidArgument)?;
        let mut words = [0; 4];
        for (word, component) in words.iter_mut().zip(value) {
            *word = component.to_bits().rotate_left(16);
        }
        for index in indices {
            unsafe {
                self.inline_transfer(location, offset + index as u32, &words[..value.len()])?
            };
        }
        Ok(())
    }

    pub(super) unsafe fn upload_vertex_microcode(
        &mut self,
        start: u32,
        code: &[u8],
    ) -> Result<(), Error> {
        if code.is_empty() || code.len() % INSTRUCTION_SIZE != 0 {
            return Err(Error::InvalidArgument);
        }
        let count = code.len() / INSTRUCTION_SIZE;
        if start >= 512 || count > 512 - start as usize {
            return Err(Error::InvalidArgument);
        }
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
    pub(super) fn set_vertex_constants(
        &mut self,
        start: u16,
        values: &[[f32; 4]],
    ) -> Result<(), Error> {
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

    pub(super) unsafe fn emit_fragment_config(
        &mut self,
        config: &cgb::FragmentConfiguration,
        location: Location,
        set_output_mask: bool,
    ) -> Result<(), Error> {
        let address = fragment_address(location, config.offset)?;
        if config.registers > 48 {
            return Err(Error::InvalidArgument);
        }
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

fn fragment_address(location: Location, offset: u32) -> Result<u32, Error> {
    let dma_context = u32::from(location as u8) + 1;
    if offset % (64 * dma_context) != 0 || offset >= 0x2000_0000 {
        return Err(Error::InvalidArgument);
    }
    Ok(offset | dma_context)
}
