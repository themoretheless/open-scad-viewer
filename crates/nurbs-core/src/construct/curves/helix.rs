//! Cubic Hermite approximation of a cylindrical helix, with explicit real-arithmetic estimate.
use crate::{Result, check, curve::Curve};
#[cfg(feature = "codec")]
mod serialization;

pub struct Approximation {
    pub curve: Curve,
    pub spans: usize,
    pub budget: f64,
    pub real_arithmetic_error_estimate: f64,
}
/// P(t)=center+[r*cos(phase+2*pi*turns*t),r*sin(...),height*t], t in [0,1].
/// The budget applies to the real-arithmetic Hermite remainder estimate only.
/// Binary64 trig/control/evaluation rounding is not included or certified.
pub fn approximate(
    center: [f64; 3],
    radius: f64,
    height: f64,
    turns: f64,
    phase_degrees: f64,
    max_deviation: f64,
) -> Result<Approximation> {
    check(
        radius.is_finite() && radius > 0.,
        "Helix radius must be finite and positive",
    )?;
    tapered(
        center,
        [radius, radius],
        [radius, radius],
        height,
        turns,
        phase_degrees,
        max_deviation,
    )
}

/// Elliptical constant-radius helix; both authored XY radii are positive.
pub fn approximate_elliptic(
    center: [f64; 3],
    radius_x: f64,
    radius_y: f64,
    height: f64,
    turns: f64,
    phase_degrees: f64,
    max_deviation: f64,
) -> Result<Approximation> {
    check(
        radius_x.is_finite() && radius_x > 0. && radius_y.is_finite() && radius_y > 0.,
        "Elliptic helix radii must be finite and positive",
    )?;
    tapered(
        center,
        [radius_x, radius_y],
        [radius_x, radius_y],
        height,
        turns,
        phase_degrees,
        max_deviation,
    )
}

/// Conical helix with linearly varying nonnegative circular radius; zero-radius apex allowed.
pub fn approximate_conical(
    center: [f64; 3],
    start_radius: f64,
    end_radius: f64,
    height: f64,
    turns: f64,
    phase_degrees: f64,
    max_deviation: f64,
) -> Result<Approximation> {
    tapered(
        center,
        [start_radius, start_radius],
        [end_radius, end_radius],
        height,
        turns,
        phase_degrees,
        max_deviation,
    )
}

/// Constant circular radius and a cubic axial law constrained by endpoint pitch.
/// Pitch is signed axial distance per signed revolution: dz/dt = turns * pitch.
/// Height and endpoint pitches are independent; the interior may reverse direction.
/// XY uses the same uncertified Hermite remainder as `approximate`; cubic Z is
/// represented algebraically in the existing unit-weight spline basis.
pub fn approximate_variable_pitch(
    center: [f64; 3],
    radius: f64,
    height: f64,
    turns: f64,
    start_pitch: f64,
    end_pitch: f64,
    phase_degrees: f64,
    max_deviation: f64,
) -> Result<Approximation> {
    check(
        start_pitch.is_finite() && end_pitch.is_finite(),
        "Helix endpoint pitches must be finite",
    )?;
    let a = turns * start_pitch;
    let b = 3. * height - 2. * a - turns * end_pitch;
    let c = -2. * height + a + turns * end_pitch;
    check(
        [a, b, c].iter().all(|v| v.is_finite()),
        "Helix axial polynomial overflow",
    )?;
    let mut result = approximate(center, radius, 0., turns, phase_degrees, max_deviation)?;
    // Cubic blossom at the three successive knots gives the B-spline controls.
    for (i, point) in result.curve.control_points.iter_mut().enumerate() {
        let u = result.curve.knots[i + 1];
        let v = result.curve.knots[i + 2];
        let w = result.curve.knots[i + 3];
        point[2] =
            center[2] + a * ((u + v + w) / 3.) + b * ((u * v + u * w + v * w) / 3.) + c * u * v * w;
        check(point[2].is_finite(), "Helix axial control overflow")?;
    }
    result.curve.validate()?;
    Ok(result)
}

fn tapered(
    center: [f64; 3],
    start: [f64; 2],
    end: [f64; 2],
    height: f64,
    turns: f64,
    phase_degrees: f64,
    max_deviation: f64,
) -> Result<Approximation> {
    check(
        center.iter().all(|x| x.is_finite())
            && start.iter().chain(&end).all(|r| r.is_finite() && *r >= 0.)
            && start.iter().chain(&end).any(|r| *r > 0.)
            && height.is_finite()
            && turns.is_finite()
            && turns != 0.
            && phase_degrees.is_finite()
            && max_deviation.is_finite()
            && max_deviation > 0.,
        "Helix needs finite coordinates, positive radius/budget and nonzero turns",
    )?;
    let omega = 2. * std::f64::consts::PI * turns;
    check(omega.is_finite(), "Helix angular frequency overflow")?;
    let phase = phase_degrees.rem_euclid(360.).to_radians();
    let slope = [end[0] - start[0], end[1] - start[1]];
    // For linear radius: |fourth derivative| <= max(r)*omega^4+4*|r'|*|omega|^3.
    // Scalar Hermite remainder <= M*h^4/384; hypot covers both XY coordinates.
    let mut chosen = None;
    for spans in 1..=85 {
        let step = omega.abs() / spans as f64;
        let squared = step * step;
        let coordinate_bounds = std::array::from_fn::<_, 2, _>(|d| {
            (start[d].max(end[d]) * (squared * squared)
                + 4. * slope[d].abs() * squared * step / spans as f64)
                / 384.
        });
        let estimate = coordinate_bounds[0].hypot(coordinate_bounds[1]);
        if estimate.is_finite() && estimate > 0. && estimate <= max_deviation {
            chosen = Some((spans, estimate));
            break;
        }
    }
    let (spans, estimate) = chosen.ok_or_else(|| {
        crate::input("Helix error estimate is unrepresentable or needs more than 85 cubic spans")
    })?;
    let mut points = Vec::with_capacity(spans + 1);
    let mut tangents = Vec::with_capacity(spans + 1);
    let mut parameters = Vec::with_capacity(spans + 1);
    for i in 0..=spans {
        let t = i as f64 / spans as f64;
        let angle = phase + omega * t;
        let (sin, cos) = angle.sin_cos();
        let radius = [
            (1. - t) * start[0] + t * end[0],
            (1. - t) * start[1] + t * end[1],
        ];
        points.push([
            center[0] + radius[0] * cos,
            center[1] + radius[1] * sin,
            center[2] + height * t,
        ]);
        tangents.push([
            slope[0] * cos - radius[0] * omega * sin,
            slope[1] * sin + radius[1] * omega * cos,
            height,
        ]);
        parameters.push(t);
    }
    let curve = crate::hermite::interpolate_inferred(&points, &tangents, &parameters)?;
    Ok(Approximation {
        curve,
        spans,
        budget: max_deviation,
        real_arithmetic_error_estimate: estimate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "codec")]
    use value_codec::json;
    #[test]
    #[cfg(feature = "codec")]
    fn sampled_actual_error_stays_below_real_arithmetic_estimate() {
        for turns in [1., -2.5] {
            let a = approximate([3., 4., 5.], 2., -7., turns, 37., 1e-4).unwrap();
            assert!(a.real_arithmetic_error_estimate <= a.budget);
            for i in 0..=1000 {
                let t = i as f64 / 1000.;
                let angle = 37_f64.to_radians() + 2. * std::f64::consts::PI * turns * t;
                let p = a.curve.evaluate(t).unwrap().point;
                let exact = [3. + 2. * angle.cos(), 4. + 2. * angle.sin(), 5. - 7. * t];
                let distance = (0..3)
                    .map(|d| (p[d] - exact[d]).powi(2))
                    .sum::<f64>()
                    .sqrt();
                assert!(distance <= a.real_arithmetic_error_estimate + 1e-12);
            }
        }
    }
    #[test]
    fn tighter_budget_increases_spans_and_rejects_unrepresentable_requests() {
        let a = approximate([0.; 3], 2., 10., 1., 0., 1e-2).unwrap();
        let b = approximate([0.; 3], 2., 10., 1., 0., 1e-5).unwrap();
        assert!(b.spans > a.spans);
        assert!(approximate([0.; 3], 2., 10., 100., 0., 1e-8).is_err());
        assert!(approximate([0.; 3], 2., 10., 0., 0., 1e-3).is_err());
        assert!(approximate([0.; 3], 2., 10., 1., 0., 0.).is_err());
        #[cfg(feature = "codec")]
        {
            let report = value_codec::to_value(a).unwrap();
            assert_eq!(report["report"]["roundingCertified"], json!(false));
        }
    }
    #[cfg(feature = "transport")]
    #[test]
    fn json_dispatch_preserves_physical_inputs_and_uncertified_report() {
        let value=crate::transport::dispatch(json!({"op":"curve_helix","center":[3.,4.,5.],"radius":2.,"height":6.,"turns":0.25,"phase_degrees":0.,"max_deviation":1e-4})).unwrap();
        let curve: Curve = value_codec::from_value(value["curve"].clone()).unwrap();
        let p = curve.evaluate(1.).unwrap().point;
        for (actual, expected) in p.iter().zip([3., 6., 11.]) {
            assert!((actual - expected).abs() < 1e-12);
        }
        assert_eq!(value["report"]["continuousBound"], json!(false));
    }
    #[test]
    fn elliptic_helix_matches_independent_anisotropic_formula() {
        let a = approximate_elliptic([3., 4., 5.], 2., 4., -7., -1.5, 37., 1e-4).unwrap();
        for i in 0..=1000 {
            let t = i as f64 / 1000.;
            let angle = 37_f64.to_radians() - 3. * std::f64::consts::PI * t;
            let p = a.curve.evaluate(t).unwrap().point;
            let exact = [3. + 2. * angle.cos(), 4. + 4. * angle.sin(), 5. - 7. * t];
            let distance = (0..3)
                .map(|d| (p[d] - exact[d]).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!(distance <= a.real_arithmetic_error_estimate + 1e-12);
        }
        assert!(approximate_elliptic([0.; 3], 0., 2., 1., 1., 0., 1e-3).is_err());
    }
    #[test]
    fn conical_helix_includes_radius_derivative_in_the_remainder_estimate() {
        for (start, end) in [(1., 5.), (5., 0.)] {
            let a = approximate_conical([3., 4., 5.], start, end, -9., 1., 23., 1e-4).unwrap();
            for i in 0..=1000 {
                let t = i as f64 / 1000.;
                let angle = 23_f64.to_radians() + 2. * std::f64::consts::PI * t;
                let radius = start + (end - start) * t;
                let p = a.curve.evaluate(t).unwrap().point;
                let exact = [
                    3. + radius * angle.cos(),
                    4. + radius * angle.sin(),
                    5. - 9. * t,
                ];
                let distance = (0..3)
                    .map(|d| (p[d] - exact[d]).powi(2))
                    .sum::<f64>()
                    .sqrt();
                assert!(distance <= a.real_arithmetic_error_estimate + 1e-12);
            }
        }
        assert!(approximate_conical([0.; 3], 0., 0., 1., 1., 0., 1e-3).is_err());
        assert!(approximate_conical([0.; 3], -1., 2., 1., 1., 0., 1e-3).is_err());
    }
    #[test]
    fn variable_pitch_matches_cubic_axis_and_endpoint_derivatives() {
        for turns in [0.75, -0.75] {
            let h = 7.;
            let m0 = turns * 2.;
            let m1 = turns * 11.;
            let a =
                approximate_variable_pitch([3., 4., 5.], 2., h, turns, 2., 11., 23., 1e-4).unwrap();
            for i in 0..=1000 {
                let t = i as f64 / 1000.;
                let angle = 23_f64.to_radians() + 2. * std::f64::consts::PI * turns * t;
                let z = (t * t * t - 2. * t * t + t) * m0
                    + (-2. * t * t * t + 3. * t * t) * h
                    + (t * t * t - t * t) * m1;
                let p = a.curve.evaluate(t).unwrap().point;
                let exact = [3. + 2. * angle.cos(), 4. + 2. * angle.sin(), 5. + z];
                let distance = (0..3)
                    .map(|d| (p[d] - exact[d]).powi(2))
                    .sum::<f64>()
                    .sqrt();
                assert!(distance <= a.real_arithmetic_error_estimate + 1e-12);
            }
            assert!((a.curve.evaluate(0.).unwrap().d1.unwrap()[2] - m0).abs() < 1e-11);
            assert!((a.curve.evaluate(1.).unwrap().d1.unwrap()[2] - m1).abs() < 1e-11);
        }
        assert!(approximate_variable_pitch([0.; 3], 2., 1., 1., f64::NAN, 2., 0., 1e-3).is_err());
        assert!(approximate_variable_pitch([0.; 3], 2., 1., 2., f64::MAX, 2., 0., 1e-3).is_err());
    }
    #[cfg(feature = "transport")]
    #[test]
    fn variable_pitch_json_preserves_height_pitches_and_report() {
        let value=crate::transport::dispatch(json!({"op":"curve_variable_pitch_helix","center":[0.,0.,0.],"radius":2.,"height":3.,"turns":0.25,"start_pitch":2.,"end_pitch":7.,"phase_degrees":0.,"max_deviation":1e-4})).unwrap();
        let curve: Curve = value_codec::from_value(value["curve"].clone()).unwrap();
        assert!((curve.evaluate(1.).unwrap().point[2] - 3.).abs() < 1e-12);
        assert_eq!(value["report"]["continuousBound"], json!(false));
    }
}
