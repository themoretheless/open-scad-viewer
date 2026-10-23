//! Two-angle spherical trajectory with an explicit ideal Hermite remainder.
pub use crate::helix::Approximation;
use crate::{Result, check};

/// P(t)=center+r*[cos(B)*cos(A),cos(B)*sin(A),sin(B)], t in [0,1].
/// A/B advance linearly by signed longitude/latitude turns; phases use degrees.
/// Pole crossings are allowed. This is not an equal-area or geodesic solver.
pub fn approximate(
    center: [f64; 3],
    radius: f64,
    longitude_turns: f64,
    latitude_turns: f64,
    longitude_phase_degrees: f64,
    latitude_phase_degrees: f64,
    budget: f64,
) -> Result<Approximation> {
    check(
        center.iter().all(|x| x.is_finite())
            && radius.is_finite()
            && radius > 0.
            && [
                longitude_turns,
                latitude_turns,
                longitude_phase_degrees,
                latitude_phase_degrees,
                budget,
            ]
            .iter()
            .all(|x| x.is_finite())
            && longitude_turns != 0.
            && latitude_turns != 0.
            && budget > 0.,
        "Spherical spiral needs finite center, positive radius/budget and nonzero turns",
    )?;
    let a = std::f64::consts::TAU * longitude_turns;
    let b = std::f64::consts::TAU * latitude_turns;
    check(
        [a, b, a + b, a - b].iter().all(|x| x.is_finite()),
        "Spherical frequency overflow",
    )?;
    let mut choice = None;
    for spans in 1..=85 {
        let n = spans as f64;
        let fourth = |w: f64| {
            let s = w / n;
            let s2 = s * s;
            s2 * s2
        };
        // Product-to-sum gives XY harmonics at a+b and a-b; Z is at b.
        let xy = radius / 2. * (fourth(a + b) + fourth(a - b));
        let z = radius * fourth(b);
        let estimate = xy.hypot(xy).hypot(z) / 384.;
        if estimate.is_finite() && estimate > 0. && estimate <= budget {
            choice = Some((spans, estimate));
            break;
        }
    }
    let (spans, estimate) = choice.ok_or_else(|| {
        crate::input("Spherical remainder is unrepresentable or exceeds 85 cubic spans")
    })?;
    let phase_a = longitude_phase_degrees.rem_euclid(360.).to_radians();
    let phase_b = latitude_phase_degrees.rem_euclid(360.).to_radians();
    let mut points = Vec::new();
    let mut tangents = Vec::new();
    let mut parameters = Vec::new();
    for i in 0..=spans {
        let t = i as f64 / spans as f64;
        let (sa, ca) = (phase_a + a * t).sin_cos();
        let (sb, cb) = (phase_b + b * t).sin_cos();
        points.push([
            center[0] + radius * cb * ca,
            center[1] + radius * cb * sa,
            center[2] + radius * sb,
        ]);
        tangents.push([
            radius * (-b * sb * ca - a * cb * sa),
            radius * (-b * sb * sa + a * cb * ca),
            radius * b * cb,
        ]);
        parameters.push(t);
    }
    let curve = crate::hermite::interpolate_inferred(&points, &tangents, &parameters)?;
    Ok(Approximation {
        curve,
        spans,
        budget,
        real_arithmetic_error_estimate: estimate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_formula_sphere_radius_pole_crossings_and_endpoint_jets() {
        for (p, q, phase) in [(1., 0.5, -90.), (-0.75, 0.25, 23.), (0.5, -0.75, 47.)] {
            let fit = approximate([3., 4., 5.], 2., p, q, 31., phase, 1e-4).unwrap();
            for i in 0..=1000 {
                let t = i as f64 / 1000.;
                let a = 31_f64.to_radians() + std::f64::consts::TAU * p * t;
                let b = phase.to_radians() + std::f64::consts::TAU * q * t;
                let exact = [
                    3. + 2. * b.cos() * a.cos(),
                    4. + 2. * b.cos() * a.sin(),
                    5. + 2. * b.sin(),
                ];
                let point = fit.curve.evaluate(t).unwrap().point;
                let error = (point[0] - exact[0])
                    .hypot(point[1] - exact[1])
                    .hypot(point[2] - exact[2]);
                assert!(error <= fit.real_arithmetic_error_estimate + 1e-12);
                let radial = (point[0] - 3.).hypot(point[1] - 4.).hypot(point[2] - 5.);
                assert!((radial - 2.).abs() <= fit.real_arithmetic_error_estimate + 1e-12);
            }
            for t in [0., 1.] {
                let a = 31_f64.to_radians() + std::f64::consts::TAU * p * t;
                let b = phase.to_radians() + std::f64::consts::TAU * q * t;
                let da = std::f64::consts::TAU * p;
                let db = std::f64::consts::TAU * q;
                let expected = [
                    2. * (-db * b.sin() * a.cos() - da * b.cos() * a.sin()),
                    2. * (-db * b.sin() * a.sin() + da * b.cos() * a.cos()),
                    2. * db * b.cos(),
                ];
                let d = fit.curve.evaluate(t).unwrap().d1.unwrap();
                for k in 0..3 {
                    assert!((d[k] - expected[k]).abs() < 1e-9);
                }
            }
        }
    }
    #[test]
    fn refuses_invalid_and_unattainable_requests_without_relaxing_budget() {
        assert!(approximate([0.; 3], 0., 1., 0.5, 0., 0., 1e-4).is_err());
        assert!(approximate([0.; 3], 2., 0., 0.5, 0., 0., 1e-4).is_err());
        assert!(approximate([0.; 3], 2., 1., 0., 0., 0., 1e-4).is_err());
        assert!(approximate([0.; 3], 2., 10., 5., 0., 0., 1e-12).is_err());
    }
}
