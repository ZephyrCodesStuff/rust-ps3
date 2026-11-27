//! Typed PPU kernel entry points. SDK functions assembled from multiple kernel
//! operations belong in the higher-level API.
//!
//! Pointers passed as register arguments use ordinary Rust raw pointers.
//! Embedded addresses in LV2 structures use the documented 32- or 64-bit
//! fields. All structures use the native big-endian PPU representation.
//!
//! Each entry follows the public selector and register layout selected by the
//! crate's `sdk-370` or `sdk-192` feature.

macro_rules! assert_layout {
    ($ty:ty, $size:literal, $align:literal $(, $field:ident = $offset:literal)* $(,)?) => {
        const _: () = {
            assert!(core::mem::size_of::<$ty>() == $size);
            assert!(core::mem::align_of::<$ty>() == $align);
            $(assert!(core::mem::offset_of!($ty, $field) == $offset);)*
        };
    };
}

mod debug;
mod fs;
mod memory;
mod overlay;
mod platform;
mod process;
mod prx;
mod security;
mod spu;
mod sync;
mod thread;
mod time;

pub use debug::*;
pub use fs::*;
pub use memory::*;
pub use overlay::*;
pub use platform::*;
pub use process::*;
pub(crate) use prx::{_sys_prx_start_module, _sys_prx_stop_module, SysPrxStartStopModuleOption};
pub use security::*;
pub use spu::*;
pub use sync::*;
pub use thread::*;
pub use time::*;
