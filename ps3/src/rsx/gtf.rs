//! Reads GPU-ready texture data from big-endian Cell GTF files.

use super::{Buffer, Error, Texture};

const HEADER_BYTES: usize = 12;
const ATTRIBUTE_BYTES: usize = 36;

/// A validated GTF texture container borrowing its original file bytes.
pub struct File<'a> {
    data: &'a [u8],
    attributes: &'a [u8],
    version: u32,
}

/// A texture descriptor and its stored mip chain.
pub struct Image<'a> {
    pub id: u32,
    pub descriptor: Texture,
    pub pixels: &'a [u8],
}

fn u16_at(data: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes(data[offset..offset + 2].try_into().unwrap())
}

fn u32_at(data: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap())
}

impl<'a> File<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, Error> {
        if data.len() < HEADER_BYTES {
            return Err(Error::InvalidArgument);
        }
        let count = u32_at(data, 8) as usize;
        let table_bytes = count
            .checked_mul(ATTRIBUTE_BYTES)
            .ok_or(Error::InvalidArgument)?;
        let end = HEADER_BYTES
            .checked_add(table_bytes)
            .ok_or(Error::InvalidArgument)?;
        let attributes = data.get(HEADER_BYTES..end).ok_or(Error::InvalidArgument)?;
        let texture_bytes = u32_at(data, 4) as usize;
        if texture_bytes > data.len() - end {
            return Err(Error::InvalidArgument);
        }
        for attribute in attributes.chunks_exact(ATTRIBUTE_BYTES) {
            let offset = u32_at(attribute, 4) as usize;
            let length = u32_at(attribute, 8) as usize;
            if offset < end
                || length == 0
                || offset
                    .checked_add(length)
                    .is_none_or(|last| last > data.len())
            {
                return Err(Error::InvalidArgument);
            }
        }
        Ok(Self {
            data,
            attributes,
            version: u32_at(data, 0),
        })
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn images(&self) -> impl ExactSizeIterator<Item = Image<'a>> + '_ {
        self.attributes.chunks_exact(ATTRIBUTE_BYTES).map(|attr| {
            let offset = u32_at(attr, 4) as usize;
            let length = u32_at(attr, 8) as usize;
            let tex = &attr[12..];
            Image {
                id: u32_at(attr, 0),
                descriptor: Texture {
                    format: tex[0],
                    mipmaps: tex[1],
                    dimension: tex[2],
                    cubemap: tex[3],
                    remap: u32_at(tex, 4),
                    width: u16_at(tex, 8),
                    height: u16_at(tex, 10),
                    depth: u16_at(tex, 12),
                    location: tex[14],
                    reserved: tex[15],
                    pitch: u32_at(tex, 16),
                    offset: u32_at(tex, 20),
                },
                pixels: &self.data[offset..offset + length],
            }
        })
    }

    pub fn image(&self, id: u32) -> Option<Image<'a>> {
        self.images().find(|image| image.id == id)
    }
}

impl Image<'_> {
    /// Returns a descriptor for a buffer containing this image's pixels at
    /// `start`. The resulting GPU offset must be 128-byte aligned. Copy the
    /// mip chain to that location before binding it.
    pub fn descriptor_at(&self, buffer: &Buffer, start: u32) -> Result<Texture, Error> {
        let length = u32::try_from(self.pixels.len()).map_err(|_| Error::InvalidArgument)?;
        let offset = buffer
            .offset()
            .checked_add(start)
            .ok_or(Error::InvalidArgument)?;
        if offset % 128 != 0
            || start
                .checked_add(length)
                .is_none_or(|end| end > buffer.size())
        {
            return Err(Error::InvalidArgument);
        }
        Ok(Texture {
            location: buffer.location() as u8,
            offset,
            ..self.descriptor
        })
    }
}
