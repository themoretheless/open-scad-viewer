//! Closed cubic interpolation with a cyclic moment system.
use crate::{Result, check, curve::Curve};

/// Points include a repeated endpoint, which must match exactly. Parameters are
/// strictly increasing. Output is clamped on [0,1], not a periodic knot encoding.
/// Real-arithmetic C2 includes the seam; binary64 continuity is not certified.
pub fn interpolate(points: &[[f64; 3]], parameters: &[f64]) -> Result<Curve> {
    let count = points.len();
    check(
        (4..=86).contains(&count) && parameters.len() == count,
        "Closed spline needs matching 4..86 sites including the repeated endpoint",
    )?;
    check(
        points.iter().flatten().all(|x| x.is_finite())
            && parameters.iter().all(|x| x.is_finite())
            && parameters.windows(2).all(|p| p[0] < p[1]),
        "Closed spline needs finite sites and increasing parameters",
    )?;
    check(
        points[0] == points[count - 1],
        "Closed spline endpoints must match exactly",
    )?;
    let span = parameters[count - 1] - parameters[0];
    check(
        span.is_finite() && span > 0.,
        "Closed spline parameter span overflow",
    )?;
    let mut u: Vec<f64> = parameters
        .iter()
        .map(|t| (t - parameters[0]) / span)
        .collect();
    u[0] = 0.;
    u[count - 1] = 1.;
    check(
        u.windows(2).all(|p| p[0] < p[1]),
        "Closed spline normalized intervals collapse",
    )?;
    let n = count - 1;
    let h: Vec<f64> = u.windows(2).map(|p| p[1] - p[0]).collect();
    let slopes: Vec<[f64; 3]> = (0..n)
        .map(|i| std::array::from_fn(|k| (points[i + 1][k] - points[i][k]) / h[i]))
        .collect();
    check(
        slopes.iter().flatten().all(|x| x.is_finite()),
        "Closed spline secant overflow",
    )?;
    let mut matrix = vec![vec![0.; n]; n];
    let mut rhs = vec![[0.; 3]; n];
    for i in 0..n {
        let prev = (i + n - 1) % n;
        let next = (i + 1) % n;
        matrix[i][i] = 2. * (h[prev] + h[i]);
        matrix[i][prev] += h[prev];
        matrix[i][next] += h[i];
        rhs[i] = std::array::from_fn(|k| 6. * (slopes[i][k] - slopes[prev][k]));
    }
    // Positive, strictly diagonally dominant cyclic moment system. Elimination
    // is bounded to at most 85 unknowns; no approximation or endpoint snapping.
    for i in 0..n {
        let pivot = matrix[i][i];
        check(
            pivot.is_finite() && pivot > 0.,
            "Closed spline moment system is unrepresentable",
        )?;
        for j in i + 1..n {
            let factor = matrix[j][i] / pivot;
            for k in i + 1..n {
                matrix[j][k] -= factor * matrix[i][k];
            }
            for k in 0..3 {
                rhs[j][k] -= factor * rhs[i][k];
            }
        }
    }
    let mut moments = vec![[0.; 3]; n];
    for i in (0..n).rev() {
        for k in 0..3 {
            let sum: f64 = (i + 1..n).map(|j| matrix[i][j] * moments[j][k]).sum();
            moments[i][k] = (rhs[i][k] - sum) / matrix[i][i];
        }
        check(
            moments[i].iter().all(|x| x.is_finite()),
            "Closed spline moments overflow",
        )?;
    }
    let mut tangents: Vec<[f64; 3]> = (0..n)
        .map(|i| {
            std::array::from_fn(|k| {
                slopes[i][k] - h[i] * (2. * moments[i][k] + moments[(i + 1) % n][k]) / 6.
            })
        })
        .collect();
    tangents.push(tangents[0]);
    crate::hermite::interpolate_inferred(points, &tangents, &u)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn symmetric_loop_matches_independent_segment_polynomials_and_seam_jets() {
        let points = [
            [1., 0., 0.],
            [0., 1., 0.],
            [-1., 0., 0.],
            [0., -1., 0.],
            [1., 0., 0.],
        ];
        let c = interpolate(&points, &[0., 1., 2., 3., 4.]).unwrap();
        let first = c.trim(0., 0.25).unwrap();
        // First span controls: (1,0),(1,1/2),(1/2,1),(0,1).
        for s in [0., 0.13, 0.5, 0.87, 1.] {
            let q = first.evaluate(s / 4.).unwrap();
            let a = 1. - s;
            let x = a * a * a + 3. * a * a * s + 1.5 * a * s * s;
            let y = 1.5 * a * a * s + 3. * a * s * s + s * s * s;
            assert!((q.point[0] - x).abs() < 1e-12);
            assert!((q.point[1] - y).abs() < 1e-12);
        }
        let a = first.evaluate(0.).unwrap();
        let b = c.trim(0.75, 1.).unwrap().evaluate(1.).unwrap();
        for k in 0..3 {
            assert_eq!(a.point[k], b.point[k]);
            assert!((a.d1.as_ref().unwrap()[k] - b.d1.as_ref().unwrap()[k]).abs() < 1e-11);
            assert!((a.d2.as_ref().unwrap()[k] - b.d2.as_ref().unwrap()[k]).abs() < 1e-10);
        }
        assert!(!c.periodic);
    }
    #[test]
    fn nonuniform_spatial_loop_matches_all_authored_sites_and_join_jets() {
        let p = [
            [0., 1., 2.],
            [2., -1., 3.],
            [-1., 4., 0.],
            [5., 2., -2.],
            [0., 1., 2.],
        ];
        let t = [-2., -1.7, -0.8, 0., 3.];
        let u = t.map(|t| (t + 2.) / 5.);
        let c = interpolate(&p, &t).unwrap();
        for i in 0..4 {
            let left = c.trim(u[i], u[i + 1]).unwrap().evaluate(u[i + 1]).unwrap();
            let next = (i + 1) % 4;
            let right = c
                .trim(u[next], u[next + 1])
                .unwrap()
                .evaluate(u[next])
                .unwrap();
            for k in 0..3 {
                assert!((left.point[k] - p[i + 1][k]).abs() < 1e-12);
                assert!(
                    (left.d1.as_ref().unwrap()[k] - right.d1.as_ref().unwrap()[k]).abs() < 1e-9
                );
                assert!(
                    (left.d2.as_ref().unwrap()[k] - right.d2.as_ref().unwrap()[k]).abs() < 1e-8
                );
            }
        }
    }
    #[test]
    fn refuses_open_endpoints_bad_parameters_and_budgets_without_snapping() {
        let p = [[0.; 3], [1.; 3], [2.; 3], [0.; 3]];
        assert!(interpolate(&p, &[0., 1., 2., 3.]).is_ok());
        let mut open = p;
        open[3][0] = f64::from_bits(1);
        assert!(interpolate(&open, &[0., 1., 2., 3.]).is_err());
        assert!(interpolate(&p, &[0., 1., 1., 3.]).is_err());
        assert!(interpolate(&p, &[-1e9, 0., f64::from_bits(1), 1e9]).is_err());
        assert!(interpolate(&p[..3], &[0., 1., 2.]).is_err());
        assert!(
            interpolate(
                &[[0.; 3]; 87],
                &(0..87).map(|x| x as f64).collect::<Vec<_>>()
            )
            .is_err()
        );
        assert_eq!(
            interpolate(
                &[[0.; 3]; 86],
                &(0..86).map(|x| x as f64).collect::<Vec<_>>()
            )
            .unwrap()
            .control_points
            .len(),
            256
        );
    }
}
