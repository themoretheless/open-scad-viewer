//! Adaptive original-curve/original-normal secants for contact-ball laws.
use super::*;
use nurbs_core::{curve_secant_trace, surface_normal_trace};
type V = [I; 3];
fn vector(b: [[f64; 2]; 3]) -> Result<V> {
    Ok([
        I::new(b[0][0], b[0][1])?,
        I::new(b[1][0], b[1][1])?,
        I::new(b[2][0], b[2][1])?,
    ])
}
fn law(c: &nurbs_core::curve::Curve, q: f64, r: [f64; 2], domain: [f64; 2]) -> Result<I> {
    let d = c.domain();
    let t = if q == 0. {
        I::point(d[0])
    } else if q == 1. {
        I::point(d[1])
    } else {
        I::point(d[0]).add(I::point(d[1]).sub(I::point(d[0]))?.mul(I::point(q))?)?
    };
    let t = t
        .sub(I::point(domain[0]))?
        .div(I::point(domain[1]).sub(I::point(domain[0]))?)?;
    I::point(r[0]).add(t.mul(I::point(r[1]).sub(I::point(r[0]))?)?)
}
struct Bounds {
    upper: f64,
    lower: f64,
    at: f64,
}
fn difference_curve(
    contacts: [ContactUse<'_>; 2],
    cells: &mut usize,
    max: usize,
) -> Result<Option<(nurbs_core::curve::Curve, f64)>> {
    let a = contacts[0].curve;
    let b = contacts[1].curve;
    if a.degree != b.degree || a.knots != b.knots || a.weights != b.weights {
        return Ok(None);
    }
    let mut points = Vec::new();
    let mut error = 0_f64;
    for (a, b) in a.control_points.iter().zip(&b.control_points) {
        if *cells == max {
            return Ok(None);
        }
        *cells += 1;
        let p = (0..3).map(|k| b[k] - a[k]).collect::<Vec<_>>();
        if p.iter().any(|x| !x.is_finite()) {
            return Ok(None);
        }
        let residual = (0..3)
            .map(|k| I::point(b[k]).sub(I::point(a[k]))?.sub(I::point(p[k])))
            .collect::<Result<Vec<_>>>()?;
        error = error.max(norm_bounds(&residual)?[1]);
        points.push(p);
    }
    Ok(Some((
        nurbs_core::curve::Curve {
            control_points: points,
            ..a.clone()
        },
        error,
    )))
}
fn taylor(
    supports: [&Surface; 2],
    contacts: [ContactUse<'_>; 2],
    r: [f64; 2],
    domain: [f64; 2],
    sides: [f64; 2],
    q: [f64; 2],
    difference: Option<&(nurbs_core::curve::Curve, f64)>,
    position_error: f64,
    cells: &mut usize,
    max: usize,
) -> Result<Option<Bounds>> {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut error = I::point(position_error);
    let mut difference_ends = None;
    let mut witness_error = position_error;
    if let Some((curve, coefficient_error)) = difference {
        if *cells == max {
            return Ok(None);
        }
        let p = curve_secant_trace::inspect(curve, q, false, max - *cells)?;
        *cells += p.cells;
        let (Some(end), Some(e)) = (p.endpoints, p.secant_error_upper) else {
            return Ok(None);
        };
        difference_ends = Some([vector(end[0])?, vector(end[1])?]);
        error = error.add(I::point(e))?.add(I::point(*coefficient_error))?;
        witness_error = I::point(witness_error)
            .add(I::point(*coefficient_error))?
            .hi;
    }
    for side in 0..2 {
        if difference_ends.is_none() {
            if *cells == max {
                return Ok(None);
            }
            let p = curve_secant_trace::inspect(contacts[side].curve, q, false, max - *cells)?;
            *cells += p.cells;
            let (Some(end), Some(e)) = (p.endpoints, p.secant_error_upper) else {
                return Ok(None);
            };
            positions.push([vector(end[0])?, vector(end[1])?]);
            error = error.add(I::point(e))?;
        }
        if *cells == max {
            return Ok(None);
        }
        let n = surface_normal_trace::inspect(
            contacts[side].support_pcurve,
            supports[side],
            q,
            contacts[side].reversed[0],
            max - *cells,
        )?;
        *cells += n.cells;
        let (Some(end), Some(e)) = (n.endpoints, n.secant_error_upper) else {
            return Ok(None);
        };
        normals.push([vector(end[0])?, vector(end[1])?]);
        error = error.add(I::point(e).mul(I::point(r[0].max(r[1])))?)?;
    }
    let radii = [
        [
            law(contacts[0].curve, q[0], r, domain)?,
            law(contacts[0].curve, q[1], r, domain)?,
        ],
        [
            law(contacts[1].curve, q[0], r, domain)?,
            law(contacts[1].curve, q[1], r, domain)?,
        ],
    ];
    let mut upper = 0_f64;
    let mut lower = 0_f64;
    let mut at = q[0];
    for pole in 0..3 {
        let mut residual = [I::point(0.); 3];
        for k in 0..3 {
            let delta = |end: usize| match &difference_ends {
                Some(points) => Ok(points[end][k]),
                None => positions[1][end][k].sub(positions[0][end][k]),
            };
            let product = |side: usize| -> Result<I> {
                let value = if pole == 1 {
                    radii[side][0]
                        .mul(normals[side][1][k])?
                        .add(radii[side][1].mul(normals[side][0][k])?)?
                        .mul(I::point(0.5))?
                } else {
                    radii[side][pole / 2].mul(normals[side][pole / 2][k])?
                };
                value.mul(I::point(sides[side].signum()))
            };
            let value = if pole == 1 {
                delta(0)?.add(delta(1)?)?.mul(I::point(0.5))?
            } else {
                delta(pole / 2)?
            };
            residual[k] = value.sub(product(0)?)?.add(product(1)?)?;
        }
        let b = norm_bounds(&residual)?;
        upper = upper.max(b[1]);
        if pole != 1 {
            let witness = I::point(b[0]).sub(I::point(witness_error))?.lo.max(0.);
            if witness > lower {
                lower = witness;
                at = q[pole / 2];
            }
        }
    }
    Ok(Some(Bounds {
        upper: I::point(upper).add(error)?.hi,
        lower,
        at,
    }))
}
fn coarse(
    supports: [&Surface; 2],
    contacts: [ContactUse<'_>; 2],
    r: [f64; 2],
    domain: [f64; 2],
    sides: [f64; 2],
    q: [f64; 2],
    position_error: f64,
    cells: &mut usize,
    max: usize,
) -> Result<Option<Bounds>> {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    for side in 0..2 {
        if *cells == max {
            return Ok(None);
        }
        let p = curve_secant_trace::range_bounds(contacts[side].curve, q, false, max - *cells)?;
        *cells += p.cells;
        let Some(p) = p.bounds else { return Ok(None) };
        positions.push(vector(p)?);
        if *cells == max {
            return Ok(None);
        }
        let n = surface_normal_trace::range_bounds(
            contacts[side].support_pcurve,
            supports[side],
            q,
            contacts[side].reversed[0],
            max - *cells,
        )?;
        *cells += n.cells;
        let Some(n) = n.normal else { return Ok(None) };
        normals.push(vector(n)?);
    }
    let mut radius = Vec::new();
    for c in contacts {
        let a = law(c.curve, q[0], r, domain)?;
        let b = law(c.curve, q[1], r, domain)?;
        radius.push(I::new(a.lo.min(b.lo), a.hi.max(b.hi))?);
    }
    let mut residual = [I::point(0.); 3];
    for k in 0..3 {
        residual[k] = positions[1][k]
            .sub(positions[0][k])?
            .sub(
                radius[0]
                    .mul(normals[0][k])?
                    .mul(I::point(sides[0].signum()))?,
            )?
            .add(
                radius[1]
                    .mul(normals[1][k])?
                    .mul(I::point(sides[1].signum()))?,
            )?;
    }
    let b = norm_bounds(&residual)?;
    Ok(Some(Bounds {
        upper: I::point(b[1]).add(I::point(position_error))?.hi,
        lower: I::point(b[0]).sub(I::point(position_error))?.lo.max(0.),
        at: q[0] * 0.5 + q[1] * 0.5,
    }))
}
pub(super) fn inspect(
    supports: [&Surface; 2],
    contacts: [ContactUse<'_>; 2],
    r: [f64; 2],
    domain: [f64; 2],
    sides: [f64; 2],
    position_error: f64,
    mut out: Report,
    tolerance: f64,
    max: usize,
) -> Result<Report> {
    out.maximum_center_gap_mm = None;
    out.reason = "work-limit";
    let difference = difference_curve(contacts, &mut out.cells, max)?;
    let mut pending = vec![[0., 1.]];
    let mut upper = 0_f64;
    let mut lower = 0_f64;
    while let Some(q) = pending.pop() {
        if out.cells == max {
            return Ok(out);
        }
        out.cells += 1;
        let bound = match taylor(
            supports,
            contacts,
            r,
            domain,
            sides,
            q,
            difference.as_ref(),
            position_error,
            &mut out.cells,
            max,
        )? {
            Some(b) => Some(b),
            None => coarse(
                supports,
                contacts,
                r,
                domain,
                sides,
                q,
                position_error,
                &mut out.cells,
                max,
            )?,
        };
        if let Some(b) = bound {
            if b.lower > tolerance {
                out.decision = Decision::ExceedsTolerance;
                out.reason = "original-normal-trace-violation";
                out.violation_witness = Some([b.at, b.lower]);
                return Ok(out);
            }
            if b.upper <= tolerance {
                upper = upper.max(b.upper);
                lower = lower.max(b.lower);
                continue;
            }
        }
        let middle = q[0] * 0.5 + q[1] * 0.5;
        if !(q[0] < middle && middle < q[1]) {
            out.reason = "parameter-precision-limit";
            return Ok(out);
        }
        pending.push([middle, q[1]]);
        pending.push([q[0], middle]);
    }
    out.maximum_center_gap_mm = Some([lower, upper]);
    out.decision = Decision::WithinTolerance;
    out.reason = "complete-original-normal-secant-bound";
    Ok(out)
}
