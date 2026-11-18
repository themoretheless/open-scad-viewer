//! Numerical path discovery followed by independent complete-domain replay.
use super::{Limits, PathRequest, Report, certify, contains, domain};
use crate::{
    Result, check, offset_contact_path as path, offset_contact_predictor as predictor,
    surface::Surface,
    surface_contact::Verdict,
    surface_offset::{self, ContactBand, PairDomain},
};
use std::collections::VecDeque;

#[derive(Clone, Copy)]
pub struct DiscoveryLimits {
    /// Complete-domain possible-box search, independent of final qualification.
    pub candidate_boxes: usize,
    pub parameter_tolerance: f64,
    pub predictor_queries: usize,
    pub max_paths: usize,
    pub path: path::Limits,
    /// Path cells are shared by proposal validation AND final fresh replay.
    pub qualification: Limits,
}
pub struct DiscoveryReport {
    pub requests: Vec<PathRequest>,
    pub search: Option<surface_offset::Candidates>,
    pub coverage: Report,
    pub predictor_queries: usize,
    pub path_attempts: usize,
    pub proposal_path_cells: usize,
    pub preflight_boxes: usize,
    /// Numerical failures are diagnostics, never exclusion or absence evidence.
    pub numerical_failures: Vec<String>,
}
impl DiscoveryReport {
    pub fn path_cells(&self) -> usize {
        self.proposal_path_cells + self.coverage.path_cells
    }
}
fn tube(axis: usize, cell: &path::Cell) -> PairDomain {
    let mut first = [[0.; 2]; 2];
    first[axis] = cell.drive;
    first[1 - axis] = cell.first_other;
    [first, cell.second]
}
/// Discover numerical requests without caller seeds or positive receipts.
/// Possible boxes propose fixed-axis roots; full-natural and then local-drive
/// requests require fresh continuous path proofs. Spatially opposite seed boxes
/// alternate, so one component cannot monopolize a small query budget by order.
/// The final full-natural coverage walk independently replays selected requests;
/// discovery failure/limits can never authorize absence or completeness.
///
/// Candidate-search boxes and final-qualification boxes have explicit separate
/// limits, plus one fixed preflight box. Half the shared path-cell allowance is
/// reserved for final replay. This does not count connected components, infer
/// UV trims, extend supports or authorize BRep topology. Unsupported boundary,
/// tangent and chart-switch regions remain explicit in coverage.pending.
pub fn discover(
    surfaces: [&Surface; 2],
    distances: [f64; 2],
    limits: DiscoveryLimits,
) -> Result<DiscoveryReport> {
    check(
        (1..=100000).contains(&limits.candidate_boxes)
            && limits.parameter_tolerance.is_finite()
            && limits.parameter_tolerance > 0.
            && limits.parameter_tolerance <= 1.
            && (1..=1024).contains(&limits.predictor_queries)
            && (1..=32).contains(&limits.max_paths)
            && limits.qualification.path_cells >= 2 * limits.max_paths
            && (1..=100000).contains(&limits.path.cells)
            && (1..=100000).contains(&limits.path.spans)
            && (1..=32).contains(&limits.path.iterations)
            && limits.path.numerical_tolerance_mm.is_finite()
            && limits.path.numerical_tolerance_mm > 0.
            && limits.path.padding_fraction.is_finite()
            && limits.path.padding_fraction > 0.
            && limits.path.padding_fraction <= 0.25,
        "Choose bounded discovery, predictor, path and replay work",
    )?;
    // Reuse the authority owner's validation for both literal source charts and
    // all qualification limits BEFORE an early separation can hide bad input.
    let preflight = certify(
        surfaces,
        distances,
        &[],
        Limits {
            boxes: 1,
            ..limits.qualification
        },
    )?;
    check(
        (1..=100000).contains(&limits.qualification.boxes),
        "Invalid final coverage box allowance",
    )?;
    let mut out = DiscoveryReport {
        requests: vec![],
        search: None,
        coverage: preflight,
        predictor_queries: 0,
        path_attempts: 0,
        proposal_path_cells: 0,
        preflight_boxes: 1,
        numerical_failures: vec![],
    };
    if out.coverage.root_coverage_proven {
        return Ok(out);
    }
    let natural = surfaces.map(domain);
    let search = surface_offset::intersection_candidates(
        surfaces,
        natural,
        distances,
        limits.parameter_tolerance,
        limits.candidate_boxes,
        limits.qualification.spans_per_bound,
    )?;
    let mut seeds: VecDeque<_> = search
        .boxes
        .iter()
        .chain(&search.pending)
        .copied()
        .collect();
    let mut tubes = Vec::<PairDomain>::new();
    let proposal_quota = limits.qualification.path_cells / 2;
    let mut considered = 0usize;
    'boxes: while !seeds.is_empty()
        && out.predictor_queries < limits.predictor_queries
        && out.requests.len() < limits.max_paths
        && out.proposal_path_cells < proposal_quota
    {
        let cell = if considered % 2 == 0 {
            seeds.pop_front()
        } else {
            seeds.pop_back()
        }
        .unwrap();
        considered += 1;
        if tubes.iter().any(|t| contains(*t, cell)) {
            continue;
        }
        let seed = cell.map(|d| d.map(|r| r[0] * 0.5 + r[1] * 0.5));
        for axis in 0..2 {
            let mut qualified = None;
            let mut padding = limits.path.padding_fraction;
            // A wide proposal box can be inconclusive at a perfectly regular
            // nonlinear root. Shrinking is ONLY a fresh proposal: no old
            // certificate is inherited. Every retry consumes the shared query
            // allowance; numerical failure/exclusion never prunes the source.
            for _ in 0..=8 {
                if out.predictor_queries == limits.predictor_queries {
                    break 'boxes;
                }
                out.predictor_queries += 1;
                let root = match predictor::propose(
                    crate::offset::contact_pcurve::OffsetFaces {
                        surfaces,
                        distances,
                    },
                    seed,
                    axis,
                    limits.path.numerical_tolerance_mm,
                    limits.path.iterations,
                    padding,
                    limits.path.spans,
                ) {
                    Ok(r) => r,
                    Err(e) => {
                        out.numerical_failures.push(e.to_string());
                        break;
                    }
                };
                if root.residual_mm > limits.path.numerical_tolerance_mm {
                    break;
                }
                if matches!(root.certificate, Some(Verdict::Witness(_))) {
                    qualified = Some(root);
                    break;
                }
                if !matches!(root.certificate, Some(Verdict::Unresolved)) {
                    break;
                }
                padding *= 0.5;
            }
            let Some(root) = qualified else {
                continue;
            };
            let point = [
                root.first_uv.map(|x| [x, x]),
                root.second_uv.map(|x| [x, x]),
            ];
            // Skipping a numerical duplicate changes only proposals. Final
            // coverage never uses this point test as an ownership receipt.
            if tubes.iter().any(|t| contains(*t, point)) {
                break;
            }
            let domains = [natural[0][axis], cell[0][axis]];
            for (attempt, drive) in domains.into_iter().enumerate() {
                if attempt == 1 && drive == natural[0][axis] {
                    continue;
                }
                if out.proposal_path_cells == proposal_quota {
                    break 'boxes;
                }
                let mut work = limits.path;
                work.padding_fraction = padding;
                work.cells = work.cells.min(proposal_quota - out.proposal_path_cells);
                let mut endpoints = [[root.first_uv, root.second_uv]; 2];
                for end in 0..2 {
                    endpoints[end][0][axis] = drive[end];
                }
                out.path_attempts += 1;
                // Numerical path errors cannot prune a region. Charge the
                // entire attempted cell cap when no work report is returned.
                let report = match path::certify(surfaces, distances, axis, drive, endpoints, work)
                {
                    Ok(r) => {
                        out.proposal_path_cells += r.visited;
                        r
                    }
                    Err(e) => {
                        out.proposal_path_cells += work.cells;
                        out.numerical_failures.push(e.to_string());
                        continue;
                    }
                };
                if !report.continuous_path_proven {
                    continue;
                }
                for c in &report.cells {
                    if matches!(c.verdict, ContactBand::ContinuousBranch(_)) {
                        tubes.push(tube(axis, c));
                    }
                }
                out.requests.push(PathRequest {
                    axis,
                    drive,
                    seeds: endpoints,
                    limits: work,
                });
                break;
            }
            if out.requests.len() == limits.max_paths {
                break 'boxes;
            }
        }
    }
    out.search = Some(search);
    out.coverage = certify(
        surfaces,
        distances,
        &out.requests,
        Limits {
            path_cells: limits.qualification.path_cells - out.proposal_path_cells,
            ..limits.qualification
        },
    )?;
    Ok(out)
}
