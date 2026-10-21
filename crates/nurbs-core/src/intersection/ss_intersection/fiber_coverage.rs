//! Complete original-product exclusion outside an owned uniform root fiber.
//! This proves parameter-root coverage; physical branch/trim graphs are separate.
use super::fiber::*;
mod components;
mod construct_trace;
mod mixed;
mod world_trace;
pub use components::{
    FiberBridgeSpec, FiberComponentsResult, FiberSeparation, QualifiedFiberComponents,
    connect_mixed_fiber_cover,
};
pub use construct_trace::{FiberTraceBuild, FiberTraceBuildLimits, construct_fiber_trace};
pub use world_trace::{
    FiberWorldTraceLimits, FiberWorldTraceResult, QualifiedFiberWorldTrace,
    qualify_fiber_world_trace,
};
mod transition;
use crate::{Result, check, surface::Surface};
pub use mixed::{
    MixedCoverageKind, MixedCoverageNode, MixedFiberCoverageResult, QualifiedMixedFiberCoverage,
    cover_mixed_fiber_box, cover_mixed_fiber_surfaces,
};
pub use transition::{
    FiberTransitionResult, QualifiedFiberTransition, TransitionOrientation,
    qualify_fiber_transition,
};
type Box4 = [[f64; 2]; 4];
#[derive(Clone, Copy, Debug)]
pub struct CoverageLimits {
    pub max_cells: usize,
    pub max_rectangles: usize,
    pub max_depth: usize,
}
impl Default for CoverageLimits {
    fn default() -> Self {
        Self {
            max_cells: 4096,
            max_rectangles: 100000,
            max_depth: 24,
        }
    }
}
impl CoverageLimits {
    fn validate(self) -> Result<()> {
        check(
            (1..=65536).contains(&self.max_cells)
                && (1..=100000).contains(&self.max_rectangles)
                && self.max_depth <= 32,
            "Fiber coverage requires bounded cells, original rectangles and depth",
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FiberCoverageStatus {
    Complete,
    AdditionalRoots,
    Unresolved,
}
#[derive(Clone, Debug, PartialEq)]
pub struct FiberCoverageReport {
    pub status: FiberCoverageStatus,
    pub reason: &'static str,
    pub cells: usize,
    pub excluded_cells: usize,
    pub rectangles: usize,
    pub requested_entire_surfaces: bool,
    pub unresolved_box: Option<Box4>,
}
#[derive(Clone, Debug, PartialEq)]
pub enum FiberCoverageKind {
    Excluded(FiberReport),
    Split {
        axis: usize,
        parameter: f64,
        children: [usize; 2],
        attempt: FiberReport,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub struct FiberCoverageNode {
    domain: Box4,
    kind: Option<FiberCoverageKind>,
}
impl FiberCoverageNode {
    pub fn domain(&self) -> Box4 {
        self.domain
    }
    pub fn kind(&self) -> Option<&FiberCoverageKind> {
        self.kind.as_ref()
    }
}
pub struct FiberAtlasCoverageResult {
    report: FiberCoverageReport,
    coverage: Option<QualifiedFiberAtlasCoverage>,
    additional: Option<QualifiedFiber>,
}
impl FiberAtlasCoverageResult {
    pub fn report(&self) -> &FiberCoverageReport {
        &self.report
    }
    pub fn coverage(&self) -> Option<&QualifiedFiberAtlasCoverage> {
        self.coverage.as_ref()
    }
    pub fn into_coverage(self) -> Option<QualifiedFiberAtlasCoverage> {
        self.coverage
    }
    pub fn additional_fiber(&self) -> Option<&QualifiedFiber> {
        self.additional.as_ref()
    }
}
#[derive(Clone, Debug)]
pub struct QualifiedFiberAtlasCoverage {
    fiber: QualifiedFiber,
    other_fibers: Vec<QualifiedFiber>,
    domain: Box4,
    roots: Vec<usize>,
    nodes: Vec<FiberCoverageNode>,
    limits: CoverageLimits,
    report: FiberCoverageReport,
}
impl QualifiedFiberAtlasCoverage {
    pub fn fibers(&self) -> impl Iterator<Item = &QualifiedFiber> {
        std::iter::once(&self.fiber).chain(self.other_fibers.iter())
    }
    pub fn domain(&self) -> Box4 {
        self.domain
    }
    pub fn nodes(&self) -> &[FiberCoverageNode] {
        &self.nodes
    }
    pub fn report(&self) -> &FiberCoverageReport {
        &self.report
    }
    pub fn covers_entire_surfaces(&self) -> bool {
        self.domain == natural(self.fiber.source())
    }
    /// Replay every original exclusion and the complete stored partition tree.
    /// No cached leaf status, sample or altered root list is coverage authority.
    pub fn reverify(&self) -> Result<()> {
        self.limits.validate()?;
        let fibers = self.fibers().cloned().collect::<Vec<_>>();
        validate_family(&fibers, self.domain)?;
        let free = self.fiber.free_axis().index();
        check(
            fibers.iter().all(|f| self.domain[free] == f.domain()[free]),
            "Coverage free interval is not the complete original qualified interval",
        )?;
        let slabs = complement_union(self.domain, &fibers, free, self.limits.max_cells)?;
        check(
            self.roots.len() == slabs.len() && self.nodes.len() <= self.limits.max_cells,
            "Incomplete original complement root census",
        )?;
        let mut budget = Work {
            used: 0,
            max: self.limits.max_rectangles,
        };
        for fiber in &fibers {
            replay_fiber(fiber, &mut budget)?;
        }
        let mut seen = vec![false; self.nodes.len()];
        let mut pending = Vec::with_capacity(self.roots.len());
        for (&root, domain) in self.roots.iter().zip(slabs) {
            check(
                root < self.nodes.len() && self.nodes[root].domain == domain,
                "Original complement slab changed",
            )?;
            pending.push((root, 0usize));
        }
        pending.reverse();
        let mut excluded = 0;
        while let Some((id, depth)) = pending.pop() {
            check(
                id < self.nodes.len() && !seen[id] && depth <= self.limits.max_depth,
                "Invalid/cyclic coverage tree",
            )?;
            seen[id] = true;
            let node = &self.nodes[id];
            let fresh = run(&self.fiber, node.domain, &mut budget)?;
            let fresh = fresh
                .ok_or_else(|| crate::input("Coverage replay exhausted original-rectangle work"))?;
            match node.kind.as_ref() {
                Some(FiberCoverageKind::Excluded(old)) => {
                    check(
                        fresh.report().status == FiberStatus::Excluded && fresh.report() == old,
                        "Original exclusion receipt changed",
                    )?;
                    excluded += 1;
                }
                Some(FiberCoverageKind::Split {
                    axis,
                    parameter,
                    children,
                    attempt,
                }) => {
                    check(
                        fresh.report().status == FiberStatus::Unresolved
                            && fresh.report() == attempt,
                        "Original subdivision attempt changed",
                    )?;
                    check(
                        *axis < 4
                            && node.domain[*axis][0] < *parameter
                            && *parameter < node.domain[*axis][1],
                        "Coverage split no longer partitions its original box",
                    )?;
                    let mut left = node.domain;
                    let mut right = node.domain;
                    left[*axis][1] = *parameter;
                    right[*axis][0] = *parameter;
                    check(
                        children[0] < self.nodes.len()
                            && children[1] < self.nodes.len()
                            && self.nodes[children[0]].domain == left
                            && self.nodes[children[1]].domain == right,
                        "Coverage child boxes omit or change an original region",
                    )?;
                    pending.push((children[1], depth + 1));
                    pending.push((children[0], depth + 1));
                }
                None => return Err(crate::input("Unprocessed coverage node")),
            }
        }
        check(
            seen.iter().all(|v| *v)
                && self.report.status == FiberCoverageStatus::Complete
                && self.report.cells == self.nodes.len()
                && self.report.excluded_cells == excluded
                && self.report.rectangles == budget.used
                && self.report.requested_entire_surfaces == self.covers_entire_surfaces(),
            "Coverage receipt or reachability census changed",
        )
    }
}
struct Work {
    used: usize,
    max: usize,
}
fn natural(source: &FiberSources) -> Box4 {
    let d = |s: &Surface| {
        [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ]
    };
    let a = d(source.first());
    let b = d(source.second());
    [a[0], a[1], b[0], b[1]]
}
fn validate_domain(fiber: &QualifiedFiber, domain: Box4) -> Result<()> {
    let all = natural(fiber.source());
    let tube = fiber.domain();
    let free = fiber.free_axis().index();
    check(
        (0..4).all(|k| {
            domain[k].iter().all(|v| v.is_finite())
                && all[k][0] <= domain[k][0]
                && domain[k][1] <= all[k][1]
                && domain[k][0] <= tube[k][0]
                && tube[k][1] <= domain[k][1]
                && (if k == free {
                    domain[k][0] <= domain[k][1]
                } else {
                    domain[k][0] < domain[k][1]
                })
        }),
        "Coverage domain must contain its original fiber and lie inside both original surfaces",
    )
}
// Axis peeling creates at most six closed slabs. Shared interfaces are included
// on both sides; strict fiber inclusion never permits dropping a boundary root.
fn complement(domain: Box4, tube: Box4, free: usize) -> Vec<Box4> {
    let mut rem = domain;
    let mut slabs = Vec::with_capacity(6);
    for k in (0..4).filter(|&k| k != free) {
        if rem[k][0] < tube[k][0] {
            let mut d = rem;
            d[k][1] = tube[k][0];
            slabs.push(d);
            rem[k][0] = tube[k][0];
        }
        if tube[k][1] < rem[k][1] {
            let mut d = rem;
            d[k][0] = tube[k][1];
            slabs.push(d);
            rem[k][1] = tube[k][1];
        }
    }
    slabs
}
fn replay_fiber(fiber: &QualifiedFiber, work: &mut Work) -> Result<()> {
    check(
        work.used < work.max,
        "Missing original-fiber replay allowance",
    )?;
    let mut limits = fiber.limits();
    limits.max_rectangles = limits.max_rectangles.min(work.max - work.used);
    let fresh = fiber
        .source()
        .certify(fiber.domain(), fiber.free_axis(), limits)?;
    work.used += fresh.report().rectangles;
    check(
        fresh.report().status == FiberStatus::UniformUniqueRoot && fresh.report() == fiber.report(),
        "Original root fiber could not be freshly replayed within the shared work allowance",
    )
}
fn run(fiber: &QualifiedFiber, domain: Box4, work: &mut Work) -> Result<Option<FiberResult>> {
    if work.used == work.max {
        return Ok(None);
    }
    let r = fiber.source().certify(
        domain,
        fiber.free_axis(),
        FiberLimits {
            max_rectangles: work.max - work.used,
            preconditioner: None,
        },
    )?;
    work.used += r.report().rectangles;
    Ok(Some(r))
}
/// Prove parameter-root coverage for disjoint tubes sharing one free axis.
/// This does not establish distinct physical branches or a trim graph.
pub fn cover_fiber_atlas_surfaces(
    fibers: &[QualifiedFiber],
    limits: CoverageLimits,
) -> Result<FiberAtlasCoverageResult> {
    check(!fibers.is_empty(), "Fiber atlas must not be empty")?;
    cover_fiber_atlas_box(fibers, natural(fibers[0].source()), limits)
}
pub fn cover_fiber_atlas_box(
    fibers: &[QualifiedFiber],
    domain: Box4,
    limits: CoverageLimits,
) -> Result<FiberAtlasCoverageResult> {
    limits.validate()?;
    validate_family(fibers, domain)?;
    let fiber = &fibers[0];
    let mut result = FiberAtlasCoverageResult {
        report: FiberCoverageReport {
            status: FiberCoverageStatus::Unresolved,
            reason: "free-interval-incomplete",
            cells: 0,
            excluded_cells: 0,
            rectangles: 0,
            requested_entire_surfaces: domain == natural(fiber.source()),
            unresolved_box: None,
        },
        coverage: None,
        additional: None,
    };
    let free = fiber.free_axis().index();
    if fibers.iter().any(|f| domain[free] != f.domain()[free]) {
        return Ok(result);
    }
    let mut work = Work {
        used: 0,
        max: limits.max_rectangles,
    };
    if fibers.iter().any(|f| replay_fiber(f, &mut work).is_err()) {
        result.report.reason = "fiber-replay-unproven";
        result.report.rectangles = work.used;
        return Ok(result);
    }
    let slabs = match complement_union(domain, fibers, free, limits.max_cells) {
        Ok(slabs) => slabs,
        Err(_) => {
            result.report.reason = "cell-budget";
            result.report.rectangles = work.used;
            return Ok(result);
        }
    };
    if slabs.len() > limits.max_cells {
        result.report.reason = "cell-budget";
        result.report.rectangles = work.used;
        return Ok(result);
    }
    let mut nodes = slabs
        .into_iter()
        .map(|domain| FiberCoverageNode { domain, kind: None })
        .collect::<Vec<_>>();
    let roots = (0..nodes.len()).collect::<Vec<_>>();
    let mut pending = roots
        .iter()
        .rev()
        .map(|&id| (id, 0usize))
        .collect::<Vec<_>>();
    let mut excluded = 0;
    while let Some((id, depth)) = pending.pop() {
        let cell = nodes[id].domain;
        let Some(r) = run(fiber, cell, &mut work)? else {
            result.report.reason = "rectangle-budget";
            result.report.unresolved_box = Some(cell);
            break;
        };
        match r.report().status {
            FiberStatus::Excluded => {
                nodes[id].kind = Some(FiberCoverageKind::Excluded(r.report().clone()));
                excluded += 1;
            }
            FiberStatus::UniformUniqueRoot => {
                result.report.status = FiberCoverageStatus::AdditionalRoots;
                result.report.reason = "additional-original-root-fiber";
                result.report.unresolved_box = Some(cell);
                result.additional = r.into_fiber();
                break;
            }
            FiberStatus::Unresolved => {
                if depth == limits.max_depth {
                    result.report.reason = "depth-limit";
                    result.report.unresolved_box = Some(cell);
                    break;
                }
                if nodes.len() + 2 > limits.max_cells {
                    result.report.reason = "cell-budget";
                    result.report.unresolved_box = Some(cell);
                    break;
                }
                let axis = (0..4)
                    .filter(|&a| domain[a][0] < domain[a][1])
                    .max_by(|&a, &b| {
                        let width =
                            |k: usize| (cell[k][1] - cell[k][0]) / (domain[k][1] - domain[k][0]);
                        width(a).total_cmp(&width(b)).then_with(|| b.cmp(&a))
                    })
                    .unwrap();
                let middle = 0.5 * cell[axis][0] + 0.5 * cell[axis][1];
                if middle <= cell[axis][0] || middle >= cell[axis][1] {
                    result.report.reason = "subdivision-unrepresentable";
                    result.report.unresolved_box = Some(cell);
                    break;
                }
                let mut left = cell;
                let mut right = cell;
                left[axis][1] = middle;
                right[axis][0] = middle;
                let children = [nodes.len(), nodes.len() + 1];
                nodes.push(FiberCoverageNode {
                    domain: left,
                    kind: None,
                });
                nodes.push(FiberCoverageNode {
                    domain: right,
                    kind: None,
                });
                nodes[id].kind = Some(FiberCoverageKind::Split {
                    axis,
                    parameter: middle,
                    children,
                    attempt: r.report().clone(),
                });
                pending.push((children[1], depth + 1));
                pending.push((children[0], depth + 1));
            }
        }
    }
    result.report.cells = nodes.len();
    result.report.excluded_cells = excluded;
    result.report.rectangles = work.used;
    if result.additional.is_none()
        && result.report.unresolved_box.is_none()
        && nodes.iter().all(|n| n.kind.is_some())
        && excluded > 0
    {
        result.report.status = FiberCoverageStatus::Complete;
        result.report.reason = "complete-original-fiber-complement";
        result.coverage = Some(QualifiedFiberAtlasCoverage {
            fiber: fiber.clone(),
            other_fibers: fibers[1..].to_vec(),
            domain,
            roots,
            nodes,
            limits,
            report: result.report.clone(),
        });
    } else if nodes.is_empty()
        && result.report.unresolved_box.is_none()
        && result.report.reason == "free-interval-incomplete"
    {
        result.report.status = FiberCoverageStatus::Complete;
        result.report.reason = "fiber-is-complete-requested-product";
        result.coverage = Some(QualifiedFiberAtlasCoverage {
            fiber: fiber.clone(),
            other_fibers: fibers[1..].to_vec(),
            domain,
            roots,
            nodes,
            limits,
            report: result.report.clone(),
        });
    }
    Ok(result)
}

fn validate_family(fibers: &[QualifiedFiber], domain: Box4) -> Result<()> {
    check(
        (1..=64).contains(&fibers.len()),
        "Fiber atlas requires one to 64 tubes",
    )?;
    let first = &fibers[0];
    let free = first.free_axis().index();
    for (i, f) in fibers.iter().enumerate() {
        validate_domain(f, domain)?;
        check(
            f.free_axis() == first.free_axis()
                && f.source().first() == first.source().first()
                && f.source().second() == first.source().second(),
            "Fiber atlas must share actual ordered surfaces and free axis",
        )?;
        for previous in &fibers[..i] {
            check(
                (0..4).filter(|&k| k != free).any(|k| {
                    f.domain()[k][1] <= previous.domain()[k][0]
                        || previous.domain()[k][1] <= f.domain()[k][0]
                }),
                "Fiber atlas tubes have overlapping interiors",
            )?;
        }
    }
    Ok(())
}
fn complement_union(
    domain: Box4,
    fibers: &[QualifiedFiber],
    free: usize,
    max: usize,
) -> Result<Vec<Box4>> {
    let mut cells = vec![domain];
    for f in fibers {
        let mut next = Vec::with_capacity(cells.len());
        for cell in cells {
            let mut clip = cell;
            for k in (0..4).filter(|&k| k != free) {
                clip[k] = [
                    cell[k][0].max(f.domain()[k][0]),
                    cell[k][1].min(f.domain()[k][1]),
                ];
            }
            let pieces = if (0..4)
                .filter(|&k| k != free)
                .any(|k| clip[k][0] >= clip[k][1])
            {
                vec![cell]
            } else {
                complement(cell, clip, free)
            };
            check(
                next.len() + pieces.len() <= max,
                "Fiber atlas complement exceeds cell budget",
            )?;
            next.extend(pieces);
        }
        cells = next;
    }
    Ok(cells)
}
/// Complete coverage of exactly one parameter-root graph.
#[derive(Clone, Debug)]
pub struct QualifiedFiberCoverage {
    atlas: QualifiedFiberAtlasCoverage,
}
impl QualifiedFiberCoverage {
    pub fn fiber(&self) -> &QualifiedFiber {
        &self.atlas.fiber
    }
    pub fn domain(&self) -> Box4 {
        self.atlas.domain()
    }
    pub fn nodes(&self) -> &[FiberCoverageNode] {
        self.atlas.nodes()
    }
    pub fn report(&self) -> &FiberCoverageReport {
        self.atlas.report()
    }
    pub fn covers_entire_surfaces(&self) -> bool {
        self.atlas.covers_entire_surfaces()
    }
    pub fn reverify(&self) -> Result<()> {
        check(
            self.atlas.other_fibers.is_empty(),
            "Single fiber coverage contains additional fibers",
        )?;
        self.atlas.reverify()
    }
}
pub struct FiberCoverageResult {
    report: FiberCoverageReport,
    coverage: Option<QualifiedFiberCoverage>,
    additional: Option<QualifiedFiber>,
}
impl FiberCoverageResult {
    pub fn report(&self) -> &FiberCoverageReport {
        &self.report
    }
    pub fn coverage(&self) -> Option<&QualifiedFiberCoverage> {
        self.coverage.as_ref()
    }
    pub fn into_coverage(self) -> Option<QualifiedFiberCoverage> {
        self.coverage
    }
    pub fn additional_fiber(&self) -> Option<&QualifiedFiber> {
        self.additional.as_ref()
    }
}
pub fn cover_fiber_surfaces(
    fiber: &QualifiedFiber,
    limits: CoverageLimits,
) -> Result<FiberCoverageResult> {
    cover_fiber_box(fiber, natural(fiber.source()), limits)
}
pub fn cover_fiber_box(
    fiber: &QualifiedFiber,
    domain: Box4,
    limits: CoverageLimits,
) -> Result<FiberCoverageResult> {
    let result = cover_fiber_atlas_box(std::slice::from_ref(fiber), domain, limits)?;
    Ok(FiberCoverageResult {
        report: result.report,
        coverage: result
            .coverage
            .map(|atlas| QualifiedFiberCoverage { atlas }),
        additional: result.additional,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_rejects_missing_original_slabs_or_forged_exclusion_receipts() {
        let a = Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: [0.2, 0.4]
                .iter()
                .map(|&x| vec![vec![x, 0., 0.], vec![x, 0.5, 0.], vec![x, 1., 1.]])
                .collect(),
            weights: vec![vec![1.; 3]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let b = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.25], vec![0., 1., 0.25]],
                vec![vec![1., 0., 0.25], vec![1., 1., 0.25]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let fiber = FiberSources::new(&a, &b)
            .unwrap()
            .certify(
                [[0., 1.], [0.4, 0.6], [0.1, 0.5], [0.3, 0.7]],
                FiberAxis::FirstU,
                Default::default(),
            )
            .unwrap()
            .into_fiber()
            .unwrap();
        let owner = cover_fiber_atlas_surfaces(std::slice::from_ref(&fiber), Default::default())
            .unwrap()
            .into_coverage()
            .unwrap();
        owner.reverify().unwrap();
        let mut bad = owner.clone();
        bad.roots.pop();
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.nodes.pop();
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.nodes[0].domain[1][1] -= 0.001;
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        if let Some(FiberCoverageKind::Excluded(r)) = &mut bad.nodes[0].kind {
            r.status = FiberStatus::UniformUniqueRoot;
        }
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.nodes.push(bad.nodes[0].clone());
        assert!(bad.reverify().is_err());
        let mut d = fiber.domain();
        d[0] = [0.2, 0.8];
        let partial = fiber
            .source()
            .certify(d, FiberAxis::FirstU, Default::default())
            .unwrap()
            .into_fiber()
            .unwrap();
        let mut forged =
            cover_fiber_atlas_box(std::slice::from_ref(&partial), d, Default::default())
                .unwrap()
                .into_coverage()
                .unwrap();
        assert!(forged.nodes.is_empty());
        forged.reverify().unwrap();
        forged.domain[0] = [0., 1.];
        assert!(forged.reverify().is_err());
    }
}

mod discovery;
pub use discovery::{FiberDiscoveryLimits,FiberDiscoveryKind,FiberDiscoveryNode,FiberDiscoveryResult,QualifiedDiscoveredFiberAtlas,discover_fiber_box,discover_fiber_surfaces};
mod discovered_components;
pub use discovered_components::{DiscoveredComponentLimits,OriginalFiberPointConnection,QualifiedDiscoveredFiberComponents,DiscoveredFiberComponentsResult,connect_discovered_fiber_atlas};

mod physical_branches;
pub use physical_branches::{PhysicalFiberBranchLimits, OriginalPhysicalFiberBranch, OriginalPhysicalFiberVertex, OriginalPhysicalFiberTrim, QualifiedPhysicalFiberBranches, assemble_physical_fiber_branches};
