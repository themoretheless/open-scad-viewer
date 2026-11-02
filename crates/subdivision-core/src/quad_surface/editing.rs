//! Topological control-cage reconciliation and strip deletion.
//! These are shape-changing cage edits, not approximation/preservation receipts.
use super::QuadSurface;
use crate::{Cage, Result, edge, error};
use std::collections::{BTreeMap, BTreeSet};
mod deletion;
pub use deletion::{DeletionOptions,VertexDeletion,DeletionVerification};

pub struct Reconciliation {
    surface: QuadSurface,
    vertex_maps: [Vec<usize>; 2],
    maximum_control_displacement: f64,
}
impl Reconciliation {
    pub fn surface(&self) -> &QuadSurface {
        &self.surface
    }
    pub fn into_surface(self) -> QuadSurface {
        self.surface
    }
    pub fn vertex_maps(&self) -> &[Vec<usize>; 2] {
        &self.vertex_maps
    }
    pub fn maximum_control_displacement(&self) -> f64 {
        self.maximum_control_displacement
    }
}
fn loop_is_boundary(cage: &Cage, vertices: &[usize]) -> Result<()> {
    if !(3..=4096).contains(&vertices.len())
        || vertices.iter().any(|&v| v >= cage.vertices.len())
        || vertices.iter().copied().collect::<BTreeSet<_>>().len() != vertices.len()
    {
        return Err(error(
            "Need a bounded simple original boundary vertex cycle",
        ));
    }
    let edges = cage.build_edges();
    for i in 0..vertices.len() {
        if !edges
            .get(&edge(vertices[i], vertices[(i + 1) % vertices.len()]))
            .is_some_and(|u| u.len() == 1)
        {
            return Err(error(
                "Every reconciliation cycle edge must be an original boundary edge",
            ));
        }
    }
    Ok(())
}
/// Join complete original boundary cycles by explicit vertex correspondence.
/// Matching control vertices are averaged within the caller's displacement
/// allowance; no nearest-point pairing or hidden orientation reversal occurs.
/// Full manifold/fan validation gates the resulting retained limit surface.
/// Reconciliation can change geometry near the seam; no old-limit G2 or
/// approximation certificate is implied.
pub fn reconcile(
    a: &QuadSurface,
    a_loop: &[usize],
    b: &QuadSurface,
    b_loop: &[usize],
    maximum_pair_distance: f64,
) -> Result<Reconciliation> {
    if a_loop.len() != b_loop.len()
        || !maximum_pair_distance.is_finite()
        || maximum_pair_distance < 0.
    {
        return Err(error(
            "Matching cycles and finite nonnegative pair allowance required",
        ));
    }
    loop_is_boundary(&a.cage, a_loop)?;
    loop_is_boundary(&b.cage, b_loop)?;
    let mut vertices = a.cage.vertices.clone();
    let mut a_map = (0..vertices.len()).collect::<Vec<_>>();
    let mut b_map = vec![usize::MAX; b.cage.vertices.len()];
    let mut maximum = 0f64;
    for (&i, &j) in a_loop.iter().zip(b_loop) {
        let pa = a.cage.vertices[i];
        let pb = b.cage.vertices[j];
        let distance = (0..3).fold(0f64, |d, k| d.hypot(pa[k] - pb[k]));
        if !distance.is_finite() || distance > maximum_pair_distance {
            return Err(error(
                "Control boundary pair exceeds reconciliation allowance",
            ));
        }
        let p = std::array::from_fn(|k| pa[k] / 2. + pb[k] / 2.);
        vertices[i] = p;
        b_map[j] = i;
        for q in [pa, pb] {
            maximum = maximum.max((0..3).fold(0f64, |d, k| d.hypot(p[k] - q[k])));
        }
    }
    for (j, p) in b.cage.vertices.iter().enumerate() {
        if b_map[j] == usize::MAX {
            b_map[j] = vertices.len();
            vertices.push(*p);
        }
    }
    let mut faces = a.cage.faces.clone();
    faces.extend(
        b.cage
            .faces
            .iter()
            .map(|f| f.iter().map(|&v| b_map[v]).collect()),
    );
    let surface = QuadSurface::new(Cage { vertices, faces })?;
    // Retain explicit original mappings even when the first cage's IDs happen
    // to remain numerically unchanged; callers must not infer this for edits.
    a_map.shrink_to_fit();
    Ok(Reconciliation {
        surface,
        vertex_maps: [a_map, b_map],
        maximum_control_displacement: maximum,
    })
}

#[derive(Clone, Copy, Debug)]
pub struct StripFace {
    pub face: usize,
    /// Collapse opposite original face edges (0,2) or (1,3).
    pub across_edge: usize,
}
pub struct StripDeletion {
    surface: QuadSurface,
    vertex_map: Vec<Option<usize>>,
    face_map: Vec<Option<usize>>,
    maximum_control_displacement: f64,
}
impl StripDeletion {
    pub fn surface(&self) -> &QuadSurface {
        &self.surface
    }
    pub fn into_surface(self) -> QuadSurface {
        self.surface
    }
    pub fn vertex_map(&self) -> &[Option<usize>] {
        &self.vertex_map
    }
    pub fn face_map(&self) -> &[Option<usize>] {
        &self.face_map
    }
    pub fn maximum_control_displacement(&self) -> f64 {
        self.maximum_control_displacement
    }
}
fn root(parents: &mut [usize], mut v: usize) -> usize {
    while parents[v] != v {
        parents[v] = parents[parents[v]];
        v = parents[v];
    }
    v
}
impl QuadSurface {
    /// Delete one nonbranching quad strip/ring and merge its paired control
    /// vertices. Every remaining face must still have four distinct vertices;
    /// the entire resulting cage must remain oriented and manifold.
    /// Positions are component averages. This changes shape and makes no bound
    /// on displacement of the old LIMIT surface, only on control displacement.
    pub fn delete_strip(&self, strip: &[StripFace]) -> Result<StripDeletion> {
        if strip.is_empty()
            || strip.len() > 4096
            || strip.len() >= self.cage.faces.len()
            || strip
                .iter()
                .any(|s| s.face >= self.cage.faces.len() || s.across_edge > 1)
            || strip.iter().map(|s| s.face).collect::<BTreeSet<_>>().len() != strip.len()
        {
            return Err(error(
                "Need a nonempty distinct bounded strip and retained faces",
            ));
        }
        let mut collapsed = BTreeMap::<(usize, usize), Vec<usize>>::new();
        let mut parents = (0..self.cage.vertices.len()).collect::<Vec<_>>();
        for s in strip {
            let f = &self.cage.faces[s.face];
            for i in [s.across_edge, s.across_edge + 2] {
                let (a, b) = (f[i], f[(i + 1) % 4]);
                collapsed.entry(edge(a, b)).or_default().push(s.face);
                let ra = root(&mut parents, a);
                let rb = root(&mut parents, b);
                parents[rb] = ra;
            }
        }
        let mut adjacency = BTreeMap::<usize, BTreeSet<usize>>::new();
        for owners in collapsed.values() {
            if owners.len() > 2 {
                return Err(error("Strip branches at an original collapse edge"));
            }
            if let [a, b] = owners.as_slice() {
                adjacency.entry(*a).or_default().insert(*b);
                adjacency.entry(*b).or_default().insert(*a);
            }
        }
        if adjacency.values().any(|v| v.len() > 2) {
            return Err(error("Strip must be nonbranching"));
        }
        let mut connected = BTreeSet::from([strip[0].face]);
        loop {
            let before = connected.len();
            for f in connected.iter().copied().collect::<Vec<_>>() {
                if let Some(next) = adjacency.get(&f) {
                    connected.extend(next);
                }
            }
            if before == connected.len() {
                break;
            }
        }
        if connected.len() != strip.len() {
            return Err(error(
                "Strip collapse directions do not form one connected strip",
            ));
        }
        let mut groups = BTreeMap::<usize, Vec<usize>>::new();
        for v in 0..parents.len() {
            let r = root(&mut parents, v);
            groups.entry(r).or_default().push(v);
        }
        if groups.values().any(|g| g.len() > 2) {
            return Err(error("Strip folds back and merges more than a vertex pair"));
        }
        let removed = strip.iter().map(|s| s.face).collect::<BTreeSet<_>>();
        let mut used = BTreeSet::new();
        let mut original_faces = Vec::new();
        let mut face_map = vec![None; self.cage.faces.len()];
        for (i, f) in self.cage.faces.iter().enumerate() {
            if removed.contains(&i) {
                continue;
            }
            let mapped = f.iter().map(|&v| root(&mut parents, v)).collect::<Vec<_>>();
            if mapped.iter().copied().collect::<BTreeSet<_>>().len() != 4 {
                return Err(error("Strip deletion would collapse a retained quad"));
            }
            used.extend(mapped.iter().copied());
            face_map[i] = Some(original_faces.len());
            original_faces.push(mapped);
        }
        let mut vertices = Vec::new();
        let mut new_ids = BTreeMap::new();
        let mut vertex_map = vec![None; parents.len()];
        let mut maximum = 0f64;
        for r in used {
            let group = &groups[&r];
            let p = std::array::from_fn(|k| {
                group
                    .iter()
                    .map(|&v| self.cage.vertices[v][k] / group.len() as f64)
                    .sum()
            });
            let id = vertices.len();
            vertices.push(p);
            new_ids.insert(r, id);
            for &v in group {
                vertex_map[v] = Some(id);
                maximum =
                    maximum.max((0..3).fold(0f64, |d, k| d.hypot(p[k] - self.cage.vertices[v][k])));
            }
        }
        let faces = original_faces
            .into_iter()
            .map(|f| f.into_iter().map(|v| new_ids[&v]).collect())
            .collect();
        let surface = QuadSurface::new(Cage { vertices, faces })?;
        Ok(StripDeletion {
            surface,
            vertex_map,
            face_map,
            maximum_control_displacement: maximum,
        })
    }
}
