//! Firmware routines for loading and inspecting SPU images.

use core::ffi::c_void;

use crate::sys::raw::{SysSpuImage, SysSpuSegment};

pub const SYS_SPU_IMAGE_PROTECT: u32 = 0;
pub const SYS_SPU_IMAGE_DIRECT: u32 = 1;

extern "C" {
    pub fn sys_spu_image_import(image: *mut SysSpuImage, elf: *const c_void, mode: u32) -> i32;
    pub fn sys_spu_image_close(image: *mut SysSpuImage) -> i32;
    pub fn sys_spu_elf_get_information(elf_addr: u32, entry: *mut u32, count: *mut i32) -> i32;
    pub fn sys_spu_elf_get_segments(elf_addr: u32, segments: *mut SysSpuSegment, count: i32)
        -> i32;
    pub fn sys_raw_spu_load(id: i32, path: *const u8, entry: *mut u32) -> i32;
    pub fn sys_raw_spu_image_load(id: i32, image: *mut SysSpuImage) -> i32;
}
