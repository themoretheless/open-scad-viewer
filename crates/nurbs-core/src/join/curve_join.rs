//! Geometric endpoint continuity from outward original rational jets.
use crate::{
    Result, check,
    curve::Curve,
    curve_jets,
    distance_bounds::{Interval as I, box_distance},
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
    let domain = c.domain();
    if !c.periodic
        && c.knots[..=c.degree].iter().all(|&t| t == domain[0])
        && c.knots[c.control_points.len()..]
            .iter()
            .all(|&t| t == domain[1])
    {
        // The clamped endpoint is literally its original Cartesian pole,
        // regardless of rational gauge. Do not re-divide a tiny weight.
        j[0] = c.control_points[if last { c.control_points.len() - 1 } else { 0 }]
            .iter()
            .map(|&v| I::point(v))
            .collect();
    }
    // Reference tangent exits its end; edited tangent enters its end.
    if reference != last {
        for v in &mut j[1] {
            *v = v.mul(I::point(-1.))?;
        }
    }
    Ok(j)
}
fn axial_endpoint(c: &Curve, end: Endpoint) -> Option<usize> {
    if c.periodic || c.degree == 0 {
        return None;
    }
    let n = c.control_points.len();
    let last = matches!(end, Endpoint::End);
    let d = c.domain();
    if !c.knots[..=c.degree].iter().all(|&t| t == d[0]) || !c.knots[n..].iter().all(|&t| t == d[1])
    {
        return None;
    }
    let count = (c.degree + 1).min(3);
    let points = if last {
        &c.control_points[n - count..]
    } else {
        &c.control_points[..count]
    };
    (0..points[0].len()).find(|&axis| {
        (0..points[0].len()).all(|k| k == axis || points.iter().all(|p| p[k] == points[0][k]))
    })
}
fn axial_direction(c: &Curve, end: Endpoint, reference: bool) -> Option<(usize, f64)> {
    let axis = axial_endpoint(c, end)?;
    let last = matches!(end, Endpoint::End);
    let n = c.control_points.len();
    let (a, b) = if last { (n - 2, n - 1) } else { (0, 1) };
    let a = c.control_points[a][axis];
    let b = c.control_points[b][axis];
    let sign = if b > a {
        1.
    } else if b < a {
        -1.
    } else {
        return None;
    };
    Some((axis, if reference != last { -sign } else { sign }))
}
fn differential(j: &[Vec<I>], axis: Option<(usize, f64)>) -> Result<Option<(Vec<I>, Vec<I>)>> {
    if let Some((axis, sign)) = axis {
        // Clamped original control identity: both endpoint derivatives lie on
        // this coordinate axis, for ANY rational weights. The actual first
        // derivative has the exact sign of the original adjacent pole
        // difference: p*w1/w0/delta_u is strictly positive after validation.
        // No near-zero component or sampled jet is rounded away.
        let mut tangent = vec![I::point(0.); j[1].len()];
        tangent[axis] = I::point(sign);
        return Ok(Some((tangent, vec![I::point(0.); j[1].len()])));
    }
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
    if let (Some((ta, ka)), Some((tb, kb))) = (
        differential(&a, axial_direction(reference, reference_end, true))?,
        differential(&b, axial_direction(edited, edited_end, false))?,
    ) {
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

#[cfg(test)]
mod axial_tests {
    use super::*;
    #[test]
    fn only_exact_clamped_endpoint_jets_have_zero_normal_acceleration() {
        let mut c = Curve::from_polyline(vec![vec![0.; 3], vec![1., 0., 0.], vec![2., 0., 0.]])
            .unwrap()
            .elevate(3)
            .unwrap();
        assert_eq!(axial_endpoint(&c, Endpoint::Start), Some(0));
        c.control_points[1][1] = f64::from_bits(1);
        assert_eq!(axial_endpoint(&c, Endpoint::Start), None);
        c.control_points[1][1] = 0.;
        c.control_points[1] = c.control_points[0].clone();
        let j = jets(&c, Endpoint::Start, false).unwrap();
        assert!(
            differential(&j, axial_direction(&c, Endpoint::Start, false))
                .unwrap()
                .is_none()
        );
    }
}
