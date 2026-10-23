use math_core::Result;
use crate::{Constraint, Sketch, error, length, subtract};

impl Sketch {
    pub(crate) fn validate(&self) -> Result<()> {
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

    pub(crate) fn constraint_residual_len(constraint: &Constraint) -> usize {
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

    pub(crate) fn eval_constraint_residual(&self, constraint: &Constraint, x: &[f64]) -> ([f64; 2], usize) {
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

    pub(crate) fn residual_into(&self, x: &[f64], out: &mut [f64]) {
        let mut offset = 0;
        for constraint in &self.constraints {
            let (vals, len) = self.eval_constraint_residual(constraint, x);
            out[offset..offset + len].copy_from_slice(&vals[..len]);
            offset += len;
        }
    }

    pub(crate) fn residual(&self, x: &[f64]) -> Vec<Vec<f64>> {
        self.constraints
            .iter()
            .map(|constraint| {
                let (vals, len) = self.eval_constraint_residual(constraint, x);
                vals[..len].to_vec()
            })
            .collect()
    }
}

pub(crate) fn constraint_active_vars(sketch: &Sketch, constraint: &Constraint) -> ([usize; 16], usize) {
    let mut vars = [0usize; 16];
    let mut len = 0usize;
    let add_pt = |vars: &mut [usize; 16], len: &mut usize, p: usize| {
        vars[*len] = 2 * p;
        vars[*len + 1] = 2 * p + 1;
        *len += 2;
    };
    let n_pts = sketch.points.len();
    let add_cir = |vars: &mut [usize; 16], len: &mut usize, c: usize| {
        let center = sketch.circles[c].center;
        vars[*len] = 2 * center;
        vars[*len + 1] = 2 * center + 1;
        vars[*len + 2] = 2 * n_pts + c;
        *len += 3;
    };
    match *constraint {
        Constraint::Fix { point, .. } => add_pt(&mut vars, &mut len, point),
        Constraint::Horizontal { a, b }
        | Constraint::Vertical { a, b }
        | Constraint::Coincident { a, b }
        | Constraint::Distance { a, b, .. }
        | Constraint::FixAngle { a, b, .. } => {
            add_pt(&mut vars, &mut len, a);
            add_pt(&mut vars, &mut len, b);
        }
        Constraint::Parallel { a, b, c, d }
        | Constraint::Perpendicular { a, b, c, d }
        | Constraint::EqualLength { a, b, c, d }
        | Constraint::Angle { a, b, c, d, .. } => {
            add_pt(&mut vars, &mut len, a);
            add_pt(&mut vars, &mut len, b);
            add_pt(&mut vars, &mut len, c);
            add_pt(&mut vars, &mut len, d);
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
            for p in [a, b, c, d, e, f, g, h] {
                add_pt(&mut vars, &mut len, p);
            }
        }
        Constraint::PointToLineDistance { point, a, b, .. }
        | Constraint::Midpoint { point, a, b }
        | Constraint::PointOnLine { point, a, b } => {
            add_pt(&mut vars, &mut len, point);
            add_pt(&mut vars, &mut len, a);
            add_pt(&mut vars, &mut len, b);
        }
        Constraint::Symmetric {
            a,
            b,
            axis_a,
            axis_b,
        } => {
            add_pt(&mut vars, &mut len, a);
            add_pt(&mut vars, &mut len, b);
            add_pt(&mut vars, &mut len, axis_a);
            add_pt(&mut vars, &mut len, axis_b);
        }
        Constraint::Collinear { a, b, c } => {
            add_pt(&mut vars, &mut len, a);
            add_pt(&mut vars, &mut len, b);
            add_pt(&mut vars, &mut len, c);
        }
        Constraint::Radius { circle, .. } => {
            vars[len] = 2 * n_pts + circle;
            len += 1;
        }
        Constraint::Concentric { a, b } => {
            add_pt(&mut vars, &mut len, sketch.circles[a].center);
            add_pt(&mut vars, &mut len, sketch.circles[b].center);
        }
        Constraint::EqualRadius { a, b } => {
            vars[len] = 2 * n_pts + a;
            vars[len + 1] = 2 * n_pts + b;
            len += 2;
        }
        Constraint::PointOnCircle { point, circle }
        | Constraint::PointOnArc { point, circle, .. } => {
            add_pt(&mut vars, &mut len, point);
            add_cir(&mut vars, &mut len, circle);
        }
        Constraint::TangentLineCircle { a, b, circle } => {
            add_pt(&mut vars, &mut len, a);
            add_pt(&mut vars, &mut len, b);
            add_cir(&mut vars, &mut len, circle);
        }
        Constraint::TangentCircles { a, b, .. } => {
            add_cir(&mut vars, &mut len, a);
            add_cir(&mut vars, &mut len, b);
        }
    }
    let slice = &mut vars[..len];
    slice.sort_unstable();
    let mut unique_len = 0;
    for i in 0..len {
        if i == 0 || vars[i] != vars[unique_len - 1] {
            vars[unique_len] = vars[i];
            unique_len += 1;
        }
    }
    (vars, unique_len)
}

pub(crate) fn jacobian_into(
    sketch: &Sketch,
    row_offsets: &[usize],
    scratch: &mut [f64],
    matrix: &mut [f64],
    n: usize,
) {
    matrix.fill(0.0);
    let pts_base = 2 * sketch.points.len();
    for (ci, constraint) in sketch.constraints.iter().enumerate() {
        let r0 = row_offsets[ci];
        let r0_idx = r0 * n;
        let r1_idx = (r0 + 1) * n;
        match *constraint {
            Constraint::Fix { point, .. } => {
                matrix[r0_idx + 2 * point] = 1.0;
                matrix[r1_idx + 2 * point + 1] = 1.0;
            }
            Constraint::Horizontal { a, b } => {
                let row0 = &mut matrix[r0_idx..r0_idx + n];
                row0[2 * a + 1] -= 1.0;
                row0[2 * b + 1] += 1.0;
            }
            Constraint::Vertical { a, b } => {
                let row0 = &mut matrix[r0_idx..r0_idx + n];
                row0[2 * a] -= 1.0;
                row0[2 * b] += 1.0;
            }
            Constraint::FixAngle { a, b, radians } => {
                let (sin_t, cos_t) = radians.sin_cos();
                let row0 = &mut matrix[r0_idx..r0_idx + n];
                row0[2 * a] -= sin_t;
                row0[2 * a + 1] += cos_t;
                row0[2 * b] += sin_t;
                row0[2 * b + 1] -= cos_t;
            }
            Constraint::PointOnLine { point, a, b } => {
                let pa = [scratch[2 * a], scratch[2 * a + 1]];
                let pb = [scratch[2 * b], scratch[2 * b + 1]];
                let pp = [scratch[2 * point], scratch[2 * point + 1]];
                let ux = pb[0] - pa[0];
                let uy = pb[1] - pa[1];
                let vx = pp[0] - pa[0];
                let vy = pp[1] - pa[1];
                let lu = ux.hypot(uy);
                if lu > 1e-12 {
                    let cross = ux * vy - uy * vx;
                    let inv_lu = 1.0 / lu;
                    let inv_lu3 = inv_lu * inv_lu * inv_lu;
                    let d_pbx = vy * inv_lu - cross * ux * inv_lu3;
                    let d_pby = -vx * inv_lu - cross * uy * inv_lu3;
                    let d_ppx = -uy * inv_lu;
                    let d_ppy = ux * inv_lu;
                    let row0 = &mut matrix[r0_idx..r0_idx + n];
                    row0[2 * b] += d_pbx;
                    row0[2 * b + 1] += d_pby;
                    row0[2 * point] += d_ppx;
                    row0[2 * point + 1] += d_ppy;
                    row0[2 * a] -= d_pbx + d_ppx;
                    row0[2 * a + 1] -= d_pby + d_ppy;
                }
            }
            Constraint::Coincident { a, b } => {
                matrix[r0_idx + 2 * a] += 1.0;
                matrix[r0_idx + 2 * b] -= 1.0;
                matrix[r1_idx + 2 * a + 1] += 1.0;
                matrix[r1_idx + 2 * b + 1] -= 1.0;
            }
            Constraint::Midpoint { point, a, b } => {
                matrix[r0_idx + 2 * point] += 1.0;
                matrix[r0_idx + 2 * a] -= 0.5;
                matrix[r0_idx + 2 * b] -= 0.5;
                matrix[r1_idx + 2 * point + 1] += 1.0;
                matrix[r1_idx + 2 * a + 1] -= 0.5;
                matrix[r1_idx + 2 * b + 1] -= 0.5;
            }
            Constraint::Radius { circle, .. } => {
                matrix[r0_idx + pts_base + circle] = 1.0;
            }
            Constraint::Concentric { a, b } => {
                let ca = sketch.circles[a].center;
                let cb = sketch.circles[b].center;
                matrix[r0_idx + 2 * ca] += 1.0;
                matrix[r0_idx + 2 * cb] -= 1.0;
                matrix[r1_idx + 2 * ca + 1] += 1.0;
                matrix[r1_idx + 2 * cb + 1] -= 1.0;
            }
            Constraint::EqualRadius { a, b } => {
                let row0 = &mut matrix[r0_idx..r0_idx + n];
                row0[pts_base + a] += 1.0;
                row0[pts_base + b] -= 1.0;
            }
            Constraint::Distance { a, b, .. } => {
                let pa = [scratch[2 * a], scratch[2 * a + 1]];
                let pb = [scratch[2 * b], scratch[2 * b + 1]];
                let dx = pa[0] - pb[0];
                let dy = pa[1] - pb[1];
                let len = dx.hypot(dy);
                if len > 1e-12 {
                    let gx = dx / len;
                    let gy = dy / len;
                    let row0 = &mut matrix[r0_idx..r0_idx + n];
                    row0[2 * a] += gx;
                    row0[2 * a + 1] += gy;
                    row0[2 * b] -= gx;
                    row0[2 * b + 1] -= gy;
                }
            }
            Constraint::EqualLength { a, b, c, d } => {
                let pa = [scratch[2 * a], scratch[2 * a + 1]];
                let pb = [scratch[2 * b], scratch[2 * b + 1]];
                let pc = [scratch[2 * c], scratch[2 * c + 1]];
                let pd = [scratch[2 * d], scratch[2 * d + 1]];
                let ux = pb[0] - pa[0];
                let uy = pb[1] - pa[1];
                let vx = pd[0] - pc[0];
                let vy = pd[1] - pc[1];
                let lu = ux.hypot(uy);
                let lv = vx.hypot(vy);
                let row0 = &mut matrix[r0_idx..r0_idx + n];
                if lu > 1e-12 {
                    let inv_lu = 1.0 / lu;
                    row0[2 * a] -= ux * inv_lu;
                    row0[2 * a + 1] -= uy * inv_lu;
                    row0[2 * b] += ux * inv_lu;
                    row0[2 * b + 1] += uy * inv_lu;
                }
                if lv > 1e-12 {
                    let inv_lv = 1.0 / lv;
                    row0[2 * c] += vx * inv_lv;
                    row0[2 * c + 1] += vy * inv_lv;
                    row0[2 * d] -= vx * inv_lv;
                    row0[2 * d + 1] -= vy * inv_lv;
                }
            }
            Constraint::PointOnCircle { point, circle } => {
                let pp = [scratch[2 * point], scratch[2 * point + 1]];
                let center_idx = sketch.circles[circle].center;
                let pc = [scratch[2 * center_idx], scratch[2 * center_idx + 1]];
                let dx = pp[0] - pc[0];
                let dy = pp[1] - pc[1];
                let len = dx.hypot(dy);
                let row0 = &mut matrix[r0_idx..r0_idx + n];
                if len > 1e-12 {
                    let gx = dx / len;
                    let gy = dy / len;
                    row0[2 * point] += gx;
                    row0[2 * point + 1] += gy;
                    row0[2 * center_idx] -= gx;
                    row0[2 * center_idx + 1] -= gy;
                }
                row0[pts_base + circle] -= 1.0;
            }
            Constraint::TangentCircles { a, b, internal } => {
                let ca = sketch.circles[a].center;
                let cb = sketch.circles[b].center;
                let pca = [scratch[2 * ca], scratch[2 * ca + 1]];
                let pcb = [scratch[2 * cb], scratch[2 * cb + 1]];
                let dx = pca[0] - pcb[0];
                let dy = pca[1] - pcb[1];
                let len = dx.hypot(dy);
                let row0 = &mut matrix[r0_idx..r0_idx + n];
                if len > 1e-12 {
                    let gx = dx / len;
                    let gy = dy / len;
                    row0[2 * ca] += gx;
                    row0[2 * ca + 1] += gy;
                    row0[2 * cb] -= gx;
                    row0[2 * cb + 1] -= gy;
                }
                if internal {
                    let ra = scratch[pts_base + a];
                    let rb = scratch[pts_base + b];
                    let s = if ra >= rb { 1.0 } else { -1.0 };
                    row0[pts_base + a] -= s;
                    row0[pts_base + b] += s;
                } else {
                    row0[pts_base + a] -= 1.0;
                    row0[pts_base + b] -= 1.0;
                }
            }
            _ => {
                let (active, active_len) = constraint_active_vars(sketch, constraint);
                for &j in &active[..active_len] {
                    let orig = scratch[j];
                    let h = (orig.abs() * 1e-7).max(1e-6);
                    scratch[j] = orig + h;
                    let (plus, len) = sketch.eval_constraint_residual(constraint, scratch);
                    scratch[j] = orig - h;
                    let (minus, _) = sketch.eval_constraint_residual(constraint, scratch);
                    scratch[j] = orig;
                    let inv_2h = 1.0 / (2.0 * h);
                    for k in 0..len {
                        matrix[(r0 + k) * n + j] = (plus[k] - minus[k]) * inv_2h;
                    }
                }
            }
        }
    }
}
