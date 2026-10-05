//! Source UV curves must follow the same isolated contact branch as the patch.
//! World-space proximity alone can accept a wrong chart on overlapping faces.
use crate::{
    Result, check,
    curve::Curve,
    distance_bounds::Interval as I,
    offset_contact_tangent,
    surface::Surface,
    surface_contact::{Verdict, Witness},
    surface_offset,
};
pub struct Cell {
    pub interval: [f64; 2],
    pub error_upper_uv: Option<f64>,
    pub anchor_error_uv: Option<[f64; 2]>,
    pub admitted: bool,
}
pub struct Report {
    pub side: usize,
    pub cells: Vec<Cell>,
    pub visited: usize,
    pub band_queries: usize,
    pub section_queries: usize,
    pub correspondence_proven: bool,
    pub reason: &'static str,
}
impl Report {
    pub fn to_value(&self) -> value_codec::Value {
        use value_codec::json;
        json!({"method":"interval-contact-pcurve-correspondence","scope":"original-source-parameter-branch","side":self.side,
            "sourceParameterCorrespondenceProven":self.correspondence_proven,"reason":self.reason,
            "visitedCells":self.visited,"bandQueries":self.band_queries,"sectionQueries":self.section_queries,
            "cells":self.cells.iter().map(|c|json!({"parameterInterval":c.interval,"errorUpperUv":c.error_upper_uv,
                "anchorErrorIntervalUv":c.anchor_error_uv,"admitted":c.admitted})).collect::<Vec<_>>(),
            "wholeCurveComplete":false,"trimMembershipProven":false,"worldAgreementProven":false,"topologyAuthority":false})
    }
}
fn lift(p: &Curve) -> Result<Surface> {
    p.validate()?;
    check(
        p.control_points[0].len() == 2,
        "Contact pcurve must be two-dimensional",
    )?;
    let s = Surface {
        degree_u: p.degree,
        degree_v: 1,
        knots_u: p.knots.clone(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: p
            .control_points
            .iter()
            .map(|uv| vec![vec![uv[0], uv[1], 0.]; 2])
            .collect(),
        weights: p.weights.iter().map(|w| vec![*w; 2]).collect(),
        periodic_u: p.periodic,
        periodic_v: false,
    };
    s.validate()?;
    Ok(s)
}
fn interval(x: [f64; 2]) -> Result<I> {
    I::new(x[0], x[1])
}
/// Additional section queries refine only an already certified root enclosure.
/// If the tighter query is inconclusive, retain the last certified enclosure.
fn section(
    s: [&Surface; 2],
    d: [f64; 2],
    axis: usize,
    t: f64,
    free: [f64; 2],
    second: [[f64; 2]; 2],
    spans: usize,
    refinements: usize,
) -> Result<(Option<Witness>, usize)> {
    let mut calls = 1;
    let Verdict::Witness(mut witness) =
        surface_offset::certify_contact_section(s, d, axis, t, free, second, spans)?
    else {
        return Ok((None, calls));
    };
    for _ in 0..refinements {
        let next_free = witness.first_uv[1 - axis];
        let next_second = witness.second_uv;
        if next_free[0] >= next_free[1] || next_second.iter().any(|r| r[0] >= r[1]) {
            break;
        }
        calls += 1;
        match surface_offset::certify_contact_section(s, d, axis, t, next_free, next_second, spans)?
        {
            Verdict::Witness(w) => witness = w,
            _ => break,
        }
    }
    Ok((Some(witness), calls))
}
/// A linear source pcurve is a proposal. Full-band correspondence is separate.
pub fn propose(
    s: [&Surface; 2],
    d: [f64; 2],
    axis: usize,
    drive: [f64; 2],
    free: [f64; 2],
    second: [[f64; 2]; 2],
    spans: usize,
    refinements: usize,
) -> Result<Option<[Curve; 2]>> {
    check(
        axis < 2 && drive[0] < drive[1] && refinements <= 16,
        "Choose valid contact pcurve parameters and at most 16 refinements",
    )?;
    let mut uv = [vec![], vec![]];
    for t in drive {
        let (Some(w), _) = section(s, d, axis, t, free, second, spans, refinements)? else {
            return Ok(None);
        };
        let mid = |x: [f64; 2]| x[0] + (x[1] - x[0]) * 0.5;
        let mut first = w.first_uv.map(mid);
        first[axis] = t;
        uv[0].push(first.to_vec());
        uv[1].push(w.second_uv.map(mid).to_vec());
    }
    Ok(Some(uv.map(|control_points| Curve {
        degree: 1,
        knots: vec![drive[0], drive[0], drive[1], drive[1]],
        control_points,
        weights: vec![1.; 2],
        periodic: false,
    })))
}
/// Full-interval UV correspondence in the original driving source parameter.
/// `max_cells` bounds all evaluated nodes; root refinements have a separate cap.
/// Failed leaves retain complete coverage. This is not a trim-loop or topology proof.
pub fn certify(
    p: &Curve,
    side: usize,
    s: [&Surface; 2],
    d: [f64; 2],
    axis: usize,
    drive: [f64; 2],
    free: [f64; 2],
    second: [[f64; 2]; 2],
    spans: usize,
    tolerance_uv: f64,
    max_cells: usize,
    refinements: usize,
) -> Result<Report> {
    check(
        side < 2 && axis < 2 && drive[0] < drive[1],
        "Choose valid contact side, axis and driving interval",
    )?;
    check(
        tolerance_uv.is_finite()
            && tolerance_uv > 0.
            && max_cells > 0
            && max_cells <= 100000
            && refinements <= 16,
        "Contact pcurve needs a positive UV tolerance and bounded work",
    )?;
    let lifted = lift(p)?;
    check(
        p.domain() == drive,
        "Contact pcurve domain must equal the original driving interval",
    )?;
    let smooth = offset_contact_tangent::differentiable(&lifted, [drive, [0., 0.]], 1);
    let mut out = Report {
        side,
        cells: vec![],
        visited: 0,
        band_queries: 0,
        section_queries: 0,
        correspondence_proven: false,
        reason: "pcurve-work-limit",
    };
    let mut pending = vec![drive];
    let mut mismatch = false;
    while let Some(range) = pending.pop() {
        out.visited += 1;
        out.band_queries += 1;
        let tangent = offset_contact_tangent::certify(s, d, axis, range, free, second, spans)?;
        let mut cell = Cell {
            interval: range,
            error_upper_uv: None,
            anchor_error_uv: None,
            admitted: false,
        };
        if smooth && tangent.regular {
            if let Some(parameters) = tangent.derivatives {
                let jets = surface_offset::jacobian_bounds(&lifted, [range, [0., 0.]], 0., spans)?;
                if let Some(jets) = jets.derivatives {
                    let target = if side == 0 {
                        if axis == 0 {
                            [[1., 1.], parameters[0]]
                        } else {
                            [parameters[0], [1., 1.]]
                        }
                    } else {
                        [parameters[1], parameters[2]]
                    };
                    let tm = range[0] + (range[1] - range[0]) * 0.5;
                    let (anchor, calls) =
                        section(s, d, axis, tm, free, second, spans, refinements)?;
                    out.section_queries += calls;
                    if let Some(anchor) = anchor {
                        let root = if side == 0 {
                            anchor.first_uv
                        } else {
                            anchor.second_uv
                        };
                        let pc = crate::curve_surface_agreement::curve_bounds(p, I::point(tm))?;
                        let mut error = [I::point(0.); 3];
                        for k in 0..2 {
                            error[k] = pc[k].sub(interval(root[k])?)?;
                        }
                        let anchor_error = offset_contact_tangent::speed(error)?;
                        cell.anchor_error_uv = Some([anchor_error.lo, anchor_error.hi]);
                        mismatch |= anchor_error.lo > tolerance_uv;
                        let delta = interval(range)?.sub(I::point(tm))?;
                        for k in 0..2 {
                            let derivative = interval(jets[0][k])?.sub(interval(target[k])?)?;
                            error[k] = error[k].add(derivative.mul(delta)?)?;
                        }
                        let upper = offset_contact_tangent::speed(error)?.hi;
                        cell.error_upper_uv = Some(upper);
                        cell.admitted = upper <= tolerance_uv;
                    }
                }
            }
        }
        let mid = range[0] + (range[1] - range[0]) * 0.5;
        let disproven = cell.anchor_error_uv.is_some_and(|e| e[0] > tolerance_uv);
        if !cell.admitted
            && !disproven
            && smooth
            && mid > range[0]
            && mid < range[1]
            && out.visited + pending.len() + 2 <= max_cells
        {
            pending.push([mid, range[1]]);
            pending.push([range[0], mid]);
        } else {
            out.cells.push(cell);
        }
    }
    out.correspondence_proven = out.cells.iter().all(|c| c.admitted);
    out.reason = if out.correspondence_proven {
        "pcurve-follows-contact-branch"
    } else if mismatch {
        "pcurve-contact-mismatch"
    } else if !smooth {
        "pcurve-continuity-unproven"
    } else {
        "pcurve-work-limit"
    };
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn pair() -> [Surface; 2] {
        let a = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let mut b = a.clone();
        for row in &mut b.control_points {
            for p in row {
                let z = p[1];
                p[1] = 0.5;
                p[2] = z;
            }
        }
        [a, b]
    }
    #[test]
    fn proposed_uv_curves_follow_both_original_contact_parameters() {
        let [a, b] = pair();
        let curves = propose(
            [&a, &b],
            [0.2, 0.2],
            0,
            [0.35, 0.39],
            [0.25, 0.35],
            [[0.30, 0.44], [0.15, 0.25]],
            2,
            3,
        )
        .unwrap()
        .unwrap();
        for side in 0..2 {
            let r = certify(
                &curves[side],
                side,
                [&a, &b],
                [0.2, 0.2],
                0,
                [0.35, 0.39],
                [0.25, 0.35],
                [[0.30, 0.44], [0.15, 0.25]],
                2,
                1e-8,
                31,
                3,
            )
            .unwrap();
            assert!(r.correspondence_proven, "{}", r.reason);
            assert!(r.band_queries == r.visited);
            assert!(r.section_queries <= 4 * r.visited);
        }
    }
    #[test]
    fn overlapping_source_chart_is_rejected_even_when_world_points_coincide() {
        let [a, mut b] = pair();
        b.control_points.push(b.control_points[0].clone());
        b.weights.push(vec![1.; 2]);
        b.knots_u = vec![0., 0., 1., 2., 2.];
        let curves = propose(
            [&a, &b],
            [0.2, 0.2],
            0,
            [0.35, 0.39],
            [0.25, 0.35],
            [[0.30, 0.44], [0.15, 0.25]],
            2,
            3,
        )
        .unwrap()
        .unwrap();
        let mut wrong = curves[1].clone();
        for p in &mut wrong.control_points {
            p[0] = 2. - p[0];
        }
        for t in [0.35, 0.37, 0.39] {
            let uv = curves[1].evaluate(t).unwrap().point;
            let other = wrong.evaluate(t).unwrap().point;
            let p = b.evaluate(uv[0], uv[1]).unwrap().point;
            let q = b.evaluate(other[0], other[1]).unwrap().point;
            for k in 0..3 {
                assert!((p[k] - q[k]).abs() < 1e-14);
            }
        }
        let r = certify(
            &wrong,
            1,
            [&a, &b],
            [0.2, 0.2],
            0,
            [0.35, 0.39],
            [0.25, 0.35],
            [[0.30, 0.44], [0.15, 0.25]],
            2,
            1e-6,
            31,
            3,
        )
        .unwrap();
        assert!(!r.correspondence_proven);
        assert_eq!(r.reason, "pcurve-contact-mismatch");
    }
}
