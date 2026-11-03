//! Both paths issue independent original-source ownership. A failed exact
//! family attempt never grants topology to a marching/approximate candidate.
use super::*;
use std::sync::Arc;
mod closed_proposals;
mod isolated_contact;
mod periodic_plane;
pub use isolated_contact::{
    IsolatedGraphContactSpec, QualifiedIsolatedGraphContact, discover_isolated_graph_contact,
    qualify_isolated_graph_contact,
};
pub use periodic_plane::{
    PeriodicPlaneSSKind, PeriodicSSBuildLimits, QualifiedPeriodicPlaneSS, qualify_periodic_plane_ss,
};

/// Separate finite allowances for discovery, complete mixed coverage, transition
/// census and physical embedding. A failed stage publishes no partial owner.
#[derive(Clone, Copy, Debug)]
pub struct ClosedSSBuildLimits {
    pub max_charts: usize,
    /// At most this many additional complete-coverage attempts, each with the coverage allowance.
    pub max_repairs: usize,
    pub seed_steps: usize,
    pub max_seeds: usize,
    pub proposal_rectangles: usize,
    pub coverage: CoverageLimits,
    pub components: MixedComponentDiscoveryLimits,
    pub physical: ClosedFiberBranchLimits,
}

impl Default for ClosedSSBuildLimits {
    fn default() -> Self {
        Self {
            max_charts: 64,
            max_repairs: 4,
            seed_steps: 16,
            max_seeds: 4096,
            proposal_rectangles: 100000,
            coverage: Default::default(),
            components: Default::default(),
            physical: Default::default(),
        }
    }
}

/// Construct embedded closed intersections directly from the original carriers.
/// Different charts may use different free coordinates; no global parameter,
/// fitted pcurve or polynomial/rational representation of the branch is needed.
/// This certifies the natural surface product, not arbitrary face trims or a
/// retained Boolean region. Singular roots and product-boundary contact refuse.
pub fn construct_qualified_closed_ss_branches(
    source: &FiberSources,
    limits: ClosedSSBuildLimits,
) -> crate::Result<QualifiedClosedFiberBranches> {
    crate::check(
        (1..=64).contains(&limits.max_charts) && limits.max_repairs <= 64,
        "Need bounded closed proposal chart count",
    )?;
    for s in [source.first(), source.second()] {
        crate::check(
            s.control_points.iter().flatten().all(|p| p.len() == 3),
            "Closed physical branches require spatial carriers",
        )?;
    }
    let conditioned = FiberSources::with_exact_graph_shear(source.first(), source.second()).ok();
    let source = conditioned.as_ref().unwrap_or(source);
    let (mut charts, mut used) = closed_proposals::construct(source, limits)?;
    closed_proposals::prune(&mut charts);
    let mut round = 0;
    let coverage = loop {
        let attempted = match cover_graph_fiber_surfaces(&charts, limits.coverage)? {
            Some(reduced) => reduced,
            None => cover_mixed_fiber_surfaces(&charts, limits.coverage)?,
        };
        let diagnostic = format!(
            "Closed intersection mixed root coverage remains unproved: {} charts, {:?}; proposal work {used}/{}, repair round {round}/{}",
            charts.len(),
            attempted.report(),
            limits.proposal_rectangles,
            limits.max_repairs
        );
        let unresolved = attempted.report().unresolved_box;
        if let Some(coverage) = attempted.into_coverage() {
            break coverage;
        }
        if round == limits.max_repairs
            || charts.len() == limits.max_charts
            || used == limits.proposal_rectangles
        {
            return Err(crate::input(&diagnostic));
        }
        let Some(cell) = unresolved else {
            return Err(crate::input(&diagnostic));
        };
        let (extra, work) = closed_proposals::repair(
            source,
            cell,
            limits.proposal_rectangles - used,
            limits.max_charts - charts.len(),
        )?;
        used += work;
        if extra.is_empty() {
            return Err(crate::input(&diagnostic));
        }
        charts.extend(extra);
        closed_proposals::prune(&mut charts);
        if charts.len() >= 56 {
            closed_proposals::compact(&mut charts, &mut used, limits.proposal_rectangles)?;
        }
        round += 1;
    };
    let components = connect_mixed_fiber_cover_automatic(&coverage, limits.components)?
        .into_components()
        .ok_or_else(|| crate::input("Closed intersection transition census remains unproved"))?;
    assemble_closed_fiber_branches(Arc::new(components), limits.physical)
}
#[derive(Clone, Copy, Debug, Default)]
pub struct QualifiedSSBuildLimits {
    pub periodic: PeriodicSSBuildLimits,
    /// Independent closed-chart discovery, complete census and physical proof.
    pub closed: ClosedSSBuildLimits,
    /// Independent bounded exact-identity attempt, including its refusals.
    pub quadratic: QuadraticGraphSSLimits,
    pub discovery: FiberDiscoveryLimits,
    pub components: DiscoveredComponentLimits,
    pub physical: PhysicalFiberBranchLimits,
}
#[derive(Clone, Debug)]
pub enum QualifiedSurfaceIntersection {
    PeriodicPlaneSection(QualifiedPeriodicPlaneSS),
    ClosedOrDegenerateGraph(QualifiedQuadraticGraphSS),
    RegularOpenArcs(QualifiedPhysicalFiberBranches),
    RegularClosedLoops(QualifiedClosedFiberBranches),
    IsolatedGraphContact(QualifiedIsolatedGraphContact),
}
impl QualifiedSurfaceIntersection {
    pub fn reverify(&self) -> crate::Result<()> {
        match self {
            Self::PeriodicPlaneSection(o) => o.reverify(),
            Self::ClosedOrDegenerateGraph(o) => o.reverify(),
            Self::RegularOpenArcs(o) => o.reverify(),
            Self::RegularClosedLoops(o) => o.reverify(),
            Self::IsolatedGraphContact(o) => o.reverify(),
        }
    }
    pub fn covers_entire_surfaces(&self) -> bool {
        match self {
            Self::PeriodicPlaneSection(o) => o.covers_entire_surfaces(),
            Self::ClosedOrDegenerateGraph(o) => o.covers_entire_surfaces(),
            Self::RegularOpenArcs(o) => o.covers_entire_surfaces(),
            Self::RegularClosedLoops(o) => o.owner().proof().coverage().covers_entire_surfaces(),
            Self::IsolatedGraphContact(o) => o.covers_entire_surfaces(),
        }
    }
}
pub fn construct_qualified_ss_branches(
    source: &FiberSources,
    limits: QualifiedSSBuildLimits,
) -> crate::Result<QualifiedSurfaceIntersection> {
    // Validate scalar work controls even if another path succeeds first.
    crate::check(
        (1..=4096).contains(&limits.quadratic.max_operations),
        "Invalid exact graph attempt allowance",
    )?;
    let contact = discover_isolated_graph_contact(source)?;
    // A proved even-power gap of degree >2 between identical affine XY
    // graphs excludes a pair of quadratic height graphs. Select that exact
    // family directly; no quadratic attempt has exhausted a work allowance.
    if contact
        .as_ref()
        .is_some_and(|o| o.specification().powers.iter().any(|p| *p > 2))
    {
        return Ok(QualifiedSurfaceIntersection::IsolatedGraphContact(
            contact.unwrap(),
        ));
    }
    match qualify_quadratic_graph_ss(source, limits.quadratic) {
        Ok(owner) => return Ok(QualifiedSurfaceIntersection::ClosedOrDegenerateGraph(owner)),
        Err(error) if error.code == crate::RESOURCE_LIMIT => return Err(error),
        Err(_) => {}
    }
    if let Some(owner) = periodic_plane::attempt(source, limits.periodic)? {
        return Ok(QualifiedSurfaceIntersection::PeriodicPlaneSection(owner));
    }
    if let Some(contact) = contact {
        return Ok(QualifiedSurfaceIntersection::IsolatedGraphContact(contact));
    }
    // A regular open atlas may refuse because a component has no boundary.
    // Its explicit resource error remains final; no larger fallback masks it.
    match construct_regular_open(source, limits) {
        Ok(owner) => return Ok(QualifiedSurfaceIntersection::RegularOpenArcs(owner)),
        Err(error) if error.code == crate::RESOURCE_LIMIT => return Err(error),
        Err(_) => {}
    }
    Ok(QualifiedSurfaceIntersection::RegularClosedLoops(
        construct_qualified_closed_ss_branches(source, limits.closed)?,
    ))
}
fn construct_regular_open(
    source: &FiberSources,
    limits: QualifiedSSBuildLimits,
) -> crate::Result<QualifiedPhysicalFiberBranches> {
    // Unsupported exact identities may use a separately bounded fresh proof.
    // Exhaustion was returned above, never masked by a larger fallback budget.
    // Only COMPLETE original-root coverage and physical embedding may succeed.
    let atlas = discover_fiber_surfaces(source, limits.discovery)?
        .into_atlas()
        .ok_or_else(|| {
            crate::input("No complete original-root atlas for this unsupported exact graph family")
        })?;
    let components = connect_discovered_fiber_atlas(Arc::new(atlas), limits.components)?
        .into_components()
        .ok_or_else(|| crate::input("Original root component census remains unproved"))?;
    assemble_physical_fiber_branches(Arc::new(components), limits.physical)
}
