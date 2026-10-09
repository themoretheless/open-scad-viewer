use super::*;

/// Arc-length resampling of a polyline into `n` uniform stations.
pub(super) fn resample_polyline(line: &[[f64; 3]], n: usize) -> Result<Vec<[f64; 3]>> {
    check(
        line.len() >= 2 && line.len() <= 4096,
        "Hold lines need 2..=4096 points",
    )?;
    for (i, p) in line.iter().enumerate() {
        crate::foundation::guards::require_finite_at(p[0], "hold_line", 3 * i)?;
        crate::foundation::guards::require_finite_at(p[1], "hold_line", 3 * i + 1)?;
        crate::foundation::guards::require_finite_at(p[2], "hold_line", 3 * i + 2)?;
    }
    let mut s = vec![0.; line.len()];
    for i in 1..line.len() {
        s[i] = s[i - 1] + norm(sub(line[i], line[i - 1]));
    }
    let total = s[line.len() - 1];
    check(total > 0., "Hold line must have positive length")?;
    let mut out = Vec::with_capacity(n);
    let mut span = 0;
    for k in 0..n {
        let target = total * k as f64 / (n - 1) as f64;
        while span + 1 < line.len() && s[span + 1] < target {
            span += 1;
        }
        let next = (span + 1).min(line.len() - 1);
        let w = if s[next] > s[span] { ((target - s[span]) / (s[next] - s[span])).clamp(0., 1.) } else { 0. };
        out.push(add(line[span], scale(sub(line[next], line[span]), w)));
    }
    Ok(out)
}

/// Local closest-point recovery for hold-line stations: coarse grid seed plus
/// Gauss-Newton polish. Returns `(uv, point, normal)`.
pub(super) fn project_point(surface: &Surface, target: [f64; 3], tol: f64) -> Result<Option<([f64; 2], [f64; 3], [f64; 3])>> {
    require_finite_point(&target, "projection_target")?;
    let (du, dv) = domain(surface);
    let mut best: Option<(f64, f64, f64)> = None;
    for i in 0..SEED_GRID {
        for j in 0..SEED_GRID {
            let u = du[0] + (du[1] - du[0]) * (i as f64 + 0.5) / SEED_GRID as f64;
            let v = dv[0] + (dv[1] - dv[0]) * (j as f64 + 0.5) / SEED_GRID as f64;
            if let Some((p, _)) = point_normal(surface, u, v)? {
                let d = norm(sub(p, target));
                if best.is_none_or(|(bd, _, _)| d < bd) {
                    best = Some((d, u, v));
                }
            }
        }
    }
    let Some((_, mut u, mut v)) = best else { return Ok(None) };
    let mut budget = Budget::with_iterations(NEWTON_ITERS + 1)?.guard("fillet_project");
    for _ in 0..NEWTON_ITERS {
        budget.tick()?;
        let e = match surface.evaluate(u, v) {
            Ok(e) => e,
            Err(_) => return Ok(None),
        };
        let Some((su, sv)) = e.first_derivatives() else { return Ok(None) };
        let r = sub(e.point, target);
        if norm(r) <= tol * 0.1 {
            break;
        }
        let g = [[dot(su, su), dot(su, sv)], [dot(su, sv), dot(sv, sv)]];
        let det = g[0][0] * g[1][1] - g[0][1] * g[1][0];
        if det.abs() <= 1e-300 {
            return Ok(None);
        }
        let rhs = [dot(r, su), dot(r, sv)];
        let du_ = (g[1][1] * rhs[0] - g[0][1] * rhs[1]) / det;
        let dv_ = (-g[0][1] * rhs[0] + g[0][0] * rhs[1]) / det;
        u -= du_;
        v -= dv_;
        if !(du[0] - 1e-9..=du[1] + 1e-9).contains(&u) || !(dv[0] - 1e-9..=dv[1] + 1e-9).contains(&v) {
            return Ok(None);
        }
    }
    match point_normal(surface, u, v)? {
        Some((p, n)) if norm(sub(p, target)) <= 100. * tol => Ok(Some(([u, v], p, n))),
        _ => Ok(None),
    }
}
