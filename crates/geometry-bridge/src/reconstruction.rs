//! Explicit mesh-to-NURBS conversion. PN patches approximate a chosen smoothing;
//! they do not recover unknown original CAD surfaces or prove global continuity.
use super::*;
use polygon_core::solid::proximity::{closest_triangle, valid_source};
type Point = [f64; 3];
use math_core::{cross, dot, norm, sub};
#[derive(Clone, Copy)]
pub enum Mode {
    Faceted,
    PointNormal,
}
impl value_codec::Serialize for Mode {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Faceted => value_codec::Value::String("faceted".into()),
            Self::PointNormal => value_codec::Value::String("point_normal".into()),
        }
    }
}
impl<'de> value_codec::Deserialize<'de> for Mode {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        match value.as_str().unwrap_or("") {
            "faceted" => Ok(Self::Faceted),
            "point_normal" => Ok(Self::PointNormal),
            _ => Err(value_codec::error("Unknown enum variant")),
        }
    }
}
#[derive(Clone)]
pub struct PatchSet {
    pub patches: Vec<Surface>,
    pub face_ids: Vec<usize>,
    pub mode: Mode,
    pub sampled_max_deviation_mm: f64,
    pub sample_count: usize,
    pub error_bound_certified: bool,
}
impl value_codec::Serialize for PatchSet {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "patches".into(),
            value_codec::Serialize::to_value(&self.patches),
        );
        object.insert(
            "faceIds".into(),
            value_codec::Serialize::to_value(&self.face_ids),
        );
        object.insert("mode".into(), value_codec::Serialize::to_value(&self.mode));
        object.insert(
            "sampledMaxDeviationMm".into(),
            value_codec::Serialize::to_value(&self.sampled_max_deviation_mm),
        );
        object.insert(
            "sampleCount".into(),
            value_codec::Serialize::to_value(&self.sample_count),
        );
        object.insert(
            "errorBoundCertified".into(),
            value_codec::Serialize::to_value(&self.error_bound_certified),
        );
        value_codec::Value::Object(object)
    }
}
impl<'de> value_codec::Deserialize<'de> for PatchSet {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let patches: Vec<Surface> = value_codec::Deserialize::from_value(
            object
                .remove("patches")
                .ok_or_else(|| value_codec::error("Missing field patches"))?,
        )?;
        let face_ids: Vec<usize> = value_codec::Deserialize::from_value(
            object
                .remove("faceIds")
                .ok_or_else(|| value_codec::error("Missing field faceIds"))?,
        )?;
        let mode: Mode = value_codec::Deserialize::from_value(
            object
                .remove("mode")
                .ok_or_else(|| value_codec::error("Missing field mode"))?,
        )?;
        let sampled_max_deviation_mm: f64 = value_codec::Deserialize::from_value(
            object
                .remove("sampledMaxDeviationMm")
                .ok_or_else(|| value_codec::error("Missing field sampledMaxDeviationMm"))?,
        )?;
        let sample_count: usize = value_codec::Deserialize::from_value(
            object
                .remove("sampleCount")
                .ok_or_else(|| value_codec::error("Missing field sampleCount"))?,
        )?;
        let error_bound_certified: bool = value_codec::Deserialize::from_value(
            object
                .remove("errorBoundCertified")
                .ok_or_else(|| value_codec::error("Missing field errorBoundCertified"))?,
        )?;
        Ok(Self {
            patches,
            face_ids,
            mode,
            sampled_max_deviation_mm,
            sample_count,
            error_bound_certified,
        })
    }
}
fn surface(cp: Vec<Vec<Point>>) -> Surface {
    let degree = cp.len() - 1;
    let mut knots = vec![0.; degree + 1];
    knots.extend(vec![1.; degree + 1]);
    Surface {
        degree_u: degree,
        degree_v: degree,
        knots_u: knots.clone(),
        knots_v: knots,
        weights: vec![vec![1.; degree + 1]; degree + 1],
        control_points: cp
            .into_iter()
            .map(|r| r.into_iter().map(|p| p.to_vec()).collect())
            .collect(),
        periodic_u: false,
        periodic_v: false,
    }
}
fn pn(p: [Point; 3], n: [Point; 3], a: f64, b: f64, c: f64) -> Point {
    let edge = |i: usize, j: usize| {
        let w = dot(sub(p[j], p[i]), n[i]);
        std::array::from_fn::<_, 3, _>(|k| (2. * p[i][k] + p[j][k] - w * n[i][k]) / 3.)
    };
    let e = [
        edge(0, 1),
        edge(1, 0),
        edge(1, 2),
        edge(2, 1),
        edge(2, 0),
        edge(0, 2),
    ];
    let center: Point = std::array::from_fn(|k| {
        1.5 * e.iter().map(|p| p[k] / 6.).sum::<f64>()
            - 0.5 * p.iter().map(|p| p[k] / 3.).sum::<f64>()
    });
    std::array::from_fn(|k| {
        a * a * a * p[0][k]
            + b * b * b * p[1][k]
            + c * c * c * p[2][k]
            + 3. * (a * a * b * e[0][k]
                + a * b * b * e[1][k]
                + b * b * c * e[2][k]
                + b * c * c * e[3][k]
                + a * c * c * e[4][k]
                + a * a * c * e[5][k])
            + 6. * a * b * c * center[k]
    })
}
fn bernstein(f: [Point; 4]) -> [Point; 4] {
    [
        f[0],
        std::array::from_fn(|k| 3. * f[1][k] - 1.5 * f[2][k] - 5. / 6. * f[0][k] + f[3][k] / 3.),
        std::array::from_fn(|k| -1.5 * f[1][k] + 3. * f[2][k] + f[0][k] / 3. - 5. / 6. * f[3][k]),
        f[3],
    ]
}
pub fn nurbs_from_mesh(mesh: &Mesh, mode: Mode, max_deviation_mm: f64) -> Result<PatchSet> {
    if !max_deviation_mm.is_finite() || max_deviation_mm < 0. {
        return Err(input(
            "Expected nonnegative finite sampled-deviation tolerance",
        ));
    }
    let mesh = valid_source(mesh, 2048)?;
    let point = |i: usize| {
        [
            mesh.positions[3 * i],
            mesh.positions[3 * i + 1],
            mesh.positions[3 * i + 2],
        ]
    };
    let mut normals = vec![[0.; 3]; mesh.positions.len() / 3];
    if matches!(mode, Mode::PointNormal) {
        let report = mesh.inspect()?;
        if report.non_manifold_edges > 0 || report.orientation_conflicts > 0 {
            return Err(input(
                "Point-normal fitting requires consistently oriented manifold edges",
            ));
        }
        for t in mesh.indices.chunks_exact(3) {
            let n = cross(sub(point(t[1]), point(t[0])), sub(point(t[2]), point(t[0])));
            for &i in t {
                for (k, v) in normals[i].iter_mut().enumerate() {
                    *v += n[k];
                }
            }
        }
        for n in &mut normals {
            let l = norm(*n);
            if l == 0. {
                return Err(input(
                    "Cannot infer a normal at a cancelling or unused vertex",
                ));
            }
            for v in n {
                *v /= l;
            }
        }
    }
    let mut patches = Vec::new();
    let mut maximum: f64 = 0.;
    let mut count = 0;
    for t in mesh.indices.chunks_exact(3) {
        let p = [point(t[0]), point(t[1]), point(t[2])];
        let n = [normals[t[0]], normals[t[1]], normals[t[2]]];
        let s = match mode {
            Mode::Faceted => surface(vec![vec![p[0], p[2]], vec![p[1], p[2]]]),
            Mode::PointNormal => {
                let mut samples = [[[0.; 3]; 4]; 4];
                for (i, row) in samples.iter_mut().enumerate() {
                    for (j, q) in row.iter_mut().enumerate() {
                        let u = i as f64 / 3.;
                        let v = j as f64 / 3.;
                        *q = pn(p, n, (1. - u) * (1. - v), u * (1. - v), v);
                    }
                }
                for j in [0, 1, 2, 3] {
                    let f = bernstein(std::array::from_fn(|i| samples[i][j]));
                    for i in 0..4 {
                        samples[i][j] = f[i];
                    }
                }
                for row in &mut samples {
                    *row = bernstein(*row);
                    row[3] = p[2];
                }
                surface(samples.iter().map(|row| row.to_vec()).collect())
            }
        };
        s.validate()?;
        // Samples use the native rational evaluator, including the collapsed edge.
        let sampler = SurfaceSampler::new(&s)?;
        for i in 0..=8 {
            for j in 0..=8 {
                let q = sampler.evaluate(i as f64 / 8., j as f64 / 8.)?.point;
                let d = norm(sub(q, closest_triangle(q, p[0], p[1], p[2])));
                maximum = maximum.max(d);
                count += 1;
            }
        }
        patches.push(s);
    }
    let numeric_slack =
        mesh.positions.iter().map(|v| v.abs()).fold(1., f64::max) * f64::EPSILON * 32.;
    if maximum > max_deviation_mm + numeric_slack {
        return Err(input(format!(
            "Sampled NURBS deviation {maximum} mm exceeds tolerance {max_deviation_mm} mm"
        )));
    }
    Ok(PatchSet {
        face_ids: (0..patches.len()).collect(),
        patches,
        mode,
        sampled_max_deviation_mm: maximum,
        sample_count: count,
        error_bound_certified: false,
    })
}
/// Tessellate individual patches and sew matching boundary samples. No B-rep
/// is inferred for smooth patches; the source face IDs remain explicit.
pub fn tessellate_patches(set: &PatchSet, segments: usize) -> Result<brep::Tessellation> {
    if set.patches.is_empty()
        || set.patches.len() > 2048
        || set.face_ids.len() != set.patches.len()
        || !(1..=16).contains(&segments)
        || set.patches.len().saturating_mul(segments * segments * 2) > 20_000
    {
        return Err(input("NURBS patch tessellation budget exceeded"));
    }
    let mut mesh = Mesh {
        positions: vec![],
        indices: vec![],
        uv: None,
    };
    let mut ids = Vec::new();
    for (i, s) in set.patches.iter().enumerate() {
        let built = tessellate_nurbs(
            s,
            &tessellation::Options {
                segments_u: segments,
                segments_v: segments,
                trim: None,
                max_triangles: Some(20_000),
            },
        )?;
        let offset = mesh.positions.len() / 3;
        ids.extend(vec![set.face_ids[i]; built.mesh.indices.len() / 3]);
        mesh.indices
            .extend(built.mesh.indices.iter().map(|j| j + offset));
        mesh.positions.extend(built.mesh.positions);
    }
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    let mut magnitude: f64 = 1.;
    for p in mesh.positions.chunks_exact(3) {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
            magnitude = magnitude.max(p[k].abs());
        }
    }
    let tolerance = norm(sub(hi, lo)) * 1e-10 + magnitude * f64::EPSILON * 32.;
    let mesh = brep::weld(mesh, tolerance)?;
    let report = mesh.inspect()?;
    Ok(brep::Tessellation {
        built: BuiltMesh { mesh, report },
        face_ids: ids,
    })
}

/// Exact planar, trimmed NURBS B-rep of the source triangle boundary. Topology
/// budgets (notably 256 faces) apply. No smooth-face recognition is implied.
pub fn nurbs_brep_from_mesh(mesh: &Mesh) -> Result<brep_core::Model> {
    let source = valid_source(mesh, 256)?;
    let polygon = polygon_core::solid::brep::from_mesh(&source, None)?;
    let mut faces = Vec::new();
    let mut loops = polygon
        .loops
        .iter()
        .map(|_| brep_topology::Loop {
            coedges: Vec::new(),
        })
        .collect::<Vec<_>>();
    let line = |a: Vec<f64>, b: Vec<f64>| Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a, b],
        weights: vec![1., 1.],
        periodic: false,
    };
    for f in &polygon.faces {
        let m = &f.surface.mesh;
        let point = |i: usize| {
            [
                m.positions[3 * i],
                m.positions[3 * i + 1],
                m.positions[3 * i + 2],
            ]
        };
        let a = point(m.indices[0]);
        let b = point(m.indices[1]);
        let c = point(m.indices[2]);
        let ab = sub(b, a);
        let ac = sub(c, a);
        let d = std::array::from_fn(|i| b[i] + c[i] - a[i]);
        let s = surface(vec![vec![a, c], vec![b, d]]);
        let uv = |p: Point| {
            let q = sub(p, a);
            let aa = dot(ab, ab);
            let bb = dot(ac, ac);
            let cc = dot(ab, ac);
            let denominator = aa * bb - cc * cc;
            vec![
                (dot(q, ab) * bb - dot(q, ac) * cc) / denominator,
                (dot(q, ac) * aa - dot(q, ab) * cc) / denominator,
            ]
        };
        for &li in std::iter::once(&f.outer).chain(&f.holes) {
            for co in &polygon.loops[li].coedges {
                let edge = &polygon.edges[co.edge];
                let [mut v0, mut v1] = edge.vertices;
                if co.reversed {
                    std::mem::swap(&mut v0, &mut v1);
                }
                loops[li].coedges.push(brep_topology::Coedge {
                    edge: co.edge,
                    reversed: co.reversed,
                    pcurve: line(
                        uv(polygon.vertices[v0].point),
                        uv(polygon.vertices[v1].point),
                    ),
                });
            }
        }
        faces.push(brep_topology::Face {
            surface: s,
            outer: f.outer,
            holes: f.holes.clone(),
        });
    }
    let edges = polygon
        .edges
        .iter()
        .map(|e| brep_topology::Edge {
            vertices: e.vertices,
            curve: line(
                polygon.vertices[e.vertices[0]].point.to_vec(),
                polygon.vertices[e.vertices[1]].point.to_vec(),
            ),
        })
        .collect();
    let model = brep_core::Model(brep_topology::Model {
        vertices: polygon.vertices,
        edges,
        loops,
        faces,
        shells: polygon.shells,
        bodies: polygon.bodies,
        tolerance_mm: polygon.tolerance_mm,
    });
    model.validate()?;
    Ok(model)
}

#[derive(Clone, Debug)]
pub struct SubdivisionFit {
    pub cage: subdivision_core::Cage,
    pub iterations: usize,
    pub vertex_residual_before_mm: f64,
    pub vertex_residual_after_mm: f64,
    pub deviation: polygon_core::solid::proximity::Deviation,
    pub correspondence: &'static str,
}
impl value_codec::Serialize for SubdivisionFit {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert("cage".into(), value_codec::Serialize::to_value(&self.cage));
        object.insert(
            "iterations".into(),
            value_codec::Serialize::to_value(&self.iterations),
        );
        object.insert(
            "vertexResidualBeforeMm".into(),
            value_codec::Serialize::to_value(&self.vertex_residual_before_mm),
        );
        object.insert(
            "vertexResidualAfterMm".into(),
            value_codec::Serialize::to_value(&self.vertex_residual_after_mm),
        );
        object.insert(
            "deviation".into(),
            value_codec::Serialize::to_value(&self.deviation),
        );
        object.insert(
            "correspondence".into(),
            value_codec::Serialize::to_value(&self.correspondence),
        );
        value_codec::Value::Object(object)
    }
}

/// Mesh inspect/fit stays here. The cage kernel only sees faces and residuals.
pub fn mesh_to_subdivision(mesh: &Mesh, iterations: usize) -> Result<SubdivisionFit> {
    let source = valid_source(mesh, 2048)?;
    let cage = subdivision_core::Cage::from_faces(
        source
            .positions
            .chunks_exact(3)
            .map(|p| [p[0], p[1], p[2]])
            .collect(),
        source.indices.chunks_exact(3).map(|t| t.to_vec()).collect(),
    )?;
    let preview = crate::mesh_from_triangles(cage.subdivide(1)?.triangulate()?.0);
    polygon_core::solid::proximity::sample_deviation(&source, &preview)?;
    let fit = subdivision_core::fit(&cage, iterations)?;
    let output = crate::mesh_from_triangles(fit.cage.subdivide(1)?.triangulate()?.0);
    let deviation = polygon_core::solid::proximity::sample_deviation(&source, &output)?;
    Ok(SubdivisionFit {
        cage: fit.cage,
        iterations: fit.iterations,
        vertex_residual_before_mm: fit.vertex_residual_before_mm,
        vertex_residual_after_mm: fit.vertex_residual_after_mm,
        deviation,
        correspondence: fit.correspondence,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cube() -> Mesh {
        let m = brep_core::cuboid([-1.; 3], [1.; 3]).unwrap();
        brep::nurbs(&m, 1).unwrap().built.mesh
    }
    #[test]
    fn exact_nurbs_round_trip() {
        let mesh = cube();
        let set = nurbs_from_mesh(&mesh, Mode::Faceted, 0.).unwrap();
        assert_eq!(set.patches.len(), mesh.indices.len() / 3);
        let output = tessellate_patches(&set, 3).unwrap();
        assert!(output.built.report.closed);
        assert!((output.built.report.signed_volume_mm3 - 8.).abs() < 1e-9);
        assert!(
            polygon_core::solid::proximity::sample_deviation(&mesh, &output.built.mesh)
                .unwrap()
                .sampled_max_mm
                < 1e-9
        );
    }
    #[test]
    fn smooth_nurbs_is_bounded_and_continuous_at_seams() {
        let mesh = cube();
        assert!(nurbs_from_mesh(&mesh, Mode::PointNormal, 0.).is_err());
        let set = nurbs_from_mesh(&mesh, Mode::PointNormal, 1.).unwrap();
        assert!(set.sampled_max_deviation_mm > 0.01);
        assert_eq!(set.patches[0].degree_u, 3);
        let out = tessellate_patches(&set, 3).unwrap();
        assert!(out.built.report.closed, "{:?}", out.built.report);
        assert!(out.built.report.signed_volume_mm3 > 0.);
    }
    #[test]
    fn mesh_sdf_sign_and_hollow_orientation() {
        let m = cube();
        let field = sdf_core::Field::from_triangles(crate::triangles_from_mesh(&m), true).unwrap();
        assert!((field.evaluate([0.; 3]).unwrap() + 1.).abs() < 1e-12);
        assert!((field.evaluate([2., 0., 0.]).unwrap() - 1.).abs() < 1e-12);
        assert_eq!(field.evaluate([1., 0., 0.]).unwrap(), 0.);
        let mut hollow = m.clone();
        let offset = hollow.positions.len() / 3;
        hollow.positions.extend(m.positions.iter().map(|v| v / 2.));
        for t in m.indices.chunks_exact(3) {
            hollow
                .indices
                .extend([offset + t[0], offset + t[2], offset + t[1]]);
        }
        let f = sdf_core::Field::from_triangles(crate::triangles_from_mesh(&hollow), true).unwrap();
        assert!((f.evaluate([0.; 3]).unwrap() - 0.5).abs() < 1e-12);
        assert!(f.evaluate([0.75, 0., 0.]).unwrap() < 0.);
        let open = Mesh {
            positions: vec![0., 0., 0., 1., 0., 0., 0., 1., 0.],
            indices: vec![0, 1, 2],
            uv: None,
        };
        assert!(sdf_core::Field::from_triangles(crate::triangles_from_mesh(&open), true).is_err());
        assert_eq!(
            sdf_core::Field::from_triangles(crate::triangles_from_mesh(&open), false)
                .unwrap()
                .evaluate([0., 0., 2.])
                .unwrap(),
            2.
        );
    }
    #[test]
    fn subdivision_fit_reduces_interpolation_residual() {
        let m = cube();
        let r = mesh_to_subdivision(&m, 16).unwrap();
        assert!(r.vertex_residual_after_mm < r.vertex_residual_before_mm * 0.05);
        assert_eq!(r.cage.faces.len(), m.indices.len() / 3);
        assert!(r.deviation.sampled_max_mm.is_finite());
        assert!(!r.deviation.error_bound_certified);
    }
}
