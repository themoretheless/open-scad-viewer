#![doc = include_str!("../README.md")]
//! Shared bounded 2D sketch solver. Independent of polygon and NURBS geometry.
#[cfg(feature = "codec")]
mod serialization;
pub use math_core::{Error, Result};
const INVALID_INPUT: &str = "SKETCH_INVALID_INPUT";
fn error(message: impl Into<String>) -> Error {
    Error::new(INVALID_INPUT, message)
}
#[derive(Clone, Debug)]
pub struct Circle {
    pub center: usize,
    pub radius: f64,
}

#[derive(Clone, Debug)]
pub enum Constraint {
    Fix {
        point: usize,
        at: [f64; 2],
    },
    Horizontal {
        a: usize,
        b: usize,
    },
    Vertical {
        a: usize,
        b: usize,
    },
    Coincident {
        a: usize,
        b: usize,
    },
    Distance {
        a: usize,
        b: usize,
        value: f64,
    },
    Parallel {
        a: usize,
        b: usize,
        c: usize,
        d: usize,
    },
    Perpendicular {
        a: usize,
        b: usize,
        c: usize,
        d: usize,
    },
    EqualLength {
        a: usize,
        b: usize,
        c: usize,
        d: usize,
    },
    Angle {
        a: usize,
        b: usize,
        c: usize,
        d: usize,
        radians: f64,
    },
    PointToLineDistance {
        point: usize,
        a: usize,
        b: usize,
        value: f64,
    },
    Midpoint {
        point: usize,
        a: usize,
        b: usize,
    },
    Symmetric {
        a: usize,
        b: usize,
        axis_a: usize,
        axis_b: usize,
    },
    Collinear {
        a: usize,
        b: usize,
        c: usize,
    },
    Radius {
        circle: usize,
        value: f64,
    },
    Concentric {
        a: usize,
        b: usize,
    },
    EqualRadius {
        a: usize,
        b: usize,
    },
    PointOnCircle {
        point: usize,
        circle: usize,
    },
    TangentLineCircle {
        a: usize,
        b: usize,
        circle: usize,
    },
    TangentCircles {
        a: usize,
        b: usize,
        internal: bool,
    },
}

#[derive(Clone, Debug)]
pub struct Sketch {
    pub points: Vec<[f64; 2]>,
    pub circles: Vec<Circle>,
    pub constraints: Vec<Constraint>,
}

#[derive(Clone, Debug)]
pub struct Solution {
    pub sketch: Sketch,
    pub status: &'static str,
    pub iterations: usize,
    pub max_residual: f64,
    pub residuals: Vec<Vec<f64>>,
    pub degrees_of_freedom: usize,
    pub convergence_certified: bool,
}

fn length(v: [f64; 2]) -> f64 {
    v[0].hypot(v[1])
}
fn subtract(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}
impl Sketch {
    fn validate(&self) -> Result<()> {
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
            };
            if !valid {
                return Err(error("Invalid sketch constraint reference/value"));
            }
        }
        Ok(())
    }
    fn constraint_residual_len(constraint: &Constraint) -> usize {
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

    fn eval_constraint_residual(&self, constraint: &Constraint, x: &[f64]) -> ([f64; 2], usize) {
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
        }
    }

    fn residual_into(&self, x: &[f64], out: &mut [f64]) {
        let mut offset = 0;
        for constraint in &self.constraints {
            let (vals, len) = self.eval_constraint_residual(constraint, x);
            out[offset..offset + len].copy_from_slice(&vals[..len]);
            offset += len;
        }
    }

    fn residual(&self, x: &[f64]) -> Vec<Vec<f64>> {
        self.constraints
            .iter()
            .map(|constraint| {
                let (vals, len) = self.eval_constraint_residual(constraint, x);
                vals[..len].to_vec()
            })
            .collect()
    }
}
fn solve_linear_in_place(a: &mut [f64], b: &mut [f64], x: &mut [f64], n: usize) -> bool {
    for i in 0..n {
        let pivot = (i..n)
            .max_by(|&j, &k| a[j * n + i].abs().total_cmp(&a[k * n + i].abs()))
            .unwrap();
        if a[pivot * n + i].abs() < 1e-18 {
            return false;
        }
        if i != pivot {
            for col in i..n {
                a.swap(i * n + col, pivot * n + col);
            }
            b.swap(i, pivot);
        }
        let inv_diag = 1.0 / a[i * n + i];
        for j in i + 1..n {
            let f = a[j * n + i] * inv_diag;
            if f != 0.0 {
                let (before, after) = a.split_at_mut(j * n);
                let pivot_row = &before[i * n + i..i * n + n];
                let target_row = &mut after[i..n];
                for (value, &pivot_val) in target_row.iter_mut().zip(pivot_row) {
                    *value -= f * pivot_val;
                }
                b[j] -= f * b[i];
            }
        }
    }
    for i in (0..n).rev() {
        let row = &a[i * n..i * n + n];
        let mut sum = 0.0;
        for j in i + 1..n {
            sum += row[j] * x[j];
        }
        x[i] = (b[i] - sum) / row[i];
    }
    true
}
fn constraint_active_vars(sketch: &Sketch, constraint: &Constraint) -> ([usize; 8], usize) {
    let mut vars = [0usize; 8];
    let mut len = 0usize;
    let add_pt = |vars: &mut [usize; 8], len: &mut usize, p: usize| {
        vars[*len] = 2 * p;
        vars[*len + 1] = 2 * p + 1;
        *len += 2;
    };
    let n_pts = sketch.points.len();
    let add_cir = |vars: &mut [usize; 8], len: &mut usize, c: usize| {
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
        | Constraint::Distance { a, b, .. } => {
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
        Constraint::PointToLineDistance { point, a, b, .. }
        | Constraint::Midpoint { point, a, b } => {
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
        Constraint::PointOnCircle { point, circle } => {
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

fn jacobian_into(
    sketch: &Sketch,
    row_offsets: &[usize],
    scratch: &mut [f64],
    matrix: &mut [f64],
    n: usize,
) {
    matrix.fill(0.0);
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
                matrix[r0_idx + 2 * a + 1] -= 1.0;
                matrix[r0_idx + 2 * b + 1] += 1.0;
            }
            Constraint::Vertical { a, b } => {
                matrix[r0_idx + 2 * a] -= 1.0;
                matrix[r0_idx + 2 * b] += 1.0;
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
                matrix[r0_idx + 2 * sketch.points.len() + circle] = 1.0;
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
                let base = 2 * sketch.points.len();
                matrix[r0_idx + base + a] += 1.0;
                matrix[r0_idx + base + b] -= 1.0;
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
                    matrix[r0_idx + 2 * a] += gx;
                    matrix[r0_idx + 2 * a + 1] += gy;
                    matrix[r0_idx + 2 * b] -= gx;
                    matrix[r0_idx + 2 * b + 1] -= gy;
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
                if lu > 1e-12 {
                    matrix[r0_idx + 2 * a] -= ux / lu;
                    matrix[r0_idx + 2 * a + 1] -= uy / lu;
                    matrix[r0_idx + 2 * b] += ux / lu;
                    matrix[r0_idx + 2 * b + 1] += uy / lu;
                }
                if lv > 1e-12 {
                    matrix[r0_idx + 2 * c] += vx / lv;
                    matrix[r0_idx + 2 * c + 1] += vy / lv;
                    matrix[r0_idx + 2 * d] -= vx / lv;
                    matrix[r0_idx + 2 * d + 1] -= vy / lv;
                }
            }
            Constraint::PointOnCircle { point, circle } => {
                let pp = [scratch[2 * point], scratch[2 * point + 1]];
                let center_idx = sketch.circles[circle].center;
                let pc = [scratch[2 * center_idx], scratch[2 * center_idx + 1]];
                let dx = pp[0] - pc[0];
                let dy = pp[1] - pc[1];
                let len = dx.hypot(dy);
                if len > 1e-12 {
                    let gx = dx / len;
                    let gy = dy / len;
                    matrix[r0_idx + 2 * point] += gx;
                    matrix[r0_idx + 2 * point + 1] += gy;
                    matrix[r0_idx + 2 * center_idx] -= gx;
                    matrix[r0_idx + 2 * center_idx + 1] -= gy;
                }
                matrix[r0_idx + 2 * sketch.points.len() + circle] -= 1.0;
            }
            Constraint::TangentCircles { a, b, internal } => {
                let ca = sketch.circles[a].center;
                let cb = sketch.circles[b].center;
                let pca = [scratch[2 * ca], scratch[2 * ca + 1]];
                let pcb = [scratch[2 * cb], scratch[2 * cb + 1]];
                let dx = pca[0] - pcb[0];
                let dy = pca[1] - pcb[1];
                let len = dx.hypot(dy);
                if len > 1e-12 {
                    let gx = dx / len;
                    let gy = dy / len;
                    matrix[r0_idx + 2 * ca] += gx;
                    matrix[r0_idx + 2 * ca + 1] += gy;
                    matrix[r0_idx + 2 * cb] -= gx;
                    matrix[r0_idx + 2 * cb + 1] -= gy;
                }
                let base = 2 * sketch.points.len();
                if internal {
                    let ra = scratch[base + a];
                    let rb = scratch[base + b];
                    let s = if ra >= rb { 1.0 } else { -1.0 };
                    matrix[r0_idx + base + a] -= s;
                    matrix[r0_idx + base + b] += s;
                } else {
                    matrix[r0_idx + base + a] -= 1.0;
                    matrix[r0_idx + base + b] -= 1.0;
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
fn rank_flat(a: &mut [f64], m: usize, n: usize) -> usize {
    let mut row = 0;
    for col in 0..n {
        if row >= m {
            break;
        }
        let pivot = (row..m)
            .max_by(|&i, &j| a[i * n + col].abs().total_cmp(&a[j * n + col].abs()))
            .unwrap();
        if a[pivot * n + col].abs() < 1e-7 {
            continue;
        }
        if row != pivot {
            for c in col..n {
                a.swap(row * n + c, pivot * n + c);
            }
        }
        let inv_diag = 1.0 / a[row * n + col];
        for i in row + 1..m {
            let f = a[i * n + col] * inv_diag;
            if f != 0.0 {
                let (before, after) = a.split_at_mut(i * n);
                let pivot_row = &before[row * n + col..row * n + n];
                let target_row = &mut after[col..n];
                for (value, &pivot_val) in target_row.iter_mut().zip(pivot_row) {
                    *value -= f * pivot_val;
                }
            }
        }
        row += 1;
    }
    row
}
/// Bounded numerical solver policy, shared by native and named adapters.
#[derive(Clone, Copy, Debug)]
pub struct SolverOptions {
    pub tolerance: f64,
    pub max_iterations: usize,
}
pub fn solve(sketch: &Sketch, tolerance: f64) -> Result<Solution> {
    solve_with_options(
        sketch,
        SolverOptions {
            tolerance,
            max_iterations: 96,
        },
    )
}
pub fn solve_with_options(sketch: &Sketch, options: SolverOptions) -> Result<Solution> {
    let SolverOptions {
        tolerance,
        max_iterations,
    } = options;
    if !(1..=96).contains(&max_iterations) {
        return Err(error("Invalid solver iteration limit"));
    }

    sketch.validate()?;
    if !tolerance.is_finite() || !(1e-8..=0.1).contains(&tolerance) {
        return Err(error("Invalid solver tolerance"));
    }
    let mut x: Vec<_> = sketch
        .points
        .iter()
        .flatten()
        .copied()
        .chain(sketch.circles.iter().map(|c| c.radius))
        .collect();
    let n = x.len();
    let mut row_offsets = Vec::with_capacity(sketch.constraints.len());
    let mut m = 0;
    for c in &sketch.constraints {
        row_offsets.push(m);
        m += Sketch::constraint_residual_len(c);
    }

    let mut damping = 1e-3;
    let mut iterations = 0;
    let mut residual = vec![0.0; m];
    let mut cand_residual = vec![0.0; m];
    let mut j_flat = vec![0.0; m * n];
    let mut scratch = x.clone();
    let mut a = vec![0.0; n * n];
    let mut b = vec![0.0; n];
    let mut delta = vec![0.0; n];
    let mut candidate = vec![0.0; n];

    sketch.residual_into(&x, &mut residual);
    let mut current_obj: f64 = residual.iter().map(|v| v * v).sum();

    for _ in 0..max_iterations {
        if residual.iter().all(|v| v.abs() <= tolerance) {
            break;
        }
        iterations += 1;
        scratch.copy_from_slice(&x);
        jacobian_into(sketch, &row_offsets, &mut scratch, &mut j_flat, n);
        a.fill(0.0);
        b.fill(0.0);
        for k in 0..m {
            let row = &j_flat[k * n..(k + 1) * n];
            let rk = residual[k];
            for i in 0..n {
                let row_i = row[i];
                if row_i != 0.0 {
                    b[i] -= row_i * rk;
                    let a_row = &mut a[i * n + i..i * n + n];
                    for (val, &row_l) in a_row.iter_mut().zip(&row[i..n]) {
                        *val += row_i * row_l;
                    }
                }
            }
        }
        for i in 0..n {
            for l in i + 1..n {
                a[l * n + i] = a[i * n + l];
            }
            a[i * n + i] += damping;
        }
        if !solve_linear_in_place(&mut a, &mut b, &mut delta, n) {
            damping *= 10.;
            continue;
        }
        for (cand, (&xv, &dv)) in candidate.iter_mut().zip(x.iter().zip(&delta)) {
            *cand = xv + dv;
        }
        let valid = candidate.iter().all(|v| v.is_finite() && v.abs() <= 1e6)
            && candidate[sketch.points.len() * 2..]
                .iter()
                .all(|r| *r > 1e-9);
        if valid {
            sketch.residual_into(&candidate, &mut cand_residual);
            let cand_obj: f64 = cand_residual.iter().map(|v| v * v).sum();
            if cand_obj < current_obj {
                x.copy_from_slice(&candidate);
                std::mem::swap(&mut residual, &mut cand_residual);
                current_obj = cand_obj;
                damping = (damping / 3.).max(1e-12);
                continue;
            }
        }
        damping = (damping * 10.).min(1e12);
    }
    let residuals = sketch.residual(&x);
    let max = residual.iter().map(|v| v.abs()).fold(0., f64::max);
    scratch.copy_from_slice(&x);
    jacobian_into(sketch, &row_offsets, &mut scratch, &mut j_flat, n);
    let dof = n - rank_flat(&mut j_flat, m, n);
    let mut result = sketch.clone();
    for (i, p) in result.points.iter_mut().enumerate() {
        *p = [x[2 * i], x[2 * i + 1]];
    }
    for (i, c) in result.circles.iter_mut().enumerate() {
        c.radius = x[sketch.points.len() * 2 + i];
    }
    let degenerate = has_degenerate_constraint(&result, tolerance);
    let status = if degenerate {
        "degenerate"
    } else if max > tolerance {
        "not_converged"
    } else if dof > 0 {
        "underconstrained"
    } else {
        "solved"
    };
    Ok(Solution {
        sketch: result,
        status,
        iterations,
        max_residual: max,
        residuals,
        degrees_of_freedom: dof,
        convergence_certified: false,
    })
}

fn is_constraint_degenerate(sketch: &Sketch, c: &Constraint, tolerance: f64) -> bool {
    match *c {
        Constraint::Parallel { a, b, c, d }
        | Constraint::Perpendicular { a, b, c, d }
        | Constraint::Angle { a, b, c, d, .. } => {
            length(subtract(sketch.points[a], sketch.points[b])) <= tolerance
                || length(subtract(sketch.points[c], sketch.points[d])) <= tolerance
        }
        Constraint::PointToLineDistance { a, b, .. }
        | Constraint::Collinear { a, b, .. }
        | Constraint::TangentLineCircle { a, b, .. } => {
            length(subtract(sketch.points[a], sketch.points[b])) <= tolerance
        }
        Constraint::Symmetric { axis_a, axis_b, .. } => {
            length(subtract(sketch.points[axis_a], sketch.points[axis_b])) <= tolerance
        }
        Constraint::TangentCircles {
            a,
            b,
            internal: true,
        } => (sketch.circles[a].radius - sketch.circles[b].radius).abs() <= tolerance,
        _ => false,
    }
}

fn has_degenerate_constraint(sketch: &Sketch, tolerance: f64) -> bool {
    sketch
        .constraints
        .iter()
        .any(|c| is_constraint_degenerate(sketch, c, tolerance))
}

fn degenerate_constraint_indices(sketch: &Sketch, tolerance: f64) -> Vec<usize> {
    sketch
        .constraints
        .iter()
        .enumerate()
        .filter_map(|(i, c)| is_constraint_degenerate(sketch, c, tolerance).then_some(i))
        .collect()
}

/// Constraint diagnostics for named sketch adapters, independent of transport.
#[derive(Clone, Debug)]
pub struct Diagnostics {
    pub redundant_equations: usize,
    pub degenerate_constraints: Vec<usize>,
    pub constraint_residuals: Vec<f64>,
    pub inconsistent: bool,
}
pub fn solve_with_diagnostics(sketch: &Sketch, tolerance: f64) -> Result<(Solution, Diagnostics)> {
    solve_with_diagnostics_options(
        sketch,
        SolverOptions {
            tolerance,
            max_iterations: 96,
        },
    )
}
pub fn solve_with_diagnostics_options(
    sketch: &Sketch,
    options: SolverOptions,
) -> Result<(Solution, Diagnostics)> {
    let tolerance = options.tolerance;
    let solution = solve_with_options(sketch, options)?;
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
        m += Sketch::constraint_residual_len(c);
    }
    let mut j_flat = vec![0.0; m * n];
    jacobian_into(solved, &row_offsets, &mut x, &mut j_flat, n);
    let mut j_rank_buf = j_flat.clone();
    let jacobian_rank = rank_flat(&mut j_rank_buf, m, n);
    let residual: Vec<_> = solution.residuals.iter().flatten().copied().collect();
    let linear = solved.constraints.iter().all(|c| {
        matches!(
            c,
            Constraint::Fix { .. }
                | Constraint::Horizontal { .. }
                | Constraint::Vertical { .. }
                | Constraint::Coincident { .. }
                | Constraint::Midpoint { .. }
                | Constraint::Concentric { .. }
                | Constraint::EqualRadius { .. }
        )
    });
    let inconsistent = if solution.max_residual > tolerance && linear {
        let mut aug = Vec::with_capacity(m * (n + 1));
        for (row, &r) in j_flat.chunks_exact(n).zip(&residual) {
            aug.extend_from_slice(row);
            aug.push(r);
        }
        rank_flat(&mut aug, m, n + 1) > jacobian_rank
    } else {
        false
    };
    let degenerate_constraints = degenerate_constraint_indices(solved, tolerance);
    let diagnostics = Diagnostics {
        redundant_equations: residual.len().saturating_sub(jacobian_rank),
        degenerate_constraints,
        constraint_residuals: solution
            .residuals
            .iter()
            .map(|r| r.iter().map(|x| x.abs()).fold(0., f64::max))
            .collect(),
        inconsistent,
    };
    Ok((solution, diagnostics))
}

/// Detailed degrees-of-freedom and constraint conflict report for UI highlighting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DofAnalysis {
    pub degrees_of_freedom: usize,
    pub conflicting_constraints: Vec<usize>,
    pub underconstrained_points: Vec<usize>,
}

/// Analyzes degrees of freedom, conflicting constraints, and underconstrained points
/// for a sketch at the given tolerance.
pub fn analyze_dof(sketch: &Sketch, tolerance: f64) -> Result<DofAnalysis> {
    let (solution, _) = solve_with_diagnostics(sketch, tolerance)?;
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
        m += Sketch::constraint_residual_len(c);
    }
    let mut j_flat = vec![0.0; m * n];
    jacobian_into(solved, &row_offsets, &mut x, &mut j_flat, n);

    // Gaussian elimination with column pivot tracking to identify constrained variables.
    let mut a = j_flat;
    let mut pivot_cols = vec![false; n];
    let mut row = 0;
    for col in 0..n {
        if row >= m {
            break;
        }
        let pivot = (row..m)
            .max_by(|&i, &j| a[i * n + col].abs().total_cmp(&a[j * n + col].abs()))
            .unwrap();
        if a[pivot * n + col].abs() < 1e-7 {
            continue;
        }
        if row != pivot {
            for c in col..n {
                a.swap(row * n + c, pivot * n + c);
            }
        }
        pivot_cols[col] = true;
        let inv_diag = 1.0 / a[row * n + col];
        for i in row + 1..m {
            let f = a[i * n + col] * inv_diag;
            if f != 0.0 {
                let (before, after) = a.split_at_mut(i * n);
                let pivot_row = &before[row * n + col..row * n + n];
                let target_row = &mut after[col..n];
                for (value, &pivot_val) in target_row.iter_mut().zip(pivot_row) {
                    *value -= f * pivot_val;
                }
            }
        }
        row += 1;
    }

    let mut underconstrained_points = Vec::new();
    for p in 0..solved.points.len() {
        if !pivot_cols[2 * p] || !pivot_cols[2 * p + 1] {
            underconstrained_points.push(p);
        }
    }

    let conflicting_constraints = solution
        .residuals
        .iter()
        .enumerate()
        .filter_map(|(ci, res)| {
            let max_r = res.iter().map(|v| v.abs()).fold(0.0, f64::max);
            (max_r > tolerance).then_some(ci)
        })
        .collect();

    Ok(DofAnalysis {
        degrees_of_freedom: solution.degrees_of_freedom,
        conflicting_constraints,
        underconstrained_points,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn solver_iteration_policy_is_bounded() {
        let sketch = Sketch {
            points: vec![[1., 1.]],
            circles: vec![],
            constraints: vec![Constraint::Fix {
                point: 0,
                at: [0., 0.],
            }],
        };
        for limit in [0, 97] {
            assert!(
                solve_with_options(
                    &sketch,
                    SolverOptions {
                        tolerance: 1e-6,
                        max_iterations: limit
                    }
                )
                .is_err()
            );
        }
        let result = solve_with_options(
            &sketch,
            SolverOptions {
                tolerance: 1e-6,
                max_iterations: 1,
            },
        )
        .unwrap();
        assert_eq!(result.iterations, 1);
        assert!(result.max_residual > 1e-6);
    }
    #[test]
    fn diagnostics_distinguish_linear_conflicts_and_redundancy() {
        let sketch = Sketch {
            points: vec![[0.2, -0.1], [1., 0.], [0., 1.]],
            circles: vec![],
            constraints: vec![
                Constraint::Fix {
                    point: 0,
                    at: [0., 0.],
                },
                Constraint::Fix {
                    point: 0,
                    at: [10., 0.],
                },
            ],
        };
        let (solution, report) = solve_with_diagnostics(&sketch, 1e-6).unwrap();
        assert!(report.inconsistent);
        assert_eq!(report.redundant_equations, 2);
        assert_eq!(report.constraint_residuals.len(), 2);
        assert!(solution.max_residual > 4.9);
        assert!(report.degenerate_constraints.is_empty());
        let sketch = Sketch {
            constraints: vec![],
            ..sketch
        };
        let (solution, report) = solve_with_diagnostics(&sketch, 0.1).unwrap();
        assert_eq!(solution.degrees_of_freedom, 6);
        assert!(!report.inconsistent);
    }
    #[test]
    fn line_circle_tangency() {
        let s = Sketch {
            points: vec![[0., 0.], [10., 0.], [3., 3.]],
            circles: vec![Circle {
                center: 2,
                radius: 1.,
            }],
            constraints: vec![
                Constraint::Fix {
                    point: 0,
                    at: [0., 0.],
                },
                Constraint::Fix {
                    point: 1,
                    at: [10., 0.],
                },
                Constraint::Vertical { a: 0, b: 2 },
                Constraint::Radius {
                    circle: 0,
                    value: 2.,
                },
                Constraint::TangentLineCircle {
                    a: 0,
                    b: 1,
                    circle: 0,
                },
            ],
        };
        let r = solve(&s, 1e-6).unwrap();
        assert_eq!(r.status, "solved");
        assert!((r.sketch.points[2][1] - 2.).abs() < 1e-5);
    }
    #[test]
    fn circle_circle_tangency_and_conflicts() {
        let s = Sketch {
            points: vec![[0., 0.], [4., 0.]],
            circles: vec![
                Circle {
                    center: 0,
                    radius: 1.,
                },
                Circle {
                    center: 1,
                    radius: 1.,
                },
            ],
            constraints: vec![
                Constraint::Fix {
                    point: 0,
                    at: [0., 0.],
                },
                Constraint::Horizontal { a: 0, b: 1 },
                Constraint::Radius {
                    circle: 0,
                    value: 2.,
                },
                Constraint::Radius {
                    circle: 1,
                    value: 1.,
                },
                Constraint::TangentCircles {
                    a: 0,
                    b: 1,
                    internal: false,
                },
            ],
        };
        let r = solve(&s, 1e-6).unwrap();
        assert_eq!(r.status, "solved");
        assert!((r.sketch.points[1][0] - 3.).abs() < 1e-5);
        let mut bad = s.clone();
        bad.constraints.push(Constraint::Radius {
            circle: 0,
            value: 7.,
        });
        assert_eq!(solve(&bad, 1e-6).unwrap().status, "not_converged");
    }
    #[test]
    fn reports_underconstraint() {
        let s = Sketch {
            points: vec![[0., 0.], [2., 0.]],
            circles: vec![],
            constraints: vec![Constraint::Distance {
                a: 0,
                b: 1,
                value: 1.,
            }],
        };
        assert_eq!(solve(&s, 1e-6).unwrap().status, "underconstrained");
    }
    #[test]
    fn angle_midpoint_symmetric_collinear_and_concentric_constraints() {
        let s = Sketch {
            points: vec![
                [0., 0.],
                [10., 0.],
                [4., 6.],
                [1., 1.],
                [-2., 4.],
                [0., 10.],
            ],
            circles: vec![
                Circle {
                    center: 0,
                    radius: 5.,
                },
                Circle {
                    center: 3,
                    radius: 2.,
                },
            ],
            constraints: vec![
                Constraint::Fix {
                    point: 0,
                    at: [0., 0.],
                },
                Constraint::Fix {
                    point: 1,
                    at: [10., 0.],
                },
                Constraint::Fix {
                    point: 5,
                    at: [0., 10.],
                },
                Constraint::Angle {
                    a: 0,
                    b: 1,
                    c: 0,
                    d: 2,
                    radians: std::f64::consts::FRAC_PI_4,
                },
                Constraint::Distance {
                    a: 0,
                    b: 2,
                    value: 10.0 * std::f64::consts::SQRT_2,
                },
                Constraint::Midpoint {
                    point: 3,
                    a: 0,
                    b: 1,
                },
                Constraint::Symmetric {
                    a: 2,
                    b: 4,
                    axis_a: 0,
                    axis_b: 5,
                },
                Constraint::Radius {
                    circle: 0,
                    value: 6.,
                },
                Constraint::EqualRadius { a: 0, b: 1 },
            ],
        };
        let r = solve(&s, 1e-6).unwrap();
        assert_eq!(r.status, "solved");
        assert!((r.sketch.points[2][0] - 10.0).abs() < 1e-5);
        assert!((r.sketch.points[2][1] - 10.0).abs() < 1e-5);
        assert!((r.sketch.points[3][0] - 5.0).abs() < 1e-6);
        assert!(r.sketch.points[3][1].abs() < 1e-6);
        assert!((r.sketch.points[4][0] + 10.0).abs() < 1e-5);
        assert!((r.sketch.points[4][1] - 10.0).abs() < 1e-5);
        assert!((r.sketch.circles[1].radius - 6.0).abs() < 1e-6);
    }
    #[test]
    fn solves_large_chain_exceeding_legacy_24_point_limit() {
        let count = 40;
        let mut points = Vec::with_capacity(count);
        let mut constraints = Vec::new();
        for i in 0..count {
            points.push([i as f64 * 0.9, (i % 3) as f64 * 0.2]);
        }
        constraints.push(Constraint::Fix {
            point: 0,
            at: [0., 0.],
        });
        for i in 0..count - 1 {
            constraints.push(Constraint::Horizontal { a: i, b: i + 1 });
            constraints.push(Constraint::Distance {
                a: i,
                b: i + 1,
                value: 1.0,
            });
        }
        let s = Sketch {
            points,
            circles: vec![],
            constraints,
        };
        let r = solve(&s, 1e-6).unwrap();
        assert_eq!(r.status, "solved");
        assert!((r.sketch.points[count - 1][0] - (count - 1) as f64).abs() < 1e-5);
    }
    #[test]
    fn analyze_dof_identifies_underconstrained_points_and_conflicts() {
        let s = Sketch {
            points: vec![[0., 0.], [5., 0.], [2., 3.]],
            circles: vec![],
            constraints: vec![
                Constraint::Fix {
                    point: 0,
                    at: [0., 0.],
                },
                Constraint::Fix {
                    point: 1,
                    at: [5., 0.],
                },
                Constraint::Horizontal { a: 0, b: 2 },
            ],
        };
        let dof = analyze_dof(&s, 1e-6).unwrap();
        assert_eq!(dof.degrees_of_freedom, 1);
        assert_eq!(dof.underconstrained_points, vec![2]);
        assert!(dof.conflicting_constraints.is_empty());

        let conflict = Sketch {
            points: vec![[0., 0.], [5., 0.]],
            circles: vec![],
            constraints: vec![
                Constraint::Fix {
                    point: 0,
                    at: [0., 0.],
                },
                Constraint::Fix {
                    point: 1,
                    at: [5., 0.],
                },
                Constraint::Distance {
                    a: 0,
                    b: 1,
                    value: 10.,
                },
            ],
        };
        let conflict_dof = analyze_dof(&conflict, 1e-6).unwrap();
        assert!(!conflict_dof.conflicting_constraints.is_empty());
    }
}
