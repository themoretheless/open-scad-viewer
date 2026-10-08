//! Whole-chart injectivity under one fixed exact integer linear projection.
//! Sampling may propose a projection elsewhere, but never certifies this result.
use crate::{
    check, distance_bounds::Interval as I, surface::Surface, surface_measure::jets, Result,
};
#[derive(Clone, Debug)]
pub struct ProfileParameterization {
    pub weights: [f64; 2],
    pub derivative_lower: f64,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub certified: bool,
    pub cells: usize,
    pub projection: [[i8; 3]; 2],
    pub row_weights: [f64; 2],
    pub profile_parameterization: Option<ProfileParameterization>,
    pub reason: Option<&'static str>,
}
pub fn inspect(s: &Surface, projection: [[i8; 3]; 2], max_cells: usize) -> Result<Report> {
    inspect_with_depth(s, projection, max_cells, 12)
}
fn inspect_with_depth(
    s: &Surface,
    projection: [[i8; 3]; 2],
    max_cells: usize,
    max_depth: usize,
) -> Result<Report> {
    let first = inspect_metric(s, projection, [1., 1.], max_cells, max_depth)?;
    if first.certified || first.cells == max_cells {
        return Ok(first);
    }
    let mut second = inspect_weighted(s, projection, max_cells - first.cells, max_depth)?;
    second.cells += first.cells;
    Ok(second)
}
fn inspect_weighted(
    s: &Surface,
    projection: [[i8; 3]; 2],
    max_cells: usize,
    max_depth: usize,
) -> Result<Report> {
    let u = 0.5 * s.knots_u[s.degree_u] + 0.5 * s.knots_u[s.control_points.len()];
    let v = 0.5 * s.knots_v[s.degree_v] + 0.5 * s.knots_v[s.control_points[0].len()];
    let Some((du, dv)) = s.evaluate(u, v)?.first_derivatives() else {
        return inspect_metric(s, projection, [1., 1.], 0, max_depth);
    };
    let diagonal = [
        (0..3).map(|k| projection[0][k] as f64 * du[k]).sum::<f64>(),
        (0..3).map(|k| projection[1][k] as f64 * dv[k]).sum::<f64>(),
    ];
    let scale = (diagonal[0] / diagonal[1]).abs();
    if !scale.is_finite() || scale <= 0. || scale == 1. {
        return inspect_metric(s, projection, [1., 1.], 0, max_depth);
    }
    // A sampled diagonal only proposes the fixed metric. Every chart cell
    // still requires the complete outward SPD proof; failures charge its budget.
    inspect_metric(s, projection, [scale, 1.], max_cells, max_depth)
}

fn inspect_metric(
    s: &Surface,
    projection: [[i8; 3]; 2],
    row_weights: [f64; 2],
    max_cells: usize,
    max_depth: usize,
) -> Result<Report> {
    s.validate()?;
    check(
        max_cells <= 100000
            && s.control_points.iter().flatten().all(|p| p.len() == 3)
            && row_weights.iter().all(|v| v.is_finite() && *v > 0.)
            && projection.iter().flatten().all(|v| (-16..=16).contains(v)),
        "Invalid linear projection or cell budget",
    )?;
    let independent = (0..3).any(|k| {
        projection[0][k] as i16 * projection[1][(k + 1) % 3] as i16
            != projection[1][k] as i16 * projection[0][(k + 1) % 3] as i16
    });
    check(independent, "Linear projection rows must be independent")?;
    let mut out = Report {
        certified: false,
        cells: 0,
        projection,
        row_weights,
        profile_parameterization: None,
        reason: Some("linear-monotone-projection-unproved"),
    };
    let degrees = [s.degree_u, s.degree_v];
    let counts = [s.control_points.len(), s.control_points[0].len()];
    let knots = [&s.knots_u, &s.knots_v];
    if s.periodic_u
        || s.periodic_v
        || (0..2).any(|k| {
            knots[k]
                .iter()
                .filter(|v| **v > knots[k][degrees[k]] && **v < knots[k][counts[k]])
                .any(|v| knots[k].iter().filter(|x| *x == v).count() > degrees[k])
        })
    {
        out.reason = Some("continuous-rectangle-domain-unproved");
        return Ok(out);
    }
    let mut pending = Vec::new();
    for u in degrees[0]..counts[0] {
        for v in degrees[1]..counts[1] {
            if knots[0][u] < knots[0][u + 1] && knots[1][v] < knots[1][v + 1] {
                if pending.len() == max_cells {
                    out.reason = Some("cell-budget-exhausted");
                    return Ok(out);
                }
                pending.push((
                    [u, v],
                    [
                        [knots[0][u], knots[0][u + 1]],
                        [knots[1][v], knots[1][v + 1]],
                    ],
                    0usize,
                ));
            }
        }
    }
    while let Some((span, domain, depth)) = pending.pop() {
        if out.cells == max_cells {
            out.reason = Some("cell-budget-exhausted");
            return Ok(out);
        }
        out.cells += 1;
        let result = (|| -> Result<i8> {
            let j = jets::calculate(s, span, domain, None)?;
            let mut projected = [[I::point(0.); 2]; 2];
            for row in 0..2 {
                for axis in 0..2 {
                    let width = I::point(domain[axis][1]).sub(I::point(domain[axis][0]))?;
                    for k in 0..3 {
                        if projection[row][k] == 0 {
                            continue;
                        }
                        let derivative = if axis == 0 { j[1][0][k] } else { j[0][1][k] };
                        projected[row][axis] = projected[row][axis].add(
                            derivative
                                .div(width)?
                                .mul(I::point(projection[row][k] as f64))?,
                        )?;
                    }
                }
            }
            for row in 0..2 {
                for axis in 0..2 {
                    projected[row][axis] = projected[row][axis].mul(I::point(row_weights[row]))?;
                }
            }
            let [[uu, uv], [vu, vv]] = projected;
            if uu.hi <= 0. || vv.hi <= 0. {
                return Ok(-1);
            }
            let off = uv.add(vu)?.mul(I::point(0.5))?;
            let radius = I::point(off.lo.abs().max(off.hi.abs()));
            Ok(i8::from(
                uu.lo > 0. && vv.lo > 0. && uu.mul(vv)?.sub(radius.mul(radius)?)?.lo > 0.,
            ))
        })();
        match result {
            Ok(1) => (),
            Ok(0) if depth < max_depth => {
                let axis = depth % 2;
                let middle = domain[axis][0] * 0.5 + domain[axis][1] * 0.5;
                if !(domain[axis][0] < middle && middle < domain[axis][1]) {
                    out.reason = Some("parameter-subdivision-unresolved");
                    return Ok(out);
                }
                let mut left = domain;
                let mut right = domain;
                left[axis][1] = middle;
                right[axis][0] = middle;
                pending.push((span, right, depth + 1));
                pending.push((span, left, depth + 1));
            }
            Ok(0) => {
                out.reason = Some("subdivision-depth-exhausted");
                return Ok(out);
            }
            Err(_) => {
                out.reason = Some("numerical-enclosure-unresolved");
                return Ok(out);
            }
            _ => return Ok(out),
        }
    }
    out.certified = true;
    out.reason = None;
    Ok(out)
}
/// Exact comparisons of retained corner controls only propose a projection.
/// The full outward proof remains mandatory; no corner sample certifies a chart.
pub fn inspect_candidate(s: &Surface, max_cells: usize) -> Result<Report> {
    s.validate()?;
    if let Some((polynomial, parameterization)) = polynomial_profile_image(s)? {
        let mut report = inspect_candidate_projected(&polynomial, max_cells)?;
        report.profile_parameterization = Some(parameterization);
        return Ok(report);
    }
    inspect_candidate_projected(s, max_cells)
}
/// Exact positive profile reparameterization of a retained quintic image.
pub fn polynomial_profile_image(s: &Surface) -> Result<Option<(Surface, ProfileParameterization)>> {
    s.validate()?;
    // Constant longitudinal weights on a linear-profile quintic wall change
    // only its profile traversal. Prove the identical polynomial image under
    // the exact positive rational bijection, without modifying the source.
    if s.degree_u == 1
        && s.degree_v == 5
        && s.knots_u == [0., 0., 1., 1.]
        && s.control_points.len() == 2
        && !s.periodic_u
        && s.weights.iter().all(|row| row.iter().all(|w| *w == row[0]))
    {
        let weights = [s.weights[0][0], s.weights[1][0]];
        if weights != [1., 1.] && weights.iter().all(|w| (1e-100..=1e100).contains(w)) {
            let lower = I::point(weights[0].min(weights[1]))
                .div(I::point(weights[0].max(weights[1])))?
                .lo;
            if lower > 0. {
                let mut polynomial = s.clone();
                polynomial.weights.iter_mut().for_each(|row| row.fill(1.));
                return Ok(Some((
                    polynomial,
                    ProfileParameterization {
                        weights,
                        derivative_lower: lower,
                    },
                )));
            }
        }
    }
    Ok(None)
}
impl ProfileParameterization {
    pub fn inverse_interval(&self, lambda: [f64; 2]) -> Result<[f64; 2]> {
        Self {
            weights: [self.weights[1], self.weights[0]],
            derivative_lower: self.derivative_lower,
        }
        .map_interval(lambda)
        .map(|mapped| mapped.0)
    }

    pub fn map_interval(&self, u: [f64; 2]) -> Result<([f64; 2], [f64; 2])> {
        check(
            u[0] >= 0.
                && u[1] <= 1.
                && u[0] <= u[1]
                && self.weights.iter().all(|w| (1e-100..=1e100).contains(w)),
            "Invalid rational profile interval",
        )?;
        let x = I::new(u[0], u[1])?;
        let a = I::point(self.weights[0]);
        let b = I::point(self.weights[1]);
        let denominator = a.mul(I::point(1.).sub(x)?)?.add(b.mul(x)?)?.intersect(
            self.weights[0].min(self.weights[1]),
            self.weights[0].max(self.weights[1]),
        )?;
        let mapped = b.mul(x)?.div(denominator)?.intersect(0., 1.)?;
        let derivative = a.mul(b)?.div(denominator.mul(denominator)?)?;
        Ok(([mapped.lo, mapped.hi], [derivative.lo, derivative.hi]))
    }
}
fn inspect_candidate_projected(s: &Surface, max_cells: usize) -> Result<Report> {
    // Preserve the legacy unit-metric candidates and their shared budget first.
    // New metric proposals cannot consume work needed by existing certificates.
    // New degree-five linear-profile walls benefit from the metric during each
    // candidate attempt. Legacy source charts retain their original ordering.
    let quintic = s.degree_u == 1 && s.degree_v == 5;
    let first = inspect_candidate_proposals(s, max_cells, quintic)?;
    if first.certified || first.cells == max_cells {
        return Ok(first);
    }
    let independent = (0..3).any(|k| {
        first.projection[0][k] as i16 * first.projection[1][(k + 1) % 3] as i16
            != first.projection[1][k] as i16 * first.projection[0][(k + 1) % 3] as i16
    });
    if !independent {
        return Ok(first);
    }
    let mut second = inspect_weighted(s, first.projection, max_cells - first.cells, 14)?;
    second.cells += first.cells;
    Ok(second)
}
fn inspect_candidate_proposals(s: &Surface, max_cells: usize, weighted: bool) -> Result<Report> {
    s.validate()?;
    check(
        max_cells <= 100000 && s.control_points.iter().flatten().all(|p| p.len() == 3),
        "Invalid linear injectivity budget/dimension",
    )?;
    let a = &s.control_points[0][0];
    let ends = [
        &s.control_points[s.control_points.len() - 1][0],
        &s.control_points[0][s.control_points[0].len() - 1],
    ];
    let mut projection: [[i8; 3]; 2] = std::array::from_fn(|row| {
        std::array::from_fn(|k| {
            if ends[row][k] > a[k] {
                1
            } else if ends[row][k] < a[k] {
                -1
            } else {
                0
            }
        })
    });
    // An axis-aligned station chord can dominate the symmetric Jacobian if
    // included in both rows. Propose the transverse row without that axis.
    // This changes only the candidate; the same complete proof is required.
    let station_axes = (0..3)
        .filter(|&k| projection[1][k] != 0)
        .collect::<Vec<_>>();
    if station_axes.len() == 1 {
        projection[0][station_axes[0]] = 0;
    }
    let independent = (0..3).any(|k| {
        projection[0][k] as i16 * projection[1][(k + 1) % 3] as i16
            != projection[1][k] as i16 * projection[0][(k + 1) % 3] as i16
    });
    let first = if !independent {
        Report {
            certified: false,
            cells: 0,
            projection,
            row_weights: [1., 1.],
            profile_parameterization: None,
            reason: Some("linear-projection-proposal-unproved"),
        }
    } else {
        inspect_proposal(s, projection, max_cells, 12, weighted)?
    };
    if first.certified || first.cells == max_cells {
        return Ok(first);
    }
    // A rounded transverse direction only proposes another fixed integer map.
    // Every cell is proved again, with the remaining shared work budget.
    let chord: [f64; 3] = std::array::from_fn(|k| ends[1][k] - a[k]);
    let length2: f64 = chord.iter().map(|x| x * x).sum();
    let dot: f64 = (0..3).map(|k| projection[0][k] as f64 * chord[k]).sum();
    let transverse: [f64; 3] =
        std::array::from_fn(|k| projection[0][k] as f64 - dot / length2 * chord[k]);
    let scale = transverse.iter().map(|x| x.abs()).fold(0., f64::max);
    if !length2.is_finite() || length2 <= 0. || !scale.is_finite() || scale <= 0. {
        return Ok(first);
    }
    let station_scale = chord.iter().map(|x| x.abs()).fold(0., f64::max);
    let alternative = [
        transverse.map(|x| (2. * x / scale).round() as i8),
        chord.map(|x| (2. * x / station_scale).round() as i8),
    ];
    let independent = (0..3).any(|k| {
        alternative[0][k] as i16 * alternative[1][(k + 1) % 3] as i16
            != alternative[1][k] as i16 * alternative[0][(k + 1) % 3] as i16
    });
    if !independent || alternative == projection {
        return Ok(first);
    }
    let mut second = inspect_proposal(s, alternative, max_cells - first.cells, 12, weighted)?;
    second.cells += first.cells;
    if second.certified || second.cells == max_cells {
        return Ok(second);
    }
    // Midpoint tangents propose dual directions. The sampled jet supplies no
    // evidence: only the fixed integer map's whole-domain interval proof counts.
    let u = 0.5 * s.knots_u[s.degree_u] + 0.5 * s.knots_u[s.control_points.len()];
    let v = 0.5 * s.knots_v[s.degree_v] + 0.5 * s.knots_v[s.control_points[0].len()];
    let Some((du, dv)) = s.evaluate(u, v)?.first_derivatives() else {
        return Ok(second);
    };
    let dot = |a: [f64; 3], b: [f64; 3]| (0..3).map(|k| a[k] * b[k]).sum::<f64>();
    let uu = dot(du, du);
    let uv = dot(du, dv);
    let vv = dot(dv, dv);
    let dual: [[f64; 3]; 2] = [
        std::array::from_fn(|k| vv * du[k] - uv * dv[k]),
        std::array::from_fn(|k| uu * dv[k] - uv * du[k]),
    ];
    let mut candidate = [[0_i8; 3]; 2];
    for row in 0..2 {
        let scale = dual[row].iter().map(|x| x.abs()).fold(0., f64::max);
        if !scale.is_finite() || scale <= 0. {
            return Ok(second);
        }
        candidate[row] = dual[row].map(|x| (16. * x / scale).round() as i8);
    }
    let independent = (0..3).any(|k| {
        candidate[0][k] as i16 * candidate[1][(k + 1) % 3] as i16
            != candidate[1][k] as i16 * candidate[0][(k + 1) % 3] as i16
    });
    if !independent || candidate == alternative || candidate == projection {
        return Ok(second);
    }
    let mut third = inspect_proposal(s, candidate, max_cells - second.cells, 14, weighted)?;
    third.cells += second.cells;
    Ok(third)
}
fn inspect_proposal(
    s: &Surface,
    projection: [[i8; 3]; 2],
    max_cells: usize,
    depth: usize,
    weighted: bool,
) -> Result<Report> {
    if weighted {
        let first = inspect_weighted(s, projection, max_cells, depth)?;
        if first.certified || first.cells == max_cells {
            return Ok(first);
        }
        let mut second = inspect_metric(s, projection, [1., 1.], max_cells - first.cells, depth)?;
        second.cells += first.cells;
        Ok(second)
    } else {
        inspect_metric(s, projection, [1., 1.], max_cells, depth)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_rational_profile_bijection_preserves_the_source_and_budget() {
        let s = Surface {
            degree_u: 1,
            degree_v: 5,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 0., 0., 0., 0., 1., 1., 1., 1., 1., 1.],
            control_points: (0..2)
                .map(|u| (0..6).map(|v| vec![u as f64, v as f64 / 5., 0.]).collect())
                .collect(),
            weights: vec![vec![1.; 6], vec![2.; 6]],
            periodic_u: false,
            periodic_v: false,
        };
        let before = s.clone();
        let proof = inspect_candidate(&s, 1000).unwrap();
        assert!(proof.certified);
        let source = proof.profile_parameterization.unwrap();
        assert_eq!(source.weights, [1., 2.]);
        assert!(source.derivative_lower > 0. && source.derivative_lower <= 0.5);
        assert_eq!(s, before);
        let denied = inspect_candidate(&s, 0).unwrap();
        assert!(!denied.certified && denied.cells == 0);
        let mut changed = s.clone();
        changed.weights[1][2] = 3.;
        assert!(inspect_candidate(&changed, 1000)
            .unwrap()
            .profile_parameterization
            .is_none());
    }
    #[test]
    fn fixed_weighted_metric_is_proposed_then_proved_on_the_whole_chart() {
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![-16., 16., 0.]],
                vec![vec![0.125, 0.125, 0.], vec![-15.875, 16.125, 0.]],
            ],
            weights: vec![vec![1., 1.]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let map = [[1, 0, 0], [0, 1, 0]];
        assert!(!inspect_metric(&s, map, [1., 1.], 64, 1).unwrap().certified);
        let r = inspect(&s, map, 64).unwrap();
        assert!(r.certified);
        assert_eq!(r.row_weights, [1. / 128., 1.]);
        assert!(r.cells <= 64);
        for budget in [0, 1, 2] {
            let refused = inspect(&s, map, budget).unwrap();
            assert!(!refused.certified);
            assert!(refused.cells <= budget);
        }
        let mut singular = s.clone();
        singular.control_points[1][0] = vec![0.125, -0.125, 0.];
        singular.control_points[1][1] = vec![-15.875, 15.875, 0.];
        assert!(!inspect(&singular, map, 64).unwrap().certified);
        assert!(inspect_metric(&s, map, [-1., 1.], 64, 1).is_err());
    }
    #[test]
    fn transverse_proposal_does_not_mix_long_station_motion_into_profile_row() {
        let mut s = quarter();
        for row in &mut s.control_points {
            for (v, p) in row.iter_mut().enumerate() {
                p[2] = 100. * v as f64 + p[0];
            }
        }
        assert!(
            !inspect(&s, [[-1, 1, -1], [0, 0, 1]], 1000)
                .unwrap()
                .certified
        );
        let r = inspect_candidate(&s, 1000).unwrap();
        assert!(r.certified, "{r:?}");
        assert_eq!(r.projection, [[-1, 1, 0], [0, 0, 1]]);
        assert!(!inspect_candidate(&s, 0).unwrap().certified);
    }
    #[test]
    fn spatial_station_uses_a_fixed_transverse_integer_projection() {
        let mut s = quarter();
        for row in &mut s.control_points {
            for (v, p) in row.iter_mut().enumerate() {
                let y = p[1];
                p[1] = y + 10. * v as f64;
                p[2] = -2. * y + 5. * v as f64;
            }
        }
        let r = inspect_candidate(&s, 10000).unwrap();
        assert!(r.certified, "{r:?}");
        assert!(r.cells <= 10000);
        assert!(!inspect_candidate(&s, 0).unwrap().certified);
    }
    fn quarter() -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![1., 0., 0.], vec![1., 0., 5.]],
                vec![vec![1., 1., 0.], vec![1., 1., 5.]],
                vec![vec![0., 1., 0.], vec![0., 1., 5.]],
            ],
            weights: vec![vec![1.; 2], vec![0.5_f64.sqrt(); 2], vec![1.; 2]],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn rational_quarter_wall_has_one_global_oblique_projection() {
        let s = quarter();
        let p = [[-1, 1, 0], [0, 0, 1]];
        let r = inspect(&s, p, 1000).unwrap();
        assert!(inspect_candidate(&s, 1000).unwrap().certified);
        assert!(r.certified, "{r:?}");
        assert!(r.cells <= 1000);
        assert!(!inspect(&s, p, 0).unwrap().certified);
        assert!(!inspect(&s, [[1, 0, 0], [0, 0, 1]], 100).unwrap().certified);
        assert!(inspect(&s, [[1, 0, 0], [1, 0, 0]], 100).is_err());
    }
    #[test]
    fn folded_chart_cannot_change_projection_between_spans() {
        let mut s = quarter();
        s.degree_u = 1;
        s.knots_u = vec![0., 0., 0.5, 1., 1.];
        s.control_points[2] = s.control_points[0].clone();
        s.weights = vec![vec![1.; 2]; 3];
        assert!(!inspect_candidate(&s, 1000).unwrap().certified);
        assert!(
            !inspect(&s, [[-1, 1, 0], [0, 0, 1]], 1000)
                .unwrap()
                .certified
        );
    }
}
