//! Tensor-product Bezier, bilinear and planar patches. Domains are [0,1]^2.
use crate::{Result, check, surface::Surface};

pub fn bezier(points: Vec<Vec<Vec<f64>>>, weights: Option<Vec<Vec<f64>>>) -> Result<Surface> {
    check(
        (2..=26).contains(&points.len()),
        "Bezier surface requires 2..26 rows",
    )?;
    let columns = points[0].len();
    check(
        (2..=26).contains(&columns) && points.iter().all(|r| r.len() == columns),
        "Bezier surface requires a rectangular 2..26-column control net",
    )?;
    let degree_u = points.len() - 1;
    let degree_v = columns - 1;
    let mut knots_u = vec![0.; degree_u + 1];
    knots_u.extend(vec![1.; degree_u + 1]);
    let mut knots_v = vec![0.; degree_v + 1];
    knots_v.extend(vec![1.; degree_v + 1]);
    let weights = weights.unwrap_or_else(|| vec![vec![1.; columns]; points.len()]);
    let surface = Surface {
        degree_u,
        degree_v,
        knots_u,
        knots_v,
        control_points: points,
        weights,
        periodic_u: false,
        periodic_v: false,
    };
    surface.validate()?;
    Ok(surface)
}

/// Four corner positions indexed [u][v]; arbitrary nonplanar bilinear patches
/// are supported. Degenerate patches are not certified as regular surfaces.
pub fn bilinear(corners: [[[f64; 3]; 2]; 2]) -> Result<Surface> {
    bezier(
        corners
            .iter()
            .map(|r| r.iter().map(|p| p.to_vec()).collect())
            .collect(),
        None,
    )
}

/// Affine plane origin + u*axis_u + v*axis_v, with independent span vectors.
pub fn plane(origin: [f64; 3], axis_u: [f64; 3], axis_v: [f64; 3]) -> Result<Surface> {
    check(
        origin
            .iter()
            .chain(&axis_u)
            .chain(&axis_v)
            .all(|x| x.is_finite()),
        "Plane coordinates must be finite",
    )?;
    let su = axis_u.iter().fold(0_f64, |a, x| a.max(x.abs()));
    let sv = axis_v.iter().fold(0_f64, |a, x| a.max(x.abs()));
    check(su > 0. && sv > 0., "Plane axes must be nonzero")?;
    let a = axis_u.map(|x| x / su);
    let b = axis_v.map(|x| x / sv);
    let cross = [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ];
    check(
        cross.iter().any(|x| x.abs() > 1e-14),
        "Plane axes must be independent and numerically well-conditioned",
    )?;
    let surface = bilinear(std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            std::array::from_fn(|k| origin[k] + i as f64 * axis_u[k] + j as f64 * axis_v[k])
        })
    }))?;
    let a = std::array::from_fn::<_, 3, _>(|k| {
        surface.control_points[1][0][k] - surface.control_points[0][0][k]
    });
    let b = std::array::from_fn::<_, 3, _>(|k| {
        surface.control_points[0][1][k] - surface.control_points[0][0][k]
    });
    let sa = a.iter().fold(0_f64, |s, x| s.max(x.abs()));
    let sb = b.iter().fold(0_f64, |s, x| s.max(x.abs()));
    check(
        sa > 0. && sb > 0.,
        "Plane spans collapse at the supplied coordinate precision",
    )?;
    let a = a.map(|x| x / sa);
    let b = b.map(|x| x / sb);
    let cross = [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ];
    check(
        cross.iter().any(|x| x.abs() > 1e-14),
        "Materialized plane spans are not independently representable",
    )?;
    Ok(surface)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "transport")]
    #[test]
    fn patch_json_constructors_validate_control_net_shapes() {
        for v in [
            value_codec::json!({"op":"surface_plane","origin":[0,0,0],"axisU":[2,0,1],"axisV":[0,3,1]}),
            value_codec::json!({"op":"surface_bilinear","corners":[[[0,0,0],[0,1,0]],[[1,0,0],[1,1,2]]]}),
            value_codec::json!({"op":"surface_bezier","points":[[[0,0,0],[0,1,0]],[[1,0,0],[1,1,2]]]}),
        ] {
            let s: Surface = value_codec::from_value(crate::dispatch(v).unwrap()).unwrap();
            assert_eq!(s.knots_u,vec![0.,0.,1.,1.]);
            assert_eq!(s.knots_v,vec![0.,0.,1.,1.]);
        }
        assert!(
            crate::dispatch(
                value_codec::json!({"op":"surface_bezier","points":[[[0,0,0],[0,1,0]],[[1,0,0]]]})
            )
            .is_err()
        );
    }
    fn casteljau(mut p: Vec<[f64; 4]>, t: f64) -> [f64; 4] {
        for n in (1..p.len()).rev() {
            for i in 0..n {
                for a in 0..4 {
                    p[i][a] = (1. - t) * p[i][a] + t * p[i + 1][a]
                }
            }
        }
        p[0]
    }
    #[test]
    fn rational_tensor_patch_matches_independent_two_axis_casteljau() {
        let points = (0..4)
            .map(|i| {
                (0..3)
                    .map(|j| vec![i as f64 - 1., j as f64 + 2., (i * j) as f64 - 3.])
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let weights = (0..4)
            .map(|i| {
                (0..3)
                    .map(|j| 1. + ((i + 2 * j) % 4) as f64)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let s = bezier(points.clone(), Some(weights.clone())).unwrap();
        for u in [0., 0.17, 0.63, 1.] {
            for v in [0., 0.31, 0.9, 1.] {
                let rows = points
                    .iter()
                    .zip(&weights)
                    .map(|(r, w)| {
                        casteljau(
                            r.iter()
                                .zip(w)
                                .map(|(p, w)| [p[0] * w, p[1] * w, p[2] * w, *w])
                                .collect(),
                            v,
                        )
                    })
                    .collect();
                let h = casteljau(rows, u);
                let q = s.evaluate(u, v).unwrap().point;
                for a in 0..3 {
                    assert!((q[a] - h[a] / h[3]).abs() < 1e-12)
                }
            }
        }
        assert!(bezier(vec![vec![vec![0.; 3]; 2], vec![vec![0.; 3]; 3]], None).is_err());
        assert!(bezier(points, Some(vec![vec![1.; 3]; 2])).is_err());
    }
    #[test]
    fn plane_and_bilinear_match_affine_and_corner_formulas() {
        let origin = [2., -3., 5.];
        let a = [4., 1., 2.];
        let b = [-1., 6., 3.];
        let s = plane(origin, a, b).unwrap();
        let corners = [[[1., 2., 3.], [4., 1., 8.]], [[8., -1., 2.], [7., 6., -4.]]];
        let t = bilinear(corners).unwrap();
        for u in [0., 0.23, 0.8, 1.] {
            for v in [0., 0.37, 1.] {
                let p = s.evaluate(u, v).unwrap().point;
                let q = t.evaluate(u, v).unwrap().point;
                for k in 0..3 {
                    assert!((p[k] - (origin[k] + u * a[k] + v * b[k])).abs() < 1e-12);
                    let expected = (1. - u) * (1. - v) * corners[0][0][k]
                        + (1. - u) * v * corners[0][1][k]
                        + u * (1. - v) * corners[1][0][k]
                        + u * v * corners[1][1][k];
                    assert!((q[k] - expected).abs() < 1e-12);
                }
            }
        }
        assert!(plane(origin, a, a).is_err());
        assert!(plane(origin, [0.; 3], b).is_err());
        assert!(plane([1e9; 3], [1e-30, 0., 0.], [0., 1e-30, 0.]).is_err());
    }
}
