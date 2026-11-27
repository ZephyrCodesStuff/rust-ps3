//! Public SDK functions provided by the firmware's user-mode libraries.
//!
//! Enable the `sdk-imports` Cargo feature and link the application's PRX
//! import table when building its executable. These low-level declarations
//! expect the pointers, buffer sizes, object state and permissions required by
//! the corresponding SDK function. Embedded pointers use 32-bit effective
//! addresses. Pointer arguments use Rust raw pointers.

#[cfg(feature = "sdk-debug")]
pub mod debug;
pub mod memory;
pub mod process;
pub mod prx;
pub mod spu;
pub mod sync;
pub mod thread;
