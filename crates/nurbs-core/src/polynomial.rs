//! Exact tensor power-basis graphs converted to polynomial NURBS patches.
//! Arithmetic is binary64; no sampling or least-squares fitting is performed.
use crate::{Result, check, curve::Curve, surface::Surface};

fn choose(n: usize, k: usize) -> f64 {
    if k > n {
        return 0.;
    }
    let k = k.min(n - k);
    (1..=k).fold(1., |v, i| v * (n - k + i) as f64 / i as f64)
}

/// z = sum coefficients[i][j] * x^i * y^j over bounds [x0,x1,y0,y1].
/// Coefficients are rectangular and at most 13x13 (degree 12 per axis).
/// Constant/linear graphs are supported; both NURBS degrees are at least one.
/// Coordinates and coefficients must be finite; overflow returns an error.
pub fn graph(bounds: [f64; 4], coefficients: &[Vec<f64>]) -> Result<Surface> {
    let [x0, x1, y0, y1] = bounds;
    check(
        bounds.iter().all(|v| v.is_finite())
            && x0 < x1
            && y0 < y1
            && (x1 - x0).is_finite()
            && (y1 - y0).is_finite(),
        "Polynomial graph needs finite increasing bounds",
    )?;
    check(
        !coefficients.is_empty() && coefficients.len() <= 13,
        "Polynomial graph requires 1..13 coefficient rows",
    )?;
    let ny = coefficients[0].len();
    check(
        ny > 0
            && ny <= 13
            && coefficients
                .iter()
                .all(|r| r.len() == ny && r.iter().all(|v| v.is_finite())),
        "Polynomial coefficients must be a finite rectangular grid with 1..13 columns",
    )?;
    let nx = coefficients.len();
    let p = (nx - 1).max(1);
    let q = (ny - 1).max(1);
    let mut power = vec![vec![0.; q + 1]; p + 1];
    // Affine substitution x=x0+(x1-x0)u, y=y0+(y1-y0)v.
    for (i, row) in coefficients.iter().enumerate() {
        for (j, &c) in row.iter().enumerate() {
            if c == 0. {
                continue;
            }
            for (k, out) in power.iter_mut().enumerate().take(i + 1) {
                for (l, value) in out.iter_mut().enumerate().take(j + 1) {
                    *value += c
                        * choose(i, k)
                        * x0.powi((i - k) as i32)
                        * (x1 - x0).powi(k as i32)
                        * choose(j, l)
                        * y0.powi((j - l) as i32)
                        * (y1 - y0).powi(l as i32);
                }
            }
        }
    }
    check(
        power.iter().flatten().all(|x| x.is_finite()),
        "Polynomial affine substitution overflow",
    )?;
    let mut controls = vec![vec![vec![0.; 3]; q + 1]; p + 1];
    for (i, row) in controls.iter_mut().enumerate() {
        for (j, point) in row.iter_mut().enumerate() {
            point[0] = x0 * (1. - i as f64 / p as f64) + x1 * (i as f64 / p as f64);
            point[1] = y0 * (1. - j as f64 / q as f64) + y1 * (j as f64 / q as f64);
            for (k, power_row) in power.iter().enumerate().take(i + 1) {
                for (l, &c) in power_row.iter().enumerate().take(j + 1) {
                    point[2] += c * choose(i, k) / choose(p, k) * choose(j, l) / choose(q, l);
                }
            }
        }
    }
    let knots = |degree: usize| {
        std::iter::repeat_n(0., degree + 1)
            .chain(std::iter::repeat_n(1., degree + 1))
            .collect()
    };
    let result = Surface {
        degree_u: p,
        degree_v: q,
        knots_u: knots(p),
        knots_v: knots(q),
        control_points: controls,
        weights: vec![vec![1.; q + 1]; p + 1],
        periodic_u: false,
        periodic_v: false,
    };
    result.validate()?;
    Ok(result)
}

/// P(t)=sum coefficients[i]*t^i, over a finite increasing domain.
/// Output parameter [0,1] maps affinely to t. Degree at most 12.
pub fn parametric_curve(domain: [f64; 2], coefficients: &[[f64; 3]]) -> Result<Curve> {
    let grid: Vec<Vec<[f64; 3]>> = coefficients.iter().map(|c| vec![*c]).collect();
    let patch = parametric_surface([domain[0], domain[1], 0., 1.], &grid)?;
    let result = Curve {
        degree: patch.degree_u,
        knots: patch.knots_u,
        control_points: patch
            .control_points
            .into_iter()
            .map(|row| row[0].clone())
            .collect(),
        weights: vec![1.; coefficients.len().max(2)],
        periodic: false,
    };
    result.validate()?;
    Ok(result)
}

/// S(u,v)=sum coefficients[i][j]*u^i*v^j, with independent XYZ polynomials.
/// Unlike graph(), this supports overhangs and arbitrary 3D polynomial patches.
/// A rectangular grid at most 13x13; regularity/self-intersection not certified.
pub fn parametric_surface(domain: [f64; 4], coefficients: &[Vec<[f64; 3]>]) -> Result<Surface> {
    check(
        !coefficients.is_empty() && coefficients.len() <= 13,
        "Parametric polynomial needs 1..13 rows",
    )?;
    let count = coefficients[0].len();
    check(
        count > 0 && count <= 13 && coefficients.iter().all(|r| r.len() == count),
        "Parametric coefficients must be a rectangular 1..13 column grid",
    )?;
    let coordinate = |axis: usize| {
        coefficients
            .iter()
            .map(|row| row.iter().map(|c| c[axis]).collect())
            .collect::<Vec<Vec<f64>>>()
    };
    let x = graph(domain, &coordinate(0))?;
    let y = graph(domain, &coordinate(1))?;
    let z = graph(domain, &coordinate(2))?;
    let mut result = x.clone();
    for (i, row) in result.control_points.iter_mut().enumerate() {
        for (j, p) in row.iter_mut().enumerate() {
            *p = vec![
                x.control_points[i][j][2],
                y.control_points[i][j][2],
                z.control_points[i][j][2],
            ];
        }
    }
    result.validate()?;
    Ok(result)
}

/// Homogeneous power coefficients [X,Y,Z,W], representing (X/W,Y/W,Z/W).
/// Requires a common strict sign for every Bernstein denominator control.
/// Mixed-sign controls are refused even if the denominator might be positive;
/// callers must subdivide or prove a different positive representation.
pub fn rational_surface(domain: [f64; 4], coefficients: &[Vec<[f64; 4]>]) -> Result<Surface> {
    check(
        !coefficients.is_empty() && coefficients.len() <= 13,
        "Rational polynomial needs 1..13 rows",
    )?;
    let count = coefficients[0].len();
    check(
        count > 0
            && count <= 13
            && coefficients
                .iter()
                .all(|r| r.len() == count && r.iter().flatten().all(|v| v.is_finite())),
        "Homogeneous coefficients must be a finite rectangular grid",
    )?;
    let scale = coefficients
        .iter()
        .flatten()
        .flatten()
        .fold(0_f64, |a, v| a.max(v.abs()));
    check(
        scale > 0.,
        "Homogeneous polynomial cannot be identically zero",
    )?;
    let component = |axis: usize| {
        coefficients
            .iter()
            .map(|row| row.iter().map(|c| c[axis] / scale).collect())
            .collect::<Vec<Vec<f64>>>()
    };
    let x = graph(domain, &component(0))?;
    let y = graph(domain, &component(1))?;
    let z = graph(domain, &component(2))?;
    let w = graph(domain, &component(3))?;
    let weights: Vec<Vec<f64>> = w
        .control_points
        .iter()
        .map(|row| row.iter().map(|p| p[2]).collect())
        .collect();
    let positive = weights.iter().flatten().all(|v| *v > 0.);
    let negative = weights.iter().flatten().all(|v| *v < 0.);
    check(
        positive || negative,
        "Denominator Bernstein controls must share a strict sign; zero/potential poles or unresolved mixed signs require subdivision",
    )?;
    let largest = weights.iter().flatten().fold(0_f64, |a, v| a.max(v.abs()));
    let mut result = x.clone();
    for (i, row) in result.control_points.iter_mut().enumerate() {
        for (j, p) in row.iter_mut().enumerate() {
            let denom = weights[i][j];
            *p = vec![
                x.control_points[i][j][2] / denom,
                y.control_points[i][j][2] / denom,
                z.control_points[i][j][2] / denom,
            ];
            result.weights[i][j] = denom.abs() / largest;
        }
    }
    result.validate()?;
    Ok(result)
}

/// Homogeneous rational spatial curve; normalized output domain [0,1].
pub fn rational_curve(domain: [f64; 2], coefficients: &[[f64; 4]]) -> Result<Curve> {
    let grid: Vec<_> = coefficients.iter().map(|c| vec![*c]).collect();
    let s = rational_surface([domain[0], domain[1], 0., 1.], &grid)?;
    let result = Curve {
        degree: s.degree_u,
        knots: s.knots_u,
        control_points: s.control_points.into_iter().map(|r| r[0].clone()).collect(),
        weights: s.weights.into_iter().map(|r| r[0]).collect(),
        periodic: false,
    };
    result.validate()?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rational_circle_chart_and_sign_normalization() {
        // P(t)=((1-t²)/(1+t²),2t/(1+t²),0), t in [0,1].
        let coefficients = [[1., 0., 0., 1.], [0., 2., 0., 0.], [-1., 0., 0., 1.]];
        let a = rational_curve([0., 1.], &coefficients).unwrap();
        let neg = coefficients.map(|p| p.map(|x| -x));
        let b = rational_curve([0., 1.], &neg).unwrap();
        for i in 0..=50 {
            let t = i as f64 / 50.;
            let q = a.evaluate(t).unwrap().point;
            assert!((q[0] - (1. - t * t) / (1. + t * t)).abs() < 1e-13);
            assert!((q[1] - 2. * t / (1. + t * t)).abs() < 1e-13);
            assert!((q[0] * q[0] + q[1] * q[1] - 1.).abs() < 1e-13);
            let r = b.evaluate(t).unwrap().point;
            assert!(q.iter().zip(r).all(|(x, y)| (x - y).abs() < 1e-13));
        }
        assert!(rational_curve([0., 1.], &[[1., 0., 0., 0.], [0., 0., 0., 1.]]).is_err());
        // Positive analytic denominator, but mixed Bernstein controls: refuse.
        assert!(
            rational_curve(
                [-1., 1.],
                &[[1., 0., 0., 0.01], [0., 0., 0., 0.], [0., 0., 0., 1.]]
            )
            .is_err()
        );
    }
    #[test]
    fn nonseparable_rational_patch_and_json_dispatch() {
        let c = vec![
            vec![[0., 0., 0., 1.], [0., 1., 0., 0.]],
            vec![[1., 0., 0., 0.], [0., 0., 1., 1.]],
        ];
        let s = rational_surface([0., 1., 0., 1.], &c).unwrap();
        for i in 0..=20 {
            for j in 0..=20 {
                let u = i as f64 / 20.;
                let v = j as f64 / 20.;
                let d = 1. + u * v;
                let q = s.evaluate(u, v).unwrap().point;
                assert!((q[0] - u / d).abs() < 1e-13);
                assert!((q[1] - v / d).abs() < 1e-13);
                assert!((q[2] - u * v / d).abs() < 1e-13);
            }
        }
        #[cfg(feature = "transport")]
        {
            assert!(crate::dispatch(value_codec::json!({"op":"curve_rational_polynomial","domain":[0.,1.],"coefficients":[[1.,0.,0.,1.],[0.,1.,0.,1.]]})).is_ok());
            assert!(crate::dispatch(value_codec::json!({"op":"surface_rational_polynomial","domain":[0.,1.,0.,1.],"coefficients":[[[0.,0.,0.,1.],[0.,1.,0.,0.]],[[1.,0.,0.,0.],[0.,0.,1.,1.]]]})).is_ok());
        }
    }
    #[test]
    fn spatial_curve_and_overhanging_patch_follow_xyz_polynomials() {
        let c = parametric_curve(
            [-1., 2.],
            &[[1., 2., 3.], [2., -1., 0.], [0., 0., 1.], [0., 2., 0.]],
        )
        .unwrap();
        for i in 0..=30 {
            let u = i as f64 / 30.;
            let t = -1. + 3. * u;
            let q = c.evaluate(u).unwrap().point;
            assert!((q[0] - (1. + 2. * t)).abs() < 1e-12);
            assert!((q[1] - (2. - t + 2. * t.powi(3))).abs() < 1e-12);
            assert!((q[2] - (3. + t * t)).abs() < 1e-12);
        }
        // x=u², y=v, z=u+u*v. Two u values can share the same x,y.
        let mut coefficients = vec![vec![[0.; 3]; 2]; 3];
        coefficients[2][0][0] = 1.;
        coefficients[0][1][1] = 1.;
        coefficients[1][0][2] = 1.;
        coefficients[1][1][2] = 1.;
        let s = parametric_surface([-1., 1., 0., 2.], &coefficients).unwrap();
        for i in 0..=20 {
            for j in 0..=20 {
                let a = i as f64 / 20.;
                let b = j as f64 / 20.;
                let u = -1. + 2. * a;
                let v = 2. * b;
                let q = s.evaluate(a, b).unwrap().point;
                assert!((q[0] - u * u).abs() < 1e-12);
                assert!((q[1] - v).abs() < 1e-12);
                assert!((q[2] - (u + u * v)).abs() < 1e-12);
            }
        }
        assert!(parametric_curve([0., 1.], &[]).is_err());
        assert!(parametric_surface([0., 1., 0., 1.], &[vec![[f64::NAN; 3]]]).is_err());
        #[cfg(feature = "transport")]
        {
            assert!(crate::dispatch(value_codec::json!({"op":"curve_polynomial_parametric","domain":[-1.,2.],"coefficients":[[1.,2.,3.],[2.,-1.,0.],[0.,0.,1.]]})).is_ok());
            assert!(crate::dispatch(value_codec::json!({"op":"surface_polynomial_parametric","domain":[0.,1.,0.,1.],"coefficients":[[[0.,0.,0.],[0.,1.,0.]],[[1.,0.,0.],[0.,0.,1.]]]})).is_ok());
            assert!(crate::dispatch(value_codec::json!({"op":"curve_polynomial_parametric","domain":[0.,1.],"coefficients":[[1.,2.]]})).is_err());
        }
    }
    #[test]
    fn matches_nonseparable_polynomial_and_derivatives() {
        // z=2+3x-4y+0.2x^3*y^2; independent analytic oracle.
        let mut coefficients = vec![vec![0.; 3]; 4];
        coefficients[0][0] = 2.;
        coefficients[1][0] = 3.;
        coefficients[0][1] = -4.;
        coefficients[3][2] = 0.2;
        let s = graph([-2., 3., -1., 2.], &coefficients).unwrap();
        for i in 0..=20 {
            for j in 0..=20 {
                let u = i as f64 / 20.;
                let v = j as f64 / 20.;
                let x = -2. + 5. * u;
                let y = -1. + 3. * v;
                let e = s.evaluate(u, v).unwrap();
                let (du, dv) = e.first_derivatives().unwrap();
                assert!(
                    (e.point[2] - (2. + 3. * x - 4. * y + 0.2 * x.powi(3) * y * y)).abs() < 1e-11
                );
                assert!((du[2] - 5. * (3. + 0.6 * x * x * y * y)).abs() < 1e-11);
                assert!((dv[2] - 3. * (-4. + 0.4 * x.powi(3) * y)).abs() < 1e-11);
            }
        }
    }
    #[test]
    fn degree_twelve_and_json_boundary_are_supported() {
        let mut c = vec![vec![0.; 13]; 13];
        c[12][0] = 1.;
        c[0][12] = 1.;
        c[12][12] = 1.;
        let s = graph([-0.5, 0.5, -0.5, 0.5], &c).unwrap();
        assert_eq!((s.degree_u, s.degree_v), (12, 12));
        for i in 0..=10 {
            for j in 0..=10 {
                let u = i as f64 / 10.;
                let v = j as f64 / 10.;
                let x = u - 0.5;
                let y = v - 0.5;
                let q = s.evaluate(u, v).unwrap().point;
                assert!((q[2] - (x.powi(12) + y.powi(12) + x.powi(12) * y.powi(12))).abs() < 1e-14);
            }
        }
        #[cfg(feature = "transport")]
        {
            let request = value_codec::json!({"op":"surface_polynomial_graph","bounds":[0.,1.,0.,1.],"coefficients":[[2.,3.],[4.,5.]]});
            assert!(crate::dispatch(request).is_ok());
        }
    }
    #[test]
    fn constant_graph_and_invalid_inputs() {
        let s = graph([0., 1., 0., 1.], &[vec![7.]]).unwrap();
        assert_eq!(s.evaluate(0.3, 0.6).unwrap().point[2], 7.);
        assert!(graph([0., 1., 0., 1.], &[]).is_err());
        assert!(graph([0., 1., 0., 1.], &[vec![1.], vec![1., 2.]]).is_err());
        assert!(graph([0., 1., 0., 1.], &vec![vec![1.]; 14]).is_err());
        assert!(graph([1., 1., 0., 1.], &[vec![1.]]).is_err());
        assert!(
            graph(
                [1e100, 2e100, 0., 1.],
                &[vec![0.], vec![0.], vec![0.], vec![0.], vec![1.]]
            )
            .is_err()
        );
    }
}
