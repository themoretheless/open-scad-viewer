//! Independent polygon/triangle-mesh algorithms in Rust.
//! Does not depend on NURBS, Manifold, C/C++, WASM or the application.
//! Coordinates in this host contract are millimeters; algorithms use binary64.
pub mod boolean;
pub mod brep;
pub mod cad;
pub mod edit;
pub mod modeling;
pub mod proximity;
pub mod tessellation;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

pub const MAX_TRIANGLES: usize = 20_000;
pub const MAX_MESH_TRIANGLES: usize = 100_000;
pub const MAX_VERTICES: usize = 300_000;
#[derive(Debug, Clone, Serialize)]
pub struct Error {
    pub code: &'static str,
    pub message: String,
}
impl Error {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            code: "POLYGON_INVALID_INPUT",
            message: message.into(),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;
pub(crate) fn check(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(Error::new(message))
    }
}
pub(crate) fn norm(v: &[f64]) -> f64 {
    v.iter().fold(0_f64, |n, x| n.hypot(*x))
}
pub(crate) fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub(crate) fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
/// Common owned exchange format: independent buffers, no kernel pointers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mesh {
    pub positions: Vec<f64>,
    pub indices: Vec<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uv: Option<Vec<f64>>,
}
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Seams {
    pub u: bool,
    pub v: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Construction {
    TriangleMesh,
    Boolean,
    SampledSurface,
    FixedVectorThickening,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boolean: Option<boolean::BooleanReport>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uv_area: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampled_deviation_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_seams_welded: Option<Seams>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collapsed_boundary_count: Option<usize>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltMesh {
    #[serde(flatten)]
    pub mesh: Mesh,
    pub report: Report,
}
impl Mesh {
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
            "Mesh exceeds the polygon resource budget.",
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
    fn edges(&self) -> BTreeMap<(usize, usize), Vec<[usize; 2]>> {
        let mut edges = BTreeMap::<_, Vec<_>>::new();
        for t in self.indices.chunks_exact(3) {
            for k in 0..3 {
                let a = t[k];
                let b = t[(k + 1) % 3];
                edges.entry((a.min(b), a.max(b))).or_default().push([a, b]);
            }
        }
        edges
    }
    pub fn inspect(&self) -> Result<Report> {
        self.validate()?;
        let reference = if self.positions.is_empty() {
            [0.; 3]
        } else {
            self.point(0)?
        };
        let mut volume = 0.;
        let mut volume_compensation = 0.;
        let mut degenerate = 0;
        for t in self.indices.chunks_exact(3) {
            let a = self.point(t[0])?;
            let b = self.point(t[1])?;
            let c = self.point(t[2])?;
            let ab = sub(b, a);
            let ac = sub(c, a);
            let normal = cross(ab, ac);
            if norm(&normal) <= f64::EPSILON * (norm(&ab) * norm(&ac)).max(1.) {
                degenerate += 1;
            }
            let ar = sub(a, reference);
            let br = sub(b, reference);
            let cr = sub(c, reference);
            let bc = cross(br, cr);
            let term = ar[0] * bc[0] + ar[1] * bc[1] + ar[2] * bc[2] - volume_compensation;
            let next = volume + term;
            volume_compensation = (next - volume) - term;
            volume = next;
        }
        check(
            volume.is_finite(),
            "Mesh volume exceeded finite numeric bounds.",
        )?;
        let edges = self.edges();
        let boundary = edges.values().filter(|u| u.len() == 1).count();
        let non_manifold = edges.values().filter(|u| u.len() > 2).count();
        let orientation = edges
            .values()
            .filter(|u| u.len() == 2 && u[0][0] == u[1][0])
            .count();
        Ok(Report {
            boolean: None,
            triangle_count: self.indices.len() / 3,
            vertex_count: self.positions.len() / 3,
            boundary_edges: boundary,
            non_manifold_edges: non_manifold,
            orientation_conflicts: orientation,
            degenerate_triangles: degenerate,
            closed: !self.indices.is_empty()
                && boundary == 0
                && non_manifold == 0
                && orientation == 0
                && degenerate == 0,
            signed_volume_mm3: volume / 6.,
            error_bound_certified: false,
            self_intersection_status: "not_checked".into(),
            construction: Construction::TriangleMesh,
            uv_area: None,
            sampled_deviation_mm: None,
            parameter_seams_welded: None,
            collapsed_boundary_count: None,
        })
    }
    /// Ordered boundary loops, each repeating its first vertex at the end.
    /// Ambiguous nonmanifold/branching boundaries are rejected, not guessed.
    pub fn boundary_loops(&self) -> Result<Vec<Vec<usize>>> {
        let report = self.inspect()?;
        check(
            report.non_manifold_edges == 0
                && report.orientation_conflicts == 0
                && report.degenerate_triangles == 0,
            "Boundary loops require a consistently oriented manifold mesh.",
        )?;
        let mut next = BTreeMap::new();
        let mut incoming = BTreeSet::new();
        for uses in self.edges().values().filter(|u| u.len() == 1) {
            let [a, b] = uses[0];
            check(
                next.insert(a, b).is_none() && incoming.insert(b),
                "Boundary loops branch at a vertex.",
            )?;
        }
        check(
            next.keys().all(|k| incoming.contains(k)),
            "Boundary chain does not close.",
        )?;
        let mut loops = Vec::new();
        while let Some((&start, _)) = next.first_key_value() {
            let mut path = vec![start];
            let mut current = start;
            loop {
                let target = next
                    .remove(&current)
                    .ok_or_else(|| Error::new("Boundary chain does not close."))?;
                path.push(target);
                if target == start {
                    break;
                }
                current = target;
            }
            loops.push(path);
        }
        Ok(loops)
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
        for p in result.positions.chunks_exact_mut(3) {
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
    pub fn reverse_winding(&mut self) {
        for t in self.indices.chunks_exact_mut(3) {
            t.swap(1, 2);
        }
    }
    pub fn thicken(&self, vector: [f64; 3]) -> Result<BuiltMesh> {
        check(
            vector.iter().all(|v| v.is_finite()) && norm(&vector) > 0.,
            "Thickening vector must be finite and nonzero.",
        )?;
        let source = self.inspect()?;
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
        for t in self.indices.chunks_exact(3) {
            let [a, b, c] = [t[0], t[1], t[2]];
            indices.extend([a, c, b, a + count, b + count, c + count]);
        }
        for uses in self.edges().values().filter(|u| u.len() == 1) {
            let [a, b] = uses[0];
            indices.extend([a, b, b + count, a, b + count, a + count]);
        }
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
        check(report.closed && report.signed_volume_mm3>0.,"Thickening produced degenerate or open topology; the vector may be tangent to the surface.")?;
        report.construction = Construction::FixedVectorThickening;
        Ok(BuiltMesh { mesh, report })
    }
    pub fn export_stl(&self) -> Result<String> {
        let report = self.inspect()?;
        check(
            report.closed && report.signed_volume_mm3 > 0.,
            "STL export requires a closed, consistently oriented mesh with positive volume.",
        )?;
        let mut output = String::from("solid modelgraph_nurbs_sampled\n");
        for t in self.indices.chunks_exact(3) {
            let a = self.point(t[0])?;
            let b = self.point(t[1])?;
            let c = self.point(t[2])?;
            let normal = cross(sub(b, a), sub(c, a));
            let len = norm(&normal);
            writeln!(
                output,
                "  facet normal {} {} {}\n    outer loop",
                normal[0] / len,
                normal[1] / len,
                normal[2] / len
            )
            .unwrap();
            for p in [a, b, c] {
                writeln!(output, "      vertex {} {} {}", p[0], p[1], p[2]).unwrap();
            }
            output.push_str("    endloop\n  endfacet\n");
            check(output.len() <= 4 * 1024 * 1024, "STL export exceeds 4 MiB.")?;
        }
        output.push_str("endsolid modelgraph_nurbs_sampled\n");
        Ok(output)
    }
}

#[cfg(test)]
mod tests;
