//! Low-level PlayStation 3 Level 2 (LV2) Kernel bindings and types.

macro_rules! assert_layout {
    ($ty:ty, $size:literal, $align:literal $(, $field:ident = $offset:literal)* $(,)?) => {
        const _: () = {
            assert!(core::mem::size_of::<$ty>() == $size);
            assert!(core::mem::align_of::<$ty>() == $align);
            $(assert!(core::mem::offset_of!($ty, $field) == $offset);)*
        };
    };
}

pub mod entry;
pub mod error;
#[cfg(feature = "sdk-imports")]
pub mod sdk;
pub mod syscalls;
pub mod types;

pub use syscalls::*;
pub use types::*;
