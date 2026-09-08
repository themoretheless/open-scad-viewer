//! Geometry-independent indexed B-rep incidence shared by both kernels.
//! C, S and P are application-owned edge, face and parameter-curve geometry.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Serialize)]
pub struct Error {
    pub code: &'static str,
    pub message: String,
}
pub type Result<T> = std::result::Result<T, Error>;
fn invalid(message: impl Into<String>) -> Error {
    Error {
        code: "BREP_INVALID_TOPOLOGY",
        message: message.into(),
    }
}
fn require(ok: bool, message: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(invalid(message))
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Vertex {
    pub point: [f64; 3],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Edge<C> {
    pub vertices: [usize; 2],
    pub curve: C,
}
/// A face-local use of an edge. pcurve follows the traversal direction in UV.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coedge<P> {
    pub edge: usize,
    pub reversed: bool,
    pub pcurve: P,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Loop<P> {
    pub coedges: Vec<Coedge<P>>,
}
/// Outer loop is CCW in UV, holes CW. Shell face uses control normal reversal.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Face<S> {
    pub surface: S,
    pub outer: usize,
    pub holes: Vec<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FaceUse {
    pub face: usize,
    pub reversed: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shell {
    pub faces: Vec<FaceUse>,
    pub closed: bool,
}
/// Outer and cavity shells are explicit; containment is not inferred or certified.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Body {
    pub outer_shell: usize,
    pub inner_shells: Vec<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Model<C, S, P> {
    pub vertices: Vec<Vertex>,
    pub edges: Vec<Edge<C>>,
    pub loops: Vec<Loop<P>>,
    pub faces: Vec<Face<S>>,
    pub shells: Vec<Shell>,
    pub bodies: Vec<Body>,
    pub tolerance_mm: f64,
}

impl<C, S, P> Model<C, S, P> {
    pub fn validate_topology(&self) -> Result<()> {
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
            "Invalid B-rep tolerance",
        )?;
        let mut vertices = vec![false; self.vertices.len()];
        for v in &self.vertices {
            require(
                v.point.iter().all(|x| x.is_finite() && x.abs() <= 1e6),
                "Invalid vertex coordinates",
            )?;
        }
        for e in &self.edges {
            for &v in &e.vertices {
                require(v < vertices.len(), "Unknown edge vertex")?;
                vertices[v] = true;
            }
        }
        require(vertices.iter().all(|v| *v), "Unused vertex")?;
        let mut loops = vec![false; self.loops.len()];
        let mut edges = vec![false; self.edges.len()];
        for f in &self.faces {
            for &l in std::iter::once(&f.outer).chain(&f.holes) {
                require(l < loops.len(), "Unknown face loop")?;
                require(!loops[l], "Loop reused by faces")?;
                loops[l] = true;
                let cs = &self.loops[l].coedges;
                require(!cs.is_empty(), "Empty loop")?;
                for (i, c) in cs.iter().enumerate() {
                    let e = self
                        .edges
                        .get(c.edge)
                        .ok_or_else(|| invalid("Unknown coedge edge"))?;
                    edges[c.edge] = true;
                    let next = &cs[(i + 1) % cs.len()];
                    let next_edge = self
                        .edges
                        .get(next.edge)
                        .ok_or_else(|| invalid("Unknown coedge edge"))?;
                    require(
                        e.vertices[usize::from(!c.reversed)]
                            == next_edge.vertices[usize::from(next.reversed)],
                        "Disconnected loop vertex chain",
                    )?;
                }
            }
        }
        require(
            loops.iter().all(|v| *v) && edges.iter().all(|v| *v),
            "Unused loop or edge",
        )?;
        let mut face_owner = vec![None; self.faces.len()];
        for (si, s) in self.shells.iter().enumerate() {
            require(!s.faces.is_empty(), "Empty shell")?;
            let mut incidence = BTreeMap::<usize, Vec<(usize, bool)>>::new();
            let mut links = BTreeMap::<usize, Vec<(usize, usize)>>::new();
            for u in &s.faces {
                let f = self
                    .faces
                    .get(u.face)
                    .ok_or_else(|| invalid("Shell references unknown face"))?;
                require(
                    face_owner[u.face].replace(si).is_none(),
                    "Face belongs to multiple shells or is repeated",
                )?;
                for l in std::iter::once(&f.outer).chain(&f.holes) {
                    let cs = &self.loops[*l].coedges;
                    for (i, c) in cs.iter().enumerate() {
                        incidence
                            .entry(c.edge)
                            .or_default()
                            .push((u.face, c.reversed ^ u.reversed));
                        let next = &cs[(i + 1) % cs.len()];
                        let vertex = self.edges[c.edge].vertices[usize::from(!c.reversed)];
                        links.entry(vertex).or_default().push((c.edge, next.edge));
                    }
                }
            }
            let mut adjacency = BTreeMap::<usize, Vec<usize>>::new();
            for edges in incidence.values() {
                require(
                    edges.len() <= 2,
                    "Non-manifold edge has more than two face uses",
                )?;
                if edges.len() == 1 {
                    require(!s.closed, "Closed shell has a boundary edge")?;
                } else {
                    require(
                        edges[0].1 != edges[1].1,
                        "Adjacent face uses traverse an edge in the same direction",
                    )?;
                    adjacency.entry(edges[0].0).or_default().push(edges[1].0);
                    adjacency.entry(edges[1].0).or_default().push(edges[0].0);
                }
            }
            let mut seen = BTreeSet::new();
            let mut queue = vec![s.faces[0].face];
            while let Some(f) = queue.pop() {
                if seen.insert(f) {
                    queue.extend(adjacency.get(&f).into_iter().flatten());
                }
            }
            require(
                seen.len() == s.faces.len(),
                "Shell has disconnected face components",
            )?;
            for fan in links.values() {
                let mut graph = BTreeMap::<usize, Vec<usize>>::new();
                for (a, b) in fan {
                    graph.entry(*a).or_default().push(*b);
                    graph.entry(*b).or_default().push(*a);
                }
                require(
                    graph
                        .values()
                        .all(|n| n.len() <= 2 && (!s.closed || n.len() == 2)),
                    "Non-manifold vertex link",
                )?;
                let mut seen = BTreeSet::new();
                let mut q = vec![*graph.keys().next().unwrap()];
                while let Some(e) = q.pop() {
                    if seen.insert(e) {
                        q.extend(&graph[&e]);
                    }
                }
                require(
                    seen.len() == graph.len(),
                    "Vertex has disconnected face fans",
                )?;
            }
        }
        require(face_owner.iter().all(Option::is_some), "Unused face")?;
        let mut owned = BTreeSet::new();
        for b in &self.bodies {
            for s in std::iter::once(&b.outer_shell).chain(&b.inner_shells) {
                require(
                    self.shells
                        .get(*s)
                        .ok_or_else(|| invalid("Body references unknown shell"))?
                        .closed,
                    "Body requires closed shells",
                )?;
                require(owned.insert(*s), "Shell reused by multiple body boundaries")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sheet() -> Model<(), (), ()> {
        Model {
            vertices: vec![
                Vertex {
                    point: [0., 0., 0.],
                },
                Vertex {
                    point: [1., 0., 0.],
                },
                Vertex {
                    point: [0., 1., 0.],
                },
            ],
            edges: vec![
                Edge {
                    vertices: [0, 1],
                    curve: (),
                },
                Edge {
                    vertices: [1, 2],
                    curve: (),
                },
                Edge {
                    vertices: [2, 0],
                    curve: (),
                },
            ],
            loops: vec![Loop {
                coedges: (0..3)
                    .map(|edge| Coedge {
                        edge,
                        reversed: false,
                        pcurve: (),
                    })
                    .collect(),
            }],
            faces: vec![Face {
                surface: (),
                outer: 0,
                holes: vec![],
            }],
            shells: vec![Shell {
                faces: vec![FaceUse {
                    face: 0,
                    reversed: false,
                }],
                closed: false,
            }],
            bodies: vec![],
            tolerance_mm: 1e-7,
        }
    }
    #[test]
    fn no_geometry_kernel_required() {
        sheet().validate_topology().unwrap();
    }
    #[test]
    fn closed_shell_requires_two_opposite_uses() {
        let mut m = sheet();
        m.shells[0].closed = true;
        assert!(m.validate_topology().is_err());
    }
    #[test]
    fn bodies_cannot_reference_open_shells() {
        let mut m = sheet();
        m.bodies.push(Body {
            outer_shell: 0,
            inner_shells: vec![],
        });
        assert!(m.validate_topology().is_err());
    }
    #[test]
    fn broken_reference_is_a_typed_error() {
        let mut m = sheet();
        m.loops[0].coedges[1].edge = 99;
        assert_eq!(
            m.validate_topology().unwrap_err().code,
            "BREP_INVALID_TOPOLOGY"
        );
    }
}
