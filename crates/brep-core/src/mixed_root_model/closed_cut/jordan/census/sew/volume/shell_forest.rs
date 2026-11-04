//! Even/odd material from an exhaustive set of immutable pairwise root proofs.
use super::*;
mod nesting;
mod topology;
pub struct QualifiedRootShellForest {
    roots: Vec<Arc<QualifiedPairedRootSolid>>,
    relations: Vec<Arc<QualifiedRootRelation>>,
    parents: Vec<Option<usize>>,
    depths: Vec<usize>,
    model: MixedBrep,
    lineage: Vec<crate::mixed_root_model::MixedTopologyMap>,
    volume: [f64; 2],
    absolute_error: f64,
}
impl QualifiedRootShellForest {
    pub fn qualify(
        roots: Vec<Arc<QualifiedPairedRootSolid>>,
        relations: Vec<Arc<QualifiedRootRelation>>,
        absolute_error: f64,
    ) -> Result<Self> {
        if !(2..=64).contains(&roots.len())
            || !absolute_error.is_finite()
            || absolute_error <= 0.
            || roots.iter().any(|r| {
                r.model().topology().bodies.len() != 1 || r.model().topology().shells.len() != 1
            })
        {
            return Err(unproved(
                "Invalid root shell forest source owners or volume accuracy",
            ));
        }
        for i in 0..roots.len() {
            for j in 0..i {
                if Arc::ptr_eq(&roots[i], &roots[j]) {
                    return Err(unproved("Root shell forest repeats the same source owner"));
                }
            }
        }
        let (parents, depths) = nesting::qualify(&roots, &relations)?;
        let mut total = I::point(0.);
        for (root, depth) in roots.iter().zip(&depths) {
            let v = root.volume();
            let v = I::new(v[0], v[1])?;
            total = if depth % 2 == 0 {
                total.add(v)?
            } else {
                total.sub(v)?
            };
        }
        if total.lo <= 0. || I::point(total.hi).sub(I::point(total.lo))?.hi > absolute_error {
            return Err(unproved(
                "Root shell forest positive material volume or accuracy unproved",
            ));
        }
        let (model, lineage) = topology::assemble(&roots, &parents, &depths)?;
        Ok(Self {
            roots,
            relations,
            parents,
            depths,
            model,
            lineage,
            volume: [total.lo, total.hi],
            absolute_error,
        })
    }
    pub fn model(&self) -> &MixedBrep {
        &self.model
    }
    pub fn roots(&self) -> &[Arc<QualifiedPairedRootSolid>] {
        &self.roots
    }
    pub fn parents(&self) -> &[Option<usize>] {
        &self.parents
    }
    pub fn depths(&self) -> &[usize] {
        &self.depths
    }
    pub fn lineage(&self) -> &[crate::mixed_root_model::MixedTopologyMap] {
        &self.lineage
    }
    pub fn volume(&self) -> [f64; 2] {
        self.volume
    }
    pub fn material_admitted(&self) -> bool {
        true
    }
    /// Each source has its own explicit point work allowance.
    pub fn classify_point(
        &self,
        point: [f64; 3],
        directions: &[[f64; 3]],
        limits: RootSolidPointLimits,
    ) -> Result<Option<bool>> {
        let mut parity = false;
        for source in &self.roots {
            let Some(inside) = source.classify_point(point, directions, limits)?.parity else {
                return Ok(None);
            };
            parity ^= inside;
        }
        Ok(Some(parity))
    }
    pub fn reverify(&self) -> Result<()> {
        for (root, map) in self.roots.iter().zip(&self.lineage) {
            if !map.source_matches(root.model()) || !map.target_matches(self.model()) {
                return Err(unproved("Root forest lineage refers to another snapshot"));
            }
            root.reverify()?;
        }
        let relations = self
            .relations
            .iter()
            .map(|r| r.replay().map(Arc::new))
            .collect::<Result<Vec<_>>>()?;
        let fresh = Self::qualify(self.roots.clone(), relations, self.absolute_error)?;
        if fresh.parents != self.parents
            || fresh.depths != self.depths
            || fresh.volume != self.volume
            || !fresh
                .lineage
                .iter()
                .zip(&self.lineage)
                .all(|(a, b)| a.same_mapping(b))
        {
            return Err(unproved(
                "Root forest nesting, volume or lineage proof changed",
            ));
        }
        Ok(())
    }
}
