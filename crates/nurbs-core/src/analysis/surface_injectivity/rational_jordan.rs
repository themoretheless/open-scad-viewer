//! A simple projected boundary and a nonvanishing projected Jacobian prove
//! global injectivity by planar degree, including for nonconvex images.
use super::*;
use crate::curve::Curve;
use num_rational::BigRational as Q;

pub(crate) fn certify(s: &Surface, work: &mut usize, limit: usize) -> Option<[usize; 2]> {
    let degree = [s.degree_u, s.degree_v];
    let size = [s.control_points.len(), s.control_points[0].len()];
    let knots = [&s.knots_u, &s.knots_v];
    if s.periodic_u || s.periodic_v || degree.iter().any(|d| !(2..=5).contains(d)) {
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
                // Across internal knots the original homogeneous spline,
                // and hence its positive-weight quotient, must be C1.
                if multiplicity >= d {
                    return None;
                }
            }
        }
    }
    let single_span = size == [degree[0] + 1, degree[1] + 1];
    let [m, n] = size.map(|count| count - 1);
    let origin = &s.control_points[0][0];
    let u = &s.control_points[1][0];
    let v = &s.control_points[0][1];
    // The corner tangents only propose a coordinate projection and sign.
    let area = |[a, b]: [usize; 2]| {
        (u[a] - origin[a]) * (v[b] - origin[b]) - (u[b] - origin[b]) * (v[a] - origin[a])
    };
    let axes = [[0, 1], [0, 2], [1, 2]]
        .into_iter()
        .max_by(|&a, &b| area(a).abs().total_cmp(&area(b).abs()))?;
    let orientation = area(axes);
    if !orientation.is_finite() || orientation == 0. {
        return None;
    }

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
    let sign = I::point(orientation.signum());
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

    // Off the boundary, degree is the winding number of the Jordan curve.
    // All preimages have the same orientation and each counts once. There
    // are zero preimages outside and at most one inside. An interior point
    // cannot map onto the curve: local openness would then give an exterior
    // preimage. The boundary itself is already injective.
    Some(axes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bowed_boundary_uses_jordan_degree_and_rejects_crossings() {
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
        assert!(super::super::rational_quadrilateral::certify(&s, &mut 0, 1000).is_none());
        let mut used = 0;
        assert_eq!(certify(&s, &mut used, 1000), Some([0, 2]));
        assert!(certify(&s, &mut 0, used - 1).is_none());

        let mut exact_insert_source = s.clone();
        for point in exact_insert_source.control_points.iter_mut().flatten() {
            point[2] *= 5.;
        }
        let inserted = crate::surface_edit::insert(
            &exact_insert_source, crate::surface::Axis::V, 0.5, 1,
        ).unwrap();
        let mut multispan_work = 0;
        assert_eq!(certify(&inserted, &mut multispan_work, 1000), Some([0, 2]));
        assert!(certify(&inserted, &mut 0, multispan_work - 1).is_none());
        let only_c0 = crate::surface_edit::insert(
            &exact_insert_source, crate::surface::Axis::V, 0.5, 4,
        ).unwrap();
        assert!(certify(&only_c0, &mut 0, 1000).is_none());

        // The midpoint of the bottom arc now lies above the top edge,
        // forcing two boundary crossings by continuity.
        s.control_points[1][0][2] = 20.;
        assert!(s.evaluate(0.5, 0.).unwrap().point[2] > 8.);
        assert!(certify(&s, &mut 0, 1000).is_none());
    }
}
