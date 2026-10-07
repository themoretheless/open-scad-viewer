//! Conservative directed endpoint pairing of original clamped curves.
use crate::{
    Result, check,
    curve::Curve,
    distance_bounds::{Interval, box_distance},
};
#[derive(Clone, Debug)]
pub struct Pair {
    pub source: usize,
    pub target: usize,
    pub gap_bounds: [f64; 2],
    pub endpoints_equal: bool,
}
#[derive(Clone, Debug)]
pub struct Report {
    /// Directed cycles of source indices; curves are neither moved nor reversed.
    pub cyclic_chains: Vec<Vec<usize>>,
    pub unresolved_curves: Vec<usize>,
    pub pairs: Vec<Pair>,
    pub pair_tests: usize,
    pub all_chains_closed_within_tolerance: bool,
    pub all_endpoints_equal: bool,
}
/// Retain only unique directed successors, with unique incoming incidence.
/// Ambiguous candidates and distance intervals straddling tolerance are not
/// resolved by nearest-neighbor choice. No embedding/manifold/solid claim.
pub fn assemble(curves: &[Curve], tolerance: f64, max_pair_tests: usize) -> Result<Report> {
    check(
        (1..=256).contains(&curves.len())
            && tolerance.is_finite()
            && tolerance > 0.
            && max_pair_tests <= 65536,
        "Curve chain requires 1..256 curves, positive finite tolerance and at most 65536 pair tests",
    )?;
    let dimension = curves[0].control_points.first().map_or(0, Vec::len);
    for c in curves {
        c.validate()?;
        let [a, b] = c.domain();
        check(
            c.control_points[0].len() == dimension
                && c.knots[..=c.degree].iter().all(|&t| t == a)
                && c.knots[c.knots.len() - c.degree - 1..]
                    .iter()
                    .all(|&t| t == b),
            "Curve chains require consistent dimensions and clamped endpoints",
        )?;
    }
    let n = curves.len();
    if n * n > max_pair_tests {
        return Ok(Report {
            cyclic_chains: vec![],
            unresolved_curves: (0..n).collect(),
            pairs: vec![],
            pair_tests: 0,
            all_chains_closed_within_tolerance: false,
            all_endpoints_equal: false,
        });
    }
    let mut candidates: Vec<Vec<Pair>> = vec![vec![]; n];
    let mut uncertain = vec![false; n];
    let mut incoming = vec![0; n];
    for i in 0..n {
        for j in 0..n {
            let end = curves[i].control_points.last().unwrap();
            let start = &curves[j].control_points[0];
            let equal = end == start;
            let (lo, hi) = if equal {
                (0., 0.)
            } else {
                let a: Vec<_> = end.iter().map(|&x| Interval::point(x)).collect();
                let b: Vec<_> = start.iter().map(|&x| Interval::point(x)).collect();
                box_distance(&a, &b)?
            };
            if hi <= tolerance {
                incoming[j] += 1;
                candidates[i].push(Pair {
                    source: i,
                    target: j,
                    gap_bounds: [lo, hi],
                    endpoints_equal: equal,
                });
            } else if lo <= tolerance {
                incoming[j] += 1;
                uncertain[i] = true;
            }
        }
    }
    let mut next = vec![None; n];
    for i in 0..n {
        if candidates[i].len() == 1 && !uncertain[i] {
            let j = candidates[i][0].target;
            next[i] = Some(j);
        }
    }
    for i in 0..n {
        if next[i].is_some_and(|j| incoming[j] != 1) {
            next[i] = None;
        }
    }
    let mut visited = vec![false; n];
    let mut cycles = Vec::new();
    let mut unresolved = Vec::new();
    for seed in 0..n {
        if visited[seed] {
            continue;
        }
        let mut path = Vec::new();
        let mut current = seed;
        loop {
            if visited[current] {
                if current == seed {
                    cycles.push(path);
                } else {
                    unresolved.extend(path);
                }
                break;
            }
            visited[current] = true;
            path.push(current);
            if let Some(j) = next[current] {
                current = j;
            } else {
                unresolved.extend(path);
                break;
            }
        }
    }
    unresolved.sort_unstable();
    // Only emit pair evidence belonging to a complete directed cycle.
    let pairs: Vec<_> = cycles
        .iter()
        .flatten()
        .map(|&i| candidates[i][0].clone())
        .collect();
    let closed = unresolved.is_empty();
    let equal = closed && pairs.iter().all(|p| p.endpoints_equal);
    Ok(Report {
        cyclic_chains: cycles,
        unresolved_curves: unresolved,
        pairs,
        pair_tests: n * n,
        all_chains_closed_within_tolerance: closed,
        all_endpoints_equal: equal,
    })
}

#[derive(Clone, Debug)]
pub struct Closure {
    pub source: Report,
    /// Present only after all pairings and the global change budget are proved.
    pub curves: Option<Vec<Curve>>,
    /// Continuous maximum deviation of every changed curve from its source.
    pub max_deviation_upper_bound: Option<f64>,
    pub within_tolerance: bool,
}
/// Close confirmed cycles by moving each last control to the successor's
/// original first control. Positive rational bases bound the whole curve
/// displacement by that one control displacement; weights/bases stay fixed.
/// No surface agreement, embedding, orientation or shell claim is made.
pub fn close(
    curves: &[Curve],
    join_tolerance: f64,
    max_change: f64,
    max_pair_tests: usize,
) -> Result<Closure> {
    check(
        max_change.is_finite() && max_change >= 0.,
        "Curve closure requires a finite nonnegative change budget",
    )?;
    let source = assemble(curves, join_tolerance, max_pair_tests)?;
    if !source.all_chains_closed_within_tolerance {
        return Ok(Closure {
            source,
            curves: None,
            max_deviation_upper_bound: None,
            within_tolerance: false,
        });
    }
    let bound = source
        .pairs
        .iter()
        .map(|p| p.gap_bounds[1])
        .fold(0_f64, f64::max);
    if bound > max_change {
        return Ok(Closure {
            source,
            curves: None,
            max_deviation_upper_bound: Some(bound),
            within_tolerance: false,
        });
    }
    let mut closed = curves.to_vec();
    for pair in &source.pairs {
        *closed[pair.source].control_points.last_mut().unwrap() =
            curves[pair.target].control_points[0].clone();
    }
    for c in &closed {
        c.validate()?;
    }
    // Equality follows directly from untouched source starts and assigned ends.
    check(
        source.pairs.iter().all(|p| {
            closed[p.source].control_points.last() == closed[p.target].control_points.first()
        }),
        "Curve closure failed to preserve exact endpoint equality",
    )?;
    Ok(Closure {
        source,
        curves: Some(closed),
        max_deviation_upper_bound: Some(bound),
        within_tolerance: true,
    })
}
