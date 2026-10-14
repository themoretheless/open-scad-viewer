//! Independent polygon/triangle-mesh algorithms in Rust.
//! Does not depend on NURBS, Manifold, C/C++, WASM or the application.
//! Coordinates in this host contract are millimeters; algorithms use binary64.
//!
//! Public layout:
//! - [`solid`] — triangle meshes
//! - [`appearance`] — paint / style (not shape)
//!
//! 2D paths/rings live in `planar-geometry`. Print planning lives in `slicer-core`.
#![feature(
    try_blocks,
    gen_blocks,
    yield_expr,
    super_let,
    deref_patterns,
    yeet_expr
)]
#![allow(unused_features)]
pub mod appearance;
pub mod gradient_mesh;
#[cfg(feature = "cuda")]
mod lattice_cuda;
#[cfg(feature = "gpu")]
pub mod lattice_gpu;
pub mod lattice_tools;
pub mod local_blend;
pub mod mesh_editor;
pub mod mesh_export;
pub mod mesh_shell;
mod mesh_topology;
pub mod model_3mf;
pub mod package_3mf;
pub mod profile_tools;
pub mod scene_flatten;
#[cfg(feature = "codec")]
#[path = "serialization.rs"]
mod serialization;
pub mod solid;

pub const MAX_TRIANGLES: usize = 20_000;
pub use ::mesh_topology::{MAX_MESH_TRIANGLES, MAX_VERTICES};
pub use math_core::{Error, Result};
pub(crate) fn mesh_error(err: Error) -> Error {
    match err.code {
        "MESH_INVALID_INPUT"
        | "MESH_QUERY_INVALID_INPUT"
        | "MESH_SECTION_INVALID_INPUT"
        | "MESH_IO_INVALID_INPUT" => error(
            err.message
                .replace("the mesh resource budget", "the polygon resource budget"),
        ),
        _ => err,
    }
}
pub(crate) const INVALID_INPUT: &str = "POLYGON_INVALID_INPUT";
pub(crate) fn error(message: impl Into<String>) -> Error {
    Error::new(INVALID_INPUT, message)
}
pub(crate) fn check(condition: bool, message: &str) -> Result<()> {
    math_core::ensure(condition, INVALID_INPUT, message)
}
pub(crate) use math_core::{cross, dot, norm, scale, sub};
/// Common owned exchange format: independent buffers, no kernel pointers.
#[derive(Debug, Clone)]
pub struct Mesh {
    pub positions: Vec<f64>,
    pub indices: Vec<usize>,
    pub uv: Option<Vec<f64>>,
}
impl From<geometry_ops::Triangles> for Mesh {
    fn from(t: geometry_ops::Triangles) -> Self {
        Self {
            positions: t.positions,
            indices: t.indices,
            uv: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Seams {
    pub u: bool,
    pub v: bool,
}

#[derive(Debug, Clone)]
pub enum Construction {
    TriangleMesh,
    Boolean,
    SampledSurface,
    FixedVectorThickening,
}

#[derive(Debug, Clone)]
pub struct MeshDiagnosticLocations {
    pub boundary_edges: Vec<[usize; 2]>,
    pub non_manifold_edges: Vec<[usize; 2]>,
    pub orientation_edges: Vec<[usize; 2]>,
    pub degenerate_triangles: Vec<usize>,
}
#[derive(Debug, Clone)]
pub struct Report {
    pub boolean: Option<solid::boolean::BooleanReport>,
    pub triangle_count: usize,
    pub vertex_count: usize,
    pub boundary_edges: usize,
    pub non_manifold_edges: usize,
    pub orientation_conflicts: usize,
    pub degenerate_triangles: usize,
    pub closed: bool,
    pub signed_volume_mm3: f64,
    pub error_bound_certified: bool,
    pub self_intersection_status: String,
    pub construction: Construction,
    pub uv_area: Option<f64>,
    pub sampled_deviation_mm: Option<f64>,
    pub parameter_seams_welded: Option<Seams>,
    pub collapsed_boundary_count: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct BuiltMesh {
    pub mesh: Mesh,
    pub report: Report,
}

impl Mesh {
    /// Borrow the independent kernels' neutral buffer view without copying.
    pub fn view(&self) -> ::mesh_topology::MeshView<'_> {
        ::mesh_topology::MeshView {
            positions: &self.positions,
            indices: &self.indices,
            uv: self.uv.as_deref(),
        }
    }
    pub fn validate(&self) -> Result<()> {
        self.view().validate().map_err(mesh_error)
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
    /// Exact indexed locations, using the same predicates as inspect().
    /// Boundary edges remain available even when they cannot form ordered loops.
    pub fn diagnostic_locations(&self) -> Result<MeshDiagnosticLocations> {
        self.validate()?;
        let edges = mesh_topology::EdgeUses::new(&self.indices);
        let (non_manifold_edges, orientation_edges) = edges.defects();
        let mut degenerate_triangles = Vec::new();
        for (i, t) in self.indices.as_chunks::<3>().0.iter().enumerate() {
            let a = self.point(t[0])?;
            let ab = sub(self.point(t[1])?, a);
            let ac = sub(self.point(t[2])?, a);
            if norm(cross(ab, ac)) <= f64::EPSILON * (norm(ab) * norm(ac)).max(1.) {
                degenerate_triangles.push(i);
            }
        }
        Ok(MeshDiagnosticLocations {
            boundary_edges: edges.boundary().collect(),
            non_manifold_edges,
            orientation_edges,
            degenerate_triangles,
        })
    }
    pub fn inspect(&self) -> Result<Report> {
        self.inspect_with_edges().map(|(report, _)| report)
    }
    fn inspect_with_edges(&self) -> Result<(Report, mesh_topology::EdgeUses)> {
        let (report, edges) = self.view().inspect_with_edges().map_err(mesh_error)?;
        Ok((
            Report {
                boolean: None,
                triangle_count: report.triangle_count,
                vertex_count: report.vertex_count,
                boundary_edges: report.boundary_edges,
                non_manifold_edges: report.non_manifold_edges,
                orientation_conflicts: report.orientation_conflicts,
                degenerate_triangles: report.degenerate_triangles,
                closed: report.closed,
                signed_volume_mm3: report.signed_volume_mm3,
                error_bound_certified: false,
                self_intersection_status: "not_checked".into(),
                construction: Construction::TriangleMesh,
                uv_area: None,
                sampled_deviation_mm: None,
                parameter_seams_welded: None,
                collapsed_boundary_count: None,
            },
            edges,
        ))
    }
    /// Ordered directed boundary loops, with ambiguous branches rejected.
    pub fn boundary_loops(&self) -> Result<Vec<Vec<usize>>> {
        self.view().boundary_loops().map_err(mesh_error)
    }
    /// Affine transformation; reflection also reverses face winding. UVs still
    /// identify source parameters, but do not assert equality to a source surface.
    pub fn transform(&self, m: [[f64; 4]; 4]) -> Result<Self> {
        self.validate()?;
        check(
            m.iter().flatten().all(|v| v.is_finite()) && m[3] == [0., 0., 0., 1.],
            "Mesh transform must be a finite affine matrix.",
        )?;
        let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
        check(
            det.is_finite() && det != 0.,
            "Mesh transform must be nonsingular.",
        )?;
        let mut result = self.clone();
        for p in result.positions.as_chunks_mut::<3>().0 {
            let q = [p[0], p[1], p[2]];
            for i in 0..3 {
                p[i] = m[i][3] + (0..3).map(|j| m[i][j] * q[j]).sum::<f64>();
            }
        }
        if det < 0. {
            result.reverse_winding();
        }
        result.validate()?;
        Ok(result)
    }
    /// Finite projective map on each triangle. Crossing W=0 is refused.
    /// UVs retain source parameters; they do not describe transformed surfaces.
    pub fn transform_projective(&self, matrix: [[f64; 4]; 4]) -> Result<Self> {
        self.validate()?;
        let reversed = math_core::projective::orientation(matrix)
            .ok_or_else(|| error("Projective mesh matrix must be finite and nonsingular"))?;
        let projected = self
            .positions
            .chunks_exact(3)
            .map(|p| {
                math_core::projective::point(matrix, [p[0], p[1], p[2]])
                    .map_err(|_| error("Projective mesh vertex is non-finite or lies on W=0"))
            })
            .collect::<Result<Vec<_>>>()?;
        for triangle in self.indices.chunks_exact(3) {
            math_core::projective::triangle_domain(std::array::from_fn(|i| {
                projected[triangle[i] as usize]
            }))
            .map_err(|_| error("Projective mesh triangle crosses W=0"))?;
        }
        let mut result = self.clone();
        result.positions = projected.iter().flat_map(|p| p.position).collect();
        if reversed {
            result.reverse_winding();
        }
        result.validate()?;
        Ok(result)
    }
    pub fn reverse_winding(&mut self) {
        for t in self.indices.as_chunks_mut::<3>().0 {
            t.swap(1, 2);
        }
    }
    pub fn thicken(&self, vector: [f64; 3]) -> Result<BuiltMesh> {
        check(
            vector.iter().all(|v| v.is_finite()) && norm(vector) > 0.,
            "Thickening vector must be finite and nonzero.",
        )?;
        let (source, edges) = self.inspect_with_edges()?;
        check(
            source.non_manifold_edges == 0
                && source.orientation_conflicts == 0
                && source.degenerate_triangles == 0
                && source.boundary_edges > 0,
            "Thickening requires a consistently oriented open surface mesh with a boundary.",
        )?;
        check(
            source.triangle_count * 2 + source.boundary_edges * 2 <= MAX_TRIANGLES,
            "Thickened mesh exceeds 20000 triangles; reduce surface segment counts.",
        )?;
        let count = self.positions.len() / 3;
        let mut positions = self.positions.clone();
        positions.extend(
            self.positions
                .iter()
                .enumerate()
                .map(|(i, v)| v + vector[i % 3]),
        );
        let mut indices = Vec::new();
        for t in self.indices.as_chunks::<3>().0 {
            let [a, b, c] = [t[0], t[1], t[2]];
            indices.extend([a, c, b, a + count, b + count, c + count]);
        }
        for [a, b] in edges.boundary() {
            indices.extend([a, b, b + count, a, b + count, a + count]);
        }
        drop(edges);
        // New wall vertices are not samples of the original surface: drop UVs.
        let mut mesh = Self {
            positions,
            indices,
            uv: None,
        };
        let mut report = mesh.inspect()?;
        if report.signed_volume_mm3 < 0. {
            mesh.reverse_winding();
            report = mesh.inspect()?;
        }
        check(
            report.closed && report.signed_volume_mm3 > 0.,
            "Thickening produced degenerate or open topology; the vector may be tangent to the surface.",
        )?;
        report.construction = Construction::FixedVectorThickening;
        Ok(BuiltMesh { mesh, report })
    }
    pub fn export_stl(&self) -> Result<String> {
        mesh_io::export_stl(&self.view()).map_err(mesh_error)
    }
}

#[cfg(test)]
mod tests;
