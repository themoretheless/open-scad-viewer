//! Compact rational conics and surfaces of revolution. Angles are in degrees.
//! Exact rational constructions in real arithmetic; binary64 rounding remains.
//!
//! Conics live in `conics`, revolution side surfaces in `revolution`; this hub
//! keeps lines, polylines and the polynomial quadratic patch, and re-exports
//! both submodules so all public paths are unchanged.
use crate::{
    Result, check,
    curve::Curve,
    surface::Surface,
};

mod conics;
mod revolution;

pub use conics::{circle, circle_arc, circle_quadrants, conic_osculating, conic_rho, ellipse_arc, hyperbola, parabola};
pub use revolution::{cone, cone_frustum, cylinder, ellipsoid, elliptic_cylinder, hyperboloid_one_sheet, hyperboloid_two_sheet, sphere, torus};

/// Exact straight segment, with normalized domain [0,1].
pub fn line(start: [f64; 3], end: [f64; 3]) -> Result<Curve> {
    polyline(&[start, end], false)
}

/// Degree-one path with equal parameter intervals per segment, not arc length.
/// Closed paths repeat the first point and retain a clamped nonperiodic basis.
pub fn polyline(points: &[[f64; 3]], closed: bool) -> Result<Curve> {
    check(
        points.len() >= 2 && points.len() <= 256,
        "Polyline requires 2..256 points",
    )?;
    check(
        points.iter().flatten().all(|x| x.is_finite()),
        "Polyline points must be finite",
    )?;
    check(
        points.windows(2).all(|pair| pair[0] != pair[1]),
        "Polyline has a zero-length segment",
    )?;
    let mut points = points.iter().map(|p| p.to_vec()).collect::<Vec<_>>();
    if closed {
        if points.first() != points.last() {
            points.push(points[0].clone())
        }
        check(
            points.len() >= 4,
            "A closed polyline requires at least three vertices",
        )?;
    }
    let mut curve = Curve::from_polyline(points)?;
    let end = curve.domain()[1];
    for knot in &mut curve.knots {
        *knot /= end
    }
    curve.validate()?;
    Ok(curve)
}

/// Graph z=a*x²+b*x*y+c*y²+d*x+e*y+f over an XY rectangle.
/// Coefficient order is [a,b,c,d,e,f]. Both parameter domains are [0,1].
/// Polynomial tensor construction, not a sampled fit; surface has no caps.
pub fn quadratic_patch(bounds: [f64; 4], coefficients: [f64; 6]) -> Result<Surface> {
    let [x0, x1, y0, y1] = bounds;
    check(
        bounds.iter().chain(&coefficients).all(|v| v.is_finite()) && x0 < x1 && y0 < y1,
        "Quadratic patch needs finite coefficients and increasing XY bounds",
    )?;
    let x = [x0, x0 / 2. + x1 / 2., x1];
    let y = [y0, y0 / 2. + y1 / 2., y1];
    let xx = [x0 * x0, x0 * x1, x1 * x1];
    let yy = [y0 * y0, y0 * y1, y1 * y1];
    let [a, b, c, d, e, f] = coefficients;
    let controls = (0..3)
        .map(|i| {
            (0..3)
                .map(|j| {
                    vec![
                        x[i],
                        y[j],
                        a * xx[i] + b * x[i] * y[j] + c * yy[j] + d * x[i] + e * y[j] + f,
                    ]
                })
                .collect()
        })
        .collect();
    let result = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: controls,
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    result.validate()?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    #[test]
    fn canonical_quarters_preserve_exact_projective_g2_including_closure() {
        for sweep in [90.,-90.] {
            let patches=(0..4).map(|i| {
                let arc=circle_arc([0.;3],[0.,0.,1.],0.2,i as f64*sweep,sweep).unwrap();
                Surface {degree_u:1,degree_v:2,knots_u:vec![0.,0.,1.,1.],knots_v:arc.knots,
                    control_points:[0.,1.].iter().map(|&z|arc.control_points.iter().map(|p|vec![p[0],p[1],z]).collect()).collect(),
                    weights:vec![arc.weights;2],periodic_u:false,periodic_v:false}
            }).collect::<Vec<_>>();
            for i in 0..4 {
                let audit=crate::continuity::inspect_surface_projective_strip_jets(&patches[i],&patches[(i+1)%4],"vMax","vMin",2,1.,1000000).unwrap();
                assert!(audit.certified,"{audit:?}");
            }
        }
        let circle=circle([0.;3],[0.,0.,1.],0.2).unwrap();
        assert_eq!(circle.control_points[0],circle.control_points[8]);
        assert_eq!(circle.control_points[2],vec![0.,0.2,0.]);
        assert_eq!(circle.control_points[4],vec![-0.2,0.,0.]);
        // A nearby authored angle keeps the general trigonometric path.
        let angle=f64::from_bits(90f64.to_bits()+1);
        let arc=circle_arc([0.;3],[0.,0.,1.],1.,angle,90.).unwrap();
        assert_ne!(arc.control_points[0][0],0.);
    }

    #[test]
    fn quadrant_arcs_close_and_preserve_signed_radius() {
        for radius in [5., -5., 0.] {
            let arcs = circle_quadrants(radius).unwrap();
            for i in 0..4 {
                arcs[i].validate().unwrap();
                let end = arcs[i].evaluate(1.).unwrap().point;
                let start = arcs[(i + 1) % 4].evaluate(0.).unwrap().point;
                for j in 0..3 {
                    assert!((end[j] - start[j]).abs() < 1e-12);
                }
                for t in [0., 0.25, 0.5, 0.75, 1.] {
                    let p = arcs[i].evaluate(t).unwrap().point;
                    assert!((p[0].hypot(p[1]) - radius.abs()).abs() < 1e-12);
                    assert_eq!(p[2], 0.);
                }
            }
        }
    }
    #[test]
    fn lines_and_polylines_preserve_sites_and_piecewise_affine_segments() {
        let sites = [[2., -1., 3.], [8., 5., -2.], [-4., 9., 7.]];
        let c = super::polyline(&sites, false).unwrap();
        assert_eq!(c.domain(), [0., 1.]);
        for segment in 0..2 {
            for t in [0., 0.17, 0.5, 0.89, 1.] {
                let p = c.evaluate((segment as f64 + t) / 2.).unwrap().point;
                for a in 0..3 {
                    assert!(
                        (p[a] - ((1. - t) * sites[segment][a] + t * sites[segment + 1][a])).abs()
                            < 1e-12
                    )
                }
            }
        }
        let closed = super::polyline(&sites, true).unwrap();
        assert_eq!(closed.control_points.first(), closed.control_points.last());
        assert_eq!(closed.control_points.len(), 4);
        assert_eq!(
            super::polyline(&[sites[0], sites[1], sites[2], sites[0]], true)
                .unwrap()
                .control_points
                .len(),
            4
        );
        let line = super::line(sites[0], sites[1]).unwrap();
        assert_eq!(line.degree, 1);
        assert_eq!(line.control_points.len(), 2);
        assert!(super::line(sites[0], sites[0]).is_err());
        assert!(super::polyline(&sites[..2], true).is_err());
        assert!(super::polyline(&[[f64::NAN, 0., 0.], [1., 0., 0.]], false).is_err());
        let too_many = (0..256).map(|i| [i as f64, 0., 0.]).collect::<Vec<_>>();
        assert!(super::polyline(&too_many, true).is_err());
    }
    #[test]
    fn circular_arcs_preserve_radius_plane_and_sweep_handedness() {
        let center = [2., -3., 5.];
        let normals: [[f64; 3]; 4] = [
            [0., 0., 1.],
            [1., 2., 3.],
            [1e-300, -2e-300, 3e-300],
            [1e300, 2e300, 3e300],
        ];
        for normal in normals {
            let scale = normal.iter().fold(0_f64, |a, x| a.max(x.abs()));
            let n = normal.map(|x| x / scale);
            let len = n.iter().map(|x| x * x).sum::<f64>().sqrt();
            let n = n.map(|x| x / len);
            let full = super::circle(center, normal, 7.).unwrap();
            assert_eq!(full.control_points.first(), full.control_points.last());
            for sweep in [-280., 75., 360.] {
                let c = super::circle_arc(center, normal, 7., 25., sweep).unwrap();
                for i in 0..=32 {
                    let p = c.evaluate(i as f64 / 32.).unwrap().point;
                    let delta = std::array::from_fn::<_, 3, _>(|a| p[a] - center[a]);
                    assert!((delta.iter().map(|x| x * x).sum::<f64>() - 49.).abs() < 1e-11);
                    assert!(delta.iter().zip(n).map(|(a, b)| a * b).sum::<f64>().abs() < 1e-12);
                }
            }
        }
        let positive = super::circle_arc([0.; 3], [0., 0., 1.], 2., 0., 90.)
            .unwrap()
            .evaluate(1.)
            .unwrap()
            .point;
        let negative = super::circle_arc([0.; 3], [0., 0., 1.], 2., 0., -90.)
            .unwrap()
            .evaluate(1.)
            .unwrap()
            .point;
        assert!((positive[1] - 2.).abs() < 1e-12 && (negative[1] + 2.).abs() < 1e-12);
        assert!(super::circle(center, [0.; 3], 7.).is_err());
        assert!(super::circle(center, [0., 0., 1.], 0.).is_err());
    }
    use super::*;
    #[test]
    fn hyperboloids_satisfy_their_implicit_equations_and_sheet_selection() {
        let center = [1., 2., 3.];
        let radii = [2., 3., 4.];
        for kind in 0..3 {
            let s = if kind == 0 {
                hyperboloid_one_sheet(center, radii, -1., 2.)
            } else {
                hyperboloid_two_sheet(center, radii, 0., 2., kind == 2)
            }
            .unwrap();
            for i in 0..=20 {
                for j in 0..=20 {
                    let q = s.evaluate(i as f64 / 20., j as f64 / 5.).unwrap().point;
                    let x = (q[0] - 1.) / 2.;
                    let y = (q[1] - 2.) / 3.;
                    let z = (q[2] - 3.) / 4.;
                    let f = if kind == 0 {
                        x * x + y * y - z * z
                    } else {
                        z * z - x * x - y * y
                    };
                    assert!((f - 1.).abs() < 2e-13);
                    if kind == 1 {
                        assert!(z >= 1. - 1e-14);
                    }
                    if kind == 2 {
                        assert!(z <= -1. + 1e-14);
                    }
                }
            }
            for row in &s.control_points {
                assert_eq!(row.first(), row.last());
            }
        }
        assert!(hyperboloid_two_sheet(center, radii, -1., 1., false).is_err());
        assert!(hyperboloid_one_sheet(center, [0., 1., 1.], 0., 1.).is_err());
    }
    #[test]
    fn quadratic_patch_matches_polynomial_on_asymmetric_domains() {
        for coefficients in [
            [1., 0., 2., 0., 0., 0.],
            [1., 0., -2., 0., 0., 0.],
            [0.3, -0.7, 1.2, 2., -3., 4.],
        ] {
            let s = quadratic_patch([-2., 3., -4., 1.], coefficients).unwrap();
            let [a, b, c, d, e, f] = coefficients;
            for i in 0..=20 {
                for j in 0..=20 {
                    let u = i as f64 / 20.;
                    let v = j as f64 / 20.;
                    let x = -2. + 5. * u;
                    let y = -4. + 5. * v;
                    let q = s.evaluate(u, v).unwrap();
                    assert!((q.point[0] - x).abs() < 1e-13);
                    assert!((q.point[1] - y).abs() < 1e-13);
                    assert!(
                        (q.point[2] - (a * x * x + b * x * y + c * y * y + d * x + e * y + f))
                            .abs()
                            < 1e-12
                    );
                    let (du, dv) = q.first_derivatives().unwrap();
                    assert!((du[2] - 5. * (2. * a * x + b * y + d)).abs() < 1e-12);
                    assert!((dv[2] - 5. * (b * x + 2. * c * y + e)).abs() < 1e-12);
                }
            }
        }
        assert!(quadratic_patch([0., 0., 0., 1.], [0.; 6]).is_err());
        assert!(quadratic_patch([0., 1., 0., 1.], [f64::NAN; 6]).is_err());
    }
    #[test]
    fn conics_and_side_surfaces_match_analytic_geometry() {
        let p = parabola([0.; 3], [2., 0., 0.], [0., 3., 0.], -2., 3.).unwrap();
        let h = hyperbola([0.; 3], [2., 0., 0.], [0., 3., 0.], -1., 2.).unwrap();
        let c = elliptic_cylinder([1., 2., 3.], 2., 3., 5.).unwrap();
        for i in 0..=100 {
            let s = i as f64 / 100.;
            let t = -2. + 5. * s;
            let q = p.evaluate(s).unwrap().point;
            assert!((q[0] - 2. * t).abs() < 1e-13);
            assert!((q[1] - 3. * t * t).abs() < 1e-13);
            let q = h.evaluate(s).unwrap().point;
            assert!((q[0] * q[0] / 4. - q[1] * q[1] / 9. - 1.).abs() < 1e-13);
            let q = c.evaluate(s, 0.37).unwrap().point;
            assert!(((q[0] - 1.).powi(2) / 4. + (q[1] - 2.).powi(2) / 9. - 1.).abs() < 1e-13);
            assert!((q[2] - 4.85).abs() < 1e-13);
        }
        for (bottom, top) in [(3., 1.), (0., 3.), (3., 0.)] {
            let c = cone_frustum([0.; 3], bottom, top, 5.).unwrap();
            for i in 0..=20 {
                for j in 0..=20 {
                    let u = i as f64 / 20.;
                    let v = j as f64 / 5.;
                    let q = c.evaluate(u, v).unwrap().point;
                    assert!((q[0].hypot(q[1]) - (bottom + (top - bottom) * u)).abs() < 1e-13);
                    assert!((q[2] - 5. * u).abs() < 1e-13);
                }
            }
        }
        assert!(cone_frustum([0.; 3], 0., 0., 1.).is_err());
        assert!(elliptic_cylinder([0.; 3], 1., 1., -1.).is_err());
        assert!(parabola([0.; 3], [1., 0., 0.], [0., 1., 0.], 1., 1.).is_err());
        assert!(hyperbola([0.; 3], [1., 0., 0.], [0., 1., 0.], -1000., 1000.).is_err());
    }
    #[test]
    fn rational_arcs_stay_on_the_ellipse_for_both_hands() {
        for sweep in [35., -125., 360., -360.] {
            let c = ellipse_arc([1., 2., 3.], [4., 0., 0.], [0., 2., 0.], 17., sweep).unwrap();
            for i in 0..=100 {
                let p = c.evaluate(i as f64 / 100.).unwrap().point;
                assert!(((p[0] - 1.).powi(2) / 16. + (p[1] - 2.).powi(2) / 4. - 1.).abs() < 2e-14);
                assert!((p[2] - 3.).abs() < 2e-14);
            }
            if sweep.abs() == 360. {
                assert_eq!(c.control_points.first(), c.control_points.last());
            }
        }
    }
    #[test]
    #[cfg(feature = "transport")]
    fn circular_surface_families_preserve_radius_and_height() {
        let center = [1., -2., 3.];
        for (kind, surface) in [
            sphere(center, 5.).unwrap(),
            cylinder(center, 5., 7.).unwrap(),
            cone(center, 5., 7.).unwrap(),
        ]
        .iter()
        .enumerate()
        {
            let u0 = surface.knots_u[surface.degree_u];
            let u1 = surface.knots_u[surface.control_points.len()];
            let v0 = surface.knots_v[surface.degree_v];
            let v1 = surface.knots_v[surface.control_points[0].len()];
            for i in 0..=20 {
                for j in 0..=20 {
                    let p = surface
                        .evaluate(
                            u0 + (u1 - u0) * i as f64 / 20.,
                            v0 + (v1 - v0) * j as f64 / 20.,
                        )
                        .unwrap()
                        .point;
                    let radial = (p[0] - center[0]).hypot(p[1] - center[1]);
                    let z = p[2] - center[2];
                    let error = match kind {
                        0 => radial * radial + z * z - 25.,
                        1 => radial - 5.,
                        _ => radial - 5. * (1. - z / 7.),
                    };
                    assert!(error.abs() < 1e-12);
                    if kind > 0 {
                        assert!(z >= -1e-12 && z <= 7. + 1e-12);
                    }
                }
            }
        }
        for radius in [0., -1., f64::NAN, f64::INFINITY] {
            assert!(sphere(center, radius).is_err());
            assert!(cylinder(center, radius, 7.).is_err());
            assert!(cone(center, radius, 7.).is_err());
        }
        for (op, extra) in [
            ("surface_sphere", ""),
            ("surface_cylinder", ",\"height\":7"),
            ("surface_cone", ",\"height\":7"),
        ] {
            let request = format!("{{\"op\":\"{op}\",\"center\":[1,-2,3],\"radius\":5{extra}}}");
            assert!(crate::transport::dispatch(value_codec::from_str(&request).unwrap()).is_ok());
        }
    }
    #[test]
    fn surfaces_satisfy_independent_implicit_equations() {
        let e = ellipsoid([1., 2., 3.], [2., 3., 4.]).unwrap();
        let t = torus([1., 2., 3.], 5., 2., 1.).unwrap();
        for i in 0..=20 {
            for j in 0..=20 {
                let u = i as f64 / 20.;
                let v = j as f64 / 5.;
                let p = e.evaluate(u, v).unwrap().point;
                assert!(
                    ((p[0] - 1.).powi(2) / 4.
                        + (p[1] - 2.).powi(2) / 9.
                        + (p[2] - 3.).powi(2) / 16.
                        - 1.)
                        .abs()
                        < 3e-14
                );
                let p = t.evaluate(u, v).unwrap().point;
                let r = (p[0] - 1.).hypot(p[1] - 2.);
                assert!(((r - 5.).powi(2) / 4. + (p[2] - 3.).powi(2) - 1.).abs() < 3e-14);
            }
        }
        for s in [&e, &t] {
            for row in &s.control_points {
                assert_eq!(row.first(), row.last());
            }
        }
        for row in [&e.control_points[0], e.control_points.last().unwrap()] {
            assert!(row.iter().all(|p| p[0] == 1. && p[1] == 2.));
        }
    }
    #[test]
    fn oblique_arc_preserves_its_plane_and_conic() {
        let c = ellipse_arc([0.; 3], [2., 0., 2.], [1., 3., 1.], -40., 230.).unwrap();
        for i in 0..=100 {
            let p = c.evaluate(i as f64 / 100.).unwrap().point;
            let sin = p[1] / 3.;
            let cos = (p[0] - sin) / 2.;
            assert!((sin * sin + cos * cos - 1.).abs() < 2e-14);
            assert!((p[0] - p[2]).abs() < 2e-14);
        }
    }
    #[cfg(feature = "transport")]
    #[test]
    fn json_constructors_are_reachable_and_validate_fields() {
        for request in [
            r#"{"op":"curve_line","start":[0,0,0],"end":[2,3,4]}"#,
            r#"{"op":"curve_polyline","points":[[0,0,0],[2,0,0],[0,3,0]],"closed":true}"#,
            r#"{"op":"curve_circle","center":[0,0,0],"normal":[1,2,3],"radius":5}"#,
            r#"{"op":"curve_circle_arc","center":[0,0,0],"normal":[0,0,1],"radius":5,"startDegrees":30,"sweepDegrees":-270}"#,
            r#"{"op":"surface_hyperboloid_one_sheet","center":[0,0,0],"radii":[2,3,4],"start":-1,"end":1}"#,
            r#"{"op":"surface_hyperboloid_two_sheet","center":[0,0,0],"radii":[2,3,4],"start":0,"end":1,"lower":true}"#,
            r#"{"op":"surface_quadratic_patch","bounds":[-2,3,-4,1],"coefficients":[1,0,-1,0,0,0]}"#,
            r#"{"op":"curve_parabola","center":[0,0,0],"axisU":[2,0,0],"axisV":[0,3,0],"start":-2,"end":3}"#,
            r#"{"op":"curve_hyperbola","center":[0,0,0],"axisU":[2,0,0],"axisV":[0,3,0],"start":-1,"end":2}"#,
            r#"{"op":"surface_elliptic_cylinder","center":[0,0,0],"radiusX":2,"radiusY":3,"height":5}"#,
            r#"{"op":"surface_cone_frustum","center":[0,0,0],"bottomRadius":3,"topRadius":0,"height":5}"#,
            r#"{"op":"curve_ellipse_arc","center":[0,0,0],"axisU":[2,0,0],"axisV":[0,1,0],"startDegrees":0,"sweepDegrees":90}"#,
            r#"{"op":"surface_ellipsoid","center":[0,0,0],"radii":[2,3,4]}"#,
            r#"{"op":"surface_torus","center":[0,0,0],"majorRadius":5,"radialRadius":2,"axialRadius":1}"#,
        ] {
            let v = value_codec::from_str(request).unwrap();
            assert!(crate::dispatch(v).is_ok());
        }
        assert!(
            crate::dispatch(
                value_codec::from_str(
                    r#"{"op":"surface_ellipsoid","center":[0,0,0],"radii":[1,0,2]}"#
                )
                .unwrap()
            )
            .is_err()
        );
    }
    #[test]
    fn invalid_inputs_return_errors() {
        assert!(ellipse_arc([0.; 3], [1., 0., 0.], [2., 0., 0.], 0., 90.).is_err());
        assert!(ellipse_arc([0.; 3], [1., 0., 0.], [0., 1., 0.], 0., 0.).is_err());
        assert!(torus([0.; 3], 1., 2., 1.).is_err());
        assert!(ellipsoid([0.; 3], [1., 0., 1.]).is_err());
        assert!(torus([f64::NAN, 0., 0.], 5., 1., 1.).is_err());
    }
    #[test]
    fn conic_rho_half_is_parabola_with_rho_shoulder() {
        // Symmetric tangents, rho = 1/2: polynomial parabola.
        let c = conic_rho([-1., 0., 0.], [1., 0., 0.], [1., 1., 0.], [-1., 1., 0.], 0.5).unwrap();
        assert_eq!(c.weights, vec![1., 1., 1.]);
        for i in 0..=20 {
            let u = i as f64 / 20.;
            let p = c.evaluate(u).unwrap().point;
            let x = 2. * u - 1.;
            assert!((p[0] - x).abs() < 1e-13);
            assert!((p[1] - (1. - x * x) / 2.).abs() < 1e-13);
            assert_eq!(p[2], 0.);
        }
        // Shoulder point: (1-rho)*chord midpoint + rho*tangent intersection.
        let shoulder = c.evaluate(0.5).unwrap().point;
        assert!((shoulder[0]).abs() < 1e-13 && (shoulder[1] - 0.5).abs() < 1e-13);
    }
    #[test]
    fn conic_rho_ellipse_matches_ellipse_arc_and_rejects_invalid() {
        // Quarter ellipse: w = cos(45°), rho = w/(1+w).
        let w = std::f64::consts::FRAC_1_SQRT_2;
        let rho = w / (1. + w);
        assert!(rho < 0.5);
        let c = conic_rho([2., 0., 0.], [0., 1., 0.], [0., 1., 0.], [-1., 0., 0.], rho).unwrap();
        let e = ellipse_arc([0.; 3], [2., 0., 0.], [0., 1., 0.], 0., 90.).unwrap();
        for i in 0..=20 {
            let u = i as f64 / 20.;
            let p = c.evaluate(u).unwrap().point;
            let q = e.evaluate(u).unwrap().point;
            for a in 0..3 {
                assert!((p[a] - q[a]).abs() < 1e-12);
            }
            assert!((p[0].powi(2) / 4. + p[1].powi(2) - 1.).abs() < 1e-12);
        }
        for bad_rho in [0., -0.5, 1., 1.5, f64::NAN] {
            assert!(
                conic_rho([0.; 3], [1., 0., 0.], [0., 1., 0.], [-1., 0., 0.], bad_rho).is_err()
            );
        }
        // Parallel tangents have no intersection.
        assert!(conic_rho([0.; 3], [1., 0., 0.], [0., 1., 0.], [0., 2., 0.], 0.5).is_err());
        assert!(conic_rho([0.; 3], [1., 0., 0.], [0., 0., 0.], [0., 1., 0.], 0.5).is_err());
        assert!(conic_rho([0.; 3], [0.; 3], [1., 0., 0.], [0., 1., 0.], 0.5).is_err());
    }
    #[test]
    fn conic_osculating_matches_point_tangent_and_signed_curvature() {
        for curvature in [2., -0.5, 7.] {
            let c = conic_osculating([3., -1., 2.], [0.6, 0.8, 0.], curvature).unwrap();
            let start = c.evaluate(0.).unwrap();
            for a in 0..3 {
                assert!((start.point[a] - [3., -1., 2.][a]).abs() < 1e-12);
            }
            let d1 = start.d1.unwrap();
            let d2 = start.d2.unwrap();
            let speed = d1.iter().map(|x| x * x).sum::<f64>().sqrt();
            // Unit tangent matches the requested direction.
            for a in 0..3 {
                assert!((d1[a] / speed - [0.6, 0.8, 0.][a]).abs() < 1e-12);
            }
            // Signed curvature from the jet: |d1 x d2| / |d1|^3, signed by normal side.
            let cross = [
                d1[1] * d2[2] - d1[2] * d2[1],
                d1[2] * d2[0] - d1[0] * d2[2],
                d1[0] * d2[1] - d1[1] * d2[0],
            ];
            let k = cross.iter().map(|x| x * x).sum::<f64>().sqrt() / speed.powi(3);
            assert!((k - curvature.abs()).abs() < 1e-9);
            // Center of curvature from the perpendicular acceleration component.
            let d2_perp: Vec<f64> = (0..3)
                .map(|a| d2[a] - d1[a] * d1.iter().zip(d2.iter()).map(|(x, y)| x * y).sum::<f64>() / speed.powi(2))
                .collect();
            let turn = d2_perp.iter().map(|x| x * x).sum::<f64>().sqrt();
            let center = (0..3)
                .map(|a| start.point[a] + d2_perp[a] / turn / curvature.abs())
                .collect::<Vec<_>>();
            for i in 0..=16 {
                let p = c.evaluate(i as f64 / 16.).unwrap().point;
                let r = (0..3)
                    .map(|a| (p[a] - center[a]).powi(2))
                    .sum::<f64>()
                    .sqrt();
                assert!((r - 1. / curvature.abs()).abs() < 1e-9);
            }
        }
        assert!(conic_osculating([0.; 3], [1., 0., 0.], 0.).is_err());
        assert!(conic_osculating([0.; 3], [1., 0., 0.], f64::NAN).is_err());
        assert!(conic_osculating([0.; 3], [0.; 3], 1.).is_err());
    }
}
