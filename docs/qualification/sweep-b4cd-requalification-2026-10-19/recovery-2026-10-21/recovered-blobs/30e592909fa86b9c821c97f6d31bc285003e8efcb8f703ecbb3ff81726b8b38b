use crate::{MAX_MESH_TRIANGLES, MAX_VERTICES, Result, check};

/// Borrowed binary64 xyz/triangle buffers. Methods validate before traversal.
/// UVs are optional and validated, but do not participate in topology.
#[derive(Debug, Clone, Copy)]
pub struct MeshView<'a> {
    pub positions: &'a [f64],
    pub indices: &'a [usize],
    pub uv: Option<&'a [f64]>,
}
impl<'a> MeshView<'a> {
    pub fn new(positions: &'a [f64], indices: &'a [usize]) -> Self {
        Self {
            positions,
            indices,
            uv: None,
        }
    }
    pub fn validate(&self) -> Result<()> {
        check(
            self.positions.len().is_multiple_of(3)
                && self.indices.len().is_multiple_of(3)
                && self.positions.iter().all(|v| v.is_finite())
                && self.indices.iter().all(|i| *i < self.positions.len() / 3),
            "Malformed triangle mesh.",
        )?;
        check(
            self.indices.len() / 3 <= MAX_MESH_TRIANGLES
                && self.positions.len() / 3 <= MAX_VERTICES,
            "Mesh exceeds the mesh resource budget.",
        )?;
        if let Some(uv) = &self.uv {
            check(
                uv.len() == self.positions.len() / 3 * 2 && uv.iter().all(|v| v.is_finite()),
                "Malformed mesh UV coordinates.",
            )?;
        }
        Ok(())
    }
    pub fn point(&self, index: usize) -> Result<[f64; 3]> {
        check(
            index < self.positions.len() / 3,
            "Vertex index is outside the mesh.",
        )?;
        Ok([
            self.positions[3 * index],
            self.positions[3 * index + 1],
            self.positions[3 * index + 2],
        ])
    }
}
