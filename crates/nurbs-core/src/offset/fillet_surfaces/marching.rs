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




