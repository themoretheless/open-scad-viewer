use super::*;

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
        for &to in points.iter().skip(1) {
            segments.push(PathSegment::Line { to });
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
        for (j, segment) in self.segments.iter().enumerate().take(n) {
            let b = (j + 1) % n;
            if j == node {
                continue;
            }
            if b == node {
                let dest = (node + 1) % n;
                out.push(PathSegment::Line { to: anchors[dest] });
                continue;
            }
            out.push(*segment);
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

    pub fn anchor_handles(&self, node: usize) -> Result<AnchorHandles> {
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
