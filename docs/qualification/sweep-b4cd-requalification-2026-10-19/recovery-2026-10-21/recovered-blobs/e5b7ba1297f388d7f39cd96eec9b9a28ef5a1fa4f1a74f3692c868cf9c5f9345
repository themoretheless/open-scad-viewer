use crate::math::*;
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
fn essential(pairs: &[(V3, V3)]) -> M3 {
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

fn sampson(e: M3, et: M3, a: V3, b: V3) -> f64 {
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
fn adaptive_budget(inliers: usize, total: usize, sample: i32, maximum: usize) -> usize {
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
fn pixel_sampson(e: M3, et: M3, a: V3, b: V3, fa2: f64, fb2: f64) -> f64 {
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
fn refine_epipolar(camera: &mut Camera, first: &Camera, pairs: &[(V3, V3)], analytic: bool) {
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
fn refine_joint(first: &Camera, camera: &mut Camera, observations: &[([f64; 2], [f64; 2])]) {
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
fn has_2d_extent(points: impl Iterator<Item = V3>) -> bool {
    let points: Vec<_> = points.collect();
    let n = points.len() as f64;
    let mx = points.iter().map(|p| p[0]).sum::<f64>() / n;
    let my = points.iter().map(|p| p[1]).sum::<f64>() / n;
    let (mut xx, mut xy, mut yy) = (0., 0., 0.);
    for p in points {
        let (x, y) = (p[0] - mx, p[1] - my);
        xx += x * x;
        xy += x * y;
        yy += y * y;
    }
    xx + yy > 1e-12 && xx * yy - xy * xy > 1e-5 * (xx + yy).powi(2)
}
/// A single homography is ambiguous for an unconstrained eight-point 3D seed.
/// This conservative check declines the seed; it does not claim the scene is planar.
fn homography_supported(pairs: &[(V3, V3)], fa: f64, fb: f64) -> bool {
    fn fit(pairs: &[(V3, V3)], reverse: bool) -> Option<M3> {
        let mut ata = [[0.; 8]; 8];
        let mut atb = [0.; 8];
        for &(mut a, mut b) in pairs {
            if reverse {
                std::mem::swap(&mut a, &mut b);
            }
            let rows = [
                (
                    [a[0], a[1], 1., 0., 0., 0., -b[0] * a[0], -b[0] * a[1]],
                    b[0],
                ),
                (
                    [0., 0., 0., a[0], a[1], 1., -b[1] * a[0], -b[1] * a[1]],
                    b[1],
                ),
            ];
            for (row, rhs) in rows {
                for i in 0..8 {
                    atb[i] += row[i] * rhs;
                    for j in i..8 {
                        let v = row[i] * row[j];
                        ata[i][j] += v;
                        if j > i {
                            ata[j][i] += v;
                        }
                    }
                }
            }
        }
        let h = solve(ata, atb)?;
        Some([[h[0], h[1], h[2]], [h[3], h[4], h[5]], [h[6], h[7], 1.]])
    }
    let (Some(ab), Some(ba)) = (fit(pairs, false), fit(pairs, true)) else {
        return false;
    };
    let error = |h, p: V3, q: V3, f: f64| {
        let v = mv(h, p);
        if v[2].abs() < 1e-12 {
            f64::INFINITY
        } else {
            ((v[0] / v[2] - q[0]).powi(2) + (v[1] / v[2] - q[1]).powi(2)) * f * f
        }
    };
    let good = pairs
        .iter()
        .filter(|&&(a, b)| error(ab, a, b, fb) < 6.25 && error(ba, b, a, fa) < 6.25)
        .count();
    good >= 12 && good * 10 >= pairs.len() * 9
}
/// Initialize a second view in the first camera's coordinate frame.
/// The first camera must have finite identity rotation and zero translation;
/// each entry may differ by at most 1e-12. Arbitrary first-camera poses are rejected.
/// Use `relative_with_options` for the rejection reason in `RobustReport`.
pub fn relative(
    a: &Camera,
    b: &Camera,
    pixels: &[([f64; 2], [f64; 2])],
) -> Option<(Camera, Vec<usize>)> {
    relative_with_options(
        a,
        b,
        pixels,
        &GeometryOptions::default(),
        &mut RobustReport::default(),
    )
}
/// Configurable relative pose with the same identity-first-camera contract as `relative`.
/// An invalid first pose returns `None` before sampling and sets `invalid_first_camera`.
pub fn relative_with_options(
    a: &Camera,
    b: &Camera,
    pixels: &[([f64; 2], [f64; 2])],
    options: &GeometryOptions,
    report: &mut RobustReport,
) -> Option<(Camera, Vec<usize>)> {
    *report = RobustReport::default();
    const IDENTITY_TOLERANCE: f64 = 1e-12;
    let identity_rotation = a.rotation.iter().enumerate().all(|(i, row)| {
        row.iter()
            .enumerate()
            .all(|(j, value)| value.is_finite() && (*value - ID[i][j]).abs() <= IDENTITY_TOLERANCE)
    });
    let zero_translation = a
        .translation
        .iter()
        .all(|value| value.is_finite() && value.abs() <= IDENTITY_TOLERANCE);
    if !identity_rotation || !zero_translation {
        report.invalid_first_camera = true;
        return None;
    }
    if pixels.len() < 12
        || options.validate().is_err()
        || ![a.focal, b.focal, a.cx, a.cy, b.cx, b.cy]
            .iter()
            .all(|v| v.is_finite())
        || a.focal <= 0.
        || b.focal <= 0.
    {
        return None;
    }
    let mut pairs = Vec::with_capacity(pixels.len());
    let mut finite = true;
    for &(x, y) in pixels {
        let pair = (a.ray(x), b.ray(y));
        finite &= x
            .iter()
            .chain(y.iter())
            .chain(pair.0.iter())
            .chain(pair.1.iter())
            .all(|v| v.is_finite());
        pairs.push(pair);
    }
    if !finite {
        return None;
    }
    let fit = |pairs: &[(V3, V3)]| {
        let e = essential(pairs);
        if !options.enforce_essential {
            return e;
        }
        let (u, s, v) = svd(e);
        let mean = (s[0] + s[1]) * 0.5;
        let constrained = std::array::from_fn(|i| [u[i][0] * mean, u[i][1] * mean, 0.]);
        mm(constrained, tr(v))
    };
    let threshold = (2.5 / a.focal.min(b.focal)).powi(2);
    let fa2 = a.focal.powi(2);
    let fb2 = b.focal.powi(2);
    // A partial cost can only grow, so a pass that reaches the cutoff can never win.
    let score = |e: M3, cutoff: f64, good: &mut Vec<usize>| {
        good.clear();
        let et = tr(e);
        let mut cost = 0.;
        for (i, &(x, y)) in pairs.iter().enumerate() {
            let error = if options.pixel_sampson {
                pixel_sampson(e, et, x, y, fa2, fb2) / 6.25
            } else {
                sampson(e, et, x, y) / threshold
            };
            if error < 1. {
                good.push(i);
            }
            cost += error.min(1.);
            if cost >= cutoff {
                break;
            }
        }
        cost
    };
    let mut rng = Rng::new();
    let mut best: Vec<usize> = Vec::with_capacity(pairs.len());
    let mut good: Vec<usize> = Vec::with_capacity(pairs.len());
    let mut scratch: Vec<usize> = Vec::with_capacity(pairs.len());
    let mut sample: Vec<(V3, V3)> = Vec::with_capacity(8);
    let mut consensus: Vec<(V3, V3)> = Vec::with_capacity(pairs.len());
    let mut best_cost = f64::INFINITY;
    let mut best_matrix = [[0.; 3]; 3];
    let maximum = if options.adaptive_ransac { 4096 } else { 768 };
    let mut budget = maximum;
    for iteration in 0..maximum {
        if iteration >= budget {
            break;
        }
        report.iterations += 1;
        sample.clear();
        sample.extend(rng.subset(pairs.len(), 8).iter().map(|&i| pairs[i]));
        if options.reject_degenerate
            && (!has_2d_extent(sample.iter().map(|p| p.0))
                || !has_2d_extent(sample.iter().map(|p| p.1)))
        {
            report.degenerate_samples += 1;
            continue;
        }
        let mut e = fit(&sample);
        report.hypotheses += 1;
        let cutoff = if options.local_optimization {
            best_cost
        } else {
            f64::INFINITY
        };
        let mut cost = score(e, cutoff, &mut good);
        if if options.local_optimization {
            cost < best_cost
        } else {
            good.len() > best.len()
        } {
            if options.local_optimization && good.len() >= 12 {
                for _ in 0..3 {
                    consensus.clear();
                    consensus.extend(good.iter().map(|&i| pairs[i]));
                    let next_matrix = fit(&consensus);
                    let next_cost = score(next_matrix, cost, &mut scratch);
                    report.local_refits += 1;
                    if scratch.len() < 12 || next_cost >= cost {
                        break;
                    }
                    std::mem::swap(&mut good, &mut scratch);
                    cost = next_cost;
                    e = next_matrix;
                }
            }
            std::mem::swap(&mut best, &mut good);
            best_cost = cost;
            best_matrix = e;
            if options.adaptive_ransac {
                budget = adaptive_budget(best.len(), pairs.len(), 8, maximum).max(48);
            }
        }
        if !options.adaptive_ransac && best.len() * 10 > pairs.len() * 9 {
            break;
        }
    }
    if best.len() < 12 {
        return None;
    }
    report.inliers = best.len();
    report.support_ratio = best.len() as f64 / pixels.len() as f64;
    if report.support_ratio < options.minimum_seed_inlier_ratio {
        report.insufficient_support = true;
        return None;
    }
    let good: Vec<_> = best.iter().map(|&i| pairs[i]).collect();
    if options.reject_degenerate && homography_supported(&good, a.focal, b.focal) {
        report.rejected_degenerate = true;
        return None;
    }
    let e = if options.local_optimization {
        best_matrix
    } else {
        fit(&good)
    };
    let (mut u, _, mut v) = svd(e);
    if det(u) < 0. {
        for row in &mut u {
            row[2] *= -1.;
        }
    }
    if det(v) < 0. {
        for row in &mut v {
            row[2] *= -1.;
        }
    }
    let w = [[0., -1., 0.], [1., 0., 0.], [0., 0., 1.]];
    let t = [u[0][2], u[1][2], u[2][2]];
    let a_center = a.center();
    let mut result = None;
    let mut max_good = 0;
    let mut accepted: Vec<usize> = Vec::with_capacity(best.len());
    let mut kept: Vec<usize> = Vec::new();
    for w in [w, tr(w)] {
        for sign in [-1., 1.] {
            let mut c = b.clone();
            c.rotation = mm(mm(u, w), tr(v));
            c.translation = scale(t, sign);
            let c_center = c.center();
            accepted.clear();
            accepted.extend(best.iter().copied().filter(|&i| {
                let (x, y) = pixels[i];
                let Some(p) = triangulate(&[(a, x), (&c, y)]) else {
                    return false;
                };
                let ra = unit(sub(p, a_center));
                let rb = unit(sub(p, c_center));
                let parallax = dot(ra, rb).clamp(-1., 1.).acos();
                let error = |cam: &Camera, q: [f64; 2]| {
                    cam.project(p).map_or(f64::INFINITY, |uv| {
                        (uv[0] - q[0]).powi(2) + (uv[1] - q[1]).powi(2)
                    })
                };
                parallax > 0.008 && error(a, x) < 2500. && error(&c, y) < 2500.
            }));
            if accepted.len() > max_good {
                max_good = accepted.len();
                result = Some(c);
                std::mem::swap(&mut accepted, &mut kept);
            }
        }
    }
    let mut c = result?;
    let initial = kept;
    let mut triangulated: Vec<(V3, [f64; 2])> = Vec::with_capacity(initial.len());
    let mut previous_points: Vec<(V3, [f64; 2])> = Vec::new();
    let mut previous_pose = c.clone();
    let mut have_previous = false;
    for _ in 0..25 {
        triangulated.clear();
        triangulated.extend(initial.iter().filter_map(|&i| {
            let (x, y) = pixels[i];
            triangulate(&[(a, x), (&c, y)]).map(|p| (p, y))
        }));
        // refine is a deterministic function of the pose and these points, so a
        // bitwise repeated state makes every further iteration a no-op.
        let same_pose = c
            .rotation
            .iter()
            .flatten()
            .chain(c.translation.iter())
            .map(|v| v.to_bits())
            .eq(previous_pose
                .rotation
                .iter()
                .flatten()
                .chain(previous_pose.translation.iter())
                .map(|v| v.to_bits()));
        let same_points = triangulated.len() == previous_points.len()
            && triangulated
                .iter()
                .zip(previous_points.iter())
                .all(|((p, q), (pp, pq))| {
                    p.map(|v| v.to_bits()) == pp.map(|v| v.to_bits())
                        && q.map(|v| v.to_bits()) == pq.map(|v| v.to_bits())
                });
        if have_previous && same_pose && same_points {
            break;
        }
        previous_pose = c.clone();
        previous_points.clear();
        previous_points.extend(triangulated.iter().copied());
        have_previous = true;
        refine(&mut c, &triangulated);
        let length = norm(c.translation);
        if length > 1e-8 {
            c.translation = scale(c.translation, 1. / length);
        }
    }
    if options.epipolar_refinement {
        let consensus = initial.iter().map(|&i| pairs[i]).collect::<Vec<_>>();
        refine_epipolar(&mut c, a, &consensus, options.analytic_epipolar_jacobian);
        report.local_refits += 1;
    }
    if options.joint_refinement {
        let observed = initial.iter().map(|&i| pixels[i]).collect::<Vec<_>>();
        refine_joint(a, &mut c, &observed);
        report.local_refits += 1;
    }
    let good: Vec<_> = initial
        .into_iter()
        .filter(|&i| {
            let (x, y) = pixels[i];
            triangulate(&[(a, x), (&c, y)]).is_some_and(|p| {
                [(a, x), (&c, y)].iter().all(|(cam, q)| {
                    cam.project(p)
                        .is_some_and(|uv| (uv[0] - q[0]).powi(2) + (uv[1] - q[1]).powi(2) < 16.)
                })
            })
        })
        .collect();
    report.inliers = good.len();
    report.support_ratio = good.len() as f64 / pixels.len() as f64;
    if report.support_ratio < options.minimum_seed_inlier_ratio {
        report.insufficient_support = true;
        return None;
    }
    (good.len() >= 12).then_some((c, good))
}
fn dlt(template: &Camera, pairs: &[(V3, [f64; 2])]) -> Option<Camera> {
    let center = scale(
        pairs.iter().fold([0.; 3], |s, (p, _)| add(s, *p)),
        1. / pairs.len() as f64,
    );
    let spread = (pairs
        .iter()
        .map(|(p, _)| dot(sub(*p, center), sub(*p, center)))
        .sum::<f64>()
        / pairs.len() as f64)
        .sqrt();
    if spread < 1e-8 {
        return None;
    }
    let rows = pairs.iter().flat_map(|&(point, pixel)| {
        let p = scale(sub(point, center), 1. / spread);
        let q = template.ray(pixel);
        let v = [p[0], p[1], p[2], 1.];
        let mut r = [0.; 12];
        let mut s = [0.; 12];
        for k in 0..4 {
            r[k] = v[k];
            r[8 + k] = -q[0] * v[k];
            s[4 + k] = v[k];
            s[8 + k] = -q[1] * v[k];
        }
        [r, s]
    });
    let v = smallest(rows);
    let m: M3 = std::array::from_fn(|i| std::array::from_fn(|j| v[i * 4 + j]));
    let sign = if det(m) < 0. { -1. } else { 1. };
    let sc = (norm(m[0]) + norm(m[1]) + norm(m[2])) / 3.;
    if sc < 1e-12 {
        return None;
    }
    let r0 = unit(scale(m[0], sign));
    let r1 = unit(sub(
        scale(m[1], sign),
        scale(r0, dot(scale(m[1], sign), r0)),
    ));
    let r = [r0, r1, cross(r0, r1)];
    let mut c = template.clone();
    c.rotation = r;
    c.translation = sub(
        scale([v[3], v[7], v[11]], sign * spread / sc),
        mv(r, center),
    );
    Some(c)
}
pub fn refine(c: &mut Camera, pairs: &[(V3, [f64; 2])]) {
    if pairs.len() < 6 {
        return;
    }
    let loss = |cam: &Camera| {
        pairs
            .iter()
            .map(|&(p, q)| {
                cam.project(p).map_or(1e6, |uv| {
                    ((uv[0] - q[0]).powi(2) + (uv[1] - q[1]).powi(2)).min(100.)
                })
            })
            .sum::<f64>()
    };
    let mut lambda = 1e-4;
    let mut value = loss(c);
    for _ in 0..20 {
        let mut a = [[0.; 6]; 6];
        let mut b = [0.; 6];
        for &(p, q) in pairs {
            let p = add(mv(c.rotation, p), c.translation);
            if p[2] <= 1e-8 {
                continue;
            }
            let uv = [c.focal * p[0] / p[2] + c.cx, c.focal * p[1] / p[2] + c.cy];
            let dp = [
                [0., -p[2], p[1]],
                [p[2], 0., -p[0]],
                [-p[1], p[0], 0.],
                [1., 0., 0.],
                [0., 1., 0.],
                [0., 0., 1.],
            ];
            for k in 0..2 {
                let j: [f64; 6] = std::array::from_fn(|i| {
                    let d = dp[i];
                    c.focal * (d[k] * p[2] - p[k] * d[2]) / (p[2] * p[2])
                });
                let err = q[k] - uv[k];
                let weight = if err.abs() > 3. { 3. / err.abs() } else { 1. };
                let wj: [f64; 6] = std::array::from_fn(|i| weight * j[i]);
                for x in 0..6 {
                    b[x] += wj[x] * err;
                    for y in x..6 {
                        a[x][y] += wj[x] * j[y];
                        if y > x {
                            a[y][x] += wj[y] * j[x];
                        }
                    }
                }
            }
        }
        for (i, row) in a.iter_mut().enumerate() {
            row[i] += lambda * (row[i] + 1.);
        }
        let Some(delta) = solve(a, b) else {
            break;
        };
        let r = rotation([delta[0], delta[1], delta[2]]);
        let mut candidate = c.clone();
        candidate.rotation = mm(r, c.rotation);
        candidate.translation = add(mv(r, c.translation), [delta[3], delta[4], delta[5]]);
        let candidate_loss = loss(&candidate);
        if candidate_loss < value {
            *c = candidate;
            value = candidate_loss;
            lambda = (lambda * 0.3).max(1e-9);
        } else {
            lambda *= 10.;
        }
    }
}
pub fn pnp(template: &Camera, pairs: &[(V3, [f64; 2])]) -> Option<Camera> {
    pnp_with_options(
        template,
        pairs,
        &GeometryOptions::default(),
        &mut RobustReport::default(),
    )
}
pub fn pnp_with_options(
    template: &Camera,
    pairs: &[(V3, [f64; 2])],
    options: &GeometryOptions,
    report: &mut RobustReport,
) -> Option<Camera> {
    *report = RobustReport::default();
    if pairs.len() < 8
        || ![template.focal, template.cx, template.cy]
            .iter()
            .all(|v| v.is_finite())
        || template.focal <= 0.
        || pairs
            .iter()
            .any(|(p, q)| p.iter().chain(q).any(|v| !v.is_finite()))
    {
        return None;
    }
    let mut rng = Rng::new();
    let mut guess = template.clone();
    refine(&mut guess, pairs);
    let mut best: Vec<_> = pairs
        .iter()
        .copied()
        .filter(|&(p, q)| {
            guess
                .project(p)
                .is_some_and(|uv| (uv[0] - q[0]).powi(2) + (uv[1] - q[1]).powi(2) < 16.)
        })
        .collect();
    let mut result = Some(guess);
    let maximum = if options.adaptive_ransac { 384 } else { 192 };
    let mut budget = maximum;
    let mut sample: Vec<(V3, [f64; 2])> = Vec::with_capacity(6);
    let mut good: Vec<(V3, [f64; 2])> = Vec::with_capacity(pairs.len());
    let mut accepted: Vec<(V3, [f64; 2])> = Vec::with_capacity(pairs.len());
    for iteration in 0..maximum {
        if iteration >= budget {
            break;
        }
        report.iterations += 1;
        sample.clear();
        sample.extend(rng.subset(pairs.len(), 6).iter().map(|&i| pairs[i]));
        if options.reject_degenerate {
            let mean = scale(
                sample.iter().fold([0.; 3], |sum, (p, _)| add(sum, *p)),
                1. / sample.len() as f64,
            );
            let mut covariance = [[0.; 3]; 3];
            for (point, _) in &sample {
                let d = sub(*point, mean);
                for i in 0..3 {
                    for j in 0..3 {
                        covariance[i][j] += d[i] * d[j];
                    }
                }
            }
            let trace = covariance[0][0] + covariance[1][1] + covariance[2][2];
            if trace < 1e-12 || det(covariance) < 1e-8 * trace.powi(3) {
                report.degenerate_samples += 1;
                continue;
            }
        }
        let Some(mut c) = dlt(template, &sample) else {
            continue;
        };
        report.hypotheses += 1;
        refine(&mut c, &sample);
        good.clear();
        good.extend(pairs.iter().copied().filter(|&(p, q)| {
            c.project(p)
                .is_some_and(|uv| (uv[0] - q[0]).powi(2) + (uv[1] - q[1]).powi(2) < 16.)
        }));
        if good.len() > best.len() {
            if options.local_optimization {
                for _ in 0..2 {
                    let mut next = c.clone();
                    refine(&mut next, &good);
                    report.local_refits += 1;
                    accepted.clear();
                    accepted.extend(pairs.iter().copied().filter(|&(p, q)| {
                        next.project(p)
                            .is_some_and(|uv| (uv[0] - q[0]).powi(2) + (uv[1] - q[1]).powi(2) < 16.)
                    }));
                    if accepted.len() < good.len() {
                        break;
                    }
                    c = next;
                    std::mem::swap(&mut good, &mut accepted);
                }
            }
            std::mem::swap(&mut best, &mut good);
            result = Some(c);
            if options.adaptive_ransac {
                budget = adaptive_budget(best.len(), pairs.len(), 6, maximum).max(24);
            }
        }
        if !options.adaptive_ransac && best.len() * 10 > pairs.len() * 9 {
            break;
        }
    }
    if best.len() < 8.max(pairs.len() / 5) {
        return None;
    }
    let mut c = result?;
    refine(&mut c, &best);
    if options.local_optimization {
        let accepted = pairs
            .iter()
            .filter(|&&(p, q)| {
                c.project(p)
                    .is_some_and(|uv| (uv[0] - q[0]).powi(2) + (uv[1] - q[1]).powi(2) < 16.)
            })
            .count();
        if accepted < 8.max(pairs.len() / 5) {
            return None;
        }
        report.inliers = accepted;
    } else {
        report.inliers = best.len();
    }
    Some(c)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[expect(clippy::type_complexity, reason = "compact deterministic camera fixture")]
    fn scene() -> (Camera, Camera, Vec<(V3, [f64; 2], [f64; 2])>) {
        let a = Camera::identity(600., 320., 240.);
        let mut b = a.clone();
        b.rotation = rotation([0.03, 0.08, -0.01]);
        b.translation = [-0.8, 0.03, 0.04];
        let mut rng = Rng::new();
        let p = (0..80)
            .map(|_| {
                let p = [
                    rng.next(1000) as f64 / 400. - 1.2,
                    rng.next(1000) as f64 / 500. - 1.,
                    3. + rng.next(1000) as f64 / 300.,
                ];
                (p, a.project(p).unwrap(), b.project(p).unwrap())
            })
            .collect();
        (a, b, p)
    }
    #[test]
    fn shared_projection_rejects_nonfinite_coordinates_and_intrinsics() {
        let camera = Camera::identity(700., 320., 240.);
        let point = [0.5, -0.25, 4.];
        assert_eq!(
            camera.project(point),
            camera.project_camera_point(camera.camera_point(point))
        );
        for invalid in [[f64::NAN, 0., 1.], [0., f64::INFINITY, 1.], [0., 0., -1.]] {
            assert!(camera.project(invalid).is_none());
            assert!(camera.project_camera_point(invalid).is_none());
        }
        let mut invalid = camera;
        invalid.focal = f64::INFINITY;
        assert!(invalid.project(point).is_none());
    }

    #[test]
    fn triangulation_recovers_depth() {
        let (a, b, points) = scene();
        for (p, x, y) in points {
            let q = triangulate(&[(&a, x), (&b, y)]).unwrap();
            assert!(norm(sub(p, q)) < 1e-8);
        }
    }
    #[test]
    fn robust_pose_with_outliers() {
        let (a, b, points) = scene();
        let mut pairs: Vec<_> = points.iter().map(|&(_, x, y)| (x, y)).collect();
        for (i, p) in pairs.iter_mut().take(15).enumerate() {
            p.1 = [(i * 31 % 600) as f64, (i * 53 % 400) as f64];
        }
        let (c, good) = relative(&a, &b, &pairs).expect("relative pose");
        assert!(good.len() > 55, "{}", good.len());
        assert!(norm(sub(unit(c.translation), unit(b.translation))) < 0.04);
        for i in 0..3 {
            assert!(norm(sub(c.rotation[i], b.rotation[i])) < 0.03);
        }
    }
    #[test]
    fn resection_recovers_camera() {
        let (_, b, points) = scene();
        let pairs: Vec<_> = points.iter().map(|&(p, _, y)| (p, y)).collect();
        let c = pnp(&Camera::identity(600., 320., 240.), &pairs).unwrap();
        assert!(norm(sub(c.translation, b.translation)) < 1e-3);
    }
    #[test]
    fn mixed_focal_sampson_has_pixel_units_and_is_symmetric() {
        let a = Camera::identity(280., 320., 240.);
        let b = Camera::identity(1500., 320., 240.);
        let r = rotation([0.03, 0.08, -0.01]);
        let t = [-0.8, 0.03, 0.04];
        let e = mm([[0., -t[2], t[1]], [t[2], 0., -t[0]], [-t[1], t[0], 0.]], r);
        let (x, y) = ([411.4, 301.2], [124.7, 516.3]);
        let constraint = |x, y| dot(b.ray(y), mv(e, a.ray(x)));
        let epsilon = 1e-4;
        let mut gradient_norm = 0.;
        for side in 0..2 {
            for k in 0..2 {
                let (mut xx, mut yy) = (x, y);
                if side == 0 {
                    xx[k] += epsilon;
                } else {
                    yy[k] += epsilon;
                }
                gradient_norm += ((constraint(xx, yy) - constraint(x, y)) / epsilon).powi(2);
            }
        }
        let measured = pixel_sampson(
            e,
            tr(e),
            a.ray(x),
            b.ray(y),
            a.focal.powi(2),
            b.focal.powi(2),
        );
        let numerical = constraint(x, y).powi(2) / gradient_norm;
        assert!((measured / numerical - 1.).abs() < 1e-6);
        let reversed = pixel_sampson(
            tr(e),
            e,
            b.ray(y),
            a.ray(x),
            b.focal.powi(2),
            a.focal.powi(2),
        );
        assert!((measured - reversed).abs() < 1e-10);
    }
    #[test]
    fn robust_estimators_reject_nonfinite_correspondences_without_panics() {
        let camera = Camera::identity(600., 320., 240.);
        let pixels = vec![([f64::NAN, 0.], [0., 0.]); 20];
        let mut report = RobustReport::default();
        assert!(
            relative_with_options(
                &camera,
                &camera,
                &pixels,
                &GeometryOptions::ROBUST,
                &mut report
            )
            .is_none()
        );
        assert_eq!(report.iterations, 0);
        let points = vec![([0., 0., 4.], [f64::INFINITY, 0.]); 20];
        assert!(
            pnp_with_options(&camera, &points, &GeometryOptions::ROBUST, &mut report).is_none()
        );
        assert_eq!(report.iterations, 0);
    }
    #[test]
    fn low_support_does_not_become_a_successful_seed() {
        let (a, b, points) = scene();
        let mut pixels = points.iter().map(|&(_, x, y)| (x, y)).collect::<Vec<_>>();
        for (i, pair) in pixels.iter_mut().enumerate().take(48) {
            pair.1 = [(i * 79 % 640) as f64, (i * 107 % 480) as f64];
        }
        let mut report = RobustReport::default();
        let pose = relative_with_options(&a, &b, &pixels, &GeometryOptions::default(), &mut report);
        assert!(pose.is_none());
        assert!(report.insufficient_support || report.inliers < 12);
    }
    fn mixed_pose_holdout_error(options: GeometryOptions, outliers: usize) -> f64 {
        let (_, mut b, points) = scene();
        let a = Camera::identity(280., 320., 240.);
        b.focal = 1500.;
        let pixels = points[..60]
            .iter()
            .enumerate()
            .map(|(i, (p, _, _))| {
                let x = a.project(*p).unwrap();
                let mut y = b.project(*p).unwrap();
                if i < outliers {
                    y = [(i * 79 % 640) as f64, (i * 107 % 480) as f64];
                }
                (x, y)
            })
            .collect::<Vec<_>>();
        let (pose, good) =
            relative_with_options(&a, &b, &pixels, &options, &mut RobustReport::default())
                .expect("a supported pose");
        assert!(good.len() >= 45);
        let error = points[60..]
            .iter()
            .map(|(p, _, _)| {
                let projected = pose.project(scale(*p, 1. / norm(b.translation))).unwrap();
                let expected = b.project(*p).unwrap();
                (projected[0] - expected[0]).powi(2) + (projected[1] - expected[1]).powi(2)
            })
            .sum::<f64>();
        (error / 20.).sqrt()
    }
    #[test]
    fn clean_mixed_focal_pose_predicts_unused_world_points() {
        assert!(mixed_pose_holdout_error(GeometryOptions::default(), 0) < 0.1);
    }
    #[test]
    fn epipolar_outlier_does_not_regress_against_frozen_estimator() {
        // One accidental epipolar-consistent outlier passes both estimators. This
        // is a comparative regression check, not an assertion of metric accuracy.
        let baseline = mixed_pose_holdout_error(GeometryOptions::BASELINE, 12);
        let current = mixed_pose_holdout_error(GeometryOptions::default(), 12);
        assert!(
            current <= baseline + 1e-6,
            "baseline {baseline}, current {current}"
        );
    }
    #[test]
    fn relative_pose_rejects_nonidentity_first_camera_before_sampling() {
        let (a, b, points) = scene();
        for translated in [false, true] {
            let mut first = a.clone();
            if translated {
                first.translation = [0.2, 0., 0.];
            } else {
                first.rotation = rotation([0., 0.1, 0.]);
            }
            let pixels = points
                .iter()
                .map(|(p, _, y)| (first.project(*p).unwrap(), *y))
                .collect::<Vec<_>>();
            let mut report = RobustReport::default();
            assert!(
                relative_with_options(
                    &first,
                    &b,
                    &pixels,
                    &GeometryOptions::default(),
                    &mut report
                )
                .is_none()
            );
            assert!(report.invalid_first_camera);
            assert_eq!(report.iterations, 0);
            assert_eq!(report.hypotheses, 0);
        }
    }
    #[test]
    fn relative_pose_identity_tolerance_is_finite_and_accepts_roundoff() {
        let (mut a, b, points) = scene();
        let pixels = points.iter().map(|&(_, x, y)| (x, y)).collect::<Vec<_>>();
        a.rotation[0][0] += 5e-13;
        a.translation[0] = 5e-13;
        let mut report = RobustReport::default();
        assert!(
            relative_with_options(&a, &b, &pixels, &GeometryOptions::default(), &mut report)
                .is_some()
        );
        assert!(!report.invalid_first_camera);
        for (rotation_value, translation_value) in [
            (f64::NAN, 0.),
            (1., f64::INFINITY),
            (1. + 2e-12, 0.),
            (1., 2e-12),
        ] {
            a.rotation = ID;
            a.rotation[0][0] = rotation_value;
            a.translation = [translation_value, 0., 0.];
            assert!(
                relative_with_options(&a, &b, &pixels, &GeometryOptions::default(), &mut report)
                    .is_none()
            );
            assert!(report.invalid_first_camera);
            assert_eq!(report.iterations, 0);
        }
    }
    /// Noisy non-planar synthetic scene with a known relative pose. Uniform
    /// pixel noise plus a fixed share of scrambled matches, all from the
    /// deterministic kernel Rng.
    #[expect(clippy::type_complexity, reason = "compact deterministic camera fixture")]
    fn noisy_scene(
        fa: f64,
        fb: f64,
        noise: f64,
        outliers: usize,
    ) -> (
        Camera,
        Camera,
        Vec<([f64; 2], [f64; 2])>,
        Vec<(V3, [f64; 2])>,
    ) {
        let a = Camera::identity(fa, 320., 240.);
        let mut b = Camera::identity(fb, 320., 240.);
        b.rotation = rotation([0.05, 0.12, -0.02]);
        b.translation = [-0.8, 0.05, 0.1];
        let mut rng = Rng::new();
        let mut train = Vec::new();
        for i in 0..240 {
            let p = [
                rng.next(1000) as f64 / 250. - 2.,
                rng.next(1000) as f64 / 300. - 1.6,
                2.5 + rng.next(1000) as f64 / 250.,
            ];
            let mut x = a.project(p).unwrap();
            let mut y = b.project(p).unwrap();
            x[0] += (rng.next(2000) as f64 / 1000. - 1.) * noise;
            x[1] += (rng.next(2000) as f64 / 1000. - 1.) * noise;
            y[0] += (rng.next(2000) as f64 / 1000. - 1.) * noise;
            y[1] += (rng.next(2000) as f64 / 1000. - 1.) * noise;
            if i < outliers {
                y = [(i * 79 % 640) as f64, (i * 137 % 480) as f64];
            }
            train.push((x, y));
        }
        let holdout = (0..60)
            .map(|_| {
                let p = [
                    rng.next(1000) as f64 / 250. - 2.,
                    rng.next(1000) as f64 / 300. - 1.6,
                    2.5 + rng.next(1000) as f64 / 250.,
                ];
                (p, b.project(p).unwrap())
            })
            .collect();
        (a, b, train, holdout)
    }
    /// Reprojection RMSE on independent points at the known baseline scale.
    fn holdout_rmse(pose: &Camera, b: &Camera, holdout: &[(V3, [f64; 2])]) -> f64 {
        let baseline_scale = 1. / norm(b.translation);
        (holdout
            .iter()
            .map(|(p, y)| {
                let uv = pose.project(scale(*p, baseline_scale)).unwrap();
                (uv[0] - y[0]).powi(2) + (uv[1] - y[1]).powi(2)
            })
            .sum::<f64>()
            / holdout.len() as f64)
            .sqrt()
    }
    fn rotation_error_deg(pose: &Camera, b: &Camera) -> f64 {
        let mut trace = 0.;
        for i in 0..3 {
            for j in 0..3 {
                trace += pose.rotation[i][j] * b.rotation[i][j];
            }
        }
        ((trace - 1.) / 2.).clamp(-1., 1.).acos().to_degrees()
    }
    #[test]
    fn joint_refinement_is_deterministic_and_improves_noisy_pose() {
        let (a, b, train, holdout) = noisy_scene(800., 800., 0.6, 24);
        let frozen = relative_with_options(
            &a,
            &b,
            &train,
            &GeometryOptions::default(),
            &mut RobustReport::default(),
        )
        .expect("default pose");
        let joint = relative_with_options(
            &a,
            &b,
            &train,
            &GeometryOptions::JOINT,
            &mut RobustReport::default(),
        )
        .expect("joint pose");
        let repeated = relative_with_options(
            &a,
            &b,
            &train,
            &GeometryOptions::JOINT,
            &mut RobustReport::default(),
        )
        .expect("repeated joint pose");
        for (x, y) in joint
            .0
            .rotation
            .iter()
            .flatten()
            .chain(joint.0.translation.iter())
            .zip(
                repeated
                    .0
                    .rotation
                    .iter()
                    .flatten()
                    .chain(repeated.0.translation.iter()),
            )
        {
            assert_eq!(
                x.to_bits(),
                y.to_bits(),
                "joint refinement must be deterministic"
            );
        }
        let default_rmse = holdout_rmse(&frozen.0, &b, &holdout);
        let joint_rmse = holdout_rmse(&joint.0, &b, &holdout);
        assert!(
            joint_rmse <= default_rmse,
            "joint {joint_rmse} should not regress against default {default_rmse}"
        );
        assert!(joint_rmse < 1.0, "joint holdout RMSE {joint_rmse}");
        assert!(
            rotation_error_deg(&joint.0, &b) <= rotation_error_deg(&frozen.0, &b) + 1e-9,
            "joint rotation error must not exceed the default"
        );
    }
    #[test]
    fn analytic_epipolar_jacobian_tracks_finite_differences() {
        let (a, b, train, _) = noisy_scene(600., 600., 0.4, 12);
        let finite = relative_with_options(
            &a,
            &b,
            &train,
            &GeometryOptions::ROBUST,
            &mut RobustReport::default(),
        )
        .expect("finite-difference pose");
        let analytic = GeometryOptions {
            analytic_epipolar_jacobian: true,
            ..GeometryOptions::ROBUST
        };
        let exact = relative_with_options(&a, &b, &train, &analytic, &mut RobustReport::default())
            .expect("analytic pose");
        for i in 0..3 {
            assert!(
                norm(sub(exact.0.rotation[i], finite.0.rotation[i])) < 1e-4,
                "rotation row {i} diverged"
            );
        }
        assert!(norm(sub(exact.0.translation, finite.0.translation)) < 1e-4);
    }
}
