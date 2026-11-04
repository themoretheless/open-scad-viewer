//! Lossless indexed topology transfer. These maps bind source storage to child
//! storage; they do not confer geometry, containment or material authority.
use super::*;
use std::ops::Range;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MixedTopologyMap {
    source: StorageSnapshot,
    target: StorageSnapshot,
    vertices: Range<usize>,
    edges: Range<usize>,
    loops: Range<usize>,
    faces: Range<usize>,
    shells: Range<usize>,
    bodies: Vec<usize>,
    coedge_counts: Vec<usize>,
    face_use_counts: Vec<usize>,
    shell_reversed: Vec<bool>,
}
fn index(range: &Range<usize>, source: usize) -> Option<usize> {
    (source < range.len()).then(|| range.start + source)
}
impl MixedTopologyMap {
    pub fn source_matches(&self, model: &MixedBrep) -> bool {
        self.source == model.snapshot
    }
    pub fn target_matches(&self, model: &MixedBrep) -> bool {
        self.target == model.snapshot
    }
    /// Compare the index/orientation policy only. This does not establish
    /// source or target identity; use the snapshot matchers for that purpose.
    pub fn same_mapping(&self, other: &Self) -> bool {
        self.vertices == other.vertices
            && self.edges == other.edges
            && self.loops == other.loops
            && self.faces == other.faces
            && self.shells == other.shells
            && self.bodies == other.bodies
            && self.coedge_counts == other.coedge_counts
            && self.face_use_counts == other.face_use_counts
            && self.shell_reversed == other.shell_reversed
    }
    pub fn then(&self, next: &Self) -> Result<Self> {
        if self.target != next.source {
            return Err(unproved(
                "Topology maps refer to different intermediate snapshots",
            ));
        }
        let range = |a: &Range<usize>, b: &Range<usize>| -> Result<Range<usize>> {
            if a.is_empty() {
                return Ok(b.start..b.start);
            }
            let start = index(b, a.start)
                .ok_or_else(|| unproved("Topology map composition leaves source range"))?;
            let end = index(b, a.end - 1)
                .ok_or_else(|| unproved("Topology map composition leaves source range"))?
                + 1;
            Ok(start..end)
        };
        let bodies = self
            .bodies
            .iter()
            .map(|i| {
                next.body(*i)
                    .ok_or_else(|| unproved("Topology map composition lost a body"))
            })
            .collect::<Result<Vec<_>>>()?;
        let shell_reversed = self
            .shell_reversed
            .iter()
            .enumerate()
            .map(|(i, r)| {
                Ok(*r
                    ^ next
                        .shell_orientation_reversed(self.shells.start + i)
                        .ok_or_else(|| {
                            unproved("Topology map composition lost shell orientation")
                        })?)
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            source: self.source.clone(),
            target: next.target.clone(),
            vertices: range(&self.vertices, &next.vertices)?,
            edges: range(&self.edges, &next.edges)?,
            loops: range(&self.loops, &next.loops)?,
            faces: range(&self.faces, &next.faces)?,
            shells: range(&self.shells, &next.shells)?,
            bodies,
            coedge_counts: self.coedge_counts.clone(),
            face_use_counts: self.face_use_counts.clone(),
            shell_reversed,
        })
    }
    pub(crate) fn retarget(&mut self, model: &MixedBrep) {
        self.target = model.snapshot.clone();
    }
    pub fn vertex(&self, source: usize) -> Option<usize> {
        index(&self.vertices, source)
    }
    pub fn edge(&self, source: usize) -> Option<usize> {
        index(&self.edges, source)
    }
    pub fn loop_index(&self, source: usize) -> Option<usize> {
        index(&self.loops, source)
    }
    pub fn face(&self, source: usize) -> Option<usize> {
        index(&self.faces, source)
    }
    pub fn shell(&self, source: usize) -> Option<usize> {
        index(&self.shells, source)
    }
    pub fn body(&self, source: usize) -> Option<usize> {
        self.bodies.get(source).copied()
    }
    pub fn coedge(&self, wire: usize, source: usize) -> Option<[usize; 2]> {
        (source < *self.coedge_counts.get(wire)?).then(|| [self.loops.start + wire, source])
    }
    pub fn face_use(&self, shell: usize, source: usize) -> Option<[usize; 2]> {
        (source < *self.face_use_counts.get(shell)?).then(|| [self.shells.start + shell, source])
    }
    pub fn shell_orientation_reversed(&self, source: usize) -> Option<bool> {
        self.shell_reversed.get(source).copied()
    }
    pub(crate) fn reverse_shell(&mut self, source: usize) -> Result<()> {
        let flag = self
            .shell_reversed
            .get_mut(source)
            .ok_or_else(|| unproved("Missing transferred shell orientation"))?;
        *flag = !*flag;
        Ok(())
    }
    pub(crate) fn reparent_body(&mut self, source: usize, target: usize) -> Result<()> {
        let body = self
            .bodies
            .get_mut(source)
            .ok_or_else(|| unproved("Missing transferred source body"))?;
        *body = target;
        Ok(())
    }
}
pub struct MixedTopologyTransfer {
    model: MixedBrep,
    maps: [MixedTopologyMap; 2],
}
impl MixedTopologyTransfer {
    pub fn model(&self) -> &MixedBrep {
        &self.model
    }
    pub fn sources(&self) -> [&MixedTopologyMap; 2] {
        [&self.maps[0], &self.maps[1]]
    }
    pub fn into_parts(self) -> (MixedBrep, [MixedTopologyMap; 2]) {
        (self.model, self.maps)
    }
}
fn map(source: &MixedBrep, offsets: [usize; 6]) -> Result<MixedTopologyMap> {
    let identity = source.snapshot.clone();
    let source = source.topology();
    let counts = [
        source.vertices.len(),
        source.edges.len(),
        source.loops.len(),
        source.faces.len(),
        source.shells.len(),
        source.bodies.len(),
    ];
    let mut ranges = Vec::new();
    for (offset, count) in offsets.into_iter().zip(counts) {
        ranges.push(
            offset
                ..offset
                    .checked_add(count)
                    .ok_or_else(|| unproved("Mixed topology transfer index overflow"))?,
        );
    }
    Ok(MixedTopologyMap {
        source: identity.clone(),
        target: identity,
        vertices: ranges[0].clone(),
        edges: ranges[1].clone(),
        loops: ranges[2].clone(),
        faces: ranges[3].clone(),
        shells: ranges[4].clone(),
        bodies: ranges[5].clone().collect(),
        coedge_counts: source.loops.iter().map(|l| l.coedges.len()).collect(),
        face_use_counts: source.shells.iter().map(|s| s.faces.len()).collect(),
        shell_reversed: vec![false; source.shells.len()],
    })
}
impl MixedBrep {
    pub(crate) fn identity_map(&self) -> Result<MixedTopologyMap> {
        map(self, [0; 6])
    }
    /// Concatenate all original topology, preserving seam/trim metadata and
    /// literal root selectors. Result is storage only, not a qualified solid.
    pub fn concatenate(&self, other: &Self) -> Result<MixedTopologyTransfer> {
        let mut t = self.topology.clone();
        let mut b = other.topology.clone();
        let offsets = [
            t.vertices.len(),
            t.edges.len(),
            t.loops.len(),
            t.faces.len(),
            t.shells.len(),
            t.bodies.len(),
        ];
        let mut left = map(self, [0; 6])?;
        let mut right = map(other, offsets)?;
        for e in &mut b.edges {
            e.vertices = e.vertices.map(|v| v + offsets[0]);
        }
        for l in &mut b.loops {
            for c in &mut l.coedges {
                c.edge += offsets[1];
                if let MixedPcurve::Root(p) = &mut c.pcurve {
                    p.world_edge += offsets[1];
                }
            }
        }
        for f in &mut b.faces {
            f.outer += offsets[2];
            for l in &mut f.holes {
                *l += offsets[2];
            }
        }
        for s in &mut b.shells {
            for f in &mut s.faces {
                f.face += offsets[3];
            }
        }
        for body in &mut b.bodies {
            body.outer_shell += offsets[4];
            for s in &mut body.inner_shells {
                *s += offsets[4];
            }
        }
        t.vertices.extend(b.vertices);
        t.edges.extend(b.edges);
        t.loops.extend(b.loops);
        t.faces.extend(b.faces);
        t.shells.extend(b.shells);
        t.bodies.extend(b.bodies);
        t.tolerance_mm = t.tolerance_mm.max(b.tolerance_mm);
        let model = Self::from_topology(t)?;
        left.retarget(&model);
        right.retarget(&model);
        Ok(MixedTopologyTransfer {
            model,
            maps: [left, right],
        })
    }
}
