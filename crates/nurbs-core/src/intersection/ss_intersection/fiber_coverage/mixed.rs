//! Closed original-product coverage by overlapping fibers with different free
//! axes. Parameter-root coverage does not identify physical branches or trims.
use super::*;
#[derive(Clone, Debug, PartialEq)]
pub enum MixedCoverageKind {
    Fiber {
        index: usize,
    },
    /// A fresh original Krawczyk image encloses EVERY root of this closed cell
    /// inside the named chart. This does not assert that the cell is nonempty.
    RootImage {
        index: usize,
        attempt: FiberReport,
    },
    Excluded(FiberReport),
    Split {
        axis: usize,
        parameter: f64,
        children: [usize; 2],
        attempt: FiberReport,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub struct MixedCoverageNode {
    domain: Box4,
    probes: Vec<(FiberAxis, FiberReport)>,
    kind: Option<MixedCoverageKind>,
}
impl MixedCoverageNode {
    pub fn probes(&self) -> &[(FiberAxis, FiberReport)] {
        &self.probes
    }
    pub fn domain(&self) -> Box4 {
        self.domain
    }
    pub fn kind(&self) -> Option<&MixedCoverageKind> {
        self.kind.as_ref()
    }
}
pub struct MixedFiberCoverageResult {
    report: FiberCoverageReport,
    coverage: Option<QualifiedMixedFiberCoverage>,
}
impl MixedFiberCoverageResult {
    pub fn report(&self) -> &FiberCoverageReport {
        &self.report
    }
    pub fn coverage(&self) -> Option<&QualifiedMixedFiberCoverage> {
        self.coverage.as_ref()
    }
    pub fn into_coverage(self) -> Option<QualifiedMixedFiberCoverage> {
        self.coverage
    }
}
#[derive(Clone, Debug)]
pub struct QualifiedMixedFiberCoverage {
    fibers: Vec<QualifiedFiber>,
    domain: Box4,
    nodes: Vec<MixedCoverageNode>,
    limits: CoverageLimits,
    report: FiberCoverageReport,
}
impl QualifiedMixedFiberCoverage {
    pub fn fibers(&self) -> &[QualifiedFiber] {
        &self.fibers
    }
    pub fn domain(&self) -> Box4 {
        self.domain
    }
    pub fn nodes(&self) -> &[MixedCoverageNode] {
        &self.nodes
    }
    pub fn report(&self) -> &FiberCoverageReport {
        &self.report
    }
    pub fn covers_entire_surfaces(&self) -> bool {
        self.domain == natural(self.fibers[0].source())
    }
    /// Replay every actual source fiber and every closed cell in the stored
    /// product partition. Neither overlapping charts nor shared cell faces
    /// are discarded. No branch-bijection or transition claim is implied.
    pub fn reverify(&self) -> Result<()> {
        let mut work = Work {
            used: 0,
            max: self.limits.max_rectangles,
        };
        self.replay(&mut work)
    }
    // Shared caller work is charged even when independent replay refuses.
    pub(super) fn replay(&self, work: &mut Work) -> Result<()> {
        check(
            work.used <= work.max,
            "Invalid shared original coverage work ledger",
        )?;
        let mut local = Work {
            used: 0,
            max: self.limits.max_rectangles.min(work.max - work.used),
        };
        let result = self.verify_with(&mut local);
        work.used += local.used;
        result
    }
    fn verify_with(&self, work: &mut Work) -> Result<()> {
        self.limits.validate()?;
        validate(&self.fibers, self.domain)?;
        check(
            !self.nodes.is_empty()
                && self.nodes.len() <= self.limits.max_cells
                && self.nodes[0].domain == self.domain,
            "Mixed coverage root census changed",
        )?;
        for f in &self.fibers {
            replay_fiber(f, work)?;
        }
        let mut seen = vec![false; self.nodes.len()];
        let mut pending = vec![(0, 0)];
        let mut excluded = 0;
        let mut covered = 0;
        while let Some((id, depth)) = pending.pop() {
            check(
                id < self.nodes.len() && !seen[id] && depth <= self.limits.max_depth,
                "Mixed coverage tree has omitted, cyclic or excessive nodes",
            )?;
            seen[id] = true;
            let node = &self.nodes[id];
            check(
                node.probes.len() <= 4,
                "Too many original mixed-axis queries",
            )?;
            let mut axes = [false; 4];
            for (axis, report) in &node.probes {
                check(!axes[axis.index()], "Duplicate mixed-axis original query")?;
                axes[axis.index()] = true;
                let fresh = probe(self.fibers[0].source(), node.domain, *axis, work)?;
                check(
                    fresh.as_ref() == Some(report),
                    "Mixed-axis original source query changed",
                )?;
            }
            let last = node.probes.last();
            match node
                .kind
                .as_ref()
                .ok_or_else(|| crate::input("Unproven mixed coverage leaf"))?
            {
                MixedCoverageKind::Fiber { index } => {
                    check(
                        *index < self.fibers.len()
                            && node.probes.is_empty()
                            && contains(self.fibers[*index].domain(), node.domain),
                        "Mixed coverage cell is outside its actual root fiber",
                    )?;
                    covered += 1;
                }
                MixedCoverageKind::RootImage { index, attempt } => {
                    check(
                        *index < self.fibers.len()
                            && last.is_some_and(|(axis, r)| {
                                r == attempt
                                    && contains_root_image(
                                        node.domain,
                                        r,
                                        axis.index(),
                                        self.fibers[*index].domain(),
                                    )
                            }),
                        "Mixed original root-image containment receipt changed",
                    )?;
                    covered += 1;
                }
                MixedCoverageKind::Excluded(report) => {
                    check(
                        last.is_some_and(|(_, r)| {
                            r == report && report.status == FiberStatus::Excluded
                        }),
                        "Mixed original exclusion receipt changed",
                    )?;
                    excluded += 1;
                }
                MixedCoverageKind::Split {
                    axis,
                    parameter,
                    children,
                    attempt,
                } => {
                    check(
                        last.is_some_and(|(_, r)| r == attempt)
                            && *axis < 4
                            && node.domain[*axis][0] < *parameter
                            && *parameter < node.domain[*axis][1]
                            && children[0] != children[1]
                            && children.iter().all(|&i| i < self.nodes.len()),
                        "Mixed coverage split/original attempt changed",
                    )?;
                    let mut a = node.domain;
                    let mut b = node.domain;
                    a[*axis][1] = *parameter;
                    b[*axis][0] = *parameter;
                    check(
                        self.nodes[children[0]].domain == a && self.nodes[children[1]].domain == b,
                        "Mixed coverage split omits a closed original product face",
                    )?;
                    pending.push((children[1], depth + 1));
                    pending.push((children[0], depth + 1));
                }
            }
        }
        check(
            seen.iter().all(|&s| s)
                && covered > 0
                && self.report.status == FiberCoverageStatus::Complete
                && self.report.reason == "complete-original-mixed-fiber-cover"
                && self.report.cells == self.nodes.len()
                && self.report.excluded_cells == excluded
                && self.report.rectangles == work.used
                && self.report.requested_entire_surfaces == self.covers_entire_surfaces()
                && self.report.unresolved_box.is_none(),
            "Mixed coverage report/census changed",
        )
    }
}
fn probe(
    source: &FiberSources,
    domain: Box4,
    axis: FiberAxis,
    work: &mut Work,
) -> Result<Option<FiberReport>> {
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
    work.used += r.report().rectangles;
    Ok(Some(r.report().clone()))
}
fn contains(outer: Box4, inner: Box4) -> bool {
    (0..4).all(|a| outer[a][0] <= inner[a][0] && inner[a][1] <= outer[a][1])
}
fn overlaps(a: Box4, b: Box4) -> bool {
    (0..4).all(|k| a[k][0] <= b[k][1] && b[k][0] <= a[k][1])
}
fn contains_root_image(cell: Box4, report: &FiberReport, free: usize, chart: Box4) -> bool {
    if report.status == FiberStatus::Excluded {
        return false;
    }
    let Some(image) = report.root_image else {
        return false;
    };
    let mut roots = cell;
    let mut index = 0;
    for k in 0..4 {
        if k != free {
            roots[k] = [
                cell[k][0].max(image[index][0]),
                cell[k][1].min(image[index][1]),
            ];
            index += 1;
        }
    }
    roots
        .iter()
        .all(|d| d.iter().all(|v| v.is_finite()) && d[0] <= d[1])
        && contains(chart, roots)
}
fn validate(fibers: &[QualifiedFiber], domain: Box4) -> Result<()> {
    check(
        (1..=64).contains(&fibers.len()),
        "Mixed coverage needs one to64 actual fibers",
    )?;
    let first = &fibers[0];
    let all = natural(first.source());
    check(
        (0..4).all(|a| domain[a].iter().all(|v| v.is_finite()) && domain[a][0] < domain[a][1])
            && contains(all, domain),
        "Invalid mixed coverage original product domain",
    )?;
    for f in fibers {
        check(
            f.source().first() == first.source().first()
                && f.source().second() == first.source().second()
                && contains(domain, f.domain()),
            "Mixed fibers must share actual ordered original surfaces and lie in the requested product",
        )?;
    }
    Ok(())
}
fn split_at(cell: Box4, domain: Box4, fibers: &[QualifiedFiber]) -> Option<(usize, f64)> {
    // Align with actual fiber boundaries, but balance the partition instead
    // of peeling boundaries in input order. A dense curved atlas otherwise
    // exhausts depth along a long chain of tiny slabs before it can cover the
    // roots. This chooses proposals only; every closed child still needs its
    // own unchanged original exclusion or full chart-containment proof.
    let mut best: Option<(f64, usize, f64)> = None;
    for f in fibers.iter().filter(|f| overlaps(cell, f.domain())) {
        let tube = f.domain();
        for a in 0..4 {
            for t in tube[a] {
                if cell[a][0] < t && t < cell[a][1] {
                    let score =
                        (t - cell[a][0]).min(cell[a][1] - t) / (domain[a][1] - domain[a][0]);
                    if best.is_none_or(|(old, axis, parameter)| {
                        score > old || (score == old && (a < axis || (a == axis && t < parameter)))
                    }) {
                        best = Some((score, a, t));
                    }
                }
            }
        }
    }
    if let Some((_, a, t)) = best {
        return Some((a, t));
    }
    let a = (0..4).max_by(|&a, &b| {
        let width = |k: usize| (cell[k][1] - cell[k][0]) / (domain[k][1] - domain[k][0]);
        width(a).total_cmp(&width(b)).then_with(|| b.cmp(&a))
    })?;
    let t = 0.5 * cell[a][0] + 0.5 * cell[a][1];
    (cell[a][0] < t && t < cell[a][1]).then_some((a, t))
}
/// Cover a complete closed original parameter product by freshly verified
/// uniform root fibers and complete original exclusions. Fibers may overlap
/// and use different free axes/intervals. Missing roots, uncovered chart ends,
/// exhausted work and unresolved boundaries never become complete coverage.
/// This is a root-set cover, not transition ownership or a physical trim graph.
pub fn cover_mixed_fiber_box(
    fibers: &[QualifiedFiber],
    domain: Box4,
    limits: CoverageLimits,
) -> Result<MixedFiberCoverageResult> {
    limits.validate()?;
    validate(fibers, domain)?;
    let mut result = MixedFiberCoverageResult {
        coverage: None,
        report: FiberCoverageReport {
            status: FiberCoverageStatus::Unresolved,
            reason: "fiber-replay-unproven",
            cells: 0,
            excluded_cells: 0,
            rectangles: 0,
            requested_entire_surfaces: domain == natural(fibers[0].source()),
            unresolved_box: None,
        },
    };
    let mut work = Work {
        used: 0,
        max: limits.max_rectangles,
    };
    if fibers.iter().any(|f| replay_fiber(f, &mut work).is_err()) {
        result.report.rectangles = work.used;
        return Ok(result);
    }
    let mut nodes = vec![MixedCoverageNode {
        domain,
        kind: None,
        probes: vec![],
    }];
    let mut pending = vec![(0, 0)];
    let mut excluded = 0;
    let mut covered = 0;
    'cells: while let Some((id, depth)) = pending.pop() {
        let cell = nodes[id].domain;
        if let Some(index) = fibers.iter().position(|f| contains(f.domain(), cell)) {
            nodes[id].kind = Some(MixedCoverageKind::Fiber { index });
            covered += 1;
            continue;
        }
        let mut axes = [false; 4];
        for axis in [
            fibers[0].free_axis(),
            FiberAxis::FirstU,
            FiberAxis::FirstV,
            FiberAxis::SecondU,
            FiberAxis::SecondV,
        ] {
            if axes[axis.index()] {
                continue;
            }
            axes[axis.index()] = true;
            let Some(r) = probe(fibers[0].source(), cell, axis, &mut work)? else {
                result.report.reason = "rectangle-budget";
                result.report.unresolved_box = Some(cell);
                break 'cells;
            };
            nodes[id].probes.push((axis, r.clone()));
            if r.status == FiberStatus::Excluded {
                nodes[id].kind = Some(MixedCoverageKind::Excluded(r));
                excluded += 1;
                continue 'cells;
            }
            if let Some(index) = fibers
                .iter()
                .position(|f| contains_root_image(cell, &r, axis.index(), f.domain()))
            {
                nodes[id].kind = Some(MixedCoverageKind::RootImage { index, attempt: r });
                covered += 1;
                continue 'cells;
            }
        }
        let attempt = nodes[id].probes.last().unwrap().1.clone();
        let split = split_at(cell, domain, fibers);
        if depth == limits.max_depth || nodes.len() + 2 > limits.max_cells || split.is_none() {
            result.report.reason = if depth == limits.max_depth {
                "depth-limit"
            } else if split.is_none() {
                "subdivision-unrepresentable"
            } else {
                "cell-budget"
            };
            result.report.unresolved_box = Some(cell);
            break;
        }
        let (axis, parameter) = split.unwrap();
        let mut a = cell;
        let mut b = cell;
        a[axis][1] = parameter;
        b[axis][0] = parameter;
        let children = [nodes.len(), nodes.len() + 1];
        nodes.push(MixedCoverageNode {
            domain: a,
            kind: None,
            probes: vec![],
        });
        nodes.push(MixedCoverageNode {
            domain: b,
            kind: None,
            probes: vec![],
        });
        nodes[id].kind = Some(MixedCoverageKind::Split {
            axis,
            parameter,
            children,
            attempt,
        });
        pending.push((children[1], depth + 1));
        pending.push((children[0], depth + 1));
    }
    result.report.cells = nodes.len();
    result.report.excluded_cells = excluded;
    result.report.rectangles = work.used;
    if pending.is_empty() && nodes.iter().all(|n| n.kind.is_some()) && covered > 0 {
        result.report.status = FiberCoverageStatus::Complete;
        result.report.reason = "complete-original-mixed-fiber-cover";
        result.report.unresolved_box = None;
        result.coverage = Some(QualifiedMixedFiberCoverage {
            fibers: fibers.to_vec(),
            domain,
            nodes,
            limits,
            report: result.report.clone(),
        });
    }
    Ok(result)
}
pub fn cover_mixed_fiber_surfaces(
    fibers: &[QualifiedFiber],
    limits: CoverageLimits,
) -> Result<MixedFiberCoverageResult> {
    check(!fibers.is_empty(), "Missing mixed root fiber sources")?;
    cover_mixed_fiber_box(fibers, natural(fibers[0].source()), limits)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn closed_partition_and_cached_census_damage_cannot_replay() {
        let a = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0.25, 0., 0.], vec![0.25, 1., 0.]],
                vec![vec![0.5, 0., 0.], vec![0.5, 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let b = Surface {
            control_points: vec![
                vec![vec![0., 0.5, -1.], vec![0., 0.5, 1.]],
                vec![vec![1., 0.5, -1.], vec![1., 0.5, 1.]],
            ],
            ..a.clone()
        };
        let sources = FiberSources::new(&a, &b).unwrap();
        let fiber = sources
            .certify(
                [[0., 1.], [0.25, 0.75], [0.125, 0.625], [0.25, 0.75]],
                FiberAxis::FirstU,
                FiberLimits::default(),
            )
            .unwrap()
            .into_fiber()
            .unwrap();
        let owner = cover_mixed_fiber_surfaces(&[fiber], CoverageLimits::default())
            .unwrap()
            .into_coverage()
            .unwrap();
        owner.reverify().unwrap();
        // The whole product is wider than the chart, but EVERY original root
        // is enclosed inside it. No sampled point or candidate lift proves this.
        assert_eq!(owner.nodes.len(), 1);
        assert!(!contains(owner.fibers[0].domain(), owner.domain));
        assert!(matches!(
            owner.nodes[0].kind,
            Some(MixedCoverageKind::RootImage { .. })
        ));
        let mut bad = owner.clone();
        bad.nodes[0].probes[0].0 = FiberAxis::FirstV;
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        let duplicate = bad.nodes[0].probes[0].clone();
        bad.nodes[0].probes.push(duplicate);
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.report.rectangles -= 1;
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.report.excluded_cells += 1;
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.nodes.pop();
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.nodes[0].domain[0][0] = 0.125;
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        let n = bad
            .nodes
            .iter_mut()
            .find(|n| matches!(n.kind, Some(MixedCoverageKind::RootImage { .. })))
            .unwrap();
        if let Some(MixedCoverageKind::RootImage { index, .. }) = &mut n.kind {
            *index = 999;
        }
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        if let Some(MixedCoverageKind::RootImage { attempt, .. }) = &mut bad.nodes[0].kind {
            attempt.root_image.as_mut().unwrap()[0] = [0., 0.1];
        }
        assert!(bad.reverify().is_err());
        let mut bad = owner;
        let attempt = match bad.nodes[0].kind.as_ref().unwrap() {
            MixedCoverageKind::RootImage { attempt, .. } => attempt.clone(),
            _ => unreachable!(),
        };
        bad.nodes[0].kind = Some(MixedCoverageKind::Split {
            axis: 0,
            parameter: 0.5,
            children: [0, 0],
            attempt,
        });
        assert!(bad.reverify().is_err());
    }
}
