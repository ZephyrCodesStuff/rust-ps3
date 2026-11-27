//! The PlayStation 3 RSX graphics interface.
//!
//! The `rsx` feature links the firmware's GCM imports. Use `moldier patch`
//! to finalize the executable's import table.

mod commands;
mod context;
mod draw;
mod memory;
mod pipeline;
pub mod raw;
pub mod shader;
mod sync;
mod texture;
mod transfer;
pub mod types;
pub mod video;

pub use commands::ClearMask;
pub use context::{Context, Error, FlipMode, Method};
pub use draw::{IndexType, Location, Primitive, Rect, VertexAttribute, VertexType, Viewport};
pub use memory::{Buffer, IoBuffer, LocalArena};
pub use pipeline::{
    Blend, BlendChannel, BlendEquation, BlendFactor, Compare, CullFace, FaceWinding, PolygonMode,
    StencilAction, StencilFace,
};
pub use sync::{Label, LabelStage, ReportKind};
pub use texture::{MagFilter, MinFilter, Sampler, WrapMode};
pub use transfer::TransferMemory;
pub use types::*;
