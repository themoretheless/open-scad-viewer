//! A proper local diffeomorphism onto the interior of a convex quadrilateral
//! is injective. The boundary is proved independently by exact pole tests.
use super::*;
use num_rational::BigRational as Q;
use num_traits::{Signed, Zero};

fn orient(a: &[Q; 2], b: &[Q; 2], p: &[Q; 2]) -> Q {
    (&b[0] - &a[0]) * (&p[1] - &a[1]) - (&b[1] - &a[1]) * (&p[0] - &a[0])
}

pub(crate) fn certify(s: &Surface, work: &mut usize, limit: usize) -> Option<[usize; 2]> {
    let [m, n] = [s.degree_u, s.degree_v];
    if s.periodic_u
        || s.periodic_v
        || [m, n].iter().any(|d| !(2..=5).contains(d))
        || s.control_points.len() != m + 1
        || s.control_points[0].len() != n + 1
    {
        return None;
    }
    let mut spend = |cost: usize| {
        if cost > limit - *work {
            return None;
        }
        *work += cost;
        Some(())
    };
    for (knots, degree) in [(&s.knots_u, m), (&s.knots_v, n)] {
        spend(1)?;
        if knots
            .iter()
            .enumerate()
            .any(|(i, k)| *k != knots[if i <= degree { 0 } else { degree + 1 }])
        {
            return None;
        }
    }
    let origin = &s.control_points[0][0];
    let u = &s.control_points[m][0];
    let v = &s.control_points[0][n];
    // The approximate area only chooses a projection. All boundary signs
    // and the oriented determinant are certified in that projection below.
    let area = |[a, b]: [usize; 2]| {
        ((u[a] - origin[a]) * (v[b] - origin[b]) - (u[b] - origin[b]) * (v[a] - origin[a])).abs()
    };
    let axes = [[0, 1], [0, 2], [1, 2]]
        .into_iter()
        .max_by(|&a, &b| area(a).total_cmp(&area(b)))?;

    let mut poles = Vec::with_capacity(m + 1);
    for row in &s.control_points {
        let mut projected = Vec::with_capacity(n + 1);
        for point in row {
            spend(1)?;
            projected.push([
                Q::from_float(point[axes[0]])?,
                Q::from_float(point[axes[1]])?,
            ]);
        }
        poles.push(projected);
    }
    let corners = [&poles[0][0], &poles[m][0], &poles[m][n], &poles[0][n]];
    let first = orient(corners[0], corners[1], corners[2]);
    if first.is_zero() {
        return None;
    }
    let positive = first.is_positive();
    let oriented = |value: &Q| {
        if positive {
            !value.is_negative()
        } else {
            !value.is_positive()
        }
    };
    for side in 0..4 {
        spend(1)?;
        let a = corners[side];
        let b = corners[(side + 1) % 4];
        for offset in [2, 3] {
            let value = orient(a, b, corners[(side + offset) % 4]);
            if value.is_zero() || !oriented(&value) {
                return None;
            }
        }
        let length = if side % 2 == 0 { m } else { n };
        let coordinate = usize::from(a[0] == b[0]);
        let increasing = a[coordinate] < b[coordinate];
        let mut previous = &a[coordinate];
        for k in 1..=length {
            let p = match side {
                0 => &poles[k][0],
                1 => &poles[m][k],
                2 => &poles[m - k][n],
                _ => &poles[0][n - k],
            };
            if !orient(a, b, p).is_zero()
                || if increasing {
                    previous >= &p[coordinate]
                } else {
                    previous <= &p[coordinate]
                }
            {
                return None;
            }
            previous = &p[coordinate];
        }
        // The boundary tests above establish four strictly convex sides.
        // Positive rational Bernstein bases confine the patch to their hull.
        for p in poles.iter().flatten() {
            if !oriented(&orient(a, b, p)) {
                return None;
            }
        }
    }
    let mut weights = Vec::with_capacity(m + 1);
    let mut x = Vec::with_capacity(m + 1);
    let mut y = Vec::with_capacity(m + 1);
    let sign = I::point(if positive { 1. } else { -1. });
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
    super::rational_rectangle::positive_jacobian(&x, &y, &weights, spend)?;
    // Interior points map strictly inside (positive weights include all
    // corners). The boundary maps bijectively side by side. The positive
    // determinant makes the interior map a proper local diffeomorphism
    // onto the simply connected interior, hence a one-sheeted covering.
    Some(axes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trapezoidal_boundary_passes_and_a_backtracking_side_refuses() {
        let mut s = Surface {
            degree_u: 2,
            degree_v: 4,
            knots_u: [vec![0.; 3], vec![1.; 3]].concat(),
            knots_v: [vec![0.; 5], vec![1.; 5]].concat(),
            control_points: (0..=2)
                .map(|u| {
                    (0..=4)
                        .map(|v| vec![u as f64 * (2. + v as f64 / 8.), 0., v as f64])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 5]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        s.weights[1][2] = 4.;
        s.control_points[1][2][2] = 3.5;
        assert!(super::super::rational_rectangle::certify(&s, &mut 0, 1000).is_none());
        let mut used = 0;
        assert_eq!(certify(&s, &mut used, 1000), Some([0, 2]));
        assert!(certify(&s, &mut 0, used - 1).is_none());
        s.control_points[0][1][2] = 20.;
        assert!(s.evaluate(0., 0.25).unwrap().point[2] > 4.);
        assert!(certify(&s, &mut 0, 1000).is_none());
    }
}
