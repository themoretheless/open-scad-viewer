//! Fresh contact-clock roots own one shared middle edge on both source faces.
use crate::{Model, TopoId, feature_ops::edge_partition as shared,
    fillet_contact::{Contact, convex_branch as branch}, region_replacement::EdgeRadius};
use nurbs_core::{Error, Result, interval_eval::{self, Interval as I}, surface::Surface};

#[derive(Clone, Copy)]
pub struct Limits {
    /// Branch cells, original root enclosures and shared subdivision use one ledger.
    pub refinement: shared::Limits,
    pub topology_visits: usize,
    pub branch_exact_work: u64,
    pub branch_uv_work: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self { refinement: Default::default(), topology_visits: 100_000,
            branch_exact_work: 1_000_000, branch_uv_work: 100_000 }
    }
}
fn refuse(s: &str) -> Error { Error::new("BREP_CONTACT_INTERVAL_UNPROVEN", s) }

/// Immutable ownership of the actual refinement and freshly reconstructed roots.
/// No material is removed; transition construction/G2/radius remain separate.
pub struct ContactInterval {
    split: shared::Split,
    skin: Surface,
    contacts: [Contact; 2],
    shell: usize,
    request: EdgeRadius,
    key: String,
    tolerance_uv: f64,
    limits: Limits,
    branch: branch::Report,
    bands: [[f64; 2]; 2],
    bounds: [[[f64; 2]; 3]; 2],
    errors: [f64; 2],
    cells: usize,
}
impl ContactInterval {
    pub fn model(&self) -> &Model { self.split.model() }
    /// Original complete contact curves, in original support-face order.
    pub fn contacts(&self) -> &[Contact; 2] { &self.contacts }
    pub fn middle_edge(&self) -> usize { self.split.edges()[1] }
    pub fn endpoints(&self) -> [usize; 2] { [self.split.vertices()[0], self.split.vertices()[1]] }
    pub fn endpoint_identities(&self) -> [TopoId; 2] {
        self.endpoints().map(|v| self.model().1.vertices[v])
    }
    pub fn root_parameters(&self) -> [[f64; 2]; 2] { self.bands }
    pub fn root_position_bounds(&self) -> [[[f64; 2]; 3]; 2] { self.bounds }
    pub fn root_error_upper(&self) -> [f64; 2] { self.errors }
    pub fn branch(&self) -> &branch::Report { &self.branch }
    pub fn partition(&self) -> &shared::Split { &self.split }
    pub fn cells(&self) -> usize { self.cells }
    /// Stored mutable reports and rounded root midpoints are never authorities.
    pub fn reverify(&self) -> Result<()> {
        let actual = between_contacts(self.split.source(), self.shell, self.request,
            &self.skin, [&self.contacts[0], &self.contacts[1]], &self.key,
            self.tolerance_uv, self.limits)?;
        if actual.model() != self.model() || actual.bands != self.bands
            || actual.bounds != self.bounds || actual.errors != self.errors
            || actual.endpoints() != self.endpoints() {
            return Err(refuse("Saved contact interval differs from fresh original reconstruction"));
        }
        Ok(())
    }
}
/// Recompute ownership from literal source/contact geometry, then refine both
/// original uses once at the two clock roots. Representatives are authoring
/// proposals only; every original-world and saved lift is requalified by Split,
/// and the actual common vertices are bounded against the entire root images.
/// Currently requires two disjoint interior roots; full-edge/end-touching
/// intervals need their own endpoint-role transaction. This is not solid trimming.
pub fn between_contacts(
    source: &Model, shell: usize, request: EdgeRadius, skin: &Surface,
    contacts: [&Contact; 2], key: &str, tolerance_uv: f64, limits: Limits,
) -> Result<ContactInterval> {
    if !(1..=100_000).contains(&limits.refinement.max_cells)
        || !tolerance_uv.is_finite() || tolerance_uv <= 0. {
        return Err(refuse("Need bounded shared work and a positive UV tolerance"));
    }
    let branch = branch::inspect(
        source,
        shell,
        request,
        skin,
        contacts,
        branch::BranchBudget {
            max_visits: limits.topology_visits,
            max_cells: limits.refinement.max_cells,
            max_exact: limits.branch_exact_work,
            max_uv_work: limits.branch_uv_work,
        },
    )?;
    if branch.decision == branch::Decision::Unresolved {
        return Err(refuse("Fresh contact branch ownership remains unresolved"));
    }
    let bands = branch.common_source_clock
        .ok_or_else(|| refuse("Fresh contact branch has no common source-clock interval"))?;
    let original = &source.edges[request.edge].curve;
    let domain = original.domain();
    if !(domain[0] < bands[0][0] && bands[0][1] < bands[1][0] && bands[1][1] < domain[1]) {
        return Err(refuse("Need two separated strictly interior contact-clock roots"));
    }
    let mut cells = branch.cells;
    let mut bounds = [[[0.; 2]; 3]; 2];
    for i in 0..2 {
        let cost = original.control_points.len() - original.degree;
        if cost > limits.refinement.max_cells.saturating_sub(cells) {
            return Err(refuse("Original root enclosure budget exhausted"));
        }
        cells += cost;
        let b = interval_eval::evaluate_interval(original, I::new(bands[i][0], bands[i][1])?)?;
        bounds[i] = std::array::from_fn(|k| [b[k].lo, b[k].hi]);
    }
    let parameters = bands.map(|band| super::root_representative(band, domain));
    if parameters.iter().zip(bands).any(|(&p, b)| p < b[0] || p > b[1])
        || !(domain[0] < parameters[0] && parameters[0] < parameters[1] && parameters[1] < domain[1]) {
        return Err(refuse("No ordered interior representatives inside original root bands"));
    }
    let remaining = shared::Limits { max_cells: limits.refinement.max_cells.saturating_sub(cells),
        ..limits.refinement };
    if remaining.max_cells == 0 { return Err(refuse("No work remains for shared refinement")); }
    let split = shared::split_with_uv_tolerance(source, request.edge, &parameters, key,
        tolerance_uv, remaining)?;
    cells += split.evidence().cells;
    if split.edges().len() != 3 || split.vertices().len() != 2
        || split.model().edges[split.edges()[1]].vertices != [split.vertices()[0], split.vertices()[1]] {
        return Err(refuse("Middle edge lost common root endpoint roles"));
    }
    let tolerance = source.1.local_tolerances_mm.get(&source.1.edges[request.edge])
        .copied().unwrap_or(source.tolerance_mm).min(source.tolerance_mm);
    let mut errors = [0.; 2];
    for i in 0..2 {
        let saved = split.model().vertices[split.vertices()[i]].point;
        let mut squared = I::point(0.);
        for k in 0..3 {
            let delta = I::point(saved[k]).sub(I::new(bounds[i][k][0], bounds[i][k][1])?)?;
            squared = squared.add(delta.mul(delta)?)?;
        }
        errors[i] = squared.hi.max(0.).sqrt().next_up();
        if !errors[i].is_finite() || errors[i] > tolerance {
            return Err(refuse("Saved shared vertex is not bounded to the entire original contact root"));
        }
    }
    Ok(ContactInterval { split, skin: skin.clone(), contacts: contacts.map(Clone::clone),
        shell, request, key: key.into(), tolerance_uv, limits, branch, bands, bounds, errors, cells })
}
