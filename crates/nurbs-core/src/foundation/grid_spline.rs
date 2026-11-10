//! Tensor product natural cubic interpolation of a rectangular site grid.
use crate::{Result, check, surface::Surface};

/// Site grid is indexed [u][v]. Cubic spans have normalized [0,1] parameters.
/// The current surface budget admits 2..11 sites per axis (up to 31 controls).
pub fn interpolate(
    points: &[Vec<[f64; 3]>],
    parameters_u: &[f64],
    parameters_v: &[f64],
) -> Result<Surface> {
    let nu = points.len();
    check(
        (2..=11).contains(&nu) && parameters_u.len() == nu,
        "Grid spline needs 2..11 U sites and matching parameters",
    )?;
    let nv = points[0].len();
    check(
        (2..=11).contains(&nv) && parameters_v.len() == nv && points.iter().all(|r| r.len() == nv),
        "Grid spline needs a rectangular 2..11 V site grid and matching parameters",
    )?;
    check(
        points.iter().flatten().flatten().all(|x| x.is_finite()),
        "Grid spline sites must be finite",
    )?;
    let mut along_u = Vec::with_capacity(nv);
    for j in 0..nv {
        let sites: Vec<[f64; 3]> = points.iter().map(|r| r[j]).collect();
        along_u.push(crate::natural_spline::interpolate(&sites, parameters_u)?);
    }
    let mut controls = Vec::with_capacity(along_u[0].control_points.len());
    let mut knots_v = Vec::new();
    for i in 0..along_u[0].control_points.len() {
        let sites: Vec<[f64; 3]> = along_u
            .iter()
            .map(|c| std::array::from_fn(|k| c.control_points[i][k]))
            .collect();
        let curve = crate::natural_spline::interpolate(&sites, parameters_v)?;
        knots_v = curve.knots;
        controls.push(curve.control_points);
    }
    let s = Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: along_u[0].knots.clone(),
        knots_v,
        weights: vec![vec![1.; controls[0].len()]; controls.len()],
        control_points: controls,
        periodic_u: false,
        periodic_v: false,
    };
    s.validate()?;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_sites_of_the_panel_with_inferred_stationary_tangents() {
        let p = vec![
            vec![[0., 0., 0.], [0., 10., 0.], [0., 20., 0.]],
            vec![[10., 0., 0.], [10., 10., 8.], [10., 20., 0.]],
            vec![[30., 0., 0.], [30., 10., 0.], [30., 20., 0.]],
        ];
        let s = interpolate(&p, &[0., 1., 3.], &[0., 1., 2.]).unwrap();
        for i in 0..3 {
            for j in 0..3 {
                let q = s.evaluate([0., 1. / 3., 1.][i], [0., 0.5, 1.][j]).unwrap();
                for k in 0..3 {
                    assert!((q.point[k] - p[i][j][k]).abs() < 1e-11);
                }
            }
        }
    }
    #[test]
    fn reproduces_bilinear_geometry_and_nonuniform_authored_sites() {
        let u = [-2., -1.5, 1., 3.];
        let v = [0., 0.3, 2.];
        let points: Vec<Vec<[f64; 3]>> = u
            .iter()
            .map(|&u| v.iter().map(|&v| [u, v, u * v]).collect())
            .collect();
        let s = interpolate(&points, &u, &v).unwrap();
        for x in [0., 0.13, 0.5, 0.87, 1.] {
            for y in [0., 0.17, 0.5, 0.83, 1.] {
                let q = s.evaluate(x, y).unwrap();
                let a = -2. + 5. * x;
                let b = 2. * y;
                for k in 0..3 {
                    assert!((q.point[k] - [a, b, a * b][k]).abs() < 1e-10);
                }
                let (du, dv) = q.first_derivatives().unwrap();
                for k in 0..3 {
                    assert!((du[k] - [5., 0., 5. * b][k]).abs() < 1e-9);
                    assert!((dv[k] - [0., 2., 2. * a][k]).abs() < 1e-9);
                }
            }
        }
        for i in 0..u.len() {
            for j in 0..v.len() {
                let q = s.evaluate((u[i] + 2.) / 5., v[j] / 2.).unwrap();
                for k in 0..3 {
                    assert!((q.point[k] - points[i][j][k]).abs() < 1e-11);
                }
            }
        }
    }
    #[test]
    fn matches_independent_product_of_natural_cubic_functions() {
        let u = [0., 1. / 3., 1.];
        let v = [0., 0.5, 1.];
        let f = [0., 2., 0.];
        let g = [0., 3., 0.];
        let p: Vec<Vec<[f64; 3]>> = (0..3)
            .map(|i| (0..3).map(|j| [u[i], v[j], f[i] * g[j]]).collect())
            .collect();
        let s = interpolate(&p, &u, &v).unwrap();
        let f = |x: f64| {
            let (a, b, p0, p1, m0, m1) = if x < 1. / 3. {
                (0., 1. / 3., 0., 2., 0., -27.)
            } else {
                (1. / 3., 1., 2., 0., -27., 0.)
            };
            let h = b - a;
            let t = (x - a) / h;
            let l = 1. - t;
            l * p0 + t * p1 + h * h * ((l * l * l - l) * m0 + (t * t * t - t) * m1) / 6.
        };
        let g = |x: f64| {
            let x = x.min(1. - x);
            9. * x - 12. * x * x * x
        };
        for x in [0., 0.13, 0.5, 0.87, 1.] {
            for y in [0., 0.17, 0.5, 0.83, 1.] {
                let q = s.evaluate(x, y).unwrap();
                assert!((q.point[2] - f(x) * g(y)).abs() < 1e-10);
            }
        }
    }
    #[test]
    fn validates_rectangle_parameters_and_budget_before_constructing() {
        assert!(interpolate(&[], &[], &[]).is_err());
        assert!(interpolate(&[vec![[0.; 3]; 2], vec![[0.; 3]; 3]], &[0., 1.], &[0., 1.]).is_err());
        assert!(
            interpolate(
                &vec![vec![[0.; 3]; 12]; 2],
                &[0., 1.],
                &(0..12).map(|i| i as f64).collect::<Vec<_>>()
            )
            .is_err()
        );
        assert!(interpolate(&vec![vec![[0.; 3]; 2]; 2], &[0., 0.], &[0., 1.]).is_err());
        let p = vec![vec![[0.; 3]; 11]; 11];
        let t: Vec<f64> = (0..11).map(|i| i as f64).collect();
        let s = interpolate(&p, &t, &t).unwrap();
        assert_eq!(s.control_points.len(), 31);
        assert_eq!(s.control_points[0].len(), 31);
    }
}
