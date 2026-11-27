//! Named RSX method addresses, grouped by hardware class.
//!
//! Addresses include the subchannel selected by the firmware initializer.
//! Use [`super::Context::write_method`] with the payload for the selected
//! method. Indexed families expose the address of the first register.

use super::Method;

const fn method(address: u16) -> Method {
    Method::new(address).unwrap()
}

/// FIFO flow control and command-processor semaphores.
pub mod nv406e {
    use super::method;
    use super::Method;

    pub const SET_REFERENCE: Method = method(0x0050);
    pub const SET_CONTEXT_DMA_SEMAPHORE: Method = method(0x0060);
    pub const SEMAPHORE_OFFSET: Method = method(0x0064);
    pub const SEMAPHORE_ACQUIRE: Method = method(0x0068);
    pub const SEMAPHORE_RELEASE: Method = method(0x006c);
}

/// The RSX 3D engine.
pub mod nv4097 {
    use super::method;
    use super::Method;

    pub const SET_OBJECT: Method = method(0x0000);
    pub const NO_OPERATION: Method = method(0x0100);
    pub const NOTIFY: Method = method(0x0104);
    pub const WAIT_FOR_IDLE: Method = method(0x0110);
    pub const PM_TRIGGER: Method = method(0x0140);
    pub const SET_CONTEXT_DMA_NOTIFIES: Method = method(0x0180);
    pub const SET_CONTEXT_DMA_A: Method = method(0x0184);
    pub const SET_CONTEXT_DMA_B: Method = method(0x0188);
    pub const SET_CONTEXT_DMA_COLOR_B: Method = method(0x018c);
    pub const SET_CONTEXT_DMA_STATE: Method = method(0x0190);
    pub const SET_CONTEXT_DMA_COLOR_A: Method = method(0x0194);
    pub const SET_CONTEXT_DMA_ZETA: Method = method(0x0198);
    pub const SET_CONTEXT_DMA_VERTEX_A: Method = method(0x019c);
    pub const SET_CONTEXT_DMA_VERTEX_B: Method = method(0x01a0);
    pub const SET_CONTEXT_DMA_SEMAPHORE: Method = method(0x01a4);
    pub const SET_CONTEXT_DMA_REPORT: Method = method(0x01a8);
    pub const SET_CONTEXT_DMA_CLIP_ID: Method = method(0x01ac);
    pub const SET_CONTEXT_DMA_CULL_DATA: Method = method(0x01b0);
    pub const SET_CONTEXT_DMA_COLOR_C: Method = method(0x01b4);
    pub const SET_CONTEXT_DMA_COLOR_D: Method = method(0x01b8);
    pub const SET_SURFACE_CLIP_HORIZONTAL: Method = method(0x0200);
    pub const SET_SURFACE_CLIP_VERTICAL: Method = method(0x0204);
    pub const SET_SURFACE_FORMAT: Method = method(0x0208);
    pub const SET_SURFACE_PITCH_A: Method = method(0x020c);
    pub const SET_SURFACE_COLOR_AOFFSET: Method = method(0x0210);
    pub const SET_SURFACE_ZETA_OFFSET: Method = method(0x0214);
    pub const SET_SURFACE_COLOR_BOFFSET: Method = method(0x0218);
    pub const SET_SURFACE_PITCH_B: Method = method(0x021c);
    pub const SET_SURFACE_COLOR_TARGET: Method = method(0x0220);
    pub const SET_SURFACE_PITCH_Z: Method = method(0x022c);
    pub const INVALIDATE_ZCULL: Method = method(0x0234);
    pub const SET_CYLINDRICAL_WRAP: Method = method(0x0238);
    pub const SET_CYLINDRICAL_WRAP1: Method = method(0x023c);
    pub const SET_SURFACE_PITCH_C: Method = method(0x0280);
    pub const SET_SURFACE_PITCH_D: Method = method(0x0284);
    pub const SET_SURFACE_COLOR_COFFSET: Method = method(0x0288);
    pub const SET_SURFACE_COLOR_DOFFSET: Method = method(0x028c);
    pub const SET_WINDOW_OFFSET: Method = method(0x02b8);
    pub const SET_WINDOW_CLIP_TYPE: Method = method(0x02bc);
    pub const SET_WINDOW_CLIP_HORIZONTAL: Method = method(0x02c0);
    pub const SET_WINDOW_CLIP_VERTICAL: Method = method(0x02c4);
    pub const SET_DITHER_ENABLE: Method = method(0x0300);
    pub const SET_ALPHA_TEST_ENABLE: Method = method(0x0304);
    pub const SET_ALPHA_FUNC: Method = method(0x0308);
    pub const SET_ALPHA_REF: Method = method(0x030c);
    pub const SET_BLEND_ENABLE: Method = method(0x0310);
    pub const SET_BLEND_FUNC_SFACTOR: Method = method(0x0314);
    pub const SET_BLEND_FUNC_DFACTOR: Method = method(0x0318);
    pub const SET_BLEND_COLOR: Method = method(0x031c);
    pub const SET_BLEND_EQUATION: Method = method(0x0320);
    pub const SET_COLOR_MASK: Method = method(0x0324);
    pub const SET_STENCIL_TEST_ENABLE: Method = method(0x0328);
    pub const SET_STENCIL_MASK: Method = method(0x032c);
    pub const SET_STENCIL_FUNC: Method = method(0x0330);
    pub const SET_STENCIL_FUNC_REF: Method = method(0x0334);
    pub const SET_STENCIL_FUNC_MASK: Method = method(0x0338);
    pub const SET_STENCIL_OP_FAIL: Method = method(0x033c);
    pub const SET_STENCIL_OP_ZFAIL: Method = method(0x0340);
    pub const SET_STENCIL_OP_ZPASS: Method = method(0x0344);
    pub const SET_TWO_SIDED_STENCIL_TEST_ENABLE: Method = method(0x0348);
    pub const SET_BACK_STENCIL_MASK: Method = method(0x034c);
    pub const SET_BACK_STENCIL_FUNC: Method = method(0x0350);
    pub const SET_BACK_STENCIL_FUNC_REF: Method = method(0x0354);
    pub const SET_BACK_STENCIL_FUNC_MASK: Method = method(0x0358);
    pub const SET_BACK_STENCIL_OP_FAIL: Method = method(0x035c);
    pub const SET_BACK_STENCIL_OP_ZFAIL: Method = method(0x0360);
    pub const SET_BACK_STENCIL_OP_ZPASS: Method = method(0x0364);
    pub const SET_SHADE_MODE: Method = method(0x0368);
    pub const SET_BLEND_ENABLE_MRT: Method = method(0x036c);
    pub const SET_COLOR_MASK_MRT: Method = method(0x0370);
    pub const SET_LOGIC_OP_ENABLE: Method = method(0x0374);
    pub const SET_LOGIC_OP: Method = method(0x0378);
    pub const SET_BLEND_COLOR2: Method = method(0x037c);
    pub const SET_DEPTH_BOUNDS_TEST_ENABLE: Method = method(0x0380);
    pub const SET_DEPTH_BOUNDS_MIN: Method = method(0x0384);
    pub const SET_DEPTH_BOUNDS_MAX: Method = method(0x0388);
    pub const SET_CLIP_MIN: Method = method(0x0394);
    pub const SET_CLIP_MAX: Method = method(0x0398);
    pub const SET_CONTROL0: Method = method(0x03b0);
    pub const SET_LINE_WIDTH: Method = method(0x03b8);
    pub const SET_LINE_SMOOTH_ENABLE: Method = method(0x03bc);
    pub const SET_ANISO_SPREAD: Method = method(0x03c0);
    pub const SET_SCISSOR_HORIZONTAL: Method = method(0x08c0);
    pub const SET_SCISSOR_VERTICAL: Method = method(0x08c4);
    pub const SET_FOG_MODE: Method = method(0x08cc);
    pub const SET_FOG_PARAMS: Method = method(0x08d0);
    pub const SET_SHADER_PROGRAM: Method = method(0x08e4);
    pub const SET_VERTEX_TEXTURE_OFFSET: Method = method(0x0900);
    pub const SET_VERTEX_TEXTURE_FORMAT: Method = method(0x0904);
    pub const SET_VERTEX_TEXTURE_ADDRESS: Method = method(0x0908);
    pub const SET_VERTEX_TEXTURE_CONTROL0: Method = method(0x090c);
    pub const SET_VERTEX_TEXTURE_CONTROL3: Method = method(0x0910);
    pub const SET_VERTEX_TEXTURE_FILTER: Method = method(0x0914);
    pub const SET_VERTEX_TEXTURE_IMAGE_RECT: Method = method(0x0918);
    pub const SET_VERTEX_TEXTURE_BORDER_COLOR: Method = method(0x091c);
    pub const SET_VIEWPORT_HORIZONTAL: Method = method(0x0a00);
    pub const SET_VIEWPORT_VERTICAL: Method = method(0x0a04);
    pub const SET_POINT_CENTER_MODE: Method = method(0x0a0c);
    pub const ZCULL_SYNC: Method = method(0x0a1c);
    pub const SET_VIEWPORT_OFFSET: Method = method(0x0a20);
    pub const SET_VIEWPORT_SCALE: Method = method(0x0a30);
    pub const SET_POLY_OFFSET_POINT_ENABLE: Method = method(0x0a60);
    pub const SET_POLY_OFFSET_LINE_ENABLE: Method = method(0x0a64);
    pub const SET_POLY_OFFSET_FILL_ENABLE: Method = method(0x0a68);
    pub const SET_DEPTH_FUNC: Method = method(0x0a6c);
    pub const SET_DEPTH_MASK: Method = method(0x0a70);
    pub const SET_DEPTH_TEST_ENABLE: Method = method(0x0a74);
    pub const SET_POLYGON_OFFSET_SCALE_FACTOR: Method = method(0x0a78);
    pub const SET_POLYGON_OFFSET_BIAS: Method = method(0x0a7c);
    pub const SET_VERTEX_DATA_SCALED4S_M: Method = method(0x0a80);
    pub const SET_TEXTURE_CONTROL2: Method = method(0x0b00);
    pub const SET_TEX_COORD_CONTROL: Method = method(0x0b40);
    pub const SET_TRANSFORM_PROGRAM: Method = method(0x0b80);
    pub const SET_SPECULAR_ENABLE: Method = method(0x1428);
    pub const SET_TWO_SIDE_LIGHT_EN: Method = method(0x142c);
    pub const CLEAR_ZCULL_SURFACE: Method = method(0x1438);
    pub const SET_PERFORMANCE_PARAMS: Method = method(0x1450);
    pub const SET_FLAT_SHADE_OP: Method = method(0x1454);
    pub const SET_EDGE_FLAG: Method = method(0x145c);
    pub const SET_USER_CLIP_PLANE_CONTROL: Method = method(0x1478);
    pub const SET_POLYGON_STIPPLE: Method = method(0x147c);
    pub const SET_POLYGON_STIPPLE_PATTERN: Method = method(0x1480);
    pub const SET_VERTEX_DATA3F_M: Method = method(0x1500);
    pub const SET_VERTEX_DATA_ARRAY_OFFSET: Method = method(0x1680);
    pub const INVALIDATE_VERTEX_CACHE_FILE: Method = method(0x1710);
    pub const INVALIDATE_VERTEX_FILE: Method = method(0x1714);
    pub const PIPE_NOP: Method = method(0x1718);
    pub const SET_VERTEX_DATA_BASE_OFFSET: Method = method(0x1738);
    pub const SET_VERTEX_DATA_BASE_INDEX: Method = method(0x173c);
    pub const SET_VERTEX_DATA_ARRAY_FORMAT: Method = method(0x1740);
    pub const CLEAR_REPORT_VALUE: Method = method(0x17c8);
    pub const SET_ZPASS_PIXEL_COUNT_ENABLE: Method = method(0x17cc);
    pub const GET_REPORT: Method = method(0x1800);
    pub const SET_ZCULL_STATS_ENABLE: Method = method(0x1804);
    pub const SET_BEGIN_END: Method = method(0x1808);
    pub const ARRAY_ELEMENT16: Method = method(0x180c);
    pub const ARRAY_ELEMENT32: Method = method(0x1810);
    pub const DRAW_ARRAYS: Method = method(0x1814);
    pub const INLINE_ARRAY: Method = method(0x1818);
    pub const SET_INDEX_ARRAY_ADDRESS: Method = method(0x181c);
    pub const SET_INDEX_ARRAY_DMA: Method = method(0x1820);
    pub const DRAW_INDEX_ARRAY: Method = method(0x1824);
    pub const SET_FRONT_POLYGON_MODE: Method = method(0x1828);
    pub const SET_BACK_POLYGON_MODE: Method = method(0x182c);
    pub const SET_CULL_FACE: Method = method(0x1830);
    pub const SET_FRONT_FACE: Method = method(0x1834);
    pub const SET_POLY_SMOOTH_ENABLE: Method = method(0x1838);
    pub const SET_CULL_FACE_ENABLE: Method = method(0x183c);
    pub const SET_TEXTURE_CONTROL3: Method = method(0x1840);
    pub const SET_VERTEX_DATA2F_M: Method = method(0x1880);
    pub const SET_VERTEX_DATA2S_M: Method = method(0x1900);
    pub const SET_VERTEX_DATA4UB_M: Method = method(0x1940);
    pub const SET_VERTEX_DATA4S_M: Method = method(0x1980);
    pub const SET_TEXTURE_OFFSET: Method = method(0x1a00);
    pub const SET_TEXTURE_FORMAT: Method = method(0x1a04);
    pub const SET_TEXTURE_ADDRESS: Method = method(0x1a08);
    pub const SET_TEXTURE_CONTROL0: Method = method(0x1a0c);
    pub const SET_TEXTURE_CONTROL1: Method = method(0x1a10);
    pub const SET_TEXTURE_FILTER: Method = method(0x1a14);
    pub const SET_TEXTURE_IMAGE_RECT: Method = method(0x1a18);
    pub const SET_TEXTURE_BORDER_COLOR: Method = method(0x1a1c);
    pub const SET_VERTEX_DATA4F_M: Method = method(0x1c00);
    pub const SET_COLOR_KEY_COLOR: Method = method(0x1d00);
    pub const SET_SHADER_CONTROL: Method = method(0x1d60);
    pub const SET_INDEXED_CONSTANT_READ_LIMITS: Method = method(0x1d64);
    pub const SET_SEMAPHORE_OFFSET: Method = method(0x1d6c);
    pub const BACK_END_WRITE_SEMAPHORE_RELEASE: Method = method(0x1d70);
    pub const TEXTURE_READ_SEMAPHORE_RELEASE: Method = method(0x1d74);
    pub const SET_ZMIN_MAX_CONTROL: Method = method(0x1d78);
    pub const SET_ANTI_ALIASING_CONTROL: Method = method(0x1d7c);
    pub const SET_SURFACE_COMPRESSION: Method = method(0x1d80);
    pub const SET_ZCULL_EN: Method = method(0x1d84);
    pub const SET_SHADER_WINDOW: Method = method(0x1d88);
    pub const SET_ZSTENCIL_CLEAR_VALUE: Method = method(0x1d8c);
    pub const SET_COLOR_CLEAR_VALUE: Method = method(0x1d90);
    pub const CLEAR_SURFACE: Method = method(0x1d94);
    pub const SET_CLEAR_RECT_HORIZONTAL: Method = method(0x1d98);
    pub const SET_CLEAR_RECT_VERTICAL: Method = method(0x1d9c);
    pub const SET_CLIP_ID_TEST_ENABLE: Method = method(0x1da4);
    pub const SET_RESTART_INDEX_ENABLE: Method = method(0x1dac);
    pub const SET_RESTART_INDEX: Method = method(0x1db0);
    pub const SET_LINE_STIPPLE: Method = method(0x1db4);
    pub const SET_LINE_STIPPLE_PATTERN: Method = method(0x1db8);
    pub const SET_VERTEX_DATA1F_M: Method = method(0x1e40);
    pub const SET_TRANSFORM_EXECUTION_MODE: Method = method(0x1e94);
    pub const SET_RENDER_ENABLE: Method = method(0x1e98);
    pub const SET_TRANSFORM_PROGRAM_LOAD: Method = method(0x1e9c);
    pub const SET_TRANSFORM_PROGRAM_START: Method = method(0x1ea0);
    pub const SET_ZCULL_CONTROL0: Method = method(0x1ea4);
    pub const SET_ZCULL_CONTROL1: Method = method(0x1ea8);
    pub const SET_SCULL_CONTROL: Method = method(0x1eac);
    pub const SET_POINT_SIZE: Method = method(0x1ee0);
    pub const SET_POINT_PARAMS_ENABLE: Method = method(0x1ee4);
    pub const SET_POINT_SPRITE_CONTROL: Method = method(0x1ee8);
    pub const SET_TRANSFORM_TIMEOUT: Method = method(0x1ef8);
    pub const SET_TRANSFORM_CONSTANT_LOAD: Method = method(0x1efc);
    pub const SET_TRANSFORM_CONSTANT: Method = method(0x1f00);
    pub const SET_FREQUENCY_DIVIDER_OPERATION: Method = method(0x1fc0);
    pub const SET_ATTRIB_COLOR: Method = method(0x1fc4);
    pub const SET_ATTRIB_TEX_COORD: Method = method(0x1fc8);
    pub const SET_ATTRIB_TEX_COORD_EX: Method = method(0x1fcc);
    pub const SET_ATTRIB_UCLIP0: Method = method(0x1fd0);
    pub const SET_ATTRIB_UCLIP1: Method = method(0x1fd4);
    pub const INVALIDATE_L2: Method = method(0x1fd8);
    pub const SET_REDUCE_DST_COLOR: Method = method(0x1fe0);
    pub const SET_NO_PARANOID_TEXTURE_FETCHES: Method = method(0x1fe8);
    pub const SET_SHADER_PACKER: Method = method(0x1fec);
    pub const SET_VERTEX_ATTRIB_INPUT_MASK: Method = method(0x1ff0);
    pub const SET_VERTEX_ATTRIB_OUTPUT_MASK: Method = method(0x1ff4);
    pub const SET_TRANSFORM_BRANCH_BITS: Method = method(0x1ff8);
}

/// Memory-to-memory transfers.
pub mod nv0039 {
    use super::method;
    use super::Method;

    pub const SET_OBJECT: Method = method(0x2000);
    pub const SET_CONTEXT_DMA_NOTIFIES: Method = method(0x2180);
    pub const SET_CONTEXT_DMA_BUFFER_IN: Method = method(0x2184);
    pub const SET_CONTEXT_DMA_BUFFER_OUT: Method = method(0x2188);
    pub const OFFSET_IN: Method = method(0x230c);
    pub const OFFSET_OUT: Method = method(0x2310);
    pub const PITCH_IN: Method = method(0x2314);
    pub const PITCH_OUT: Method = method(0x2318);
    pub const LINE_LENGTH_IN: Method = method(0x231c);
    pub const LINE_COUNT: Method = method(0x2320);
    pub const FORMAT: Method = method(0x2324);
    pub const BUFFER_NOTIFY: Method = method(0x2328);
}

/// Linear surfaces for 2D operations.
pub mod nv3062 {
    use super::method;
    use super::Method;

    pub const SET_OBJECT: Method = method(0x6000);
    pub const SET_CONTEXT_DMA_NOTIFIES: Method = method(0x6180);
    pub const SET_CONTEXT_DMA_IMAGE_SOURCE: Method = method(0x6184);
    pub const SET_CONTEXT_DMA_IMAGE_DESTIN: Method = method(0x6188);
    pub const SET_COLOR_FORMAT: Method = method(0x6300);
    pub const SET_PITCH: Method = method(0x6304);
    pub const SET_OFFSET_SOURCE: Method = method(0x6308);
    pub const SET_OFFSET_DESTIN: Method = method(0x630c);
}

/// Swizzled surfaces for 2D operations.
pub mod nv309e {
    use super::method;
    use super::Method;

    pub const SET_OBJECT: Method = method(0x8000);
    pub const SET_CONTEXT_DMA_NOTIFIES: Method = method(0x8180);
    pub const SET_CONTEXT_DMA_IMAGE: Method = method(0x8184);
    pub const SET_FORMAT: Method = method(0x8300);
    pub const SET_OFFSET: Method = method(0x8304);
}

/// Image transfers with inline color data.
pub mod nv308a {
    use super::method;
    use super::Method;

    pub const SET_OBJECT: Method = method(0xa000);
    pub const SET_CONTEXT_DMA_NOTIFIES: Method = method(0xa180);
    pub const SET_CONTEXT_COLOR_KEY: Method = method(0xa184);
    pub const SET_CONTEXT_CLIP_RECTANGLE: Method = method(0xa188);
    pub const SET_CONTEXT_PATTERN: Method = method(0xa18c);
    pub const SET_CONTEXT_ROP: Method = method(0xa190);
    pub const SET_CONTEXT_BETA1: Method = method(0xa194);
    pub const SET_CONTEXT_BETA4: Method = method(0xa198);
    pub const SET_CONTEXT_SURFACE: Method = method(0xa19c);
    pub const SET_COLOR_CONVERSION: Method = method(0xa2f8);
    pub const SET_OPERATION: Method = method(0xa2fc);
    pub const SET_COLOR_FORMAT: Method = method(0xa300);
    pub const POINT: Method = method(0xa304);
    pub const SIZE_OUT: Method = method(0xa308);
    pub const SIZE_IN: Method = method(0xa30c);
    pub const COLOR: Method = method(0xa400);
}

/// Image scaling and format conversion.
pub mod nv3089 {
    use super::method;
    use super::Method;

    pub const SET_OBJECT: Method = method(0xc000);
    pub const SET_CONTEXT_DMA_NOTIFIES: Method = method(0xc180);
    pub const SET_CONTEXT_DMA_IMAGE: Method = method(0xc184);
    pub const SET_CONTEXT_PATTERN: Method = method(0xc188);
    pub const SET_CONTEXT_ROP: Method = method(0xc18c);
    pub const SET_CONTEXT_BETA1: Method = method(0xc190);
    pub const SET_CONTEXT_BETA4: Method = method(0xc194);
    pub const SET_CONTEXT_SURFACE: Method = method(0xc198);
    pub const SET_COLOR_CONVERSION: Method = method(0xc2fc);
    pub const SET_COLOR_FORMAT: Method = method(0xc300);
    pub const SET_OPERATION: Method = method(0xc304);
    pub const CLIP_POINT: Method = method(0xc308);
    pub const CLIP_SIZE: Method = method(0xc30c);
    pub const IMAGE_OUT_POINT: Method = method(0xc310);
    pub const IMAGE_OUT_SIZE: Method = method(0xc314);
    pub const DS_DX: Method = method(0xc318);
    pub const DT_DY: Method = method(0xc31c);
    pub const IMAGE_IN_SIZE: Method = method(0xc400);
    pub const IMAGE_IN_FORMAT: Method = method(0xc404);
    pub const IMAGE_IN_OFFSET: Method = method(0xc408);
    pub const IMAGE_IN: Method = method(0xc40c);
}
