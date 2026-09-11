//! Corner styles and rounded primitive → Bézier conversion.
use crate::path::{BezierPath, PathSegment};
use crate::{check, Result};

const KAPPA: f64 = 0.552_285;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CornerStyle {
    #[default]
    Round,
    Chamfer,
    Inverted,
    Notch,
}

fn lerp(a: [f64; 2], b: [f64; 2], t: f64) -> [f64; 2] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

fn resolve_corner(radii: &[f64], styles: &[CornerStyle], i: usize) -> (CornerStyle, f64) {
    let raw = radii.get(i).copied().unwrap_or(0.0);
    let r = if raw.is_finite() { raw } else { 0.0 };
    if styles.is_empty() {
        if r < 0.0 {
            (CornerStyle::Chamfer, -r)
        } else {
            (CornerStyle::Round, r)
        }
    } else {
        (styles.get(i).copied().unwrap_or_default(), r.abs())
    }
}

fn push_corner(
    segments: &mut Vec<PathSegment>,
    a: [f64; 2],
    b: [f64; 2],
    v: [f64; 2],
    style: CornerStyle,
) {
    match style {
        CornerStyle::Round => segments.push(PathSegment::Cubic {
            c1: lerp(a, v, KAPPA),
            c2: lerp(b, v, KAPPA),
            to: b,
        }),
        CornerStyle::Chamfer => segments.push(PathSegment::Line { to: b }),
        CornerStyle::Inverted => {
            let inset = [a[0] + b[0] - v[0], a[1] + b[1] - v[1]];
            segments.push(PathSegment::Cubic {
                c1: lerp(a, inset, KAPPA),
                c2: lerp(b, inset, KAPPA),
                to: b,
            });
        }
        CornerStyle::Notch => {
            segments.push(PathSegment::Line { to: v });
            segments.push(PathSegment::Line { to: b });
        }
    }
}

fn push_quad_corner(segments: &mut Vec<PathSegment>, a: [f64; 2], b: [f64; 2], control: [f64; 2]) {
    segments.push(PathSegment::Cubic {
        c1: [
            a[0] + (2.0 / 3.0) * (control[0] - a[0]),
            a[1] + (2.0 / 3.0) * (control[1] - a[1]),
        ],
        c2: [
            b[0] + (2.0 / 3.0) * (control[0] - b[0]),
            b[1] + (2.0 / 3.0) * (control[1] - b[1]),
        ],
        to: b,
    });
}

/// Rounded / chamfered / inverted / notched rectangle. Radii order: NW,NE,SE,SW.
pub fn rounded_rect(
    min: [f64; 2],
    max: [f64; 2],
    radii: &[f64],
    styles: &[CornerStyle],
) -> Result<BezierPath> {
    let (x0, y0) = (min[0].min(max[0]), min[1].min(max[1]));
    let (x1, y1) = (min[0].max(max[0]), min[1].max(max[1]));
    let w = x1 - x0;
    let h = y1 - y0;
    check(w > 0. && h > 0., "Degenerate rounded rect")?;
    let (s_nw, mut r_nw) = resolve_corner(radii, styles, 0);
    let (s_ne, mut r_ne) = resolve_corner(radii, styles, 1);
    let (s_se, mut r_se) = resolve_corner(radii, styles, 2);
    let (s_sw, mut r_sw) = resolve_corner(radii, styles, 3);
    let clamp_pair = |a: &mut f64, b: &mut f64, edge: f64| {
        let sum = *a + *b;
        if sum > edge && sum > 0. {
            let s = edge / sum;
            *a *= s;
            *b *= s;
        }
    };
    clamp_pair(&mut r_nw, &mut r_ne, w);
    clamp_pair(&mut r_sw, &mut r_se, w);
    clamp_pair(&mut r_nw, &mut r_sw, h);
    clamp_pair(&mut r_ne, &mut r_se, h);

    let start = [x0 + r_nw, y0];
    let mut segments = Vec::new();
    let mut current = start;
    let push_line = |segments: &mut Vec<PathSegment>, current: &mut [f64; 2], to: [f64; 2]| {
        if dist(*current, to) > 1e-9 {
            segments.push(PathSegment::Line { to });
            *current = to;
        }
    };

    push_line(&mut segments, &mut current, [x0 + w - r_ne, y0]);
    if r_ne > 0. {
        let a = current;
        let b = [x0 + w, y0 + r_ne];
        push_corner(&mut segments, a, b, [x0 + w, y0], s_ne);
        current = b;
    }
    push_line(&mut segments, &mut current, [x0 + w, y0 + h - r_se]);
    if r_se > 0. {
        let a = current;
        let b = [x0 + w - r_se, y0 + h];
        push_corner(&mut segments, a, b, [x0 + w, y0 + h], s_se);
        current = b;
    }
    push_line(&mut segments, &mut current, [x0 + r_sw, y0 + h]);
    if r_sw > 0. {
        let a = current;
        let b = [x0, y0 + h - r_sw];
        push_corner(&mut segments, a, b, [x0, y0 + h], s_sw);
        current = b;
    }
    push_line(&mut segments, &mut current, [x0, y0 + r_nw]);
    if r_nw > 0. {
        let a = current;
        let b = [x0 + r_nw, y0];
        push_corner(&mut segments, a, b, [x0, y0], s_nw);
    }
    BezierPath::closed(start, segments)
}

/// Closed polygon with per-vertex corner styles.
pub fn rounded_polygon(
    points: &[[f64; 2]],
    radii: &[f64],
    styles: &[CornerStyle],
) -> Result<BezierPath> {
    let n = points.len();
    check(n >= 3, "Rounded polygon needs ≥3 points")?;
    let edge_len: Vec<f64> = (0..n)
        .map(|i| dist(points[i], points[(i + 1) % n]))
        .collect();
    let mut p_starts = vec![[0.0; 2]; n];
    let mut p_ends = vec![[0.0; 2]; n];
    let mut modes = vec![None::<CornerStyle>; n];
    for i in 0..n {
        let (style, mag) = resolve_corner(radii, styles, i);
        let prev = points[(i + n - 1) % n];
        let curr = points[i];
        let next = points[(i + 1) % n];
        if mag == 0. {
            p_starts[i] = curr;
            p_ends[i] = curr;
            continue;
        }
        let d_prev = edge_len[(i + n - 1) % n];
        let d_next = edge_len[i];
        let r = mag.min((d_prev * 0.5).min(d_next * 0.5));
        let t_prev = if d_prev > 0. { r / d_prev } else { 0. };
        let t_next = if d_next > 0. { r / d_next } else { 0. };
        p_starts[i] = lerp(curr, prev, t_prev);
        p_ends[i] = lerp(curr, next, t_next);
        if r > 1e-4 {
            modes[i] = Some(style);
        } else {
            p_starts[i] = curr;
            p_ends[i] = curr;
        }
    }

    let start = p_starts[0];
    let mut segments = Vec::new();
    for i in 0..n {
        let v = points[i];
        let a = p_starts[i];
        let b = p_ends[i];
        if i > 0 {
            let prev_end = p_ends[i - 1];
            if dist(prev_end, a) > 1e-9 {
                segments.push(PathSegment::Line { to: a });
            }
        }
        match modes[i] {
            None => {
                let at = if i == 0 {
                    start
                } else {
                    segments.last().map(|s| s.end()).unwrap_or(start)
                };
                if dist(at, v) > 1e-9 {
                    segments.push(PathSegment::Line { to: v });
                }
            }
            Some(CornerStyle::Round) => push_quad_corner(&mut segments, a, b, v),
            Some(CornerStyle::Inverted) => {
                let inset = [a[0] + b[0] - v[0], a[1] + b[1] - v[1]];
                push_quad_corner(&mut segments, a, b, inset);
            }
            Some(CornerStyle::Chamfer) => segments.push(PathSegment::Line { to: b }),
            Some(CornerStyle::Notch) => {
                segments.push(PathSegment::Line { to: v });
                segments.push(PathSegment::Line { to: b });
            }
        }
    }
    let at = segments.last().map(|s| s.end()).unwrap_or(start);
    if dist(at, start) > 1e-9 {
        segments.push(PathSegment::Line { to: start });
    }
    BezierPath::closed(start, segments)
}

/// Fillet interior corners of a path to radius `r` (anchor polyline).
pub fn round_corners(path: &BezierPath, radius: f64) -> Result<BezierPath> {
    check(
        radius >= 0. && radius.is_finite(),
        "Invalid round-corners radius",
    )?;
    let pts = path.flatten()?;
    let mut ring = pts;
    if path.closed && ring.len() >= 2 && dist(ring[0], *ring.last().unwrap()) <= 1e-9 {
        ring.pop();
    }
    if path.closed {
        check(ring.len() >= 3, "Closed path needs ≥3 anchors to round")?;
        let radii = vec![radius; ring.len()];
        let styles = vec![CornerStyle::Round; ring.len()];
        rounded_polygon(&ring, &radii, &styles)
    } else {
        check(ring.len() >= 3, "Open path needs ≥3 anchors to round")?;
        round_open_polyline(&ring, radius)
    }
}

fn round_open_polyline(points: &[[f64; 2]], radius: f64) -> Result<BezierPath> {
    let n = points.len();
    let start = points[0];
    if n == 2 {
        return BezierPath::from_polyline(points, false);
    }
    let edge_len: Vec<f64> = (0..n - 1).map(|i| dist(points[i], points[i + 1])).collect();
    let mut segments = Vec::new();
    for i in 1..n - 1 {
        let prev = points[i - 1];
        let curr = points[i];
        let next = points[i + 1];
        let d_prev = edge_len[i - 1];
        let d_next = edge_len[i];
        let r = radius.min((d_prev * 0.5).min(d_next * 0.5));
        if r <= 1e-4 {
            segments.push(PathSegment::Line { to: curr });
            continue;
        }
        let a = lerp(curr, prev, r / d_prev);
        let b = lerp(curr, next, r / d_next);
        segments.push(PathSegment::Line { to: a });
        push_quad_corner(&mut segments, a, b, curr);
    }
    segments.push(PathSegment::Line { to: points[n - 1] });
    BezierPath::open(start, segments)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounded_rect_styles() {
        let p = rounded_rect(
            [0., 0.],
            [10., 6.],
            &[2., 2., 2., 2.],
            &[
                CornerStyle::Round,
                CornerStyle::Chamfer,
                CornerStyle::Notch,
                CornerStyle::Inverted,
            ],
        )
        .unwrap();
        assert!(p.closed);
        assert!(p.segments.len() >= 8);
    }

    #[test]
    fn round_corners_square() {
        let sq = BezierPath::from_rect([0., 0.], [4., 4.]).unwrap();
        let r = round_corners(&sq, 0.5).unwrap();
        assert!(r.closed);
        assert!(r
            .segments
            .iter()
            .any(|s| matches!(s, PathSegment::Cubic { .. })));
    }
}
