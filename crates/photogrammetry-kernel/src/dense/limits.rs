//! Shared element-count contract between volume extraction and simplification.
//! These caps do not bound allocator overhead or whole-process peak memory.
pub(super) const MAX_SURFACE_VERTICES: usize = 500_000;
pub(super) const MAX_SURFACE_TRIANGLES: usize = 1_000_000;
