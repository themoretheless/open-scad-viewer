//! Geometric endpoint continuity from outward original rational jets.
use crate::{
    check,
    curve::Curve,
    curve_jets,
    distance_bounds::{box_distance, Interval as I},
    Result,
};
#[derive(Clone, Copy)]
pub enum Endpoint {
    Start,
    End,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    WithinTolerance,
    ExceedsTolerance,
    Unresolved,
    RegularityUnproven,
}
pub struct Tolerances {
    pub position: f64,
    pub tangent: f64,
    pub curvature: f64,
}
pub struct Report {
    pub g0: Decision,
    pub g1: Decision,
    pub g2: Decision,
    pub position_error: [f64; 2],
    pub tangent_error: Option<[f64; 2]>,
    pub curvature_error: Option<[f64; 2]>,
}
fn norm(v: &[I]) -> Result<I> {
    let (lo, hi) = box_distance(v, &vec![I::point(0.); v.len()])?;
    I::new(lo, hi)
}
fn error(a: &[I], b: &[I]) -> Result<I> {
    norm(
        &a.iter()
            .zip(b)
            .map(|(x, y)| x.sub(*y))
            .collect::<Result<Vec<_>>>()?,
    )
}
fn decision(i: I, t: f64) -> Decision {
    if i.hi <= t {
        Decision::WithinTolerance
    } else if i.lo > t {
        Decision::ExceedsTolerance
    } else {
        Decision::Unresolved
    }
}
fn jets(c: &Curve, end: Endpoint, reference: bool) -> Result<Vec<Vec<I>>> {
    let last = matches!(end, Endpoint::End);
    let spans = (c.degree..c.control_points.len())
        .filter(|&i| c.knots[i] < c.knots[i + 1])
        .collect::<Vec<_>>();
    let span = if last {
        *spans.last().unwrap()
    } else {
        spans[0]
    };
    let mut j = curve_jets::endpoint(c, span, [c.knots[span], c.knots[span + 1]], last)?;
    // Reference tangent exits its end; edited tangent enters its end.
    if reference != last {
        for v in &mut j[1] {
            *v = v.mul(I::point(-1.))?;
        }
    }
    Ok(j)
}
fn differential(j: &[Vec<I>]) -> Result<Option<(Vec<I>, Vec<I>)>> {
    let speed = norm(&j[1])?;
    if speed.lo <= 0. {
        return Ok(None);
    }
    let tangent = j[1]
        .iter()
        .map(|v| v.div(speed)?.intersect(-1., 1.))
        .collect::<Result<Vec<_>>>()?;
    let a = j[2]
        .iter()
        .map(|v| v.div(speed))
        .collect::<Result<Vec<_>>>()?;
    let dot = tangent
        .iter()
        .zip(&a)
        .try_fold(I::point(0.), |sum, (t, a)| sum.add(t.mul(*a)?))?;
    let curvature = a
        .iter()
        .zip(&tangent)
        .map(|(a, t)| a.sub(t.mul(dot)?)?.div(speed))
        .collect::<Result<Vec<_>>>()?;
    Ok(Some((tangent, curvature)))
}
/// Reference exits its selected endpoint; edited enters its selected endpoint.
/// G1 uses unit-tangent difference; G2 uses curvature-vector difference, so
/// positive parameter-speed changes do not impose parametric C1/C2 equality.
pub fn inspect(
    reference: &Curve,
    reference_end: Endpoint,
    edited: &Curve,
    edited_end: Endpoint,
    tol: Tolerances,
) -> Result<Report> {
    reference.validate()?;
    edited.validate()?;
    check(
        reference.control_points[0].len() == edited.control_points[0].len(),
        "Joined dimensions must match",
    )?;
    check(
        [tol.position, tol.tangent, tol.curvature]
            .iter()
            .all(|v| v.is_finite() && *v >= 0.),
        "Join tolerances must be finite and nonnegative",
    )?;
    let a = jets(reference, reference_end, true)?;
    let b = jets(edited, edited_end, false)?;
    let position = error(&a[0], &b[0])?;
    let g0 = decision(position, tol.position);
    let mut out = Report {
        g0,
        g1: if g0 == Decision::WithinTolerance {
            Decision::RegularityUnproven
        } else {
            g0
        },
        g2: if g0 == Decision::WithinTolerance {
            Decision::RegularityUnproven
        } else {
            g0
        },
        position_error: [position.lo, position.hi],
        tangent_error: None,
        curvature_error: None,
    };
    if let (Some((ta, ka)), Some((tb, kb))) = (differential(&a)?, differential(&b)?) {
        let tangent = error(&ta, &tb)?;
        let curvature = error(&ka, &kb)?;
        out.tangent_error = Some([tangent.lo, tangent.hi]);
        out.curvature_error = Some([curvature.lo, curvature.hi]);
        out.g1 = if g0 == Decision::WithinTolerance {
            decision(tangent, tol.tangent)
        } else {
            g0
        };
        out.g2 = if out.g1 == Decision::WithinTolerance {
            decision(curvature, tol.curvature)
        } else {
            out.g1
        };
    }
    Ok(out)
}
