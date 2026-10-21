use super::*;

impl Sketch {
    pub(super) fn validate(&self) -> Result<()> {
        if self.points.is_empty()
            || self.points.len() > 512
            || self.circles.len() > 256
            || self.constraints.len() > 2048
            || self
                .points
                .iter()
                .flatten()
                .any(|v| !v.is_finite() || v.abs() > 1e6)
            || self.circles.iter().any(|c| {
                c.center >= self.points.len()
                    || !c.radius.is_finite()
                    || c.radius <= 1e-9
                    || c.radius > 1e6
            })
        {
            return Err(error("Sketch input or budget invalid"));
        }
        let point = |i: usize| i < self.points.len();
        let circle = |i: usize| i < self.circles.len();
        let positive = |v: f64| v.is_finite() && v > 0. && v <= 1e6;
        for c in &self.constraints {
            let valid = match *c {
                Constraint::Fix { point: p, at } => {
                    point(p) && at.iter().all(|v| v.is_finite() && v.abs() <= 1e6)
                }
                Constraint::Horizontal { a, b }
                | Constraint::Vertical { a, b }
                | Constraint::Coincident { a, b } => point(a) && point(b),
                Constraint::Distance { a, b, value } => {
                    point(a) && point(b) && value.is_finite() && value > 0.
                }
                Constraint::Parallel { a, b, c, d }
                | Constraint::Perpendicular { a, b, c, d }
                | Constraint::EqualLength { a, b, c, d } => [a, b, c, d].iter().all(|&p| point(p)),
                Constraint::Angle {
                    a,
                    b,
                    c,
                    d,
                    radians,
                } => {
                    [a, b, c, d].iter().all(|&p| point(p))
                        && a != b
                        && c != d
                        && radians.is_finite()
                        && radians.abs() <= std::f64::consts::TAU
                }
                Constraint::PointToLineDistance {
                    point: p,
                    a,
                    b,
                    value,
                } => point(p) && point(a) && point(b) && a != b && value.is_finite() && value >= 0.,
                Constraint::Midpoint { point: p, a, b } => point(p) && point(a) && point(b),
                Constraint::Symmetric {
                    a,
                    b,
                    axis_a,
                    axis_b,
                } => [a, b, axis_a, axis_b].iter().all(|&p| point(p)) && axis_a != axis_b,
                Constraint::Collinear { a, b, c } => {
                    point(a) && point(b) && point(c) && a != b
                }
                Constraint::Radius { circle: c, value } => circle(c) && positive(value),
                Constraint::Concentric { a, b } | Constraint::EqualRadius { a, b } => {
                    circle(a) && circle(b) && a != b
                }
                Constraint::PointOnCircle {
                    point: p,
                    circle: c,
                } => point(p) && circle(c),
                Constraint::TangentLineCircle { a, b, circle: c } => {
                    point(a) && point(b) && a != b && circle(c)
                }
                Constraint::TangentCircles { a, b, .. } => circle(a) && circle(b) && a != b,
                Constraint::PointOnLine { point: p, a, b } => {
                    point(p) && point(a) && point(b) && a != b
                }
                Constraint::FixAngle { a, b, radians } => {
                    point(a)
                        && point(b)
                        && a != b
                        && radians.is_finite()
                        && radians.abs() <= std::f64::consts::TAU
                }
                Constraint::EqualAngle {
                    a,
                    b,
                    c,
                    d,
                    e,
                    f,
                    g,
                    h,
                } => {
                    [a, b, c, d, e, f, g, h].iter().all(|&p| point(p))
                        && a != b
                        && c != d
                        && e != f
                        && g != h
                }
                Constraint::PointOnArc {
                    point: p,
                    circle: c,
                    start_angle,
                    end_angle,
                } => {
                    point(p)
                        && circle(c)
                        && start_angle.is_finite()
                        && end_angle.is_finite()
                        && start_angle.abs() <= std::f64::consts::TAU
                        && end_angle.abs() <= std::f64::consts::TAU
                }
            };
            if !valid {
                return Err(error("Invalid sketch constraint reference/value"));
            }
        }
        Ok(())
    }
    pub(super) fn constraint_residual_len(constraint: &Constraint) -> usize {
        match *constraint {
            Constraint::Fix { .. }
            | Constraint::Coincident { .. }
            | Constraint::Angle { .. }
            | Constraint::Midpoint { .. }
            | Constraint::Symmetric { .. }
            | Constraint::Concentric { .. } => 2,
            _ => 1,
        }
    }

    pub(super) fn eval_constraint_residual(&self, constraint: &Constraint, x: &[f64]) -> ([f64; 2], usize) {
        let p = |i: usize| [x[2 * i], x[2 * i + 1]];
        let r = |i: usize| x[2 * self.points.len() + i];
        let center = |i: usize| p(self.circles[i].center);
        match *constraint {
            Constraint::Fix { point, at } => (subtract(p(point), at), 2),
            Constraint::Horizontal { a, b } => ([p(b)[1] - p(a)[1], 0.0], 1),
            Constraint::Vertical { a, b } => ([p(b)[0] - p(a)[0], 0.0], 1),
            Constraint::Coincident { a, b } => (subtract(p(a), p(b)), 2),
            Constraint::Distance { a, b, value } => ([length(subtract(p(a), p(b))) - value, 0.0], 1),
            Constraint::Parallel { a, b, c, d }
            | Constraint::Perpendicular { a, b, c, d }
            | Constraint::EqualLength { a, b, c, d } => {
                let u = subtract(p(b), p(a));
                let v = subtract(p(d), p(c));
                let lu = length(u);
                let lv = length(v);
                let scale = lu.max(lv).max(1e-12);
                let val = match constraint {
                    Constraint::Parallel { .. } => (u[0] * v[1] - u[1] * v[0]) / scale,
                    Constraint::Perpendicular { .. } => (u[0] * v[0] + u[1] * v[1]) / scale,
                    _ => lu - lv,
                };
                ([val, 0.0], 1)
            }
            Constraint::Angle {
                a,
                b,
                c,
                d,
                radians,
            } => {
                let u = subtract(p(b), p(a));
                let v = subtract(p(d), p(c));
                let lu = length(u).max(1e-12);
                let lv = length(v).max(1e-12);
                let ux = u[0] / lu;
                let uy = u[1] / lu;
                let vx = v[0] / lv;
                let vy = v[1] / lv;
                let (sin_t, cos_t) = radians.sin_cos();
                let rx = ux * cos_t - uy * sin_t;
                let ry = ux * sin_t + uy * cos_t;
                let scale = lu.max(lv);
                (
                    [(rx * vy - ry * vx) * scale, (rx * vx + ry * vy - 1.0) * scale],
                    2,
                )
            }
            Constraint::PointToLineDistance {
                point,
                a,
                b,
                value,
            } => {
                let u = subtract(p(b), p(a));
                let v = subtract(p(point), p(a));
                let dist = (u[0] * v[1] - u[1] * v[0]).abs() / length(u).max(1e-12);
                ([dist - value, 0.0], 1)
            }
            Constraint::Midpoint { point, a, b } => {
                let pa = p(a);
                let pb = p(b);
                let pm = p(point);
                (
                    [
                        pm[0] - 0.5 * (pa[0] + pb[0]),
                        pm[1] - 0.5 * (pa[1] + pb[1]),
                    ],
                    2,
                )
            }
            Constraint::Symmetric {
                a,
                b,
                axis_a,
                axis_b,
            } => {
                let pa = p(a);
                let pb = p(b);
                let l0 = p(axis_a);
                let l1 = p(axis_b);
                let axis = subtract(l1, l0);
                let axis_len = length(axis).max(1e-12);
                let diff = subtract(pb, pa);
                let mid = [0.5 * (pa[0] + pb[0]) - l0[0], 0.5 * (pa[1] + pb[1]) - l0[1]];
                (
                    [
                        (diff[0] * axis[0] + diff[1] * axis[1]) / axis_len,
                        (axis[0] * mid[1] - axis[1] * mid[0]) / axis_len,
                    ],
                    2,
                )
            }
            Constraint::Collinear { a, b, c } => {
                let u = subtract(p(b), p(a));
                let v = subtract(p(c), p(a));
                ([(u[0] * v[1] - u[1] * v[0]) / length(u).max(1e-12), 0.0], 1)
            }
            Constraint::Radius { circle, value } => ([r(circle) - value, 0.0], 1),
            Constraint::Concentric { a, b } => (subtract(center(a), center(b)), 2),
            Constraint::EqualRadius { a, b } => ([r(a) - r(b), 0.0], 1),
            Constraint::PointOnCircle { point, circle } => (
                [length(subtract(p(point), center(circle))) - r(circle), 0.0],
                1,
            ),
            Constraint::TangentLineCircle { a, b, circle } => {
                let u = subtract(p(b), p(a));
                let v = subtract(center(circle), p(a));
                (
                    [
                        (u[0] * v[1] - u[1] * v[0]).abs() / length(u).max(1e-12) - r(circle),
                        0.0,
                    ],
                    1,
                )
            }
            Constraint::TangentCircles { a, b, internal } => (
                [
                    length(subtract(center(a), center(b)))
                        - if internal {
                            (r(a) - r(b)).abs()
                        } else {
                            r(a) + r(b)
                        },
                    0.0,
                ],
                1,
            ),
            Constraint::PointOnLine { point, a, b } => {
                let u = subtract(p(b), p(a));
                let v = subtract(p(point), p(a));
                ([(u[0] * v[1] - u[1] * v[0]) / length(u).max(1e-12), 0.0], 1)
            }
            Constraint::FixAngle { a, b, radians } => {
                let u = subtract(p(b), p(a));
                let (sin_t, cos_t) = radians.sin_cos();
                ([u[0] * sin_t - u[1] * cos_t, 0.0], 1)
            }
            Constraint::EqualAngle {
                a,
                b,
                c,
                d,
                e,
                f,
                g,
                h,
            } => {
                let u1 = subtract(p(b), p(a));
                let v1 = subtract(p(d), p(c));
                let u2 = subtract(p(f), p(e));
                let v2 = subtract(p(h), p(g));
                let l1 = (length(u1) * length(v1)).max(1e-12);
                let l2 = (length(u2) * length(v2)).max(1e-12);
                let cos1 = (u1[0] * v1[0] + u1[1] * v1[1]) / l1;
                let cos2 = (u2[0] * v2[0] + u2[1] * v2[1]) / l2;
                let scale = l1.sqrt().max(l2.sqrt());
                ([(cos1 - cos2) * scale, 0.0], 1)
            }
            Constraint::PointOnArc {
                point,
                circle,
                start_angle,
                end_angle,
            } => {
                let d = subtract(p(point), center(circle));
                let rad_res = length(d) - r(circle);
                let theta = d[1].atan2(d[0]);
                let span = (end_angle - start_angle).rem_euclid(std::f64::consts::TAU);
                let rel = (theta - start_angle).rem_euclid(std::f64::consts::TAU);
                let ang_res = if span <= 1e-12 || rel <= span {
                    0.0
                } else {
                    let over_end = rel - span;
                    let under_start = std::f64::consts::TAU - rel;
                    if over_end <= under_start {
                        over_end * r(circle)
                    } else {
                        -under_start * r(circle)
                    }
                };
                ([rad_res.hypot(ang_res), 0.0], 1)
            }
        }
    }

    pub(super) fn residual_into(&self, x: &[f64], out: &mut [f64]) {
        let mut offset = 0;
        for constraint in &self.constraints {
            let (vals, len) = self.eval_constraint_residual(constraint, x);
            out[offset..offset + len].copy_from_slice(&vals[..len]);
            offset += len;
        }
    }

    pub(super) fn residual(&self, x: &[f64]) -> Vec<Vec<f64>> {
        self.constraints
            .iter()
            .map(|constraint| {
                let (vals, len) = self.eval_constraint_residual(constraint, x);
                vals[..len].to_vec()
            })
            .collect()
    }

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
}
