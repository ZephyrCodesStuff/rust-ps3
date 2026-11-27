//! Direct imports from the firmware's `cellGcmSys` library.
//!
//! # Safety
//!
//! Follow the GCM contract for the process, FIFO, GPU resources and every
//! address and output buffer. Pointer fields and pointer-to-pointer outputs
//! use 32-bit effective addresses. Event callbacks use compact CellOS
//! descriptors. A null descriptor unregisters the corresponding handler.

use core::ffi::c_void;

use crate::sys::raw::PpuFunctionDescriptor;

use super::{
    Config, ContextData, Control, DisplayInfo, NotifyData, OffsetTable, ReportData, TileInfo,
    ZcullInfo,
};

extern "C" {
    #[link_name = "_cellGcmInitBody"]
    pub(crate) fn init_body(
        context: *mut u32,
        cmd_size: u32,
        io_size: u32,
        io_address: *const c_void,
    ) -> i32;
    #[link_name = "_cellGcmSetFlipCommand"]
    pub(crate) fn set_flip_command(context: *mut ContextData, id: u8) -> i32;
    #[link_name = "_cellGcmSetFlipCommandWithWaitLabel"]
    pub(crate) fn set_flip_command_with_wait_label(
        context: *mut ContextData,
        id: u8,
        label: u8,
        value: u32,
    ) -> i32;
    #[link_name = "cellGcmAddressToOffset"]
    pub fn address_to_offset(address: *const c_void, offset: *mut u32) -> i32;
    #[link_name = "cellGcmBindTile"]
    pub fn bind_tile(index: u8) -> i32;
    #[link_name = "cellGcmBindZcull"]
    pub fn bind_zcull(
        index: u8,
        offset: u32,
        width: u32,
        height: u32,
        cull_start: u32,
        z_format: u32,
        aa_format: u32,
        z_cull_dir: u32,
        z_cull_format: u32,
        s_func: u32,
        s_ref: u32,
        s_mask: u32,
    ) -> i32;
    #[link_name = "cellGcmDumpGraphicsError"]
    pub fn dump_graphics_error() -> i32;
    #[link_name = "cellGcmGetConfiguration"]
    pub fn get_configuration(config: *mut Config);
    #[link_name = "cellGcmGetControlRegister"]
    pub fn get_control_register() -> *mut Control;
    #[link_name = "cellGcmGetCurrentDisplayBufferId"]
    pub fn get_current_display_buffer_id(id: *mut u8) -> i32;
    #[link_name = "cellGcmGetCurrentField"]
    pub fn get_current_field() -> u32;
    #[link_name = "cellGcmGetDefaultCommandWordSize"]
    pub fn get_default_command_word_size() -> u32;
    #[link_name = "cellGcmGetDefaultSegmentWordSize"]
    pub fn get_default_segment_word_size() -> u32;
    #[link_name = "cellGcmGetDisplayBufferByFlipIndex"]
    pub fn get_display_buffer_by_flip_index(qid: u32) -> i32;
    #[link_name = "cellGcmGetDisplayInfo"]
    pub fn get_display_info() -> *const DisplayInfo;
    #[link_name = "cellGcmGetFlipStatus"]
    pub fn get_flip_status() -> u32;
    #[link_name = "cellGcmGetLabelAddress"]
    pub fn get_label_address(index: u8) -> *mut u32;
    #[link_name = "cellGcmGetLastFlipTime"]
    pub fn get_last_flip_time() -> i64;
    #[link_name = "cellGcmGetLastSecondVTime"]
    pub fn get_last_second_v_time() -> i64;
    #[link_name = "cellGcmGetMaxIoMapSize"]
    pub fn get_max_io_map_size() -> u32;
    #[link_name = "cellGcmGetNotifyDataAddress"]
    pub fn get_notify_data_address(index: u32) -> *mut NotifyData;
    #[link_name = "cellGcmGetOffsetTable"]
    pub fn get_offset_table(table: *mut OffsetTable);
    #[link_name = "cellGcmGetReport"]
    pub fn get_report(r#type: u32, index: u32) -> u32;
    #[link_name = "cellGcmGetReportDataAddress"]
    pub fn get_report_data_address(index: u32) -> *mut ReportData;
    #[link_name = "cellGcmGetReportDataAddressLocation"]
    pub fn get_report_data_address_location(index: u32, location: u32) -> *mut ReportData;
    #[link_name = "cellGcmGetReportDataLocation"]
    pub fn get_report_data_location(index: u32, location: u32) -> u32;
    #[link_name = "cellGcmGetTileInfo"]
    pub fn get_tile_info() -> *const TileInfo;
    #[link_name = "cellGcmGetTiledPitchSize"]
    pub fn get_tiled_pitch_size(size: u32) -> u32;
    #[link_name = "cellGcmGetTimeStamp"]
    pub fn get_time_stamp(index: u32) -> u64;
    #[link_name = "cellGcmGetTimeStampLocation"]
    pub fn get_time_stamp_location(index: u32, location: u32) -> u64;
    #[link_name = "cellGcmGetVBlankCount"]
    pub fn get_v_blank_count() -> u64;
    #[link_name = "cellGcmGetZcullInfo"]
    pub fn get_zcull_info() -> *const ZcullInfo;
    #[link_name = "cellGcmInitCursor"]
    pub fn init_cursor() -> i32;
    #[link_name = "cellGcmInitDefaultFifoMode"]
    pub fn init_default_fifo_mode(mode: i32) -> i32;
    #[link_name = "cellGcmInitSystemMode"]
    pub fn init_system_mode(mode: u64) -> i32;
    #[link_name = "cellGcmIoOffsetToAddress"]
    pub fn io_offset_to_address(io_offset: u32, address: *mut u32) -> i32;
    #[link_name = "cellGcmMapEaIoAddress"]
    pub fn map_ea_io_address(ea: *const c_void, io: u32, size: u32) -> i32;
    #[link_name = "cellGcmMapEaIoAddressWithFlags"]
    pub fn map_ea_io_address_with_flags(
        ea: *const c_void,
        io: u32,
        size: u32,
        userflags: u32,
    ) -> i32;
    #[link_name = "cellGcmMapLocalMemory"]
    pub fn map_local_memory(address: *mut u32, size: *mut u32) -> i32;
    #[link_name = "cellGcmMapMainMemory"]
    pub fn map_main_memory(address: *const c_void, size: u32, offset: *mut u32) -> i32;
    #[link_name = "cellGcmReserveIoMapSize"]
    pub fn reserve_io_map_size(size: u32) -> i32;
    #[link_name = "cellGcmResetFlipStatus"]
    pub fn reset_flip_status();
    #[link_name = "cellGcmSetCursorDisable"]
    pub fn set_cursor_disable() -> i32;
    #[link_name = "cellGcmSetCursorEnable"]
    pub fn set_cursor_enable() -> i32;
    #[link_name = "cellGcmSetCursorImageOffset"]
    pub fn set_cursor_image_offset(offset: u32) -> i32;
    #[link_name = "cellGcmSetCursorPosition"]
    pub fn set_cursor_position(x: i32, y: i32) -> i32;
    #[link_name = "cellGcmSetDebugOutputLevel"]
    pub fn set_debug_output_level(level: i32);
    #[link_name = "cellGcmSetDefaultCommandBuffer"]
    pub fn set_default_command_buffer();
    #[link_name = "cellGcmSetDefaultCommandBufferAndSegmentWordSize"]
    pub fn set_default_command_buffer_and_segment_word_size(
        buffer_size: u32,
        segment_size: u32,
    ) -> i32;
    #[link_name = "cellGcmSetDefaultFifoSize"]
    pub fn set_default_fifo_size(buffer_size: u32, segment_size: u32) -> i32;
    #[link_name = "cellGcmSetDisplayBuffer"]
    pub fn set_display_buffer(id: u8, offset: u32, pitch: u32, width: u32, height: u32) -> i32;
    #[link_name = "cellGcmSetFlipHandler"]
    pub fn set_flip_handler(handler: *const PpuFunctionDescriptor);
    #[link_name = "cellGcmSetFlipImmediate"]
    pub fn set_flip_immediate(id: u8) -> i32;
    #[link_name = "cellGcmSetFlipMode"]
    pub fn set_flip_mode(mode: u32);
    #[link_name = "cellGcmSetFlipStatus"]
    pub fn set_flip_status();
    #[link_name = "cellGcmSetGraphicsHandler"]
    pub fn set_graphics_handler(handler: *const PpuFunctionDescriptor);
    #[link_name = "cellGcmSetInvalidateTile"]
    pub fn set_invalidate_tile(index: u8);
    #[link_name = "cellGcmSetPrepareFlip"]
    pub(crate) fn prepare_flip(context: *mut ContextData, id: u8) -> i32;
    #[link_name = "cellGcmSetQueueHandler"]
    pub fn set_queue_handler(handler: *const PpuFunctionDescriptor);
    #[link_name = "cellGcmSetSecondVFrequency"]
    pub fn set_second_v_frequency(freq: u32);
    #[link_name = "cellGcmSetSecondVHandler"]
    pub fn set_second_v_handler(handler: *const PpuFunctionDescriptor);
    #[link_name = "cellGcmSetTile"]
    pub fn set_tile(
        index: u8,
        location: u8,
        offset: u32,
        size: u32,
        pitch: u32,
        comp: u8,
        base: u16,
        bank: u8,
    );
    #[link_name = "cellGcmSetTileInfo"]
    pub fn set_tile_info(
        index: u8,
        location: u8,
        offset: u32,
        size: u32,
        pitch: u32,
        comp: u8,
        base: u16,
        bank: u8,
    ) -> i32;
    #[link_name = "cellGcmSetUserHandler"]
    pub fn set_user_handler(handler: *const PpuFunctionDescriptor);
    #[link_name = "cellGcmSetVBlankFrequency"]
    pub fn set_v_blank_frequency(freq: u32);
    #[link_name = "cellGcmSetVBlankHandler"]
    pub fn set_v_blank_handler(handler: *const PpuFunctionDescriptor);
    #[link_name = "cellGcmSetZcull"]
    pub fn set_zcull(
        index: u8,
        offset: u32,
        width: u32,
        height: u32,
        cull_start: u32,
        z_format: u32,
        aa_format: u32,
        z_cull_dir: u32,
        z_cull_format: u32,
        s_func: u32,
        s_ref: u32,
        s_mask: u32,
    );
    #[link_name = "cellGcmSortRemapEaIoAddress"]
    pub fn sort_remap_ea_io_address() -> i32;
    #[link_name = "cellGcmUnbindTile"]
    pub fn unbind_tile(index: u8) -> i32;
    #[link_name = "cellGcmUnbindZcull"]
    pub fn unbind_zcull(index: u8) -> i32;
    #[link_name = "cellGcmUnmapEaIoAddress"]
    pub fn unmap_ea_io_address(ea: *const c_void) -> i32;
    #[link_name = "cellGcmUnmapIoAddress"]
    pub fn unmap_io_address(io: u32) -> i32;
    #[link_name = "cellGcmUnreserveIoMapSize"]
    pub fn unreserve_io_map_size(size: u32) -> i32;
    #[link_name = "cellGcmUpdateCursor"]
    pub fn update_cursor() -> i32;
}
