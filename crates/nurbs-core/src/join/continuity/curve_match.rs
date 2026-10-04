//! Endpoint G1 construction with an outward bound on tangent misalignment.
use super::regularity::I;
use crate::{
    Result, check,
    curve::Curve,
    intersection::{next_down, next_up},
};
#[cfg(feature = "codec")]
mod serialization;
#[cfg(feature = "codec")]
pub use serialization::checked;

pub enum CurveMatchReason {
    UnprovenEndpointRegularity,
    UnprovenTangentOrientation,
    TangentErrorExceedsBudget,
    Accepted,
}
pub struct CurveMatchReport {
    pub accepted: bool,
    pub sine_angle_upper: Option<f64>,
    pub angle_degrees_upper: Option<f64>,
    pub max_angle_degrees: f64,
    pub regularity_certified: bool,
    pub orientation_certified: bool,
    pub reason: CurveMatchReason,
}
pub struct CurveMatch {
    pub curve: Curve,
    pub report: CurveMatchReport,
}
fn endpoint(curve: &Curve, end: &str) -> Result<(usize, usize)> {
    check(
        !curve.periodic,
        "G1 endpoint matching requires a non-periodic curve",
    )?;
    let n = curve.control_points.len();
    let p = curve.degree;
    let (i, j, knots, value) = match end {
        "start" => (0, 1, &curve.knots[..=p], curve.knots[p]),
        "end" => (
            n - 1,
            n - 2,
            &curve.knots[curve.knots.len() - p - 1..],
            curve.knots[n],
        ),
        _ => return Err(crate::input("Unknown curve endpoint")),
    };
    check(
        knots.iter().all(|&k| k == value),
        "Curve must be exactly clamped at the joined endpoint",
    )?;
    Ok((i, j))
}
fn norm(v: &[f64]) -> f64 {
    v.iter().fold(0_f64, |a, &x| a.hypot(x))
}
fn norm_bounds(v: &[I]) -> (f64, f64) {
    let mut lower = 0_f64;
    let mut upper = I::point(0.);
    for x in v {
        let lo = if x.lo > 0. {
            x.lo
        } else if x.hi < 0. {
            -x.hi
        } else {
            0.
        };
        lower = next_down(lower + next_down(lo * lo)).max(0.);
        let hi = I::point(x.lo.abs().max(x.hi.abs()));
        upper = upper.add(hi.mul(hi));
    }
    (next_down(lower.sqrt()).max(0.), next_up(upper.hi.sqrt()))
}
/// Positive rational weights make endpoint derivatives positive multiples of
/// these handles. Their wedge norm bounds sin(angle); positive dot product
/// disambiguates continuation from the opposite orientation.
pub fn checked_report(
    reference: &Curve,
    edited: &Curve,
    reference_end: &str,
    edited_end: &str,
    max_angle_degrees: f64,
) -> Result<CurveMatch> {
    reference.validate()?;
    edited.validate()?;
    check(
        max_angle_degrees.is_finite() && (0. ..90.).contains(&max_angle_degrees),
        "Tangent angle tolerance must be finite and in [0,90)",
    )?;
    let dimension = reference.control_points[0].len();
    check(
        (dimension == 2 || dimension == 3) && edited.control_points[0].len() == dimension,
        "Curve matching requires equally dimensioned 2D or 3D curves",
    )?;
    let (ri, ra) = endpoint(reference, reference_end)?;
    let (ei, ea) = endpoint(edited, edited_end)?;
    let joint = &reference.control_points[ri];
    let direction: Vec<f64> = joint
        .iter()
        .zip(&reference.control_points[ra])
        .map(|(a, b)| a - b)
        .collect();
    let length = norm(&direction);
    check(
        length.is_finite() && length > 0.,
        "Reference curve has a degenerate endpoint tangent",
    )?;
    let handle: Vec<f64> = edited.control_points[ea]
        .iter()
        .zip(&edited.control_points[ei])
        .map(|(a, b)| a - b)
        .collect();
    let target = norm(&handle);
    check(target.is_finite(), "Edited endpoint handle overflowed")?;
    let scale = if target > 0. { target / length } else { 1. };
    check(
        scale.is_finite() && scale > 0.,
        "Endpoint handle scale is inadmissible",
    )?;
    let mut output = edited.clone();
    output.control_points[ei] = joint.clone();
    output.control_points[ea] = joint
        .iter()
        .zip(&direction)
        .map(|(x, d)| x + d * scale)
        .collect();
    output.validate()?;
    let a: Vec<I> = joint
        .iter()
        .zip(&reference.control_points[ra])
        .map(|(&x, &y)| I::point(x).sub(I::point(y)))
        .collect();
    let b: Vec<I> = output.control_points[ea]
        .iter()
        .zip(joint)
        .map(|(&x, &y)| I::point(x).sub(I::point(y)))
        .collect();
    let (al, au) = norm_bounds(&a);
    let (bl, bu) = norm_bounds(&b);
    let regular = al > 0. && bl > 0. && au.is_finite() && bu.is_finite();
    let mut dot = I::point(0.);
    for i in 0..dimension {
        dot = dot.add(a[i].mul(b[i]));
    }
    let mut wedge = Vec::new();
    for i in 0..dimension {
        for j in i + 1..dimension {
            wedge.push(a[i].mul(b[j]).sub(a[j].mul(b[i])));
        }
    }
    let sine = if regular {
        let denominator = I::point(al).mul(I::point(bl));
        I::point(norm_bounds(&wedge).1)
            .div_positive(denominator)
            .ok()
            .map(|v| v.hi)
            .filter(|v| v.is_finite())
    } else {
        None
    };
    // asin(x) <= x/sqrt(1-x*x) on [0,1). The binary64 PI constant
    // below is less than mathematical pi, so dividing by it is conservative.
    let angle = sine.filter(|&x| x < 1.).and_then(|x| {
        let square = I::point(1.).sub(I::point(x).mul(I::point(x)));
        if square.lo <= 0. {
            return None;
        }
        let denominator = next_down(square.lo.sqrt());
        I::point(x)
            .div_positive(I::point(denominator))
            .ok()?
            .mul(I::point(180.))
            .div_positive(I::point(std::f64::consts::PI))
            .ok()
            .map(|v| v.hi)
            .filter(|v| v.is_finite())
    });
    let accepted = regular && dot.lo > 0. && angle.is_some_and(|x| x <= max_angle_degrees);
    Ok(CurveMatch {
        curve: if accepted { output } else { edited.clone() },
        report: CurveMatchReport {
            accepted,
            sine_angle_upper: sine,
            angle_degrees_upper: angle,
            max_angle_degrees,
            regularity_certified: regular,
            orientation_certified: dot.lo > 0.,
            reason: if !regular {
                CurveMatchReason::UnprovenEndpointRegularity
            } else if dot.lo <= 0. {
                CurveMatchReason::UnprovenTangentOrientation
            } else if !accepted {
                CurveMatchReason::TangentErrorExceedsBudget
            } else {
                CurveMatchReason::Accepted
            },
        },
    })
}
#[cfg(test)]
#[path = "curve_match/tests.rs"]
mod tests;
