//! Explicit bounded shared-generator repair; regularity and B-rep ownership
//! must be revalidated independently before these sections admit a Solid.
use crate::{Error, Result, curve::Curve, distance_bounds::Interval};
pub struct Report {
    pub curves: Option<Vec<Curve>>,
    pub displacement_upper: Option<f64>,
    pub work: u64,
    pub reason: &'static str,
}
pub fn repair(curves: &[Curve], quantum: f64, tolerance: f64, max_work: u64) -> Result<Report> {
    if curves.is_empty()
        || curves.len() > 64
        || !quantum.is_finite()
        || quantum <= 0.
        || !tolerance.is_finite()
        || tolerance < 0.
        || max_work > 1000000
    {
        return Err(Error::new(
            "NURBS_INVALID_INPUT",
            "Invalid circle section repair budget",
        ));
    }
    let bits = quantum.to_bits();
    let fraction = bits & ((1u64 << 52) - 1);
    let exponent = (bits >> 52) & 2047;
    if !(exponent > 0 && fraction == 0 || exponent == 0 && fraction.is_power_of_two()) {
        return Err(Error::new(
            "NURBS_INVALID_INPUT",
            "Circle repair quantum must be a power of two",
        ));
    }
    let mut report = Report {
        curves: None,
        displacement_upper: None,
        work: 0,
        reason: "work-limit",
    };
    let mut output = Vec::new();
    let mut upper = 0_f64;
    for curve in curves {
        curve.validate()?;
        if curve.degree != 2
            || curve.control_points.len() != 9
            || curve.control_points.iter().any(|p| p.len() != 3)
            || curve.control_points[0] != curve.control_points[8]
            || curve.weights.iter().any(|w| *w <= 0.)
            || curve
                .weights
                .iter()
                .enumerate()
                .any(|(i, w)| *w != curve.weights[i % 2])
        {
            report.reason = "unsupported-section";
            return Ok(report);
        }
        let signs = [
            (1., 0.),
            (1., 1.),
            (0., 1.),
            (-1., 1.),
            (-1., 0.),
            (-1., -1.),
            (0., -1.),
            (1., -1.),
            (1., 0.),
        ];
        let mut generators = [[0.; 3]; 3];
        for k in 0..3 {
            let center = curve.control_points[0][k] * 0.5 + curve.control_points[4][k] * 0.5;
            for (i, x) in [
                center,
                curve.control_points[0][k] - center,
                curve.control_points[2][k] - center,
            ]
            .into_iter()
            .enumerate()
            {
                if report.work == max_work {
                    return Ok(report);
                }
                report.work += 1;
                let n = (x / quantum).round();
                if !n.is_finite() || n.abs() >= 2_f64.powi(50) {
                    report.reason = "mantissa-range";
                    return Ok(report);
                }
                generators[i][k] = n;
            }
        }
        let mut repaired = curve.clone();
        for (i, (a, b)) in signs.into_iter().enumerate() {
            let mut squared = Interval::point(0.);
            for k in 0..3 {
                if report.work == max_work {
                    return Ok(report);
                }
                report.work += 1;
                let q = (generators[0][k] + a * generators[1][k] + b * generators[2][k]) * quantum;
                if !q.is_finite() {
                    report.reason = "numeric-range";
                    return Ok(report);
                }
                repaired.control_points[i][k] = q;
                let d = Interval::point(q).sub(Interval::point(curve.control_points[i][k]))?;
                let m = d.lo.abs().max(d.hi.abs());
                squared = squared.add(Interval::point(m).mul(Interval::point(m))?)?;
            }
            upper = upper.max(squared.hi.max(0.).sqrt().next_up());
        }
        output.push(repaired);
    }
    report.displacement_upper = Some(upper);
    if upper > tolerance {
        report.reason = "displacement-budget";
        return Ok(report);
    }
    report.curves = Some(output);
    report.reason = "bounded-shared-generators";
    Ok(report)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_shared_tangents_and_refuses_exhausted_budgets() {
        let mut c = crate::primitives::circle_arc([0.; 3], [0., 0., 1.], 1., 0., 360.).unwrap();
        for p in &mut c.control_points {
            let (x, y) = (p[0], p[1]);
            p[0] = 10. + 0.31 * x + 0.27 * y;
            p[1] = 4. + 0.19 * x - 0.41 * y;
            p[2] = 3. + 0.11 * x + 0.17 * y;
        }
        let r = repair(&[c.clone()], 2_f64.powi(-40), 1e-10, 1000).unwrap();
        assert!(r.displacement_upper.unwrap() < 1e-10);
        let corrected = &r.curves.unwrap()[0];
        for j in [0, 2, 4, 6] {
            for k in 0..3 {
                assert_eq!(
                    corrected.control_points[(j + 7) % 8][k]
                        + corrected.control_points[(j + 1) % 8][k],
                    2. * corrected.control_points[j][k]
                );
            }
        }
        assert!(
            repair(&[c.clone()], 2_f64.powi(-40), 0., 1000)
                .unwrap()
                .curves
                .is_none()
        );
        assert!(
            repair(&[c.clone()], 2_f64.powi(-40), 1e-10, 35)
                .unwrap()
                .curves
                .is_none()
        );
        assert!(
            repair(&[c], 2_f64.powi(-60), 1e-10, 1000)
                .unwrap()
                .curves
                .is_none()
        );
    }
}

#[cfg(all(test, feature = "transport"))]
mod transport_tests {
    #[test]
    fn json_round_trip_preserves_bounded_candidate_and_atomic_refusal() {
        let curve = crate::primitives::circle_arc([0.; 3], [0., 0., 1.], 1., 0., 360.).unwrap();
        let request = value_codec::json!({"op":"curve_repair_circle_section","curves":[curve],"quantum":2_f64.powi(-40),"tolerance":1e-9,"maxWork":1000});
        let result = crate::transport::dispatch(request.clone()).unwrap();
        assert_eq!(result["reason"], "bounded-shared-generators");
        assert!(result["curves"].is_array());
        let mut exhausted = request;
        exhausted["maxWork"] = value_codec::json!(0);
        let refused = crate::transport::dispatch(exhausted).unwrap();
        assert!(refused["curves"].is_null());
        assert_eq!(refused["reason"], "work-limit");
    }
}
