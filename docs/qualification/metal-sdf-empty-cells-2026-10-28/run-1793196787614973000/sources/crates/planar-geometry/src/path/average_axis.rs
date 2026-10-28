use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AverageAxis {
    Horizontal,
    Vertical,
    Both,
}

pub type AnchorHandles = (Option<[f64; 2]>, Option<[f64; 2]>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandleSide {
    In,
    Out,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandleLink {
    Free,
    Mirrored,
    Symmetric,
}

pub fn average_anchor_targets(positions: &[[f64; 2]], axis: AverageAxis) -> Vec<[f64; 2]> {
    if positions.is_empty() {
        return Vec::new();
    }
    let n = positions.len() as f64;
    let mean_x = positions.iter().map(|p| p[0]).sum::<f64>() / n;
    let mean_y = positions.iter().map(|p| p[1]).sum::<f64>() / n;
    positions
        .iter()
        .map(|p| match axis {
            AverageAxis::Horizontal => [mean_x, p[1]],
            AverageAxis::Vertical => [p[0], mean_y],
            AverageAxis::Both => [mean_x, mean_y],
        })
        .collect()
}

pub(super) fn catmull_rom_segments(pts: &[[f64; 2]], closed: bool) -> Vec<PathSegment> {
    let n = pts.len();
    let seg_count = if closed { n } else { n.saturating_sub(1) };
    const T: f64 = 1.0 / 6.0;
    let mut segs = Vec::with_capacity(seg_count);
    for i in 0..seg_count {
        let p1 = pts[i];
        let p2 = pts[(i + 1) % n];
        let p0 = if closed {
            pts[(i + n - 1) % n]
        } else if i == 0 {
            p1
        } else {
            pts[i - 1]
        };
        let p3 = if closed {
            pts[(i + 2) % n]
        } else if i + 2 < n {
            pts[i + 2]
        } else {
            p2
        };
        segs.push(PathSegment::Cubic {
            c1: [p1[0] + (p2[0] - p0[0]) * T, p1[1] + (p2[1] - p0[1]) * T],
            c2: [p2[0] - (p3[0] - p1[0]) * T, p2[1] - (p3[1] - p1[1]) * T],
            to: p2,
        });
    }
    segs
}

pub(super) fn offset_open_polyline(path: &BezierPath, distance: f64) -> Result<BezierPath> {
    let pts = path.flatten()?;
    check(pts.len() >= 2, "Open path too short to offset")?;
    let mut out = Vec::with_capacity(pts.len());
    for i in 0..pts.len() {
        let (a, b) = if i + 1 < pts.len() {
            (pts[i], pts[i + 1])
        } else {
            (pts[i - 1], pts[i])
        };
        let d = sub2(b, a);
        let len = d[0].hypot(d[1]).max(1e-12);
        let n = [-d[1] / len * distance, d[0] / len * distance];
        if i > 0 && i + 1 < pts.len() {
            let prev = sub2(pts[i], pts[i - 1]);
            let plen = prev[0].hypot(prev[1]).max(1e-12);
            let n0 = [-prev[1] / plen * distance, prev[0] / plen * distance];
            out.push([
                pts[i][0] + (n0[0] + n[0]) * 0.5,
                pts[i][1] + (n0[1] + n[1]) * 0.5,
            ]);
        } else {
            out.push([pts[i][0] + n[0], pts[i][1] + n[1]]);
        }
    }
    BezierPath::from_polyline(&out, false)
}

/// Join two open paths end-to-end, optionally reversing either side.
pub fn join_paths(
    head: &BezierPath,
    reverse_head: bool,
    tail: &BezierPath,
    reverse_tail: bool,
    weld_eps: f64,
) -> Result<BezierPath> {
    join_paths_with_bridge(head, reverse_head, tail, reverse_tail, &[], weld_eps)
}

/// Forward tangent at an endpoint, with a chord fallback for a collapsed handle.
pub fn endpoint_tangent(path: &BezierPath, at_end: bool) -> Option<[f64; 2]> {
    let (primary, fallback) = if at_end {
        let segment = path.segments.last()?;
        let previous = if path.segments.len() > 1 {
            path.segments[path.segments.len() - 2].end()
        } else {
            path.start
        };
        let from = match *segment {
            PathSegment::Line { .. } => previous,
            PathSegment::Cubic { c2, .. } => c2,
        };
        (sub2(segment.end(), from), sub2(segment.end(), previous))
    } else {
        let segment = path.segments.first()?;
        let to = match *segment {
            PathSegment::Line { to } => to,
            PathSegment::Cubic { c1, .. } => c1,
        };
        (sub2(to, path.start), sub2(segment.end(), path.start))
    };
    let nonzero = |d: [f64; 2]| d[0].abs() > 1e-6 || d[1].abs() > 1e-6;
    if nonzero(primary) {
        Some(primary)
    } else if nonzero(fallback) {
        Some(fallback)
    } else {
        None
    }
}

fn tangent_bridge(head: &BezierPath, tail: &BezierPath) -> Vec<PathSegment> {
    let (Some(da), Some(dt)) = (endpoint_tangent(head, true), endpoint_tangent(tail, false)) else {
        return Vec::new();
    };
    let db = [-dt[0], -dt[1]];
    let a = head.segments.last().map_or(head.start, PathSegment::end);
    let b = tail.start;
    let denominator = cross2(da, db);
    if denominator.abs() < 1e-9 {
        return Vec::new();
    }
    let ab = sub2(b, a);
    let ta = cross2(ab, db) / denominator;
    let tb = cross2(ab, da) / denominator;
    if ta <= 0.0 || tb <= 0.0 {
        return Vec::new();
    }
    let intersection = [a[0] + ta * da[0], a[1] + ta * da[1]];
    let reach = (dist(a, b) * 3.0).max(1.0);
    if dist(a, intersection) > reach || dist(b, intersection) > reach {
        return Vec::new();
    }
    let straight_cubic = |from, to| PathSegment::Cubic {
        c1: lerp(from, to, 1.0 / 3.0),
        c2: lerp(from, to, 2.0 / 3.0),
        to,
    };
    vec![
        straight_cubic(a, intersection),
        straight_cubic(intersection, b),
    ]
}

/// Curvex endpoint Join: continue both tangents through a nearby forward-ray
/// intersection. Parallel, backward or distant intersections use a straight join.
pub fn join_paths_at_tangents(
    head: &BezierPath,
    reverse_head: bool,
    tail: &BezierPath,
    reverse_tail: bool,
    weld_eps: f64,
) -> Result<BezierPath> {
    let a = if reverse_head {
        head.reverse()
    } else {
        head.clone()
    };
    let b = if reverse_tail {
        tail.reverse()
    } else {
        tail.clone()
    };
    join_paths_with_bridge(&a, false, &b, false, &tangent_bridge(&a, &b), weld_eps)
}

/// Close an open path using the same bounded tangent continuation as endpoint Join.
pub fn close_path_at_tangents(path: &BezierPath, weld_eps: f64) -> Result<BezierPath> {
    check(
        !path.closed && !path.segments.is_empty(),
        "Close requires a nonempty open path",
    )?;
    let empty_tail = BezierPath::open(path.start, Vec::new())?;
    let joined = join_paths_with_bridge(
        path,
        false,
        &empty_tail,
        false,
        &tangent_bridge(path, path),
        weld_eps,
    )?;
    BezierPath::closed(joined.start, joined.segments)
}

/// Join open paths using an optional line/cubic connector. A connector that
/// misses the tail is completed with a line; a welded join ignores it.
pub fn join_paths_with_bridge(
    head: &BezierPath,
    reverse_head: bool,
    tail: &BezierPath,
    reverse_tail: bool,
    bridge: &[PathSegment],
    weld_eps: f64,
) -> Result<BezierPath> {
    check(
        weld_eps >= 0.0 && weld_eps.is_finite(),
        "Invalid join weld tolerance",
    )?;
    validate_segments(bridge)?;
    check(
        !head.closed && !tail.closed,
        "join_paths requires open paths",
    )?;
    let a = if reverse_head {
        head.reverse()
    } else {
        head.clone()
    };
    let b = if reverse_tail {
        tail.reverse()
    } else {
        tail.clone()
    };
    let head_end = a.segments.last().map_or(a.start, |s| s.end());
    let mut segments = a.segments;
    let within_weld = |a: [f64; 2], b: [f64; 2]| {
        (a[0] - b[0]).abs() <= weld_eps && (a[1] - b[1]).abs() <= weld_eps
    };
    let coincide = within_weld(head_end, b.start);
    if !coincide {
        segments.extend_from_slice(bridge);
        if !within_weld(segments.last().map_or(head_end, PathSegment::end), b.start) {
            segments.push(PathSegment::Line { to: b.start });
        }
    }
    segments.extend(b.segments);
    BezierPath::open(a.start, segments)
}

pub(super) fn validate_segments(segments: &[PathSegment]) -> Result<()> {
    check(
        segments.len() <= MAX_SEGMENTS,
        "Path exceeds segment budget",
    )?;
    for seg in segments {
        match *seg {
            PathSegment::Line { to } => {
                check(
                    to[0].is_finite() && to[1].is_finite(),
                    "Non-finite path point",
                )?;
            }
            PathSegment::Cubic { c1, c2, to } => {
                for p in [c1, c2, to] {
                    check(
                        p[0].is_finite() && p[1].is_finite(),
                        "Non-finite path point",
                    )?;
                }
            }
        }
    }
    Ok(())
}

pub(super) fn flatten_cubic(
    p0: [f64; 2],
    c1: [f64; 2],
    c2: [f64; 2],
    p3: [f64; 2],
    tolerance: f64,
    out: &mut Vec<[f64; 2]>,
) -> Result<()> {
    let mut stack = vec![([p0, c1, c2, p3], 0usize)];
    while let Some(([a, b, c, d], depth)) = stack.pop() {
        let direction = sub2(d, a);
        let length = direction[0].hypot(direction[1]);
        let distance_to_chord = |p: [f64; 2]| {
            if length == 0. {
                return dist(p, a);
            }
            let v = sub2(p, a);
            let t = ((v[0] / length) * (direction[0] / length)
                + (v[1] / length) * (direction[1] / length))
                .clamp(0., 1.);
            dist(p, lerp(a, d, t))
        };
        // The Bézier convex hull lies in this capsule around the chord. Use
        // distance to the finite segment, not its supporting line: collinear
        // overshoots and loops must be subdivided too.
        if distance_to_chord(b).max(distance_to_chord(c)) <= tolerance {
            out.push(d);
            check(
                out.len() <= MAX_FLATTEN_POINTS,
                "Flatten exceeded point budget",
            )?;
            continue;
        }
        check(
            depth < 32,
            "Flatten cannot meet tolerance within subdivision budget",
        )?;
        let ab = lerp(a, b, 0.5);
        let bc = lerp(b, c, 0.5);
        let cd = lerp(c, d, 0.5);
        let abc = lerp(ab, bc, 0.5);
        let bcd = lerp(bc, cd, 0.5);
        let middle = lerp(abc, bcd, 0.5);
        stack.push(([middle, bcd, cd, d], depth + 1));
        stack.push(([a, ab, abc, middle], depth + 1));
    }
    Ok(())
}

pub(crate) fn segment_from(start: [f64; 2], segments: &[PathSegment], idx: usize) -> [f64; 2] {
    let mut from = start;
    for seg in &segments[..idx] {
        from = seg.end();
    }
    from
}

pub(super) fn neighbour_anchors(path: &BezierPath, node: usize) -> Result<([f64; 2], [f64; 2], [f64; 2])> {
    let n = path.anchor_count();
    check(node < n, "Anchor index out of range")?;
    let anchors = path.anchors();
    let prev = if node == 0 {
        check(path.closed, "Open-path endpoints have no smooth neighbours")?;
        anchors[n - 1]
    } else {
        anchors[node - 1]
    };
    let next = if node == n - 1 {
        check(path.closed, "Open-path endpoints have no smooth neighbours")?;
        anchors[0]
    } else {
        anchors[node + 1]
    };
    Ok((prev, anchors[node], next))
}

pub(super) fn incoming_segment(path: &BezierPath, node: usize) -> Option<usize> {
    let n = path.anchor_count();
    if node == 0 {
        if path.closed {
            Some(path.segments.len() - 1)
        } else {
            None
        }
    } else if node < n {
        Some(node - 1)
    } else {
        None
    }
}

pub(super) fn outgoing_segment(path: &BezierPath, node: usize) -> Option<usize> {
    let n = path.anchor_count();
    if node == n - 1 {
        if path.closed {
            Some(path.segments.len() - 1)
        } else {
            None
        }
    } else if node < n {
        Some(node)
    } else {
        None
    }
}

pub(super) fn ensure_cubic(segments: &mut [PathSegment], idx: usize, from: [f64; 2]) {
    if let PathSegment::Line { to } = segments[idx] {
        segments[idx] = PathSegment::Cubic {
            c1: lerp(from, to, 1.0 / 3.0),
            c2: lerp(from, to, 2.0 / 3.0),
            to,
        };
    }
}

// Convex hull + rotating calipers avoids quadratic diameter search on dense
// curves. Ties retain the first original-index pair, as Curvex's pair scan does.
pub(super) fn farthest_pair(points: &[[f64; 2]]) -> (usize, usize) {
    let mut order: Vec<usize> = (0..points.len()).collect();
    order.sort_by(|&a, &b| points[a].partial_cmp(&points[b]).unwrap().then(a.cmp(&b)));
    order.dedup_by(|a, b| points[*a] == points[*b]);
    if order.len() < 2 {
        return (0, 1);
    }
    let turn = |a: usize, b: usize, c: usize| {
        cross2(sub2(points[b], points[a]), sub2(points[c], points[a]))
    };
    let mut hull = Vec::new();
    for &i in &order {
        while hull.len() >= 2 && turn(hull[hull.len() - 2], hull[hull.len() - 1], i) <= 0.0 {
            hull.pop();
        }
        hull.push(i);
    }
    let lower = hull.len();
    for &i in order.iter().rev().skip(1) {
        while hull.len() > lower && turn(hull[hull.len() - 2], hull[hull.len() - 1], i) <= 0.0 {
            hull.pop();
        }
        hull.push(i);
    }
    hull.pop();
    let n = hull.len();
    let mut best = (0, 1);
    let mut best_distance = -1.0;
    let mut consider = |a: usize, b: usize| {
        let pair = (a.min(b), a.max(b));
        let d = sub2(points[a], points[b]);
        let d = d[0] * d[0] + d[1] * d[1];
        if d > best_distance || (d == best_distance && pair < best) {
            best_distance = d;
            best = pair;
        }
    };
    let mut j = 1;
    for i in 0..n {
        let next = (i + 1) % n;
        while turn(hull[i], hull[next], hull[(j + 1) % n]).abs()
            > turn(hull[i], hull[next], hull[j]).abs()
        {
            j = (j + 1) % n;
        }
        consider(hull[i], hull[j]);
        consider(hull[next], hull[j]);
        let j_next = (j + 1) % n;
        if turn(hull[i], hull[next], hull[j_next]).abs() == turn(hull[i], hull[next], hull[j]).abs()
        {
            consider(hull[i], hull[j_next]);
            consider(hull[next], hull[j_next]);
        }
    }
    best
}

pub(super) fn rdp_keep(points: &[[f64; 2]], eps: f64) -> Vec<bool> {
    let n = points.len();
    let mut keep = vec![false; n];
    if n == 0 {
        return keep;
    }
    keep[0] = true;
    keep[n - 1] = true;
    if n < 3 {
        return keep;
    }
    let mut stack = vec![(0usize, n - 1)];
    while let Some((start, end)) = stack.pop() {
        if end <= start + 1 {
            continue;
        }
        let a = points[start];
        let b = points[end];
        let mut far_idx = start;
        let mut far_dist = -1.0;
        for (offset, p) in points[start + 1..end].iter().enumerate() {
            let d = perp_line_distance(*p, a, b);
            if d > far_dist {
                far_dist = d;
                far_idx = start + 1 + offset;
            }
        }
        if far_dist > eps {
            keep[far_idx] = true;
            stack.push((start, far_idx));
            stack.push((far_idx, end));
        }
    }
    keep
}
