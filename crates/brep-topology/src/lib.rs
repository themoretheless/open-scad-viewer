//! Geometry-independent indexed B-rep incidence shared by both kernels.
//! C, S and P are application-owned edge, face and parameter-curve geometry.
#![feature(
    try_blocks,
    gen_blocks,
    yield_expr,
    super_let,
    deref_patterns,
    yeet_expr
)]
#![allow(unused_features)]
pub use math_core::{Error, Result};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeSet;
pub mod persistent_naming;
pub use persistent_naming::{
    ChangeKind, ChangeProvenance, ChangeSet, TopoId, TopoKind, TopologyChange,
};
const INVALID_TOPOLOGY: &str = "BREP_INVALID_TOPOLOGY";
fn invalid(message: impl Into<String>) -> Error {
    Error::new(INVALID_TOPOLOGY, message)
}
fn require(ok: bool, message: &str) -> Result<()> {
    math_core::ensure(ok, INVALID_TOPOLOGY, message)
}
/// Resource ceilings of one validated model.
pub const MAX_ENTITIES: usize = 16384;
pub const MAX_FACES: usize = 1024;
pub const MAX_COEDGES: usize = 32768;

#[derive(Clone, Debug)]
pub struct Vertex<V = [f64; 3]> {
    pub point: V,
}
impl<V: value_codec::Serialize> value_codec::Serialize for Vertex<V> {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "point".into(),
            value_codec::Serialize::to_value(&self.point),
        );
        value_codec::Value::Object(object)
    }
}
impl<'de, V: value_codec::Deserialize<'de>> value_codec::Deserialize<'de> for Vertex<V> {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let point: V = value_codec::Deserialize::from_value(
            object
                .remove("point")
                .ok_or_else(|| value_codec::error("Missing field point"))?,
        )?;
        if let Some(key) = object.keys().next() {
            return Err(value_codec::error(format!("Unknown field {key}")));
        }
        Ok(Self { point })
    }
}
#[derive(Clone, Debug)]
pub struct Edge<C> {
    /// A surface boundary collapsed to one pole vertex. It contributes no
    /// one-dimensional incidence; geometric kernels must certify collapse.
    pub degenerate: bool,
    pub vertices: [usize; 2],
    pub curve: C,
}
impl<C: value_codec::Serialize> value_codec::Serialize for Edge<C> {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        if self.degenerate {
            object.insert("degenerate".into(), value_codec::Value::Bool(true));
        }
        object.insert(
            "vertices".into(),
            value_codec::Serialize::to_value(&self.vertices),
        );
        object.insert(
            "curve".into(),
            value_codec::Serialize::to_value(&self.curve),
        );
        value_codec::Value::Object(object)
    }
}
impl<'de, C: value_codec::Deserialize<'de>> value_codec::Deserialize<'de> for Edge<C> {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let degenerate = object
            .remove("degenerate")
            .map(value_codec::Deserialize::from_value)
            .transpose()?
            .unwrap_or(false);
        let vertices: [usize; 2] = value_codec::Deserialize::from_value(
            object
                .remove("vertices")
                .ok_or_else(|| value_codec::error("Missing field vertices"))?,
        )?;
        let curve: C = value_codec::Deserialize::from_value(
            object
                .remove("curve")
                .ok_or_else(|| value_codec::error("Missing field curve"))?,
        )?;
        if let Some(key) = object.keys().next() {
            return Err(value_codec::error(format!("Unknown field {key}")));
        }
        Ok(Self {
            degenerate,
            vertices,
            curve,
        })
    }
}
/// A face-local use of an edge. pcurve follows the traversal direction in UV.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoedgeTrim {
    /// Parameter interval on the independently represented three-dimensional curve.
    pub curve_parameter: [f64; 2],
    /// Parameter interval on the face-local pcurve.
    pub pcurve_parameter: [f64; 2],
    /// Integer lifts applied to periodic surface coordinates at the interval ends.
    pub periodic_lift: [[i32; 2]; 2],
}

impl CoedgeTrim {
    pub fn validate(self) -> Result<()> {
        require(
            self.curve_parameter
                .iter()
                .chain(&self.pcurve_parameter)
                .all(|v| v.is_finite()),
            "Coedge trim parameters must be finite",
        )?;
        require(
            self.curve_parameter[0] != self.curve_parameter[1]
                && self.pcurve_parameter[0] != self.pcurve_parameter[1],
            "Coedge trim intervals must be non-empty",
        )
    }
}

#[derive(Clone, Debug)]
pub struct Coedge<P> {
    pub edge: usize,
    pub reversed: bool,
    pub pcurve: P,
}
impl<P: value_codec::Serialize> value_codec::Serialize for Coedge<P> {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert("edge".into(), value_codec::Serialize::to_value(&self.edge));
        object.insert(
            "reversed".into(),
            value_codec::Serialize::to_value(&self.reversed),
        );
        object.insert(
            "pcurve".into(),
            value_codec::Serialize::to_value(&self.pcurve),
        );
        value_codec::Value::Object(object)
    }
}
impl<'de, P: value_codec::Deserialize<'de>> value_codec::Deserialize<'de> for Coedge<P> {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let edge: usize = value_codec::Deserialize::from_value(
            object
                .remove("edge")
                .ok_or_else(|| value_codec::error("Missing field edge"))?,
        )?;
        let reversed: bool = value_codec::Deserialize::from_value(
            object
                .remove("reversed")
                .ok_or_else(|| value_codec::error("Missing field reversed"))?,
        )?;
        let pcurve: P = value_codec::Deserialize::from_value(
            object
                .remove("pcurve")
                .ok_or_else(|| value_codec::error("Missing field pcurve"))?,
        )?;
        if let Some(key) = object.keys().next() {
            return Err(value_codec::error(format!("Unknown field {key}")));
        }
        Ok(Self {
            edge,
            reversed,
            pcurve,
        })
    }
}
#[derive(Clone, Debug)]
pub struct Loop<P> {
    pub coedges: Vec<Coedge<P>>,
}
impl<P: value_codec::Serialize> value_codec::Serialize for Loop<P> {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "coedges".into(),
            value_codec::Serialize::to_value(&self.coedges),
        );
        value_codec::Value::Object(object)
    }
}
impl<'de, P: value_codec::Deserialize<'de>> value_codec::Deserialize<'de> for Loop<P> {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let coedges: Vec<Coedge<P>> = value_codec::Deserialize::from_value(
            object
                .remove("coedges")
                .ok_or_else(|| value_codec::error("Missing field coedges"))?,
        )?;
        if let Some(key) = object.keys().next() {
            return Err(value_codec::error(format!("Unknown field {key}")));
        }
        Ok(Self { coedges })
    }
}
/// Outer loop is CCW in UV, holes CW. Shell face uses control normal reversal.
#[derive(Clone, Debug)]
pub struct Face<S> {
    pub surface: S,
    pub outer: usize,
    pub holes: Vec<usize>,
}
impl<S: value_codec::Serialize> value_codec::Serialize for Face<S> {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "surface".into(),
            value_codec::Serialize::to_value(&self.surface),
        );
        object.insert(
            "outer".into(),
            value_codec::Serialize::to_value(&self.outer),
        );
        object.insert(
            "holes".into(),
            value_codec::Serialize::to_value(&self.holes),
        );
        value_codec::Value::Object(object)
    }
}
impl<'de, S: value_codec::Deserialize<'de>> value_codec::Deserialize<'de> for Face<S> {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let surface: S = value_codec::Deserialize::from_value(
            object
                .remove("surface")
                .ok_or_else(|| value_codec::error("Missing field surface"))?,
        )?;
        let outer: usize = value_codec::Deserialize::from_value(
            object
                .remove("outer")
                .ok_or_else(|| value_codec::error("Missing field outer"))?,
        )?;
        let holes: Vec<usize> = value_codec::Deserialize::from_value(
            object
                .remove("holes")
                .ok_or_else(|| value_codec::error("Missing field holes"))?,
        )?;
        if let Some(key) = object.keys().next() {
            return Err(value_codec::error(format!("Unknown field {key}")));
        }
        Ok(Self {
            surface,
            outer,
            holes,
        })
    }
}
#[derive(Clone, Debug)]
pub struct FaceUse {
    pub face: usize,
    pub reversed: bool,
}
impl value_codec::Serialize for FaceUse {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert("face".into(), value_codec::Serialize::to_value(&self.face));
        object.insert(
            "reversed".into(),
            value_codec::Serialize::to_value(&self.reversed),
        );
        value_codec::Value::Object(object)
    }
}
impl<'de> value_codec::Deserialize<'de> for FaceUse {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let face: usize = value_codec::Deserialize::from_value(
            object
                .remove("face")
                .ok_or_else(|| value_codec::error("Missing field face"))?,
        )?;
        let reversed: bool = value_codec::Deserialize::from_value(
            object
                .remove("reversed")
                .ok_or_else(|| value_codec::error("Missing field reversed"))?,
        )?;
        if let Some(key) = object.keys().next() {
            return Err(value_codec::error(format!("Unknown field {key}")));
        }
        Ok(Self { face, reversed })
    }
}
#[derive(Clone, Debug)]
pub struct Shell {
    pub faces: Vec<FaceUse>,
    pub closed: bool,
}
impl value_codec::Serialize for Shell {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "faces".into(),
            value_codec::Serialize::to_value(&self.faces),
        );
        object.insert(
            "closed".into(),
            value_codec::Serialize::to_value(&self.closed),
        );
        value_codec::Value::Object(object)
    }
}
impl<'de> value_codec::Deserialize<'de> for Shell {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let faces: Vec<FaceUse> = value_codec::Deserialize::from_value(
            object
                .remove("faces")
                .ok_or_else(|| value_codec::error("Missing field faces"))?,
        )?;
        let closed: bool = value_codec::Deserialize::from_value(
            object
                .remove("closed")
                .ok_or_else(|| value_codec::error("Missing field closed"))?,
        )?;
        if let Some(key) = object.keys().next() {
            return Err(value_codec::error(format!("Unknown field {key}")));
        }
        Ok(Self { faces, closed })
    }
}
/// Outer and cavity shells are explicit; containment is not inferred or certified.
#[derive(Clone, Debug)]
pub struct Body {
    pub outer_shell: usize,
    pub inner_shells: Vec<usize>,
}
impl value_codec::Serialize for Body {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "outerShell".into(),
            value_codec::Serialize::to_value(&self.outer_shell),
        );
        object.insert(
            "innerShells".into(),
            value_codec::Serialize::to_value(&self.inner_shells),
        );
        value_codec::Value::Object(object)
    }
}
impl<'de> value_codec::Deserialize<'de> for Body {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let outer_shell: usize = value_codec::Deserialize::from_value(
            object
                .remove("outerShell")
                .ok_or_else(|| value_codec::error("Missing field outerShell"))?,
        )?;
        let inner_shells: Vec<usize> = value_codec::Deserialize::from_value(
            object
                .remove("innerShells")
                .ok_or_else(|| value_codec::error("Missing field innerShells"))?,
        )?;
        if let Some(key) = object.keys().next() {
            return Err(value_codec::error(format!("Unknown field {key}")));
        }
        Ok(Self {
            outer_shell,
            inner_shells,
        })
    }
}
#[derive(Clone, Debug)]
pub struct Model<C, S, P, V = [f64; 3]> {
    pub vertices: Vec<Vertex<V>>,
    pub edges: Vec<Edge<C>>,
    pub loops: Vec<Loop<P>>,
    pub faces: Vec<Face<S>>,
    pub shells: Vec<Shell>,
    pub bodies: Vec<Body>,
    pub tolerance_mm: f64,
}
impl<
    C: value_codec::Serialize,
    S: value_codec::Serialize,
    P: value_codec::Serialize,
    V: value_codec::Serialize,
> value_codec::Serialize for Model<C, S, P, V>
{
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "vertices".into(),
            value_codec::Serialize::to_value(&self.vertices),
        );
        object.insert(
            "edges".into(),
            value_codec::Serialize::to_value(&self.edges),
        );
        object.insert(
            "loops".into(),
            value_codec::Serialize::to_value(&self.loops),
        );
        object.insert(
            "faces".into(),
            value_codec::Serialize::to_value(&self.faces),
        );
        object.insert(
            "shells".into(),
            value_codec::Serialize::to_value(&self.shells),
        );
        object.insert(
            "bodies".into(),
            value_codec::Serialize::to_value(&self.bodies),
        );
        object.insert(
            "toleranceMm".into(),
            value_codec::Serialize::to_value(&self.tolerance_mm),
        );
        if let Ok(context) =
            cad_predicates::ToleranceContext::from_brep_tolerance_mm(self.tolerance_mm)
        {
            object.insert(
                "toleranceContext".into(),
                value_codec::Serialize::to_value(&context),
            );
        }
        value_codec::Value::Object(object)
    }
}
impl<
    'de,
    C: value_codec::Deserialize<'de>,
    S: value_codec::Deserialize<'de>,
    P: value_codec::Deserialize<'de>,
    V: value_codec::Deserialize<'de>,
> value_codec::Deserialize<'de> for Model<C, S, P, V>
{
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let vertices: Vec<Vertex<V>> = value_codec::Deserialize::from_value(
            object
                .remove("vertices")
                .ok_or_else(|| value_codec::error("Missing field vertices"))?,
        )?;
        let edges: Vec<Edge<C>> = value_codec::Deserialize::from_value(
            object
                .remove("edges")
                .ok_or_else(|| value_codec::error("Missing field edges"))?,
        )?;
        let loops: Vec<Loop<P>> = value_codec::Deserialize::from_value(
            object
                .remove("loops")
                .ok_or_else(|| value_codec::error("Missing field loops"))?,
        )?;
        let faces: Vec<Face<S>> = value_codec::Deserialize::from_value(
            object
                .remove("faces")
                .ok_or_else(|| value_codec::error("Missing field faces"))?,
        )?;
        let shells: Vec<Shell> = value_codec::Deserialize::from_value(
            object
                .remove("shells")
                .ok_or_else(|| value_codec::error("Missing field shells"))?,
        )?;
        let bodies: Vec<Body> = value_codec::Deserialize::from_value(
            object
                .remove("bodies")
                .ok_or_else(|| value_codec::error("Missing field bodies"))?,
        )?;
        let legacy_tolerance_mm = object
            .remove("toleranceMm")
            .map(value_codec::Deserialize::from_value)
            .transpose()?;
        let context: Option<cad_predicates::ToleranceContext> = object
            .remove("toleranceContext")
            .map(value_codec::Deserialize::from_value)
            .transpose()?;
        let tolerance_mm = match (context, legacy_tolerance_mm) {
            (Some(context), legacy) => {
                let on_mm = context.spatial_bounds().on_mm;
                if legacy.is_some_and(|value: f64| value.to_bits() != on_mm.to_bits()) {
                    return Err(value_codec::error(
                        "toleranceMm conflicts with toleranceContext",
                    ));
                }
                on_mm
            }
            (None, Some(value)) => {
                cad_predicates::ToleranceContext::from_brep_tolerance_mm(value)
                    .map_err(|_| value_codec::error("Invalid legacy toleranceMm"))?;
                value
            }
            (None, None) => {
                return Err(value_codec::error(
                    "Missing toleranceContext or legacy toleranceMm",
                ));
            }
        };
        if let Some(key) = object.keys().next() {
            return Err(value_codec::error(format!("Unknown field {key}")));
        }
        Ok(Self {
            vertices,
            edges,
            loops,
            faces,
            shells,
            bodies,
            tolerance_mm,
        })
    }
}

impl<C, S, P> Model<C, S, P> {
    pub fn validate_topology(&self) -> Result<()> {
        self.validate_topology_with_vertices(|point| {
            require(
                point.iter().all(|x| x.is_finite() && x.abs() <= 1e6),
                "Invalid vertex coordinates",
            )
        })
    }
}
impl<C, S, P, V> Model<C, S, P, V> {
    /// Canonical immutable predicate context derived from this model's legacy
    /// scalar. New serialized models also carry its versioned specification.
    pub fn tolerance_context(&self) -> Result<cad_predicates::ToleranceContext> {
        cad_predicates::ToleranceContext::from_brep_tolerance_mm(self.tolerance_mm)
            .map_err(|_| invalid("Invalid B-rep tolerance context"))
    }

    /// Validate indexed incidence with application-owned vertex admission.
    /// The callback must validate geometry/provenance in its owning context;
    /// this method alone does not certify geometric or solid validity.
    pub fn validate_topology_with_vertices(
        &self,
        mut validate_vertex: impl FnMut(&V) -> Result<()>,
    ) -> Result<()> {
        let count = self.vertices.len()
            + self.edges.len()
            + self.loops.len()
            + self.faces.len()
            + self.shells.len()
            + self.bodies.len();
        let uses = self.loops.iter().map(|l| l.coedges.len()).sum::<usize>();
        // Gear bodies carry a few hundred faces (six per tooth, twice for a
        // herringbone); the ceiling stays a guard against runaway authoring.
        if count > MAX_ENTITIES || uses > MAX_COEDGES || self.faces.len() > MAX_FACES {
            return Err(Error {
                code: "BREP_RESOURCE_LIMIT",
                message: format!(
                    "B-rep exceeds {MAX_ENTITIES} entities, {MAX_FACES} faces or {MAX_COEDGES} coedges"
                ),
            });
        }
        require(
            self.tolerance_mm.is_finite() && (1e-10..=1e-2).contains(&self.tolerance_mm),
            "Invalid B-rep tolerance",
        )?;
        let mut vertices = vec![false; self.vertices.len()];
        for v in &self.vertices {
            validate_vertex(&v.point)?;
        }
        for e in &self.edges {
            require(
                !e.degenerate || e.vertices[0] == e.vertices[1],
                "Collapsed boundary must reference one pole vertex",
            )?;
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
            let mut incidence = FxHashMap::<usize, Vec<(usize, bool)>>::default();
            let mut links = FxHashMap::<usize, Vec<(usize, usize)>>::default();
            let mut pole_uses = FxHashMap::<usize, FxHashSet<usize>>::default();
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
                    let all = &self.loops[*l].coedges;
                    for c in all.iter().filter(|c| self.edges[c.edge].degenerate) {
                        require(*l == f.outer, "Collapsed boundary cannot belong to a hole")?;
                        require(
                            pole_uses.entry(c.edge).or_default().insert(u.face),
                            "Collapsed boundary is repeated on one face",
                        )?;
                    }
                    let cs: Vec<_> = all
                        .iter()
                        .filter(|c| !self.edges[c.edge].degenerate)
                        .collect();
                    require(
                        !cs.is_empty() && (cs.len() == all.len() || cs.len() >= 2),
                        "Pole face needs at least two ordinary boundary edges",
                    )?;
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
            let mut adjacency = FxHashMap::<usize, Vec<usize>>::default();
            for (edge_index, edges) in &incidence {
                require(
                    edges.len() <= 2,
                    "Non-manifold edge has more than two face uses",
                )?;
                if edges.len() == 1 {
                    require(!s.closed, "Closed shell has a boundary edge")?;
                } else {
                    require(
                        edges[0].1 != edges[1].1,
                        &format!(
                            "Adjacent face uses traverse edge {edge_index} in the same direction ({:?})",
                            edges
                        ),
                    )?;
                    adjacency.entry(edges[0].0).or_default().push(edges[1].0);
                    adjacency.entry(edges[1].0).or_default().push(edges[0].0);
                }
            }
            let mut seen = FxHashSet::default();
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
                let mut graph = FxHashMap::<usize, Vec<usize>>::default();
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
                let mut seen = FxHashSet::default();
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

type VertexAdmission<'a, V> = dyn Fn(&V) -> Result<()> + 'a;
type ModelAdmission<'a, C, S, P, V> = dyn Fn(&Model<C, S, P, V>) -> Result<()> + 'a;

/// Immutable indexed topology plus a retained owner admission policy.
/// Certifies incidence/admission only, not geometric solid validity. Geometry
/// payloads and the policy must themselves be immutable; external side effects
/// in edit/admission callbacks are outside this transaction boundary.
pub struct TopologySnapshot<'a, C, S, P, V = [f64; 3]> {
    model: std::sync::Arc<Model<C, S, P, V>>,
    validate_vertex: std::sync::Arc<VertexAdmission<'a, V>>,
    validate_model: std::sync::Arc<ModelAdmission<'a, C, S, P, V>>,
}
impl<C, S, P, V> Clone for TopologySnapshot<'_, C, S, P, V> {
    fn clone(&self) -> Self {
        Self {
            model: self.model.clone(),
            validate_vertex: self.validate_vertex.clone(),
            validate_model: self.validate_model.clone(),
        }
    }
}
impl<'a, C, S, P, V> TopologySnapshot<'a, C, S, P, V> {
    pub fn new(
        model: Model<C, S, P, V>,
        validate_vertex: impl Fn(&V) -> Result<()> + 'a,
    ) -> Result<Self> {
        Self::new_with_model_validation(model, validate_vertex, |_| Ok(()))
    }
    /// Retain additional owner checks for curves, surfaces and their incidence.
    /// Both callbacks are reused on every candidate edit.
    pub fn new_with_model_validation(
        model: Model<C, S, P, V>,
        validate_vertex: impl Fn(&V) -> Result<()> + 'a,
        validate_model: impl Fn(&Model<C, S, P, V>) -> Result<()> + 'a,
    ) -> Result<Self> {
        model.validate_topology_with_vertices(&validate_vertex)?;
        validate_model(&model)?;
        Ok(Self {
            model: std::sync::Arc::new(model),
            validate_vertex: std::sync::Arc::new(validate_vertex),
            validate_model: std::sync::Arc::new(validate_model),
        })
    }
    pub fn model(&self) -> &Model<C, S, P, V> {
        &self.model
    }
}
impl<C: Clone, S: Clone, P: Clone, V: Clone> TopologySnapshot<'_, C, S, P, V> {
    /// Edit an isolated topology copy and publish a new snapshot only after the
    /// same retained owner policy and all incidence checks succeed.
    pub fn try_edit(
        &self,
        edit: impl FnOnce(&mut Model<C, S, P, V>) -> Result<()>,
    ) -> Result<Self> {
        let mut candidate = (*self.model).clone();
        edit(&mut candidate)?;
        candidate.validate_topology_with_vertices(|v| (self.validate_vertex)(v))?;
        (self.validate_model)(&candidate)?;
        Ok(Self {
            model: std::sync::Arc::new(candidate),
            validate_vertex: self.validate_vertex.clone(),
            validate_model: self.validate_model.clone(),
        })
    }
}

/// Single-writer in-memory revision store; not a persisted/distributed CAS.
pub const MAX_TOPOLOGY_HISTORY: usize = 32;
/// Owner-defined immutable admitted state. Implementations must validate every
/// edited candidate before returning it and preserve their admission policy.
/// This contract does not itself certify geometric solids.
pub trait RevisionState: Clone {
    type Model;
    /// History restoration must not replace admission with a weaker policy.
    fn same_admission_policy(&self, other: &Self) -> bool;
    fn model(&self) -> &Self::Model;
    fn try_edit(&self, edit: impl FnOnce(&mut Self::Model) -> Result<()>) -> Result<Self>;
}
impl<C: Clone, S: Clone, P: Clone, V: Clone> RevisionState for TopologySnapshot<'_, C, S, P, V> {
    type Model = Model<C, S, P, V>;
    fn same_admission_policy(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.validate_vertex, &other.validate_vertex)
            && std::sync::Arc::ptr_eq(&self.validate_model, &other.validate_model)
    }
    fn model(&self) -> &Self::Model {
        TopologySnapshot::model(self)
    }
    fn try_edit(&self, edit: impl FnOnce(&mut Self::Model) -> Result<()>) -> Result<Self> {
        TopologySnapshot::try_edit(self, edit)
    }
}
pub type TopologyStore<'a, C, S, P, V = [f64; 3]> = RevisionStore<TopologySnapshot<'a, C, S, P, V>>;
pub type TopologyCheckout<'a, C, S, P, V = [f64; 3]> =
    RevisionCheckout<TopologySnapshot<'a, C, S, P, V>>;
pub type TopologyTransaction<'a, C, S, P, V = [f64; 3]> =
    RevisionTransaction<TopologySnapshot<'a, C, S, P, V>>;
pub struct RevisionStore<T: RevisionState> {
    current: T,
    undo: Vec<T>,
    redo: Vec<T>,
    identity: std::sync::Arc<()>,
    revision: u64,
}
pub struct RevisionCheckout<T: RevisionState> {
    snapshot: T,
    identity: std::sync::Arc<()>,
    revision: u64,
}
pub struct RevisionTransaction<T: RevisionState> {
    candidate: T,
    identity: std::sync::Arc<()>,
    base_revision: u64,
}
impl<T: RevisionState> RevisionStore<T> {
    pub fn new(snapshot: T) -> Self {
        Self {
            current: snapshot,
            undo: Vec::new(),
            redo: Vec::new(),
            identity: std::sync::Arc::new(()),
            revision: 0,
        }
    }
    /// Restore already admitted states into a fresh store identity/revision.
    /// Histories are ordered oldest-to-newest, with the next restoration last.
    pub fn from_history(current: T, undo: Vec<T>, redo: Vec<T>) -> Result<Self> {
        if undo.len().saturating_add(redo.len()) > MAX_TOPOLOGY_HISTORY {
            return Err(Error::new(
                "BREP_RESOURCE_LIMIT",
                "History exceeds 32 snapshots",
            ));
        }
        if undo
            .iter()
            .chain(&redo)
            .any(|snapshot| !current.same_admission_policy(snapshot))
        {
            return Err(Error::new(
                "BREP_HISTORY_POLICY_MISMATCH",
                "History snapshots use different admission policies",
            ));
        }
        let mut store = Self::new(current);
        store.undo = undo;
        store.redo = redo;
        Ok(store)
    }
    pub fn history_snapshots(&self) -> (&[T], &[T]) {
        (&self.undo, &self.redo)
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn snapshot(&self) -> &T {
        &self.current
    }
    pub fn history_lengths(&self) -> (usize, usize) {
        (self.undo.len(), self.redo.len())
    }
    /// Restoring geometry still advances revision; pending checkouts stay stale.
    pub fn undo(&mut self) -> Result<bool> {
        self.restore_history(false)
    }
    pub fn redo(&mut self) -> Result<bool> {
        self.restore_history(true)
    }
    fn restore_history(&mut self, redo: bool) -> Result<bool> {
        let (source, target) = if redo {
            (&mut self.redo, &mut self.undo)
        } else {
            (&mut self.undo, &mut self.redo)
        };
        if source.is_empty() {
            return Ok(false);
        }
        let next = self.revision.checked_add(1).ok_or_else(|| {
            Error::new("BREP_REVISION_EXHAUSTED", "Topology revision cannot wrap")
        })?;
        let restored = source.pop().expect("History was checked nonempty");
        target.push(std::mem::replace(&mut self.current, restored));
        self.revision = next;
        Ok(true)
    }
    pub fn checkout(&self) -> RevisionCheckout<T> {
        RevisionCheckout {
            snapshot: self.current.clone(),
            identity: self.identity.clone(),
            revision: self.revision,
        }
    }
    /// Consumes an already validated edit. Stale/foreign/overflow failures leave
    /// current state unchanged. Every commit advances revision, even a no-op.
    pub fn commit(&mut self, transaction: RevisionTransaction<T>) -> Result<u64> {
        self.commit_with_check(transaction, |_| Ok(()))
    }
    /// Run a final caller check (e.g. cancellation/deadline) after identity and
    /// revision validation, immediately before replacing state. Failure leaves
    /// the current snapshot, revision and both history stacks unchanged.
    pub fn commit_with_check(
        &mut self,
        transaction: RevisionTransaction<T>,
        check: impl FnOnce(&T::Model) -> Result<()>,
    ) -> Result<u64> {
        if !std::sync::Arc::ptr_eq(&self.identity, &transaction.identity) {
            return Err(Error::new(
                "BREP_FOREIGN_TRANSACTION",
                "Transaction belongs to another topology store",
            ));
        }
        if self.revision != transaction.base_revision {
            return Err(Error::new(
                "BREP_STALE_TRANSACTION",
                "Topology changed after checkout",
            ));
        }
        let next = self.revision.checked_add(1).ok_or_else(|| {
            Error::new("BREP_REVISION_EXHAUSTED", "Topology revision cannot wrap")
        })?;
        check(transaction.candidate.model())?;
        let previous = std::mem::replace(&mut self.current, transaction.candidate);
        if self.undo.len() == MAX_TOPOLOGY_HISTORY {
            self.undo.remove(0);
        }
        self.undo.push(previous);
        self.redo.clear();
        self.revision = next;
        Ok(next)
    }
}
impl<T: RevisionState> RevisionCheckout<T> {
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn snapshot(&self) -> &T {
        &self.snapshot
    }
}
impl<T: RevisionState> RevisionCheckout<T> {
    pub fn prepare(
        &self,
        edit: impl FnOnce(&mut T::Model) -> Result<()>,
    ) -> Result<RevisionTransaction<T>> {
        let candidate = self.snapshot.try_edit(edit)?;
        if !self.snapshot.same_admission_policy(&candidate) {
            return Err(Error::new(
                "BREP_TRANSACTION_POLICY_MISMATCH",
                "Prepared edit changed admission policy",
            ));
        }
        Ok(RevisionTransaction {
            candidate,
            identity: self.identity.clone(),
            base_revision: self.revision,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn revision_exhaustion_never_wraps_or_publishes_candidate() {
        let snapshot = TopologySnapshot::new(sheet(), |point| {
            require(point.iter().all(|x| x.is_finite()), "Invalid point")
        })
        .unwrap();
        let mut store = TopologyStore::new(snapshot);
        store.revision = u64::MAX;
        let old_tolerance = store.snapshot().model().tolerance_mm;
        let edit = store
            .checkout()
            .prepare(|m| {
                m.tolerance_mm = 2e-6;
                Ok(())
            })
            .unwrap();
        assert_eq!(
            store.commit(edit).unwrap_err().code,
            "BREP_REVISION_EXHAUSTED"
        );
        assert_eq!(store.revision(), u64::MAX);
        assert_eq!(store.snapshot().model().tolerance_mm, old_tolerance);
    }
    #[test]
    fn history_is_bounded_and_branching_invalidates_redo() {
        let snapshot = TopologySnapshot::new(sheet(), |_| Ok(())).unwrap();
        let mut store = TopologyStore::new(snapshot);
        assert!(!store.undo().unwrap());
        assert!(!store.redo().unwrap());
        assert_eq!(store.revision(), 0);
        for i in 0..40 {
            let tx = store
                .checkout()
                .prepare(|m| {
                    m.tolerance_mm = (i + 1) as f64 * 1e-6;
                    Ok(())
                })
                .unwrap();
            store.commit(tx).unwrap();
        }
        assert_eq!(store.history_lengths(), (MAX_TOPOLOGY_HISTORY, 0));
        for _ in 0..MAX_TOPOLOGY_HISTORY {
            assert!(store.undo().unwrap());
        }
        assert!(!store.undo().unwrap());
        assert_eq!(store.history_lengths(), (0, MAX_TOPOLOGY_HISTORY));
        assert_eq!(store.snapshot().model().tolerance_mm, 8e-6);
        assert!(store.redo().unwrap());
        let tx = store
            .checkout()
            .prepare(|m| {
                m.tolerance_mm = 5e-6;
                Ok(())
            })
            .unwrap();
        store.commit(tx).unwrap();
        assert!(!store.redo().unwrap());
        assert_eq!(store.history_lengths(), (2, 0));
        store.revision = u64::MAX;
        assert!(store.undo().is_err());
        assert_eq!(store.history_lengths(), (2, 0));
        assert_eq!(store.snapshot().model().tolerance_mm, 5e-6);
    }
    #[test]
    fn restored_history_cannot_change_owner_admission_policy() {
        let original = TopologySnapshot::new(sheet(), |_| Ok(())).unwrap();
        let foreign = TopologySnapshot::new(sheet(), |_| Ok(())).unwrap();
        assert!(TopologyStore::from_history(original.clone(), vec![foreign], vec![]).is_err());
        assert!(TopologyStore::from_history(original.clone(), vec![original], vec![]).is_ok());
    }
    #[test]
    fn prepared_state_cannot_substitute_its_admission_policy() {
        #[derive(Clone)]
        struct OwnerState {
            value: u32,
            policy: u32,
        }
        impl RevisionState for OwnerState {
            type Model = u32;
            fn model(&self) -> &u32 {
                &self.value
            }
            fn same_admission_policy(&self, other: &Self) -> bool {
                self.policy == other.policy
            }
            fn try_edit(&self, edit: impl FnOnce(&mut u32) -> Result<()>) -> Result<Self> {
                let mut next = self.clone();
                edit(&mut next.value)?;
                next.policy += 1; // Deliberately faulty owner implementation.
                Ok(next)
            }
        }
        let store = RevisionStore::new(OwnerState {
            value: 7,
            policy: 1,
        });
        let result = store.checkout().prepare(|v| {
            *v = 9;
            Ok(())
        });
        assert!(matches!(
            result,
            Err(Error {
                code: "BREP_TRANSACTION_POLICY_MISMATCH",
                ..
            })
        ));
        assert_eq!(*store.snapshot().model(), 7);
        assert_eq!(store.revision(), 0);
        assert_eq!(store.history_lengths(), (0, 0));
    }
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
                    degenerate: false,
                    vertices: [0, 1],
                    curve: (),
                },
                Edge {
                    degenerate: false,
                    vertices: [1, 2],
                    curve: (),
                },
                Edge {
                    degenerate: false,
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
    fn legacy_tolerance_migrates_to_versioned_context() {
        let model = sheet();
        let mut value = value_codec::Serialize::to_value(&model);
        value.as_object_mut().unwrap().remove("toleranceContext");
        let restored: Model<(), (), ()> = value_codec::Deserialize::from_value(value).unwrap();
        let context = restored.tolerance_context().unwrap();
        assert_eq!(context.spatial_bounds().on_mm, model.tolerance_mm);

        let canonical = value_codec::Serialize::to_value(&restored);
        assert!(canonical.get("toleranceMm").is_some());
        assert!(canonical.get("toleranceContext").is_some());
    }
    #[test]
    fn conflicting_legacy_and_context_tolerances_are_rejected() {
        let mut value = value_codec::Serialize::to_value(&sheet());
        *value.get_mut("toleranceMm").unwrap() = value_codec::Serialize::to_value(&2e-6_f64);
        assert!(<Model<(), (), ()> as value_codec::Deserialize>::from_value(value).is_err());
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
