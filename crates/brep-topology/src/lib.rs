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
#[cfg(feature = "codec")]
#[path = "serialization.rs"]
mod serialization;
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

#[derive(Clone, Debug, PartialEq)]
pub struct Vertex<V = [f64; 3]> {
    pub point: V,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Edge<C> {
    /// A surface boundary collapsed to one pole vertex. It contributes no
    /// one-dimensional incidence; geometric kernels must certify collapse.
    pub degenerate: bool,
    pub vertices: [usize; 2],
    pub curve: C,
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

#[derive(Clone, Debug, PartialEq)]
pub struct Coedge<P> {
    pub edge: usize,
    /// Reverses only the three-dimensional curve traversal. Stored trim domains
    /// retain their authored direction and are never sorted by this flag.
    pub reversed: bool,
    pub pcurve: P,
    trim: Option<CoedgeTrim>,
}

impl<P> Coedge<P> {
    pub fn new(edge: usize, reversed: bool, pcurve: P) -> Self {
        Self {
            edge,
            reversed,
            pcurve,
            trim: None,
        }
    }

    pub fn trim(&self) -> Option<CoedgeTrim> {
        self.trim
    }

    pub fn with_trim(mut self, trim: CoedgeTrim) -> Result<Self> {
        trim.validate()?;
        self.trim = Some(trim);
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Loop<P> {
    pub coedges: Vec<Coedge<P>>,
}

/// Outer loop is CCW in UV, holes CW. Shell face uses control normal reversal.
#[derive(Clone, Debug, PartialEq)]
pub struct Face<S> {
    pub surface: S,
    pub outer: usize,
    pub holes: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FaceUse {
    pub face: usize,
    pub reversed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Shell {
    pub faces: Vec<FaceUse>,
    pub closed: bool,
}

/// Outer and cavity shells are explicit; containment is not inferred or certified.
#[derive(Clone, Debug, PartialEq)]
pub struct Body {
    pub outer_shell: usize,
    pub inner_shells: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Model<C, S, P, V = [f64; 3]> {
    pub vertices: Vec<Vertex<V>>,
    pub edges: Vec<Edge<C>>,
    pub loops: Vec<Loop<P>>,
    pub faces: Vec<Face<S>>,
    pub shells: Vec<Shell>,
    pub bodies: Vec<Body>,
    pub tolerance_mm: f64,
}

/// Which indexed entities an edit may have changed, for incremental
/// revalidation. Appended entities (indices at or beyond the pre-edit length)
/// are always implied; shrinking any array forces full validation because
/// indices shift. The scope must list every pre-existing index whose
/// validation-relevant data changed: vertex payloads, edge vertex
/// references/degenerate flags, coedge edge references/reversed flags, face
/// loop references, shell face lists/closed flags. Pure curve/surface/pcurve
/// payload edits need no entries; tolerance changes need none either.
/// Under-reporting is a contract violation: the snapshot may then publish a
/// candidate the full validator would reject.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EditScope {
    full: bool,
    vertices: Vec<usize>,
    edges: Vec<usize>,
    loops: Vec<usize>,
    faces: Vec<usize>,
    shells: Vec<usize>,
}
impl EditScope {
    /// Full revalidation; equivalent to [`TopologySnapshot::try_edit`].
    pub fn full() -> Self {
        Self {
            full: true,
            ..Self::default()
        }
    }
    /// Nothing topology-relevant changed (geometry payloads only).
    pub fn none() -> Self {
        Self::default()
    }
    pub fn vertex(mut self, index: usize) -> Self {
        self.vertices.push(index);
        self
    }
    pub fn edge(mut self, index: usize) -> Self {
        self.edges.push(index);
        self
    }
    pub fn loop_(mut self, index: usize) -> Self {
        self.loops.push(index);
        self
    }
    pub fn face(mut self, index: usize) -> Self {
        self.faces.push(index);
        self
    }
    pub fn shell(mut self, index: usize) -> Self {
        self.shells.push(index);
        self
    }
}
/// Which vertices, faces and shells must be rechecked after an edit.
/// Everything else in validation is cheap and always runs in full so that
/// error selection and ordering match full validation exactly.
struct ValidationPlan {
    full: bool,
    vertices: FxHashSet<usize>,
    faces: FxHashSet<usize>,
    shells: FxHashSet<usize>,
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
/// Topological invariants (Euler-Poincaré characteristic, genus) and fast adjacency queries.
/// Vertex index within a `Model`.
pub type VertexId = usize;
/// Edge index within a `Model`.
pub type EdgeId = usize;
/// Face index within a `Model`.
pub type FaceId = usize;
/// Convenience type alias for a B-rep topological model.
pub type Topology<C, S, P, V = [f64; 3]> = Model<C, S, P, V>;

pub mod explorer {
    use super::{EdgeId, FaceId, Model, VertexId};
    use rustc_hash::FxHashSet;

    /// Euler-Poincaré characteristic $\chi = V - E + F - H$ of the entire B-rep model,
    /// where $V$ is the number of non-degenerate vertices, $E$ is the number of
    /// non-degenerate edges, $F$ is the number of faces, and $H$ is the number of inner
    /// hole loops across all faces.
    pub fn euler_characteristic<C, S, P, V>(model: &Model<C, S, P, V>) -> i64 {
        let mut vertices = FxHashSet::default();
        let mut edges = FxHashSet::default();
        let mut hole_loops = 0i64;
        for f in &model.faces {
            hole_loops += f.holes.len() as i64;
            for &loop_id in std::iter::once(&f.outer).chain(&f.holes) {
                let Some(wire) = model.loops.get(loop_id) else {
                    continue;
                };
                for coedge in &wire.coedges {
                    let Some(edge) = model.edges.get(coedge.edge) else {
                        continue;
                    };
                    if !edge.degenerate {
                        edges.insert(coedge.edge);
                        vertices.insert(edge.vertices[0]);
                        vertices.insert(edge.vertices[1]);
                    }
                }
            }
        }
        let v = vertices.len() as i64;
        let e = edges.len() as i64;
        let f = model.faces.len() as i64;
        v - e + f - hole_loops
    }

    /// Euler-Poincaré characteristic $\chi = V - E + F - H$ of a single shell `shell_id`.
    pub fn shell_euler_characteristic<C, S, P, V>(
        model: &Model<C, S, P, V>,
        shell_id: usize,
    ) -> Option<i64> {
        let shell = model.shells.get(shell_id)?;
        let mut vertices = FxHashSet::default();
        let mut edges = FxHashSet::default();
        let mut hole_loops = 0i64;
        for face_use in &shell.faces {
            let f = model.faces.get(face_use.face)?;
            hole_loops += f.holes.len() as i64;
            for &loop_id in std::iter::once(&f.outer).chain(&f.holes) {
                let wire = model.loops.get(loop_id)?;
                for coedge in &wire.coedges {
                    let edge = model.edges.get(coedge.edge)?;
                    if !edge.degenerate {
                        edges.insert(coedge.edge);
                        vertices.insert(edge.vertices[0]);
                        vertices.insert(edge.vertices[1]);
                    }
                }
            }
        }
        let v = vertices.len() as i64;
        let e = edges.len() as i64;
        let f = shell.faces.len() as i64;
        Some(v - e + f - hole_loops)
    }

    /// Topological genus $g = (2 - \chi) / 2$ of a closed shell `shell_id`
    /// (returns `None` if the shell is open or `shell_id` is out of bounds).
    pub fn shell_genus<C, S, P, V>(model: &Model<C, S, P, V>, shell_id: usize) -> Option<i64> {
        let shell = model.shells.get(shell_id)?;
        if !shell.closed {
            return None;
        }
        let chi = shell_euler_characteristic(model, shell_id)?;
        Some((2 - chi) / 2)
    }

    /// Sum of topological genera across all closed shells in `model`, or `(2S - \chi) / 2`
    /// where $S$ is the number of closed shells.
    pub fn genus<C, S, P, V>(model: &Model<C, S, P, V>) -> i64 {
        let mut total = 0i64;
        for s in 0..model.shells.len() {
            if let Some(g) = shell_genus(model, s) {
                total += g.max(0);
            }
        }
        total
    }

    /// Sorted deduplicated indices of non-degenerate edges incident to `vertex_id`.
    pub fn vertex_edges<C, S, P, V>(model: &Model<C, S, P, V>, vertex_id: usize) -> Vec<usize> {
        let mut out = Vec::new();
        for (ei, edge) in model.edges.iter().enumerate() {
            if !edge.degenerate && (edge.vertices[0] == vertex_id || edge.vertices[1] == vertex_id)
            {
                out.push(ei);
            }
        }
        out
    }

    /// Sorted deduplicated indices of edges bounding `face_id`.
    pub fn face_edges<C, S, P, V>(model: &Model<C, S, P, V>, face_id: usize) -> Vec<usize> {
        let Some(face) = model.faces.get(face_id) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for &loop_id in std::iter::once(&face.outer).chain(&face.holes) {
            if let Some(wire) = model.loops.get(loop_id) {
                for coedge in &wire.coedges {
                    out.push(coedge.edge);
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Sorted deduplicated indices of faces sharing at least one non-degenerate edge with `face_id`.
    pub fn adjacent_faces<C, S, P, V>(model: &Model<C, S, P, V>, face_id: usize) -> Vec<usize> {
        let edges: FxHashSet<usize> = face_edges(model, face_id)
            .into_iter()
            .filter(|&e| model.edges.get(e).is_some_and(|ed| !ed.degenerate))
            .collect();
        if edges.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::new();
        for (fi, f) in model.faces.iter().enumerate() {
            if fi == face_id {
                continue;
            }
            let shares = std::iter::once(&f.outer).chain(&f.holes).any(|&l| {
                model
                    .loops
                    .get(l)
                    .is_some_and(|w| w.coedges.iter().any(|c| edges.contains(&c.edge)))
            });
            if shares {
                out.push(fi);
            }
        }
        out
    }

    fn edge_to_faces_table<C, S, P, V>(model: &Model<C, S, P, V>) -> Vec<Vec<usize>> {
        let mut table = vec![Vec::new(); model.edges.len()];
        for (fi, f) in model.faces.iter().enumerate() {
            for &loop_id in std::iter::once(&f.outer).chain(&f.holes) {
                let Some(wire) = model.loops.get(loop_id) else {
                    continue;
                };
                for coedge in &wire.coedges {
                    if let Some(edge) = model.edges.get(coedge.edge)
                        && !edge.degenerate
                    {
                        table[coedge.edge].push(fi);
                    }
                }
            }
        }
        table
    }
    fn edge_coedge_counts<C, S, P, V>(model: &Model<C, S, P, V>) -> Vec<usize> {
        let mut counts = vec![0usize; model.edges.len()];
        for f in &model.faces {
            for &loop_id in std::iter::once(&f.outer).chain(&f.holes) {
                let Some(wire) = model.loops.get(loop_id) else {
                    continue;
                };
                for coedge in &wire.coedges {
                    if let Some(edge) = model.edges.get(coedge.edge)
                        && !edge.degenerate
                    {
                        counts[coedge.edge] += 1;
                    }
                }
            }
        }
        counts
    }
    /// Sorted indices of non-degenerate boundary edges (used by exactly 1 face coedge).
    pub fn boundary_edges<C, S, P, V>(model: &Model<C, S, P, V>) -> Vec<EdgeId> {
        edge_coedge_counts(model)
            .into_iter()
            .enumerate()
            .filter_map(|(ei, count)| (count == 1).then_some(ei))
            .collect()
    }
    /// Sorted indices of non-degenerate non-manifold edges (used by 3 or more face coedges).
    pub fn non_manifold_edges<C, S, P, V>(model: &Model<C, S, P, V>) -> Vec<EdgeId> {
        edge_coedge_counts(model)
            .into_iter()
            .enumerate()
            .filter_map(|(ei, count)| (count > 2).then_some(ei))
            .collect()
    }
    /// Number of connected components of faces across the model (where two faces are adjacent
    /// if they share at least one non-degenerate edge).
    pub fn connected_face_components<C, S, P, V>(model: &Model<C, S, P, V>) -> usize {
        let n_faces = model.faces.len();
        if n_faces == 0 {
            return 0;
        }
        let edge_to_faces = edge_to_faces_table(model);
        let mut visited = vec![false; n_faces];
        let mut stack = Vec::with_capacity(n_faces);
        let mut components = 0usize;
        for start in 0..n_faces {
            if visited[start] {
                continue;
            }
            components += 1;
            visited[start] = true;
            stack.push(start);
            while let Some(fi) = stack.pop() {
                let f = &model.faces[fi];
                for &loop_id in std::iter::once(&f.outer).chain(&f.holes) {
                    let Some(wire) = model.loops.get(loop_id) else {
                        continue;
                    };
                    for coedge in &wire.coedges {
                        if let Some(neighbors) = edge_to_faces.get(coedge.edge) {
                            for &nb in neighbors {
                                if !visited[nb] {
                                    visited[nb] = true;
                                    stack.push(nb);
                                }
                            }
                        }
                    }
                }
            }
        }
        components
    }

    /// Sorted indices of isolated vertices (vertices not referenced by any edge in `model`).
    pub fn isolated_vertices<C, S, P, V>(model: &Model<C, S, P, V>) -> Vec<VertexId> {
        let mut used = vec![false; model.vertices.len()];
        for edge in &model.edges {
            for &v in &edge.vertices {
                if let Some(slot) = used.get_mut(v) {
                    *slot = true;
                }
            }
        }
        used.into_iter()
            .enumerate()
            .filter_map(|(vi, is_used)| (!is_used).then_some(vi))
            .collect()
    }

    /// Total loop count (`1 + holes.len()`) for each face `(FaceId, loop_count)` in `model`.
    pub fn face_loop_counts<C, S, P, V>(model: &Model<C, S, P, V>) -> Vec<(FaceId, usize)> {
        model
            .faces
            .iter()
            .enumerate()
            .map(|(fi, face)| (fi, 1 + face.holes.len()))
            .collect()
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
        self.try_edit_scoped(|model| edit(model).map(|_| EditScope::full()))
    }
    /// Like [`Self::try_edit`], but the edit reports an [`EditScope`] and only
    /// the provably affected checks rerun; cheap global invariants always run
    /// in full. For a valid pre-edit model and an honest scope the outcome and
    /// the first error (code and message) match full validation exactly.
    /// The model clone remains: the snapshot is immutable, so the candidate is
    /// always edited on an isolated copy; only validation is incremental.
    pub fn try_edit_scoped(
        &self,
        edit: impl FnOnce(&mut Model<C, S, P, V>) -> Result<EditScope>,
    ) -> Result<Self> {
        let mut candidate = (*self.model).clone();
        let scope = edit(&mut candidate)?;
        let plan = ValidationPlan::for_edit(&self.model, &scope, &candidate);
        candidate.validate_topology_scoped(|v| (self.validate_vertex)(v), &plan)?;
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
#[path = "tests/tests_integration.rs"]
mod tests;


#[path="lib/methods_full.rs"]
mod methods_full;
#[path="lib/methods_euler_characteristic.rs"]
mod methods_euler_characteristic;
#[path="lib/methods_new.rs"]
mod methods_new;
