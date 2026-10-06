//! Adaptive offset tubes with full driving-interval coverage and certified joins.
//! Numerical stations propose boxes. Only uniform interval contraction and
//! section roots in intersecting neighboring tubes admit a continuous path.
use crate::{check, surface::Surface, surface_contact::Verdict, surface_offset, Result};

#[derive(Clone, Copy)]
pub struct Limits {
    pub cells: usize,
    pub spans: usize,
    pub iterations: usize,
    pub numerical_tolerance_mm: f64,
    pub padding_fraction: f64,
}
pub struct Cell {
    pub drive: [f64; 2],
    pub first_other: [f64; 2],
    pub second: [[f64; 2]; 2],
    pub verdict: surface_offset::ContactBand,
}
pub struct Join {
    pub fixed: f64,
    pub verdict: Verdict,
}
pub struct Report {
    pub cells: Vec<Cell>,
    pub joins: Vec<Join>,
    pub visited: usize,
    pub numerical_queries: usize,
    pub band_queries: usize,
    pub section_queries: usize,
    pub continuous_path_proven: bool,
    pub reason: &'static str,
}
fn domains(s: &Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}
/// `seeds` are numerical endpoint proposals on two original surface charts.
/// `drive` is an explicit first-source coordinate interval. This does not prove
/// correspondence to a world edge, trimmed membership or completeness outside
/// the returned tubes. Exhaustion retains the entire requested interval.
pub fn certify(
    surfaces: [&Surface; 2],
    distances: [f64; 2],
    axis: usize,
    drive: [f64; 2],
    seeds: [[[f64; 2]; 2]; 2],
    limits: Limits,
) -> Result<Report> {
    for s in surfaces {
        s.validate()?;
    }
    check(
        axis < 2 && drive.iter().all(|x| x.is_finite()) && drive[0] < drive[1],
        "Choose a positive finite driving interval",
    )?;
    check(
        (1..=100000).contains(&limits.cells)
            && (1..=100000).contains(&limits.spans)
            && (1..=32).contains(&limits.iterations)
            && limits.numerical_tolerance_mm.is_finite()
            && limits.numerical_tolerance_mm > 0.
            && limits.padding_fraction.is_finite()
            && limits.padding_fraction > 0.
            && limits.padding_fraction <= 0.25,
        "Choose bounded path work, positive numerical tolerance and padding",
    )?;
    check(
        distances.iter().all(|d| d.is_finite() && *d != 0.)
            && distances[0].abs() == distances[1].abs(),
        "Choose equal nonzero signed offset magnitudes",
    )?;
    let natural = surfaces.map(domains);
    check(
        natural
            .iter()
            .flatten()
            .all(|d| (d[1] - d[0]).is_finite() && d[1] > d[0]),
        "Source chart widths must have finite positive numerical scale",
    )?;
    check(
        drive[0] >= natural[0][axis][0] && drive[1] <= natural[0][axis][1],
        "Driving interval must stay inside the original source domain",
    )?;
    for station in seeds {
        for side in 0..2 {
            for k in 0..2 {
                check(
                    station[side][k].is_finite()
                        && station[side][k] >= natural[side][k][0]
                        && station[side][k] <= natural[side][k][1],
                    "Endpoint seed outside original chart",
                )?;
            }
        }
    }
    let mut out = Report {
        cells: vec![],
        joins: vec![],
        visited: 0,
        numerical_queries: 0,
        band_queries: 0,
        section_queries: 0,
        continuous_path_proven: false,
        reason: "path-work-limit",
    };
    let mut queue = vec![(drive, 0usize)];
    while let Some((interval, depth)) = queue.pop() {
        if out.visited == limits.cells {
            out.cells.push(Cell {
                drive: interval,
                first_other: natural[0][1 - axis],
                second: natural[1],
                verdict: surface_offset::ContactBand::Unresolved,
            });
            continue;
        }
        out.visited += 1;
        let mid = interval[0] * 0.5 + interval[1] * 0.5;
        let mut samples = vec![];
        for t in [interval[0], mid, interval[1]] {
            let fraction = (t - drive[0]) / (drive[1] - drive[0]);
            let mut seed = std::array::from_fn(|side| {
                std::array::from_fn(|k| {
                    seeds[0][side][k] * (1. - fraction) + seeds[1][side][k] * fraction
                })
            });
            seed[0][axis] = t;
            out.numerical_queries += 1;
            let r = crate::offset_contact_predictor::propose(
                surfaces,
                distances,
                seed,
                axis,
                limits.numerical_tolerance_mm,
                limits.iterations,
                limits.padding_fraction,
                limits.spans,
            )?;
            if r.certificate.is_some() {
                out.section_queries += 1;
            }
            if r.residual_mm > limits.numerical_tolerance_mm {
                break;
            }
            samples.push([r.first_uv, r.second_uv]);
        }
        let mut first_other = natural[0][1 - axis];
        let mut second = natural[1];
        let mut verdict = surface_offset::ContactBand::Unresolved;
        if samples.len() == 3 {
            let tube = |side: usize, k: usize| {
                let d = natural[side][k];
                let pad = (d[1] - d[0]) * limits.padding_fraction;
                [
                    samples
                        .iter()
                        .map(|p| p[side][k])
                        .fold(f64::INFINITY, f64::min)
                        - pad,
                    samples
                        .iter()
                        .map(|p| p[side][k])
                        .fold(f64::NEG_INFINITY, f64::max)
                        + pad,
                ]
                .map(|x| x.max(d[0]).min(d[1]))
            };
            first_other = tube(0, 1 - axis);
            second = [tube(1, 0), tube(1, 1)];
            if first_other[0] < first_other[1] && second.iter().all(|r| r[0] < r[1]) {
                out.band_queries += 1;
                verdict = surface_offset::certify_contact_band(
                    surfaces,
                    distances,
                    axis,
                    interval,
                    first_other,
                    second,
                    limits.spans,
                )?;
            }
        }
        if matches!(verdict, surface_offset::ContactBand::Unresolved)
            && out.visited < limits.cells
            && depth < 32
            && mid > interval[0]
            && mid < interval[1]
        {
            queue.push(([mid, interval[1]], depth + 1));
            queue.push(([interval[0], mid], depth + 1));
        } else {
            out.cells.push(Cell {
                drive: interval,
                first_other,
                second,
                verdict,
            });
        }
    }
    out.cells.sort_by(|a, b| a.drive[0].total_cmp(&b.drive[0]));
    let all = out
        .cells
        .iter()
        .all(|c| matches!(c.verdict, surface_offset::ContactBand::ContinuousBranch(_)));
    if !all {
        return Ok(out);
    }
    for pair in out.cells.windows(2) {
        let a = &pair[0];
        let b = &pair[1];
        check(a.drive[1] == b.drive[0], "Internal path coverage gap")?;
        let intersect = |a: [f64; 2], b: [f64; 2]| [a[0].max(b[0]), a[1].min(b[1])];
        let free = intersect(a.first_other, b.first_other);
        let second = std::array::from_fn(|k| intersect(a.second[k], b.second[k]));
        let verdict = if free[0] < free[1] && second.iter().all(|r| r[0] < r[1]) {
            out.section_queries += 1;
            surface_offset::certify_contact_section(
                surfaces,
                distances,
                axis,
                a.drive[1],
                free,
                second,
                limits.spans,
            )?
        } else {
            Verdict::Unresolved
        };
        out.joins.push(Join {
            fixed: a.drive[1],
            verdict,
        });
    }
    out.continuous_path_proven = out
        .joins
        .iter()
        .all(|j| matches!(j.verdict, Verdict::Witness(_)));
    out.reason = if out.continuous_path_proven {
        "continuous-offset-path-proven"
    } else {
        "neighboring-tube-root-identity-unresolved"
    };
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn plane() -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 3., 0.]],
                vec![vec![3., 0., 0.], vec![3., 3., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    fn limits(cells: usize) -> Limits {
        Limits {
            cells,
            spans: 16,
            iterations: 8,
            numerical_tolerance_mm: 1e-8,
            padding_fraction: 0.01,
        }
    }
    fn cover(r: &Report, domain: [f64; 2]) {
        assert_eq!(r.cells.first().unwrap().drive[0], domain[0]);
        assert_eq!(r.cells.last().unwrap().drive[1], domain[1]);
        for pair in r.cells.windows(2) {
            assert_eq!(pair[0].drive[1], pair[1].drive[0]);
        }
    }
    #[test]
    fn planar_path_has_uniform_root_evidence_not_only_endpoint_samples() {
        let a = plane();
        let mut b = plane();
        for p in b.control_points.iter_mut().flatten() {
            p.swap(1, 2);
        }
        let r = certify(
            [&a, &b],
            [0.2, -0.2],
            0,
            [0.1, 0.9],
            [[[0.1, 0.], [0.1, 0.]], [[0.9, 0.], [0.9, 0.]]],
            limits(32),
        )
        .unwrap();
        cover(&r, [0.1, 0.9]);
        assert!(r.continuous_path_proven, "{}", r.reason);
        assert_eq!(r.band_queries, 1);
        assert_eq!(r.numerical_queries, 3);
        assert_eq!(r.section_queries, 3);
    }
    #[test]
    fn curved_path_adapts_and_work_stop_preserves_the_requested_cover() {
        let mut a = plane();
        a.degree_u = 2;
        a.knots_u = vec![0., 0., 0., 1., 1., 1.];
        a.control_points = [[3., 0.], [3., 3.], [0., 3.]]
            .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 5.]])
            .to_vec();
        a.weights = [1., std::f64::consts::FRAC_1_SQRT_2, 1.]
            .map(|w| vec![w; 2])
            .to_vec();
        let b = plane();
        let seeds = [0.2, 0.8].map(|u| {
            let p = surface_offset::evaluate(&a, [u, 0.], -0.2).unwrap().point;
            [[u, 0.], [p[0] / 3., p[1] / 3.]]
        });
        let stopped = certify([&a, &b], [-0.2, 0.2], 0, [0.2, 0.8], seeds, limits(1)).unwrap();
        cover(&stopped, [0.2, 0.8]);
        assert!(!stopped.continuous_path_proven);
        assert_eq!(stopped.visited, 1);
        let r = certify([&a, &b], [-0.2, 0.2], 0, [0.2, 0.8], seeds, limits(255)).unwrap();
        cover(&r, [0.2, 0.8]);
        assert!(
            r.continuous_path_proven,
            "{}; cells={}, visited={}",
            r.reason,
            r.cells.len(),
            r.visited
        );
        assert!(r.cells.len() > 1);
        assert_eq!(r.joins.len() + 1, r.cells.len());
        assert!(r
            .joins
            .iter()
            .all(|j| matches!(j.verdict, Verdict::Witness(_))));
        assert!(r.numerical_queries <= 3 * r.visited);
        assert!(r.band_queries <= r.visited);
        assert!(r.section_queries <= r.numerical_queries + r.joins.len());
    }
}
