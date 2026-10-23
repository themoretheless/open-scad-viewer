//! Linear-curvature planar trajectory with separate ideal quadrature/fit estimates.
use crate::{Result, check};
pub struct Approximation {
    pub fit: crate::helix::Approximation,
    pub quadrature_intervals: usize,
    pub quadrature_error_estimate: f64,
    pub hermite_error_estimate: f64,
}
#[cfg(feature = "codec")]
impl value_codec::Serialize for Approximation {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({"curve":self.fit.curve,"report":{
            "spans":self.fit.spans,"budget":self.fit.budget,
            "realArithmeticErrorEstimate":self.fit.real_arithmetic_error_estimate,
            "quadratureIntervals":self.quadrature_intervals,
            "quadratureErrorEstimate":self.quadrature_error_estimate,
            "hermiteErrorEstimate":self.hermite_error_estimate,
            "continuousBound":false,"roundingCertified":false,
            "method":"clothoid-Hermite-composite-Simpson"}})
    }
}
/// Arc length L>0, signed endpoint curvatures in inverse model lengths.
/// theta(t)=phase+L*k0*t+L*(k1-k0)*t^2/2, t in [0,1].
/// P(t)=center+L*integral_0^t [cos(theta),sin(theta),0] du.
/// Estimates exclude binary64 arithmetic, trig and accumulation rounding.
pub fn approximate(
    center: [f64; 3],
    length: f64,
    start_curvature: f64,
    end_curvature: f64,
    phase_degrees: f64,
    budget: f64,
) -> Result<Approximation> {
    check(
        center.iter().all(|x| x.is_finite())
            && length.is_finite()
            && length > 0.
            && [start_curvature, end_curvature, phase_degrees, budget]
                .iter()
                .all(|x| x.is_finite())
            && budget > 0.,
        "Clothoid needs finite inputs, positive length and budget",
    )?;
    let a = length * start_curvature;
    let b = length * (end_curvature - start_curvature);
    let w = a.abs().max((a + b).abs());
    check(
        [a, b, w].iter().all(|x| x.is_finite()),
        "Clothoid angular coefficients overflow",
    )?;
    let phase = phase_degrees.rem_euclid(360.).to_radians();
    let straight = w == 0. && b == 0.;
    let mut quadrature = None;
    for n in [2usize, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096] {
        let n4 = (n as f64).powi(4);
        let m = w.powi(4) + 6. * w * w * b.abs() + 3. * b * b;
        let e = std::f64::consts::SQRT_2 * length * m / (180. * n4);
        if e.is_finite() && (e > 0. || straight) && e <= budget / 2. {
            quadrature = Some((n, e));
            break;
        }
    }
    let (intervals, qe) = quadrature.ok_or_else(|| {
        crate::input("Clothoid quadrature estimate exceeds 4096 intervals or is unrepresentable")
    })?;
    let mut choice = None;
    for spans in 1..=85 {
        let m = length * w.powi(3).hypot(3. * w * b.abs());
        let e = std::f64::consts::SQRT_2 * m / (384. * (spans as f64).powi(4));
        if e.is_finite() && (e > 0. || straight) && e + qe <= budget {
            choice = Some((spans, e));
            break;
        }
    }
    let (spans, he) = choice.ok_or_else(|| {
        crate::input("Clothoid fit estimate exceeds 85 cubic spans or is unrepresentable")
    })?;
    let mut points = Vec::new();
    let mut tangents = Vec::new();
    let mut parameters = Vec::new();
    for i in 0..=spans {
        let t = i as f64 / spans as f64;
        let h = t / intervals as f64;
        let mut sum = [0.; 2];
        for j in 0..=intervals {
            let u = j as f64 * h;
            let theta = phase + a * u + 0.5 * b * u * u;
            let (sin, cos) = theta.sin_cos();
            let coefficient = if j == 0 || j == intervals {
                1.
            } else if j % 2 == 0 {
                2.
            } else {
                4.
            };
            sum[0] += coefficient * cos;
            sum[1] += coefficient * sin;
        }
        points.push([
            center[0] + length * h * sum[0] / 3.,
            center[1] + length * h * sum[1] / 3.,
            center[2],
        ]);
        let (sin, cos) = (phase + a * t + 0.5 * b * t * t).sin_cos();
        tangents.push([length * cos, length * sin, 0.]);
        parameters.push(t);
    }
    // Cubic Hermite position basis functions are nonnegative and sum to1:
    // endpoint quadrature errors contribute at most qe; exact jets add none.
    let curve = crate::hermite::interpolate_inferred(&points, &tangents, &parameters)?;
    Ok(Approximation {
        fit: crate::helix::Approximation {
            curve,
            spans,
            budget,
            real_arithmetic_error_estimate: qe + he,
        },
        quadrature_intervals: intervals,
        quadrature_error_estimate: qe,
        hermite_error_estimate: he,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    // Independent analytic power series for integral exp(i*s^2/2) ds.
    fn euler(s: f64) -> [f64; 2] {
        let mut result = [0.; 2];
        let mut factorial = 1.;
        for n in 0..32 {
            if n > 0 {
                factorial *= n as f64;
            }
            let term = s.powi(2 * n + 1) / (2_f64.powi(n) * factorial * (2 * n + 1) as f64);
            match n % 4 {
                0 => result[0] += term,
                1 => result[1] += term,
                2 => result[0] -= term,
                _ => result[1] -= term,
            }
        }
        result
    }
    #[test]
    fn independent_euler_series_and_endpoint_arc_length_jets() {
        for sign in [-1., 1.] {
            let fit = approximate([3., 4., 5.], 2., 0., sign * 2., 0., 1e-5).unwrap();
            for i in 0..=400 {
                let t = i as f64 / 400.;
                let s = 2. * t;
                let expected = euler(s);
                let p = fit.fit.curve.evaluate(t).unwrap().point;
                let e = (p[0] - 3. - expected[0]).hypot(p[1] - 4. - sign * expected[1]);
                assert!(e <= fit.fit.real_arithmetic_error_estimate + 1e-11);
            }
            for t in [0., 1.] {
                let jet = fit.fit.curve.evaluate(t).unwrap();
                let d = jet.d1.unwrap();
                assert!((d[0] - 2. * (sign * 2. * t * t).cos()).abs() < 1e-10);
                assert!((d[1] - 2. * (sign * 2. * t * t).sin()).abs() < 1e-10);
            }
            assert_eq!(
                fit.fit.real_arithmetic_error_estimate,
                fit.quadrature_error_estimate + fit.hermite_error_estimate
            );
        }
    }
    #[test]
    fn shifted_fresnel_series_for_nonzero_start_curvature_and_rotation() {
        let fit = approximate([3., 4., 5.], 2., 0.5, 2.5, 30., 1e-5).unwrap();
        let origin = euler(0.5);
        let rotation = 30_f64.to_radians() - 0.125;
        for i in 0..=400 {
            let t = i as f64 / 400.;
            let s = 2. * t;
            let end = euler(s + 0.5);
            let x = end[0] - origin[0];
            let y = end[1] - origin[1];
            let exact = [
                3. + x * rotation.cos() - y * rotation.sin(),
                4. + x * rotation.sin() + y * rotation.cos(),
            ];
            let p = fit.fit.curve.evaluate(t).unwrap().point;
            assert!(
                (p[0] - exact[0]).hypot(p[1] - exact[1])
                    <= fit.fit.real_arithmetic_error_estimate + 1e-10
            );
        }
    }
    #[test]
    fn straight_limit_constant_curvature_and_budget_refusals() {
        let line = approximate([0.; 3], 2., 0., 0., 0., 1e-5).unwrap();
        assert_eq!(line.fit.real_arithmetic_error_estimate, 0.);
        let circle = approximate([0.; 3], 2., 0.5, 0.5, 0., 1e-5).unwrap();
        for i in 0..=100 {
            let t = i as f64 / 100.;
            let p = circle.fit.curve.evaluate(t).unwrap().point;
            let e = (p[0] - 2. * t.sin()).hypot(p[1] - 2. * (1. - t.cos()));
            assert!(e <= circle.fit.real_arithmetic_error_estimate + 1e-11);
        }
        assert!(approximate([0.; 3], 0., 0., 1., 0., 1e-5).is_err());
        assert!(approximate([0.; 3], 2., 0., 100., 0., 1e-14).is_err());
    }
}
