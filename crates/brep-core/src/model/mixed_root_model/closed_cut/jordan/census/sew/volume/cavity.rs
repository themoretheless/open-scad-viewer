//! An original admitted body minus a strictly contained admitted root solid.
//! Complete directed boundary covers precede topology or material ownership.
use super::*;
use crate::closed_solid as closed;
pub(super) mod coverage;
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
    lineage: [crate::mixed_root_model::MixedTopologyMap; 2],
    original_topology: MixedBrep,
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
    pub fn lineage_sources(&self) -> [&MixedBrep; 2] {
        [&self.original_topology, self.inner.model()]
    }
    pub fn lineage(&self) -> [&crate::mixed_root_model::MixedTopologyMap; 2] {
        [&self.lineage[0], &self.lineage[1]]
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
        for (map, source) in self.lineage.iter().zip(self.lineage_sources()) {
            if !map.source_matches(source) || !map.target_matches(self.model()) {
                return Err(unproved(
                    "Cavity lineage refers to another source or target snapshot",
                ));
            }
        }
        self.outer.reverify()?;
        self.inner.reverify()?;
        let fresh =
            self.inner
                .qualify_cavity_in(self.outer.clone(), &self.directions, self.limits)?;
        if fresh.volume != self.volume
            || fresh.covers.work != self.covers.work
            || !fresh
                .lineage
                .iter()
                .zip(&self.lineage)
                .all(|(a, b)| a.same_mapping(b))
        {
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
        let (model, lineage, original_topology) = topology::assemble(&outer, self)?;
        Ok(QualifiedRootCavitySolid {
            outer,
            inner: self.clone(),
            model,
            lineage,
            original_topology,
            volume: [volume.lo, volume.hi],
            outer_volume,
            covers,
            directions: directions.to_vec(),
            limits,
        })
    }
}

/// Result of subtracting a strictly contained, independently qualified shell.
pub struct RootShellSubtraction {
    model: MixedBrep,
    volume: [f64; 2],
    sources: [MixedBrep; 2],
    lineage: [crate::mixed_root_model::MixedTopologyMap; 2],
}

impl RootShellSubtraction {
    pub fn model(&self) -> &MixedBrep {
        &self.model
    }

    pub fn volume(&self) -> [f64; 2] {
        self.volume
    }

    /// Host followed by the outward-oriented source of the cavity.
    pub fn sources(&self) -> &[MixedBrep; 2] {
        &self.sources
    }

    pub fn lineage(&self) -> &[crate::mixed_root_model::MixedTopologyMap; 2] {
        &self.lineage
    }
}

/// A disjoint assembly with maps from the original body snapshots.
pub struct RootDisjointBodyAssembly {
    pub model: MixedBrep,
    pub volume: [f64; 2],
    pub sources: Vec<MixedBrep>,
    pub lineage: Vec<crate::mixed_root_model::MixedTopologyMap>,
}

// Whole-carrier material admission is a conservative containment proof
// for an independently qualified shell, including its trimmed boundary.
impl crate::root_model::QualifiedRootShellPositiveFlux {
    /// Subtract a strictly contained, separately qualified root shell.
    pub fn subtract_root_shell(
        &self,
        inner: &Self,
        directions: &[[f64; 3]],
        max_separation_cells: usize,
        point_limits: RootSolidPointLimits,
        absolute_error: f64,
    ) -> Result<RootShellSubtraction> {
        if !absolute_error.is_finite() || absolute_error <= 0. {
            return Err(unproved("Invalid root cavity volume accuracy"));
        }
        if self.classify_shell_relation_with_rays(
            inner, directions, max_separation_cells, point_limits,
        )? != RootRelationKind::SecondInsideFirst
        {
            return Err(unproved("Root cavity must be strictly inside its host shell"));
        }
        self.assemble_nested_subtraction(inner, absolute_error)
    }

    pub(super) fn assemble_nested_subtraction(
        &self,
        inner: &Self,
        absolute_error: f64,
    ) -> Result<RootShellSubtraction> {
        let [outer_lo, outer_hi] = self.report().signed_bounds;
        let [inner_lo, inner_hi] = inner.report().signed_bounds;
        let volume = I::new(outer_lo, outer_hi)?.sub(I::new(inner_lo, inner_hi)?)?;
        if volume.lo <= 0.
            || I::point(volume.hi).sub(I::point(volume.lo))?.hi > absolute_error
        {
            return Err(unproved("Root cavity material volume unproved"));
        }
        let outer = MixedBrep::from_root(&self.to_body()?)?;
        let inner = MixedBrep::from_root(&inner.to_body()?)?;
        let (model, lineage, outer) = topology::assemble_models(outer, &inner)?;
        Ok(RootShellSubtraction {
            model,
            volume: [volume.lo, volume.hi],
            sources: [outer, inner],
            lineage,
        })
    }

    /// Assemble independently certified shells with strictly disjoint world
    /// bounds as separate bodies, preserving the original SS geometry.
    pub fn assemble_disjoint_bodies(shells: &[Self]) -> Result<(MixedBrep, [f64; 2])> {
        let result = Self::assemble_disjoint_bodies_with_lineage(shells)?;
        Ok((result.model, result.volume))
    }

    pub fn assemble_disjoint_bodies_with_lineage(shells: &[Self]) -> Result<RootDisjointBodyAssembly> {
        if !(1..=64).contains(&shells.len()) {
            return Err(unproved("Disjoint body assembly requires one to 64 shells"));
        }
        let mut bounds = Vec::with_capacity(shells.len());
        let mut volume = I::point(0.);
        for shell in shells {
            let next = root_boundary_bounds(shell);
            if bounds.iter().any(|previous| !bounds_separated(previous, &next)) {
                return Err(unproved("Separation of root bodies is unproved"));
            }
            bounds.push(next);
            let [lo, hi] = shell.report().signed_bounds;
            volume = volume.add(I::new(lo, hi)?)?;
        }
        let sources = shells.iter()
            .map(|shell| MixedBrep::from_root(&shell.to_body()?))
            .collect::<Result<Vec<_>>>()?;
        let mut model = sources[0].clone();
        let mut lineage = vec![model.identity_map()?];
        for next in &sources[1..] {
            let (joined, [left, right]) = model.concatenate(next)?.into_parts();
            lineage = lineage.iter().map(|map| map.then(&left)).collect::<Result<_>>()?;
            lineage.push(right);
            model = joined;
        }
        Ok(RootDisjointBodyAssembly { model, volume: [volume.lo, volume.hi], sources, lineage })
    }

    pub fn subtract_from_closed_solid(
        &self,
        outer: &closed::QualifiedClosedSolid,
        directions: &[[f64; 3]],
        limits: RootCavityLimits,
    ) -> Result<(MixedBrep, [f64; 2])> {
        let result = self.subtract_from_closed_solid_with_lineage(outer, directions, limits)?;
        Ok((result.model, result.volume))
    }

    pub fn subtract_from_closed_solid_with_lineage(
        &self,
        outer: &closed::QualifiedClosedSolid,
        directions: &[[f64; 3]],
        limits: RootCavityLimits,
    ) -> Result<RootShellSubtraction> {
        if outer.model().bodies.len() != 1
            || !limits.absolute_error.is_finite()
            || limits.absolute_error <= 0.
            || !(1..=100_000).contains(&limits.coverage_cells)
            || !(1..=16).contains(&directions.len())
            || directions
                .iter()
                .any(|d| !d.iter().all(|x| x.is_finite()) || d.iter().all(|x| *x == 0.))
            || self.source().topology().faces.len() > limits.coverage_cells
        {
            return Err(unproved("Invalid closed-solid cavity host or work limits"));
        }
        // A closed shell's bounded interior lies inside its boundary box.
        // Strictly disjoint boxes rule out swallowing a pre-existing void.
        let host = outer.model();
        let existing = &host.bodies[0].inner_shells;
        if !existing.is_empty() {
            let new_bounds = root_boundary_bounds(self);
            for &shell in existing {
                let old_bounds = carrier_bounds(
                    host.shells[shell].faces.iter().map(|f| &host.faces[f.face].surface),
                );
                if !bounds_separated(&new_bounds, &old_bounds) {
                    return Err(unproved("Root cavity separation from existing void is unproved"));
                }
            }
        }
        let mut remaining = limits.coverage_cells;
        for (index, face) in self.source().topology().faces.iter().enumerate() {
            let surface = &face.surface;
            // Visit shallow cells first so one unresolved boundary point
            // cannot postpone a definite exterior cell on the same face.
            let mut pending =
                std::collections::VecDeque::from([material_domain(self, index, surface)]);
            while let Some(domain) = pending.pop_front() {
                remaining = remaining.checked_sub(1)
                    .ok_or_else(|| unproved("Root cavity material cover budget exhausted"))?;
                let material = closed::qualify_surface_material_box(
                    outer, surface, domain, directions, limits.original_material,
                );
                if material.as_ref().is_ok_and(|proof| proof.inside()) {
                    continue;
                }
                use crate::root_model::RootFaceRegion;
                match self.classify_face_box(index, domain) {
                    Ok(RootFaceRegion::OutsideOuter | RootFaceRegion::ExcludedByHoles(_)) => continue,
                    Ok(RootFaceRegion::InsideSupport) if material.is_ok() => {
                        return Err(unproved("Root cavity face is outside host material"));
                    }
                    _ => {}
                }
                let axis = usize::from(domain[1][1] - domain[1][0] > domain[0][1] - domain[0][0]);
                let [lo, hi] = domain[axis];
                let mid = lo * 0.5 + hi * 0.5;
                if !(lo < mid && mid < hi) {
                    return Err(unproved("Root cavity material cover could not be refined"));
                }
                let mut left = domain;
                let mut right = domain;
                left[axis][1] = mid;
                right[axis][0] = mid;
                pending.extend([right, left]);
            }
        }
        let outer_volume = outer.volume_bounds(limits.original_volume)?
            .positive_volume
            .ok_or_else(|| unproved("Root cavity host volume unproved"))?;
        let [lo, hi] = self.report().signed_bounds;
        let volume = I::new(outer_volume[0], outer_volume[1])?.sub(I::new(lo, hi)?)?;
        if volume.lo <= 0.
            || I::point(volume.hi).sub(I::point(volume.lo))?.hi > limits.absolute_error
        {
            return Err(unproved("Root cavity material volume unproved"));
        }
        let inner = MixedBrep::from_root(&self.to_body()?)?;
        let (model, lineage, host) =
            topology::assemble_models(MixedBrep::from_closed_solid(outer)?, &inner)?;
        Ok(RootShellSubtraction {
            model,
            volume: [volume.lo, volume.hi],
            sources: [host, inner],
            lineage,
        })
    }
}

fn material_domain(
    shell: &crate::root_model::QualifiedRootShellPositiveFlux,
    index: usize,
    surface: &crate::Surface,
) -> [[f64; 2]; 2] {
    let natural = [
        [surface.knots_u[surface.degree_u], surface.knots_u[surface.control_points.len()]],
        [surface.knots_v[surface.degree_v], surface.knots_v[surface.control_points[0].len()]],
    ];
    let trimmed = shell.face_bounds(index);
    let periodic = [surface.periodic_u, surface.periodic_v];
    std::array::from_fn(|axis| {
        if periodic[axis] {
            natural[axis]
        } else {
            [trimmed[axis][0].max(natural[axis][0]), trimmed[axis][1].min(natural[axis][1])]
        }
    })
}

fn root_boundary_bounds(
    shell: &crate::root_model::QualifiedRootShellPositiveFlux,
) -> [[f64; 2]; 3] {
    use nurbs_core::interval_eval::evaluate_surface_interval;
    let mut bounds = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for (index, face) in shell.source().topology().faces.iter().enumerate() {
        let [[u0, u1], [v0, v1]] = material_domain(shell, index, &face.surface);
        let enclosure = I::new(u0, u1).and_then(|u| {
            evaluate_surface_interval(&face.surface, u, I::new(v0, v1)?)
        });
        let face_bounds = match enclosure {
            Ok(values) if values.len() == 3 => std::array::from_fn(|axis| [values[axis].lo, values[axis].hi]),
            _ => carrier_bounds(std::iter::once(&face.surface)),
        };
        for (bound, [lo, hi]) in bounds.iter_mut().zip(face_bounds) {
            bound[0] = bound[0].min(lo);
            bound[1] = bound[1].max(hi);
        }
    }
    bounds
}
