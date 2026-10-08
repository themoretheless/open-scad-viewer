//! Whole-chart injectivity under one fixed exact integer linear projection.
//! Sampling may propose a projection elsewhere, but never certifies this result.
use crate::{
    Result, check, distance_bounds::Interval as I, surface::Surface, surface_measure::jets,
};
#[derive(Clone, Debug)]
pub struct Report {
    pub certified: bool,
    pub cells: usize,
    pub projection: [[i8; 3]; 2],
    pub reason: Option<&'static str>,
}
pub fn inspect(s: &Surface, projection: [[i8; 3]; 2], max_cells: usize) -> Result<Report> {
    inspect_with_depth(s, projection, max_cells, 12)
}
/// Linear projection commutes with the rational basis when every projected
/// Euclidean pole is exactly representable. Never substitute rounded poles.
fn exact_projected_surface(s: &Surface, projection: [[i8; 3]; 2]) -> Option<Surface> {
    use crate::exact_curve_segments::{exact_add, exact_mul};
    let mut out = s.clone();
    for (source, target) in s
        .control_points
        .iter()
        .flatten()
        .zip(out.control_points.iter_mut().flatten())
    {
        for row in 0..2 {
            let mut value = 0.;
            for k in 0..3 {
                value = exact_add(value, exact_mul(source[k], projection[row][k] as f64)?)?;
            }
            target[row] = value;
        }
        target[2] = 0.;
    }
    Some(out)
}
/// With weights constant along V, the first control row is an independent
/// polynomial station curve. Exact pole subtraction gives S=P(V)+R(U,V),
/// hence Su=Ru and Sv=Pv+Rv. Never replace a pole by a rounded difference.
fn exact_station_split(s: &Surface) -> Option<(Surface, Surface)> {
    if s.weights.iter().any(|row| row.iter().any(|w| *w != row[0])) {
        return None;
    }
    let mut relative = s.clone();
    let mut position = s.clone();
    for u in 0..s.control_points.len() {
        for v in 0..s.control_points[0].len() {
            for k in 0..3 {
                relative.control_points[u][v][k] = crate::exact_curve_segments::exact_add(
                    s.control_points[u][v][k],
                    -s.control_points[0][v][k],
                )?;
            }
            position.control_points[u][v] = s.control_points[0][v].clone();
            position.weights[u][v] = 1.;
        }
    }
    Some((relative, position))
}
fn inspect_with_depth(
    s: &Surface,
    projection: [[i8; 3]; 2],
    max_cells: usize,
    max_depth: usize,
) -> Result<Report> {
    s.validate()?;
    check(
        max_cells <= 100000
            && s.control_points.iter().flatten().all(|p| p.len() == 3)
            && projection.iter().flatten().all(|v| (-64..=64).contains(v)),
        "Invalid linear projection or cell budget",
    )?;
    let independent = (0..3).any(|k| {
        projection[0][k] as i16 * projection[1][(k + 1) % 3] as i16
            != projection[1][k] as i16 * projection[0][(k + 1) % 3] as i16
    });
    check(independent, "Linear projection rows must be independent")?;
    let projected_surface = exact_projected_surface(s, projection);
    let working = projected_surface.as_ref().unwrap_or(s);
    let station_split = exact_station_split(working);
    let mut out = Report {
        certified: false,
        cells: 0,
        projection,
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
            // Differentiate the original homogeneous span before restricting
            // its value image. Query-cell differences amplify interval widths
            // on small translated rational walls and waste the shared budget.
            let derivatives = if let Some((relative, position)) = &station_split {
                let r = jets::calculate_partial_stable(relative, span, domain, [None; 2])?;
                let p = jets::calculate_partial_stable(position, span, domain, [None; 2])?;
                let mut dv = r[0][1];
                for k in 0..3 {
                    dv[k] = dv[k].add(p[0][1][k])?;
                }
                [r[1][0], dv]
            } else {
                let j = jets::calculate_partial_stable(working, span, domain, [None; 2])?;
                [j[1][0], j[0][1]]
            };
            let mut projected = [[I::point(0.); 2]; 2];
            for row in 0..2 {
                for axis in 0..2 {
                    let width = I::point(knots[axis][span[axis] + 1])
                        .sub(I::point(knots[axis][span[axis]]))?;
                    if projected_surface.is_some() {
                        let derivative = derivatives[axis][row];
                        projected[row][axis] = derivative.div(width)?;
                        continue;
                    }
                    for k in 0..3 {
                        if projection[row][k] == 0 {
                            continue;
                        }
                        let derivative = derivatives[axis][k];
                        projected[row][axis] = projected[row][axis].add(
                            derivative
                                .div(width)?
                                .mul(I::point(projection[row][k] as f64))?,
                        )?;
                    }
                }
            }
            // Restrict-then-differentiate and differentiate-then-restrict are
            // independent outward enclosures of the same Jacobian. Neither
            // dominates for every retained chart. Intersect them rather than
            // losing the tighter enclosure on corrected miter walls.
            let restricted = jets::calculate(working, span, domain, None)?;
            for row in 0..2 {
                for axis in 0..2 {
                    let width = I::point(domain[axis][1]).sub(I::point(domain[axis][0]))?;
                    let derivative = if axis == 0 {
                        restricted[1][0]
                    } else {
                        restricted[0][1]
                    };
                    let other = if projected_surface.is_some() {
                        derivative[row].div(width)?
                    } else {
                        let mut value = I::point(0.);
                        for k in 0..3 {
                            if projection[row][k] != 0 {
                                value = value.add(
                                    derivative[k]
                                        .div(width)?
                                        .mul(I::point(projection[row][k] as f64))?,
                                )?;
                            }
                        }
                        value
                    };
                    projected[row][axis] = I::new(
                        projected[row][axis].lo.max(other.lo),
                        projected[row][axis].hi.min(other.hi),
                    )?;
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
/// A proposal is never a certificate: inspect the actual whole surface first,
/// then spend only the remaining shared allowance on generic candidates.
pub fn inspect_candidate_with_hint(
    s: &Surface,
    hint: Option<[[i8; 3]; 2]>,
    max_cells: usize,
) -> Result<Report> {
    let Some(projection) = hint else {
        return inspect_candidate(s, max_cells);
    };
    let first = inspect(s, projection, max_cells)?;
    if first.certified || first.cells == max_cells {
        return Ok(first);
    }
    let mut fallback = inspect_candidate(s, max_cells - first.cells)?;
    fallback.cells += first.cells;
    Ok(fallback)
}
pub fn inspect_candidate(s: &Surface, max_cells: usize) -> Result<Report> {
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
    let mut first = if !independent {
        Report {
            certified: false,
            cells: 0,
            projection,
            reason: Some("linear-projection-proposal-unproved"),
        }
    } else {
        inspect(s, projection, max_cells.min(16))?
    };
    if first.certified || first.cells == max_cells {
        return Ok(first);
    }
    // Longitudinal motion can overwhelm a corner-sign proposal on a thin
    // spatial wall. Try the midpoint dual map with a bounded portion first;
    // its sampled construction is only a proposal, never positive evidence.
    if let Some(candidate) = midpoint_projection(s)? {
        if candidate != projection {
            let mut hinted =
                inspect_with_depth(s, candidate, (max_cells - first.cells).min(512), 14)?;
            hinted.cells += first.cells;
            if hinted.certified || hinted.cells == max_cells {
                return Ok(hinted);
            }
            first.cells = hinted.cells;
        }
    }
    // Preserve the original candidate as a full-budget fallback; a short
    // preliminary search is not treated as a negative geometry certificate.
    if independent && first.cells < max_cells {
        let mut retry = inspect(s, projection, max_cells - first.cells)?;
        retry.cells += first.cells;
        if retry.certified || retry.cells == max_cells {
            return Ok(retry);
        }
        first = retry;
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
    let mut second = inspect(s, alternative, max_cells - first.cells)?;
    second.cells += first.cells;
    if second.certified || second.cells == max_cells {
        return Ok(second);
    }
    let Some(candidate) = midpoint_projection(s)? else {
        return Ok(second);
    };
    if candidate == alternative || candidate == projection {
        return Ok(second);
    }
    let mut third = inspect_with_depth(s, candidate, max_cells - second.cells, 14)?;
    third.cells += second.cells;
    Ok(third)
}
fn midpoint_projection(s: &Surface) -> Result<Option<[[i8; 3]; 2]>> {
    // Midpoint tangents propose dual directions. Every actual acceptance still
    // requires one fixed integer map's whole-domain outward interval proof.
    let u = 0.5 * s.knots_u[s.degree_u] + 0.5 * s.knots_u[s.control_points.len()];
    let v = 0.5 * s.knots_v[s.degree_v] + 0.5 * s.knots_v[s.control_points[0].len()];
    let Some((du, dv)) = s.evaluate(u, v)?.first_derivatives() else {
        return Ok(None);
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
            return Ok(None);
        }
        candidate[row] = dual[row].map(|x| (16. * x / scale).round() as i8);
    }
    let independent = (0..3).any(|k| {
        candidate[0][k] as i16 * candidate[1][(k + 1) % 3] as i16
            != candidate[1][k] as i16 * candidate[0][(k + 1) % 3] as i16
    });
    Ok(independent.then_some(candidate))
}
#[cfg(test)]
mod tests {
    #[test]
    fn station_split_refuses_variable_station_weights_and_rounded_differences() {
        let mut s = quarter();
        let before = s.clone();
        assert!(exact_station_split(&s).is_some());
        assert_eq!(s, before);
        s.weights[1][1] = s.weights[1][1] * 2.;
        assert!(exact_station_split(&s).is_none());
        s = before;
        s.control_points[0][0][0] = 1.;
        s.control_points[1][0][0] = 2_f64.powi(-54);
        assert!(exact_station_split(&s).is_none());
    }
    #[test]
    fn small_translated_spatial_wall_uses_original_span_jets_under_finite_work() {
        let mut s = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![
                    vec![-0.7062272226232672, 0.5675679493147961, 0.5968080444325607],
                    vec![-0.7474669927920602, 0.49724679499522595, 0.5426947020020663],
                ],
                vec![
                    vec![-0.699565435445212, 0.5608768152774212, 0.5998523983908421],
                    vec![-0.7405573704272115, 0.4909357350407962, 0.546020243648579],
                ],
                vec![
                    vec![-0.6943107816690395, 0.5634326861227221, 0.5920719792065087],
                    vec![-0.7351112988881411, 0.4937291108059471, 0.5384203254299099],
                ],
            ],
            weights: vec![vec![1.; 2], vec![0.7071067811865476; 2], vec![1.; 2]],
            periodic_u: false,
            periodic_v: false,
        };
        let before = s.clone();
        let proof = inspect_candidate(&s, 100).unwrap();
        assert!(proof.certified, "{proof:?}");
        assert!(proof.cells > 0 && proof.cells <= 100);
        assert!(!inspect(&s, proof.projection, 0).unwrap().certified);
        assert_eq!(s, before);
        // Same authored image on shifted non-unit knot domains: stable jets
        // differentiate in the original span, never in the query-cell width.
        s.knots_u = vec![23., 23., 23., 25., 25., 25.];
        s.knots_v = vec![-7., -7., -5., -5.];
        assert!(inspect_candidate(&s, 100).unwrap().certified);
        eprintln!(
            "thin spatial wall injectivity cells={} projection={:?}",
            proof.cells, proof.projection
        );
    }
    #[test]
    fn pole_projection_requires_exact_arithmetic_and_keeps_rational_basis() {
        let s = quarter();
        let p = exact_projected_surface(&s, [[1, 1, 0], [0, 0, 1]]).unwrap();
        assert_eq!(s.weights, p.weights);
        assert_eq!(s.knots_u, p.knots_u);
        assert_eq!(s.knots_v, p.knots_v);
        assert_eq!(s.degree_u, p.degree_u);
        assert_eq!(s.degree_v, p.degree_v);
        for (a, b) in s
            .control_points
            .iter()
            .flatten()
            .zip(p.control_points.iter().flatten())
        {
            assert_eq!(b[0], a[0] + a[1]);
            assert_eq!(b[1], a[2]);
            assert_eq!(b[2], 0.);
        }
        let mut rounded = s.clone();
        rounded.control_points[0][0][0] = 1.;
        rounded.control_points[0][0][1] = 2_f64.powi(-54);
        let before = rounded.clone();
        assert!(exact_projected_surface(&rounded, [[1, 1, 0], [0, 0, 1]]).is_none());
        assert_eq!(rounded, before);
    }
    #[test]
    fn sheared_periodic_wall_keeps_unproved_injectivity_explicit() {
        let s = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![
                    vec![0.6530574994261776, -0.4177342973963124, 2.8505263778629866],
                    vec![0.6902224237062069, -0.39135860255547433, 3.564086526338608],
                ],
                vec![
                    vec![1.2456075346925495, 0.11617155954812869, 3.9102440379557444],
                    vec![1.3054479503736172, 0.15174399496075583, 4.637554869660337],
                ],
                vec![
                    vec![0.5905981815837974, 0.5726061673805435, 3.6546149667003553],
                    vec![0.6121768723162396, 0.5936397910373246, 4.317133369542564],
                ],
            ],
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        let proof = inspect_candidate(&s, 10000).unwrap();
        assert!(!proof.certified, "{proof:?}");
        assert_eq!(proof.reason, Some("subdivision-depth-exhausted"));
        let transported = inspect(&s, [[0, 17, -1], [-15, -14, 16]], 10000);
        assert!(transported.unwrap().certified);
    }

    use super::*;
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
        assert!(
            inspect(&s, [[-1, 1, 0], [0, 0, 1]], 1000)
                .unwrap()
                .certified
        );
        assert!(inspect(&s, r.projection, 1000).unwrap().certified);
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
