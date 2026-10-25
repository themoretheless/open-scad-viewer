//! Polyline stroke expansion: caps, joins, optional dash → filled outline paths.
use crate::path::BezierPath;
use crate::{Result, check};
use math_core::{add2, norm2, scale2, sub2, unit2};

#[path = "stroke_simple.rs"]
mod simple;

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
    /// Open chevron (two strokes as a filled V).
    Chevron,
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

fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    norm2(sub2(a, b))
}
fn perp(v: [f64; 2]) -> [f64; 2] {
    [-v[1], v[0]]
}

/// Stroke a path into oriented region boundaries, including clockwise holes.
/// The contours must be filled together under the NonZero rule.
pub fn outline_stroke(path: &BezierPath, opts: &StrokeOptions) -> Result<Vec<BezierPath>> {
    outline_stroke_tol(path, opts, crate::path::FLATTEN_TOLERANCE)
}

/// Stroke with a maximum chord error in document units for curves and round joins.
pub fn outline_stroke_tol(
    path: &BezierPath,
    opts: &StrokeOptions,
    tolerance: f64,
) -> Result<Vec<BezierPath>> {
    stroke_rings(path, opts, tolerance)?
        .iter()
        .map(|ring| BezierPath::from_polyline(ring, true))
        .collect()
}

/// Shared internal region pipeline. Keep normalized rings as rings until a
/// caller actually needs editable paths; renderers consume them directly.
pub(crate) fn stroke_rings(
    path: &BezierPath,
    opts: &StrokeOptions,
    tolerance: f64,
) -> Result<crate::rings::Rings> {
    let mut pts = stroke_points(path, opts, tolerance)?;
    let origin = center_stroke_points(&mut pts)?;
    let mut rings = outline_points(pts, path.closed, opts, tolerance, true)?;
    if origin != [0., 0.] {
        for point in rings.iter_mut().flatten() {
            *point = add2(*point, origin);
        }
    }
    Ok(rings)
}

/// Direct, nonoverlapping ribbon mesh when the offset boundary is certified.
/// Complex strokes retain the general union and fill tessellation behavior.
pub fn tessellate_stroke(
    path: &BezierPath,
    opts: &StrokeOptions,
    tolerance: f64,
) -> Result<crate::tessellation::FillMesh> {
    let mut pts = stroke_points(path, opts, tolerance)?;
    let origin = center_stroke_points(&mut pts)?;
    let mut mesh = tessellate_stroke_points(pts, path.closed, opts, tolerance)?;
    if origin != [0., 0.] {
        for point in &mut mesh.positions {
            *point = add2(*point, origin);
        }
    }
    Ok(mesh)
}

// Build offsets and intersections in local coordinates when document position
// dwarfs path extent. Otherwise arc endpoints lose enough bits to open joins.
fn center_stroke_points(pts: &mut [[f64; 2]]) -> Result<[f64; 2]> {
    let (scale, extent) = crate::rings::coordinate_metrics(pts.iter())?;
    let origin = if scale > extent * 1024. {
        pts[0]
    } else {
        [0., 0.]
    };
    if origin != [0., 0.] {
        for point in pts {
            *point = sub2(*point, origin);
        }
    }
    Ok(origin)
}

fn tessellate_stroke_points(
    pts: Vec<[f64; 2]>,
    closed: bool,
    opts: &StrokeOptions,
    tolerance: f64,
) -> Result<crate::tessellation::FillMesh> {
    if opts.dash.as_ref().is_none_or(Vec::is_empty)
        && let Some(mesh) = simple::mesh(&pts, closed, opts, tolerance)?
    {
        return Ok(mesh);
    }
    // A rejected ribbon must not repeat the bounded intersection certificate.
    let rings = outline_points(pts, closed, opts, tolerance, false)?;
    crate::tessellation::tessellate_normalized_rings(&rings)
}

fn stroke_points(path: &BezierPath, opts: &StrokeOptions, tolerance: f64) -> Result<Vec<[f64; 2]>> {
    check(
        opts.width > 0. && opts.width.is_finite(),
        "Invalid stroke width",
    )?;
    check(
        opts.miter_limit >= 1. && opts.miter_limit.is_finite(),
        "Invalid miter limit",
    )?;
    check(
        tolerance > 0. && tolerance.is_finite(),
        "Invalid stroke tolerance",
    )?;
    check(opts.dash_offset.is_finite(), "Invalid dash offset")?;
    let mut pts = path.flatten_tol(tolerance)?;
    pts.dedup_by(|a, b| dist(*a, *b) <= 1e-12);
    if path.closed && pts.len() > 1 && dist(pts[0], *pts.last().unwrap()) <= 1e-12 {
        pts.pop();
    }
    check(pts.len() >= 2, "Path too short to outline")?;
    Ok(pts)
}

fn outline_points(
    pts: Vec<[f64; 2]>,
    closed: bool,
    opts: &StrokeOptions,
    tolerance: f64,
    try_simple: bool,
) -> Result<crate::rings::Rings> {
    if closed
        && opts.dash.as_ref().is_none_or(Vec::is_empty)
        && let Some(rings) = convex_closed_stroke(&pts, opts, tolerance)?
    {
        return Ok(rings);
    }
    if try_simple
        && opts.dash.as_ref().is_none_or(Vec::is_empty)
        && let Some(rings) = simple::outline(&pts, closed, opts, tolerance)?
    {
        return Ok(rings);
    }
    let polylines = match &opts.dash {
        Some(dash) if !dash.is_empty() => dash_polylines(&pts, closed, dash, opts.dash_offset)?,
        _ => vec![(pts, closed)],
    };
    let mut pieces = Vec::new();
    for (poly, closed) in polylines {
        stroke_pieces(&poly, closed, opts, tolerance, &mut pieces)?;
    }
    let contours = crate::rings::nonzero(&pieces)?;
    check(!contours.is_empty(), "Stroke produced no outline")?;
    Ok(contours)
}

/// A simple convex ring has two offset boundaries, with no arrangement to
/// solve. Certify convexity and a noncollapsed inset before using this route;
/// concave rings, stars with winding >1, and consumed insets use the union path.
fn convex_closed_stroke(
    input: &[[f64; 2]],
    opts: &StrokeOptions,
    tolerance: f64,
) -> Result<Option<crate::rings::Rings>> {
    if input.len() < 3 {
        return Ok(None);
    }
    let mut pts = input.to_vec();
    if crate::rings::area(&pts) < 0. {
        pts.reverse();
    }
    let n = pts.len();
    let half = opts.width * 0.5;
    let mut directions = Vec::with_capacity(n);
    for i in 0..n {
        let d = sub2(pts[(i + 1) % n], pts[i]);
        if norm2(d) <= 1e-12 {
            return Ok(None);
        }
        directions.push(unit2(d));
    }
    let mut turn = 0.;
    for i in 0..n {
        let a = directions[(i + n - 1) % n];
        let b = directions[i];
        let cross = crate::rings::cross2(a, b);
        let dot = a[0] * b[0] + a[1] * b[1];
        if cross < -1e-12 || (cross.abs() <= 1e-12 && dot < 0.) {
            return Ok(None);
        }
        turn += cross.atan2(dot);
    }
    if (turn - std::f64::consts::TAU).abs() > 1e-7 {
        return Ok(None);
    }
    let mut outer = Vec::with_capacity(n * 2);
    let mut inner = Vec::with_capacity(n);
    for i in 0..n {
        let p = pts[i];
        let d0 = directions[(i + n - 1) % n];
        let d1 = directions[i];
        let normal0 = scale2(perp(d0), half);
        let normal1 = scale2(perp(d1), half);
        let l0 = add2(p, normal0);
        let l1 = add2(p, normal1);
        let r0 = sub2(p, normal0);
        let r1 = sub2(p, normal1);
        let cross = crate::rings::cross2(d0, d1);
        let dot = d0[0] * d1[0] + d0[1] * d1[1];
        if cross.abs() < 1e-12 {
            outer.push(r1);
            inner.push(l1);
            continue;
        }
        let Some(inside) = line_intersect(l0, add2(l0, d0), l1, add2(l1, d1)) else {
            return Ok(None);
        };
        inner.push(inside);
        match opts.join {
            LineJoin::Miter => {
                if let Some(m) = line_intersect(r0, add2(r0, d0), r1, add2(r1, d1))
                    && dist(m, p) <= half * opts.miter_limit
                {
                    outer.push(m);
                    continue;
                }
                outer.extend([r0, r1]);
            }
            LineJoin::Bevel => outer.extend([r0, r1]),
            LineJoin::Round => {
                let angle = (r0[1] - p[1]).atan2(r0[0] - p[0]);
                outer.extend(
                    arc_sector(p, half, angle, cross.atan2(dot), tolerance)?
                        .into_iter()
                        .skip(1),
                );
            }
        }
    }
    // Every inset edge must still advance along its corresponding support.
    // Together with the certified convex turn this proves all half-planes are
    // retained. Redundant/consumed edges are left to the general arrangement.
    for i in 0..n {
        let edge = sub2(inner[(i + 1) % n], inner[i]);
        let d = directions[i];
        if edge[0] * d[0] + edge[1] * d[1] <= 1e-12 {
            return Ok(None);
        }
    }
    if !outer.iter().chain(&inner).flatten().all(|v| v.is_finite()) {
        return Err(crate::error("Non-finite stroke boundary"));
    }
    inner.reverse();
    Ok(Some(vec![outer, inner]))
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
        ArrowMarker::Chevron => {
            let perp = [-dir[1], dir[0]];
            let tip = endpoint;
            let base = [endpoint[0] - dir[0] * s, endpoint[1] - dir[1] * s];
            let b1 = [base[0] + perp[0] * s * 0.55, base[1] + perp[1] * s * 0.55];
            let b2 = [base[0] - perp[0] * s * 0.55, base[1] - perp[1] * s * 0.55];
            // Thin filled chevron (V pointing along dir).
            let inset = [tip[0] - dir[0] * s * 0.35, tip[1] - dir[1] * s * 0.35];
            BezierPath::from_polyline(&[tip, b1, inset, b2], true)?
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
        let d = sub2(pts[0], pts[1]);
        if let Some(p) = arrow_marker_path(start, pts[0], d, stroke_width)? {
            out.push(p);
        }
    }
    if end != ArrowMarker::None {
        let n = pts.len();
        let d = sub2(pts[n - 1], pts[n - 2]);
        if let Some(p) = arrow_marker_path(end, pts[n - 1], d, stroke_width)? {
            out.push(p);
        }
    }
    Ok(out)
}

/// Dash a flattened polyline into open spans `(points, closed=false)`.
pub fn dash_spans(
    pts: &[[f64; 2]],
    closed: bool,
    pattern: &[f64],
    offset: f64,
) -> Result<Vec<Vec<[f64; 2]>>> {
    Ok(dash_polylines(pts, closed, pattern, offset)?
        .into_iter()
        .map(|(p, _)| p)
        .collect())
}

/// Dash a Bézier path (flattened) into open polylines suitable for UI stroking.
pub fn path_dash_spans(
    path: &BezierPath,
    pattern: &[f64],
    offset: f64,
    tolerance: f64,
) -> Result<Vec<Vec<[f64; 2]>>> {
    let pts = path.flatten_tol(tolerance)?;
    check(pts.len() >= 2, "Path too short to dash")?;
    dash_spans(&pts, path.closed, pattern, offset)
}

fn dash_polylines(
    pts: &[[f64; 2]],
    closed: bool,
    pattern: &[f64],
    offset: f64,
) -> Result<Vec<(Vec<[f64; 2]>, bool)>> {
    check(
        pts.len() >= 2 && pts.iter().flatten().all(|x| x.is_finite()),
        "Invalid dash path",
    )?;
    check(offset.is_finite(), "Invalid dash offset")?;
    check(
        !pattern.is_empty()
            && pattern.len() <= 4096
            && pattern.iter().all(|d| d.is_finite() && *d >= 0.),
        "Invalid dash pattern",
    )?;
    let mut pattern = pattern.to_vec();
    // SVG repeats the entire odd-length pattern, preserving on/off alternation.
    if pattern.len() % 2 == 1 {
        pattern.extend_from_within(..);
    }
    let period: f64 = pattern.iter().sum();
    check(
        period.is_finite() && period > 1e-12,
        "Degenerate dash period",
    )?;
    let mut phase = 0usize;
    let mut consumed = offset.rem_euclid(period);
    while consumed >= pattern[phase] {
        consumed -= pattern[phase];
        phase = (phase + 1) % pattern.len();
    }
    let mut remain = pattern[phase] - consumed;
    let mut current = Vec::new();
    let mut out: Vec<(Vec<[f64; 2]>, bool)> = Vec::new();
    let edge_count = if closed { pts.len() } else { pts.len() - 1 };
    let mut steps = 0usize;
    for i in 0..edge_count {
        let a = pts[i];
        let b = pts[(i + 1) % pts.len()];
        let length = dist(a, b);
        if length <= 1e-12 {
            continue;
        }
        let mut walked = 0.;
        while walked < length {
            steps += 1;
            check(steps <= 100_000, "Dash sampling budget exceeded")?;
            let step = remain.min(length - walked);
            let point = |d: f64| {
                [
                    a[0] + (b[0] - a[0]) * d / length,
                    a[1] + (b[1] - a[1]) * d / length,
                ]
            };
            if phase.is_multiple_of(2) {
                if current.is_empty() {
                    current.push(point(walked));
                }
                let end = point(walked + step);
                if dist(*current.last().unwrap(), end) > 1e-12 {
                    current.push(end);
                }
            }
            walked += step;
            remain -= step;
            if remain <= period * 1e-14 {
                if phase.is_multiple_of(2) && current.len() >= 2 {
                    out.push((std::mem::take(&mut current), false));
                    check(out.len() <= 4096, "Dash produced too many spans")?;
                }
                loop {
                    phase = (phase + 1) % pattern.len();
                    remain = pattern[phase];
                    if remain > 0. {
                        break;
                    }
                }
            }
        }
    }
    if current.len() >= 2 {
        out.push((current, false));
    }
    // A dash crossing the closed seam is a single joined stroke, without caps there.
    if closed && !out.is_empty() {
        let first_at_seam = dist(out[0].0[0], pts[0]) <= 1e-12;
        let last_at_seam = dist(*out.last().unwrap().0.last().unwrap(), pts[0]) <= 1e-12;
        if first_at_seam && last_at_seam {
            if out.len() == 1 {
                out[0].0.pop();
                out[0].1 = true;
            } else {
                let mut last = out.pop().unwrap().0;
                last.extend(out[0].0.iter().skip(1));
                out[0].0 = last;
            }
        }
    }
    Ok(out)
}

/// Union edge strips and outside join sectors instead of stitching two offset
/// chains across their seams. This also handles concavity and collapsed inner loops.
fn stroke_pieces(
    pts: &[[f64; 2]],
    closed: bool,
    opts: &StrokeOptions,
    tolerance: f64,
    pieces: &mut crate::rings::Rings,
) -> Result<()> {
    let n = pts.len();
    check(n >= 2, "Polyline too short")?;
    let half = opts.width * 0.5;
    let edge_count = if closed { n } else { n - 1 };
    for i in 0..edge_count {
        let mut a = pts[i];
        let mut b = pts[(i + 1) % n];
        let d = unit2(sub2(b, a));
        if dist(a, b) <= 1e-12 {
            continue;
        }
        if !closed && opts.cap == LineCap::Square {
            if i == 0 {
                a = sub2(a, scale2(d, half));
            }
            if i == edge_count - 1 {
                b = add2(b, scale2(d, half));
            }
        }
        let normal = scale2(perp(d), half);
        push_piece(
            pieces,
            vec![
                add2(a, normal),
                sub2(a, normal),
                sub2(b, normal),
                add2(b, normal),
            ],
        );
    }
    let joints = if closed { 0..n } else { 1..n - 1 };
    for i in joints {
        let p = pts[i];
        let d0 = unit2(sub2(p, pts[(i + n - 1) % n]));
        let d1 = unit2(sub2(pts[(i + 1) % n], p));
        let cross = crate::rings::cross2(d0, d1);
        let dot = d0[0] * d1[0] + d0[1] * d1[1];
        if cross.abs() < 1e-12 {
            if dot < 0. && opts.join == LineJoin::Round {
                push_piece(
                    pieces,
                    arc_sector(p, half, 0., std::f64::consts::TAU, tolerance)?,
                );
            }
            continue;
        }
        let side = if cross > 0. { -half } else { half };
        let q0 = add2(p, scale2(perp(d0), side));
        let q1 = add2(p, scale2(perp(d1), side));
        let mut wedge = vec![p, q0];
        match opts.join {
            LineJoin::Round => {
                let angle = (q0[1] - p[1]).atan2(q0[0] - p[0]);
                wedge = arc_sector(p, half, angle, cross.atan2(dot), tolerance)?;
            }
            LineJoin::Miter => {
                if let Some(m) = line_intersect(q0, add2(q0, d0), q1, add2(q1, d1))
                    && dist(p, m) <= half * opts.miter_limit
                {
                    wedge.push(m);
                }
                wedge.push(q1);
            }
            LineJoin::Bevel => wedge.push(q1),
        }
        push_piece(pieces, wedge);
    }
    if !closed && opts.cap == LineCap::Round {
        let d0 = sub2(pts[1], pts[0]);
        let d1 = sub2(pts[n - 1], pts[n - 2]);
        // Outside semicircles; their diameter is covered by the edge strip.
        for (p, angle) in [
            (pts[0], d0[1].atan2(d0[0]) + std::f64::consts::FRAC_PI_2),
            (pts[n - 1], d1[1].atan2(d1[0]) - std::f64::consts::FRAC_PI_2),
        ] {
            push_piece(
                pieces,
                arc_sector(p, half, angle, std::f64::consts::PI, tolerance)?,
            );
        }
    }
    check(
        pieces.iter().map(Vec::len).sum::<usize>() <= 100_000,
        "Stroke vertex budget exceeded",
    )?;
    Ok(())
}

fn push_piece(pieces: &mut crate::rings::Rings, mut ring: Vec<[f64; 2]>) {
    let area = crate::rings::area(&ring);
    if area.abs() <= 1e-24 {
        return;
    }
    if area < 0. {
        ring.reverse();
    }
    pieces.push(ring);
}

fn arc_sector(
    center: [f64; 2],
    radius: f64,
    start: f64,
    sweep: f64,
    tolerance: f64,
) -> Result<Vec<[f64; 2]>> {
    let step = (2. * (1. - (tolerance / radius).min(1.)).acos()).min(std::f64::consts::FRAC_PI_4);
    check(
        step > 0.,
        "Stroke tolerance below floating point resolution",
    )?;
    let count = (sweep.abs() / step).ceil().max(1.) as usize;
    check(count <= 4096, "Round stroke vertex budget exceeded")?;
    let mut ring = Vec::with_capacity(count + 2);
    ring.push(center);
    for i in 0..=count {
        let angle = start + sweep * i as f64 / count as f64;
        ring.push([
            center[0] + radius * angle.cos(),
            center[1] + radius * angle.sin(),
        ]);
    }
    Ok(ring)
}

fn line_intersect(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> Option<[f64; 2]> {
    let r = sub2(b, a);
    let s = sub2(d, c);
    let den = r[0] * s[1] - r[1] * s[0];
    if den.abs() < 1e-15 {
        return None;
    }
    let qp = sub2(c, a);
    let t = (qp[0] * s[1] - qp[1] * s[0]) / den;
    Some([a[0] + t * r[0], a[1] + t * r[1]])
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

    #[test]
    fn chevron_marker_is_closed() {
        let line = BezierPath::from_polyline(&[[0., 0.], [10., 0.]], false).unwrap();
        let marks = path_arrow_markers(&line, ArrowMarker::None, ArrowMarker::Chevron, 1.).unwrap();
        assert_eq!(marks.len(), 1);
        assert!(marks[0].closed);
        assert!(marks[0].segments.len() >= 3);
    }

    #[test]
    fn dash_spans_alternate_on_line() {
        let line = BezierPath::from_polyline(&[[0., 0.], [20., 0.]], false).unwrap();
        let spans = path_dash_spans(&line, &[4., 4.], 0., 0.25).unwrap();
        assert!(spans.len() >= 2, "spans={}", spans.len());
        assert!(spans.iter().all(|s| s.len() >= 2));
    }
    fn region_area(paths: &[BezierPath]) -> f64 {
        paths.iter().map(|p| area(&p.to_ring(0.001).unwrap())).sum()
    }

    #[test]
    fn closed_stroke_keeps_hole_and_has_no_seam_gap() {
        let rect = BezierPath::from_rect([0., 0.], [10., 10.]).unwrap();
        let out = outline_stroke(
            &rect,
            &StrokeOptions {
                width: 2.,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(out.len(), 2);
        assert!((region_area(&out) - 80.).abs() < 1e-8);
        let rings = out.iter().map(|p| p.to_ring(0.01).unwrap()).collect();
        assert!(!crate::rings::inside([5., 5.], &rings));
        for p in [[0., 5.], [5., 0.], [10., 5.], [5., 10.], [-0.5, -0.5]] {
            assert!(crate::rings::inside(p, &rings), "seam/join gap at {p:?}");
        }
    }

    #[test]
    fn caps_have_correct_area_and_round_chord_accuracy() {
        let line = BezierPath::from_polyline(&[[0., 0.], [10., 0.]], false).unwrap();
        for (cap, expected) in [
            (LineCap::Butt, 20.),
            (LineCap::Square, 24.),
            (LineCap::Round, 20. + std::f64::consts::PI),
        ] {
            let out = outline_stroke_tol(
                &line,
                &StrokeOptions {
                    width: 2.,
                    cap,
                    ..Default::default()
                },
                0.0001,
            )
            .unwrap();
            assert!(
                (region_area(&out) - expected).abs() < 0.001,
                "{cap:?}: {}",
                region_area(&out)
            );
        }
    }

    #[test]
    fn joins_distinguish_bevel_round_and_miter() {
        let line = BezierPath::from_polyline(&[[0., 0.], [10., 0.], [10., 10.]], false).unwrap();
        for (join, expected) in [
            (LineJoin::Bevel, 39.5),
            (LineJoin::Round, 39. + std::f64::consts::FRAC_PI_4),
            (LineJoin::Miter, 40.),
        ] {
            let out = outline_stroke_tol(
                &line,
                &StrokeOptions {
                    width: 2.,
                    join,
                    ..Default::default()
                },
                0.0001,
            )
            .unwrap();
            assert!(
                (region_area(&out) - expected).abs() < 0.001,
                "{join:?}: {}",
                region_area(&out)
            );
        }
    }

    #[test]
    fn acute_miter_is_clipped_to_bevel_at_limit() {
        let line = BezierPath::from_polyline(&[[0., 0.], [10., 0.], [0., 1.]], false).unwrap();
        let outline = |limit| {
            outline_stroke(
                &line,
                &StrokeOptions {
                    width: 2.,
                    miter_limit: limit,
                    ..Default::default()
                },
            )
            .unwrap()
        };
        let short = outline(2.);
        let long = outline(100.);
        assert!(region_area(&long) > region_area(&short) + 10.);
        let rightmost = |paths: &[BezierPath]| {
            paths
                .iter()
                .flat_map(|p| p.anchors())
                .map(|p| p[0])
                .fold(f64::NEG_INFINITY, f64::max)
        };
        assert!(rightmost(&short) < 11.);
        assert!(rightmost(&long) > 25.);
    }

    #[test]
    fn dash_keeps_intermediate_corners_and_repeats_odd_pattern() {
        let spans = dash_spans(&[[0., 0.], [3., 0.], [3., 4.]], false, &[5., 1.], 0.).unwrap();
        assert_eq!(spans[0], vec![[0., 0.], [3., 0.], [3., 2.]]);
        let spans = dash_spans(&[[0., 0.], [12., 0.]], false, &[1., 2., 3.], 0.).unwrap();
        assert_eq!(
            spans,
            vec![
                vec![[0., 0.], [1., 0.]],
                vec![[3., 0.], [6., 0.]],
                vec![[7., 0.], [9., 0.]]
            ]
        );
    }

    #[test]
    fn dash_closed_seam_joins_without_extra_caps() {
        let spans = dash_polylines(
            &[[0., 0.], [2., 0.], [2., 2.], [0., 2.]],
            true,
            &[3., 2.],
            0.,
        )
        .unwrap();
        assert_eq!(spans.len(), 1);
        assert!(spans[0].0.windows(3).any(|p| p[1] == [0., 0.]));
        let solid = dash_polylines(
            &[[0., 0.], [2., 0.], [2., 2.], [0., 2.]],
            true,
            &[100., 2.],
            0.,
        )
        .unwrap();
        assert!(solid[0].1);
        assert_eq!(solid[0].0.len(), 4);
    }

    #[test]
    fn invalid_dash_options_fail_before_sampling() {
        let points = [[0., 0.], [1., 0.]];
        for pattern in [&[-1., 2.][..], &[f64::NAN, 2.], &[0., 0.]] {
            assert!(dash_spans(&points, false, pattern, 0.).is_err());
        }
        assert!(dash_spans(&points, false, &[1., 1.], f64::INFINITY).is_err());
    }
    #[test]
    fn convex_route_matches_general_union_for_all_join_styles() {
        let polygon = [[0., 0.], [7., 0.], [10., 4.], [6., 9.], [1., 8.]];
        let path = BezierPath::from_polyline(&polygon, true).unwrap();
        for join in [LineJoin::Miter, LineJoin::Bevel, LineJoin::Round] {
            for width in [0.2, 2., 12.] {
                let options = StrokeOptions {
                    width,
                    join,
                    ..Default::default()
                };
                let actual = outline_stroke_tol(&path, &options, 0.002).unwrap();
                let mut pieces = vec![];
                stroke_pieces(&polygon, true, &options, 0.002, &mut pieces).unwrap();
                let expected = crate::rings::nonzero(&pieces).unwrap();
                let actual: crate::rings::Rings =
                    actual.iter().map(|p| p.to_ring(0.002).unwrap()).collect();
                let area =
                    |r: &crate::rings::Rings| r.iter().map(|r| crate::rings::area(r)).sum::<f64>();
                assert!(
                    (area(&actual) - area(&expected)).abs() < 1e-7,
                    "{join:?}, width={width}"
                );
                for x in -4..16 {
                    for y in -4..16 {
                        let p = [x as f64 + 0.237, y as f64 + 0.419];
                        assert_eq!(
                            crate::rings::inside(p, &actual),
                            crate::rings::inside(p, &expected)
                        );
                    }
                }
            }
        }
        let star: Vec<_> = (0..5)
            .map(|i| {
                let a = i as f64 * std::f64::consts::TAU * 2. / 5.;
                [a.cos() * 10., a.sin() * 10.]
            })
            .collect();
        assert!(
            convex_closed_stroke(&star, &StrokeOptions::default(), 0.01)
                .unwrap()
                .is_none()
        );
    }
}
