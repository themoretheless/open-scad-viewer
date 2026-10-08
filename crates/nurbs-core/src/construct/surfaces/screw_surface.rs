//! Approximate constant-pitch screw motion of a retained rational profile.
use crate::{Result, check, curve::Curve, surface::Surface};
pub struct Approximation {
    pub patches: Vec<Surface>,
    pub spans: usize,
    pub budget: f64,
    pub real_arithmetic_error_estimate: f64,
}
#[cfg(feature = "codec")]
impl value_codec::Serialize for Approximation {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({"patches":self.patches,"report":{
            "spans":self.spans,"budget":self.budget,
            "realArithmeticErrorEstimate":self.real_arithmetic_error_estimate,
            "continuousBound":false,"roundingCertified":false,
            "method":"profile-screw-helix-Hermite"}})
    }
}
/// Rotate P(u)-origin by phase+TAU*turns*v about axis, then translate by
/// height*v along that axis. Height/turns are signed; height may be zero.
/// Preserves profile U and normalized V subdomains. No solid sewing or RMF.
pub fn approximate(
    profile: &Curve,
    origin: [f64; 3],
    axis: [f64; 3],
    height: f64,
    turns: f64,
    phase_degrees: f64,
    budget: f64,
) -> Result<Approximation> {
    profile.validate()?;
    check(
        profile.control_points[0].len() == 3 && origin.iter().chain(&axis).all(|x| x.is_finite()),
        "Screw surface requires a 3D profile and finite origin/axis",
    )?;
    let maximum = axis.iter().fold(0_f64, |a, x| a.max(x.abs()));
    check(maximum > 0., "Screw axis must be nonzero")?;
    let mut n = axis.map(|x| x / maximum);
    let length = n[0].hypot(n[1]).hypot(n[2]);
    n = n.map(|x| x / length);
    let mut radius = 0_f64;
    for point in &profile.control_points {
        let relative = std::array::from_fn::<_, 3, _>(|d| point[d] - origin[d]);
        let dot = (0..3).map(|d| relative[d] * n[d]).sum::<f64>();
        let radial = std::array::from_fn::<_, 3, _>(|d| relative[d] - dot * n[d]);
        let r = radial[0].hypot(radial[1]).hypot(radial[2]);
        check(r.is_finite(), "Screw radial hull is unrepresentable")?;
        radius = radius.max(r);
    }
    check(
        radius > 0.,
        "Screw profile must contain an off-axis control",
    )?;
    let fit = crate::helix::approximate([0.; 3], radius, height, turns, phase_degrees, budget)?;
    let u_parts = parts(profile)?;
    let v_parts = parts(&fit.curve)?;
    check(
        u_parts.len().saturating_mul(v_parts.len()) <= 2048,
        "Screw surface exceeds 2048 retained patches",
    )?;
    let mut patches = Vec::new();
    for u in &u_parts {
        for v in &v_parts {
            let mut points = Vec::new();
            let mut weights = Vec::new();
            for (i, point) in u.control_points.iter().enumerate() {
                let relative = std::array::from_fn::<_, 3, _>(|d| point[d] - origin[d]);
                let dot = (0..3).map(|d| relative[d] * n[d]).sum::<f64>();
                let parallel = n.map(|x| x * dot);
                let tangent = [
                    n[1] * relative[2] - n[2] * relative[1],
                    n[2] * relative[0] - n[0] * relative[2],
                    n[0] * relative[1] - n[1] * relative[0],
                ];
                let mut row = Vec::new();
                let mut row_weights = Vec::new();
                for (j, angle) in v.control_points.iter().enumerate() {
                    let c = angle[0] / radius;
                    let s = angle[1] / radius;
                    row.push(
                        (0..3)
                            .map(|d| {
                                origin[d]
                                    + parallel[d]
                                    + c * (relative[d] - parallel[d])
                                    + s * tangent[d]
                                    + n[d] * angle[2]
                            })
                            .collect(),
                    );
                    row_weights.push(u.weights[i] * v.weights[j]);
                }
                points.push(row);
                weights.push(row_weights);
            }
            let surface = Surface {
                degree_u: u.degree,
                degree_v: v.degree,
                knots_u: u.knots.clone(),
                knots_v: v.knots.clone(),
                control_points: points,
                weights,
                periodic_u: u.periodic,
                periodic_v: false,
            };
            surface.validate()?;
            patches.push(surface);
        }
    }
    Ok(Approximation {
        patches,
        spans: fit.spans,
        budget,
        real_arithmetic_error_estimate: fit.real_arithmetic_error_estimate,
    })
}
fn parts(curve: &Curve) -> Result<Vec<Curve>> {
    if curve.control_points.len() <= 32 {
        return Ok(vec![curve.clone()]);
    }
    check(
        !curve.periodic,
        "Dense screw curves need explicit clamped encoding",
    )?;
    curve
        .decompose()
        .map(|segments| segments.iter().map(|s| s.definition().clone()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_rational_profile_rotation_signed_pitch_and_dense_domains() {
        let mut profile = crate::primitives::line([1., 0., 0.], [2., 0., 1.]).unwrap();
        profile.weights = vec![1., 2.];
        for (turns, height) in [(1., 4.), (-0.5, -3.)] {
            let fit =
                approximate(&profile, [0.; 3], [0., 0., 1.], height, turns, 23., 1e-4).unwrap();
            if turns == 1. {
                assert!(fit.patches.len() > 1);
            }
            for patch in &fit.patches {
                let lo = patch.knots_v[0];
                let hi = *patch.knots_v.last().unwrap();
                for u in [0., 0.13, 0.5, 0.87, 1.] {
                    for t in [0., 0.17, 0.5, 0.83, 1.] {
                        let v = lo + (hi - lo) * t;
                        let p = profile.evaluate(u).unwrap().point;
                        let angle = 23_f64.to_radians() + std::f64::consts::TAU * turns * v;
                        let q = patch.evaluate(u, v).unwrap().point;
                        let e = (q[0] - p[0] * angle.cos())
                            .hypot(q[1] - p[0] * angle.sin())
                            .hypot(q[2] - p[2] - height * v);
                        assert!(e <= fit.real_arithmetic_error_estimate + 1e-11);
                    }
                }
            }
        }
    }
    #[test]
    fn dense_profile_and_motion_split_both_parameter_axes() {
        let points: Vec<_> = (0..34)
            .map(|i| [1. + i as f64 * 0.01, 0., i as f64 * 0.02])
            .collect();
        let mut profile = crate::primitives::polyline(&points, false).unwrap();
        profile.weights = (0..34).map(|i| 1. + (i % 3) as f64).collect();
        let fit = approximate(&profile, [0.; 3], [0., 0., 1.], 1., 1., 0., 1e-4).unwrap();
        assert_eq!(fit.patches.len(), 33 * fit.spans);
        for patch in &fit.patches {
            let u = (patch.knots_u[0] + patch.knots_u.last().unwrap()) / 2.;
            let v = (patch.knots_v[0] + patch.knots_v.last().unwrap()) / 2.;
            let p = profile.evaluate(u).unwrap().point;
            let q = patch.evaluate(u, v).unwrap().point;
            let angle = std::f64::consts::TAU * v;
            let e = (q[0] - p[0] * angle.cos())
                .hypot(q[1] - p[0] * angle.sin())
                .hypot(q[2] - p[2] - v);
            assert!(e <= fit.real_arithmetic_error_estimate + 1e-11);
        }
    }
    #[test]
    fn arbitrary_axis_origin_and_zero_height_rotation() {
        let profile = crate::primitives::line([10., 21., 30.], [10., 22., 30.]).unwrap();
        let fit = approximate(
            &profile,
            [10., 20., 30.],
            [1e300, 0., 0.],
            0.,
            0.25,
            0.,
            1e-4,
        )
        .unwrap();
        let patch = fit.patches.last().unwrap();
        let q = patch.evaluate(0.5, 1.).unwrap().point;
        assert!((q[0] - 10.).abs() < 1e-11);
        assert!((q[1] - 20.).abs() < 1e-11);
        assert!((q[2] - 31.5).abs() < 1e-11);
        let oblique = crate::primitives::line([1., -1., 0.], [2., -2., 0.]).unwrap();
        let fit = approximate(
            &oblique,
            [0.; 3],
            [1., 1., 0.],
            std::f64::consts::SQRT_2,
            0.25,
            0.,
            1e-4,
        )
        .unwrap();
        let q = fit.patches.last().unwrap().evaluate(0.5, 1.).unwrap().point;
        assert!((q[0] - 1.).abs() < 1e-11);
        assert!((q[1] - 1.).abs() < 1e-11);
        assert!((q[2] + 1.5 * std::f64::consts::SQRT_2).abs() < 1e-11);
        assert!(approximate(&profile, [10., 20., 30.], [0.; 3], 1., 1., 0., 1e-4).is_err());
    }
}
