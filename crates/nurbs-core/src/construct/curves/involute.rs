//! Planar base-circle involute, cubic Hermite approximation with explicit ideal remainder.
pub use crate::helix::Approximation;
use crate::{Result, check, curve::Curve};

/// P(theta)=center+radius*[cos(theta)+theta*sin(theta),sin(theta)-theta*cos(theta),0].
/// Parameters are radians; the native curve domain is normalized to [0,1].
/// The estimate excludes binary64 trig/control/evaluation rounding.
pub fn approximate(
    center: [f64; 3],
    radius: f64,
    start: f64,
    end: f64,
    budget: f64,
) -> Result<Approximation> {
    check(
        center.iter().all(|x| x.is_finite())
            && radius.is_finite()
            && radius > 0.
            && start.is_finite()
            && end.is_finite()
            && end > start
            && budget.is_finite()
            && budget > 0.,
        "Involute requires finite center, positive radius/budget and increasing angles",
    )?;
    let range = end - start;
    check(range.is_finite(), "Involute angle range overflow")?;
    // ||P''''(theta)||=radius*sqrt(9+theta^2). Coordinate Hermite bounds
    // combined by hypot give sqrt(2)*radius*max||unit P''''||*h^4/384.
    let derivative = radius * start.abs().max(end.abs()).hypot(3.);
    let mut choice = None;
    for n in 1..=85 {
        let h = range / n as f64;
        let h2 = h * h;
        let estimate = (derivative * (h2 * h2) / 384.) * 2_f64.sqrt();
        if estimate.is_finite() && estimate > 0. && estimate <= budget {
            choice = Some((n, estimate));
            break;
        }
    }
    let (spans, estimate) = choice.ok_or_else(|| {
        crate::input("Involute estimate is unrepresentable or needs more than 85 cubic spans")
    })?;
    let mut points = Vec::with_capacity(spans + 1);
    let mut tangents = Vec::with_capacity(spans + 1);
    let mut parameters = Vec::with_capacity(spans + 1);
    for i in 0..=spans {
        let t = i as f64 / spans as f64;
        let theta = (1. - t) * start + t * end;
        let (sin, cos) = theta.sin_cos();
        points.push([
            center[0] + radius * (cos + theta * sin),
            center[1] + radius * (sin - theta * cos),
            center[2],
        ]);
        tangents.push([
            radius * theta * cos * range,
            radius * theta * sin * range,
            0.,
        ]);
        parameters.push(t);
    }
    let curve: Curve = crate::hermite::interpolate_inferred(&points, &tangents, &parameters)?;
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
    fn independent_samples_and_analytic_endpoint_tangents() {
        for (start, end) in [(0., 2.), (-2., 1.), (1., 4.)] {
            let a = approximate([3., 4., 5.], 2., start, end, 1e-4).unwrap();
            for i in 0..=1000 {
                let t = i as f64 / 1000.;
                let theta = start + (end - start) * t;
                let exact = [
                    3. + 2. * (theta.cos() + theta * theta.sin()),
                    4. + 2. * (theta.sin() - theta * theta.cos()),
                    5.,
                ];
                let p = a.curve.evaluate(t).unwrap().point;
                let distance = (0..3)
                    .map(|d| (p[d] - exact[d]).powi(2))
                    .sum::<f64>()
                    .sqrt();
                assert!(distance <= a.real_arithmetic_error_estimate + 1e-12);
            }
            for (t, theta) in [(0., start), (1., end)] {
                let d = a.curve.evaluate(t).unwrap().d1.unwrap();
                assert!((d[0] - 2. * theta * theta.cos() * (end - start)).abs() < 1e-11);
                assert!((d[1] - 2. * theta * theta.sin() * (end - start)).abs() < 1e-11);
            }
        }
    }
    #[test]
    fn budget_and_invalid_inputs_are_not_silently_relaxed() {
        let a = approximate([0.; 3], 2., 0., 2., 1e-2).unwrap();
        let b = approximate([0.; 3], 2., 0., 2., 1e-5).unwrap();
        assert!(b.spans > a.spans);
        assert!(approximate([0.; 3], 2., 0., 100., 1e-12).is_err());
        assert!(approximate([0.; 3], 0., 0., 2., 1e-4).is_err());
        assert!(approximate([0.; 3], 2., 2., 0., 1e-4).is_err());
        assert!(approximate([0.; 3], 2., 0., 2., 0.).is_err());
    }
    #[cfg(feature = "transport")]
    #[test]
    fn json_preserves_uncertified_report_and_base_circle_endpoint() {
        let value=crate::dispatch(value_codec::json!({"op":"curve_involute","center":[3.,4.,5.],"radius":2.,"start_radians":0.,"end_radians":2.,"max_deviation":1e-4})).unwrap();
        let curve: Curve = value_codec::from_value(value["curve"].clone()).unwrap();
        assert_eq!(curve.evaluate(0.).unwrap().point, vec![5., 4., 5.]);
        assert_eq!(
            value["report"]["roundingCertified"],
            value_codec::json!(false)
        );
    }
}

/// Quintic endpoint-Hermite approximation of the same analytic involute.
/// Matches position, first and second derivatives at each span endpoint.
/// The continuous real-arithmetic remainder excludes binary64 rounding.
pub fn approximate_quintic(
    center: [f64; 3],
    radius: f64,
    start: f64,
    end: f64,
    budget: f64,
) -> Result<Approximation> {
    check(
        center.iter().all(|x| x.is_finite())
            && radius.is_finite()
            && radius > 0.
            && start.is_finite()
            && end.is_finite()
            && end > start
            && budget.is_finite()
            && budget > 0.,
        "Quintic involute requires finite center, positive radius/budget and increasing angles",
    )?;
    let range = end - start;
    check(range.is_finite(), "Involute angle range overflow")?;
    // ||P^(6)||=r*hypot(5,theta). Endpoint-Hermite remainder is
    // f^(6)(xi)*(theta-a)^3*(theta-b)^3/6!, bounded by M*h^6/46080.
    let derivative = radius * start.abs().max(end.abs()).hypot(5.);
    let (spans, estimate) = (1..=51)
        .find_map(|count| {
            let h = range / count as f64;
            let h2 = h * h;
            let error = derivative * h2 * h2 * h2 * 2_f64.sqrt() / 46080.;
            (error.is_finite() && error > 0. && error <= budget).then_some((count, error))
        })
        .ok_or_else(|| {
            crate::input("Quintic involute estimate is unrepresentable or needs more than 51 spans")
        })?;
    let h = range / spans as f64;
    let jet = |theta: f64| {
        let (sin, cos) = theta.sin_cos();
        (
            [
                center[0] + radius * (cos + theta * sin),
                center[1] + radius * (sin - theta * cos),
                center[2],
            ],
            [radius * theta * cos, radius * theta * sin, 0.],
            [
                radius * (cos - theta * sin),
                radius * (sin + theta * cos),
                0.,
            ],
        )
    };
    let mut points = Vec::with_capacity(5 * spans + 1);
    let mut knots = vec![0.; 6];
    for i in 0..spans {
        let (a, da, dda) = jet(start + range * i as f64 / spans as f64);
        let (b, db, ddb) = jet(if i + 1 == spans {
            end
        } else {
            start + range * (i + 1) as f64 / spans as f64
        });
        if i == 0 {
            points.push(a.to_vec());
        }
        points.push((0..3).map(|j| a[j] + h * da[j] / 5.).collect());
        points.push(
            (0..3)
                .map(|j| a[j] + 2. * h * da[j] / 5. + h * h * dda[j] / 20.)
                .collect(),
        );
        points.push(
            (0..3)
                .map(|j| b[j] - 2. * h * db[j] / 5. + h * h * ddb[j] / 20.)
                .collect(),
        );
        points.push((0..3).map(|j| b[j] - h * db[j] / 5.).collect());
        points.push(b.to_vec());
        knots.extend(std::iter::repeat_n((i + 1) as f64 / spans as f64, 5));
    }
    knots.push(1.);
    let curve = Curve {
        degree: 5,
        knots,
        weights: vec![1.; points.len()],
        control_points: points,
        periodic: false,
    };
    curve.validate()?;
    Ok(Approximation {
        curve,
        spans,
        budget,
        real_arithmetic_error_estimate: estimate,
    })
}
