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
pub mod solid;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

pub const MAX_TRIANGLES: usize = 20_000;
pub const MAX_MESH_TRIANGLES: usize = 100_000;
pub const MAX_VERTICES: usize = 300_000;
pub use math_core::{Error, Result};
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
impl value_codec::Serialize for Mesh {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "positions".into(),
            value_codec::Serialize::to_value(&self.positions),
        );
        object.insert(
            "indices".into(),
            value_codec::Serialize::to_value(&self.indices),
        );
        if self.uv.is_some() {
            object.insert("uv".into(), value_codec::Serialize::to_value(&self.uv));
        }
        value_codec::Value::Object(object)
    }
}
impl<'de> value_codec::Deserialize<'de> for Mesh {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let positions: Vec<f64> = value_codec::Deserialize::from_value(
            object
                .remove("positions")
                .ok_or_else(|| value_codec::error("Missing field positions"))?,
        )?;
        let indices: Vec<usize> = value_codec::Deserialize::from_value(
            object
                .remove("indices")
                .ok_or_else(|| value_codec::error("Missing field indices"))?,
        )?;
        let uv: Option<Vec<f64>> = if let Some(v) = object.remove("uv") {
            value_codec::Deserialize::from_value(v)?
        } else {
            Default::default()
        };
        Ok(Self {
            positions,
            indices,
            uv,
        })
    }
}
#[derive(Debug, Clone, Copy, Default)]
pub struct Seams {
    pub u: bool,
    pub v: bool,
}
impl value_codec::Serialize for Seams {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert("u".into(), value_codec::Serialize::to_value(&self.u));
        object.insert("v".into(), value_codec::Serialize::to_value(&self.v));
        value_codec::Value::Object(object)
    }
}
impl<'de> value_codec::Deserialize<'de> for Seams {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let u: bool = value_codec::Deserialize::from_value(
            object
                .remove("u")
                .ok_or_else(|| value_codec::error("Missing field u"))?,
        )?;
        let v: bool = value_codec::Deserialize::from_value(
            object
                .remove("v")
                .ok_or_else(|| value_codec::error("Missing field v"))?,
        )?;
        Ok(Self { u, v })
    }
}
#[derive(Debug, Clone)]
pub enum Construction {
    TriangleMesh,
    Boolean,
    SampledSurface,
    FixedVectorThickening,
}
impl value_codec::Serialize for Construction {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::TriangleMesh => value_codec::Value::String("triangle_mesh".into()),
            Self::Boolean => value_codec::Value::String("boolean".into()),
            Self::SampledSurface => value_codec::Value::String("sampled_surface".into()),
            Self::FixedVectorThickening => {
                value_codec::Value::String("fixed_vector_thickening".into())
            }
        }
    }
}
impl<'de> value_codec::Deserialize<'de> for Construction {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        match value.as_str().unwrap_or("") {
            "triangle_mesh" => Ok(Self::TriangleMesh),
            "boolean" => Ok(Self::Boolean),
            "sampled_surface" => Ok(Self::SampledSurface),
            "fixed_vector_thickening" => Ok(Self::FixedVectorThickening),
            _ => Err(value_codec::error("Unknown enum variant")),
        }
    }
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
impl value_codec::Serialize for Report {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        if self.boolean.is_some() {
            object.insert(
                "boolean".into(),
                value_codec::Serialize::to_value(&self.boolean),
            );
        }
        object.insert(
            "triangleCount".into(),
            value_codec::Serialize::to_value(&self.triangle_count),
        );
        object.insert(
            "vertexCount".into(),
            value_codec::Serialize::to_value(&self.vertex_count),
        );
        object.insert(
            "boundaryEdges".into(),
            value_codec::Serialize::to_value(&self.boundary_edges),
        );
        object.insert(
            "nonManifoldEdges".into(),
            value_codec::Serialize::to_value(&self.non_manifold_edges),
        );
        object.insert(
            "orientationConflicts".into(),
            value_codec::Serialize::to_value(&self.orientation_conflicts),
        );
        object.insert(
            "degenerateTriangles".into(),
            value_codec::Serialize::to_value(&self.degenerate_triangles),
        );
        object.insert(
            "closed".into(),
            value_codec::Serialize::to_value(&self.closed),
        );
        object.insert(
            "signedVolumeMm3".into(),
            value_codec::Serialize::to_value(&self.signed_volume_mm3),
        );
        object.insert(
            "errorBoundCertified".into(),
            value_codec::Serialize::to_value(&self.error_bound_certified),
        );
        object.insert(
            "selfIntersectionStatus".into(),
            value_codec::Serialize::to_value(&self.self_intersection_status),
        );
        object.insert(
            "construction".into(),
            value_codec::Serialize::to_value(&self.construction),
        );
        if self.uv_area.is_some() {
            object.insert(
                "uvArea".into(),
                value_codec::Serialize::to_value(&self.uv_area),
            );
        }
        if self.sampled_deviation_mm.is_some() {
            object.insert(
                "sampledDeviationMm".into(),
                value_codec::Serialize::to_value(&self.sampled_deviation_mm),
            );
        }
        if self.parameter_seams_welded.is_some() {
            object.insert(
                "parameterSeamsWelded".into(),
                value_codec::Serialize::to_value(&self.parameter_seams_welded),
            );
        }
        if self.collapsed_boundary_count.is_some() {
            object.insert(
                "collapsedBoundaryCount".into(),
                value_codec::Serialize::to_value(&self.collapsed_boundary_count),
            );
        }
        value_codec::Value::Object(object)
    }
}
impl<'de> value_codec::Deserialize<'de> for Report {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let boolean: Option<solid::boolean::BooleanReport> =
            if let Some(v) = object.remove("boolean") {
                value_codec::Deserialize::from_value(v)?
            } else {
                Default::default()
            };
        let triangle_count: usize = value_codec::Deserialize::from_value(
            object
                .remove("triangleCount")
                .ok_or_else(|| value_codec::error("Missing field triangleCount"))?,
        )?;
        let vertex_count: usize = value_codec::Deserialize::from_value(
            object
                .remove("vertexCount")
                .ok_or_else(|| value_codec::error("Missing field vertexCount"))?,
        )?;
        let boundary_edges: usize = value_codec::Deserialize::from_value(
            object
                .remove("boundaryEdges")
                .ok_or_else(|| value_codec::error("Missing field boundaryEdges"))?,
        )?;
        let non_manifold_edges: usize = value_codec::Deserialize::from_value(
            object
                .remove("nonManifoldEdges")
                .ok_or_else(|| value_codec::error("Missing field nonManifoldEdges"))?,
        )?;
        let orientation_conflicts: usize = value_codec::Deserialize::from_value(
            object
                .remove("orientationConflicts")
                .ok_or_else(|| value_codec::error("Missing field orientationConflicts"))?,
        )?;
        let degenerate_triangles: usize = value_codec::Deserialize::from_value(
            object
                .remove("degenerateTriangles")
                .ok_or_else(|| value_codec::error("Missing field degenerateTriangles"))?,
        )?;
        let closed: bool = value_codec::Deserialize::from_value(
            object
                .remove("closed")
                .ok_or_else(|| value_codec::error("Missing field closed"))?,
        )?;
        let signed_volume_mm3: f64 = value_codec::Deserialize::from_value(
            object
                .remove("signedVolumeMm3")
                .ok_or_else(|| value_codec::error("Missing field signedVolumeMm3"))?,
        )?;
        let error_bound_certified: bool = value_codec::Deserialize::from_value(
            object
                .remove("errorBoundCertified")
                .ok_or_else(|| value_codec::error("Missing field errorBoundCertified"))?,
        )?;
        let self_intersection_status: String = value_codec::Deserialize::from_value(
            object
                .remove("selfIntersectionStatus")
                .ok_or_else(|| value_codec::error("Missing field selfIntersectionStatus"))?,
        )?;
        let construction: Construction = value_codec::Deserialize::from_value(
            object
                .remove("construction")
                .ok_or_else(|| value_codec::error("Missing field construction"))?,
        )?;
        let uv_area: Option<f64> = if let Some(v) = object.remove("uvArea") {
            value_codec::Deserialize::from_value(v)?
        } else {
            Default::default()
        };
        let sampled_deviation_mm: Option<f64> = if let Some(v) = object.remove("sampledDeviationMm")
        {
            value_codec::Deserialize::from_value(v)?
        } else {
            Default::default()
        };
        let parameter_seams_welded: Option<Seams> =
            if let Some(v) = object.remove("parameterSeamsWelded") {
                value_codec::Deserialize::from_value(v)?
            } else {
                Default::default()
            };
        let collapsed_boundary_count: Option<usize> =
            if let Some(v) = object.remove("collapsedBoundaryCount") {
                value_codec::Deserialize::from_value(v)?
            } else {
                Default::default()
            };
        Ok(Self {
            boolean,
            triangle_count,
            vertex_count,
            boundary_edges,
            non_manifold_edges,
            orientation_conflicts,
            degenerate_triangles,
            closed,
            signed_volume_mm3,
            error_bound_certified,
            self_intersection_status,
            construction,
            uv_area,
            sampled_deviation_mm,
            parameter_seams_welded,
            collapsed_boundary_count,
        })
    }
}
#[derive(Debug, Clone)]
pub struct BuiltMesh {
    pub mesh: Mesh,
    pub report: Report,
}
impl value_codec::Serialize for BuiltMesh {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        if let value_codec::Value::Object(fields) = value_codec::Serialize::to_value(&self.mesh) {
            object.extend(fields);
        }
        object.insert(
            "report".into(),
            value_codec::Serialize::to_value(&self.report),
        );
        value_codec::Value::Object(object)
    }
}
impl<'de> value_codec::Deserialize<'de> for BuiltMesh {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let mesh: Mesh = value_codec::Deserialize::from_value(value.clone())?;
        let report: Report = value_codec::Deserialize::from_value(
            object
                .remove("report")
                .ok_or_else(|| value_codec::error("Missing field report"))?,
        )?;
        Ok(Self { mesh, report })
    }
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
        for t in self.indices.as_chunks::<3>().0 {
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
        for t in self.indices.as_chunks::<3>().0 {
            let a = self.point(t[0])?;
            let b = self.point(t[1])?;
            let c = self.point(t[2])?;
            let ab = sub(b, a);
            let ac = sub(c, a);
            let normal = cross(ab, ac);
            if norm(normal) <= f64::EPSILON * (norm(ab) * norm(ac)).max(1.) {
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
                    .ok_or_else(|| error("Boundary chain does not close."))?;
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
        for t in self.indices.as_chunks::<3>().0 {
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
        check(
            report.closed && report.signed_volume_mm3 > 0.,
            "Thickening produced degenerate or open topology; the vector may be tangent to the surface.",
        )?;
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
        for t in self.indices.as_chunks::<3>().0 {
            let a = self.point(t[0])?;
            let b = self.point(t[1])?;
            let c = self.point(t[2])?;
            let normal = cross(sub(b, a), sub(c, a));
            let len = norm(normal);
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
