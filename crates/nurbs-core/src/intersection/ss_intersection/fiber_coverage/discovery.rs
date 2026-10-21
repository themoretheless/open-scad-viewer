//! Automatic bounded original-product atlas discovery. Proposals grant no
//! authority: every closed leaf owns either a fresh exclusion or a root fiber.
//! Parameter-root coverage is separate from physical branch/trim ownership.
use super::*;
use std::collections::VecDeque;
const AXES: [FiberAxis; 4] = [
    FiberAxis::FirstU,
    FiberAxis::FirstV,
    FiberAxis::SecondU,
    FiberAxis::SecondV,
];
#[derive(Clone, Copy, Debug)]
pub struct FiberDiscoveryLimits {
    pub coverage: CoverageLimits,
    pub max_fibers: usize,
}
impl Default for FiberDiscoveryLimits {
    fn default() -> Self {
        Self {
            coverage: CoverageLimits::default(),
            max_fibers: 64,
        }
    }
}
impl FiberDiscoveryLimits {
    fn validate(self) -> Result<()> {
        self.coverage.validate()?;
        check(
            (1..=64).contains(&self.max_fibers),
            "Need bounded original atlas discovery fibers",
        )
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum FiberDiscoveryKind {
    Excluded(FiberReport),
    Fiber {
        index: usize,
    },
    Split {
        axis: usize,
        parameter: f64,
        children: [usize; 2],
    },
}
#[derive(Clone, Debug, PartialEq)]
pub struct FiberDiscoveryNode {
    domain: Box4,
    kind: Option<FiberDiscoveryKind>,
}
impl FiberDiscoveryNode {
    pub fn domain(&self) -> Box4 {
        self.domain
    }
    pub fn kind(&self) -> Option<&FiberDiscoveryKind> {
        self.kind.as_ref()
    }
}
#[derive(Clone, Debug)]
pub struct QualifiedDiscoveredFiberAtlas {
    source: FiberSources,
    domain: Box4,
    fibers: Vec<QualifiedFiber>,
    nodes: Vec<FiberDiscoveryNode>,
    limits: FiberDiscoveryLimits,
}
pub struct FiberDiscoveryResult {
    report: FiberCoverageReport,
    atlas: Option<QualifiedDiscoveredFiberAtlas>,
}
impl FiberDiscoveryResult {
    pub fn report(&self) -> &FiberCoverageReport {
        &self.report
    }
    pub fn atlas(&self) -> Option<&QualifiedDiscoveredFiberAtlas> {
        self.atlas.as_ref()
    }
    pub fn into_atlas(self) -> Option<QualifiedDiscoveredFiberAtlas> {
        self.atlas
    }
}
fn contains(outer: Box4, inner: Box4) -> bool {
    (0..4).all(|a| outer[a][0] <= inner[a][0] && inner[a][1] <= outer[a][1])
}
fn validate_box(source: &FiberSources, domain: Box4) -> Result<()> {
    check(
        contains(natural(source), domain)
            && domain.iter().all(|d| {
                d.iter().all(|v| v.is_finite()) && d[0] < d[1] && (d[1] - d[0]).is_finite()
            }),
        "Need a positive finite original atlas product inside both sources",
    )
}
fn attempt(
    source: &FiberSources,
    domain: Box4,
    axis: FiberAxis,
    work: &mut Work,
) -> Result<Option<super::super::fiber::FiberResult>> {
    if work.used == work.max {
        return Ok(None);
    }
    let r = source.certify(
        domain,
        axis,
        FiberLimits {
            max_rectangles: work.max - work.used,
            preconditioner: None,
        },
    )?;
    work.used = work
        .used
        .checked_add(r.report().rectangles)
        .filter(|n| *n <= work.max)
        .ok_or_else(|| crate::input("Original atlas work exceeded allowance"))?;
    Ok(Some(r))
}
fn expanded(cell: Box4, domain: Box4, free: usize) -> Box4 {
    std::array::from_fn(|a| {
        if a == free {
            cell[a]
        } else {
            let half = (cell[a][1] - cell[a][0]) * 0.5;
            [
                (cell[a][0] - half).max(domain[a][0]).min(cell[a][0]),
                (cell[a][1] + half).min(domain[a][1]).max(cell[a][1]),
            ]
        }
    })
}
fn split(cell: Box4, root: Box4) -> Option<(usize, f64)> {
    let mut candidates: [(usize, f64); 4] =
        std::array::from_fn(|a| (a, (cell[a][1] - cell[a][0]) / (root[a][1] - root[a][0])));
    candidates.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    candidates.into_iter().find_map(|(a, _)| {
        let m = cell[a][0] * 0.5 + cell[a][1] * 0.5;
        (cell[a][0] < m && m < cell[a][1]).then_some((a, m))
    })
}
impl QualifiedDiscoveredFiberAtlas {
    pub fn source(&self) -> &FiberSources {
        &self.source
    }
    pub fn domain(&self) -> Box4 {
        self.domain
    }
    pub fn fibers(&self) -> &[QualifiedFiber] {
        &self.fibers
    }
    pub fn nodes(&self) -> &[FiberDiscoveryNode] {
        &self.nodes
    }
    pub fn covers_entire_surfaces(&self) -> bool {
        self.domain == natural(&self.source)
    }
    /// Independently replay the complete CLOSED product partition and every
    /// actual original fiber/exclusion with one bounded rectangle ledger.
    pub fn reverify(&self) -> Result<()> {
        let mut work = Work {
            used: 0,
            max: self.limits.coverage.max_rectangles,
        };
        self.replay(&mut work)
    }
    pub(super) fn replay(&self, work: &mut Work) -> Result<()> {
        check(
            work.used <= work.max,
            "Invalid shared discovery replay work",
        )?;
        let mut local = Work {
            used: 0,
            max: self
                .limits
                .coverage
                .max_rectangles
                .min(work.max - work.used),
        };
        let result = self.verify_with(&mut local);
        work.used += local.used;
        result
    }
    fn verify_with(&self, work: &mut Work) -> Result<()> {
        self.limits.validate()?;
        validate_box(&self.source, self.domain)?;
        check(
            !self.nodes.is_empty()
                && self.nodes.len() <= self.limits.coverage.max_cells
                && self.nodes[0].domain == self.domain
                && self.fibers.len() <= self.limits.max_fibers,
            "Original discovery root/cell/fiber census changed",
        )?;
        for f in &self.fibers {
            check(
                f.source().first() == self.source.first()
                    && f.source().second() == self.source.second()
                    && contains(self.domain, f.domain()),
                "Discovered fiber rebound its original source/product",
            )?;
            replay_fiber(f, work)?;
        }
        let mut seen = vec![false; self.nodes.len()];
        let mut used = vec![false; self.fibers.len()];
        let mut pending = vec![(0, 0)];
        while let Some((id, depth)) = pending.pop() {
            check(
                id < self.nodes.len() && !seen[id] && depth <= self.limits.coverage.max_depth,
                "Discovery tree omits, repeats or cycles through an original cell",
            )?;
            seen[id] = true;
            let n = &self.nodes[id];
            match n
                .kind
                .as_ref()
                .ok_or_else(|| crate::input("Unproved original discovery leaf"))?
            {
                FiberDiscoveryKind::Fiber { index } => {
                    check(
                        *index < self.fibers.len()
                            && contains(self.fibers[*index].domain(), n.domain),
                        "Discovery leaf is not inside its actual original fiber",
                    )?;
                    used[*index] = true;
                }
                FiberDiscoveryKind::Excluded(report) => {
                    let fresh = attempt(&self.source, n.domain, FiberAxis::FirstU, work)?
                        .ok_or_else(|| {
                            crate::input("Original discovery replay rectangle budget")
                        })?;
                    check(
                        report.status == FiberStatus::Excluded && fresh.report() == report,
                        "Original discovery exclusion receipt changed",
                    )?;
                }
                FiberDiscoveryKind::Split {
                    axis,
                    parameter,
                    children,
                } => {
                    check(
                        *axis < 4
                            && n.domain[*axis][0] < *parameter
                            && *parameter < n.domain[*axis][1]
                            && children[0] != children[1]
                            && children.iter().all(|i| *i < self.nodes.len()),
                        "Original discovery split changed",
                    )?;
                    let mut a = n.domain;
                    let mut b = n.domain;
                    a[*axis][1] = *parameter;
                    b[*axis][0] = *parameter;
                    check(
                        self.nodes[children[0]].domain == a && self.nodes[children[1]].domain == b,
                        "Discovery split omitted a closed original product face",
                    )?;
                    pending.push((children[1], depth + 1));
                    pending.push((children[0], depth + 1));
                }
            }
        }
        check(
            seen.iter().all(|v| *v) && used.iter().all(|v| *v),
            "Discovery stores orphan cells or unrelated fibers",
        )
    }
}
pub fn discover_fiber_box(
    source: &FiberSources,
    domain: Box4,
    limits: FiberDiscoveryLimits,
) -> Result<FiberDiscoveryResult> {
    limits.validate()?;
    validate_box(source, domain)?;
    let mut result = FiberDiscoveryResult {
        atlas: None,
        report: FiberCoverageReport {
            status: FiberCoverageStatus::Unresolved,
            reason: "discovery-unresolved",
            cells: 1,
            excluded_cells: 0,
            rectangles: 0,
            requested_entire_surfaces: domain == natural(source),
            unresolved_box: None,
        },
    };
    let mut nodes = vec![FiberDiscoveryNode { domain, kind: None }];
    let mut fibers: Vec<QualifiedFiber> = Vec::new();
    let mut work = Work {
        used: 0,
        max: limits.coverage.max_rectangles,
    };
    let mut pending = VecDeque::from([(0, 0)]);
    let mut failure = None;
    'cells: while let Some((id, depth)) = pending.pop_front() {
        let cell = nodes[id].domain;
        if let Some(index) = fibers.iter().position(|f| contains(f.domain(), cell)) {
            nodes[id].kind = Some(FiberDiscoveryKind::Fiber { index });
            continue;
        }
        for round in 0..2 {
            for (free, &axis) in AXES.iter().enumerate() {
                let proposal = if round == 0 {
                    cell
                } else {
                    expanded(cell, domain, free)
                };
                if round == 1 && proposal == cell {
                    continue;
                }
                let Some(r) = attempt(source, proposal, axis, &mut work)? else {
                    failure = Some(("rectangle-budget", cell));
                    break 'cells;
                };
                if r.report().reason == "rectangle-budget" {
                    failure = Some(("rectangle-budget", cell));
                    break 'cells;
                }
                // Only the first, unexpanded actual-cell attempt supplies an
                // exclusion leaf, whose replay axis is retained by convention.
                if round == 0 && free == 0 && r.report().status == FiberStatus::Excluded {
                    nodes[id].kind = Some(FiberDiscoveryKind::Excluded(r.report().clone()));
                    result.report.excluded_cells += 1;
                    continue 'cells;
                }
                if let Some(f) = r.into_fiber() {
                    if fibers.len() == limits.max_fibers {
                        failure = Some(("fiber-budget", cell));
                        break 'cells;
                    }
                    let index = fibers.len();
                    fibers.push(f);
                    nodes[id].kind = Some(FiberDiscoveryKind::Fiber { index });
                    continue 'cells;
                }
            }
        }
        if depth == limits.coverage.max_depth || nodes.len() + 2 > limits.coverage.max_cells {
            failure = Some((
                if depth == limits.coverage.max_depth {
                    "depth-budget"
                } else {
                    "cell-budget"
                },
                cell,
            ));
            break;
        }
        let Some((axis, parameter)) = split(cell, domain) else {
            failure = Some(("parameter-resolution", cell));
            break;
        };
        let mut a = cell;
        let mut b = cell;
        a[axis][1] = parameter;
        b[axis][0] = parameter;
        let children = [nodes.len(), nodes.len() + 1];
        nodes.push(FiberDiscoveryNode {
            domain: a,
            kind: None,
        });
        nodes.push(FiberDiscoveryNode {
            domain: b,
            kind: None,
        });
        nodes[id].kind = Some(FiberDiscoveryKind::Split {
            axis,
            parameter,
            children,
        });
        pending.push_back((children[0], depth + 1));
        pending.push_back((children[1], depth + 1));
    }
    result.report.cells = nodes.len();
    result.report.rectangles = work.used;
    if let Some((reason, cell)) = failure {
        result.report.reason = reason;
        result.report.unresolved_box = Some(cell);
    } else {
        result.report.status = FiberCoverageStatus::Complete;
        result.report.reason = "complete-discovered-original-product";
        result.atlas = Some(QualifiedDiscoveredFiberAtlas {
            source: source.clone(),
            domain,
            fibers,
            nodes,
            limits,
        });
    }
    Ok(result)
}
pub fn discover_fiber_surfaces(
    source: &FiberSources,
    limits: FiberDiscoveryLimits,
) -> Result<FiberDiscoveryResult> {
    discover_fiber_box(source, natural(source), limits)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saved_discovery_nodes_cannot_forge_closed_product_coverage() {
        let a = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0.25, 0., -0.25], vec![0.25, 1., 0.75]],
                vec![vec![0.5, 0., -0.25], vec![0.5, 1., 0.75]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let mut b = a.clone();
        for row in &mut b.control_points {
            for p in row {
                p[2] = 0.;
            }
        }
        b.control_points[0][0][0] = 0.;
        b.control_points[0][1][0] = 0.;
        b.control_points[1][0][0] = 1.;
        b.control_points[1][1][0] = 1.;
        let source = FiberSources::new(&a, &b).unwrap();
        let atlas = discover_fiber_surfaces(&source, FiberDiscoveryLimits::default())
            .unwrap()
            .into_atlas()
            .unwrap();
        atlas.reverify().unwrap();
        let mut bad = atlas.clone();
        bad.nodes[0].kind = None;
        assert!(bad.reverify().is_err());
        let mut bad = atlas.clone();
        bad.nodes[0].domain[0][1] = 0.5;
        assert!(bad.reverify().is_err());
        let mut bad = atlas.clone();
        bad.nodes[0].kind = Some(FiberDiscoveryKind::Fiber { index: usize::MAX });
        assert!(bad.reverify().is_err());
        let mut bad = atlas.clone();
        bad.nodes.push(bad.nodes[0].clone());
        assert!(bad.reverify().is_err());
        let mut changed = a;
        changed.control_points[0][0][2] += 0.125;
        let mut bad = atlas;
        bad.source = FiberSources::new(&changed, &b).unwrap();
        assert!(bad.reverify().is_err());
    }
}
