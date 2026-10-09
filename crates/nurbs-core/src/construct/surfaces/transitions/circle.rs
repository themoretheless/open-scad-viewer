//! Ruled circle-to-circle transition with explicitly authored seam directions.
use crate::{Result, check, curve::Curve, surface::Surface};
#[derive(Clone, Debug)]
pub struct CircleSection {
    pub center: [f64; 3],
    pub normal: [f64; 3],
    pub seam: [f64; 3],
    pub radius: f64,
}
fn unit(v: [f64; 3]) -> Result<[f64; 3]> {
    check(
        v.iter().all(|x| x.is_finite()),
        "Circle frame direction must be finite",
    )?;
    let scale = v.iter().fold(0_f64, |a, x| a.max(x.abs()));
    check(scale > 0., "Circle frame direction must be nonzero")?;
    let v = v.map(|x| x / scale);
    let length = v[0].hypot(v[1]).hypot(v[2]);
    Ok(v.map(|x| x / length))
}
pub(crate) fn profile(section: &CircleSection) -> Result<Curve> {
    check(
        section.radius.is_finite() && section.radius > 0.,
        "Circle transition radius must be positive and finite",
    )?;
    let n = unit(section.normal)?;
    let seam = unit(section.seam)?;
    let dot = seam.iter().zip(n).map(|(x, y)| x * y).sum::<f64>();
    let projection: [f64; 3] = std::array::from_fn(|k| seam[k] - dot * n[k]);
    let length = projection[0].hypot(projection[1]).hypot(projection[2]);
    check(
        length > 1e-12,
        "Circle seam direction must not be parallel to normal",
    )?;
    let u = projection.map(|x| x / length);
    let v = math_core::cross(n, u);
    crate::primitives::ellipse_arc(
        section.center,
        u.map(|x| x * section.radius),
        v.map(|x| x * section.radius),
        0.,
        360.,
    )
}
/// S(u,v)=(1-v)A(u)+vB(u), with identical rational angular parameterization.
/// U/V=[0,1]. Authored seams choose correspondence, not shortest twist.
/// Boundaries are circles in real arithmetic; no G1/G2 blend, caps, sewing,
/// regularity, self-intersection or rounding-inclusive certificate is implied.
pub fn ruled(start: &CircleSection, end: &CircleSection) -> Result<Surface> {
    let a = profile(start)?;
    let b = profile(end)?;
    super::ruled_loft_two(a, b, "Circle transition endpoints must differ")
}
#[cfg(test)]
mod tests {
    use super::*;
    fn section(center: [f64; 3], normal: [f64; 3], seam: [f64; 3], radius: f64) -> CircleSection {
        CircleSection {
            center,
            normal,
            seam,
            radius,
        }
    }
    fn angular(u: f64) -> [f64; 2] {
        let t = u * 4.;
        let quadrant = (t.floor() as usize).min(3);
        let t = t - quadrant as f64;
        let h = std::f64::consts::FRAC_1_SQRT_2;
        let a = (1. - t).powi(2);
        let b = 2. * h * t * (1. - t);
        let c = t * t;
        let x = (a + b) / (a + b + c);
        let y = (b + c) / (a + b + c);
        match quadrant {
            0 => [x, y],
            1 => [-y, x],
            2 => [-x, -y],
            _ => [y, -x],
        }
    }
    #[test]
    fn coaxial_cylinder_cone_and_translated_circles_match_independent_equation() {
        for end_radius in [1., 3.] {
            let a = section([3., 4., 5.], [0., 0., 1.], [1., 0., 2.], 1.);
            let b = section([4., 6., 12.], [0., 0., 1.], [1., 0., 0.], end_radius);
            let s = ruled(&a, &b).unwrap();
            assert_eq!(s.degree_u, 2);
            assert_eq!(s.degree_v, 1);
            assert_eq!(s.control_points.len(), 9);
            for i in 0..=1000 {
                let u = i as f64 / 1000.;
                let q = angular(u);
                for v in [0., 0.13, 0.5, 0.87, 1.] {
                    let r = 1. + v * (end_radius - 1.);
                    let p = s.evaluate(u, v).unwrap().point;
                    let expected = [3. + v + r * q[0], 4. + 2. * v + r * q[1], 5. + 7. * v];
                    assert!(p.iter().zip(expected).all(|(x, y)| (x - y).abs() < 1e-11));
                }
            }
        }
    }
    #[test]
    fn orthogonal_circle_planes_preserve_authored_correspondence_and_seam() {
        let a = section([0.; 3], [0., 0., 1.], [1., 0., 0.], 2.);
        let b = section([3., 4., 5.], [1., 0., 0.], [0., 1., 0.], 3.);
        let s = ruled(&a, &b).unwrap();
        for i in 0..=1000 {
            let u = i as f64 / 1000.;
            let [x, y] = angular(u);
            for v in [0., 0.13, 0.5, 0.87, 1.] {
                let p = s.evaluate(u, v).unwrap().point;
                let expected = [
                    (1. - v) * 2. * x + v * 3.,
                    (1. - v) * 2. * y + v * (4. + 3. * x),
                    v * (5. + 3. * y),
                ];
                assert!(p.iter().zip(expected).all(|(x, y)| (x - y).abs() < 1e-11));
            }
        }
        assert_eq!(s.control_points.first(), s.control_points.last());
    }
    #[test]
    fn rejects_invalid_frames_radii_and_coincident_profiles() {
        let a = section([0.; 3], [0., 0., 1.], [1., 0., 0.], 1.);
        assert!(ruled(&a, &a).is_err());
        for b in [
            section([0., 0., 1.], [0.; 3], [1., 0., 0.], 1.),
            section([0., 0., 1.], [0., 0., 1.], [0., 0., 2.], 1.),
            section([0., 0., 1.], [0., 0., 1.], [1., 0., 0.], 0.),
            section([f64::NAN, 0., 1.], [0., 0., 1.], [1., 0., 0.], 1.),
        ] {
            assert!(ruled(&a, &b).is_err());
        }
    }
}
