//! Shared GPU platform services for rasterization and computation.
//! Device creation, backend reports and byte transport live here; render and
//! compute pipeline policy belongs to their respective crates.

#![recursion_limit = "256"]

mod backend;
mod buffer;
mod context;
mod executor;
mod readback;
mod memory;
mod profiling;

pub use backend::{
    BackendKind, BackendReport, SubgroupReport, available_backend_report,
    available_subgroup_report, backend_label, tuned_workgroup_size,
};
pub use buffer::{pack_f32, pack_u32, read_buffer, storage_entry, uniform_entry};
pub use context::{GpuContext, GpuContextError};
pub use profiling::{GpuTimer, GpuTimestamp, TimestampError, TimestampReadback};
pub use executor::block_on;
pub use memory::{GpuBuffer, GpuBufferView, BufferError};
pub use readback::{ByteReadback, ReadbackError, try_read_buffer};
pub use wgpu;

#[cfg(feature = "cuda")]
pub mod cuda;
