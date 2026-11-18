//! A simple projected boundary and a nonvanishing projected Jacobian prove
//! global injectivity by planar degree, including for nonconvex images.
use super::*;
use crate::curve::Curve;
use num_rational::BigRational as Q;

/// Prove injectivity and a strict projected Jacobian sign in one fixed
/// integer projection. Returns the result and consumed work.
pub fn certify_jordan_projection(
    s: &Surface,
    projection: [[i8; 3]; 2],
    max_work: usize,
) -> Result<(bool, usize)> {
    s.validate()?;
    let poles = s.control_points.len() * s.control_points[0].len();
    if poles > max_work {
        return Ok((false, 0));
    }
    let mut work = poles;
    let Some(projected) =
        crate::surface_linear_monotonicity::exact_projected_surface(s, projection)
    else {
        return Ok((false, work));
    };
    let certified = certify(&projected, &mut work, max_work).is_some();
    Ok((certified, work))
}

fn supported(s: &Surface) -> bool {
    !s.periodic_u && !s.periodic_v && [s.degree_u, s.degree_v].iter().all(|d| (2..=5).contains(d))
}

pub(super) fn certify_oblique(
    s: &Surface,
    work: &mut usize,
    limit: usize,
) -> Option<[[f64; 3]; 2]> {
    if !supported(s) {
        return None;
    }
    let poles = s.control_points.len() * s.control_points[0].len();
    if poles > limit - *work {
        return None;
    }
    *work += poles;
    // The evaluated midpoint only proposes a frame. Exact signed sums
    // can also remove a common displacement along a diagonal direction.
    let midpoint = crate::surface_linear_monotonicity::midpoint_projection(s)
        .ok()
        .flatten();
    let diagonals = (0..3).flat_map(|drop| {
        let keep = [(drop + 1) % 3, (drop + 2) % 3];
        [[-1, -1], [-1, 1], [1, -1], [1, 1]].map(move |signs| {
            let mut rows = [[0_i8; 3]; 2];
            for row in 0..2 {
                rows[row][keep[row]] = 1;
                rows[row][drop] = signs[row];
            }
            rows
        })
    });
    for rows in midpoint.into_iter().chain(diagonals) {
        if rows
            .iter()
            .all(|row| row.iter().filter(|&&value| value != 0).count() == 1)
        {
            continue;
        }
        if poles > limit - *work {
            break;
        }
        *work += poles;
        let Some(projected) = crate::surface_linear_monotonicity::exact_projected_surface(s, rows)
        else {
            continue;
        };
        if certify(&projected, work, limit).is_some() {
            return Some(rows.map(|row| row.map(f64::from)));
        }
    }
    None
}

pub(crate) fn certify(s: &Surface, work: &mut usize, limit: usize) -> Option<[usize; 2]> {
    let degree = [s.degree_u, s.degree_v];
    let size = [s.control_points.len(), s.control_points[0].len()];
    let knots = [&s.knots_u, &s.knots_v];
    if !supported(s) {
        return None;
    }
    let mut spend = |cost: usize| {
        if cost > limit - *work {
            return None;
        }
        *work += cost;
        Some(())
    };
    let mut spans = [Vec::new(), Vec::new()];
    for axis in 0..2 {
        let d = degree[axis];
        let count = size[axis];
        let k = knots[axis];
        spend(1)?;
        if k[..d].iter().any(|&v| v != k[d])
            || k[count..].iter().any(|&v| v != k[count])
            || k[d + 1] == k[d]
            || k[count - 1] == k[count]
        {
            return None;
        }
        let mut multiplicity = 1;
        for i in d..count {
            spend(1)?;
            if k[i] < k[i + 1] {
                spans[axis].push(i);
                multiplicity = 1;
            } else {
                multiplicity += 1;
                // The projected map must be continuous across every knot.
                // The determinant proof includes both closed one-sided cells.
                if multiplicity > d {
                    return None;
                }
            }
        }
    }
    let origin = &s.control_points[0][0];
    let u = &s.control_points[1][0];
    let v = &s.control_points[0][1];
    // The corner tangents only propose a coordinate projection and sign.
    let area = |[a, b]: [usize; 2]| {
        (u[a] - origin[a]) * (v[b] - origin[b]) - (u[b] - origin[b]) * (v[a] - origin[a])
    };
    let mut candidates = [[0, 1], [0, 2], [1, 2]].map(|axes| (axes, area(axes)));
    candidates.sort_by(|a, b| b.1.abs().total_cmp(&a.1.abs()));
    for (axes, orientation) in candidates {
        if !orientation.is_finite() || orientation == 0. {
            continue;
        }
        if certify_projection(s, &spans, axes, orientation.signum(), work, limit).is_some() {
            return Some(axes);
        }
    }
    None
}

fn certify_projection(
    s: &Surface,
    spans: &[Vec<usize>; 2],
    axes: [usize; 2],
    orientation: f64,
    work: &mut usize,
    limit: usize,
) -> Option<()> {
    let degree = [s.degree_u, s.degree_v];
    let size = [s.control_points.len(), s.control_points[0].len()];
    let knots = [&s.knots_u, &s.knots_v];
    let single_span = size == [degree[0] + 1, degree[1] + 1];
    let [m, n] = size.map(|count| count - 1);
    let origin = &s.control_points[0][0];
    let mut spend = |cost: usize| {
        if cost > limit - *work {
            return None;
        }
        *work += cost;
        Some(())
    };

    // Extract the four original clamped boundary curves. Reversed sides
    // need exact knot reflection to keep the projected curve image.
    let mut boundary = Vec::with_capacity(4);
    for side in 0..4 {
        spend(1)?;
        let axis = side % 2;
        let d = degree[axis];
        let indices = (0..size[axis]).map(|i| match side {
            0 => (i, 0),
            1 => (m, i),
            2 => (m - i, n),
            _ => (0, n - i),
        });
        let (control_points, weights) = indices
            .map(|(i, j)| {
                (
                    axes.map(|k| s.control_points[i][j][k]).to_vec(),
                    s.weights[i][j],
                )
            })
            .unzip();
        let k = knots[axis];
        let curve_knots = if size[axis] == d + 1 {
            [vec![0.; d + 1], vec![1.; d + 1]].concat()
        } else if side < 2 {
            k.clone()
        } else {
            let lo = k[d];
            let hi = k[size[axis]];
            let sum = Q::from_float(lo)? + Q::from_float(hi)?;
            let mut reversed = Vec::with_capacity(k.len());
            for &v in k.iter().rev() {
                spend(1)?;
                let mapped = if v == lo {
                    hi
                } else if v == hi {
                    lo
                } else {
                    lo + (hi - v)
                };
                if Q::from_float(mapped)? != &sum - Q::from_float(v)? {
                    return None;
                }
                reversed.push(mapped);
            }
            reversed
        };
        boundary.push(Curve {
            degree: d,
            knots: curve_knots,
            control_points,
            weights,
            periodic: false,
        });
    }
    // Charge a fixed unit per pair in addition to subdivision cells. Keep
    // this preliminary search bounded so a failed loop does not spend the
    // entire chart ledger. Distance tolerance only stops the search;
    // positive separation still requires a strictly positive lower bound.
    spend(6)?;
    let available = (limit - *work).min(32);
    if available == 0 {
        return None;
    }
    let proof = crate::trim_simplicity::inspect(&boundary, 1e-8, 6, available).ok()?;
    *work += proof.cells;
    if !proof.proven_simple {
        return None;
    }

    let mut spend = |cost: usize| {
        if cost > limit - *work {
            return None;
        }
        *work += cost;
        Some(())
    };
    let sign = I::point(orientation);
    if single_span {
        let mut weights = Vec::with_capacity(m + 1);
        let mut x = Vec::with_capacity(m + 1);
        let mut y = Vec::with_capacity(m + 1);
        for (row, weight_row) in s.control_points.iter().zip(&s.weights) {
            let mut wrow = Vec::with_capacity(n + 1);
            let mut xrow = Vec::with_capacity(n + 1);
            let mut yrow = Vec::with_capacity(n + 1);
            for (p, &weight) in row.iter().zip(weight_row) {
                let w = I::point(weight);
                wrow.push(w);
                xrow.push(
                    I::point(p[axes[0]])
                        .sub(I::point(origin[axes[0]]))
                        .ok()?
                        .mul(w)
                        .ok()?,
                );
                yrow.push(
                    I::point(p[axes[1]])
                        .sub(I::point(origin[axes[1]]))
                        .ok()?
                        .mul(sign)
                        .ok()?
                        .mul(w)
                        .ok()?,
                );
            }
            weights.push(wrow);
            x.push(xrow);
            y.push(yrow);
        }
        super::rational_rectangle::positive_jacobian(&x, &y, &weights, &mut spend)?;
    } else {
        for &u in &spans[0] {
            for &v in &spans[1] {
                spend(1)?;
                let domain = [
                    [knots[0][u], knots[0][u + 1]],
                    [knots[1][v], knots[1][v + 1]],
                ];
                let net = crate::curve_surface_composition::surface_net_on_centered(
                    s,
                    [u, v],
                    domain,
                    [origin[0], origin[1], origin[2]],
                )
                .ok()?;
                let column = |axis: usize, factor: I| {
                    net.iter()
                        .map(|row| {
                            row.iter()
                                .map(|point| point[axis].mul(factor).ok())
                                .collect::<Option<Vec<_>>>()
                        })
                        .collect::<Option<Vec<_>>>()
                };
                let x = column(axes[0], I::point(1.))?;
                let y = column(axes[1], sign)?;
                let weights = column(3, I::point(1.))?;
                super::rational_rectangle::positive_jacobian(&x, &y, &weights, &mut spend)?;
            }
        }
    }

    // For values off the projected knot lines and boundary, every
    // preimage lies in an open cell and contributes the same sign to the
    // degree. The Jordan boundary gives absolute degree one inside and zero
    // outside. The projected grid has empty interior.
    //
    // At an interior grid point, each incident cell's rational polynomial
    // extends across its sides with positive weight and nonzero Jacobian
    // near the point. The inverse function theorem on each extension
    // isolates the point in its fiber. Take a small ball about the point
    // whose boundary avoids that value. Arbitrarily nearby values off the
    // projected grid have a preimage from an incident open cell inside
    // the ball. Each such preimage contributes the same sign, so the
    // local degree is nonzero and constant near the original value.
    //
    // Thus two interior preimages would give two contributions for nearby
    // values off the grid. An interior preimage on the Jordan boundary
    // would give a contribution for a nearby exterior value. Both contradict
    // the global degree. The Jordan boundary already gives injectivity there.
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retries_the_projection_after_the_strongest_corner_minor_folds() {
        let surface = Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: [0., 0.5, 1.]
                .into_iter()
                .map(|x| {
                    [[0., 0.], [0.5, 4.], [1., 0.]]
                        .into_iter()
                        .map(|[y, z]| vec![x, y, z])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        let spans = [vec![2], vec![2]];
        let mut folded_work = 0;
        assert!(certify_projection(&surface, &spans, [0, 2], 1., &mut folded_work, 1000).is_none());
        assert!(folded_work > 0);
        let mut fallback_work = 0;
        assert!(
            certify_projection(&surface, &spans, [0, 1], 1., &mut fallback_work, 1000).is_some()
        );

        let mut used = 0;
        assert_eq!(certify(&surface, &mut used, 1000), Some([0, 1]));
        assert!(used >= folded_work + fallback_work);
        assert!(certify(&surface, &mut 0, used - 1).is_none());
    }

    fn bowed_wall() -> Surface {
        let mut s = Surface {
            degree_u: 2,
            degree_v: 4,
            knots_u: [vec![0.; 3], vec![1.; 3]].concat(),
            knots_v: [vec![0.; 5], vec![1.; 5]].concat(),
            control_points: (0..=2)
                .map(|u| {
                    (0..=4)
                        .map(|v| vec![2. * u as f64, 0., 4. + v as f64])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 5]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        s.weights[1][2] = 4.;
        s.control_points[1][2][2] = 4.5;
        for j in 0..3 {
            s.control_points[1][j][2] += 0.125;
        }
        s
    }

    #[test]
    fn oblique_projection_proves_a_chart_with_no_coordinate_certificate() {
        let mut surface = bowed_wall();
        // A large common displacement folds the coordinate projections.
        // Subtracting Z from X and Y recovers the original bowed chart.
        for row in &mut surface.control_points {
            for (point, h) in row.iter_mut().zip([0., 32., 32., 32., 0.]) {
                *point = vec![point[0] + h, point[2] + h, h];
            }
        }
        let mut coordinate_work = 0;
        assert!(certify(&surface, &mut coordinate_work, 10_000).is_none());
        let mut used = 0;
        let rows = certify_oblique(&surface, &mut used, 10_000).expect("oblique Jordan proof");
        assert!(
            rows.iter()
                .any(|row| row.iter().filter(|&&value| value != 0.).count() > 1)
        );
        assert!(certify_oblique(&surface, &mut 0, used - 1).is_none());
        let report = super::super::certify_chart(&surface, 10_000).unwrap();
        assert_eq!(
            report.reason, "global-exact-oblique-jordan-jacobian",
            "{report:?}"
        );
        assert_eq!(report.linear_projection, Some(rows));
        assert!(report.proven && report.contraction_upper.is_none());
    }

    #[test]
    fn bowed_boundary_uses_jordan_degree_and_rejects_crossings() {
        let mut s = bowed_wall();
        assert!(super::super::rational_quadrilateral::certify(&s, &mut 0, 1000).is_none());
        let mut used = 0;
        assert_eq!(certify(&s, &mut used, 1000), Some([0, 2]));
        assert!(certify(&s, &mut 0, used - 1).is_none());

        let mut exact_insert_source = s.clone();
        for point in exact_insert_source.control_points.iter_mut().flatten() {
            point[2] *= 5.;
        }
        let inserted =
            crate::surface_edit::insert(&exact_insert_source, crate::surface::Axis::V, 0.5, 1)
                .unwrap();
        let mut multispan_work = 0;
        assert_eq!(certify(&inserted, &mut multispan_work, 1000), Some([0, 2]));
        assert!(certify(&inserted, &mut 0, multispan_work - 1).is_none());
        let mut only_c0 =
            crate::surface_edit::insert(&exact_insert_source, crate::surface::Axis::V, 0.5, 4)
                .unwrap();
        // Move the full-multiplicity knot without changing the control net.
        // The shape is unchanged and the two one-sided speeds now differ.
        for knot in &mut only_c0.knots_v[5..9] {
            *knot = 0.25;
        }
        only_c0.validate().unwrap();
        let mut c0_work = 0;
        assert_eq!(certify(&only_c0, &mut c0_work, 1000), Some([0, 2]));
        assert!(certify(&only_c0, &mut 0, c0_work - 1).is_none());

        let mut discontinuous = only_c0;
        discontinuous.knots_v.insert(5, 0.25);
        for (points, weights) in discontinuous
            .control_points
            .iter_mut()
            .zip(&mut discontinuous.weights)
        {
            let mut shifted = points[4].clone();
            shifted[2] += 0.125;
            points.insert(5, shifted);
            weights.insert(5, weights[4]);
        }
        assert!(certify(&discontinuous, &mut 0, 1000).is_none());

        // The midpoint of the bottom arc now lies above the top edge,
        // forcing two boundary crossings by continuity.
        s.control_points[1][0][2] = 20.;
        assert!(s.evaluate(0.5, 0.).unwrap().point[2] > 8.);
        assert!(certify(&s, &mut 0, 1000).is_none());
    }
}
