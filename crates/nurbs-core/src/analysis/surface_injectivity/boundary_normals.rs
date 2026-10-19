//! Original homogeneous boundary jets. Restrict values AFTER differentiating;
//! expanding a point boundary to a whole cross span cannot prove cap normals.
use super::*;
use crate::numerics::bernstein_tensor::{Net, derivative, evaluate_line, minus, product};

/// Exact natural-boundary query for a literal tensor Bezier surface. `cross`
/// is 0 (u) or 1 (v); `end` is 0 or 1. Bounds cover the requested complete along
/// interval, using ORIGINAL homogeneous derivative coefficients. No sampled
/// derivatives or collapsed-width derivative extraction supplies credit.
pub fn boundary_normal_direction_bounds(
    s: &Surface,
    cross: usize,
    end: usize,
    along: [f64; 2],
    direction: [f64; 3],
) -> Result<Option<[f64; 2]>> {
    s.validate()?;
    check(
        cross < 2 && end < 2 && along.iter().chain(direction.iter()).all(|v| v.is_finite()),
        "Invalid boundary normal query",
    )?;
    let degrees = [s.degree_u, s.degree_v];
    let counts = [s.control_points.len(), s.control_points[0].len()];
    let knots = [&s.knots_u, &s.knots_v];
    if degrees.iter().any(|p| *p == 0)
        || s.periodic_u
        || s.periodic_v
        || (0..2).any(|axis| {
            counts[axis] != degrees[axis] + 1
                || knots[axis][..counts[axis]]
                    .iter()
                    .any(|v| *v != knots[axis][degrees[axis]])
                || knots[axis][counts[axis]..]
                    .iter()
                    .any(|v| *v != knots[axis][counts[axis]])
        })
    {
        return Ok(None);
    }
    check(
        s.control_points.iter().flatten().all(|p| p.len() == 3),
        "XYZ surface required for boundary normals",
    )?;
    let free = 1 - cross;
    let domain = [knots[free][degrees[free]], knots[free][counts[free]]];
    check(
        along[0] <= along[1] && along[0] >= domain[0] && along[1] <= domain[1],
        "Boundary interval outside source domain",
    )?;
    let width = I::point(domain[1]).sub(I::point(domain[0]))?;
    let cross_width =
        I::point(knots[cross][counts[cross]]).sub(I::point(knots[cross][degrees[cross]]))?;
    let scale = I::point(degrees[cross] as f64).div(cross_width)?;
    let row = if end == 0 { 0 } else { counts[cross] - 1 };
    let neighbor = if end == 0 { 1 } else { row - 1 };
    let mut h: [Net; 4] = std::array::from_fn(|_| Vec::new());
    let mut hc: [Net; 4] = std::array::from_fn(|_| Vec::new());
    for i in 0..counts[free] {
        let original = |j| -> Result<[I; 4]> {
            let (u, v) = if cross == 0 { (j, i) } else { (i, j) };
            let w = I::point(s.weights[u][v]);
            let p = &s.control_points[u][v];
            Ok([
                I::point(p[0]).mul(w)?,
                I::point(p[1]).mul(w)?,
                I::point(p[2]).mul(w)?,
                w,
            ])
        };
        let a = original(row)?;
        let b = original(neighbor)?;
        for k in 0..4 {
            h[k].push(vec![a[k]]);
            hc[k].push(vec![
                if end == 0 {
                    b[k].sub(a[k])?
                } else {
                    a[k].sub(b[k])?
                }
                .mul(scale)?,
            ]);
        }
    }
    let ha = h
        .iter()
        .map(|net| {
            derivative(net, 0)?
                .into_iter()
                .map(|row| {
                    row.into_iter()
                        .map(|x| x.div(width))
                        .collect::<Result<Vec<_>>>()
                })
                .collect::<Result<Net>>()
        })
        .collect::<Result<Vec<_>>>()?;
    let w = &h[3];
    let numerator = |jet: &[Net], axis| minus(product(&jet[axis], w)?, product(&h[axis], &jet[3])?);
    let na = (0..3)
        .map(|k| numerator(&ha, k))
        .collect::<Result<Vec<_>>>()?;
    let nc = (0..3)
        .map(|k| numerator(&hc, k))
        .collect::<Result<Vec<_>>>()?;
    let (u, v) = if cross == 0 { (&nc, &na) } else { (&na, &nc) };
    let t = I::new(along[0], along[1])?
        .sub(I::point(domain[0]))?
        .div(width)?
        .intersect(0., 1.)?;
    let weight = evaluate_line(w.iter().map(|r| r[0]).collect(), t)?;
    if weight.lo <= 0. {
        return Ok(None);
    }
    let w2 = weight.mul(weight)?;
    let denominator = w2.mul(w2)?;
    let mut value = I::point(0.);
    for k in 0..3 {
        let a = (k + 1) % 3;
        let b = (k + 2) % 3;
        let polynomial = minus(product(&u[a], &v[b])?, product(&u[b], &v[a])?)?;
        value = value.add(
            evaluate_line(polynomial.iter().map(|r| r[0]).collect(), t)?
                .mul(I::point(direction[k]))?,
        )?;
    }
    let bound = value.div(denominator)?;
    Ok(Some([bound.lo, bound.hi]))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn wall() -> Surface {
        let profile = [[2., 0.], [2., 2.], [0., 2.]];
        let weights = [1., 1., 2.];
        let scale = [1., 1.25, 1.5, 1.5, 1.25, 1.];
        let height = [0., 0., 0., 1., 1., 1.];
        Surface {
            degree_u: 2,
            degree_v: 5,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: [vec![0.; 6], vec![1.; 6]].concat(),
            control_points: profile
                .iter()
                .map(|p| {
                    scale
                        .iter()
                        .zip(height)
                        .map(|(a, z)| vec![a * p[0], a * p[1], z])
                        .collect()
                })
                .collect(),
            weights: weights.into_iter().map(|w| vec![w; 6]).collect(),
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn original_flat_cap_boundary_normals_have_opposite_nonzero_signs_and_match_independent_derivatives()
     {
        let s = wall();
        let saved = s.clone();
        for end in 0..2 {
            let r = boundary_normal_direction_bounds(&s, 1, end, [0., 1.], [0., 0., 1.])
                .unwrap()
                .unwrap();
            assert!(if end == 0 { r[1] < 0. } else { r[0] > 0. }, "{end}/{r:?}");
            for u in [0., 0.125, 0.25, 0.5, 0.75, 1.] {
                let exact = (if end == 0 { -10. } else { 10. }) / (1. + u * u);
                assert!(r[0] <= exact && exact <= r[1], "{r:?} excludes {exact}");
                let point = boundary_normal_direction_bounds(&s, 1, end, [u, u], [0., 0., 1.])
                    .unwrap()
                    .unwrap();
                assert!(point[0] <= exact && exact <= point[1]);
            }
        }
        let broad = normal_direction_bounds(&s, [[0., 1.], [0., 0.]], [0., 0., 1.], 16)
            .unwrap()
            .unwrap();
        assert!(
            broad[0] <= 0. && broad[1] >= 0.,
            "older rectangle query remains conservative"
        );
        assert_eq!(s, saved);
    }
    #[test]
    fn natural_boundary_queries_follow_tensor_transposition_and_reject_invalid_requests() {
        let a = wall();
        let mut b = a.clone();
        b.degree_u = a.degree_v;
        b.degree_v = a.degree_u;
        b.knots_u = a.knots_v.clone();
        b.knots_v = a.knots_u.clone();
        b.control_points = (0..6)
            .map(|j| (0..3).map(|i| a.control_points[i][j].clone()).collect())
            .collect();
        b.weights = (0..6)
            .map(|j| (0..3).map(|i| a.weights[i][j]).collect())
            .collect();
        for end in 0..2 {
            let x = boundary_normal_direction_bounds(&a, 1, end, [0., 1.], [0., 0., 1.])
                .unwrap()
                .unwrap();
            let y = boundary_normal_direction_bounds(&b, 0, end, [0., 1.], [0., 0., 1.])
                .unwrap()
                .unwrap();
            assert!(if end == 0 {
                y[0] > 0. && x[1] < 0.
            } else {
                y[1] < 0. && x[0] > 0.
            });
        }
        for (axis, end, interval) in [(2, 0, [0., 1.]), (0, 2, [0., 1.]), (1, 0, [-0.1, 1.])] {
            assert!(
                boundary_normal_direction_bounds(&a, axis, end, interval, [0., 0., 1.]).is_err()
            );
        }
    }
}
