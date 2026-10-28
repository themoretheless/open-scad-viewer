use super::*;

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

pub(super) use crate::dist;
pub(super) fn perp(v: [f64; 2]) -> [f64; 2] {
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
pub(super) fn convex_closed_stroke(
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

pub(super) fn dash_polylines(
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
