//! The PlayStation 3 RSX graphics interface.
//!
//! The `rsx` feature links the firmware's GCM imports. Use `moldier patch`
//! to finalize the executable's import table.

mod commands;
mod context;
mod display;
mod draw;
mod event;
pub mod gtf;
mod memory;
pub mod methods;
mod pipeline;
pub mod raw;
pub mod shader;
mod sync;
mod texture;
mod tiled;
mod transfer;
pub mod types;
pub mod video;

pub use commands::ClearMask;
pub use context::{Context, Error, FlipMode, Method};
pub use display::{Framebuffer, Swapchain};
pub use draw::{
    ImmediateVertex, IndexType, Location, Primitive, Rect, VertexAttribute, VertexType, Viewport,
};
pub use event::{Event, EventCallback};
pub use memory::{Buffer, IoBuffer, LocalArena};
pub use pipeline::{
    Blend, BlendChannel, BlendEquation, BlendFactor, ClipPlane, Compare, CullFace, FaceWinding,
    LogicOp, PolygonMode, ShadeModel, StencilAction, StencilFace,
};
pub use sync::{Label, LabelStage, ReportKind};
pub use texture::{MagFilter, MinFilter, Sampler, WrapMode};
pub use tiled::{
    Antialias, DepthFormat, Tile, TileCompression, ZCull, ZCullDirection, ZCullFormat,
};
pub use transfer::{ImageCopyTarget, ImagePixelSize, ScaleTarget, TransferMemory};
pub use types::*;
