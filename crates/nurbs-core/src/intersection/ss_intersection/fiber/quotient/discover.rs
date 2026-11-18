//! Generic numerical proposals plus mandatory complete original quotient census.
//! No rotational, extrusion, graph, polynomial-degree or plane recognition.
use super::*;
use crate::ss_intersection::{CoverageLimits, FiberCoverageReport};
mod proposals;
#[derive(Clone, Copy, Debug)]
pub struct QuotientDiscoveryLimits {
    pub seed_steps: usize,
    pub max_seeds: usize,
    pub max_charts: usize,
    pub max_repairs: usize,
    pub proposal_rectangles: usize,
    pub max_evaluations: usize,
    pub coverage: CoverageLimits,
}
impl Default for QuotientDiscoveryLimits {
    fn default() -> Self {
        Self {
            seed_steps: 8,
            max_seeds: 4096,
            max_charts: 128,
            max_repairs: 8,
            proposal_rectangles: 100000,
            max_evaluations: 131072,
            coverage: Default::default(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct QuotientDiscoveryReport {
    pub reason: &'static str,
    pub seeds: usize,
    pub evaluations: usize,
    pub proposal_rectangles: usize,
    pub charts: usize,
    pub coverage_attempts: Vec<FiberCoverageReport>,
}
pub struct QuotientDiscoveryResult {
    report: QuotientDiscoveryReport,
    coverage: Option<QualifiedQuotientCoverage>,
}
impl QuotientDiscoveryResult {
    pub fn report(&self) -> &QuotientDiscoveryReport {
        &self.report
    }
    pub fn coverage(&self) -> Option<&QualifiedQuotientCoverage> {
        self.coverage.as_ref()
    }
    pub fn into_coverage(self) -> Option<QualifiedQuotientCoverage> {
        self.coverage
    }
}
struct Work {
    limits: QuotientDiscoveryLimits,
    report: QuotientDiscoveryReport,
}
const AXES: [FiberAxis; 4] = [
    FiberAxis::FirstU,
    FiberAxis::FirstV,
    FiberAxis::SecondU,
    FiberAxis::SecondV,
];
fn periodic(source: &FiberSources, k: usize) -> bool {
    let s = if k < 2 {
        source.first()
    } else {
        source.second()
    };
    if k % 2 == 0 {
        s.periodic_u
    } else {
        s.periodic_v
    }
}
fn natural(source: &FiberSources) -> [[f64; 2]; 4] {
    [
        axes::natural(source.first(), 0),
        axes::natural(source.first(), 1),
        axes::natural(source.second(), 0),
        axes::natural(source.second(), 1),
    ]
}
fn normalized(source: &FiberSources, p: [f64; 4]) -> [f64; 4] {
    let n = natural(source);
    std::array::from_fn(|k| (p[k] - n[k][0]) / (n[k][1] - n[k][0]))
}
fn interior_seed(source: &FiberSources, k: usize, index: usize) -> f64 {
    let s = if k < 2 {
        source.first()
    } else {
        source.second()
    };
    let (knots, p, n) = if k % 2 == 0 {
        (&s.knots_u, s.degree_u, s.control_points.len())
    } else {
        (&s.knots_v, s.degree_v, s.control_points[0].len())
    };
    let spans = (p..n)
        .filter(|&i| knots[i] < knots[i + 1])
        .collect::<Vec<_>>();
    let at = spans[((2 * index + 1) * spans.len() / 6).min(spans.len() - 1)];
    let point = knots[at] * 0.5 + knots[at + 1] * 0.5;
    (point - knots[p]) / (knots[n] - knots[p])
}
fn try_seed(
    source: &FiberSources,
    mut x: [f64; 4],
    free: usize,
    charts: &mut Vec<QualifiedQuotientFiber>,
    work: &mut Work,
    deduplicate: bool,
) -> Result<()> {
    if work.report.seeds == work.limits.max_seeds
        || charts.len() == work.limits.max_charts
        || work.report.proposal_rectangles == work.limits.proposal_rectangles
    {
        return Ok(());
    }
    work.report.seeds += 1;
    let Some(root) = proposals::polish(source, x, free, work)? else {
        return Ok(());
    };
    x = root;
    let n = natural(source);
    let p = std::array::from_fn(|k| {
        let phase = if periodic(source, k) {
            x[k].rem_euclid(1.)
        } else {
            x[k]
        };
        let t = (n[k][0] + phase * (n[k][1] - n[k][0])).clamp(n[k][0], n[k][1]);
        [t; 2]
    });
    for f in charts.iter() {
        if deduplicate && f.contains_original_box(p)? {
            return Ok(());
        }
    }
    for h in [0.125, 0.0625, 0.03125, 0.015625, 0.0078125] {
        let Some(d) = proposals::chart(source, x, free, h, work)? else {
            return Ok(());
        };
        if work.report.proposal_rectangles == work.limits.proposal_rectangles {
            return Ok(());
        }
        let r = source.certify_quotient(
            d,
            AXES[free],
            FiberLimits {
                max_rectangles: (work.limits.proposal_rectangles - work.report.proposal_rectangles)
                    .min(100000),
                preconditioner: None,
            },
        );
        if let Ok(r) = r {
            work.report.proposal_rectangles += r.report().rectangles;
            if let Some(f) = r.into_fiber() {
                if charts
                    .iter()
                    .any(|old| old.domain() == f.domain() && old.free_axis() == f.free_axis())
                {
                    continue;
                }
                charts.push(f);
                return Ok(());
            }
        }
    }
    Ok(())
}
/// Search generic original carriers, including periodic shores, and publish
/// only after a complete natural-product census. Budgets cover numerical
/// proposals globally. Each complete census attempt has its explicit shared
/// coverage allowance; no partial forest is promoted on exhausted repairs.
pub fn discover_quotient_fiber_surfaces(
    source: &FiberSources,
    limits: QuotientDiscoveryLimits,
) -> Result<QuotientDiscoveryResult> {
    check(
        (2..=32).contains(&limits.seed_steps)
            && (1..=16384).contains(&limits.max_seeds)
            && (1..=256).contains(&limits.max_charts)
            && limits.max_repairs <= 32
            && (1..=100000).contains(&limits.proposal_rectangles)
            && (1..=1000000).contains(&limits.max_evaluations),
        "Invalid generic quotient discovery work",
    )?;
    check(
        (1..=65536).contains(&limits.coverage.max_cells)
            && (1..=1000000).contains(&limits.coverage.max_rectangles)
            && limits.coverage.max_depth <= 64,
        "Invalid quotient census allowance",
    )?;
    let admission = natural(source).map(|d| [d[0] * 0.75 + d[1] * 0.25, d[0] * 0.25 + d[1] * 0.75]);
    axes::validate(source, admission, 0)?;
    let mut work = Work {
        limits,
        report: QuotientDiscoveryReport {
            reason: "no-regular-quotient-chart-proved",
            seeds: 0,
            evaluations: 0,
            proposal_rectangles: 0,
            charts: 0,
            coverage_attempts: Vec::new(),
        },
    };
    let mut charts = Vec::new();
    let mut stations = (0..=limits.seed_steps).collect::<Vec<_>>();
    stations.sort_by_key(|&i| (i.abs_diff(limits.seed_steps / 2), i));
    'search: for station in stations {
        for free in 0..4 {
            if station == limits.seed_steps && periodic(source, free) {
                continue;
            }
            let unknown = (0..4).filter(|&k| k != free).collect::<Vec<_>>();
            for seed in 0..27 {
                if work.report.seeds == limits.max_seeds
                    || work.report.evaluations >= limits.max_evaluations
                    || work.report.proposal_rectangles == limits.proposal_rectangles
                    || charts.len() == limits.max_charts
                {
                    break 'search;
                }
                let mut x = [0.5; 4];
                x[free] = station as f64 / limits.seed_steps as f64;
                let mut n = seed;
                for &k in &unknown {
                    x[k] = interior_seed(source, k, n % 3);
                    n /= 3;
                }
                try_seed(source, x, free, &mut charts, &mut work, true)?;
            }
        }
    }
    for repair in 0..=limits.max_repairs {
        work.report.charts = charts.len();
        if charts.is_empty() {
            break;
        }
        let attempt = cover_quotient_fiber_surfaces(&charts, limits.coverage)?;
        let coverage_report = attempt.report().clone();
        work.report.coverage_attempts.push(coverage_report.clone());
        if let Some(coverage) = attempt.into_coverage() {
            work.report.reason = "complete-original-quotient-root-census";
            return Ok(QuotientDiscoveryResult {
                report: work.report,
                coverage: Some(coverage),
            });
        }
        work.report.reason = "original-quotient-census-remains-unproved";
        if repair == limits.max_repairs {
            break;
        }
        let Some(d) = coverage_report.unresolved_box else {
            break;
        };
        let x = normalized(source, d.map(|d| d[0] * 0.5 + d[1] * 0.5));
        let before = charts.len();
        for free in 0..4 {
            try_seed(source, x, free, &mut charts, &mut work, false)?;
        }
        if charts.len() == before {
            break;
        }
    }
    Ok(QuotientDiscoveryResult {
        report: work.report,
        coverage: None,
    })
}
