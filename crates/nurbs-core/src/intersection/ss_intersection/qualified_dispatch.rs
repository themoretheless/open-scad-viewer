//! Both paths issue independent original-source ownership. A failed exact
//! family attempt never grants topology to a marching/approximate candidate.
use super::*;
use std::sync::Arc;
#[derive(Clone, Copy, Debug, Default)]
pub struct QualifiedSSBuildLimits {
    /// Independent bounded exact-identity attempt, including its refusals.
    pub quadratic: QuadraticGraphSSLimits,
    pub discovery: FiberDiscoveryLimits,
    pub components: DiscoveredComponentLimits,
    pub physical: PhysicalFiberBranchLimits,
}
#[derive(Clone, Debug)]
pub enum QualifiedSurfaceIntersection {
    ClosedOrDegenerateGraph(QualifiedQuadraticGraphSS),
    RegularOpenArcs(QualifiedPhysicalFiberBranches),
}
impl QualifiedSurfaceIntersection {
    pub fn reverify(&self) -> crate::Result<()> {
        match self {
            Self::ClosedOrDegenerateGraph(o) => o.reverify(),
            Self::RegularOpenArcs(o) => o.reverify(),
        }
    }
    pub fn covers_entire_surfaces(&self) -> bool {
        match self {
            Self::ClosedOrDegenerateGraph(o) => o.covers_entire_surfaces(),
            Self::RegularOpenArcs(o) => o.covers_entire_surfaces(),
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
    match qualify_quadratic_graph_ss(source, limits.quadratic) {
        Ok(owner) => return Ok(QualifiedSurfaceIntersection::ClosedOrDegenerateGraph(owner)),
        Err(error) if error.code == crate::RESOURCE_LIMIT => return Err(error),
        Err(_) => {}
    }
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
    Ok(QualifiedSurfaceIntersection::RegularOpenArcs(
        assemble_physical_fiber_branches(Arc::new(components), limits.physical)?,
    ))
}
