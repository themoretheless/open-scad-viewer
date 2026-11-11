//! Toroidal trajectories with an analytic harmonic fourth-derivative estimate.
pub use crate::helix::Approximation;
use crate::{Result, check};

/// A ring torus: center, major radius R and minor radius r with R > r > 0.
#[derive(Clone, Copy, Debug)]
pub struct Torus {
    pub center: [f64; 3],
    pub major_radius: f64,
    pub minor_radius: f64,
}
/// Signed turns around the major and minor circles and the starting phase of each, in degrees.
#[derive(Clone, Copy, Debug)]
pub struct TorusWinding {
    pub major_turns: f64,
    pub minor_turns: f64,
    pub major_phase_degrees: f64,
    pub minor_phase_degrees: f64,
}

/// P(t)=center+[(R+r*cos(B))*cos(A),(R+r*cos(B))*sin(A),r*sin(B)],
/// A=major_phase+TAU*major_turns*t, B=minor_phase+TAU*minor_turns*t.
/// Signed fractional turns define open trajectories on a ring torus R>r>0.
pub fn approximate(torus: Torus, winding: TorusWinding, budget: f64) -> Result<Approximation> {
    fit(torus, winding, budget, false)
}

/// Closed (p,q) torus knot, with positive coprime integers p,q >= 2.
/// Clamped encoding retains periodic=false; endpoint controls/jets are shared.
/// This does not certify knot topology or absence of approximation intersections.
pub fn approximate_knot(
    torus: Torus,
    p: usize,
    q: usize,
    [major_phase_degrees, minor_phase_degrees]: [f64; 2],
    budget: f64,
) -> Result<Approximation> {
    let (mut a, mut b) = (p, q);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    check(
        p >= 2 && q >= 2 && p <= 32 && q <= 32 && a == 1,
        "Torus knot requires coprime p,q in 2..32; links and unknots use other contracts",
    )?;
    let winding = TorusWinding {
        major_turns: p as f64,
        minor_turns: q as f64,
        major_phase_degrees,
        minor_phase_degrees,
    };
    fit(torus, winding, budget, true)
}

fn fit(torus: Torus, winding: TorusWinding, budget: f64, closed: bool) -> Result<Approximation> {
    let Torus { center, major_radius, minor_radius } = torus;
    let TorusWinding { major_turns, minor_turns, major_phase_degrees, minor_phase_degrees } = winding;
    check(
        center.iter().all(|x| x.is_finite())
            && major_radius.is_finite()
            && minor_radius.is_finite()
            && minor_radius > 0.
            && major_radius > minor_radius
            && [
                major_turns,
                minor_turns,
                major_phase_degrees,
                minor_phase_degrees,
                budget,
            ]
            .iter()
            .all(|x| x.is_finite())
            && major_turns != 0.
            && minor_turns != 0.
            && budget > 0.,
        "Toroidal spiral needs a ring torus, nonzero turns and positive finite budget",
    )?;
    let a = std::f64::consts::TAU * major_turns;
    let b = std::f64::consts::TAU * minor_turns;
    check(
        [a, b, a + b, a - b].iter().all(|x| x.is_finite()),
        "Toroidal frequency overflow",
    )?;
    let mut choice = None;
    // Product-to-sum gives XY harmonics at a, a+b and a-b; Z is at b.
    for spans in 1..=85 {
        let n = spans as f64;
        let fourth = |w: f64| {
            let s = w / n;
            let s2 = s * s;
            s2 * s2
        };
        let xy = major_radius * fourth(a) + minor_radius / 2. * (fourth(a + b) + fourth(a - b));
        let z = minor_radius * fourth(b);
        let estimate = xy.hypot(xy).hypot(z) / 384.;
        if estimate.is_finite() && estimate > 0. && estimate <= budget {
            choice = Some((spans, estimate));
            break;
        }
    }
    let (spans, estimate) = choice.ok_or_else(|| {
        crate::input("Toroidal remainder is unrepresentable or exceeds 85 cubic spans")
    })?;
    let phase_a = major_phase_degrees.rem_euclid(360.).to_radians();
    let phase_b = minor_phase_degrees.rem_euclid(360.).to_radians();
    let mut points = Vec::new();
    let mut tangents = Vec::new();
    let mut parameters = Vec::new();
    for i in 0..=spans {
        let t = i as f64 / spans as f64;
        let sample_t = if closed && i == spans { 0. } else { t };
        let (sa, ca) = (phase_a + a * sample_t).sin_cos();
        let (sb, cb) = (phase_b + b * sample_t).sin_cos();
        let radius = major_radius + minor_radius * cb;
        let dr = -minor_radius * b * sb;
        points.push([
            center[0] + radius * ca,
            center[1] + radius * sa,
            center[2] + minor_radius * sb,
        ]);
        tangents.push([
            dr * ca - radius * a * sa,
            dr * sa + radius * a * ca,
            minor_radius * b * cb,
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
    fn independent_formula_signed_turns_and_endpoint_jets() {
        for (p, q) in [(0.5, 1.25), (-0.75, 0.5), (2., 3.)] {
            let fit = approximate(
                Torus { center: [3., 4., 5.], major_radius: 3., minor_radius: 1. },
                TorusWinding { major_turns: p, minor_turns: q, major_phase_degrees: 23., minor_phase_degrees: 47. },
                1e-4,
            ).unwrap();
            for i in 0..=1000 {
                let t = i as f64 / 1000.;
                let a = 23_f64.to_radians() + std::f64::consts::TAU * p * t;
                let b = 47_f64.to_radians() + std::f64::consts::TAU * q * t;
                let exact = [
                    3. + (3. + b.cos()) * a.cos(),
                    4. + (3. + b.cos()) * a.sin(),
                    5. + b.sin(),
                ];
                let point = fit.curve.evaluate(t).unwrap().point;
                let error = (point[0] - exact[0])
                    .hypot(point[1] - exact[1])
                    .hypot(point[2] - exact[2]);
                assert!(error <= fit.real_arithmetic_error_estimate + 1e-12);
            }
            for t in [0., 1.] {
                let a = 23_f64.to_radians() + std::f64::consts::TAU * p * t;
                let b = 47_f64.to_radians() + std::f64::consts::TAU * q * t;
                let da = std::f64::consts::TAU * p;
                let db = std::f64::consts::TAU * q;
                let expected = [
                    -db * b.sin() * a.cos() - (3. + b.cos()) * da * a.sin(),
                    -db * b.sin() * a.sin() + (3. + b.cos()) * da * a.cos(),
                    db * b.cos(),
                ];
                let d = fit.curve.evaluate(t).unwrap().d1.unwrap();
                for k in 0..3 {
                    assert!((d[k] - expected[k]).abs() < 1e-9);
                }
            }
        }
    }
    #[test]
    fn closed_knot_and_refusals() {
        let a = approximate_knot(
            Torus { center: [0.; 3], major_radius: 3., minor_radius: 1. },
            2,
            3,
            [0., 0.],
            1e-4,
        ).unwrap();
        assert_eq!(
            a.curve.control_points.first(),
            a.curve.control_points.last()
        );
        assert!(!a.curve.periodic);
        assert!(approximate_knot(
            Torus { center: [0.; 3], major_radius: 3., minor_radius: 1. },
            2,
            4,
            [0., 0.],
            1e-4,
        ).is_err());
        assert!(approximate(
            Torus { center: [0.; 3], major_radius: 1., minor_radius: 1. },
            TorusWinding { major_turns: 1., minor_turns: 2., major_phase_degrees: 0., minor_phase_degrees: 0. },
            1e-4,
        ).is_err());
        assert!(approximate(
            Torus { center: [0.; 3], major_radius: 3., minor_radius: 1. },
            TorusWinding { major_turns: 2., minor_turns: 3., major_phase_degrees: 0., minor_phase_degrees: 0. },
            1e-12,
        ).is_err());
    }
}
