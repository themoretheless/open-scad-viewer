use super::*;

/// Minimum meaningful radius for this pair: resolves the "graceful failure on
/// a ridiculously small radius" case before any marching happens.
pub(super) fn minimum_radius(a: &Surface, b: &Surface, tol: f64) -> f64 {
    let mut scale = 0_f64;
    for s in [a, b] {
        for row in &s.control_points {
            for p in row {
                scale = scale.max(p.iter().fold(0., |m, x| m.max(x.abs())));
            }
        }
    }
    (1e-9 * scale.max(1.)).max(100. * tol)
}

/// Reachability warning via the certified offset validity: returns a warning
/// string when `r_max` on a side exceeds the fold-free bound.
pub(super) fn reachability_warning(
    name: &str,
    surface: &Surface,
    signed: f64,
) -> Option<String> {
    let report = offset_validity(surface, signed).ok()?;
    if report.requested_ok {
        return None;
    }
    let ([u0, u1], [v0, v1]) = domain(surface);
    let point = surface
        .evaluate(
            report.limiting_uv[0].clamp(u0, u1),
            report.limiting_uv[1].clamp(v0, v1),
        )
        .map(|e| e.point)
        .unwrap_or([f64::NAN; 3]);
    Some(format!(
        "{name}: requested radius |{signed:.6}| exceeds the certified fold-free reach {:.6} near {:?} (limiting radius)",
        report.max_offset, point
    ))
}

/// Offset-surface seed search: sample both ready-made offset surfaces on a
/// grid, take the closest pairs, and Newton-refine them on the exact map.
pub(super) fn find_seed(m: &Marcher<'_>, r: f64) -> Result<Option<[f64; 4]>> {
    let sample_grid = |surface: &Surface, distance: f64| -> Result<Vec<([f64; 2], [f64; 3])>> {
        // Ready-made offset surface when available; exact normal offset as a
        // fallback (identical for planes/spheres/cylinders by construction).
        let report = offset(surface, distance, (0.05 * r.abs()).max(10. * m.tol)).ok();
        let (du, dv) = domain(surface);
        let mut out = Vec::with_capacity(SEED_GRID * SEED_GRID);
        for i in 0..SEED_GRID {
            for j in 0..SEED_GRID {
                let u = du[0] + (du[1] - du[0]) * (i as f64 + 0.5) / SEED_GRID as f64;
                let v = dv[0] + (dv[1] - dv[0]) * (j as f64 + 0.5) / SEED_GRID as f64;
                let point = match &report {
                    Some(rep) => rep.surface.evaluate(u, v).ok().map(|e| e.point),
                    None => None,
                };
                let point = match point {
                    Some(p) => Some(p),
                    None => point_normal(surface, u, v)?.map(|(p, n)| add(p, scale(n, distance))),
                };
                if let Some(p) = point {
                    out.push(([u, v], p));
                }
            }
        }
        Ok(out)
    };
    let grid_a = sample_grid(m.a, r * m.sa)?;
    let grid_b = sample_grid(m.b, r * m.sb)?;
    if grid_a.is_empty() || grid_b.is_empty() {
        return Ok(None);
    }
    // Keep the 8 closest pairs as seed candidates (selection is budgeted).
    let mut best: Vec<(f64, [f64; 4])> = Vec::with_capacity(8);
    for &(uva, pa) in &grid_a {
        for &(uvb, pb) in &grid_b {
            let d = norm(sub(pa, pb));
            let candidate = (d, [uva[0], uva[1], uvb[0], uvb[1]]);
            if best.len() < 8 {
                best.push(candidate);
                best.sort_by(|x, y| x.0.total_cmp(&y.0));
            } else if d < best[7].0 {
                best[7] = candidate;
                best.sort_by(|x, y| x.0.total_cmp(&y.0));
            }
        }
    }
    for (_, x0) in best {
        if let Some((x, _)) = m.refine(x0, r)? {
            if m.inside(&x) {
                return Ok(Some(x));
            }
        }
    }
    Ok(None)
}

/// March one direction from the seed with a curvature-adaptive step.
pub(super) fn march_direction(
    m: &Marcher<'_>,
    seed: [f64; 4],
    sign: f64,
    radius_at: &dyn Fn(f64) -> f64,
    max_steps: usize,
    closed_at: [f64; 3],
) -> Result<(Vec<RawPoint>, bool)> {
    let mut points = Vec::new();
    let mut x = seed;
    let r0 = radius_at(0.);
    let Some(mut tangent) = m.tangent(&x, r0)? else {
        return Ok((points, false));
    };
    let h_min = 1e-4 * r0;
    let h_max = r0;
    let mut h = 0.5 * r0;
    let mut s = 0.;
    let mut closed = false;
    let mut budget = Budget::with_iterations(max_steps + 1)?.guard("fillet_marching");
    for _ in 0..max_steps {
        budget.tick()?;
        let r_here = radius_at(s);
        let h_min = h_min.min(1e-4 * r_here);
        let h_max = h_max.max(2. * r_here);
        let mut accepted = None;
        let mut step = h;
        for _ in 0..6 {
            let mut candidate = x;
            for i in 0..4 {
                candidate[i] += sign * step * tangent[i];
            }
            if let Some((refined, _)) = m.refine(candidate, radius_at(s + sign * step))? {
                if m.inside(&refined) {
                    accepted = Some(refined);
                    break;
                }
            }
            step *= 0.5;
            if step < h_min * 1e-3 {
                break;
            }
        }
        let Some(next) = accepted else { break };
        let s_next = s + sign * step;
        let r_next = radius_at(s_next);
        let Some((pa, na)) = point_normal(m.a, next[0], next[1])? else { break };
        let Some((pb, nb)) = point_normal(m.b, next[2], next[3])? else { break };
        let center = scale(add(add(pa, scale(na, r_next * m.sa)), add(pb, scale(nb, r_next * m.sb))), 0.5);
        // Curvature-adaptive next step: chord error κ h²/8 ≤ chord tolerance.
        let mut h_next = (h * 1.5).min(h_max);
        if let Some(t_next) = m.tangent(&next, r_next)? {
            let d = dot4(t_next, tangent);
            let t_next = if d >= 0. { t_next } else { t_next.map(|v| -v) };
            let kappa = d.abs().clamp(-1., 1.).acos() / step.max(1e-300);
            if kappa > 1e-12 {
                let chord_tol = (0.02 * r_next).max(10. * m.tol);
                h_next = h_next.min((8. * chord_tol / kappa).sqrt()).clamp(h_min, h_max);
            }
            tangent = t_next;
        }
        points.push(RawPoint {
            x: next,
            center,
            contact_a: pa,
            contact_b: pb,
            s: s_next,
        });
        x = next;
        s = s_next;
        h = h_next;
        // Loop closure back at the seed after enough travel.
        if points.len() > 8 && norm(sub(center, closed_at)) < 0.25 * r_next {
            closed = true;
            break;
        }
    }
    Ok((points, closed))
}

/// Full two-direction spine trace with a radius law evaluated on the
/// normalized arc-length parameter of a previous pass (`law_length`), or a
/// constant radius when `law` is None and `seed_r` is used.
pub(super) fn trace_spine(
    m: &Marcher<'_>,
    seed_r: f64,
    max_steps: usize,
) -> Result<(Vec<RawPoint>, Option<[f64; 4]>, bool)> {
    let Some(seed) = find_seed(m, seed_r)? else {
        return Ok((Vec::new(), None, false));
    };
    let (Some((pa, na)), Some((pb, nb))) = (
        point_normal(m.a, seed[0], seed[1])?,
        point_normal(m.b, seed[2], seed[3])?,
    ) else {
        return Ok((Vec::new(), None, false));
    };
    let seed_center = scale(add(add(pa, scale(na, seed_r * m.sa)), add(pb, scale(nb, seed_r * m.sb))), 0.5);
    let radius_at = |_: f64| seed_r;
    let (mut backward, closed_b) = march_direction(m, seed, -1., &radius_at, max_steps / 2, seed_center)?;
    let (forward, closed_f) = march_direction(m, seed, 1., &radius_at, max_steps / 2, seed_center)?;
    backward.reverse();
    let mut points = backward;
    points.push(RawPoint { x: seed, center: seed_center, contact_a: pa, contact_b: pb, s: 0. });
    points.extend(forward);
    // Re-base arc lengths to start at zero.
    let s0 = points[0].s;
    for p in &mut points {
        p.s -= s0;
    }
    Ok((points, Some(seed), closed_b || closed_f))
}

/// Re-polish each raw point with the radius of the final law at its measured
/// normalized arc position (variable-radius second pass).
pub(super) fn polish_with_law(
    m: &Marcher<'_>,
    points: &mut [RawPoint],
    law: &RadiusLaw,
    warnings: &mut Vec<String>,
) -> Result<Option<FilletFailure>> {
    let length = points.last().map(|p| p.s).unwrap_or(0.);
    if length <= 0. {
        return Ok(Some(FilletFailure {
            reason: FailureReason::MarchingDiverged,
            t: Some(0.),
            detail: "spine has no extent".into(),
        }));
    }
    for p in points.iter_mut() {
        let t = (p.s / length).clamp(0., 1.);
        let r = law.evaluate(t);
        match m.refine(p.x, r)? {
            Some((x, center)) if m.inside(&x) => {
                p.x = x;
                p.center = center;
                if let (Some((pa, _)), Some((pb, _))) = (
                    point_normal(m.a, x[0], x[1])?,
                    point_normal(m.b, x[2], x[3])?,
                ) {
                    p.contact_a = pa;
                    p.contact_b = pb;
                }
            }
            _ => warnings.push(format!("polish with r({t:.4}) = {r:.6} did not converge; kept first-pass point")),
        }
    }
    Ok(None)
}

/// Resample the raw spine polyline into `n` arc-length-uniform stations and
/// polish each at its own radius.
pub(super) fn stations(
    m: &Marcher<'_>,
    points: &[RawPoint],
    n: usize,
    law: &RadiusLaw,
) -> Result<Vec<SpineSample>> {
    let length = points.last().map(|p| p.s).unwrap_or(0.);
    let mut out = Vec::with_capacity(n);
    let mut span = 0;
    for k in 0..n {
        let t = k as f64 / (n - 1) as f64;
        let target = t * length;
        while span + 1 < points.len() && points[span + 1].s < target {
            span += 1;
        }
        let (p0, p1) = (&points[span], &points[(span + 1).min(points.len() - 1)]);
        let w = if p1.s > p0.s { ((target - p0.s) / (p1.s - p0.s)).clamp(0., 1.) } else { 0. };
        let mut x = [0.; 4];
        for i in 0..4 {
            x[i] = p0.x[i] + w * (p1.x[i] - p0.x[i]);
        }
        let r = law.evaluate(t);
        let (x, center, ca, cb) = match m.refine(x, r)? {
            Some((rx, rc)) if m.inside(&rx) => {
                let ca = point_normal(m.a, rx[0], rx[1])?.map(|(p, _)| p).unwrap_or(p0.contact_a);
                let cb = point_normal(m.b, rx[2], rx[3])?.map(|(p, _)| p).unwrap_or(p0.contact_b);
                (rx, rc, ca, cb)
            }
            _ => (
                x,
                add(p0.center, scale(sub(p1.center, p0.center), w)),
                add(p0.contact_a, scale(sub(p1.contact_a, p0.contact_a), w)),
                add(p0.contact_b, scale(sub(p1.contact_b, p0.contact_b), w)),
            ),
        };
        out.push(SpineSample {
            t,
            center,
            contact_a: ca,
            contact_b: cb,
            radius: r,
            uv_a: [x[0], x[1]],
            uv_b: [x[2], x[3]],
        });
    }
    Ok(out)
}
