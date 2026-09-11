//! Polyline stroke expansion: caps, joins, optional dash → filled outline paths.
use crate::path::BezierPath;
use crate::{Result, check};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineCap {
    #[default]
    Butt,
    Round,
    Square,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineJoin {
    #[default]
    Miter,
    Round,
    Bevel,
}

/// Endpoint marker painted as filled path geometry (not a stroke cap).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArrowMarker {
    #[default]
    None,
    Arrow,
    Dot,
    Bar,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StrokeOptions {
    pub width: f64,
    pub cap: LineCap,
    pub join: LineJoin,
    pub miter_limit: f64,
    /// Alternating on/off lengths; empty / None = solid.
    pub dash: Option<Vec<f64>>,
    pub dash_offset: f64,
}

impl Default for StrokeOptions {
    fn default() -> Self {
        Self {
            width: 1.0,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            miter_limit: 4.0,
            dash: None,
            dash_offset: 0.0,
        }
    }
}

fn add(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] + b[0], a[1] + b[1]]
}
fn sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}
fn mul(a: [f64; 2], s: f64) -> [f64; 2] {
    [a[0] * s, a[1] * s]
}
fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}
fn norm(v: [f64; 2]) -> [f64; 2] {
    let l = v[0].hypot(v[1]).max(1e-15);
    [v[0] / l, v[1] / l]
}
fn perp(v: [f64; 2]) -> [f64; 2] {
    [-v[1], v[0]]
}

/// Stroke a path into one or more closed outline paths.
pub fn outline_stroke(path: &BezierPath, opts: &StrokeOptions) -> Result<Vec<BezierPath>> {
    check(
        opts.width > 0. && opts.width.is_finite(),
        "Invalid stroke width",
    )?;
    check(
        opts.miter_limit >= 1. && opts.miter_limit.is_finite(),
        "Invalid miter limit",
    )?;
    let pts = path.flatten()?;
    check(pts.len() >= 2, "Path too short to outline")?;
    let mut polylines = if let Some(dash) = &opts.dash {
        if dash.iter().any(|d| *d > 0. && d.is_finite()) {
            dash_polylines(&pts, path.closed, dash, opts.dash_offset)?
        } else {
            vec![(pts, path.closed)]
        }
    } else {
        vec![(pts, path.closed)]
    };
    // Dashed closed path yields open spans.
    for (_, closed) in &mut polylines {
        if opts.dash.is_some() {
            *closed = false;
        }
    }
    let half = opts.width * 0.5;
    let mut out = Vec::new();
    for (poly, closed) in polylines {
        if poly.len() < 2 {
            continue;
        }
        out.push(stroke_polyline(&poly, closed, half, opts)?);
    }
    check(!out.is_empty(), "Stroke produced no outline")?;
    Ok(out)
}

/// Closed marker path at `endpoint`, oriented along unit `outward` tangent.
/// Size is `4 × stroke_width` (Illustrator-style).
pub fn arrow_marker_path(
    kind: ArrowMarker,
    endpoint: [f64; 2],
    outward: [f64; 2],
    stroke_width: f64,
) -> Result<Option<BezierPath>> {
    check(
        stroke_width > 0. && stroke_width.is_finite(),
        "Invalid marker stroke width",
    )?;
    check(
        endpoint.iter().chain(outward.iter()).all(|x| x.is_finite()),
        "Non-finite marker geometry",
    )?;
    let len = outward[0].hypot(outward[1]);
    check(len > 1e-12, "Degenerate marker tangent")?;
    let dir = [outward[0] / len, outward[1] / len];
    let s = (stroke_width * 4.0).max(stroke_width);
    Ok(Some(match kind {
        ArrowMarker::None => return Ok(None),
        ArrowMarker::Arrow => {
            let perp = [-dir[1], dir[0]];
            let tip = endpoint;
            let base = [endpoint[0] - dir[0] * s, endpoint[1] - dir[1] * s];
            let b1 = [base[0] + perp[0] * s * 0.5, base[1] + perp[1] * s * 0.5];
            let b2 = [base[0] - perp[0] * s * 0.5, base[1] - perp[1] * s * 0.5];
            BezierPath::from_polyline(&[tip, b1, b2], true)?
        }
        ArrowMarker::Dot => BezierPath::from_circle(endpoint, s * 0.5)?,
        ArrowMarker::Bar => {
            let perp = [-dir[1], dir[0]];
            let half_len = s * 0.6;
            let half_thick = (stroke_width * 0.5).max(stroke_width * 0.25);
            let along = [dir[0] * half_thick, dir[1] * half_thick];
            let across = [perp[0] * half_len, perp[1] * half_len];
            BezierPath::from_polyline(
                &[
                    [
                        endpoint[0] + across[0] + along[0],
                        endpoint[1] + across[1] + along[1],
                    ],
                    [
                        endpoint[0] - across[0] + along[0],
                        endpoint[1] - across[1] + along[1],
                    ],
                    [
                        endpoint[0] - across[0] - along[0],
                        endpoint[1] - across[1] - along[1],
                    ],
                    [
                        endpoint[0] + across[0] - along[0],
                        endpoint[1] + across[1] - along[1],
                    ],
                ],
                true,
            )?
        }
    }))
}

/// Start / end markers for an open path (closed paths have no free ends).
pub fn path_arrow_markers(
    path: &BezierPath,
    start: ArrowMarker,
    end: ArrowMarker,
    stroke_width: f64,
) -> Result<Vec<BezierPath>> {
    if start == ArrowMarker::None && end == ArrowMarker::None {
        return Ok(Vec::new());
    }
    check(!path.closed, "Arrow markers apply to open paths")?;
    let pts = path.flatten()?;
    check(pts.len() >= 2, "Path too short for markers")?;
    let mut out = Vec::new();
    if start != ArrowMarker::None {
        let d = sub(pts[0], pts[1]);
        if let Some(p) = arrow_marker_path(start, pts[0], d, stroke_width)? {
            out.push(p);
        }
    }
    if end != ArrowMarker::None {
        let n = pts.len();
        let d = sub(pts[n - 1], pts[n - 2]);
        if let Some(p) = arrow_marker_path(end, pts[n - 1], d, stroke_width)? {
            out.push(p);
        }
    }
    Ok(out)
}

fn dash_polylines(
    pts: &[[f64; 2]],
    closed: bool,
    pattern: &[f64],
    offset: f64,
) -> Result<Vec<(Vec<[f64; 2]>, bool)>> {
    let mut pattern: Vec<f64> = pattern
        .iter()
        .copied()
        .filter(|d| d.is_finite() && *d > 0.)
        .collect();
    check(!pattern.is_empty(), "Empty dash pattern")?;
    if pattern.len() % 2 == 1 {
        pattern.push(pattern[pattern.len() - 1]);
    }
    let period: f64 = pattern.iter().sum();
    check(period > 0., "Degenerate dash period")?;
    let mut edges = Vec::new();
    let n = if closed { pts.len() } else { pts.len() - 1 };
    for i in 0..n {
        let a = pts[i];
        let b = pts[(i + 1) % pts.len()];
        let len = dist(a, b);
        if len > 1e-12 {
            edges.push((a, b, len));
        }
    }
    check(!edges.is_empty(), "No edges to dash")?;
    let total: f64 = edges.iter().map(|e| e.2).sum();
    let off = offset.rem_euclid(period);
    let mut phase_idx = 0usize;
    let mut t = 0.0;
    for (i, d) in pattern.iter().enumerate() {
        if off < t + *d {
            phase_idx = i;
            break;
        }
        t += *d;
    }
    let mut remain = pattern[phase_idx] - (off - t);
    let mut drawing = phase_idx % 2 == 0;
    let mut out = Vec::new();
    let mut cur: Vec<[f64; 2]> = Vec::new();

    let sample = |dist_along: f64| -> [f64; 2] {
        let mut d = dist_along.clamp(0.0, total);
        for &(a, b, len) in &edges {
            if d <= len {
                let t = d / len;
                return [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
            }
            d -= len;
        }
        edges.last().map(|e| e.1).unwrap_or(pts[0])
    };

    let mut walk = 0.0_f64;
    while walk < total - 1e-12 {
        let step = remain.min(total - walk);
        let a = sample(walk);
        let b = sample(walk + step);
        if drawing {
            if cur.is_empty() {
                cur.push(a);
            }
            if dist(*cur.last().unwrap(), b) > 1e-12 {
                cur.push(b);
            }
        } else if cur.len() >= 2 {
            out.push((std::mem::take(&mut cur), false));
        } else {
            cur.clear();
        }
        walk += step;
        remain -= step;
        if remain <= 1e-12 {
            phase_idx = (phase_idx + 1) % pattern.len();
            remain = pattern[phase_idx];
            drawing = phase_idx % 2 == 0;
            if !drawing && cur.len() >= 2 {
                out.push((std::mem::take(&mut cur), false));
            }
        }
    }
    if cur.len() >= 2 {
        out.push((cur, false));
    }
    check(out.len() <= 4096, "Dash produced too many spans")?;
    Ok(out)
}

fn stroke_polyline(
    pts: &[[f64; 2]],
    closed: bool,
    half: f64,
    opts: &StrokeOptions,
) -> Result<BezierPath> {
    let n = pts.len();
    check(n >= 2, "Polyline too short")?;
    let mut left = Vec::with_capacity(n + 8);
    let mut right = Vec::with_capacity(n + 8);
    let dir = |i: usize, j: usize| norm(sub(pts[j % n], pts[i % n]));

    for i in 0..n {
        if !closed && (i == 0 || i == n - 1) {
            let (a, b) = if i == 0 {
                (pts[0], pts[1])
            } else {
                (pts[n - 2], pts[n - 1])
            };
            let d = norm(sub(b, a));
            let nr = mul(perp(d), half);
            left.push(add(pts[i], nr));
            right.push(sub(pts[i], nr));
            continue;
        }
        let prev = if i == 0 { n - 1 } else { i - 1 };
        let next = (i + 1) % n;
        let d0 = dir(prev, i);
        let d1 = dir(i, next);
        let n0 = mul(perp(d0), half);
        let n1 = mul(perp(d1), half);
        let (l, r) = join_offsets(pts[i], d0, d1, n0, n1, opts)?;
        left.push(l);
        right.push(r);
    }

    let mut outline = Vec::new();
    if closed {
        outline.extend(left);
        outline.extend(right.iter().rev().copied());
        return BezierPath::from_polyline(&outline, true);
    }

    // Start cap
    match opts.cap {
        LineCap::Butt => {}
        LineCap::Square => {
            let d = norm(sub(pts[1], pts[0]));
            let ext = mul(d, -half);
            left[0] = add(left[0], ext);
            right[0] = add(right[0], ext);
        }
        LineCap::Round => {
            // semicircle from right[0] to left[0] around pts[0]
            let c = pts[0];
            let a = right[0];
            let b = left[0];
            outline.push(a);
            append_arc(&mut outline, c, a, b, half, true);
            // then walk left, end cap, reverse right
            outline.extend_from_slice(&left);
            let a2 = *left.last().unwrap();
            let b2 = *right.last().unwrap();
            append_arc(&mut outline, pts[n - 1], a2, b2, half, true);
            for p in right.iter().rev().skip(1) {
                outline.push(*p);
            }
            return BezierPath::from_polyline(&outline, true);
        }
    }
    // End square cap adjustment
    if opts.cap == LineCap::Square {
        let d = norm(sub(pts[n - 1], pts[n - 2]));
        let ext = mul(d, half);
        let li = left.len() - 1;
        let ri = right.len() - 1;
        left[li] = add(left[li], ext);
        right[ri] = add(right[ri], ext);
    }
    outline.extend(left);
    outline.extend(right.iter().rev().copied());
    BezierPath::from_polyline(&outline, true)
}

fn join_offsets(
    p: [f64; 2],
    d0: [f64; 2],
    d1: [f64; 2],
    n0: [f64; 2],
    n1: [f64; 2],
    opts: &StrokeOptions,
) -> Result<([f64; 2], [f64; 2])> {
    let cross = d0[0] * d1[1] - d0[1] * d1[0];
    let dot = d0[0] * d1[0] + d0[1] * d1[1];
    // Nearly straight
    if cross.abs() < 1e-10 && dot > 0. {
        return Ok((add(p, n0), sub(p, n0)));
    }
    let left0 = add(p, n0);
    let left1 = add(p, n1);
    let right0 = sub(p, n0);
    let right1 = sub(p, n1);

    let miter_left = line_intersect(left0, add(left0, d0), left1, add(left1, d1));
    let miter_right = line_intersect(right0, add(right0, d0), right1, add(right1, d1));

    match opts.join {
        LineJoin::Bevel => Ok((left1, right1)), // simplified: use outgoing offset
        LineJoin::Round => {
            // Approximate round join with miter clipped toward bevel
            let l = miter_left.unwrap_or(left1);
            let r = miter_right.unwrap_or(right1);
            let half = n0[0].hypot(n0[1]);
            let l = if dist(p, l) > half * opts.miter_limit {
                left1
            } else {
                l
            };
            let r = if dist(p, r) > half * opts.miter_limit {
                right1
            } else {
                r
            };
            Ok((l, r))
        }
        LineJoin::Miter => {
            let half = n0[0].hypot(n0[1]);
            let l = match miter_left {
                Some(q) if dist(p, q) <= half * opts.miter_limit => q,
                _ => left1,
            };
            let r = match miter_right {
                Some(q) if dist(p, q) <= half * opts.miter_limit => q,
                _ => right1,
            };
            Ok((l, r))
        }
    }
}

fn line_intersect(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> Option<[f64; 2]> {
    let r = sub(b, a);
    let s = sub(d, c);
    let den = r[0] * s[1] - r[1] * s[0];
    if den.abs() < 1e-15 {
        return None;
    }
    let qp = sub(c, a);
    let t = (qp[0] * s[1] - qp[1] * s[0]) / den;
    Some([a[0] + t * r[0], a[1] + t * r[1]])
}

fn append_arc(
    out: &mut Vec<[f64; 2]>,
    center: [f64; 2],
    from: [f64; 2],
    to: [f64; 2],
    radius: f64,
    ccw: bool,
) {
    let a0 = (from[1] - center[1]).atan2(from[0] - center[0]);
    let a1 = (to[1] - center[1]).atan2(to[0] - center[0]);
    let mut delta = a1 - a0;
    if ccw && delta <= 0. {
        delta += std::f64::consts::TAU;
    }
    if !ccw && delta >= 0. {
        delta -= std::f64::consts::TAU;
    }
    let steps = ((delta.abs() / (std::f64::consts::PI * 0.25)).ceil() as usize).clamp(2, 16);
    for i in 1..=steps {
        let t = i as f64 / steps as f64;
        let a = a0 + delta * t;
        out.push([center[0] + radius * a.cos(), center[1] + radius * a.sin()]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rings::area;

    #[test]
    fn solid_rect_stroke_area() {
        let path = BezierPath::from_rect([0., 0.], [10., 0.]).unwrap(); // degenerate height - use line
        let line = BezierPath::from_polyline(&[[0., 0.], [10., 0.]], false).unwrap();
        let outlines = outline_stroke(
            &line,
            &StrokeOptions {
                width: 2.,
                cap: LineCap::Butt,
                join: LineJoin::Miter,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(outlines.len(), 1);
        let ring = outlines[0].to_ring(0.05).unwrap();
        let a: f64 = area(&ring).abs();
        assert!((a - 20.).abs() < 1.5, "area={a}");
        let _ = path;
    }

    #[test]
    fn dashed_produces_multiple() {
        let line = BezierPath::from_polyline(&[[0., 0.], [20., 0.]], false).unwrap();
        let outs = outline_stroke(
            &line,
            &StrokeOptions {
                width: 1.,
                dash: Some(vec![4., 4.]),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(outs.len() >= 2, "spans={}", outs.len());
    }

    #[test]
    fn round_cap_closed_ok() {
        let line = BezierPath::from_polyline(&[[0., 0.], [5., 0.]], false).unwrap();
        let outs = outline_stroke(
            &line,
            &StrokeOptions {
                width: 2.,
                cap: LineCap::Round,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(outs[0].closed);
    }

    #[test]
    fn arrow_and_bar_are_closed() {
        let line = BezierPath::from_polyline(&[[0., 0.], [10., 0.]], false).unwrap();
        let marks = path_arrow_markers(&line, ArrowMarker::Arrow, ArrowMarker::Bar, 1.).unwrap();
        assert_eq!(marks.len(), 2);
        assert!(marks.iter().all(|p| p.closed));
    }
}
