//! Polygon B-rep uses the same incidence model as NURBS, with mesh face geometry.
use crate::{Error, Mesh, Result};
use brep_topology::{Body, Coedge, Edge, Face, FaceUse, Loop, Shell, Vertex};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FaceGeometry {
    pub mesh: Mesh,
    pub source_face_id: usize,
}
pub type Model = brep_topology::Model<(), FaceGeometry, ()>;
fn invalid(message: impl Into<String>) -> Error {
    Error {
        code: "BREP_INVALID_GEOMETRY",
        message: message.into(),
    }
}
fn topology(m: &Model) -> Result<()> {
    m.validate_topology().map_err(|e| Error {
        code: e.code,
        message: e.message,
    })
}
/// Explicit groups preserve authored face identity. Omission makes one face per triangle.
/// Multiple shells remain shells: cavity/body containment is not guessed.
pub fn from_mesh(mesh: &Mesh, face_ids: Option<&[usize]>) -> Result<Model> {
    mesh.validate()?;
    let report = mesh.inspect()?;
    if report.degenerate_triangles > 0
        || report.non_manifold_edges > 0
        || report.orientation_conflicts > 0
    {
        return Err(invalid("Mesh has invalid triangle/edge topology"));
    }
    if face_ids.is_some_and(|ids| ids.len() != mesh.indices.len() / 3) {
        return Err(invalid("Expected one face ID per triangle"));
    }
    let mut groups = BTreeMap::<usize, Vec<usize>>::new();
    for (i, t) in mesh.indices.chunks_exact(3).enumerate() {
        groups
            .entry(face_ids.map_or(i, |ids| ids[i]))
            .or_default()
            .extend(t);
    }
    if groups.len() > 256 {
        return Err(invalid(
            "At most 256 B-rep faces; provide explicit face groups",
        ));
    }
    let mut m = Model {
        vertices: vec![],
        edges: vec![],
        loops: vec![],
        faces: vec![],
        shells: vec![],
        bodies: vec![],
        tolerance_mm: 1e-7,
    };
    let mut vertex_map = BTreeMap::<usize, usize>::new();
    let mut edges = BTreeMap::new();
    for (source_face_id, indices) in groups {
        let mut geometry = Mesh {
            positions: vec![],
            indices: vec![],
            uv: None,
        };
        let mut local = BTreeMap::new();
        let mut originals = vec![];
        for v in indices {
            let id = *local.entry(v).or_insert_with(|| {
                let id = originals.len();
                originals.push(v);
                geometry
                    .positions
                    .extend_from_slice(&mesh.positions[3 * v..3 * v + 3]);
                id
            });
            geometry.indices.push(id);
        }
        let mut boundaries = geometry.boundary_loops()?;
        for l in &mut boundaries {
            l.pop();
        }
        if boundaries.is_empty() {
            return Err(invalid(
                "Face group has no boundary; split periodic/closed surfaces into bounded patches",
            ));
        }
        // Select outer contour by projected area; this only infers holes for planar patches.
        let area = |l: &Vec<usize>| {
            let mut n = [0.; 3];
            for i in 0..l.len() {
                let a = geometry.point(l[i]).unwrap();
                let b = geometry.point(l[(i + 1) % l.len()]).unwrap();
                let c = crate::cross(a, b);
                for j in 0..3 {
                    n[j] += c[j];
                }
            }
            crate::norm(&n)
        };
        let outer = boundaries
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| area(a).total_cmp(&area(b)))
            .unwrap()
            .0;
        let mut loops = vec![];
        for boundary in &boundaries {
            let mut coedges = vec![];
            for i in 0..boundary.len() {
                let mut endpoints = [0; 2];
                for (j, &v) in [boundary[i], boundary[(i + 1) % boundary.len()]]
                    .iter()
                    .enumerate()
                {
                    let old = originals[v];
                    endpoints[j] = *vertex_map.entry(old).or_insert_with(|| {
                        let id = m.vertices.len();
                        m.vertices.push(Vertex {
                            point: mesh.point(old).unwrap(),
                        });
                        id
                    });
                }
                let [a, b] = endpoints;
                let key = (a.min(b), a.max(b));
                let edge = *edges.entry(key).or_insert_with(|| {
                    let id = m.edges.len();
                    m.edges.push(Edge {
                        vertices: [key.0, key.1],
                        curve: (),
                    });
                    id
                });
                coedges.push(Coedge {
                    edge,
                    reversed: a > b,
                    pcurve: (),
                });
            }
            loops.push(m.loops.len());
            m.loops.push(Loop { coedges });
        }
        m.faces.push(Face {
            surface: FaceGeometry {
                mesh: geometry,
                source_face_id,
            },
            outer: loops[outer],
            holes: loops
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != outer)
                .map(|(_, l)| *l)
                .collect(),
        });
    }
    let mut edge_faces = BTreeMap::<usize, Vec<usize>>::new();
    for (fi, f) in m.faces.iter().enumerate() {
        for l in std::iter::once(&f.outer).chain(&f.holes) {
            for c in &m.loops[*l].coedges {
                edge_faces.entry(c.edge).or_default().push(fi);
            }
        }
    }
    let mut seen = BTreeSet::new();
    for start in 0..m.faces.len() {
        if seen.contains(&start) {
            continue;
        }
        let mut q = vec![start];
        let mut faces = vec![];
        let mut closed = true;
        while let Some(f) = q.pop() {
            if !seen.insert(f) {
                continue;
            }
            faces.push(FaceUse {
                face: f,
                reversed: false,
            });
            let face = &m.faces[f];
            for l in std::iter::once(&face.outer).chain(&face.holes) {
                for c in &m.loops[*l].coedges {
                    closed &= edge_faces[&c.edge].len() == 2;
                    q.extend(&edge_faces[&c.edge]);
                }
            }
        }
        m.shells.push(Shell { faces, closed });
    }
    if m.shells.len() == 1 && m.shells[0].closed && report.signed_volume_mm3 > 0. {
        m.bodies.push(Body {
            outer_shell: 0,
            inner_shells: vec![],
        });
    }
    validate(&m)?;
    Ok(m)
}
/// Checks incidence and equality of each face boundary with its polygon geometry.
pub fn validate(m: &Model) -> Result<()> {
    topology(m)?;
    let mut total = 0;
    for f in &m.faces {
        let mesh = &f.surface.mesh;
        mesh.validate()?;
        let r = mesh.inspect()?;
        total += r.triangle_count;
        if total > 20000 {
            return Err(invalid("Polygon B-rep exceeds 20000 triangles"));
        }
        if r.degenerate_triangles > 0 || r.non_manifold_edges > 0 || r.orientation_conflicts > 0 {
            return Err(invalid("Invalid polygon face mesh"));
        }
        let mut expected = vec![];
        for l in std::iter::once(&f.outer).chain(&f.holes) {
            for c in &m.loops[*l].coedges {
                let e = &m.edges[c.edge];
                expected.push((
                    m.vertices[e.vertices[usize::from(c.reversed)]].point,
                    m.vertices[e.vertices[usize::from(!c.reversed)]].point,
                ));
            }
        }
        let mut adjacency = BTreeMap::<(usize, usize), Vec<usize>>::new();
        for (ti, t) in mesh.indices.chunks_exact(3).enumerate() {
            for i in 0..3 {
                let a = t[i];
                let b = t[(i + 1) % 3];
                adjacency.entry((a.min(b), a.max(b))).or_default().push(ti);
            }
        }
        let mut seen = BTreeSet::new();
        let mut q = vec![0];
        while let Some(t) = q.pop() {
            if !seen.insert(t) {
                continue;
            }
            let tri = mesh
                .indices
                .get(t * 3..t * 3 + 3)
                .ok_or_else(|| invalid("Empty polygon face"))?;
            for i in 0..3 {
                let a = tri[i];
                let b = tri[(i + 1) % 3];
                q.extend(&adjacency[&(a.min(b), a.max(b))]);
            }
        }
        if seen.len() != mesh.indices.len() / 3 {
            return Err(invalid("Polygon face geometry is disconnected"));
        }
        let mut actual = mesh.boundary_loops()?;
        for l in &mut actual {
            l.pop();
        }
        let mut found = vec![false; expected.len()];
        for l in actual {
            for i in 0..l.len() {
                let a = mesh.point(l[i])?;
                let b = mesh.point(l[(i + 1) % l.len()])?;
                let matching = expected
                    .iter()
                    .enumerate()
                    .find(|(i, (x, y))| {
                        !found[*i]
                            && crate::norm(&crate::sub(a, *x)) <= m.tolerance_mm
                            && crate::norm(&crate::sub(b, *y)) <= m.tolerance_mm
                    })
                    .map(|(i, _)| i)
                    .ok_or_else(|| invalid("Polygon face boundary does not match B-rep coedges"))?;
                found[matching] = true;
            }
        }
        if found.iter().any(|x| !*x) {
            return Err(invalid("Missing polygon face boundary"));
        }
    }
    Ok(())
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tessellation {
    #[serde(flatten)]
    pub mesh: Mesh,
    pub face_ids: Vec<usize>,
}
/// Face geometry is already tessellated. Preserve it and publish B-rep face IDs.
pub fn tessellate(m: &Model) -> Result<Tessellation> {
    validate(m)?;
    let mut mesh = Mesh {
        positions: vec![],
        indices: vec![],
        uv: None,
    };
    let mut ids = vec![];
    for shell in &m.shells {
        for u in &shell.faces {
            let f = &m.faces[u.face].surface.mesh;
            let base = mesh.positions.len() / 3;
            mesh.positions.extend(&f.positions);
            for t in f.indices.chunks_exact(3) {
                let t = if u.reversed {
                    [t[0], t[2], t[1]]
                } else {
                    [t[0], t[1], t[2]]
                };
                mesh.indices.extend(t.map(|i| i + base));
                ids.push(u.face);
            }
        }
    }
    Ok(Tessellation {
        mesh,
        face_ids: ids,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn square() -> Mesh {
        Mesh {
            positions: vec![0., 0., 0., 2., 0., 0., 2., 2., 0., 0., 2., 0.],
            indices: vec![0, 1, 2, 0, 2, 3],
            uv: None,
        }
    }
    #[test]
    fn grouped_polygon_face_has_one_boundary() {
        let m = from_mesh(&square(), Some(&[9, 9])).unwrap();
        assert_eq!(
            (
                m.vertices.len(),
                m.edges.len(),
                m.faces.len(),
                m.shells.len()
            ),
            (4, 4, 1, 1)
        );
        assert!(!m.shells[0].closed);
        assert_eq!(tessellate(&m).unwrap().face_ids, vec![0, 0]);
    }
    #[test]
    fn closed_mesh_forms_a_body() {
        let solid = square().thicken([0., 0., 2.]).unwrap();
        let m = from_mesh(&solid.mesh, None).unwrap();
        assert_eq!(m.bodies.len(), 1);
        assert!(m.shells[0].closed);
    }
    #[test]
    fn corrupted_polygon_boundary_is_rejected() {
        let mut m = from_mesh(&square(), None).unwrap();
        m.vertices[0].point[0] += 0.5;
        assert!(validate(&m).is_err());
    }
}

#[cfg(test)]
mod holes_test {
    use super::*;
    #[test]
    fn annular_polygon_retains_inner_loop() {
        let mesh = Mesh {
            positions: vec![
                -2., -2., 0., 2., -2., 0., 2., 2., 0., -2., 2., 0., -1., -1., 0., 1., -1., 0., 1.,
                1., 0., -1., 1., 0.,
            ],
            indices: vec![
                0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7, 6, 3, 0, 4, 3, 4, 7,
            ],
            uv: None,
        };
        let m = from_mesh(&mesh, Some(&[0; 8])).unwrap();
        assert_eq!(m.faces.len(), 1);
        assert_eq!(m.faces[0].holes.len(), 1);
        assert_eq!(m.loops.len(), 2);
        validate(&m).unwrap();
    }
}
