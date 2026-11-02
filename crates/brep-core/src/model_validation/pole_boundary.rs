//! Literal original boundary-pole authority, including exact periodic lifts.
use super::*;
fn active_rows(surface: &Surface, axis: usize, end: usize) -> Vec<usize> {
    let (knots, p, n) = if axis == 0 {
        (
            &surface.knots_u,
            surface.degree_u,
            surface.control_points.len(),
        )
    } else {
        (
            &surface.knots_v,
            surface.degree_v,
            surface.control_points[0].len(),
        )
    };
    let value = [knots[p], knots[n]][end];
    let span = if end == 0 {
        (p..n).find(|&i| knots[i] < knots[i + 1])
    } else {
        (p..n).rev().find(|&i| knots[i] < knots[i + 1])
    }
    .unwrap();
    if knots.iter().filter(|k| **k == value).count() >= p {
        vec![if end == 0 { span - p } else { span }]
    } else {
        (span - p..=span).collect()
    }
}
// Exact endpoint basis on one original nonempty span. This also handles pole
// boundaries produced by homogeneous cancellation across unclamped rows.
fn collapsed_value(surface: &Surface, axis: usize, end: usize, pole: [f64; 3]) -> bool {
    let rows = active_rows(surface, axis, end);
    let point = |i: usize, j: usize| {
        if axis == 0 {
            &surface.control_points[j][i]
        } else {
            &surface.control_points[i][j]
        }
    };
    let weight = |i: usize, j: usize| {
        if axis == 0 {
            surface.weights[j][i]
        } else {
            surface.weights[i][j]
        }
    };
    let count = if axis == 0 {
        surface.control_points[0].len()
    } else {
        surface.control_points.len()
    };
    if rows.len() == 1 {
        return (0..count).all(|i| point(i, rows[0]).as_slice() == pole.as_slice());
    }
    use num_rational::BigRational as Q;
    use num_traits::{One, Zero};
    let (knots, p, n) = if axis == 0 {
        (
            &surface.knots_u,
            surface.degree_u,
            surface.control_points.len(),
        )
    } else {
        (
            &surface.knots_v,
            surface.degree_v,
            surface.control_points[0].len(),
        )
    };
    let q = |v| Q::from_float(v).unwrap();
    let t = q([knots[p], knots[n]][end]);
    let span = *rows.last().unwrap();
    let mut basis = vec![Q::zero(); p + 1];
    basis[0] = Q::one();
    let mut left = basis.clone();
    let mut right = basis.clone();
    for j in 1..=p {
        left[j] = &t - q(knots[span + 1 - j]);
        right[j] = q(knots[span + j]) - &t;
        let mut saved = Q::zero();
        for r in 0..j {
            let denominator = &right[r + 1] + &left[j - r];
            if denominator.is_zero() {
                return false;
            }
            let temp = &basis[r] / denominator;
            basis[r] = &saved + &right[r + 1] * &temp;
            saved = &left[j - r] * temp;
        }
        basis[j] = saved;
    }
    if basis.iter().any(|v| v < &Q::zero()) || basis.iter().cloned().sum::<Q>() != Q::one() {
        return false;
    }
    (0..count).all(|i| {
        (0..3).all(|k| {
            (0..=p)
                .filter(|&j| !basis[j].is_zero())
                .map(|j| &basis[j] * q(weight(i, rows[j])) * (q(point(i, rows[j])[k]) - q(pole[k])))
                .sum::<Q>()
                .is_zero()
        })
    })
}
pub(crate) fn validate_pole_boundary(
    surface: &Surface,
    pcurve: &Curve,
    pole: [f64; 3],
) -> Result<()> {
    require(
        pole.iter().all(|v| v.is_finite()),
        "Pole coordinates must be finite",
    )?;
    require(
        pcurve.degree == 1 && pcurve.control_points.len() == 2,
        "Pole pcurve must be a straight surface boundary",
    )?;
    let natural = [
        [
            surface.knots_u[surface.degree_u],
            surface.knots_u[surface.control_points.len()],
        ],
        [
            surface.knots_v[surface.degree_v],
            surface.knots_v[surface.control_points[0].len()],
        ],
    ];
    let a = &pcurve.control_points[0];
    let b = &pcurve.control_points[1];
    let lies_in = |a: f64, b: f64, axis: usize| -> Result<bool> {
        let d = natural[axis];
        if a == b || !a.is_finite() || !b.is_finite() {
            return Ok(false);
        }
        if a >= d[0] && a <= d[1] && b >= d[0] && b <= d[1] {
            return Ok(true);
        }
        if ![surface.periodic_u, surface.periodic_v][axis]
            || nurbs_core::periodic_trim::verify_surface_seam_identity(surface, axis)?.is_none()
        {
            return Ok(false);
        }
        use num_rational::BigRational as Q;
        let q = |v| Q::from_float(v).unwrap();
        let width = if a < b { q(b) - q(a) } else { q(a) - q(b) };
        Ok(width <= q(d[1]) - q(d[0]))
    };
    let mut collapsed = false;
    for axis in 0..2 {
        if a[axis] != b[axis] || !lies_in(a[1 - axis], b[1 - axis], 1 - axis)? {
            continue;
        }
        for end in 0..2 {
            if a[axis] != natural[axis][end] {
                continue;
            }
            collapsed = collapsed_value(surface, axis, end, pole);
            if collapsed {
                break;
            }
        }
        if collapsed {
            break;
        }
    }
    require(
        collapsed,
        "Pole boundary is not identically collapsed in the surface control net",
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    fn source() -> Surface {
        let c = Curve {
            degree: 1,
            knots: vec![-1., 0., 1., 2., 3., 4.],
            control_points: vec![
                vec![1., 0., 0.],
                vec![0., 1., 0.],
                vec![-1., 0., 0.],
                vec![1., 0., 0.],
            ],
            weights: vec![1.; 4],
            periodic: true,
        };
        let mut top = c.clone();
        top.control_points.fill(vec![0., 0., 2.]);
        nurbs_core::surface::loft(&[c, top]).unwrap()
    }
    fn pc(a: f64, b: f64) -> Curve {
        Curve::from_polyline(vec![vec![a, 1.], vec![b, 1.]]).unwrap()
    }
    #[test]
    fn lifted_pole_boundary_requires_original_seam_and_at_most_one_lap() {
        let s = source();
        assert!(validate_pole_boundary(&s, &pc(-6., -3.), [0., 0., 2.]).is_ok());
        assert!(validate_pole_boundary(&s, &pc(-6., 0.), [0., 0., 2.]).is_err());
        let mut bad = s.clone();
        bad.knots_u[0] = (-1f64).next_up();
        assert!(validate_pole_boundary(&bad, &pc(-6., -3.), [0., 0., 2.]).is_err());
        assert!(validate_pole_boundary(&s, &pc(-6., -3.), [0., 0., 2f64.next_up()]).is_err());
    }
    #[test]
    fn inactive_endpoint_rows_cannot_impersonate_the_actual_pole() {
        let mut s = source();
        s.degree_v = 2;
        s.knots_v = vec![-1., -1., 0., 0., 1., 1., 1.];
        for row in &mut s.control_points {
            let p = row[0].clone();
            *row = vec![vec![0., 0., 2.], p.clone(), p, vec![0., 0., 2.]];
        }
        for row in &mut s.weights {
            *row = vec![1.; 4];
        }
        s.validate().unwrap();
        let pc = Curve::from_polyline(vec![vec![0., 0.], vec![3., 0.]]).unwrap();
        assert!(validate_pole_boundary(&s, &pc, [0., 0., 2.]).is_err());
        for row in &mut s.control_points {
            row[1] = vec![0., 0., 2.];
        }
        assert!(validate_pole_boundary(&s, &pc, [0., 0., 2.]).is_ok());
    }
    #[test]
    fn homogeneous_endpoint_cancellation_is_proved_exactly_without_sampling() {
        let mut s = source();
        s.degree_v = 2;
        s.knots_v = vec![-2., -1., 0., 1., 2., 3.];
        for row in &mut s.control_points {
            let p = row[0].clone();
            *row = vec![vec![p[0], p[1], 2.], vec![-2. * p[0], -2. * p[1], 2.], p];
        }
        for row in &mut s.weights {
            *row = vec![2., 1., 1.];
        }
        s.validate().unwrap();
        let pc = Curve::from_polyline(vec![vec![0., 0.], vec![3., 0.]]).unwrap();
        assert_eq!(s.evaluate(0.5, 0.).unwrap().point, [0., 0., 2.]);
        assert!(validate_pole_boundary(&s, &pc, [0., 0., 2.]).is_ok());
        s.control_points[1][1][0] = f64::from_bits(1);
        assert!(validate_pole_boundary(&s, &pc, [0., 0., 2.]).is_err());
    }
}
