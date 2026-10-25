use crate::{Circle, Constraint, Sketch, length, solve, subtract};
use crate::residuals::jacobian_into;

impl Sketch {
    /// Axis-aligned 2D bounding box `(min_xy, max_xy)` enclosing all points and circles in the sketch.
    /// Returns `None` if the sketch has no finite points or circles.
    pub fn bounding_box(&self) -> Option<([f64; 2], [f64; 2])> {
        let mut min = [f64::INFINITY; 2];
        let mut max = [f64::NEG_INFINITY; 2];
        let mut any = false;
        for &p in &self.points {
            if p[0].is_finite() && p[1].is_finite() {
                min[0] = min[0].min(p[0]);
                min[1] = min[1].min(p[1]);
                max[0] = max[0].max(p[0]);
                max[1] = max[1].max(p[1]);
                any = true;
            }
        }
        for c in &self.circles {
            if let Some(&center) = self.points.get(c.center)
                && center[0].is_finite()
                && center[1].is_finite()
                && c.radius.is_finite()
                && c.radius >= 0.0
            {
                min[0] = min[0].min(center[0] - c.radius);
                min[1] = min[1].min(center[1] - c.radius);
                max[0] = max[0].max(center[0] + c.radius);
                max[1] = max[1].max(center[1] + c.radius);
                any = true;
            }
        }
        any.then_some((min, max))
    }

    /// Identifies overconstrained or conflicting constraint indices at tolerance `tol`
    /// via residual magnitude and row-by-row Jacobian linear dependency analysis.
    pub fn overconstrained_constraints(&self, tol: f64) -> Vec<usize> {
        let tol = tol.clamp(1e-8, 0.1);
        let Ok(solution) = solve(self, tol) else {
            return Vec::new();
        };
        let solved = &solution.sketch;
        let mut x: Vec<_> = solved
            .points
            .iter()
            .flatten()
            .copied()
            .chain(solved.circles.iter().map(|c| c.radius))
            .collect();
        let n = x.len();
        let mut row_offsets = Vec::with_capacity(solved.constraints.len());
        let mut m = 0;
        for c in &solved.constraints {
            row_offsets.push(m);
            m += Self::constraint_residual_len(c);
        }
        if m == 0 || n == 0 {
            return Vec::new();
        }
        let mut j_flat = vec![0.0; m * n];
        jacobian_into(solved, &row_offsets, &mut x, &mut j_flat, n);

        let mut basis: Vec<(usize, Vec<f64>)> = Vec::with_capacity(n);
        let mut out = Vec::new();
        for (ci, c) in solved.constraints.iter().enumerate() {
            let r0 = row_offsets[ci];
            let len = Self::constraint_residual_len(c);
            let max_res = solution.residuals[ci]
                .iter()
                .map(|v| v.abs())
                .fold(0.0, f64::max);
            let mut dependent = false;
            for k in 0..len {
                let mut row = j_flat[(r0 + k) * n..(r0 + k + 1) * n].to_vec();
                for &(pivot_col, ref b_row) in &basis {
                    let factor = row[pivot_col];
                    if factor != 0.0 {
                        for col in pivot_col..n {
                            row[col] -= factor * b_row[col];
                        }
                    }
                }
                if let Some((pivot_col, &pivot_val)) = row
                    .iter()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| a.abs().total_cmp(&b.abs()))
                    && pivot_val.abs() >= 1e-7
                {
                    let inv = 1.0 / pivot_val;
                    for v in &mut row[pivot_col..] {
                        *v *= inv;
                    }
                    basis.push((pivot_col, row));
                } else {
                    dependent = true;
                }
            }
            if dependent || max_res > tol {
                out.push(ci);
            }
        }
        out
    }

    /// Automatically suggests missing geometric constraints (`Coincident`, `Horizontal`,
    /// `Vertical`, `Parallel`, `Perpendicular`, `EqualLength`, and `EqualRadius`) that hold
    /// within `linear_tol` and `angle_tol_rad` on the current sketch geometry.
    pub fn suggest_constraints(&self, linear_tol: f64, angle_tol_rad: f64) -> Vec<Constraint> {
        if !linear_tol.is_finite()
            || linear_tol < 0.0
            || !angle_tol_rad.is_finite()
            || angle_tol_rad < 0.0
        {
            return Vec::new();
        }
        let mut suggestions = Vec::new();
        let n_pts = self.points.len();
        let has_pair_constraint = |kind: u8, i: usize, j: usize| -> bool {
            let (lo, hi) = (i.min(j), i.max(j));
            self.constraints.iter().any(|c| match (kind, c) {
                (0, &Constraint::Coincident { a, b }) => (a.min(b), a.max(b)) == (lo, hi),
                (1, &Constraint::Horizontal { a, b }) => (a.min(b), a.max(b)) == (lo, hi),
                (2, &Constraint::Vertical { a, b }) => (a.min(b), a.max(b)) == (lo, hi),
                (3, &Constraint::EqualRadius { a, b }) => (a.min(b), a.max(b)) == (lo, hi),
                _ => false,
            })
        };

        // 1. Point-pair Coincident, Horizontal, Vertical.
        for i in 0..n_pts {
            let pa = self.points[i];
            if !pa[0].is_finite() || !pa[1].is_finite() {
                continue;
            }
            for j in (i + 1)..n_pts {
                let pb = self.points[j];
                if !pb[0].is_finite() || !pb[1].is_finite() {
                    continue;
                }
                let d = subtract(pb, pa);
                let len = length(d);
                if len <= linear_tol {
                    if !has_pair_constraint(0, i, j) {
                        suggestions.push(Constraint::Coincident { a: i, b: j });
                    }
                    continue;
                }
                let dy_angle = (d[1].abs() / len).asin();
                let dx_angle = (d[0].abs() / len).asin();
                if (d[1].abs() <= linear_tol || dy_angle <= angle_tol_rad)
                    && !has_pair_constraint(1, i, j)
                {
                    suggestions.push(Constraint::Horizontal { a: i, b: j });
                } else if (d[0].abs() <= linear_tol || dx_angle <= angle_tol_rad)
                    && !has_pair_constraint(2, i, j)
                {
                    suggestions.push(Constraint::Vertical { a: i, b: j });
                }
            }
        }

        // Collect active line segments referenced by existing constraints, or consecutive point pairs if none.
        let mut segments: Vec<(usize, usize)> = Vec::new();
        let push_seg = |segments: &mut Vec<(usize, usize)>, a: usize, b: usize| {
            if a < n_pts && b < n_pts && a != b {
                let seg = (a.min(b), a.max(b));
                if !segments.contains(&seg) {
                    segments.push(seg);
                }
            }
        };
        for c in &self.constraints {
            match *c {
                Constraint::Horizontal { a, b }
                | Constraint::Vertical { a, b }
                | Constraint::Distance { a, b, .. }
                | Constraint::FixAngle { a, b, .. }
                | Constraint::PointToLineDistance { a, b, .. }
                | Constraint::Midpoint { a, b, .. }
                | Constraint::TangentLineCircle { a, b, .. }
                | Constraint::PointOnLine { a, b, .. } => push_seg(&mut segments, a, b),
                Constraint::Parallel { a, b, c, d }
                | Constraint::Perpendicular { a, b, c, d }
                | Constraint::EqualLength { a, b, c, d }
                | Constraint::Angle { a, b, c, d, .. } => {
                    push_seg(&mut segments, a, b);
                    push_seg(&mut segments, c, d);
                }
                _ => {}
            }
        }
        if segments.is_empty() && n_pts >= 2 {
            for i in 0..(n_pts - 1) {
                push_seg(&mut segments, i, i + 1);
            }
        }

        // 2. Segment-pair Parallel, Perpendicular, EqualLength.
        let min_seg_len = linear_tol.max(1e-9);
        let mut seg_data = Vec::with_capacity(segments.len());
        for &(a, b) in &segments {
            let u = subtract(self.points[b], self.points[a]);
            let lu = length(u);
            if lu > min_seg_len {
                seg_data.push((a, b, u[0], u[1], lu));
            }
        }
        for s1 in 0..seg_data.len() {
            let (a, b, ux, uy, lu) = seg_data[s1];
            for &(c, d, vx, vy, lv) in &seg_data[(s1 + 1)..] {
                let inv_lulv = 1.0 / (lu * lv);
                let cross_norm = (ux * vy - uy * vx).abs() * inv_lulv;
                let dot_norm = (ux * vx + uy * vy).abs() * inv_lulv;
                if cross_norm.clamp(0.0, 1.0).asin() <= angle_tol_rad {
                    let already = self.constraints.iter().any(|con| {
                        matches!(
                            *con,
                            Constraint::Parallel {
                                a: ca,
                                b: cb,
                                c: cc,
                                d: cd
                            } if ((ca.min(cb), ca.max(cb)) == (a, b)
                                && (cc.min(cd), cc.max(cd)) == (c, d))
                                || ((ca.min(cb), ca.max(cb)) == (c, d)
                                    && (cc.min(cd), cc.max(cd)) == (a, b))
                        )
                    });
                    if !already {
                        suggestions.push(Constraint::Parallel { a, b, c, d });
                    }
                } else if dot_norm.clamp(0.0, 1.0).asin() <= angle_tol_rad {
                    let already = self.constraints.iter().any(|con| {
                        matches!(
                            *con,
                            Constraint::Perpendicular {
                                a: ca,
                                b: cb,
                                c: cc,
                                d: cd
                            } if ((ca.min(cb), ca.max(cb)) == (a, b)
                                && (cc.min(cd), cc.max(cd)) == (c, d))
                                || ((ca.min(cb), ca.max(cb)) == (c, d)
                                    && (cc.min(cd), cc.max(cd)) == (a, b))
                        )
                    });
                    if !already {
                        suggestions.push(Constraint::Perpendicular { a, b, c, d });
                    }
                }
                if (lu - lv).abs() <= linear_tol {
                    let already = self.constraints.iter().any(|con| {
                        matches!(
                            *con,
                            Constraint::EqualLength {
                                a: ca,
                                b: cb,
                                c: cc,
                                d: cd
                            } if ((ca.min(cb), ca.max(cb)) == (a, b)
                                && (cc.min(cd), cc.max(cd)) == (c, d))
                                || ((ca.min(cb), ca.max(cb)) == (c, d)
                                    && (cc.min(cd), cc.max(cd)) == (a, b))
                        )
                    });
                    if !already {
                        suggestions.push(Constraint::EqualLength { a, b, c, d });
                    }
                }
            }
        }

        // 3. Circle-pair EqualRadius.
        for i in 0..self.circles.len() {
            let ra = self.circles[i].radius;
            if !ra.is_finite() {
                continue;
            }
            for j in (i + 1)..self.circles.len() {
                let rb = self.circles[j].radius;
                if rb.is_finite()
                    && (ra - rb).abs() <= linear_tol
                    && !has_pair_constraint(3, i, j)
                {
                    suggestions.push(Constraint::EqualRadius { a: i, b: j });
                }
            }
        }
        suggestions
    }

    /// Tessellates all sketch primitives (constrained line segments, circles, and circular arcs)
    /// into 2D polylines with `arc_segments` subdivisions per full circle/arc for viewport
    /// rendering and DXF/SVG export.
    pub fn render_polylines(&self, arc_segments: usize) -> Vec<Vec<[f64; 2]>> {
        let segs = arc_segments.clamp(4, 4096);
        let mut polylines = Vec::new();
        let n_pts = self.points.len();
        let mut line_pairs: Vec<(usize, usize)> = Vec::new();
        let push_line = |line_pairs: &mut Vec<(usize, usize)>, a: usize, b: usize| {
            if a < n_pts && b < n_pts && a != b {
                let key = (a.min(b), a.max(b));
                if !line_pairs.contains(&key) {
                    line_pairs.push(key);
                }
            }
        };
        for c in &self.constraints {
            match *c {
                Constraint::Horizontal { a, b }
                | Constraint::Vertical { a, b }
                | Constraint::Distance { a, b, .. }
                | Constraint::FixAngle { a, b, .. }
                | Constraint::PointToLineDistance { a, b, .. }
                | Constraint::Midpoint { a, b, .. }
                | Constraint::TangentLineCircle { a, b, .. }
                | Constraint::PointOnLine { a, b, .. } => push_line(&mut line_pairs, a, b),
                Constraint::Parallel { a, b, c, d }
                | Constraint::Perpendicular { a, b, c, d }
                | Constraint::EqualLength { a, b, c, d }
                | Constraint::Angle { a, b, c, d, .. } => {
                    push_line(&mut line_pairs, a, b);
                    push_line(&mut line_pairs, c, d);
                }
                Constraint::Symmetric { axis_a, axis_b, .. } => {
                    push_line(&mut line_pairs, axis_a, axis_b);
                }
                Constraint::Collinear { a, b, c } => {
                    push_line(&mut line_pairs, a, b);
                    push_line(&mut line_pairs, b, c);
                }
                _ => {}
            }
        }
        if line_pairs.is_empty() && n_pts >= 2 && self.circles.is_empty() {
            for i in 0..(n_pts - 1) {
                push_line(&mut line_pairs, i, i + 1);
            }
        }
        for (a, b) in line_pairs {
            let pa = self.points[a];
            let pb = self.points[b];
            if pa[0].is_finite() && pa[1].is_finite() && pb[0].is_finite() && pb[1].is_finite() {
                polylines.push(vec![pa, pb]);
            }
        }

        // Render arcs referenced by PointOnArc, or full circles.
        let mut arc_circles = vec![false; self.circles.len()];
        for c in &self.constraints {
            if let Constraint::PointOnArc {
                circle,
                start_angle,
                end_angle,
                ..
            } = *c
                && let Some(cir) = self.circles.get(circle)
                && let Some(&center) = self.points.get(cir.center)
                && center[0].is_finite()
                && center[1].is_finite()
                && cir.radius.is_finite()
                && cir.radius > 0.0
                && start_angle.is_finite()
                && end_angle.is_finite()
            {
                arc_circles[circle] = true;
                let span = (end_angle - start_angle).rem_euclid(std::f64::consts::TAU);
                let span = if span <= 1e-12 {
                    std::f64::consts::TAU
                } else {
                    span
                };
                let steps = ((segs as f64 * (span / std::f64::consts::TAU)).ceil() as usize).max(2);
                let mut poly = Vec::with_capacity(steps + 1);
                for k in 0..=steps {
                    let theta = start_angle + span * (k as f64 / steps as f64);
                    let (s, c_val) = theta.sin_cos();
                    poly.push([center[0] + cir.radius * c_val, center[1] + cir.radius * s]);
                }
                polylines.push(poly);
            }
        }
        for (ci, cir) in self.circles.iter().enumerate() {
            if arc_circles[ci] {
                continue;
            }
            if let Some(&center) = self.points.get(cir.center)
                && center[0].is_finite()
                && center[1].is_finite()
                && cir.radius.is_finite()
                && cir.radius > 0.0
            {
                let mut poly = Vec::with_capacity(segs + 1);
                for k in 0..=segs {
                    let theta = std::f64::consts::TAU * (k as f64 / segs as f64);
                    let (s, c_val) = theta.sin_cos();
                    poly.push([center[0] + cir.radius * c_val, center[1] + cir.radius * s]);
                }
                polylines.push(poly);
            }
        }
        polylines
    }

    /// Computes a mitered 2D offset polygon for the closed sketch loop specified by
    /// `loop_point_indices` at signed offset `distance` (positive offsets outward for CCW loops,
    /// with miter-limit clamping at $4 \cdot |d|$).
    pub fn offset_closed_loop(&self, loop_point_indices: &[usize], distance: f64) -> Vec<[f64; 2]> {
        if !distance.is_finite() || loop_point_indices.len() < 3 {
            return Vec::new();
        }
        let n = if loop_point_indices.len() > 3
            && loop_point_indices.first() == loop_point_indices.last()
        {
            loop_point_indices.len() - 1
        } else {
            loop_point_indices.len()
        };
        if n < 3 {
            return Vec::new();
        }
        let mut pts = Vec::with_capacity(n);
        for &idx in &loop_point_indices[..n] {
            let Some(&p) = self.points.get(idx) else {
                return Vec::new();
            };
            if !p[0].is_finite() || !p[1].is_finite() {
                return Vec::new();
            }
            pts.push(p);
        }
        if distance.abs() <= 1e-15 {
            return pts;
        }
        let miter_limit = 4.0 * distance.abs();
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let p_prev = pts[(i + n - 1) % n];
            let p_curr = pts[i];
            let p_next = pts[(i + 1) % n];
            let e1 = subtract(p_curr, p_prev);
            let e2 = subtract(p_next, p_curr);
            let l1 = length(e1);
            let l2 = length(e2);
            if l1 <= 1e-12 || l2 <= 1e-12 {
                continue;
            }
            // Right-hand outward normal for CCW loop: (dy, -dx) / len.
            let n1 = [e1[1] / l1, -e1[0] / l1];
            let n2 = [e2[1] / l2, -e2[0] / l2];
            let bis = [n1[0] + n2[0], n1[1] + n2[1]];
            let denom = 1.0 + (n1[0] * n2[0] + n1[1] * n2[1]);
            if denom > 1e-6 {
                let scale = (distance / denom).clamp(-miter_limit, miter_limit);
                out.push([p_curr[0] + bis[0] * scale, p_curr[1] + bis[1] * scale]);
            } else {
                out.push([p_curr[0] + n1[0] * distance, p_curr[1] + n1[1] * distance]);
            }
        }
        out
    }

    /// Replaces or augments a sharp corner `(p_prev, p_corner, p_next)` with a tangent circular
    /// fillet of the given `radius`. Appends `(t1_idx, t2_idx, center_idx)` to `self.points`,
    /// appends the fillet [`Circle`] to `self.circles`, and adds `PointOnArc` constraints so the
    /// fillet arc is rendered and constrained. Returns `Some((t1_idx, t2_idx, center_idx))`.
    pub fn fillet_corner(
        &mut self,
        p_prev: usize,
        p_corner: usize,
        p_next: usize,
        radius: f64,
    ) -> Option<(usize, usize, usize)> {
        if !radius.is_finite()
            || radius <= 1e-9
            || p_prev >= self.points.len()
            || p_corner >= self.points.len()
            || p_next >= self.points.len()
            || p_prev == p_corner
            || p_next == p_corner
            || p_prev == p_next
        {
            return None;
        }
        let a = self.points[p_prev];
        let b = self.points[p_corner];
        let c = self.points[p_next];
        if [a, b, c].iter().flatten().any(|v| !v.is_finite()) {
            return None;
        }
        let u = subtract(a, b);
        let v = subtract(c, b);
        let lu = length(u);
        let lv = length(v);
        if lu <= 1e-9 || lv <= 1e-9 {
            return None;
        }
        let ux = u[0] / lu;
        let uy = u[1] / lu;
        let vx = v[0] / lv;
        let vy = v[1] / lv;
        let cos_theta = (ux * vx + uy * vy).clamp(-1.0, 1.0);
        let theta = cos_theta.acos();
        if theta <= 1e-6 || (std::f64::consts::PI - theta) <= 1e-6 {
            return None;
        }
        let half_theta = 0.5 * theta;
        let tan_half = half_theta.tan();
        let sin_half = half_theta.sin();
        if tan_half <= 1e-12 || sin_half <= 1e-12 {
            return None;
        }
        let trim_dist = radius / tan_half;
        if trim_dist >= lu || trim_dist >= lv {
            return None;
        }
        let t1 = [b[0] + ux * trim_dist, b[1] + uy * trim_dist];
        let t2 = [b[0] + vx * trim_dist, b[1] + vy * trim_dist];
        let bis = [ux + vx, uy + vy];
        let bis_len = length(bis);
        if bis_len <= 1e-12 {
            return None;
        }
        let center_dist = radius / sin_half;
        let center = [
            b[0] + (bis[0] / bis_len) * center_dist,
            b[1] + (bis[1] / bis_len) * center_dist,
        ];
        let t1_idx = self.points.len();
        self.points.push(t1);
        let t2_idx = self.points.len();
        self.points.push(t2);
        let center_idx = self.points.len();
        self.points.push(center);
        let circle_idx = self.circles.len();
        self.circles.push(Circle {
            center: center_idx,
            radius,
        });
        let a1 = (t1[1] - center[1]).atan2(t1[0] - center[0]);
        let a2 = (t2[1] - center[1]).atan2(t2[0] - center[0]);
        let span_ccw = (a2 - a1).rem_euclid(std::f64::consts::TAU);
        let (start_angle, end_angle) = if span_ccw <= std::f64::consts::PI {
            (a1, a2)
        } else {
            (a2, a1)
        };
        self.constraints.push(Constraint::PointOnArc {
            point: t1_idx,
            circle: circle_idx,
            start_angle,
            end_angle,
        });
        self.constraints.push(Constraint::PointOnArc {
            point: t2_idx,
            circle: circle_idx,
            start_angle,
            end_angle,
        });
        Some((t1_idx, t2_idx, center_idx))
    }

    /// Computes two chamfer setback points along the incoming and outgoing edges at `p_corner`:
    /// `t1` at distance `dist_prev` along `(p_corner -> p_prev)` and `t2` at distance `dist_next`
    /// along `(p_corner -> p_next)`. Appends `(t1, t2)` to `self.points` and returns their point
    /// indices `Some((t1_idx, t2_idx))`.
    pub fn chamfer_corner(
        &mut self,
        p_prev: usize,
        p_corner: usize,
        p_next: usize,
        dist_prev: f64,
        dist_next: f64,
    ) -> Option<(usize, usize)> {
        if !dist_prev.is_finite()
            || !dist_next.is_finite()
            || dist_prev <= 1e-9
            || dist_next <= 1e-9
            || p_prev >= self.points.len()
            || p_corner >= self.points.len()
            || p_next >= self.points.len()
            || p_prev == p_corner
            || p_next == p_corner
            || p_prev == p_next
        {
            return None;
        }
        let a = self.points[p_prev];
        let b = self.points[p_corner];
        let c = self.points[p_next];
        if [a, b, c].iter().flatten().any(|v| !v.is_finite()) {
            return None;
        }
        let u = subtract(a, b);
        let v = subtract(c, b);
        let lu = length(u);
        let lv = length(v);
        if lu <= dist_prev || lv <= dist_next {
            return None;
        }
        let t1 = [b[0] + (u[0] / lu) * dist_prev, b[1] + (u[1] / lu) * dist_prev];
        let t2 = [b[0] + (v[0] / lv) * dist_next, b[1] + (v[1] / lv) * dist_next];
        let t1_idx = self.points.len();
        self.points.push(t1);
        let t2_idx = self.points.len();
        self.points.push(t2);
        Some((t1_idx, t2_idx))
    }

    /// Reflects each point in `point_indices` across the line through `(axis_p0, axis_p1)`,
    /// appends the mirrored points to `self.points`, and optionally appends a
    /// [`Constraint::Symmetric`] for each original/mirrored pair. Returns the new point indices.
    pub fn mirror_points(
        &mut self,
        point_indices: &[usize],
        axis_p0: usize,
        axis_p1: usize,
        add_symmetric_constraints: bool,
    ) -> Option<Vec<usize>> {
        if axis_p0 >= self.points.len()
            || axis_p1 >= self.points.len()
            || axis_p0 == axis_p1
            || point_indices.iter().any(|&idx| idx >= self.points.len())
        {
            return None;
        }
        let p0 = self.points[axis_p0];
        let p1 = self.points[axis_p1];
        if !p0[0].is_finite()
            || !p0[1].is_finite()
            || !p1[0].is_finite()
            || !p1[1].is_finite()
        {
            return None;
        }
        let d = subtract(p1, p0);
        let l2 = d[0] * d[0] + d[1] * d[1];
        if l2 <= 1e-18 {
            return None;
        }
        let mut reflected = Vec::with_capacity(point_indices.len());
        for &idx in point_indices {
            let p = self.points[idx];
            if !p[0].is_finite() || !p[1].is_finite() {
                return None;
            }
            let v = subtract(p, p0);
            let t = (v[0] * d[0] + v[1] * d[1]) / l2;
            let foot = [p0[0] + t * d[0], p0[1] + t * d[1]];
            reflected.push((idx, [2.0 * foot[0] - p[0], 2.0 * foot[1] - p[1]]));
        }
        let mut new_indices = Vec::with_capacity(reflected.len());
        for (orig_idx, pt) in reflected {
            let new_idx = self.points.len();
            self.points.push(pt);
            new_indices.push(new_idx);
            if add_symmetric_constraints {
                self.constraints.push(Constraint::Symmetric {
                    a: orig_idx,
                    b: new_idx,
                    axis_a: axis_p0,
                    axis_b: axis_p1,
                });
            }
        }
        Some(new_indices)
    }

    /// Creates a regular polygon with `sides` vertices (`sides >= 3`) inscribed in a circle of
    /// `radius` at `center`, starting at `start_angle_rad`, and appends `EqualLength` constraints
    /// between consecutive closed polygon edges. Returns `Some((vertex_indices, constraint_indices))`.
    pub fn add_regular_polygon(
        &mut self,
        center: (f64, f64),
        radius: f64,
        sides: usize,
        start_angle_rad: f64,
    ) -> Option<(Vec<usize>, Vec<usize>)> {
        if !(3..=512).contains(&sides)
            || !center.0.is_finite()
            || !center.1.is_finite()
            || !radius.is_finite()
            || radius <= 1e-9
            || !start_angle_rad.is_finite()
            || self.points.len().saturating_add(sides) > 512
        {
            return None;
        }
        let mut vertex_indices = Vec::with_capacity(sides);
        for i in 0..sides {
            let theta = start_angle_rad + std::f64::consts::TAU * (i as f64 / sides as f64);
            let (s, c) = theta.sin_cos();
            let pt = [center.0 + radius * c, center.1 + radius * s];
            if !pt[0].is_finite() || !pt[1].is_finite() || pt[0].abs() > 1e6 || pt[1].abs() > 1e6 {
                return None;
            }
            let idx = self.points.len();
            self.points.push(pt);
            vertex_indices.push(idx);
        }
        let mut constraint_indices = Vec::with_capacity(sides - 1);
        for i in 0..(sides - 1) {
            let a = vertex_indices[i];
            let b = vertex_indices[i + 1];
            let c = vertex_indices[i + 1];
            let d = vertex_indices[(i + 2) % sides];
            let ci = self.constraints.len();
            self.constraints.push(Constraint::EqualLength { a, b, c, d });
            constraint_indices.push(ci);
        }
        Some((vertex_indices, constraint_indices))
    }

    /// Estimates remaining degrees of freedom via structural equation count:
    /// `2 * num_points - scalar_constraint_equations_count`.
    pub fn degrees_of_freedom(&self) -> isize {
        let eqs: usize = self
            .constraints
            .iter()
            .map(Self::constraint_residual_len)
            .sum();
        (2 * self.points.len()) as isize - eqs as isize
    }
}
