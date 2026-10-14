use super::*;

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
pub(super) fn dlt(template: &Camera, pairs: &[(V3, [f64; 2])]) -> Option<Camera> {
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
