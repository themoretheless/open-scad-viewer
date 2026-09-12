//! Indexed boundary topology over exact rational curve/surface definitions.
//! Validation certifies combinatorial incidence, not geometric solid validity.
#![feature(
    try_blocks,
    gen_blocks,
    yield_expr,
    super_let,
    deref_patterns,
    yeet_expr
)]
#![allow(unused_features)]
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};
use std::collections::BTreeMap;

pub mod operations;
pub use operations::{boolean, chamfer, chamfer_edges, extrude_polygon, fillet, fillet_edges};

pub use brep_topology::{Body, FaceUse, Shell, Vertex};
pub type Edge = brep_topology::Edge<Curve>;
pub type Coedge = brep_topology::Coedge<Curve>;
pub type Loop = brep_topology::Loop<Curve>;
pub type Face = brep_topology::Face<Surface>;
#[derive(Clone, Debug)]
pub struct Model(pub brep_topology::Model<Curve, Surface, Curve>);
impl value_codec::Serialize for Model {
    fn to_value(&self) -> value_codec::Value {
        value_codec::Serialize::to_value(&self.0)
    }
}
impl<'de> value_codec::Deserialize<'de> for Model {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        Ok(Self(
            <brep_topology::Model<Curve, Surface, Curve> as value_codec::Deserialize>::from_value(
                value,
            )?,
        ))
    }
}
impl std::ops::Deref for Model {
    type Target = brep_topology::Model<Curve, Surface, Curve>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for Model {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
#[derive(Debug)]
pub struct Report {
    pub vertex_count: usize,
    pub edge_count: usize,
    pub loop_count: usize,
    pub face_count: usize,
    pub shell_count: usize,
    pub body_count: usize,
    pub boundary_edge_count: usize,
    pub topology_valid: bool,
    pub geometry_agreement: &'static str,
    pub solid_geometry_status: &'static str,
}
impl value_codec::Serialize for Report {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "vertexCount".into(),
            value_codec::Serialize::to_value(&self.vertex_count),
        );
        object.insert(
            "edgeCount".into(),
            value_codec::Serialize::to_value(&self.edge_count),
        );
        object.insert(
            "loopCount".into(),
            value_codec::Serialize::to_value(&self.loop_count),
        );
        object.insert(
            "faceCount".into(),
            value_codec::Serialize::to_value(&self.face_count),
        );
        object.insert(
            "shellCount".into(),
            value_codec::Serialize::to_value(&self.shell_count),
        );
        object.insert(
            "bodyCount".into(),
            value_codec::Serialize::to_value(&self.body_count),
        );
        object.insert(
            "boundaryEdgeCount".into(),
            value_codec::Serialize::to_value(&self.boundary_edge_count),
        );
        object.insert(
            "topologyValid".into(),
            value_codec::Serialize::to_value(&self.topology_valid),
        );
        object.insert(
            "geometryAgreement".into(),
            value_codec::Serialize::to_value(&self.geometry_agreement),
        );
        object.insert(
            "solidGeometryStatus".into(),
            value_codec::Serialize::to_value(&self.solid_geometry_status),
        );
        value_codec::Value::Object(object)
    }
}
fn invalid(message: impl Into<String>) -> Error {
    Error {
        code: "BREP_INVALID_TOPOLOGY",
        message: message.into(),
    }
}
fn require(ok: bool, message: &str) -> Result<()> {
    ok.ok_or_else(|| invalid(message))
}
fn distance(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f64>()
        .sqrt()
}
fn curve_point(c: &Curve, t: f64) -> Result<Vec<f64>> {
    let d = c.domain();
    Ok(c.evaluate(d[0] + t * (d[1] - d[0]))?.point)
}
impl Model {
    pub fn loop_uv(&self, id: usize, segments: usize) -> Result<Vec<[f64; 2]>> {
        require((1..=64).contains(&segments), "Edge sampling must be 1..64")?;
        let wire = self.loops.get(id).ok_or_else(|| invalid("Unknown loop"))?;
        let mut uv = Vec::new();
        for c in &wire.coedges {
            for i in 0..segments {
                let p = curve_point(&c.pcurve, i as f64 / segments as f64)?;
                require(p.len() == 2, "pcurve must be 2D")?;
                uv.push([p[0], p[1]]);
            }
        }
        Ok(uv)
    }
    pub fn validate(&self) -> Result<Report> {
        self.0.validate_topology().map_err(|e| Error {
            code: e.code,
            message: e.message,
        })?;
        let count = self.vertices.len()
            + self.edges.len()
            + self.loops.len()
            + self.faces.len()
            + self.shells.len()
            + self.bodies.len();
        let uses = self.loops.iter().map(|l| l.coedges.len()).sum::<usize>();
        if count > 4096 || uses > 8192 || self.faces.len() > 256 {
            return Err(Error {
                code: "BREP_RESOURCE_LIMIT",
                message: "B-rep exceeds 4096 entities, 256 faces or 8192 coedges".into(),
            });
        }
        require(
            self.tolerance_mm.is_finite() && (1e-10..=1e-2).contains(&self.tolerance_mm),
            "Invalid B-rep tolerance (1e-10..1e-2 mm)",
        )?;
        let tol = self.tolerance_mm;
        let mut vertex_used = vec![false; self.vertices.len()];
        for v in &self.vertices {
            require(
                v.point.iter().all(|x| x.is_finite() && x.abs() <= 1e6),
                "Invalid vertex coordinates",
            )?;
        }
        for e in &self.edges {
            e.curve.validate()?;
            require(
                e.curve.control_points[0].len() == 3,
                "Edge curve must be 3D",
            )?;
            for (i, &v) in e.vertices.iter().enumerate() {
                let point = &self
                    .vertices
                    .get(v)
                    .ok_or_else(|| invalid("Edge references unknown vertex"))?
                    .point;
                vertex_used[v] = true;
                require(
                    distance(&curve_point(&e.curve, i as f64)?, point) <= tol,
                    "Edge endpoint does not match vertex",
                )?;
            }
        }
        require(vertex_used.iter().all(|v| *v), "Unused vertex")?;
        let mut loop_owner = vec![None; self.loops.len()];
        let mut edge_used = vec![false; self.edges.len()];
        for (fi, f) in self.faces.iter().enumerate() {
            f.surface.validate()?;
            for (li, &l) in std::iter::once(&f.outer).chain(&f.holes).enumerate() {
                let wire = self
                    .loops
                    .get(l)
                    .ok_or_else(|| invalid("Face references unknown loop"))?;
                require(
                    loop_owner[l].replace(fi).is_none(),
                    "Loop belongs to multiple faces or is repeated",
                )?;
                require(!wire.coedges.is_empty(), "Empty face loop")?;
                let mut previous: Option<usize> = None;
                let mut first = None;
                let mut previous_uv: Option<Vec<f64>> = None;
                let mut first_uv = None;
                for c in &wire.coedges {
                    let edge = self
                        .edges
                        .get(c.edge)
                        .ok_or_else(|| invalid("Coedge references unknown edge"))?;
                    edge_used[c.edge] = true;
                    c.pcurve.validate()?;
                    require(c.pcurve.control_points[0].len() == 2, "pcurve must be 2D")?;
                    let start = edge.vertices[usize::from(c.reversed)];
                    let end = edge.vertices[usize::from(!c.reversed)];
                    if let Some(p) = previous {
                        require(p == start, "Loop vertex chain is disconnected")?;
                    } else {
                        first = Some(start);
                    }
                    previous = Some(end);
                    let uv0 = curve_point(&c.pcurve, 0.)?;
                    let uv1 = curve_point(&c.pcurve, 1.)?;
                    if let Some(p) = &previous_uv {
                        require(distance(p, &uv0) <= 1e-9, "Loop UV chain is disconnected")?;
                    } else {
                        first_uv = Some(uv0);
                    }
                    previous_uv = Some(uv1);
                    // Endpoint agreement is exact to tolerance; interiors are explicitly sampled.
                    for i in 0..=8 {
                        let t = i as f64 / 8.;
                        let uv = curve_point(&c.pcurve, t)?;
                        let p = f.surface.evaluate(uv[0], uv[1])?.point;
                        let q = curve_point(&edge.curve, if c.reversed { 1. - t } else { t })?;
                        require(
                            distance(&p, &q) <= tol,
                            "pcurve/surface and 3D edge disagree at sampled parameters",
                        )?;
                    }
                }
                require(previous == first, "Loop is not closed")?;
                require(
                    distance(previous_uv.as_ref().unwrap(), first_uv.as_ref().unwrap()) <= 1e-9,
                    "UV loop is not closed",
                )?;
                let uv = self.loop_uv(l, 8)?;
                let area = (0..uv.len())
                    .map(|i| {
                        let a = uv[i];
                        let b = uv[(i + 1) % uv.len()];
                        (a[0] - uv[0][0]) * (b[1] - uv[0][1])
                            - (b[0] - uv[0][0]) * (a[1] - uv[0][1])
                    })
                    .sum::<f64>();
                require(
                    if li == 0 { area > 0. } else { area < 0. },
                    "Outer UV loop must be CCW and hole loops CW",
                )?;
            }
        }
        require(loop_owner.iter().all(Option::is_some), "Unused loop")?;
        require(edge_used.iter().all(|v| *v), "Unused edge")?;
        let boundary = self
            .shells
            .iter()
            .map(|s| {
                let mut uses = BTreeMap::<usize, usize>::new();
                for u in &s.faces {
                    let f = &self.faces[u.face];
                    for &l in std::iter::once(&f.outer).chain(&f.holes) {
                        for c in &self.loops[l].coedges {
                            *uses.entry(c.edge).or_default() += 1;
                        }
                    }
                }
                uses.values().filter(|n| **n == 1).count()
            })
            .sum();
        Ok(Report {
            vertex_count: self.vertices.len(),
            edge_count: self.edges.len(),
            loop_count: self.loops.len(),
            face_count: self.faces.len(),
            shell_count: self.shells.len(),
            body_count: self.bodies.len(),
            boundary_edge_count: boundary,
            topology_valid: true,
            geometry_agreement: "sampled_with_tolerance",
            solid_geometry_status: "not_certified",
        })
    }
}
fn line(a: Vec<f64>, b: Vec<f64>) -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a, b],
        weights: vec![1., 1.],
        periodic: false,
    }
}
/// Construct an exact six-face NURBS box with shared edges, not six loose patches.
pub fn cuboid(min: [f64; 3], max: [f64; 3]) -> Result<Model> {
    require(
        (0..3).all(|i| {
            min[i].is_finite()
                && max[i].is_finite()
                && max[i] > min[i]
                && min[i].abs() <= 1e6
                && max[i].abs() <= 1e6
        }),
        "Invalid box bounds",
    )?;
    let vertices = [
        [0, 0, 0],
        [1, 0, 0],
        [1, 1, 0],
        [0, 1, 0],
        [0, 0, 1],
        [1, 0, 1],
        [1, 1, 1],
        [0, 1, 1],
    ]
    .map(|p| Vertex {
        point: std::array::from_fn(|i| if p[i] == 0 { min[i] } else { max[i] }),
    })
    .to_vec();
    let mut m = Model(brep_topology::Model {
        vertices,
        edges: vec![],
        loops: vec![],
        faces: vec![],
        shells: vec![],
        bodies: vec![],
        tolerance_mm: 1e-7,
    });
    let mut edges = BTreeMap::new();
    for corners in [
        [0, 3, 2, 1],
        [4, 5, 6, 7],
        [0, 1, 5, 4],
        [1, 2, 6, 5],
        [2, 3, 7, 6],
        [3, 0, 4, 7],
    ] {
        let mut coedges = vec![];
        let uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
        for i in 0..4 {
            let a = corners[i];
            let b = corners[(i + 1) % 4];
            let key = (a.min(b), a.max(b));
            let edge = *edges.entry(key).or_insert_with(|| {
                let id = m.0.edges.len();
                m.0.edges.push(Edge {
                    vertices: [key.0, key.1],
                    curve: line(
                        m.0.vertices[key.0].point.to_vec(),
                        m.0.vertices[key.1].point.to_vec(),
                    ),
                });
                id
            });
            coedges.push(Coedge {
                edge,
                reversed: a > b,
                pcurve: line(uv[i].to_vec(), uv[(i + 1) % 4].to_vec()),
            });
        }
        let outer = m.0.loops.len();
        m.0.loops.push(Loop { coedges });
        m.0.faces.push(Face {
            surface: Surface {
                degree_u: 1,
                degree_v: 1,
                knots_u: vec![0., 0., 1., 1.],
                knots_v: vec![0., 0., 1., 1.],
                control_points: vec![
                    vec![
                        m.0.vertices[corners[0]].point.to_vec(),
                        m.0.vertices[corners[3]].point.to_vec(),
                    ],
                    vec![
                        m.0.vertices[corners[1]].point.to_vec(),
                        m.0.vertices[corners[2]].point.to_vec(),
                    ],
                ],
                weights: vec![vec![1., 1.], vec![1., 1.]],
                periodic_u: false,
                periodic_v: false,
            },
            outer,
            holes: vec![],
        });
    }
    m.0.shells.push(Shell {
        faces: (0..6)
            .map(|face| FaceUse {
                face,
                reversed: false,
            })
            .collect(),
        closed: true,
    });
    m.0.bodies.push(Body {
        outer_shell: 0,
        inner_shells: vec![],
    });
    m.validate()?;
    Ok(m)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn box_incidence_and_serialization() {
        let m = cuboid([0.; 3], [2., 3., 4.]).unwrap();
        let r = m.validate().unwrap();
        assert_eq!(
            (r.vertex_count, r.edge_count, r.face_count, r.body_count),
            (8, 12, 6, 1)
        );
        let restored: Model = value_codec::from_str(&value_codec::to_string(&m).unwrap()).unwrap();
        restored.validate().unwrap();
    }
    #[test]
    fn rejects_broken_incidence_geometry_and_orientation() {
        let a = cuboid([0.; 3], [1.; 3]).unwrap();
        for kind in 0..6 {
            let mut m = a.clone();
            match kind {
                0 => m.0.edges[0].vertices[0] = 999,
                1 => m.0.loops[0].coedges[0].reversed ^= true,
                2 => m.0.shells[0].faces[0].reversed = true,
                3 => m.0.faces[0].outer = m.0.faces[1].outer,
                4 => m.0.vertices[0].point[0] = 0.2,
                _ => m.0.bodies[0].inner_shells.push(0),
            }
            assert!(m.validate().is_err(), "mutation {kind}");
        }
    }
    #[test]
    fn open_shell_is_not_a_body() {
        let mut m = cuboid([0.; 3], [1.; 3]).unwrap();
        m.0.shells[0].closed = false;
        assert!(m.validate().is_err());
        m.0.bodies.clear();
        m.validate().unwrap();
    }
}
