//! Paired chord subdivision across two single-face `RootBrep` supports.
use super::*;
#[derive(Clone, Copy, Debug, Default)]
pub struct RootPairedChordLimits {
    pub holes: RootHoleAssignmentLimits,
    pub boundary: RootChordBoundaryLimits,
}
#[derive(Clone, Debug)]
pub struct QualifiedRootPairedChordSplit {
    faces: [Arc<RootBrep>; 2],
    chords: [ss::OriginalSsRootEdge; 2],
    supports: [usize; 2],
    splits: [RootFaceChordSplit; 2],
    holes: [QualifiedRootHoleAssignment; 2],
    boundaries: [QualifiedRootChordBoundary; 2],
    limits: RootPairedChordLimits,
    child_reversed: bool,
}
impl QualifiedRootPairedChordSplit {
    /// Qualify a paired chord split across two single-face `RootBrep` models
    /// that share the same boundary vertex selectors on their outer contours
    /// and are cut by a common third surface along `chords` (for example an
    /// `[A, C]` chord on `faces[0]` and a `[B, C]` chord on `faces[1]` sharing
    /// triple-intersection endpoint root selectors).
    pub fn qualify(
        faces: [Arc<RootBrep>; 2],
        chords: [ss::OriginalSsRootEdge; 2],
        supports: [usize; 2],
        limits: RootPairedChordLimits,
    ) -> Result<Self> {
        if supports.iter().any(|&s| s >= 2)
            || !chords[0]
                .start()
                .point()
                .same_selector(chords[1].start().point())
            || !chords[0]
                .end()
                .point()
                .same_selector(chords[1].end().point())
        {
            return Err(unproved(
                "Paired chords must share exact endpoint root selectors and valid supports",
            ));
        }
        let mut splits = Vec::new();
        let mut holes = Vec::new();
        let mut boundaries = Vec::new();
        for side in 0..2 {
            let t = faces[side].topology();
            if t.faces.len() != 1 {
                return Err(unproved(
                    "Paired chord split requires single-face source models per support",
                ));
            }
            let candidate = RootFaceChordSplit::split(
                faces[side].clone(),
                0,
                chords[side].clone(),
                supports[side],
                vec![false; t.faces[0].holes.len()],
            )?;
            let routed = QualifiedRootHoleAssignment::qualify(candidate, limits.holes)?;
            let split = routed.output().clone();
            let boundary = QualifiedRootChordBoundary::qualify(split.clone(), limits.boundary)?;
            QualifiedRootUvClosure::qualify(
                split.output().clone(),
                split
                    .output()
                    .topology()
                    .loops
                    .iter()
                    .map(|l| l.coedges.len())
                    .sum(),
            )?;
            splits.push(split);
            holes.push(routed);
            boundaries.push(boundary);
        }
        let splits: [RootFaceChordSplit; 2] = splits.try_into().unwrap();
        let holes: [QualifiedRootHoleAssignment; 2] = holes.try_into().unwrap();
        let boundaries: [QualifiedRootChordBoundary; 2] = boundaries.try_into().unwrap();
        let path_selectors = |split: &RootFaceChordSplit, child: usize| -> Vec<Arc<ss::OriginalSsRootPoint>> {
            let t = split.output().topology();
            t.loops[t.faces[child].outer]
                .coedges
                .iter()
                .filter(|c| c.edge != split.inserted_edge())
                .map(|c| {
                    let e = &t.edges[c.edge].curve;
                    if c.reversed {
                        e.end().point().clone()
                    } else {
                        e.start().point().clone()
                    }
                })
                .collect()
        };
        let same_selectors = |a: &[Arc<ss::OriginalSsRootPoint>], b: &[Arc<ss::OriginalSsRootPoint>]| {
            a.len() == b.len() && a.iter().all(|p| b.iter().any(|q| p.same_selector(q)))
        };
        let s0 = path_selectors(&splits[0], 0);
        let s1_first = path_selectors(&splits[1], 0);
        let s1_second = path_selectors(&splits[1], 1);
        let child_reversed = if same_selectors(&s0, &s1_first) && !same_selectors(&s0, &s1_second) {
            false
        } else if same_selectors(&s0, &s1_second) && !same_selectors(&s0, &s1_first) {
            true
        } else {
            return Err(unproved(
                "Paired chord children do not partition outer vertices into matching pairs",
            ));
        };
        Ok(Self {
            faces,
            chords,
            supports,
            splits,
            holes,
            boundaries,
            limits,
            child_reversed,
        })
    }
    pub fn faces(&self) -> &[Arc<RootBrep>; 2] {
        &self.faces
    }
    pub fn splits(&self) -> &[RootFaceChordSplit; 2] {
        &self.splits
    }
    pub fn hole_assignments(&self) -> &[QualifiedRootHoleAssignment; 2] {
        &self.holes
    }
    pub fn boundary_censuses(&self) -> &[QualifiedRootChordBoundary; 2] {
        &self.boundaries
    }
    /// Return the matched child face index `[child_on_0, child_on_1]` in
    /// `[splits()[0].output(), splits()[1].output()]` for `side` (`false` or `true`).
    pub fn paired_children(&self, side: bool) -> [usize; 2] {
        [
            usize::from(side),
            usize::from(side ^ self.child_reversed),
        ]
    }
    /// Retain one matched child pair (`false` or `true`) across both supports,
    /// returning `[retained_support_0, retained_support_1]` with verified UV closure.
    pub fn retain_pair(&self, side: bool) -> Result<[RootFaceRetention; 2]> {
        let children = self.paired_children(side);
        let a = RootFaceRetention::retain(self.splits[0].output().clone(), vec![children[0]])?;
        let b = RootFaceRetention::retain(self.splits[1].output().clone(), vec![children[1]])?;
        if a.output().topology().faces[0].holes.len() != b.output().topology().faces[0].holes.len()
        {
            return Err(unproved(
                "Retained paired chord children have mismatched hole counts across supports",
            ));
        }
        for r in [&a, &b] {
            QualifiedRootUvClosure::qualify(
                r.output().clone(),
                r.output()
                    .topology()
                    .loops
                    .iter()
                    .map(|l| l.coedges.len())
                    .sum(),
            )?;
        }
        Ok([a, b])
    }
    pub fn reverify(&self) -> Result<()> {
        for f in &self.faces {
            f.reverify()?;
        }
        for c in &self.chords {
            c.reverify()?;
        }
        for s in &self.splits {
            s.reverify()?;
        }
        for h in &self.holes {
            h.reverify()?;
        }
        for b in &self.boundaries {
            b.reverify()?;
        }
        let fresh = Self::qualify(
            self.faces.clone(),
            self.chords.clone(),
            self.supports,
            self.limits,
        )?;
        if fresh.child_reversed != self.child_reversed {
            return Err(unproved("Paired root chord split child pairing changed"));
        }
        for side in [false, true] {
            for r in self.retain_pair(side)? {
                r.reverify()?;
            }
        }
        Ok(())
    }
}
