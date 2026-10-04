//! Compact rational conics and surfaces of revolution. Angles are in degrees.
//! Exact rational constructions in real arithmetic; binary64 rounding remains.
use crate::{
    Result, check,
    curve::Curve,
    surface::{Surface, revolve},
};
use std::f64::consts::PI;

/// Affine ellipse arc: center + axis_u*cos(theta) + axis_v*sin(theta).
/// Axes are radius vectors and need not be orthogonal. Domain is [0, 1].
/// Full turns have coincident endpoints but use a clamped, non-periodic basis.
pub fn ellipse_arc(
    center: [f64; 3],
    axis_u: [f64; 3],
    axis_v: [f64; 3],
    start: f64,
    sweep: f64,
) -> Result<Curve> {
    check(
        center
            .iter()
            .chain(&axis_u)
            .chain(&axis_v)
            .chain([start, sweep].iter())
            .all(|x| x.is_finite()),
        "Ellipse data must be finite",
    )?;
    check(
        sweep != 0. && sweep.abs() <= 360.,
        "Ellipse sweep must be nonzero and within +/-360 degrees",
    )?;
    let scale_u = axis_u.iter().fold(0_f64, |a, x| a.max(x.abs()));
    let scale_v = axis_v.iter().fold(0_f64, |a, x| a.max(x.abs()));
    check(scale_u > 0. && scale_v > 0., "Ellipse axes must be nonzero")?;
    let u = axis_u.map(|x| x / scale_u);
    let v = axis_v.map(|x| x / scale_v);
    let cross = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    check(
        cross.iter().any(|x| x.abs() > 1e-14),
        "Ellipse axes must be independent and numerically well-conditioned",
    )?;
    let arcs = (sweep.abs() / 90.).ceil() as usize;
    let delta = sweep * PI / 180. / arcs as f64;
    let initial = start.rem_euclid(360.) * PI / 180.;
    let mut points: Vec<Vec<f64>> = Vec::with_capacity(2 * arcs + 1);
    let mut weights = Vec::with_capacity(2 * arcs + 1);
    for i in 0..=2 * arcs {
        let w = if i % 2 == 0 { 1. } else { (delta / 2.).cos() };
        let theta = initial + i as f64 * delta / 2.;
        let point = if i == 2 * arcs && sweep.abs() == 360. {
            points[0].clone()
        } else {
            (0..3)
                .map(|a| center[a] + (axis_u[a] * theta.cos() + axis_v[a] * theta.sin()) / w)
                .collect()
        };
        points.push(point);
        weights.push(w);
    }
    let mut knots = vec![0.; 3];
    for i in 1..arcs {
        knots.extend([i as f64 / arcs as f64; 2]);
    }
    knots.extend([1.; 3]);
    let result = Curve {
        degree: 2,
        knots,
        control_points: points,
        weights,
        periodic: false,
    };
    result.validate()?;
    Ok(result)
}

fn radii(values: &[f64]) -> Result<()> {
    check(
        values.iter().all(|v| v.is_finite() && *v > 0.),
        "Radii must be finite and positive",
    )
}

/// Axis-aligned ellipsoid. The two poles are intentional parameter singularities.
/// Returns a surface, not a certified solid or trimmed B-rep.
pub fn ellipsoid(center: [f64; 3], radius: [f64; 3]) -> Result<Surface> {
    radii(&radius)?;
    let profile = ellipse_arc(
        [0.; 3],
        [radius[0], 0., 0.],
        [0., 0., radius[2]],
        -90.,
        180.,
    )?;
    let mut result = revolve(&profile, [0.; 3], [0., 0., 1.], 360.)?;
    // Materialize exact poles instead of retaining cos(pi/2) roundoff.
    let last = result.control_points.len() - 1;
    for row in [0, last] {
        for p in &mut result.control_points[row] {
            p[0] = 0.;
            p[1] = 0.;
        }
    }
    for row in &mut result.control_points {
        for p in row {
            p[1] = (p[1] / radius[0]) * radius[1];
            for a in 0..3 {
                p[a] += center[a];
            }
        }
    }
    result.validate()?;
    Ok(result)
}

/// Ring torus about Z with an elliptical tube. Horn/spindle tori are rejected.
pub fn torus(center: [f64; 3], major: f64, radial: f64, axial: f64) -> Result<Surface> {
    radii(&[major, radial, axial])?;
    check(
        major > radial,
        "Ring torus requires major radius greater than radial tube radius",
    )?;
    let profile = ellipse_arc([major, 0., 0.], [radial, 0., 0.], [0., 0., axial], 0., 360.)?;
    let mut result = revolve(&profile, [0.; 3], [0., 0., 1.], 360.)?;
    for row in &mut result.control_points {
        for p in row {
            for a in 0..3 {
                p[a] += center[a];
            }
        }
    }
    result.validate()?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
