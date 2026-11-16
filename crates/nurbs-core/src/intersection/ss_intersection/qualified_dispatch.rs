//! Both paths issue independent original-source ownership. A failed exact
//! family attempt never grants topology to a marching/approximate candidate.
use super::*;
use std::sync::Arc;
mod closed_proposals;
mod isolated_contact;
mod periodic_plane;
pub use isolated_contact::{
    ExactIsolatedGraphContactSpec, IsolatedGraphContactSpec, QualifiedConvexGraphContact, QualifiedIsolatedGraphContact,
    discover_convex_graph_contact, qualify_convex_graph_contact,
    discover_isolated_graph_contact, qualify_exact_isolated_graph_contact,
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
    construct_closed_branches(source, None, limits)
}

/// Prove every closed component in a specified native UV product. A root
/// touching its boundary refuses. Roots outside the product are out of scope.
pub fn construct_qualified_closed_ss_branches_in_box(
    source: &FiberSources,
    domain: [[f64; 2]; 4],
    limits: ClosedSSBuildLimits,
) -> crate::Result<QualifiedClosedFiberBranches> {
    construct_closed_branches(source, Some(domain), limits)
}

fn construct_closed_branches(
    source: &FiberSources,
    domain: Option<[[f64; 2]; 4]>,
    limits: ClosedSSBuildLimits,
) -> crate::Result<QualifiedClosedFiberBranches> {
    crate::check(
        (1..=fiber_coverage::MAX_MIXED_FIBERS).contains(&limits.max_charts) && limits.max_repairs <= 128,
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
    let natural = closed_proposals::natural_product(source);
    let window = domain.unwrap_or(natural);
    crate::check((0..4).all(|axis| {
        let [lo, hi] = window[axis];
        lo.is_finite() && hi.is_finite()
            && natural[axis][0] <= lo && lo < hi && hi <= natural[axis][1]
    }), "Invalid closed intersection proposal domain")?;
    // Periodic traces currently rely on the overlap produced by the
    // natural-coordinate proposal scale. The census remains windowed.
    let proposal_domain = if [source.first(), source.second()]
        .into_iter()
        .any(|surface| surface.periodic_u || surface.periodic_v)
    {
        natural
    } else {
        window
    };
    let (mut charts, mut used) = closed_proposals::construct(source, proposal_domain, limits)?;
    charts = restrict_closed_charts(
        source,
        charts,
        domain,
        &mut used,
        limits.proposal_rectangles,
    )?;
    closed_proposals::prune(&mut charts);
    let mut round = 0;
    let coverage = loop {
        let attempted = if let Some(domain) = domain {
            match fiber_coverage::cover_graph_fiber_box(&charts, domain, limits.coverage)? {
                Some(reduced) => reduced,
                None => cover_mixed_fiber_box(&charts, domain, limits.coverage)?,
            }
        } else {
            match cover_graph_fiber_surfaces(&charts, limits.coverage)? {
                Some(reduced) => reduced,
                None => cover_mixed_fiber_surfaces(&charts, limits.coverage)?,
            }
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
            proposal_domain,
            cell,
            limits.proposal_rectangles - used,
            limits.max_charts - charts.len(),
        )?;
        used += work;
        let extra =
            restrict_closed_charts(source, extra, domain, &mut used, limits.proposal_rectangles)?;
        if extra.is_empty() {
            return Err(crate::input(&diagnostic));
        }
        charts.extend(extra);
        closed_proposals::prune(&mut charts);
        // Compaction only removes charts after proving coverage by surviving
        // charts. Their finite UV bounds therefore stay inside the window.
        if charts.len() >= limits.max_charts.saturating_sub(8).max(1) {
            closed_proposals::compact(&mut charts, &mut used, limits.proposal_rectangles)?;
        }
        round += 1;
    };
    let connection = connect_mixed_fiber_cover_automatic(&coverage, limits.components)?;
    let reason = connection.reason();
    let rectangles = connection.rectangles();
    let searches = connection.searches().len();
    let unresolved = connection.unresolved_pair().map(|pair| {
        let details = connection.searches().iter().rev().find(|search| search.charts == pair)
            .map(|search| {
                let attempt = search.attempts.last().map_or("no bridge attempt", |a| a.reason.as_str());
                format!("{}; {attempt}", search.reason)
            }).unwrap_or_else(|| "pair search unavailable".into());
        let [first, second] = pair.map(|id| {
            let chart = &coverage.fibers()[id];
            (chart.free_axis(), chart.domain())
        });
        format!(", charts {pair:?} {first:?} {second:?}: {details}")
    }).unwrap_or_default();
    let mut components = connection.into_components().ok_or_else(|| crate::input(&format!(
        "Closed intersection transition census remains unproved: {reason}, {searches} pair searches, {rectangles} rectangles{unresolved}",
    )))?;
    // A point bridge proves connectivity but has no open clock interval for
    // an oriented traversal. An original chart centered there may overlap
    // both sides. Rebuild coverage and connectivity before adopting it.
    if !components.proof().point_connections().is_empty()
        && charts.len() < limits.max_charts && round < limits.max_repairs
    {
        let mut extra = Vec::new();
        for connection in components.proof().point_connections() {
            if used == limits.proposal_rectangles || charts.len() + extra.len() == limits.max_charts {
                break;
            }
            let point = connection.point();
            let Some(image) = point.report().root_image else { continue };
            let free = point.free_axis().index();
            let cell = std::array::from_fn(|axis| {
                if axis == free { point.domain()[axis] }
                else { image[axis - usize::from(axis > free)] }
            });
            let (new, work) = closed_proposals::repair(
                source, proposal_domain, cell, limits.proposal_rectangles - used,
                limits.max_charts - charts.len() - extra.len(),
            )?;
            used += work;
            extra.extend(restrict_closed_charts(source, new, domain, &mut used, limits.proposal_rectangles)?);
        }
        if !extra.is_empty() {
            charts.extend(extra);
            let rebuilt = if let Some(domain) = domain {
                fiber_coverage::cover_graph_fiber_box(&charts, domain, limits.coverage)?
            } else {
                cover_graph_fiber_surfaces(&charts, limits.coverage)?
            };
            if let Some(rebuilt) = rebuilt.and_then(|attempt| attempt.into_coverage()) {
                if let Some(candidate) = connect_mixed_fiber_cover_automatic(&rebuilt, limits.components)?
                    .into_components()
                {
                    if candidate.proof().components().len() == components.proof().components().len() {
                        components = candidate;
                    }
                }
            }
        }
    }
    if domain.is_some() {
        fiber_coverage::assemble_closed_fiber_branches_in_box(Arc::new(components), limits.physical)
    } else {
        assemble_closed_fiber_branches(Arc::new(components), limits.physical)
    }
}
fn restrict_closed_charts(
    source: &FiberSources,
    charts: Vec<QualifiedFiber>,
    domain: Option<[[f64; 2]; 4]>,
    used: &mut usize,
    budget: usize,
) -> crate::Result<Vec<QualifiedFiber>> {
    if let Some(domain) = domain {
        let mut restricted = Vec::new();
        for chart in charts {
            let overlap = std::array::from_fn(|axis| {
                let range = chart.domain()[axis];
                [range[0].max(domain[axis][0]), range[1].min(domain[axis][1])]
            });
            if overlap.iter().any(|bounds| bounds[0] >= bounds[1]) {
                continue;
            }
            if overlap == chart.domain() {
                restricted.push(chart);
                continue;
            }
            let remaining = budget.saturating_sub(*used);
            if remaining == 0 {
                return Err(crate::input(
                    "Closed chart restriction exhausted proposal work",
                ));
            }
            // Contract dependent coordinates within the clipped
            // domain while retaining the complete clipped free interval.
            let certified = source.certify_contracted(
                overlap, chart.free_axis(), remaining.min(100000),
            )?;
            *used += certified.rectangles();
            if let Some(chart) = certified.into_fiber() {
                restricted.push(chart);
            }
        }
        Ok(restricted)
    } else {
        Ok(charts)
    }
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
    ConvexGraphContact(QualifiedConvexGraphContact),
}
impl QualifiedSurfaceIntersection {
    pub fn reverify(&self) -> crate::Result<()> {
        match self {
            Self::PeriodicPlaneSection(o) => o.reverify(),
            Self::ClosedOrDegenerateGraph(o) => o.reverify(),
            Self::RegularOpenArcs(o) => o.reverify(),
            Self::RegularClosedLoops(o) => o.reverify(),
            Self::IsolatedGraphContact(o) => o.reverify(),
            Self::ConvexGraphContact(o) => o.reverify(),
        }
    }
    pub fn covers_entire_surfaces(&self) -> bool {
        match self {
            Self::PeriodicPlaneSection(o) => o.covers_entire_surfaces(),
            Self::ClosedOrDegenerateGraph(o) => o.covers_entire_surfaces(),
            Self::RegularOpenArcs(o) => o.covers_entire_surfaces(),
            Self::RegularClosedLoops(o) => o.owner().proof().coverage().covers_entire_surfaces(),
            Self::IsolatedGraphContact(o) => o.covers_entire_surfaces(),
            Self::ConvexGraphContact(_) => true,
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
        .is_some_and(|o| o.exact_specification().powers.iter().any(|p| *p > 2))
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
    if let Some(contact) = discover_convex_graph_contact(source, 256)? {
        return Ok(QualifiedSurfaceIntersection::ConvexGraphContact(contact));
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
