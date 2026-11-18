//! Pairwise root-solid topology from two complete original boundary covers.
use super::*;
mod cover;
#[derive(Clone, Copy)]
pub struct RootRelationLimits {
    pub coverage_cells: usize,
    pub domain_queries: usize,
    pub material: RootSurfaceMaterialLimits,
}
impl Default for RootRelationLimits {
    fn default() -> Self {
        Self {
            coverage_cells: 4096,
            domain_queries: 4096,
            material: Default::default(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RootRelationKind {
    Disjoint,
    FirstInsideSecond,
    SecondInsideFirst,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RootRelationWork {
    pub cells: usize,
    pub accepted: usize,
    pub domain_queries: usize,
    pub material_attempts: usize,
    pub band_queries: usize,
    pub winding_queries: usize,
}
pub struct QualifiedRootRelation {
    first: Arc<QualifiedPairedRootSolid>,
    second: Arc<QualifiedPairedRootSolid>,
    kind: RootRelationKind,
    covers: [cover::Cover; 2],
    directions: Vec<[f64; 3]>,
    limits: RootRelationLimits,
}
impl QualifiedRootRelation {
    pub fn sources(&self) -> [&Arc<QualifiedPairedRootSolid>; 2] {
        [&self.first, &self.second]
    }
    pub fn kind(&self) -> RootRelationKind {
        self.kind
    }
    pub fn work(&self) -> [&RootRelationWork; 2] {
        [&self.covers[0].work, &self.covers[1].work]
    }
    pub fn reverify(&self) -> Result<()> {
        self.first.reverify()?;
        self.second.reverify()?;
        self.replay().map(|_| ())
    }
    pub(super) fn replay(&self) -> Result<Self> {
        let fresh =
            self.first
                .qualify_relation(self.second.clone(), &self.directions, self.limits)?;
        if fresh.kind != self.kind || fresh.work() != self.work() {
            return Err(unproved("Root relation complete boundary proof changed"));
        }
        Ok(fresh)
    }
}
impl QualifiedPairedRootSolid {
    pub fn qualify_relation(
        self: &Arc<Self>,
        other: Arc<Self>,
        directions: &[[f64; 3]],
        limits: RootRelationLimits,
    ) -> Result<QualifiedRootRelation> {
        if Arc::ptr_eq(self, &other)
            || !(1..=100000).contains(&limits.coverage_cells)
            || !(1..=65536).contains(&limits.domain_queries)
            || !(1..=16).contains(&directions.len())
            || directions
                .iter()
                .any(|d| !d.iter().all(|x| x.is_finite()) || d.iter().all(|x| *x == 0.))
        {
            return Err(unproved(
                "Invalid root relation owners, directions or work allowance",
            ));
        }
        let forward = cover::qualify(self, &other, directions, limits)?;
        let reverse = cover::qualify(&other, self, directions, limits)?;
        let kind = match (forward.inside, reverse.inside) {
            (false, false) => RootRelationKind::Disjoint,
            (true, false) => RootRelationKind::FirstInsideSecond,
            (false, true) => RootRelationKind::SecondInsideFirst,
            (true, true) => {
                return Err(unproved(
                    "Mutual bounded root containment contradicts complete separation",
                ))
            }
        };
        Ok(QualifiedRootRelation {
            first: self.clone(),
            second: other,
            kind,
            covers: [forward, reverse],
            directions: directions.to_vec(),
            limits,
        })
    }
}
