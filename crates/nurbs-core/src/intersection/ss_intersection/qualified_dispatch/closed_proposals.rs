//! Numerical proposals only. Original-source operators and the complete mixed
//! census are the authority; Newton convergence never proves a root or absence.
use super::*;
const AXES: [FiberAxis; 4] = [
    FiberAxis::FirstU,
    FiberAxis::FirstV,
    FiberAxis::SecondU,
    FiberAxis::SecondV,
];
fn det(a: [[f64; 3]; 3]) -> f64 {
    a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
        - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
        + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0])
}
fn solve(a: [[f64; 3]; 3], rhs: [f64; 3]) -> Option<[f64; 3]> {
    let d = det(a);
    if !d.is_finite() || d == 0. {
        return None;
    }
    let mut out = [0.; 3];
    for k in 0..3 {
        let mut b = a;
        for i in 0..3 {
            b[i][k] = rhs[i];
        }
        out[k] = det(b) / d;
    }
    out.iter().all(|x| x.is_finite()).then_some(out)
}
fn system(
    source: &FiberSources,
    domain: [[f64; 2]; 4],
    x: [f64; 4],
) -> Option<([f64; 3], [[f64; 4]; 3])> {
    let p: [f64; 4] = std::array::from_fn(|k| domain[k][0] + x[k] * (domain[k][1] - domain[k][0]));
    let a = source.first().evaluate(p[0], p[1]).ok()?;
    let b = source.second().evaluate(p[2], p[3]).ok()?;
    let (au, av) = a.first_derivatives()?;
    let (bu, bv) = b.first_derivatives()?;
    let columns = [au, av, bu.map(|v| -v), bv.map(|v| -v)];
    Some((
        std::array::from_fn(|k| a.point[k] - b.point[k]),
        std::array::from_fn(|k| {
            std::array::from_fn(|j| columns[j][k] * (domain[j][1] - domain[j][0]))
        }),
    ))
}
fn polish(
    source: &FiberSources,
    domain: [[f64; 2]; 4],
    mut x: [f64; 4],
    free: usize,
) -> Option<[f64; 4]> {
    let unknown: Vec<_> = (0..4).filter(|&k| k != free).collect();
    for _ in 0..20 {
        let (r, j) = system(source, domain, x)?;
        let m = std::array::from_fn(|i| std::array::from_fn(|k| j[i][unknown[k]]));
        let step = solve(m, r)?;
        let mut scale = 1.;
        // Keep proposals in the natural parameter product. This line search is
        // not a root existence argument.
        while scale >= 1. / 1024.
            && unknown
                .iter()
                .enumerate()
                .any(|(i, &k)| !(0. ..=1.).contains(&(x[k] - scale * step[i])))
        {
            scale *= 0.5;
        }
        if scale < 1. / 1024. {
            return None;
        }
        for (i, &k) in unknown.iter().enumerate() {
            x[k] -= scale * step[i];
        }
        if step.iter().all(|v| v.abs() * scale < 1e-12) {
            return Some(x);
        }
    }
    None
}
fn proposal(
    source: &FiberSources,
    domain: [[f64; 2]; 4],
    x: [f64; 4],
    free: usize,
    h: f64,
) -> [[f64; 2]; 4] {
    let slope = system(source, domain, x).and_then(|(_, j)| {
        let cols: Vec<_> = (0..4).filter(|&k| k != free).collect();
        solve(
            std::array::from_fn(|i| std::array::from_fn(|k| j[i][cols[k]])),
            std::array::from_fn(|i| -j[i][free]),
        )
        .map(|r| {
            let mut d = [1.; 4];
            for (i, &k) in cols.iter().enumerate() {
                d[k] = r[i];
            }
            d
        })
    });
    std::array::from_fn(|k| {
        // Near a flat turning point, a small unknown interval can support a
        // wide free interval. Isotropic boxes unnecessarily shrink BOTH.
        // The numerical tangent chooses a box only; original operators prove it.
        let radius = if k == free {
            h
        } else {
            slope.map_or(2. * h, |d| 1.25 * h * d[k].abs() + 8. * h * h)
        };
        [
            domain[k][0] + (x[k] - radius).max(0.) * (domain[k][1] - domain[k][0]),
            domain[k][0] + (x[k] + radius).min(1.) * (domain[k][1] - domain[k][0]),
        ]
    })
}
pub(super) fn construct(
    source: &FiberSources,
    limits: ClosedSSBuildLimits,
) -> crate::Result<(Vec<QualifiedFiber>, usize)> {
    crate::check(
        (2..=32).contains(&limits.seed_steps)
            && (1..=4096).contains(&limits.max_seeds)
            && (1..=100000).contains(&limits.proposal_rectangles),
        "Need bounded closed intersection proposals",
    )?;
    let a = surface_domain(source.first());
    let b = surface_domain(source.second());
    let domain = [a[0], a[1], b[0], b[1]];
    let mut charts: Vec<QualifiedFiber> = vec![];
    let mut used = 0;
    let mut seeds = 0;
    let mut tried = Vec::<[f64; 4]>::new();
    let mut stations = (1..limits.seed_steps).collect::<Vec<_>>();
    stations.sort_by_key(|&k| (k.abs_diff(limits.seed_steps / 2), k));
    for station in stations {
        for free in 0..4 {
            let unknown: Vec<_> = (0..4).filter(|&k| k != free).collect();
            for seed in 0..27 {
                if seeds == limits.max_seeds || used == limits.proposal_rectangles {
                    return Ok((charts, used));
                }
                seeds += 1;
                let mut x = [0.5; 4];
                x[free] = station as f64 / limits.seed_steps as f64;
                let mut n = seed;
                for &k in &unknown {
                    x[k] = (1 + n % 3) as f64 / 4.;
                    n /= 3;
                }
                let Some(x) = polish(source, domain, x, free) else {
                    continue;
                };
                // Repeated Newton seeds often propose the SAME ill-conditioned
                // root. Deduplicate proposals only; missed roots still make the
                // independent complete census refuse.
                if tried
                    .iter()
                    .any(|p| (0..4).all(|k| (p[k] - x[k]).abs() < 1e-4))
                {
                    continue;
                }
                tried.push(x);
                let Some((_, jacobian)) = system(source, domain, x) else {
                    continue;
                };
                // The polishing coordinate need not be a stable local clock.
                // Rank proposals only; original operators still verify it.
                let minor = |axis| {
                    let cols: Vec<_> = (0..4).filter(|&k| k != axis).collect();
                    det(std::array::from_fn(|i| {
                        std::array::from_fn(|j| jacobian[i][cols[j]])
                    }))
                    .abs()
                };
                let free = (0..4)
                    .max_by(|&a, &b| minor(a).total_cmp(&minor(b)))
                    .unwrap();
                let p: [f64; 4] =
                    std::array::from_fn(|k| domain[k][0] + x[k] * (domain[k][1] - domain[k][0]));
                // A qualified graph already owns all roots in its box. Skip a
                // numerically interior proposal, but never use it as coverage.
                if charts.iter().any(|f| {
                    (0..4).all(|k| {
                        f.domain()[k][0] + 0.1 * (f.domain()[k][1] - f.domain()[k][0]) < p[k]
                            && p[k] < f.domain()[k][1] - 0.1 * (f.domain()[k][1] - f.domain()[k][0])
                    })
                }) {
                    continue;
                }
                for level in 0..5 {
                    let h = 0.125 / (1usize << level) as f64;
                    let proposal = proposal(source, domain, x, free, h);
                    if used == limits.proposal_rectangles {
                        return Ok((charts, used));
                    }
                    let result = source.certify_contracted(
                        proposal,
                        AXES[free],
                        limits.proposal_rectangles - used,
                    )?;
                    used += result.rectangles();
                    if let Some(f) = result.into_fiber() {
                        charts.push(f);
                        if charts.len() == limits.max_charts {
                            return Ok((charts, used));
                        }
                        break;
                    }
                }
            }
        }
    }
    Ok((charts, used))
}

/// Propose one additional chart at a cell the complete verifier could not
/// resolve. The cell is not assumed nonempty; only fresh operators may admit it.
fn repair_one(
    source: &FiberSources,
    cell: [[f64; 2]; 4],
    remaining: usize,
    preferred: Option<usize>,
) -> crate::Result<(Option<QualifiedFiber>, usize)> {
    let a = surface_domain(source.first());
    let b = surface_domain(source.second());
    let domain = [a[0], a[1], b[0], b[1]];
    let seed = std::array::from_fn(|k| {
        ((cell[k][0] * 0.5 + cell[k][1] * 0.5) - domain[k][0]) / (domain[k][1] - domain[k][0])
    });
    let mut used = 0;
    for axis in preferred.map_or_else(|| vec![0, 1, 2, 3], |axis| vec![axis]) {
        let Some(x) = polish(source, domain, seed, axis) else {
            continue;
        };
        let Some((_, jacobian)) = system(source, domain, x) else {
            continue;
        };
        let minor = |axis| {
            let cols: Vec<_> = (0..4).filter(|&k| k != axis).collect();
            det(std::array::from_fn(|i| {
                std::array::from_fn(|j| jacobian[i][cols[j]])
            }))
            .abs()
        };
        let free = (0..4)
            .max_by(|&a, &b| minor(a).total_cmp(&minor(b)))
            .unwrap();
        for level in 0..5 {
            if used == remaining {
                return Ok((None, used));
            }
            let h = 0.0625 / (1usize << level) as f64;
            let proposal = proposal(source, domain, x, free, h);
            let attempt = source.certify_contracted(proposal, AXES[free], remaining - used)?;
            used += attempt.rectangles();
            if let Some(f) = attempt.into_fiber() {
                return Ok((Some(f), used));
            }
        }
    }
    Ok((None, used))
}

/// Propose overlapping neighbors in one repair round. All proposals share the
/// original remaining ledger; NONE is coverage authority until the fresh census.
pub(super) fn repair(
    source: &FiberSources,
    cell: [[f64; 2]; 4],
    remaining: usize,
    max_charts: usize,
) -> crate::Result<(Vec<QualifiedFiber>, usize)> {
    let (first, mut used) = repair_one(source, cell, remaining, None)?;
    let Some(first) = first else {
        return Ok((vec![], used));
    };
    let axis = first.free_axis().index();
    let domain = first.domain();
    let mut charts = vec![first];
    for fraction in [0.125, 0.875] {
        if used == remaining || charts.len() == max_charts {
            break;
        }
        let mut neighbor = domain;
        let center = domain[axis][0] + fraction * (domain[axis][1] - domain[axis][0]);
        // A singleton proposal coordinate is allowed here; repair_one uses
        // only its center as a Newton seed and proves a fresh positive box.
        neighbor[axis] = [center, center];
        let (extra, work) = repair_one(source, neighbor, remaining - used, Some(axis))?;
        used += work;
        if let Some(extra) = extra {
            charts.push(extra);
        }
    }
    Ok((charts, used))
}

/// Discard only a chart whose WHOLE certified parameter root enclosure is
/// already inside another unique-root chart. No spatial distance or sampled
/// branch label participates. The independent full census remains mandatory.
pub(super) fn prune(charts: &mut Vec<QualifiedFiber>) {
    for i in (0..charts.len()).rev() {
        let Some(image) = charts[i].report().root_image else {
            continue;
        };
        let mut roots = charts[i].domain();
        let mut at = 0;
        for k in 0..4 {
            if k != charts[i].free_axis().index() {
                roots[k][0] = roots[k][0].max(image[at][0]);
                roots[k][1] = roots[k][1].min(image[at][1]);
                at += 1;
            }
        }
        if roots.iter().any(|d| d[0] > d[1]) {
            continue;
        }
        if (0..charts.len()).any(|j| {
            j != i
                && (0..4).all(|k| {
                    charts[j].domain()[k][0] <= roots[k][0]
                        && roots[k][1] <= charts[j].domain()[k][1]
                })
        }) {
            charts.remove(i);
        }
    }
}

/// Compact overlap using fresh original operators on FOUR closed free slices.
/// Each slice must belong to another surviving chart. Work is charged to the
/// same proposal ledger, including unsuccessful attempts; no allowance reset.
pub(super) fn compact(
    charts: &mut Vec<QualifiedFiber>,
    used: &mut usize,
    max: usize,
) -> crate::Result<()> {
    for i in (0..charts.len()).rev() {
        if *used == max {
            break;
        }
        let f = &charts[i];
        let axis = f.free_axis().index();
        let [a, b] = f.domain()[axis];
        let cuts = [
            a,
            a + (b - a) * 0.25,
            a + (b - a) * 0.5,
            a + (b - a) * 0.75,
            b,
        ];
        if cuts.windows(2).any(|w| w[0] >= w[1]) {
            continue;
        }
        let mut covered = true;
        for cut in cuts.windows(2) {
            if *used == max {
                covered = false;
                break;
            }
            let mut domain = f.domain();
            domain[axis] = [cut[0], cut[1]];
            let result = f
                .source()
                .certify_contracted(domain, f.free_axis(), max - *used)?;
            *used += result.rectangles();
            let Some(part) = result.into_fiber() else {
                covered = false;
                break;
            };
            let Some(image) = part.report().root_image else {
                covered = false;
                break;
            };
            let mut roots = part.domain();
            let mut at = 0;
            for k in 0..4 {
                if k != axis {
                    roots[k][0] = roots[k][0].max(image[at][0]);
                    roots[k][1] = roots[k][1].min(image[at][1]);
                    at += 1;
                }
            }
            if roots.iter().any(|d| d[0] > d[1])
                || !(0..charts.len()).any(|j| {
                    j != i
                        && (0..4).all(|k| {
                            charts[j].domain()[k][0] <= roots[k][0]
                                && roots[k][1] <= charts[j].domain()[k][1]
                        })
                })
            {
                covered = false;
                break;
            }
        }
        if covered {
            charts.remove(i);
        }
    }
    Ok(())
}
