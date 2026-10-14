use super::*;

#[derive(Clone, Debug)]
pub struct Camera {
    pub rotation: M3,
    pub translation: V3,
    pub focal: f64,
    pub cx: f64,
    pub cy: f64,
}
impl Camera {
    pub fn identity(focal: f64, cx: f64, cy: f64) -> Self {
        Self {
            rotation: ID,
            translation: [0.; 3],
            focal,
            cx,
            cy,
        }
    }
    pub fn ray(&self, p: [f64; 2]) -> V3 {
        [
            (p[0] - self.cx) / self.focal,
            (p[1] - self.cy) / self.focal,
            1.,
        ]
    }
    /// World coordinates in the camera frame, shared by projection and Jacobians.
    pub fn camera_point(&self, point: V3) -> V3 {
        add(mv(self.rotation, point), self.translation)
    }
    /// Finite pinhole projection of a point already in the camera frame.
    pub fn project_camera_point(&self, point: V3) -> Option<[f64; 2]> {
        if !point.iter().all(|value| value.is_finite()) || point[2] <= 1e-8 {
            return None;
        }
        let pixel = [
            self.focal * point[0] / point[2] + self.cx,
            self.focal * point[1] / point[2] + self.cy,
        ];
        pixel.iter().all(|value| value.is_finite()).then_some(pixel)
    }
    pub fn project(&self, point: V3) -> Option<[f64; 2]> {
        self.project_camera_point(self.camera_point(point))
    }
    pub fn center(&self) -> V3 {
        scale(mv(tr(self.rotation), self.translation), -1.)
    }
}
pub fn triangulate(observations: &[(&Camera, [f64; 2])]) -> Option<V3> {
    let mut a = [[0.; 3]; 3];
    let mut b = [0.; 3];
    for &(c, p) in observations {
        let ray = c.ray(p);
        for k in 0..2 {
            let row: V3 = std::array::from_fn(|i| ray[k] * c.rotation[2][i] - c.rotation[k][i]);
            let rhs = c.translation[k] - ray[k] * c.translation[2];
            for i in 0..3 {
                b[i] += row[i] * rhs;
                for j in i..3 {
                    let v = row[i] * row[j];
                    a[i][j] += v;
                    if j > i {
                        a[j][i] += v;
                    }
                }
            }
        }
    }
    let v = solve(a, b)?;
    let p = [v[0], v[1], v[2]];
    if !p.iter().all(|x| x.is_finite()) || observations.iter().any(|(c, _)| c.project(p).is_none())
    {
        None
    } else {
        Some(p)
    }
}
pub(super) fn essential(pairs: &[(V3, V3)]) -> M3 {
    let normalize = |side: usize| {
        let mut center = [0.; 3];
        for p in pairs {
            center = add(center, if side == 0 { p.0 } else { p.1 });
        }
        center = scale(center, 1. / pairs.len() as f64);
        let mean = pairs
            .iter()
            .map(|p| {
                let v = if side == 0 { p.0 } else { p.1 };
                ((v[0] - center[0]).powi(2) + (v[1] - center[1]).powi(2)).sqrt()
            })
            .sum::<f64>()
            / pairs.len() as f64;
        let s = 2f64.sqrt() / mean.max(1e-10);
        [
            [s, 0., -s * center[0]],
            [0., s, -s * center[1]],
            [0., 0., 1.],
        ]
    };
    let ta = normalize(0);
    let tb = normalize(1);
    let rows = pairs.iter().map(|&(a, b)| {
        let a = mv(ta, a);
        let b = mv(tb, b);
        [
            b[0] * a[0],
            b[0] * a[1],
            b[0],
            b[1] * a[0],
            b[1] * a[1],
            b[1],
            a[0],
            a[1],
            1.,
        ]
    });
    let v = smallest(rows);
    let f = std::array::from_fn(|i| std::array::from_fn(|j| v[i * 3 + j]));
    mm(mm(tr(tb), f), ta)
}

pub(super) fn sampson(e: M3, et: M3, a: V3, b: V3) -> f64 {
    let ea = mv(e, a);
    let eb = mv(et, b);
    dot(b, ea).powi(2) / (ea[0] * ea[0] + ea[1] * ea[1] + eb[0] * eb[0] + eb[1] * eb[1]).max(1e-20)
}
pub struct Rng(u64);
impl Rng {
    pub fn new() -> Self {
        Self(0x123456789abcdef)
    }
    pub fn next(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n as u64) as usize
    }
    pub fn subset(&mut self, n: usize, k: usize) -> Vec<usize> {
        let mut r = Vec::with_capacity(k);
        while r.len() < k {
            let i = self.next(n);
            if !r.contains(&i) {
                r.push(i);
            }
        }
        r
    }
}

impl Default for Rng {
    fn default() -> Self {
        Self::new()
    }
}
/// Independent switches keep the frozen estimator available for measured comparisons.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeometryOptions {
    pub adaptive_ransac: bool,
    pub local_optimization: bool,
    pub epipolar_refinement: bool,
    pub enforce_essential: bool,
    pub pixel_sampson: bool,
    pub reject_degenerate: bool,
    /// Conservative initialization gate. Weak-consensus seeds may be correct but
    /// are declined; this ratio is support, not a probability or a metric accuracy.
    pub minimum_seed_inlier_ratio: f64,
    /// Opt-in joint two-view refinement (second camera plus inlier points, Schur
    /// elimination) after the alternating seed loop. Off by default, so the
    /// default output stays bit-identical with the frozen estimator.
    pub joint_refinement: bool,
    /// Opt-in analytic Jacobian in `refine_epipolar` instead of finite differences.
    pub analytic_epipolar_jacobian: bool,
}
impl Default for GeometryOptions {
    fn default() -> Self {
        Self::CONSENSUS
    }
}
impl GeometryOptions {
    pub fn validate(&self) -> crate::Result<()> {
        if !self.minimum_seed_inlier_ratio.is_finite()
            || !(0. ..=1.).contains(&self.minimum_seed_inlier_ratio)
        {
            return Err(crate::error(
                "Seed inlier ratio must be finite and between zero and one",
            ));
        }
        Ok(())
    }
    pub const BASELINE: Self = Self {
        adaptive_ransac: false,
        local_optimization: false,
        epipolar_refinement: false,
        enforce_essential: false,
        pixel_sampson: false,
        reject_degenerate: false,
        minimum_seed_inlier_ratio: 0.,
        joint_refinement: false,
        analytic_epipolar_jacobian: false,
    };
    pub const SAFE: Self = Self {
        pixel_sampson: true,
        reject_degenerate: true,
        minimum_seed_inlier_ratio: 0.5,
        ..Self::BASELINE
    };
    pub const ADAPTIVE: Self = Self {
        adaptive_ransac: true,
        ..Self::SAFE
    };
    pub const CONSENSUS: Self = Self {
        epipolar_refinement: false,
        ..Self::ROBUST
    };
    pub const PHYSICAL: Self = Self {
        enforce_essential: true,
        ..Self::CONSENSUS
    };
    /// Quality-first opt-in profile: CONSENSUS plus joint two-view refinement.
    pub const JOINT: Self = Self {
        joint_refinement: true,
        ..Self::CONSENSUS
    };
    pub const ROBUST: Self = Self {
        adaptive_ransac: true,
        local_optimization: true,
        epipolar_refinement: true,
        enforce_essential: false,
        pixel_sampson: true,
        reject_degenerate: true,
        minimum_seed_inlier_ratio: 0.5,
        joint_refinement: false,
        analytic_epipolar_jacobian: false,
    };
}
#[derive(Clone, Debug, Default)]
pub struct RobustReport {
    pub iterations: usize,
    pub hypotheses: usize,
    pub local_refits: usize,
    pub degenerate_samples: usize,
    pub rejected_degenerate: bool,
    /// Relative pose requires the first camera at the finite identity pose.
    pub invalid_first_camera: bool,
    pub insufficient_support: bool,
    pub support_ratio: f64,
    pub inliers: usize,
}
/// 99.9% sampling confidence, bounded by the caller's hard maximum.
/// This is a sampling budget, not a posterior probability that a pose is correct.
pub(super) fn adaptive_budget(inliers: usize, total: usize, sample: i32, maximum: usize) -> usize {
    let all_good = (inliers as f64 / total.max(1) as f64).powi(sample);
    if all_good >= 1. {
        return 1;
    }
    if all_good <= 0. {
        return maximum;
    }
    ((0.001f64.ln() / (-all_good).ln_1p()).ceil() as usize).clamp(1, maximum)
}
/// First-order epipolar distance in pixels, with each view's own focal scale.
pub(super) fn pixel_sampson(e: M3, et: M3, a: V3, b: V3, fa2: f64, fb2: f64) -> f64 {
    let ea = mv(e, a);
    let eb = mv(et, b);
    let denominator = (ea[0].powi(2) + ea[1].powi(2)) / fb2 + (eb[0].powi(2) + eb[1].powi(2)) / fa2;
    if denominator <= 1e-30 {
        return f64::INFINITY;
    }
    dot(b, ea).powi(2) / denominator
}
/// Refine the relative pose directly on symmetric epipolar residuals.
/// Alternating triangulation and one-camera PnP can have a small training error while
/// retaining a biased rotation, especially when the two focal scales differ greatly.
pub(super) fn refine_epipolar(camera: &mut Camera, first: &Camera, pairs: &[(V3, V3)], analytic: bool) {
    let essential_pose = |c: &Camera| {
        let r = mm(c.rotation, tr(first.rotation));
        let t = sub(c.translation, mv(r, first.translation));
        mm([[0., -t[2], t[1]], [t[2], 0., -t[0]], [-t[1], t[0], 0.]], r)
    };
    let skew = |v: V3| [[0., -v[2], v[1]], [v[2], 0., -v[0]], [-v[1], v[0], 0.]];
    let perturb = |c: &Camera, delta: &[f64]| {
        let mut next = c.clone();
        next.rotation = mm(rotation([delta[0], delta[1], delta[2]]), c.rotation);
        next.translation = unit(add(c.translation, [delta[3], delta[4], delta[5]]));
        next
    };
    let fa = first.focal;
    let fb = camera.focal;
    let fa2 = fa.powi(2);
    let fb2 = fb.powi(2);
    let residual = |e: M3, et: M3, x: V3, y: V3| {
        let ex = mv(e, x);
        let ey = mv(et, y);
        let denominator =
            (ex[0].powi(2) + ex[1].powi(2)) / fb2 + (ey[0].powi(2) + ey[1].powi(2)) / fa2;
        dot(y, ex) / denominator.max(1e-30).sqrt()
    };
    let cost = |c: &Camera| {
        let e = essential_pose(c);
        let et = tr(e);
        pairs
            .iter()
            .map(|&(x, y)| {
                let r = residual(e, et, x, y).abs();
                if r <= 2.5 { r * r } else { 5. * r - 6.25 }
            })
            .sum::<f64>()
    };
    let mut value = cost(camera);
    let mut damping = 1e-3;
    for _ in 0..40 {
        let e = essential_pose(camera);
        let et = tr(e);
        let eps = 1e-6;
        let perturbed: [(M3, M3); 6] = std::array::from_fn(|i| {
            let mut d = [0.; 6];
            d[i] = eps;
            let p = essential_pose(&perturb(camera, &d));
            (p, tr(p))
        });
        // Analytic derivatives of E = [t]x R for the six pose perturbations.
        // Rotation acts on the left; the translation derivative drops the radial
        // part because the perturb step renormalizes the baseline to unit length.
        let de: Option<[M3; 6]> = analytic.then(|| {
            let madd = |a: M3, b: M3| std::array::from_fn(|i| add(a[i], b[i]));
            let r = mm(camera.rotation, tr(first.rotation));
            let t = sub(camera.translation, mv(r, first.translation));
            std::array::from_fn(|i| {
                let mut axis = [0.; 3];
                axis[i % 3] = 1.;
                if i < 3 {
                    let dr = mm(skew(axis), r);
                    let dt = scale(mv(dr, first.translation), -1.);
                    madd(mm(skew(t), dr), mm(skew(dt), r))
                } else {
                    mm(
                        skew(sub(
                            axis,
                            scale(camera.translation, camera.translation[i % 3]),
                        )),
                        r,
                    )
                }
            })
        });
        let mut h = [[0.; 6]; 6];
        let mut g = [0.; 6];
        for &(x, y) in pairs {
            let r = residual(e, et, x, y);
            let weight = if r.abs() > 2.5 { 2.5 / r.abs() } else { 1. };
            let j: [f64; 6] = if let Some(de) = &de {
                let ex = mv(e, x);
                let ey = mv(et, y);
                let denominator = ((ex[0].powi(2) + ex[1].powi(2)) / fb2
                    + (ey[0].powi(2) + ey[1].powi(2)) / fa2)
                    .max(1e-30);
                let root = denominator.sqrt();
                let n = dot(y, ex);
                std::array::from_fn(|i| {
                    let dex = mv(de[i], x);
                    let dey = mv(tr(de[i]), y);
                    let dd = 2.
                        * ((ex[0] * dex[0] + ex[1] * dex[1]) / fb2
                            + (ey[0] * dey[0] + ey[1] * dey[1]) / fa2);
                    dot(y, dex) / root - n * dd / (2. * denominator * root)
                })
            } else {
                std::array::from_fn(|i| (residual(perturbed[i].0, perturbed[i].1, x, y) - r) / eps)
            };
            for i in 0..6 {
                g[i] -= weight * j[i] * r;
                for k in 0..6 {
                    h[i][k] += weight * j[i] * j[k];
                }
            }
        }
        for (i, row) in h.iter_mut().enumerate() {
            row[i] += damping * (row[i] + 1.);
        }
        let Some(delta) = solve(h, g) else {
            break;
        };
        let next = perturb(camera, &delta);
        let next_value = cost(&next);
        if next_value.is_finite() && next_value < value {
            *camera = next;
            let reduction = value - next_value;
            value = next_value;
            damping = (damping * 0.3).max(1e-9);
            if reduction < 1e-10 || delta.iter().map(|v| v * v).sum::<f64>() < 1e-16 {
                break;
            }
        } else {
            damping *= 10.;
            if damping > 1e12 {
                break;
            }
        }
    }
}
/// Opt-in joint refinement of the second camera and the triangulated inlier
/// points. Each LM step updates the pose and all points together, eliminating
/// the 3x3 point blocks (Schur complement), so the alternating
/// triangulate-and-resect loop cannot trade a biased rotation for a smaller
/// training error. The first camera is the fixed identity gauge; the baseline
/// norm is held at one by rescaling the points together with the translation,
/// which leaves every projection unchanged.
pub(super) fn refine_joint(first: &Camera, camera: &mut Camera, observations: &[([f64; 2], [f64; 2])]) {
    if observations.len() < 6 {
        return;
    }
    let mut points: Vec<V3> = Vec::with_capacity(observations.len());
    let mut kept: Vec<([f64; 2], [f64; 2])> = Vec::with_capacity(observations.len());
    for &(x, y) in observations {
        if let Some(p) = triangulate(&[(first, x), (camera, y)]) {
            points.push(p);
            kept.push((x, y));
        }
    }
    if points.len() < 6 {
        return;
    }
    let loss = |b: &Camera, points: &[V3]| {
        let mut total = 0.;
        for (&p, &(x, y)) in points.iter().zip(kept.iter()) {
            for (cam, q) in [(first, x), (b, y)] {
                total += cam.project(p).map_or(1e6, |uv| {
                    ((uv[0] - q[0]).powi(2) + (uv[1] - q[1]).powi(2)).min(100.)
                });
            }
        }
        total
    };
    let n = points.len();
    let mut v_blocks = vec![[[0.; 3]; 3]; n];
    let mut w_blocks = vec![[[0.; 6]; 3]; n];
    let mut g_points = vec![[0.; 3]; n];
    let mut lambda = 1e-4;
    let mut value = loss(camera, &points);
    for _ in 0..30 {
        let mut u = [[0.; 6]; 6];
        let mut g = [0.; 6];
        for i in 0..n {
            v_blocks[i] = [[0.; 3]; 3];
            w_blocks[i] = [[0.; 6]; 3];
            g_points[i] = [0.; 3];
        }
        for (i, (&p, &(x, y))) in points.iter().zip(kept.iter()).enumerate() {
            for (cam, q, second) in [(first, x, false), (camera, y, true)] {
                let pc = cam.camera_point(p);
                if pc[2] <= 1e-8 {
                    continue;
                }
                let uv = [
                    cam.focal * pc[0] / pc[2] + cam.cx,
                    cam.focal * pc[1] / pc[2] + cam.cy,
                ];
                let dp = [
                    [0., -pc[2], pc[1]],
                    [pc[2], 0., -pc[0]],
                    [-pc[1], pc[0], 0.],
                    [1., 0., 0.],
                    [0., 1., 0.],
                    [0., 0., 1.],
                ];
                for k in 0..2 {
                    let project = |d: V3| cam.focal * (d[k] * pc[2] - pc[k] * d[2]) / pc[2].powi(2);
                    let jc: [f64; 6] = std::array::from_fn(|i| project(dp[i]));
                    let jp: [f64; 3] = std::array::from_fn(|j| {
                        project([cam.rotation[0][j], cam.rotation[1][j], cam.rotation[2][j]])
                    });
                    let err = q[k] - uv[k];
                    let weight = if err.abs() > 3. { 3. / err.abs() } else { 1. };
                    for a in 0..3 {
                        g_points[i][a] += weight * jp[a] * err;
                        for b in a..3 {
                            let v = weight * jp[a] * jp[b];
                            v_blocks[i][a][b] += v;
                            if b > a {
                                v_blocks[i][b][a] += v;
                            }
                        }
                        if second {
                            for b in 0..6 {
                                w_blocks[i][a][b] += weight * jp[a] * jc[b];
                            }
                        }
                    }
                    if second {
                        for a in 0..6 {
                            g[a] += weight * jc[a] * err;
                            for b in a..6 {
                                let v = weight * jc[a] * jc[b];
                                u[a][b] += v;
                                if b > a {
                                    u[b][a] += v;
                                }
                            }
                        }
                    }
                }
            }
        }
        for (i, row) in u.iter_mut().enumerate() {
            row[i] += lambda * (row[i] + 1.);
        }
        for block in v_blocks.iter_mut() {
            for (i, row) in block.iter_mut().enumerate() {
                row[i] += lambda * (row[i] + 1.);
            }
        }
        // Schur complement of the point blocks: S = U - W V^-1 W^T.
        let mut s = u;
        let mut gs = g;
        let mut singular = false;
        for i in 0..n {
            let vinv_w: Option<[[f64; 6]; 3]> = (0..6)
                .map(|b| {
                    solve(
                        v_blocks[i],
                        [w_blocks[i][0][b], w_blocks[i][1][b], w_blocks[i][2][b]],
                    )
                })
                .collect::<Option<Vec<_>>>()
                .map(|cols| std::array::from_fn(|a| std::array::from_fn(|b| cols[b][a])));
            let (Some(vinv_w), Some(vinv_g)) = (vinv_w, solve(v_blocks[i], g_points[i])) else {
                singular = true;
                break;
            };
            for a in 0..6 {
                gs[a] -= (0..3).map(|k| w_blocks[i][k][a] * vinv_g[k]).sum::<f64>();
                for b in a..6 {
                    let v = (0..3)
                        .map(|k| w_blocks[i][k][a] * vinv_w[k][b])
                        .sum::<f64>();
                    s[a][b] -= v;
                    if b > a {
                        s[b][a] -= v;
                    }
                }
            }
        }
        if singular {
            break;
        }
        let Some(delta) = solve(s, gs) else {
            break;
        };
        let r = rotation([delta[0], delta[1], delta[2]]);
        let mut candidate = camera.clone();
        candidate.rotation = mm(r, camera.rotation);
        candidate.translation = add(mv(r, camera.translation), [delta[3], delta[4], delta[5]]);
        let mut candidate_points = points.clone();
        let mut failed = false;
        for i in 0..n {
            let coupling: V3 =
                std::array::from_fn(|a| (0..6).map(|b| w_blocks[i][a][b] * delta[b]).sum());
            let Some(step) = solve(v_blocks[i], sub(g_points[i], coupling)) else {
                failed = true;
                break;
            };
            candidate_points[i] = add(points[i], step);
        }
        if failed {
            break;
        }
        let length = norm(candidate.translation);
        if length > 1e-8 {
            candidate.translation = scale(candidate.translation, 1. / length);
            for p in &mut candidate_points {
                *p = scale(*p, 1. / length);
            }
        }
        let candidate_loss = loss(&candidate, &candidate_points);
        if candidate_loss < value {
            *camera = candidate;
            points = candidate_points;
            let reduction = value - candidate_loss;
            value = candidate_loss;
            lambda = (lambda * 0.3).max(1e-9);
            if reduction < 1e-10 || delta.iter().map(|v| v * v).sum::<f64>() < 1e-16 {
                break;
            }
        } else {
            lambda *= 10.;
            if lambda > 1e12 {
                break;
            }
        }
    }
}
