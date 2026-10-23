//! Infill curves: preserves the original arithmetic and proof budgets.
use super::*;

pub(super) fn hatch_angled(
    source: &Rings,
    spacing: f64,
    angle_rad: f64,
    role: PathRole,
    budget: &mut Budget,
) -> Result<Vec<Toolpath>> {
    if angle_rad.abs() <= 1e-12 {
        let mut out = hatch(source, spacing, budget)?;
        for p in &mut out {
            p.role = role;
        }
        return Ok(out);
    }
    let (s, c) = angle_rad.sin_cos();
    let rotated: Rings = source
        .iter()
        .map(|ring| {
            ring.iter()
                .map(|&[x, y]| [x * c + y * s, -x * s + y * c])
                .collect()
        })
        .collect();
    let mut out = hatch(&rotated, spacing, budget)?;
    for path in &mut out {
        path.role = role;
        for pt in &mut path.points {
            let [rx, ry] = *pt;
            *pt = [rx * c - ry * s, rx * s + ry * c];
        }
    }
    Ok(out)
}

pub(super) fn generate_cubic_infill(
    remaining: &Rings,
    spacing: f64,
    z_mm: f64,
    role: PathRole,
    budget: &mut Budget,
) -> Result<Vec<Toolpath>> {
    if remaining.is_empty() {
        return Ok(Vec::new());
    }
    let phase = (z_mm / std::f64::consts::SQRT_2).rem_euclid(spacing);
    let mut paths = Vec::new();
    for angle in [
        0.0_f64,
        std::f64::consts::FRAC_PI_3,
        -std::f64::consts::FRAC_PI_3,
    ] {
        let (s, c) = angle.sin_cos();
        let shift_x = -phase * s;
        let shift_y = phase * c;
        let shifted: Rings = remaining
            .iter()
            .map(|ring| {
                ring.iter()
                    .map(|&[x, y]| [x - shift_x, y - shift_y])
                    .collect()
            })
            .collect();
        let mut dir_paths = hatch_angled(&shifted, spacing, angle, role, budget)?;
        for p in &mut dir_paths {
            for pt in &mut p.points {
                pt[0] += shift_x;
                pt[1] += shift_y;
            }
        }
        paths.extend(dir_paths);
    }
    Ok(paths)
}

pub(super) fn point_in_rings(rings: &Rings, p: [f64; 2]) -> bool {
    let mut winding = 0i32;
    for ring in rings {
        for i in 0..ring.len() {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            if a[1] <= p[1] {
                if b[1] > p[1]
                    && (b[0] - a[0]) * (p[1] - a[1]) - (p[0] - a[0]) * (b[1] - a[1]) > 0.0
                {
                    winding += 1;
                }
            } else if b[1] <= p[1]
                && (b[0] - a[0]) * (p[1] - a[1]) - (p[0] - a[0]) * (b[1] - a[1]) < 0.0
            {
                winding -= 1;
            }
        }
    }
    winding != 0
}

pub(super) fn hilbert_d2xy(n: usize, mut d: usize) -> (usize, usize) {
    let (mut x, mut y) = (0usize, 0usize);
    let mut s = 1usize;
    while s < n {
        let rx = 1 & (d / 2);
        let ry = 1 & (d ^ rx);
        if ry == 0 {
            if rx == 1 {
                x = s - 1 - x;
                y = s - 1 - y;
            }
            std::mem::swap(&mut x, &mut y);
        }
        x += s * rx;
        y += s * ry;
        d /= 4;
        s *= 2;
    }
    (x, y)
}

pub(super) fn generate_hilbert_infill(
    remaining: &Rings,
    spacing: f64,
    role: PathRole,
    budget: &mut Budget,
) -> Result<Vec<Toolpath>> {
    if remaining.is_empty() {
        return Ok(Vec::new());
    }
    let (mut min_x, mut min_y) = (f64::INFINITY, f64::INFINITY);
    let (mut max_x, mut max_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for &[x, y] in remaining.iter().flatten() {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    let extent = (max_x - min_x).max(max_y - min_y);
    if extent <= 1e-6 {
        return Ok(Vec::new());
    }
    let target_cells = ((extent / spacing).ceil() as usize).clamp(2, 64);
    let n = target_cells.next_power_of_two().clamp(2, 64);
    let total = n * n;
    budget.work(total * point_count(remaining).max(1))?;
    let step = extent / n as f64;
    let mut paths = Vec::new();
    let mut current = Vec::new();
    for d in 0..total {
        let (hx, hy) = hilbert_d2xy(n, d);
        let pt = [min_x + (hx as f64 + 0.5) * step, min_y + (hy as f64 + 0.5) * step];
        if point_in_rings(remaining, pt) {
            current.push(pt);
        } else if current.len() >= 2 {
            budget.points(current.len())?;
            paths.push(Toolpath {
                role,
                points: std::mem::take(&mut current),
                closed: false,
            });
        } else {
            current.clear();
        }
    }
    if current.len() >= 2 {
        budget.points(current.len())?;
        paths.push(Toolpath {
            role,
            points: current,
            closed: false,
        });
    }
    Ok(paths)
}

pub(super) fn generate_archimedean_infill(
    remaining: &Rings,
    spacing: f64,
    role: PathRole,
    budget: &mut Budget,
) -> Result<Vec<Toolpath>> {
    if remaining.is_empty() {
        return Ok(Vec::new());
    }
    let (mut min_x, mut min_y) = (f64::INFINITY, f64::INFINITY);
    let (mut max_x, mut max_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for &[x, y] in remaining.iter().flatten() {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    let cx = 0.5 * (min_x + max_x);
    let cy = 0.5 * (min_y + max_y);
    let max_r = (0.5 * (max_x - min_x)).hypot(0.5 * (max_y - min_y));
    let turns = (max_r / spacing).ceil().clamp(1.0, 256.0);
    let steps = (turns as usize * 36).clamp(36, 8192);
    budget.work(steps * point_count(remaining).max(1))?;
    let b = spacing / std::f64::consts::TAU;
    let max_theta = turns * std::f64::consts::TAU;
    let mut paths = Vec::new();
    let mut current = Vec::new();
    for i in 0..=steps {
        let theta = max_theta * (i as f64 / steps as f64);
        let r = b * theta;
        let (s, c) = theta.sin_cos();
        let pt = [cx + r * c, cy + r * s];
        if point_in_rings(remaining, pt) {
            current.push(pt);
        } else if current.len() >= 2 {
            budget.points(current.len())?;
            paths.push(Toolpath {
                role,
                points: std::mem::take(&mut current),
                closed: false,
            });
        } else {
            current.clear();
        }
    }
    if current.len() >= 2 {
        budget.points(current.len())?;
        paths.push(Toolpath {
            role,
            points: current,
            closed: false,
        });
    }
    Ok(paths)
}
