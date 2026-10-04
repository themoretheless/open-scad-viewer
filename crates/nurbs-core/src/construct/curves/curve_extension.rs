//! Extension of a clamped curve beyond one end (checklist item 374).
//! The appended arc is the analytic continuation of the terminal Bézier span:
//! the homogeneous control net of that span is re-blossomed (de Casteljau
//! extrapolation) onto the widened interval, so point, tangent and curvature
//! jets are matched exactly and the joint is smooth by construction. No new
//! knot is introduced; the terminal knot value simply moves outward.
//!
//! Stability contract: polynomial extrapolation beyond the defining span is
//! inherently ill-conditioned for high degrees. Bernstein coefficients of the
//! widened span grow roughly like (1 + dt/h0)^degree with alternating signs,
//! so requesting an extension longer than about one terminal span length on a
//! high-degree curve produces exploding control coefficients. The report
//! measures `extrapolation_growth` — the maximum distance of the new control
//! points from the joint point, divided by the requested arc length — and
//! marks `stable = false` once growth exceeds `STABILITY_GROWTH_LIMIT`.
//! An exact continuation (line, circle) keeps growth near 1 at any length.
use crate::{Result, check, curve::Curve, numeric, numeric_err};

/// Which end of the curve to extend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    Start,
    Stop,
}

/// Continuity class requested at the joint. The analytic-continuation
/// construction is smooth, so every listed class is met by construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Continuity {
    /// Shared end point and tangent direction.
    G1,
    /// Shared end point, tangent direction and signed curvature.
    G2,
    /// Shared end point and matching first/second parametric derivatives.
    C2,
}

/// Documented stability bound: growth above this factor flags the extension
/// as unstable (the extrapolated coefficients are blowing up).
pub const STABILITY_GROWTH_LIMIT: f64 = 16.;

#[derive(Clone, Debug)]
pub struct ExtensionReport {
    /// The extended curve; its active domain reaches past the requested end.
    pub curve: Curve,
    /// Echoes the requested level; the smooth joint satisfies it by construction.
    pub achieved_continuity: Continuity,
    /// Max distance of new control points from the joint, over `length`.
    pub extrapolation_growth: f64,
    /// False when `extrapolation_growth` exceeds `STABILITY_GROWTH_LIMIT`.
    pub stable: bool,
}

/// Extends a clamped, non-periodic curve beyond `end` by an arc-length
/// estimate of `length` (anchored at the end speed, refined once within a
/// factor of two by chord sampling). Control count is preserved, so the
/// 256-control budget of the source carries over unchanged.
pub fn extend_curve(
    curve: &Curve,
    end: End,
    length: f64,
    continuity: Continuity,
) -> Result<ExtensionReport> {
    curve.validate()?;
    check(
        !curve.periodic,
        "Periodic curves have no free end to extend",
    )?;
    check(
        length.is_finite() && length > 0.,
        "Extension length must be finite and positive",
    )?;
    match end {
        End::Stop => extend_stop(curve, length, continuity),
        End::Start => {
            let reversed = curve.reverse()?;
            let report = extend_stop(&reversed, length, continuity)?;
            Ok(ExtensionReport {
                curve: report.curve.reverse()?,
                ..report
            })
        }
    }
}

fn extend_stop(curve: &Curve, length: f64, continuity: Continuity) -> Result<ExtensionReport> {
    let b = curve.domain()[1];
    let jet = curve.evaluate_validated(b)?;
    let d1 = jet
        .d1
        .ok_or_else(|| numeric_err("End tangent unavailable for extension"))?;
    let speed = d1.iter().map(|x| x * x).sum::<f64>().sqrt();
    numeric(
        speed.is_finite() && speed > 0.,
        "Extension requires a nonzero end speed",
    )?;
    let dt0 = length / speed;
    let (rough, _) = build(curve, dt0, length)?;
    // One bounded chord-length refinement keeps `length` an arc-length
    // estimate without taming the coefficient blow-up we need to flag.
    let measured = chord_length(&rough, b, rough.domain()[1])?;
    let factor = if measured.is_finite() && measured > 0. {
        (length / measured).clamp(0.5, 2.)
    } else {
        1.
    };
    let (built, growth) = if factor == 1. {
        let (_, growth) = build(curve, dt0, length)?;
        (rough, growth)
    } else {
        build(curve, dt0 * factor, length)?
    };
    Ok(ExtensionReport {
        curve: built,
        achieved_continuity: continuity,
        extrapolation_growth: growth,
        stable: growth.is_finite() && growth <= STABILITY_GROWTH_LIMIT,
    })
}

/// Re-blossom the terminal span's homogeneous net onto [lo, b + dt] and move
/// the terminal knots outward. Returns the curve and the growth factor.
fn build(curve: &Curve, dt: f64, length: f64) -> Result<(Curve, f64)> {
    check(
        dt.is_finite() && dt > 0.,
        "Extension parameter step must be finite and positive",
    )?;
    let p = curve.degree;
    let n = curve.control_points.len();
    let b = curve.knots[n];
    // Last nonempty span; a clamped end has knots[n-1] < knots[n] = b.
    let span = (p..n)
        .rev()
        .find(|&i| curve.knots[i] < curve.knots[i + 1])
        .ok_or_else(|| numeric_err("Curve has no nonempty terminal span"))?;
    check(
        span == n - 1,
        "Extension requires a clamped end (end knot multiplicity degree+1)",
    )?;
    let lo = curve.knots[span];
    let h0 = b - lo;
    let lambda = (h0 + dt) / h0;
    numeric(lambda.is_finite(), "Extension parameter step overflowed")?;
    let dim = curve.control_points[0].len();
    let window: Vec<Vec<f64>> = (span - p..=span)
        .map(|i| {
            curve.control_points[i]
                .iter()
                .map(|x| x * curve.weights[i])
                .chain(std::iter::once(curve.weights[i]))
                .collect()
        })
        .collect();
    // Blossom(lo^(p-j), (b+dt)^j): j de Casteljau steps at t = lambda
    // (the p-j zero arguments are no-ops) over window[0..=j].
    let mut q: Vec<Vec<f64>> = Vec::with_capacity(p + 1);
    for j in 0..=p {
        let mut row = window[..=j].to_vec();
        for _ in 0..j {
            for m in 0..row.len() - 1 {
                let merged: Vec<f64> = row[m]
                    .iter()
                    .zip(&row[m + 1])
                    .map(|(x, y)| (1. - lambda) * x + lambda * y)
                    .collect();
                row[m] = merged;
            }
        }
        q.push(row[0].clone());
    }
    numeric(
        q.iter().flatten().all(|x| x.is_finite()),
        "Extrapolation exhausted finite precision",
    )?;
    numeric(
        q.iter().all(|v| v[dim] >= 1e-12 && v[dim] <= 1e12),
        "Extrapolated weights left the admissible range",
    )?;
    let mut control_points = curve.control_points[..span - p].to_vec();
    control_points.extend(q.iter().map(|v| {
        v[..dim].iter().map(|x| x / v[dim]).collect::<Vec<f64>>()
    }));
    let mut weights = curve.weights[..span - p].to_vec();
    weights.extend(q.iter().map(|v| v[dim]));
    let mut knots = curve.knots.clone();
    for k in &mut knots[span + 1..] {
        *k = b + dt;
    }
    let out = Curve {
        degree: p,
        knots,
        control_points,
        weights,
        periodic: false,
    };
    out.validate()?;
    let joint = curve.evaluate_validated(b)?.point;
    let growth = out.control_points[span - p..]
        .iter()
        .map(|c| {
            c.iter()
                .zip(&joint)
                .map(|(x, y)| (x - y) * (x - y))
                .sum::<f64>()
                .sqrt()
        })
        .fold(0., f64::max)
        / length;
    Ok((out, growth))
}

fn chord_length(curve: &Curve, a: f64, b: f64) -> Result<f64> {
    let mut total = 0.;
    let mut previous = curve.evaluate(a)?.point;
    for i in 1..=8 {
        let point = curve.evaluate(a + (b - a) * i as f64 / 8.)?.point;
        total += point
            .iter()
            .zip(&previous)
            .map(|(x, y)| (x - y) * (x - y))
            .sum::<f64>()
            .sqrt();
        previous = point;
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::{circle_arc, line};

    #[test]
    fn straight_line_extension_stays_straight_with_unit_growth() {
        let c = line([0., 0., 0.], [2., 0., 0.]).unwrap();
        for end in [End::Start, End::Stop] {
            let r = extend_curve(&c, end, 2., Continuity::C2).unwrap();
            assert!(r.stable);
            assert!((r.extrapolation_growth - 1.).abs() < 1e-9);
            let [a, b] = r.curve.domain();
            for i in 0..=20 {
                let p = r.curve.evaluate(a + (b - a) * i as f64 / 20.).unwrap().point;
                assert!(p[1].abs() < 1e-12 && p[2].abs() < 1e-12);
            }
            let far = match end {
                End::Stop => r.curve.evaluate(b).unwrap().point,
                End::Start => r.curve.evaluate(a).unwrap().point,
            };
            let expected = match end {
                End::Stop => 4.,
                End::Start => -2.,
            };
            assert!((far[0] - expected).abs() < 1e-12);
        }
    }

    #[test]
    fn circle_extension_g2_stays_on_circle_and_is_c2_at_joint() {
        let c = circle_arc([0., 0., 0.], [0., 0., 1.], 2., 0., 60.).unwrap();
        let b = c.domain()[1];
        let r = extend_curve(&c, End::Stop, 1., Continuity::G2).unwrap();
        assert_eq!(r.achieved_continuity, Continuity::G2);
        assert!(r.stable);
        let b1 = r.curve.domain()[1];
        assert!(b1 > b);
        for i in 0..=32 {
            let p = r.curve.evaluate(b + (b1 - b) * i as f64 / 32.).unwrap().point;
            assert!((p[0].hypot(p[1]) - 2.).abs() < 1e-9);
            assert_eq!(p[2], 0.);
        }
        // One-sided jets agree across the joint: the extension is C2 there.
        let before = c.evaluate(b).unwrap();
        let after = r.curve.evaluate(b).unwrap();
        let (d1_before, d1_after) = (before.d1.as_ref().unwrap(), after.d1.as_ref().unwrap());
        let (d2_before, d2_after) = (before.d2.as_ref().unwrap(), after.d2.as_ref().unwrap());
        for axis in 0..3 {
            assert!((before.point[axis] - after.point[axis]).abs() < 1e-12);
            assert!((d1_before[axis] - d1_after[axis]).abs() < 1e-9);
            assert!((d2_before[axis] - d2_after[axis]).abs() < 1e-8);
        }
    }

    #[test]
    fn far_high_degree_extrapolation_is_flagged_unstable() {
        // Oscillating degree-5 Bézier: far extrapolation explodes combinatorially.
        let c = Curve {
            degree: 5,
            knots: [vec![0.; 6], vec![1.; 6]].concat(),
            control_points: (0..=5)
                .map(|i| vec![i as f64 / 5., if i % 2 == 0 { 0.5 } else { -0.5 }, 0.])
                .collect(),
            weights: vec![1.; 6],
            periodic: false,
        };
        c.validate().unwrap();
        let r = extend_curve(&c, End::Stop, 20., Continuity::C2).unwrap();
        assert!(!r.stable);
        assert!(r.extrapolation_growth > STABILITY_GROWTH_LIMIT);
        // A modest extension of the same curve stays stable.
        let modest = extend_curve(&c, End::Stop, 0.5, Continuity::G1).unwrap();
        assert!(modest.stable);
        assert_eq!(modest.achieved_continuity, Continuity::G1);
    }

    #[test]
    fn invalid_extension_requests_are_rejected() {
        let c = line([0., 0., 0.], [1., 0., 0.]).unwrap();
        assert!(extend_curve(&c, End::Stop, 0., Continuity::G1).is_err());
        assert!(extend_curve(&c, End::Stop, -1., Continuity::G1).is_err());
        assert!(extend_curve(&c, End::Stop, f64::NAN, Continuity::G1).is_err());
        assert!(extend_curve(&c, End::Stop, 1e300, Continuity::G1).is_err());
        // Zero-speed end: coincident controls collapse the tangent.
        let flat = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![0., 0., 0.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        flat.validate().unwrap();
        assert!(extend_curve(&flat, End::Stop, 1., Continuity::G1).is_err());
    }
}
