//! Point differential geometry of the original rational curve with interval jets.
//! Curvature/torsion are invariant under the positive local span parameter map.
use crate::{
    Result, check,
    curve::Curve,
    curve_jets,
    distance_bounds::Interval as I,
    numerics::interval_vec3::{cross, norm},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Automatic,
    Left,
    Right,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SideUsed {
    TwoSided,
    Left,
    Right,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Available,
    ContinuityNotProven,
    SpeedNotSeparatedFromZero,
    CurvatureNotSeparatedFromZero,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub parameter: f64,
    pub side: SideUsed,
    /// Conservative knot continuity; None means smooth within a span or a
    /// one-sided natural-domain endpoint. Authored extra smoothness is not inferred.
    pub continuity: Option<i32>,
    pub curvature: Option<[f64; 2]>,
    pub torsion: Option<[f64; 2]>,
    pub curvature_status: Status,
    pub torsion_status: Status,
    pub frame_status: Status,
    pub tangent: Option<[[f64; 2]; 3]>,
    pub normal: Option<[[f64; 2]; 3]>,
    pub binormal: Option<[[f64; 2]; 3]>,
}
fn divide(v: [I; 3], d: I, unit: bool) -> Result<[I; 3]> {
    let mut out = v;
    for k in 0..3 {
        out[k] = v[k].div(d)?;
        if unit {
            out[k] = out[k].intersect(-1., 1.)?;
        }
    }
    Ok(out)
}
fn bounds(v: [I; 3]) -> [[f64; 2]; 3] {
    v.map(|i| [i.lo, i.hi])
}
/// Bounds refer to stored binary64 control data interpreted as exact real data.
/// Automatic queries need C2 for curvature and C3 for torsion at interior knots.
/// Explicit one-sided queries resolve knot ambiguity; outside-domain sides fail.
/// A missing field is unresolved/undefined, never a fabricated zero or frame.
pub fn at(curve: &Curve, parameter: f64, side: Side) -> Result<Report> {
    curve.validate()?;
    let [a, b] = curve.domain();
    check(
        parameter.is_finite() && a <= parameter && parameter <= b,
        "Differential parameter must be inside the active curve domain",
    )?;
    check(
        !(parameter == a && side == Side::Left) && !(parameter == b && side == Side::Right),
        "Requested differential side lies outside the natural domain",
    )?;
    let continuity = if parameter > a && parameter < b {
        let m = curve.knots.iter().filter(|&&k| k == parameter).count();
        (m > 0).then_some(curve.degree as i32 - m as i32)
    } else {
        None
    };
    let used = match side {
        Side::Left => SideUsed::Left,
        Side::Right => SideUsed::Right,
        Side::Automatic => {
            if parameter == a {
                SideUsed::Right
            } else if parameter == b {
                SideUsed::Left
            } else {
                SideUsed::TwoSided
            }
        }
    };
    let one_sided = used != SideUsed::TwoSided;
    let allowed = |order: i32| one_sided || continuity.is_none_or(|c| c >= order);
    let span = (curve.degree..curve.control_points.len())
        .find(|&i| {
            let lo = curve.knots[i];
            let hi = curve.knots[i + 1];
            lo < hi
                && match used {
                    SideUsed::Left => lo < parameter && parameter <= hi,
                    _ => lo <= parameter && parameter < hi,
                }
        })
        .ok_or_else(|| crate::input("No knot span owns the requested differential side"))?;
    let lo = curve.knots[span];
    let hi = curve.knots[span + 1];
    // The longer endpoint restriction avoids tiny local derivative scale when
    // querying very close to a source knot. No rounded control curve is created.
    let (domain, end) = if parameter - lo >= hi - parameter {
        ([lo, parameter], true)
    } else {
        ([parameter, hi], false)
    };
    let jets = curve_jets::endpoint(curve, span, domain, end)?;
    let vector =
        |order: usize| std::array::from_fn(|k| jets[order].get(k).copied().unwrap_or(I::point(0.)));
    let speed = norm(vector(1))?;
    let mut out = Report {
        parameter,
        side: used,
        continuity,
        curvature: None,
        torsion: None,
        tangent: None,
        normal: None,
        binormal: None,
        curvature_status: if allowed(2) {
            Status::SpeedNotSeparatedFromZero
        } else {
            Status::ContinuityNotProven
        },
        torsion_status: if allowed(3) {
            Status::SpeedNotSeparatedFromZero
        } else {
            Status::ContinuityNotProven
        },
        frame_status: if allowed(2) {
            Status::SpeedNotSeparatedFromZero
        } else {
            Status::ContinuityNotProven
        },
    };
    if speed.lo <= 0. {
        return Ok(out);
    }
    let tangent = divide(vector(1), speed, true)?;
    if allowed(1) {
        out.tangent = Some(bounds(tangent));
    }
    if !allowed(2) {
        return Ok(out);
    }
    // Normalize before powers: avoids underflow of speed^3 for tiny geometry.
    let turn = cross(tangent, divide(vector(2), speed, false)?)?;
    let turn_norm = norm(turn)?;
    let curvature = turn_norm.div(speed)?;
    out.curvature = Some([curvature.lo.max(0.), curvature.hi]);
    out.curvature_status = Status::Available;
    if turn_norm.lo <= 0. {
        out.frame_status = Status::CurvatureNotSeparatedFromZero;
        if allowed(3) {
            out.torsion_status = Status::CurvatureNotSeparatedFromZero;
        }
        return Ok(out);
    }
    let binormal = divide(turn, turn_norm, true)?;
    let mut normal = cross(binormal, tangent)?;
    for k in 0..3 {
        normal[k] = normal[k].intersect(-1., 1.)?;
    }
    out.frame_status = Status::Available;
    out.binormal = Some(bounds(binormal));
    out.normal = Some(bounds(normal));
    if allowed(3) {
        let third = divide(vector(3), speed, false)?;
        let mut dot = I::point(0.);
        for k in 0..3 {
            dot = dot.add(binormal[k].mul(third[k])?)?;
        }
        let torsion = dot.div(turn_norm)?.div(speed)?;
        out.torsion = Some([torsion.lo, torsion.hi]);
        out.torsion_status = Status::Available;
    }
    Ok(out)
}
