//! An original admitted body minus a strictly contained admitted root solid.
//! Complete directed boundary covers precede topology or material ownership.
use super::*;
use crate::closed_solid as closed;
mod coverage;
mod topology;
#[derive(Clone, Copy)]
pub struct RootCavityLimits {
    pub absolute_error: f64,
    pub coverage_cells: usize,
    pub domain_queries: usize,
    pub root_material: RootSurfaceMaterialLimits,
    pub original_material: closed::SurfaceMaterialBoxLimits,
    pub original_volume: closed::VolumeLimits,
}
impl Default for RootCavityLimits {
    fn default() -> Self {
        Self {
            absolute_error: 1e-3,
            coverage_cells: 4096,
            domain_queries: 4096,
            root_material: Default::default(),
            original_material: Default::default(),
            original_volume: closed::VolumeLimits {
                axis: 2,
                origin: 0.,
                absolute_error: 1e-4,
                cells: 16384,
                spans: 32768,
                domain_cells: 1000000,
                algebra: Default::default(),
            },
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct RootCavityCoverageWork {
    pub cells: [usize; 2],
    pub accepted: [usize; 2],
    pub domain_queries: [usize; 2],
}
pub struct QualifiedRootCavitySolid {
    outer: Arc<closed::QualifiedClosedSolid>,
    inner: Arc<QualifiedPairedRootSolid>,
    model: MixedBrep,
    volume: [f64; 2],
    outer_volume: closed::VolumeReport,
    covers: coverage::Covers,
    directions: Vec<[f64; 3]>,
    limits: RootCavityLimits,
}
impl QualifiedRootCavitySolid {
    pub fn model(&self) -> &MixedBrep {
        &self.model
    }
    pub fn volume(&self) -> [f64; 2] {
        self.volume
    }
    pub fn outer(&self) -> &Arc<closed::QualifiedClosedSolid> {
        &self.outer
    }
    pub fn inner(&self) -> &Arc<QualifiedPairedRootSolid> {
        &self.inner
    }
    pub fn material_admitted(&self) -> bool {
        true
    }
    pub fn coverage_work(&self) -> &RootCavityCoverageWork {
        &self.covers.work
    }
    pub fn original_volume_proof(&self) -> &closed::VolumeReport {
        &self.outer_volume
    }
    pub fn reverify(&self) -> Result<()> {
        self.outer.reverify()?;
        self.inner.reverify()?;
        let fresh =
            self.inner
                .qualify_cavity_in(self.outer.clone(), &self.directions, self.limits)?;
        if fresh.volume != self.volume || fresh.covers.work != self.covers.work {
            return Err(unproved("Root cavity cover or material volume changed"));
        }
        Ok(())
    }
    /// The explicit point work allowances apply separately to each operand.
    pub fn classify_point(
        &self,
        point: [f64; 3],
        directions: &[[f64; 3]],
        limits: RootSolidPointLimits,
    ) -> Result<Option<bool>> {
        let a = self.outer.classify_point(
            point,
            directions,
            limits.ray_cells,
            limits.domain_queries,
        )?;
        let b = self.inner.classify_point(point, directions, limits)?;
        Ok(match (a.parity, b.parity) {
            (Some(false), _) | (_, Some(true)) => Some(false),
            (Some(true), Some(false)) => Some(true),
            _ => None,
        })
    }
}
impl QualifiedPairedRootSolid {
    pub fn qualify_cavity_in(
        self: &Arc<Self>,
        outer: Arc<closed::QualifiedClosedSolid>,
        directions: &[[f64; 3]],
        limits: RootCavityLimits,
    ) -> Result<QualifiedRootCavitySolid> {
        if !limits.absolute_error.is_finite()
            || limits.absolute_error <= 0.
            || !(1..=100000).contains(&limits.coverage_cells)
            || !(1..=65536).contains(&limits.domain_queries)
            || !(1..=16).contains(&directions.len())
            || directions
                .iter()
                .any(|d| !d.iter().all(|x| x.is_finite()) || d.iter().all(|x| *x == 0.))
            || outer.model().bodies.len() != 1
            || self.model().topology().bodies.len() != 1
            || self.model().topology().shells.len() != 1
        {
            return Err(unproved(
                "Invalid root cavity work, directions or body ownership",
            ));
        }
        let covers = coverage::qualify(self, &outer, directions, limits)?;
        let outer_volume = outer.volume_bounds(limits.original_volume)?;
        let a = outer_volume
            .positive_volume
            .ok_or_else(|| unproved("Original cavity host volume unproved"))?;
        let b = self.volume();
        let volume = I::new(a[0], a[1])?.sub(I::new(b[0], b[1])?)?;
        if volume.lo <= 0.
            || I::point(volume.hi).sub(I::point(volume.lo))?.hi > limits.absolute_error
        {
            return Err(unproved("Positive cavity result volume unproved"));
        }
        let model = topology::assemble(&outer, self)?;
        Ok(QualifiedRootCavitySolid {
            outer,
            inner: self.clone(),
            model,
            volume: [volume.lo, volume.hi],
            outer_volume,
            covers,
            directions: directions.to_vec(),
            limits,
        })
    }
}
