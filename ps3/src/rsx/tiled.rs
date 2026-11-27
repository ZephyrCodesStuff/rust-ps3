use super::{methods::nv4097, raw, Compare, Context, Error, Location};

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TileCompression {
    Disabled = 0,
    Color2x1 = 7,
    Color2x2 = 8,
    DepthSeparateStencil = 9,
    DepthRegular = 10,
    DepthDiagonal = 11,
    DepthRotated = 12,
}

#[derive(Clone, Copy, Debug)]
pub struct Tile {
    pub location: Location,
    pub offset: u32,
    pub size: u32,
    pub pitch: u32,
    pub compression: TileCompression,
    pub tag_base: u16,
    pub bank: u8,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DepthFormat {
    Z16 = 1,
    Z24S8 = 2,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Antialias {
    Single = 0,
    Diagonal2 = 3,
    Centered4 = 4,
    Rotated4 = 5,
}

#[derive(Clone, Copy, Debug)]
pub struct ZCull {
    pub offset: u32,
    pub width: u32,
    pub height: u32,
    pub start: u32,
    pub depth: DepthFormat,
    pub antialias: Antialias,
    pub direction: ZCullDirection,
    pub format: ZCullFormat,
    pub stencil_compare: Compare,
    pub stencil_reference: u8,
    pub stencil_mask: u8,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ZCullDirection {
    Less = 0,
    Greater = 1,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ZCullFormat {
    MostSignificantBits = 0,
    LeadingOnes = 1,
}

impl Context {
    /// Configures and binds a tiled region.
    ///
    /// # Safety
    ///
    /// Own the tile slot and the entire memory region. Its buffer layout
    /// and compression must match every render, texture, and transfer that
    /// accesses it. Synchronize those accesses when changing the mapping.
    pub unsafe fn bind_tile(&mut self, index: u8, tile: &Tile) -> Result<(), Error> {
        if index >= 15
            || tile.size == 0
            || tile.offset % 0x10000 != 0
            || tile.size % 0x10000 != 0
            || tile.offset.checked_add(tile.size - 1).is_none()
            || tile.pitch == 0
            || tile.tag_base > 0x7ff
            || tile.bank > 3
        {
            return Err(Error::InvalidArgument);
        }
        if unsafe { raw::get_tiled_pitch_size(tile.pitch) } != tile.pitch {
            return Err(Error::InvalidArgument);
        }
        unsafe {
            Error::status(raw::set_tile_info(
                index,
                tile.location as u8,
                tile.offset,
                tile.size,
                tile.pitch,
                tile.compression as u8,
                tile.tag_base,
                tile.bank,
            ))?;
            Error::status(raw::bind_tile(index))
        }
    }

    /// Removes an application tile binding.
    ///
    /// # Safety
    ///
    /// The slot must belong to the caller. All accesses that assume the
    /// tile mapping must have finished.
    pub unsafe fn unbind_tile(&mut self, index: u8) -> Result<(), Error> {
        unsafe { Error::status(raw::unbind_tile(index)) }
    }

    /// Binds a depth buffer to a Z-cull slot.
    ///
    /// # Safety
    ///
    /// The slot, depth allocation and cull memory must be owned by the
    /// caller and sized for the dimensions, format and sample count.
    /// Synchronize existing uses of the slot before rebinding it.
    pub unsafe fn bind_zcull(&mut self, index: u8, region: &ZCull) -> Result<(), Error> {
        if index >= 8
            || region.offset % 0x10000 != 0
            || region.start % 0x1000 != 0
            || region.width == 0
            || region.width % 64 != 0
            || region.height == 0
            || region.height % 64 != 0
        {
            return Err(Error::InvalidArgument);
        }
        unsafe {
            Error::status(raw::bind_zcull(
                index,
                region.offset,
                region.width,
                region.height,
                region.start,
                region.depth as u32,
                region.antialias as u32,
                region.direction as u32,
                region.format as u32,
                (region.stencil_compare as u32) - 0x200,
                region.stencil_reference.into(),
                region.stencil_mask.into(),
            ))
        }
    }

    /// Removes an application Z-cull binding.
    ///
    /// # Safety
    ///
    /// The caller must own the slot and all draws using it must have finished.
    pub unsafe fn unbind_zcull(&mut self, index: u8) -> Result<(), Error> {
        unsafe { Error::status(raw::unbind_zcull(index)) }
    }

    pub fn set_zcull_enabled(&mut self, depth: bool, stencil: bool) -> Result<(), Error> {
        let mode = u32::from(depth) | (u32::from(stencil) << 1);
        unsafe { self.write_method(nv4097::SET_ZCULL_EN, true, &[mode]) }
    }

    pub fn set_zcull_stats_enabled(&mut self, enabled: bool) -> Result<(), Error> {
        unsafe { self.write_method(nv4097::SET_ZCULL_STATS_ENABLE, true, &[enabled.into()]) }
    }

    pub fn set_zcull_control(
        &mut self,
        direction: ZCullDirection,
        format: ZCullFormat,
    ) -> Result<(), Error> {
        let control = direction as u32 | (format as u32) << 4;
        unsafe { self.write_method(nv4097::SET_ZCULL_CONTROL0, true, &[control]) }
    }

    pub fn set_zcull_limits(&mut self, move_forward: u16, push_back: u16) -> Result<(), Error> {
        let limits = u32::from(move_forward) << 16 | u32::from(push_back);
        unsafe { self.write_method(nv4097::SET_ZCULL_CONTROL1, true, &[limits]) }
    }

    pub fn set_stencil_cull(
        &mut self,
        compare: Compare,
        reference: u8,
        mask: u8,
    ) -> Result<(), Error> {
        let control = (compare as u32 - 0x200) | u32::from(reference) << 16 | u32::from(mask) << 24;
        unsafe { self.write_method(nv4097::SET_SCULL_CONTROL, true, &[control]) }
    }

    /// Invalidates Z-cull data for the current depth buffer.
    pub fn clear_zcull(&mut self, depth: bool, stencil: bool) -> Result<(), Error> {
        let mode = u32::from(depth) | (u32::from(stencil) << 1);
        unsafe { self.write_method(nv4097::CLEAR_ZCULL_SURFACE, true, &[mode]) }
    }
}
