//! Scissors / knife path physics on cubic Bézier paths (f64 mm).
//!
//! Algorithms adapted from the Curvex vector editor (MIT OR Apache-2.0):
//! hit-test on Line/Cubic segments, de Casteljau split, knife chord
//! intersections, open multi-cut, and closed two-hit fillable slice.
use crate::path::{BezierPath, PathSegment, segment_from};
use crate::{Result, check};
use math_core::{add2, norm2, scale2, sub2};

/// Where a scissors/knife hit lands in segment parameter space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CutHit {
    pub segment_index: usize,
    /// Parametric position within the segment, clamped away from endpoints when cutting.
    pub t: f64,
    pub distance: f64,
    pub point: [f64; 2],
}

const T_EPS: f64 = 1e-4;
const DUP_T: f64 = 1e-3;
const CUBIC_STEPS: usize = 48;

fn lerp(a: [f64; 2], b: [f64; 2], t: f64) -> [f64; 2] {
    add2(a, scale2(sub2(b, a), t))
}

fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    norm2(sub2(a, b))
}

fn dist_point_to_line(a: [f64; 2], b: [f64; 2], p: [f64; 2]) -> (f64, f64) {
    let ab = sub2(b, a);
    let ap = sub2(p, a);
    let len2 = ab[0] * ab[0] + ab[1] * ab[1];
    if len2 < 1e-18 {
        return (norm2(ap), 0.0);
    }
    let t = ((ap[0] * ab[0] + ap[1] * ab[1]) / len2).clamp(0.0, 1.0);
    let q = lerp(a, b, t);
    (dist(p, q), t)
}

fn eval_cubic(p0: [f64; 2], c1: [f64; 2], c2: [f64; 2], p3: [f64; 2], t: f64) -> [f64; 2] {
    let mt = 1.0 - t;
    let b0 = mt * mt * mt;
    let b1 = 3.0 * mt * mt * t;
    let b2 = 3.0 * mt * t * t;
    let b3 = t * t * t;
    [
        b0 * p0[0] + b1 * c1[0] + b2 * c2[0] + b3 * p3[0],
        b0 * p0[1] + b1 * c1[1] + b2 * c2[1] + b3 * p3[1],
    ]
}

fn dist_point_to_cubic(
    p0: [f64; 2],
    c1: [f64; 2],
    c2: [f64; 2],
    p3: [f64; 2],
    target: [f64; 2],
) -> (f64, f64) {
    let mut best_d = f64::INFINITY;
    let mut best_t = 0.0;
    let mut prev = p0;
    for i in 0..=CUBIC_STEPS {
        let t = i as f64 / CUBIC_STEPS as f64;
        let cur = eval_cubic(p0, c1, c2, p3, t);
        let (d, seg_t) = dist_point_to_line(prev, cur, target);
        if d < best_d {
            best_d = d;
            let t_prev = (i.saturating_sub(1)) as f64 / CUBIC_STEPS as f64;
            best_t = t_prev + (t - t_prev) * seg_t;
        }
        prev = cur;
    }
    (best_d, best_t.clamp(0.0, 1.0))
}

/// Split segment `idx` at `t`. Returns (before, after, cut_point).
pub fn split_segment_at(
    start: [f64; 2],
    segments: &[PathSegment],
    idx: usize,
    t: f64,
) -> Result<(Option<PathSegment>, Option<PathSegment>, [f64; 2])> {
    check(idx < segments.len(), "Segment index out of range")?;
    let t = t.clamp(T_EPS, 1.0 - T_EPS);
    let from = segment_from(start, segments, idx);
    Ok(match segments[idx] {
        PathSegment::Line { to } => {
            let cut = lerp(from, to, t);
            (
                Some(PathSegment::Line { to: cut }),
                Some(PathSegment::Line { to }),
                cut,
            )
        }
        PathSegment::Cubic { c1, c2, to } => {
            let q0 = lerp(from, c1, t);
            let q1 = lerp(c1, c2, t);
            let q2 = lerp(c2, to, t);
            let r0 = lerp(q0, q1, t);
            let r1 = lerp(q1, q2, t);
            let cut = lerp(r0, r1, t);
            (
                Some(PathSegment::Cubic {
                    c1: q0,
                    c2: r0,
                    to: cut,
                }),
                Some(PathSegment::Cubic { c1: r1, c2: q2, to }),
                cut,
            )
        }
    })
}

/// Closest hit on real Line/Cubic segments within `max_dist` (mm).
pub fn hit_test(path: &BezierPath, click: [f64; 2], max_dist: f64) -> Result<Option<CutHit>> {
    check(max_dist > 0.0, "Invalid hit distance")?;
    check(!path.segments.is_empty(), "Empty path")?;
    let mut best: Option<CutHit> = None;
    let mut current = path.start;
    for (i, seg) in path.segments.iter().enumerate() {
        let (d, t, end) = match *seg {
            PathSegment::Line { to } => {
                let (d, t) = dist_point_to_line(current, to, click);
                (d, t, to)
            }
            PathSegment::Cubic { c1, c2, to } => {
                let (d, t) = dist_point_to_cubic(current, c1, c2, to, click);
                (d, t, to)
            }
        };
        if d <= max_dist && best.as_ref().is_none_or(|b| d < b.distance) {
            let point = match *seg {
                PathSegment::Line { to } => lerp(current, to, t),
                PathSegment::Cubic { c1, c2, to } => eval_cubic(current, c1, c2, to, t),
            };
            best = Some(CutHit {
                segment_index: i,
                t,
                distance: d,
                point,
            });
        }
        current = end;
    }
    Ok(best)
}

/// Scissors: open a closed loop at the click, or split an open path into two.
pub fn scissors_cut(path: &BezierPath, click: [f64; 2], max_dist: f64) -> Result<Vec<BezierPath>> {
    let Some(hit) = hit_test(path, click, max_dist)? else {
        return Err(crate::error("Scissors: no path under cursor"));
    };
    cut_at(path, hit)
}

/// Cut at an already-resolved hit (preserves cubics via de Casteljau).
pub fn cut_at(path: &BezierPath, hit: CutHit) -> Result<Vec<BezierPath>> {
    check(!path.segments.is_empty(), "Empty path")?;
    let t = hit.t.clamp(T_EPS, 1.0 - T_EPS);
    let idx = hit.segment_index;
    let (before, after, cut_point) = split_segment_at(path.start, &path.segments, idx, t)?;

    let mut head: Vec<PathSegment> = path.segments[..idx].to_vec();
    if let Some(b) = before {
        head.push(b);
    }
    let mut tail: Vec<PathSegment> = Vec::new();
    if let Some(a) = after {
        tail.push(a);
    }
    tail.extend_from_slice(&path.segments[idx + 1..]);

    if path.closed {
        let mut combined = Vec::new();
        combined.extend(tail);
        combined.extend(head);
        check(!combined.is_empty(), "Scissors: cut produced nothing")?;
        return Ok(vec![BezierPath::open(cut_point, combined)?]);
    }

    let mut out = Vec::new();
    if !head.is_empty() {
        out.push(BezierPath::open(path.start, head)?);
    }
    if !tail.is_empty() {
        out.push(BezierPath::open(cut_point, tail)?);
    }
    check(!out.is_empty(), "Scissors: cut produced nothing")?;
    Ok(out)
}

fn segment_segment_t(a0: [f64; 2], a1: [f64; 2], b0: [f64; 2], b1: [f64; 2]) -> Option<f64> {
    let ax = a1[0] - a0[0];
    let ay = a1[1] - a0[1];
    let bx = b1[0] - b0[0];
    let by = b1[1] - b0[1];
    let denom = ax * by - ay * bx;
    if denom.abs() < 1e-12 {
        return None;
    }
    let dx = b0[0] - a0[0];
    let dy = b0[1] - a0[1];
    let t = (dx * by - dy * bx) / denom;
    let u = (dx * ay - dy * ax) / denom;
    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
        Some(t)
    } else {
        None
    }
}

fn cubic_knife_ts(
    p0: [f64; 2],
    c1: [f64; 2],
    c2: [f64; 2],
    p3: [f64; 2],
    k0: [f64; 2],
    k1: [f64; 2],
) -> Vec<f64> {
    let mut hits = Vec::new();
    let mut prev = p0;
    let mut prev_t = 0.0;
    for i in 1..=CUBIC_STEPS {
        let t = i as f64 / CUBIC_STEPS as f64;
        let cur = eval_cubic(p0, c1, c2, p3, t);
        if let Some(seg_t) = segment_segment_t(prev, cur, k0, k1) {
            hits.push(prev_t + (t - prev_t) * seg_t);
        }
        prev = cur;
        prev_t = t;
    }
    hits.dedup_by(|a, b| (*a - *b).abs() < DUP_T);
    hits
}

/// All knife-chord intersections sorted by path order.
pub fn knife_hits(path: &BezierPath, k0: [f64; 2], k1: [f64; 2]) -> Result<Vec<CutHit>> {
    check(
        k0.iter().chain(k1.iter()).all(|x| x.is_finite()),
        "Invalid knife segment",
    )?;
    check(dist(k0, k1) > 1e-12, "Degenerate knife segment")?;
    let mut hits = Vec::new();
    let mut current = path.start;
    for (i, seg) in path.segments.iter().enumerate() {
        match *seg {
            PathSegment::Line { to } => {
                if let Some(t) = segment_segment_t(current, to, k0, k1) {
                    hits.push(CutHit {
                        segment_index: i,
                        t,
                        distance: 0.0,
                        point: lerp(current, to, t),
                    });
                }
                current = to;
            }
            PathSegment::Cubic { c1, c2, to } => {
                for t in cubic_knife_ts(current, c1, c2, to, k0, k1) {
                    hits.push(CutHit {
                        segment_index: i,
                        t,
                        distance: 0.0,
                        point: eval_cubic(current, c1, c2, to, t),
                    });
                }
                current = to;
            }
        }
    }
    hits.sort_by(|a, b| {
        a.segment_index
            .cmp(&b.segment_index)
            .then(a.t.total_cmp(&b.t))
    });
    hits.retain(|h| h.t > T_EPS && h.t < 1.0 - T_EPS);
    let mut dedup: Vec<CutHit> = Vec::with_capacity(hits.len());
    for h in hits {
        if let Some(last) = dedup.last() {
            if last.segment_index == h.segment_index && (last.t - h.t).abs() < DUP_T {
                continue;
            }
        }
        dedup.push(h);
    }
    Ok(dedup)
}

fn extract_subpath_closed(
    start: [f64; 2],
    segments: &[PathSegment],
    a: CutHit,
    b: CutHit,
) -> Result<Option<([f64; 2], Vec<PathSegment>)>> {
    let ta = a.t.clamp(T_EPS, 1.0 - T_EPS);
    let tb = b.t.clamp(T_EPS, 1.0 - T_EPS);
    let (_, after_a, pt_a) = split_segment_at(start, segments, a.segment_index, ta)?;
    let (before_b, _, _) = split_segment_at(start, segments, b.segment_index, tb)?;
    let mut out: Vec<PathSegment> = Vec::new();
    if a.segment_index == b.segment_index && ta < tb {
        let Some(after) = after_a else {
            return Ok(None);
        };
        let local_t = ((tb - ta) / (1.0 - ta)).clamp(T_EPS, 1.0 - T_EPS);
        let local = [after];
        let (head, _, _) = split_segment_at(pt_a, &local, 0, local_t)?;
        if let Some(h) = head {
            out.push(h);
        }
    } else {
        if let Some(seg) = after_a {
            out.push(seg);
        }
        let n = segments.len();
        let mut i = (a.segment_index + 1) % n;
        let mut guard = 0;
        while i != b.segment_index {
            out.push(segments[i]);
            i = (i + 1) % n;
            guard += 1;
            if guard > n + 2 {
                break;
            }
        }
        if let Some(seg) = before_b {
            out.push(seg);
        }
    }
    if out.is_empty() {
        return Ok(None);
    }
    Ok(Some((pt_a, out)))
}

fn extract_subpath_open(
    start: [f64; 2],
    segments: &[PathSegment],
    start_hit: Option<CutHit>,
    end_hit: Option<CutHit>,
) -> Result<Option<([f64; 2], Vec<PathSegment>)>> {
    let (chunk_start, first_idx, first_override) = match start_hit {
        Some(h) => {
            let (_, after, pt) =
                split_segment_at(start, segments, h.segment_index, h.t.clamp(T_EPS, 1.0 - T_EPS))?;
            (pt, h.segment_index + 1, after)
        }
        None => (start, 0, None),
    };
    let (last_excl, last_override) = match end_hit {
        Some(h) => {
            let (before, _, _) =
                split_segment_at(start, segments, h.segment_index, h.t.clamp(T_EPS, 1.0 - T_EPS))?;
            (h.segment_index, before)
        }
        None => (segments.len(), None),
    };

    if let (Some(a), Some(b)) = (start_hit, end_hit) {
        if a.segment_index == b.segment_index && a.t < b.t {
            let Some(after) = first_override else {
                return Ok(None);
            };
            let local_t = ((b.t - a.t) / (1.0 - a.t)).clamp(T_EPS, 1.0 - T_EPS);
            let local = [after];
            let (head, _, _) = split_segment_at(chunk_start, &local, 0, local_t)?;
            let mut out = Vec::new();
            if let Some(h) = head {
                out.push(h);
            }
            return Ok(Some((chunk_start, out)));
        }
    }

    let mut out = Vec::new();
    if let Some(seg) = first_override {
        out.push(seg);
    }
    if first_idx < last_excl {
        out.extend_from_slice(&segments[first_idx..last_excl]);
    }
    if let Some(seg) = last_override {
        out.push(seg);
    }
    if out.is_empty() {
        return Ok(None);
    }
    Ok(Some((chunk_start, out)))
}

/// Knife on an open path (or closed with &lt;2 hits → scissors open): open pieces.
pub fn knife_cut(path: &BezierPath, k0: [f64; 2], k1: [f64; 2]) -> Result<Vec<BezierPath>> {
    let hits = knife_hits(path, k0, k1)?;
    check(!hits.is_empty(), "Knife: no intersection")?;
    if path.closed && hits.len() < 2 {
        return cut_at(path, hits[0]);
    }
    cut_multi(path, &hits)
}

fn cut_multi(path: &BezierPath, hits: &[CutHit]) -> Result<Vec<BezierPath>> {
    check(!hits.is_empty(), "Knife: no hits")?;
    let mut pieces: Vec<([f64; 2], Vec<PathSegment>)> = Vec::new();
    if path.closed {
        for i in 0..hits.len() {
            let a = hits[i];
            let b = hits[(i + 1) % hits.len()];
            if let Some(chunk) = extract_subpath_closed(path.start, &path.segments, a, b)? {
                pieces.push(chunk);
            }
        }
    } else {
        if let Some(head) = extract_subpath_open(path.start, &path.segments, None, Some(hits[0]))? {
            pieces.push(head);
        }
        for w in hits.windows(2) {
            if let Some(mid) =
                extract_subpath_open(path.start, &path.segments, Some(w[0]), Some(w[1]))?
            {
                pieces.push(mid);
            }
        }
        if let Some(tail) = extract_subpath_open(
            path.start,
            &path.segments,
            Some(hits[hits.len() - 1]),
            None,
        )? {
            pieces.push(tail);
        }
    }
    let mut out = Vec::new();
    for (s, segs) in pieces {
        if !segs.is_empty() {
            out.push(BezierPath::open(s, segs)?);
        }
    }
    check(!out.is_empty(), "Knife: cut produced nothing")?;
    Ok(out)
}

/// Closed fillable knife: exactly two clean crossings → two closed pieces closed by the chord.
pub fn knife_split(path: &BezierPath, k0: [f64; 2], k1: [f64; 2]) -> Result<Vec<BezierPath>> {
    check(path.closed, "Knife split requires a closed path")?;
    let hits = knife_hits(path, k0, k1)?;
    check(
        hits.len() == 2,
        "Knife split needs exactly two outline crossings",
    )?;
    let (pt_a, mut arc_ab) =
        extract_subpath_closed(path.start, &path.segments, hits[0], hits[1])?
            .ok_or_else(|| crate::error("Knife split: empty arc A→B"))?;
    let (pt_b, mut arc_ba) =
        extract_subpath_closed(path.start, &path.segments, hits[1], hits[0])?
            .ok_or_else(|| crate::error("Knife split: empty arc B→A"))?;
    arc_ab.push(PathSegment::Line { to: pt_a });
    arc_ba.push(PathSegment::Line { to: pt_b });
    check(
        arc_ab.len() >= 2 && arc_ba.len() >= 2,
        "Knife split: degenerate half",
    )?;
    Ok(vec![
        BezierPath::closed(pt_a, arc_ab)?,
        BezierPath::closed(pt_b, arc_ba)?,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scissors_open_polyline() {
        let path = BezierPath::from_polyline(&[[0., 0.], [10., 0.]], false).unwrap();
        let parts = scissors_cut(&path, [5., 0.05], 0.5).unwrap();
        assert_eq!(parts.len(), 2);
        assert!(!parts[0].closed && !parts[1].closed);
        assert!((parts[0].segments.last().unwrap().end()[0] - 5.).abs() < 1e-6);
        assert!((parts[1].start[0] - 5.).abs() < 1e-6);
    }

    #[test]
    fn scissors_opens_closed_loop() {
        let path = BezierPath::from_rect([0., 0.], [4., 2.]).unwrap();
        let parts = scissors_cut(&path, [2., 0.], 0.25).unwrap();
        assert_eq!(parts.len(), 1);
        assert!(!parts[0].closed);
        assert!((parts[0].start[0] - 2.).abs() < 1e-3);
    }

    #[test]
    fn scissors_preserves_cubic() {
        let path = BezierPath::open(
            [0., 0.],
            vec![PathSegment::Cubic {
                c1: [1., 2.],
                c2: [3., 2.],
                to: [4., 0.],
            }],
        )
        .unwrap();
        let parts = scissors_cut(&path, [2., 1.5], 1.0).unwrap();
        assert_eq!(parts.len(), 2);
        assert!(matches!(parts[0].segments[0], PathSegment::Cubic { .. }));
        assert!(matches!(parts[1].segments[0], PathSegment::Cubic { .. }));
    }

    #[test]
    fn knife_open_line() {
        let path = BezierPath::from_polyline(&[[0., 0.], [10., 0.], [10., 10.]], false).unwrap();
        let parts = knife_cut(&path, [5., -1.], [5., 1.]).unwrap();
        assert!(parts.len() >= 2);
    }

    #[test]
    fn knife_split_rect() {
        let path = BezierPath::from_rect([0., 0.], [10., 6.]).unwrap();
        let parts = knife_split(&path, [5., -1.], [5., 7.]).unwrap();
        assert_eq!(parts.len(), 2);
        assert!(parts[0].closed && parts[1].closed);
        // Each half should include the vertical chord near x=5.
        for p in &parts {
            let xs: Vec<f64> = p
                .segments
                .iter()
                .map(|s| s.end()[0])
                .chain(std::iter::once(p.start[0]))
                .collect();
            assert!(xs.iter().any(|x| (*x - 5.).abs() < 1e-6));
        }
    }

    #[test]
    fn knife_split_rejects_open() {
        let path = BezierPath::from_polyline(&[[0., 0.], [10., 0.]], false).unwrap();
        assert!(knife_split(&path, [5., -1.], [5., 1.]).is_err());
    }

    #[test]
    fn knife_hits_cubic() {
        let path = BezierPath::open(
            [0., 0.],
            vec![PathSegment::Cubic {
                c1: [0., 4.],
                c2: [4., 4.],
                to: [4., 0.],
            }],
        )
        .unwrap();
        let hits = knife_hits(&path, [-1., 2.], [5., 2.]).unwrap();
        assert!(!hits.is_empty());
        assert!(hits[0].t > T_EPS && hits[0].t < 1.0 - T_EPS);
    }
}
