//! First-class cubic Bézier paths for planar modeling.
//!
//! Algorithms adapted from the Curvex vector editor (MIT OR Apache-2.0),
//! reimplemented in binary64 to match the polygon-core CAD contract.
//! Paths flatten to polylines for planar boolean / offset / extrude.
use crate::{Result, check};
use math_core::{cross2, norm2, sub2};

/// One path segment: straight line or cubic Bézier. Endpoints are absolute mm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathSegment {
    Line {
        to: [f64; 2],
    },
    Cubic {
        c1: [f64; 2],
        c2: [f64; 2],
        to: [f64; 2],
    },
}

impl PathSegment {
    pub const fn end(&self) -> [f64; 2] {
        match *self {
            PathSegment::Line { to } | PathSegment::Cubic { to, .. } => to,
        }
    }
}

/// Editable cubic Bézier path. Anchor 0 is `start`; segment `i` runs from
/// anchor `i` to anchor `i+1`. Closed paths store an explicit closing segment.
#[derive(Debug, Clone, PartialEq)]
pub struct BezierPath {
    pub start: [f64; 2],
    pub segments: Vec<PathSegment>,
    pub closed: bool,
}

/// Default flatten tolerance in mm (chord deviation).
pub const FLATTEN_TOLERANCE: f64 = 0.25;
const ELLIPSE_KAPPA: f64 = 0.552_285;
const MAX_SEGMENTS: usize = 65_536;
const MAX_FLATTEN_POINTS: usize = 65_537;

fn lerp(a: [f64; 2], b: [f64; 2], t: f64) -> [f64; 2] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    norm2(sub2(a, b))
}

fn perp_line_distance(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let ab = sub2(b, a);
    let ap = sub2(p, a);
    let len = norm2(ab);
    if len < 1e-15 {
        return norm2(ap);
    }
    (cross2(ab, ap) / len).abs()
}

impl BezierPath {
    pub fn open(start: [f64; 2], segments: Vec<PathSegment>) -> Result<Self> {
        validate_segments(&segments)?;
        check(start.iter().all(|x| x.is_finite()), "Non-finite path start")?;
        Ok(Self {
            start,
            segments,
            closed: false,
        })
    }

    pub fn closed(start: [f64; 2], segments: Vec<PathSegment>) -> Result<Self> {
        validate_segments(&segments)?;
        check(start.iter().all(|x| x.is_finite()), "Non-finite path start")?;
        check(
            !segments.is_empty(),
            "Closed path needs at least one segment",
        )?;
        Ok(Self {
            start,
            segments,
            closed: true,
        })
    }

    pub fn from_polyline(points: &[[f64; 2]], closed: bool) -> Result<Self> {
        check(points.len() >= 2, "Polyline needs at least two points")?;
        check(
            points.len() <= MAX_SEGMENTS + 1,
            "Polyline exceeds segment budget",
        )?;
        let start = points[0];
        let last = if closed {
            points.len()
        } else {
            points.len() - 1
        };
        let mut segments = Vec::with_capacity(last);
        for i in 1..points.len() {
            segments.push(PathSegment::Line { to: points[i] });
        }
        if closed {
            segments.push(PathSegment::Line { to: start });
        }
        if closed {
            Self::closed(start, segments)
        } else {
            Self::open(start, segments)
        }
    }

    pub fn from_rect(min: [f64; 2], max: [f64; 2]) -> Result<Self> {
        let (x0, y0, x1, y1) = (
            min[0].min(max[0]),
            min[1].min(max[1]),
            min[0].max(max[0]),
            min[1].max(max[1]),
        );
        Self::from_polyline(&[[x0, y0], [x1, y0], [x1, y1], [x0, y1]], true)
    }

    pub fn from_ellipse(center: [f64; 2], rx: f64, ry: f64) -> Result<Self> {
        check(
            rx >= 0. && ry >= 0. && rx.is_finite() && ry.is_finite(),
            "Invalid ellipse radii",
        )?;
        let (cx, cy) = (center[0], center[1]);
        let kx = rx * ELLIPSE_KAPPA;
        let ky = ry * ELLIPSE_KAPPA;
        let right = [cx + rx, cy];
        let bottom = [cx, cy + ry];
        let left = [cx - rx, cy];
        let top = [cx, cy - ry];
        Self::closed(
            right,
            vec![
                PathSegment::Cubic {
                    c1: [cx + rx, cy - ky],
                    c2: [cx + kx, cy - ry],
                    to: top,
                },
                PathSegment::Cubic {
                    c1: [cx - kx, cy - ry],
                    c2: [cx - rx, cy - ky],
                    to: left,
                },
                PathSegment::Cubic {
                    c1: [cx - rx, cy + ky],
                    c2: [cx - kx, cy + ry],
                    to: bottom,
                },
                PathSegment::Cubic {
                    c1: [cx + kx, cy + ry],
                    c2: [cx + rx, cy + ky],
                    to: right,
                },
            ],
        )
    }

    pub fn from_circle(center: [f64; 2], radius: f64) -> Result<Self> {
        Self::from_ellipse(center, radius, radius)
    }

    pub fn from_polygon(points: &[[f64; 2]], closed: bool) -> Result<Self> {
        Self::from_polyline(points, closed)
    }

    pub fn anchor_count(&self) -> usize {
        if self.closed {
            self.segments.len()
        } else {
            self.segments.len() + 1
        }
    }

    pub fn anchors(&self) -> Vec<[f64; 2]> {
        let n = self.anchor_count();
        let mut out = Vec::with_capacity(n);
        out.push(self.start);
        let limit = if self.closed {
            self.segments.len().saturating_sub(1)
        } else {
            self.segments.len()
        };
        for seg in &self.segments[..limit] {
            out.push(seg.end());
        }
        out
    }

    /// Adaptive cubic flatten (Wang formula) with default tolerance.
    pub fn flatten(&self) -> Result<Vec<[f64; 2]>> {
        self.flatten_tol(FLATTEN_TOLERANCE)
    }

    pub fn flatten_tol(&self, tolerance: f64) -> Result<Vec<[f64; 2]>> {
        check(
            tolerance.is_finite() && tolerance > 0.,
            "Invalid flatten tolerance",
        )?;
        check(
            self.start.iter().all(|x| x.is_finite()),
            "Non-finite path start",
        )?;
        validate_segments(&self.segments)?;
        let tol = tolerance;
        let mut out = Vec::with_capacity(self.segments.len() * 8 + 1);
        out.push(self.start);
        let mut current = self.start;
        for seg in &self.segments {
            match *seg {
                PathSegment::Line { to } => out.push(to),
                PathSegment::Cubic { c1, c2, to } => {
                    flatten_cubic(current, c1, c2, to, tol, &mut out)?;
                }
            }
            current = seg.end();
            check(
                out.len() <= MAX_FLATTEN_POINTS,
                "Flatten exceeded point budget",
            )?;
        }
        if self.closed && out.len() > 1 {
            let last = *out.last().unwrap();
            if dist(last, self.start) > 1e-9 {
                out.push(self.start);
            }
        }
        Ok(out)
    }

    /// Closed ring for planar CAD: drop duplicate closing vertex if present.
    pub fn to_ring(&self, tolerance: f64) -> Result<Vec<[f64; 2]>> {
        check(self.closed, "to_ring requires a closed path")?;
        let mut pts = self.flatten_tol(tolerance)?;
        if pts.len() >= 2 && dist(pts[0], *pts.last().unwrap()) <= 1e-9 {
            pts.pop();
        }
        check(
            pts.len() >= 3,
            "Closed path flattened to fewer than 3 points",
        )?;
        Ok(pts)
    }

    pub fn reverse(&self) -> Self {
        if self.segments.is_empty() {
            return self.clone();
        }
        let new_start = self.segments.last().unwrap().end();
        let n = self.segments.len();
        let mut new_segments = Vec::with_capacity(n);
        for k in 0..n {
            let orig_idx = n - 1 - k;
            let prev_end = if orig_idx == 0 {
                self.start
            } else {
                self.segments[orig_idx - 1].end()
            };
            match self.segments[orig_idx] {
                PathSegment::Line { .. } => new_segments.push(PathSegment::Line { to: prev_end }),
                PathSegment::Cubic { c1, c2, .. } => new_segments.push(PathSegment::Cubic {
                    c1: c2,
                    c2: c1,
                    to: prev_end,
                }),
            }
        }
        Self {
            start: new_start,
            segments: new_segments,
            closed: self.closed,
        }
    }

    /// Split segment `idx` at parameter `t` ∈ (0,1) via de Casteljau / lerp.
    pub fn insert_anchor(&self, idx: usize, t: f64) -> Result<Self> {
        check(idx < self.segments.len(), "Segment index out of range")?;
        check(
            self.segments.len() < MAX_SEGMENTS,
            "Path exceeds segment budget",
        )?;
        let t = t.clamp(1e-9, 1.0 - 1e-9);
        let from = segment_from(self.start, &self.segments, idx);
        let (before, after) = match self.segments[idx] {
            PathSegment::Line { to } => {
                let cut = lerp(from, to, t);
                (PathSegment::Line { to: cut }, PathSegment::Line { to })
            }
            PathSegment::Cubic { c1, c2, to } => {
                let q0 = lerp(from, c1, t);
                let q1 = lerp(c1, c2, t);
                let q2 = lerp(c2, to, t);
                let r0 = lerp(q0, q1, t);
                let r1 = lerp(q1, q2, t);
                let cut = lerp(r0, r1, t);
                (
                    PathSegment::Cubic {
                        c1: q0,
                        c2: r0,
                        to: cut,
                    },
                    PathSegment::Cubic { c1: r1, c2: q2, to },
                )
            }
        };
        let mut segments = Vec::with_capacity(self.segments.len() + 1);
        segments.extend_from_slice(&self.segments[..idx]);
        segments.push(before);
        segments.push(after);
        segments.extend_from_slice(&self.segments[idx + 1..]);
        Ok(Self {
            start: self.start,
            segments,
            closed: self.closed,
        })
    }

    /// Delete anchor `node`, bridging neighbours with a straight line.
    pub fn delete_anchor(&self, node: usize) -> Result<Self> {
        let n = self.anchor_count();
        check(node < n, "Anchor index out of range")?;
        check(n > 2, "Cannot delete: path would have fewer than 2 anchors")?;
        let anchors = self.anchors();
        let seg_count = self.segments.len();
        if !self.closed {
            if node == 0 {
                return Ok(Self {
                    start: anchors[1],
                    segments: self.segments[1..].to_vec(),
                    closed: false,
                });
            }
            if node == n - 1 {
                return Ok(Self {
                    start: self.start,
                    segments: self.segments[..seg_count - 1].to_vec(),
                    closed: false,
                });
            }
            let mut out = Vec::with_capacity(seg_count - 1);
            out.extend_from_slice(&self.segments[..node - 1]);
            out.push(PathSegment::Line {
                to: anchors[node + 1],
            });
            out.extend_from_slice(&self.segments[node + 1..]);
            return Ok(Self {
                start: self.start,
                segments: out,
                closed: false,
            });
        }
        let new_start = if node == 0 { anchors[1] } else { self.start };
        let mut out = Vec::with_capacity(n - 1);
        for j in 0..n {
            let b = (j + 1) % n;
            if j == node {
                continue;
            }
            if b == node {
                let dest = (node + 1) % n;
                out.push(PathSegment::Line { to: anchors[dest] });
                continue;
            }
            out.push(self.segments[j]);
        }
        Ok(Self {
            start: new_start,
            segments: out,
            closed: true,
        })
    }

    /// Cut at anchor: closed → one open path; open interior → two open paths.
    pub fn split_at_anchor(&self, node: usize) -> Result<Vec<Self>> {
        let n = self.anchor_count();
        check(node < n, "Anchor index out of range")?;
        if self.closed {
            let mut edges = self.segments.clone();
            if edges.len() < n {
                edges.push(PathSegment::Line { to: self.start });
            }
            let new_start = if node == 0 {
                self.start
            } else {
                edges[node - 1].end()
            };
            let mut new_segments = Vec::with_capacity(n);
            for k in 0..n {
                new_segments.push(edges[(node + k) % n]);
            }
            return Ok(vec![Self {
                start: new_start,
                segments: new_segments,
                closed: false,
            }]);
        }
        if node == 0 || node == n - 1 {
            return Err(crate::error("Cannot split an open path at an endpoint"));
        }
        let left = Self {
            start: self.start,
            segments: self.segments[..node].to_vec(),
            closed: false,
        };
        let right = Self {
            start: self.segments[node - 1].end(),
            segments: self.segments[node..].to_vec(),
            closed: false,
        };
        Ok(vec![left, right])
    }

    pub fn make_anchor_smooth(&self, node: usize) -> Result<Self> {
        let (prev, anchor, next) = neighbour_anchors(self, node)?;
        let dir = sub2(next, prev);
        let len = dir[0].hypot(dir[1]);
        check(len > 1e-9, "Degenerate smooth: neighbours coincide")?;
        let ux = dir[0] / len;
        let uy = dir[1] / len;
        let mut segments = self.segments.clone();
        if let Some(oi) = outgoing_segment(self, node) {
            let from = segment_from(self.start, &segments, oi);
            ensure_cubic(&mut segments, oi, from);
            let d = dist(anchor, next) / 3.0;
            if let PathSegment::Cubic { ref mut c1, .. } = segments[oi] {
                *c1 = [anchor[0] + ux * d, anchor[1] + uy * d];
            }
        }
        if let Some(ii) = incoming_segment(self, node) {
            let from = segment_from(self.start, &segments, ii);
            ensure_cubic(&mut segments, ii, from);
            let d = dist(anchor, prev) / 3.0;
            if let PathSegment::Cubic { ref mut c2, .. } = segments[ii] {
                *c2 = [anchor[0] - ux * d, anchor[1] - uy * d];
            }
        }
        Ok(Self {
            start: self.start,
            segments,
            closed: self.closed,
        })
    }

    pub fn make_anchor_corner(&self, node: usize) -> Result<Self> {
        let n = self.anchor_count();
        check(node < n, "Anchor index out of range")?;
        let anchor = if node == 0 {
            self.start
        } else {
            self.segments[node - 1].end()
        };
        let mut segments = self.segments.clone();
        if let Some(oi) = outgoing_segment(self, node)
            && let PathSegment::Cubic { ref mut c1, .. } = segments[oi]
        {
            *c1 = anchor;
        }
        if let Some(ii) = incoming_segment(self, node)
            && let PathSegment::Cubic { ref mut c2, .. } = segments[ii]
        {
            *c2 = anchor;
        }
        Ok(Self {
            start: self.start,
            segments,
            closed: self.closed,
        })
    }

    /// Insert mid-parameter anchors on every segment (densify).
    pub fn add_anchors(&self) -> Result<Self> {
        let mut path = self.clone();
        // Insert from the end so indices stay valid.
        for i in (0..self.segments.len()).rev() {
            path = path.insert_anchor(i, 0.5)?;
        }
        Ok(path)
    }

    /// Uniform subdivision: `levels` rounds of midpoint insertion.
    pub fn subdivide(&self, levels: usize) -> Result<Self> {
        check(levels <= 6, "Subdivide levels capped at 6")?;
        let mut path = self.clone();
        for _ in 0..levels {
            path = path.add_anchors()?;
        }
        Ok(path)
    }

    /// Simplify by flattening then Ramer–Douglas–Peucker; result is a polyline path.
    pub fn simplify(&self, tolerance: f64) -> Result<Self> {
        check(tolerance.is_finite(), "Invalid simplify tolerance")?;
        let mut pts = self.flatten()?;
        if self.closed && pts.len() >= 2 && dist(pts[0], *pts.last().unwrap()) <= 1e-9 {
            pts.pop();
        }
        let minimum = if self.closed { 3 } else { 2 };
        check(pts.len() >= minimum, "Path too short to simplify")?;
        let reduce = |points: &[[f64; 2]]| -> Vec<[f64; 2]> {
            points
                .iter()
                .zip(rdp_keep(points, tolerance.max(0.0)))
                .filter_map(|(p, keep)| keep.then_some(*p))
                .collect()
        };
        let reduced = if self.closed {
            // Pin the diameter endpoints rather than the arbitrary path seam.
            let (i, j) = farthest_pair(&pts);
            let mut first = reduce(&pts[i..=j]);
            let mut second = pts[j..].to_vec();
            second.extend_from_slice(&pts[..=i]);
            let second = reduce(&second);
            if second.len() > 2 {
                first.extend_from_slice(&second[1..second.len() - 1]);
            }
            first
        } else {
            reduce(&pts)
        };
        check(reduced.len() >= minimum, "Simplify would collapse the path")?;
        Self::from_polyline(&reduced, self.closed)
    }

    /// Stroke → filled outline via flattened parallel ribbons (butt caps, miter joins).
    pub fn outline_stroke(&self, width: f64) -> Result<Self> {
        let opts = crate::stroke::StrokeOptions {
            width,
            ..Default::default()
        };
        let outlines = crate::stroke::outline_stroke(self, &opts)?;
        // This legacy single-contour API encodes all boundaries with retraced
        // connectors. Their two opposite directions cancel under NonZero fill;
        // callers that support compounds should use `outline_stroke_with`.
        let first = outlines
            .first()
            .ok_or_else(|| crate::error("Stroke produced no outline"))?;
        let mut points = first.to_ring(FLATTEN_TOLERANCE)?;
        points.push(points[0]);
        for outline in outlines.iter().skip(1) {
            let ring = outline.to_ring(FLATTEN_TOLERANCE)?;
            points.extend_from_slice(&ring);
            points.push(ring[0]);
            points.push(first.start);
        }
        Self::from_polyline(&points, true)
    }

    pub fn outline_stroke_with(&self, opts: &crate::stroke::StrokeOptions) -> Result<Vec<Self>> {
        crate::stroke::outline_stroke(self, opts)
    }

    pub fn set_anchor_position(&self, node: usize, new_pos: [f64; 2]) -> Result<Self> {
        check(
            new_pos.iter().all(|x| x.is_finite()),
            "Non-finite anchor position",
        )?;
        let n = self.anchor_count();
        check(node < n, "Anchor index out of range")?;
        let mut path = self.clone();
        if node == 0 {
            let old = path.start;
            path.start = new_pos;
            if path.closed {
                for seg in &mut path.segments {
                    let end = seg.end();
                    if dist(end, old) <= 1e-3 {
                        match seg {
                            PathSegment::Line { to } => *to = new_pos,
                            PathSegment::Cubic { to, .. } => *to = new_pos,
                        }
                    }
                }
            }
        } else {
            match &mut path.segments[node - 1] {
                PathSegment::Line { to } => *to = new_pos,
                PathSegment::Cubic { to, .. } => *to = new_pos,
            }
        }
        Ok(path)
    }

    pub fn average_anchors(&self, nodes: &[usize], axis: AverageAxis) -> Result<Self> {
        check(!nodes.is_empty(), "No anchors to average")?;
        let anchors = self.anchors();
        let mut positions = Vec::with_capacity(nodes.len());
        for &node in nodes {
            check(node < anchors.len(), "Anchor index out of range")?;
            positions.push(anchors[node]);
        }
        let targets = average_anchor_targets(&positions, axis);
        let mut path = self.clone();
        for (&node, &pos) in nodes.iter().zip(targets.iter()) {
            path = path.set_anchor_position(node, pos)?;
        }
        Ok(path)
    }

    pub fn anchor_handles(&self, node: usize) -> Result<(Option<[f64; 2]>, Option<[f64; 2]>)> {
        check(node < self.anchor_count(), "Anchor index out of range")?;
        let incoming = incoming_segment(self, node).and_then(|i| match self.segments[i] {
            PathSegment::Cubic { c2, .. } => Some(c2),
            PathSegment::Line { .. } => None,
        });
        let outgoing = outgoing_segment(self, node).and_then(|i| match self.segments[i] {
            PathSegment::Cubic { c1, .. } => Some(c1),
            PathSegment::Line { .. } => None,
        });
        Ok((incoming, outgoing))
    }

    pub fn set_anchor_handle(&self, node: usize, side: HandleSide, pos: [f64; 2]) -> Result<Self> {
        check(pos.iter().all(|x| x.is_finite()), "Non-finite handle")?;
        check(node < self.anchor_count(), "Anchor index out of range")?;
        let idx = match side {
            HandleSide::In => incoming_segment(self, node),
            HandleSide::Out => outgoing_segment(self, node),
        }
        .ok_or_else(|| crate::error("No handle on that side"))?;
        let mut path = self.clone();
        let from = segment_from(path.start, &path.segments, idx);
        ensure_cubic(&mut path.segments, idx, from);
        if let PathSegment::Cubic {
            ref mut c1,
            ref mut c2,
            ..
        } = path.segments[idx]
        {
            match side {
                HandleSide::Out => *c1 = pos,
                HandleSide::In => *c2 = pos,
            }
        }
        Ok(path)
    }

    /// Offset a closed path via stroke-ring + boolean (kurbo/curvex parity).
    /// Open paths get a parallel curve of the flattened polyline.
    ///
    /// `segments` is accepted for API compatibility with the older polyline
    /// offset; the stroke-ring path uses flatten tolerance instead.
    pub fn offset(&self, distance: f64, join: &str, segments: usize) -> Result<Vec<Self>> {
        let _ = segments;
        check(
            distance.is_finite() && distance.abs() <= 1e6,
            "Invalid offset",
        )?;
        if distance.abs() < 1e-12 {
            return Ok(vec![self.clone()]);
        }
        if self.closed {
            return crate::path_offset::offset_closed_path(
                self,
                &[],
                &crate::path_offset::OffsetOptions {
                    distance,
                    join: crate::path_offset::parse_join(join),
                    segments: segments.max(1),
                    ..Default::default()
                },
            );
        }
        Ok(vec![offset_open_polyline(self, distance)?])
    }

    /// Closed multi-contour offset (outer + holes) with full [`path_offset`] options.
    pub fn offset_region(
        &self,
        holes: &[Self],
        opts: &crate::path_offset::OffsetOptions,
    ) -> Result<Vec<Self>> {
        crate::path_offset::offset_closed_path(self, holes, opts)
    }

    /// Rebuild cubics through the current anchors with uniform Catmull–Rom.
    pub fn smooth(&self) -> Result<Self> {
        let pts = self.anchors();
        check(pts.len() >= 2, "Smooth needs at least two anchors")?;
        let segments = catmull_rom_segments(&pts, self.closed);
        if self.closed {
            Self::closed(pts[0], segments)
        } else {
            Self::open(pts[0], segments)
        }
    }

    /// Drop the closing flag (and a redundant return-to-start segment).
    pub fn open_path(&self) -> Result<Self> {
        check(self.closed, "Path is already open")?;
        let mut segments = self.segments.clone();
        if let Some(last) = segments.last()
            && dist(last.end(), self.start) <= 1e-3
            && segments.len() >= 2
        {
            segments.pop();
        }
        Self::open(self.start, segments)
    }

    /// Delete edges whose both endpoints are in `nodes`. Returns open pieces.
    pub fn delete_segments(&self, nodes: &[usize]) -> Result<Vec<Self>> {
        let n = self.anchor_count();
        check(n >= 2, "Path too short to delete a segment")?;
        let mut selected: Vec<usize> = nodes.iter().copied().filter(|&m| m < n).collect();
        selected.sort_unstable();
        selected.dedup();
        check(selected.len() >= 2, "Delete segment needs two endpoints")?;
        let is_selected = |a: usize| selected.binary_search(&a).is_ok();
        let points = self.anchors();
        let edge_count = self.segments.len();
        let endpoints = |e: usize| -> (usize, usize) {
            if self.closed {
                (e, (e + 1) % n)
            } else {
                (e, e + 1)
            }
        };
        let deleted: Vec<bool> = (0..edge_count)
            .map(|e| {
                let (a, b) = endpoints(e);
                is_selected(a) && is_selected(b)
            })
            .collect();
        check(deleted.iter().any(|&d| d), "No selected edge to delete")?;
        let mut runs: Vec<([f64; 2], Vec<PathSegment>)> = Vec::new();
        if self.closed {
            let first_deleted = deleted.iter().position(|&d| d).unwrap();
            let (_, mut cur_start) = endpoints(first_deleted);
            let mut cur_segs = Vec::new();
            for k in 1..=edge_count {
                let e = (first_deleted + k) % edge_count;
                if deleted[e] {
                    if !cur_segs.is_empty() {
                        runs.push((points[cur_start], std::mem::take(&mut cur_segs)));
                    }
                    cur_start = endpoints(e).1;
                } else {
                    cur_segs.push(self.segments[e]);
                }
            }
            if !cur_segs.is_empty() {
                runs.push((points[cur_start], cur_segs));
            }
        } else {
            let mut cur_start = 0usize;
            let mut cur_segs = Vec::new();
            for e in 0..edge_count {
                if deleted[e] {
                    if !cur_segs.is_empty() {
                        runs.push((points[cur_start], std::mem::take(&mut cur_segs)));
                    }
                    cur_start = e + 1;
                } else {
                    cur_segs.push(self.segments[e]);
                }
            }
            if !cur_segs.is_empty() {
                runs.push((points[cur_start], cur_segs));
            }
        }
        check(!runs.is_empty(), "Delete segment produced nothing")?;
        runs.into_iter()
            .map(|(start, segs)| Self::open(start, segs))
            .collect()
    }

    pub fn apply_handle_link(
        &self,
        node: usize,
        moved: HandleSide,
        mode: HandleLink,
    ) -> Result<Self> {
        check(node < self.anchor_count(), "Anchor index out of range")?;
        if mode == HandleLink::Free {
            return Ok(self.clone());
        }
        let anchor = if node == 0 {
            self.start
        } else {
            self.segments[node - 1].end()
        };
        let (incoming, outgoing) = self.anchor_handles(node)?;
        let Some(moved_pos) = (match moved {
            HandleSide::In => incoming,
            HandleSide::Out => outgoing,
        }) else {
            return Ok(self.clone());
        };
        if !self.closed
            && ((node == 0 && moved == HandleSide::Out)
                || (node + 1 == self.anchor_count() && moved == HandleSide::In))
        {
            return Ok(self.clone());
        }
        let reflected = [
            2.0 * anchor[0] - moved_pos[0],
            2.0 * anchor[1] - moved_pos[1],
        ];
        let opposite = match moved {
            HandleSide::In => HandleSide::Out,
            HandleSide::Out => HandleSide::In,
        };
        self.set_anchor_handle(node, opposite, reflected)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AverageAxis {
    Horizontal,
    Vertical,
    Both,
}

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

fn catmull_rom_segments(pts: &[[f64; 2]], closed: bool) -> Vec<PathSegment> {
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

fn offset_open_polyline(path: &BezierPath, distance: f64) -> Result<BezierPath> {
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

fn validate_segments(segments: &[PathSegment]) -> Result<()> {
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

fn flatten_cubic(
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

fn neighbour_anchors(path: &BezierPath, node: usize) -> Result<([f64; 2], [f64; 2], [f64; 2])> {
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

fn incoming_segment(path: &BezierPath, node: usize) -> Option<usize> {
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

fn outgoing_segment(path: &BezierPath, node: usize) -> Option<usize> {
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

fn ensure_cubic(segments: &mut [PathSegment], idx: usize, from: [f64; 2]) {
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
fn farthest_pair(points: &[[f64; 2]]) -> (usize, usize) {
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

fn rdp_keep(points: &[[f64; 2]], eps: f64) -> Vec<bool> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ellipse_flattens_and_closes() {
        let path = BezierPath::from_circle([0., 0.], 10.).unwrap();
        assert!(path.closed);
        assert_eq!(path.segments.len(), 4);
        let ring = path.to_ring(0.1).unwrap();
        assert!(ring.len() >= 8);
        let area: f64 = (0..ring.len())
            .map(|i| {
                let a = ring[i];
                let b = ring[(i + 1) % ring.len()];
                a[0] * b[1] - a[1] * b[0]
            })
            .sum::<f64>()
            / 2.;
        assert!((area.abs() - std::f64::consts::PI * 100.).abs() < 5.);
    }

    #[test]
    fn de_casteljau_insert_preserves_endpoints() {
        let path = BezierPath::open(
            [0., 0.],
            vec![PathSegment::Cubic {
                c1: [1., 2.],
                c2: [2., 2.],
                to: [3., 0.],
            }],
        )
        .unwrap();
        let split = path.insert_anchor(0, 0.5).unwrap();
        assert_eq!(split.segments.len(), 2);
        assert_eq!(split.start, [0., 0.]);
        assert_eq!(split.segments[1].end(), [3., 0.]);
    }

    #[test]
    fn reverse_roundtrip_endpoints() {
        let path = BezierPath::from_polyline(&[[0., 0.], [1., 0.], [1., 1.]], false).unwrap();
        let rev = path.reverse();
        assert_eq!(rev.start, [1., 1.]);
        assert_eq!(rev.segments.last().unwrap().end(), [0., 0.]);
    }

    #[test]
    fn join_welds_coincident_ends() {
        let a = BezierPath::from_polyline(&[[0., 0.], [1., 0.]], false).unwrap();
        let b = BezierPath::from_polyline(&[[1., 0.], [2., 0.]], false).unwrap();
        let j = join_paths(&a, false, &b, false, 1e-6).unwrap();
        assert_eq!(j.segments.len(), 2);
        assert_eq!(j.segments.last().unwrap().end(), [2., 0.]);
    }

    #[test]
    fn simplify_reduces_collinear() {
        let path =
            BezierPath::from_polyline(&[[0., 0.], [1., 0.], [2., 0.], [3., 0.], [3., 1.]], false)
                .unwrap();
        let s = path.simplify(0.01).unwrap();
        assert!(s.anchor_count() <= 4);
    }

    #[test]
    fn outline_stroke_closed_rect() {
        let path = BezierPath::from_rect([0., 0.], [10., 10.]).unwrap();
        let outline = path.outline_stroke(2.).unwrap();
        assert!(outline.closed);
        assert!(outline.anchor_count() >= 8);
    }

    #[test]
    fn convert_shapes_to_path() {
        let r = BezierPath::from_rect([0., 0.], [4., 2.]).unwrap();
        let c = BezierPath::from_circle([1., 1.], 2.).unwrap();
        let p = BezierPath::from_polygon(&[[0., 0.], [1., 0.], [0.5, 1.]], true).unwrap();
        assert!(r.closed && c.closed && p.closed);
    }

    #[test]
    fn split_closed_opens_at_node() {
        let path = BezierPath::from_rect([0., 0.], [1., 1.]).unwrap();
        let pieces = path.split_at_anchor(1).unwrap();
        assert_eq!(pieces.len(), 1);
        assert!(!pieces[0].closed);
    }

    #[test]
    fn average_and_handle_link() {
        let path = BezierPath::from_polyline(&[[0., 0.], [2., 2.], [4., 0.]], false).unwrap();
        let avg = path
            .average_anchors(&[0, 2], AverageAxis::Horizontal)
            .unwrap();
        assert!((avg.start[0] - 2.).abs() < 1e-9);
        assert!((avg.segments[1].end()[0] - 2.).abs() < 1e-9);
        let cubic = BezierPath::open(
            [0., 0.],
            vec![
                PathSegment::Cubic {
                    c1: [1., 1.],
                    c2: [2., 1.],
                    to: [3., 0.],
                },
                PathSegment::Cubic {
                    c1: [4., 1.],
                    c2: [5., 1.],
                    to: [6., 0.],
                },
            ],
        )
        .unwrap();
        let edited = cubic
            .set_anchor_handle(1, HandleSide::Out, [3., 2.])
            .unwrap()
            .apply_handle_link(1, HandleSide::Out, HandleLink::Mirrored)
            .unwrap();
        let (inn, out) = edited.anchor_handles(1).unwrap();
        assert_eq!(out, Some([3., 2.]));
        assert_eq!(inn, Some([3., -2.]));
    }

    #[test]
    fn offset_closed_grows_area() {
        let path = BezierPath::from_rect([0., 0.], [4., 4.]).unwrap();
        let out = path.offset(1., "Miter", 8).unwrap();
        assert_eq!(out.len(), 1);
        let a: f64 = crate::rings::area(&out[0].to_ring(0.05).unwrap()).abs();
        assert!((a - 36.).abs() < 1., "area={a}");
    }

    #[test]
    fn smooth_makes_cubics() {
        let path = BezierPath::from_polyline(&[[0., 0.], [2., 1.], [4., 0.]], false).unwrap();
        let s = path.smooth().unwrap();
        assert!(
            s.segments
                .iter()
                .all(|seg| matches!(seg, PathSegment::Cubic { .. }))
        );
    }

    #[test]
    fn open_path_drops_close() {
        let path = BezierPath::from_rect([0., 0.], [2., 2.]).unwrap();
        let open = path.open_path().unwrap();
        assert!(!open.closed);
    }

    #[test]
    fn delete_segment_splits_open() {
        let path =
            BezierPath::from_polyline(&[[0., 0.], [1., 0.], [2., 0.], [3., 0.]], false).unwrap();
        let parts = path.delete_segments(&[1, 2]).unwrap();
        assert_eq!(parts.len(), 2);
        assert!(parts.iter().all(|p| !p.closed));
    }
    #[test]
    fn flatten_meets_small_tolerance_beyond_old_sample_cap() {
        let path = BezierPath::open(
            [0., 0.],
            vec![PathSegment::Cubic {
                c1: [0., 100.],
                c2: [100., 100.],
                to: [100., 0.],
            }],
        )
        .unwrap();
        let tolerance = 0.00001;
        let points = path.flatten_tol(tolerance).unwrap();
        assert!(points.len() > 256);
        for i in 0..=1000 {
            let t = i as f64 / 1000.;
            let u = 1. - t;
            let p = [100. * (3. * u * t * t + t * t * t), 300. * u * t];
            let nearest = points
                .windows(2)
                .map(|ab| {
                    let v = sub2(ab[1], ab[0]);
                    let q = sub2(p, ab[0]);
                    let parameter =
                        ((q[0] * v[0] + q[1] * v[1]) / (v[0] * v[0] + v[1] * v[1])).clamp(0., 1.);
                    dist(p, lerp(ab[0], ab[1], parameter))
                })
                .fold(f64::INFINITY, f64::min);
            assert!(nearest <= tolerance, "error {nearest} exceeds {tolerance}");
        }
    }
    #[test]
    fn flatten_preserves_collinear_overshoot_and_rejects_invalid_options() {
        let path = BezierPath::open(
            [0., 0.],
            vec![PathSegment::Cubic {
                c1: [10., 0.],
                c2: [10., 0.],
                to: [1., 0.],
            }],
        )
        .unwrap();
        let points = path.flatten_tol(0.01).unwrap();
        assert!(points.iter().any(|p| p[0] > 7.));
        for tolerance in [0., -1., f64::NAN, f64::INFINITY] {
            assert!(path.flatten_tol(tolerance).is_err());
        }
        assert!(BezierPath::open([f64::NAN, 0.], vec![]).is_err());
    }
}
