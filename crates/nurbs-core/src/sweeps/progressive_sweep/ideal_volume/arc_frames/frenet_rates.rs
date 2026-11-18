//! Geometric Frenet--Serret rates in normalized arc units. Tangential source
//! acceleration changes speed, not curvature or angular speed of the frame.
use super::*;
use crate::sweeps::progressive_miter::{scalar_certificate, vector_certificate};
pub(crate) struct Rates {
    pub omega: f64,
    pub bend: f64,
    pub cells: usize,
}
pub(crate) fn certify(
    s: &Sweep<'_>,
    station: [f64; 2],
    p: frame::PathPhase,
    length: I,
    max: usize,
) -> Result<(Option<Rates>, usize)> {
    let mut cells = 0;
    let twist = scalar_certificate::certify_traversal(s.twist, station, max)?;
    cells += twist.cells;
    if twist.status != Status::Certified {
        return Ok((None, cells));
    }
    let [a, b] = s.twist.domain();
    let theta = I::new(twist.first.unwrap()[0], twist.first.unwrap()[1])?
        .mul(I::point(b).sub(I::point(a))?)?;
    let [a, b] = s.path.domain();
    let width = I::point(b).sub(I::point(a))?;
    let raw = I::point(a)
        .add(width.mul(I::new(p.traversal[0], p.traversal[1])?)?)?
        .intersect(a, b)?;
    let mut omega = 0f64;
    let mut bend = 0f64;
    let mut count = 0;
    for span in s.path.degree..s.path.control_points.len() {
        let lo = raw.lo.max(s.path.knots[span]);
        let hi = raw.hi.min(s.path.knots[span + 1]);
        if s.path.knots[span] >= s.path.knots[span + 1] || lo > hi {
            continue;
        }
        let j = vector_certificate::certify_fourth_span(s.path, span, [lo, hi], max - cells)?;
        cells += j.base.cells;
        if j.base.status != Status::Certified {
            return Ok((None, cells));
        }
        let v = decode(j.base.first.unwrap())?;
        let a = decode(j.base.second.unwrap())?;
        let jerk = decode(j.third.unwrap())?;
        let speed = iv::norm(v)?;
        if speed.lo <= 0. {
            return Ok((None, cells));
        }
        // Use one fixed spatial scale for all three jets. Dividing V and A
        // independently by an interval speed before the cross product loses
        // their shared denominator and widens the curvature bound.
        let scale = I::point(speed.hi);
        let v = iv::div(v, scale)?;
        let a = iv::div(a, scale)?;
        let jerk = iv::div(jerk, scale)?;
        let speed = iv::norm(v)?;
        if speed.lo <= 0. {
            return Ok((None, cells));
        }
        let turn = iv::cross(v, a)?;
        let turn_norm = iv::norm(turn)?;
        if turn_norm.lo <= 0. {
            return Ok((None, cells));
        }
        let curvature = turn_norm.div(speed)?.div(speed)?.div(speed)?.div(scale)?;
        let mut binormal = iv::div(turn, turn_norm)?;
        for x in &mut binormal {
            *x = x.intersect(-1., 1.)?;
        }
        let torsion = iv::dot_tight(binormal, jerk)?.div(turn_norm)?.div(scale)?;
        let k = curvature.mul(length)?;
        let spin = torsion.mul(length)?.add(theta)?;
        omega = omega.max(iv::norm([k, spin, I::point(0.)])?.hi);
        bend = bend.max(k.hi);
        count += 1;
    }
    if count == 0 {
        return Ok((None, cells));
    }
    Ok((Some(Rates { omega, bend, cells }), cells))
}
