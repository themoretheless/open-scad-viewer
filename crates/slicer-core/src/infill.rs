//! Scanline AET hatching and 10 infill pattern generators for `slicer-core`.

use planar_geometry::rings::Rings;
use crate::{
    Budget, InfillPattern, MAX_HATCH_LINES, PathRole, Result, Toolpath, budget_error, inset_rings,
    invalid, path_from_ring, point_count, valid_coordinate,
};

pub(crate) fn hatch(source: &Rings, spacing: f64, budget: &mut Budget) -> Result<Vec<Toolpath>> {
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for point in source.iter().flatten() {
        min_y = min_y.min(point[1]);
        max_y = max_y.max(point[1]);
    }
    if source.is_empty() || max_y - min_y < spacing * 0.25 {
        return Ok(Vec::new());
    }
    let start = min_y + spacing * 0.5;
    let end = max_y - spacing * 0.25;
    if start > end {
        return Ok(Vec::new());
    }
    let intervals = ((end - start) / spacing).floor();
    if !intervals.is_finite() || intervals >= MAX_HATCH_LINES as f64 {
        return Err(invalid(
            "TOOLPATH_HATCH_LIMIT",
            "Toolpath hatch exceeds 65536 scan lines",
        ));
    }
    let line_count = intervals as usize + 1;
    let edges = point_count(source);
    let row_work = edges
        .checked_mul(1 + edges.max(1).ilog2() as usize)
        .ok_or_else(budget_error)?;
    budget.work(line_count.checked_mul(row_work).ok_or_else(budget_error)?)?;
    // Scanline active edge table: index every non-horizontal edge once by its
    // lower endpoint, then rows only visit edges that can actually cross them.
    // The eligibility rule `(a[1] <= y && y < b[1]) || (b[1] <= y && y < a[1])`
    // is exactly `min_y <= y < max_y`, and the intersection formula, winding
    // sign and grouping below are unchanged, so spans stay bit-identical to
    // the naive per-row scan over all edges.
    struct HatchEdge {
        a: [f64; 2],
        b: [f64; 2],
        min_y: f64,
        max_y: f64,
        winding: i32,
    }
    let mut sorted_edges = Vec::with_capacity(edges);
    for ring in source {
        for i in 0..ring.len() {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            if a[1] == b[1] {
                continue;
            }
            sorted_edges.push(HatchEdge {
                a,
                b,
                min_y: a[1].min(b[1]),
                max_y: a[1].max(b[1]),
                winding: if b[1] > a[1] { -1_i32 } else { 1_i32 },
            });
        }
    }
    sorted_edges.sort_by(|x, y| x.min_y.total_cmp(&y.min_y));
    let mut active: Vec<usize> = Vec::with_capacity(sorted_edges.len().min(64));
    let mut paths = Vec::with_capacity(line_count);
    let mut hits = Vec::with_capacity(sorted_edges.len().min(64));
    let mut previous_y = None;
    let mut next_edge = 0;
    for row in 0..line_count {
        // Index-based placement avoids accumulated roundoff and non-advancing
        // floating-point additions. The budget counts every attempted row.
        let y = spacing.mul_add(row as f64, start);
        if !y.is_finite() || previous_y.is_some_and(|previous| y <= previous) {
            return Err(invalid(
                "TOOLPATH_INVALID_SETTINGS",
                "Hatch spacing does not advance coordinates",
            ));
        }
        previous_y = Some(y);
        if y > end {
            break;
        }
        while next_edge < sorted_edges.len() && sorted_edges[next_edge].min_y <= y {
            active.push(next_edge);
            next_edge += 1;
        }
        active.retain(|&index| y < sorted_edges[index].max_y);
        hits.clear();
        for &index in &active {
            let edge = &sorted_edges[index];
            let (a, b) = (edge.a, edge.b);
            let t = (y - a[1]) / (b[1] - a[1]);
            let x = (b[0] - a[0]).mul_add(t, a[0]);
            if !valid_coordinate(x) {
                return Err(invalid(
                    "TOOLPATH_INVALID_GEOMETRY",
                    "Non-finite hatch intersection",
                ));
            }
            hits.push((x, edge.winding));
        }
        hits.sort_by(|a, b| a.0.total_cmp(&b.0));
        // Sweep signed crossings so holes, disconnected islands and touching
        // boundaries share the same nonzero fill rule as the wall geometry.
        let mut winding = 0_i32;
        let mut span_start = 0.0;
        let mut index = 0;
        let row_start = paths.len();
        while index < hits.len() {
            let x = hits[index].0;
            let old_winding = winding;
            while index < hits.len() && hits[index].0 == x {
                winding += hits[index].1;
                index += 1;
            }
            if old_winding == 0 && winding != 0 {
                span_start = x;
            } else if old_winding != 0 && winding == 0 && x > span_start {
                budget.points(2)?;
                paths.push(Toolpath {
                    role: PathRole::Hatch,
                    points: vec![[span_start, y], [x, y]],
                    closed: false,
                });
            }
        }
        if winding != 0 {
            return Err(invalid(
                "TOOLPATH_INVALID_GEOMETRY",
                "Unbalanced hatch boundary",
            ));
        }
        if row % 2 == 1 {
            paths[row_start..].reverse();
            for path in &mut paths[row_start..] {
                path.points.reverse();
            }
        }
    }
    Ok(paths)
}

pub(crate) fn hatch_angled(
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

pub(crate) fn generate_infill(
    remaining: &Rings,
    spacing: f64,
    pattern: InfillPattern,
    role: PathRole,
    budget: &mut Budget,
) -> Result<Vec<Toolpath>> {
    match pattern {
        InfillPattern::Rectilinear => hatch_angled(remaining, spacing, 0.0, role, budget),
        InfillPattern::Grid => {
            let mut paths = hatch_angled(remaining, spacing, 0.0, role, budget)?;
            paths.extend(hatch_angled(
                remaining,
                spacing,
                std::f64::consts::FRAC_PI_2,
                role,
                budget,
            )?);
            Ok(paths)
        }
        InfillPattern::Triangles => {
            let mut paths = hatch_angled(remaining, spacing, 0.0, role, budget)?;
            paths.extend(hatch_angled(
                remaining,
                spacing,
                std::f64::consts::FRAC_PI_3,
                role,
                budget,
            )?);
            paths.extend(hatch_angled(
                remaining,
                spacing,
                -std::f64::consts::FRAC_PI_3,
                role,
                budget,
            )?);
            Ok(paths)
        }
        InfillPattern::Concentric => {
            let mut paths = Vec::new();
            let mut step = 1usize;
            while step <= 64 {
                let offset = inset_rings(remaining, step as f64 * spacing, budget)?;
                if offset.is_empty() {
                    break;
                }
                for ring in &offset {
                    budget.points(ring.len())?;
                    paths.push(path_from_ring(role, ring, true));
                }
                step += 1;
            }
            Ok(paths)
        }
        InfillPattern::Honeycomb => {
            let mut paths = Vec::new();
            for angle in [
                0.0,
                std::f64::consts::FRAC_PI_3,
                -std::f64::consts::FRAC_PI_3,
            ] {
                let spans = hatch_angled(remaining, spacing * 1.5, angle, role, budget)?;
                paths.reserve(spans.len());
                for span in spans {
                    if span.points.len() == 2 {
                        let [a, b] = [span.points[0], span.points[1]];
                        let dx = b[0] - a[0];
                        let dy = b[1] - a[1];
                        let len = dx.hypot(dy);
                        let step_len = (spacing * 0.5).max(0.2);
                        let steps = ((len / step_len).ceil() as usize).clamp(1, 512);
                        let (nx, ny) = if len > 1e-12 {
                            (-dy / len, dx / len)
                        } else {
                            (0.0, 0.0)
                        };
                        let amp = spacing * 0.25;
                        let mut pts = Vec::with_capacity(steps + 1);
                        for i in 0..=steps {
                            let t = i as f64 / steps as f64;
                            let offset = if i % 2 == 0 { -amp } else { amp };
                            let px = a[0] + t * dx + offset * nx;
                            let py = a[1] + t * dy + offset * ny;
                            if point_in_rings(remaining, [px, py]) {
                                pts.push([px, py]);
                            }
                        }
                        if pts.len() >= 2 {
                            budget.points(pts.len())?;
                            paths.push(Toolpath {
                                role,
                                points: pts,
                                closed: false,
                            });
                        }
                    }
                }
            }
            Ok(paths)
        }
        InfillPattern::Gyroid => {
            let spans = hatch_angled(remaining, spacing, 0.0, role, budget)?;
            let mut paths = Vec::with_capacity(spans.len());
            let k = std::f64::consts::TAU / (spacing * 2.0).max(0.5);
            let amp = spacing * 0.35;
            for (row_idx, span) in spans.into_iter().enumerate() {
                if span.points.len() == 2 {
                    let [a, b] = [span.points[0], span.points[1]];
                    let dx = b[0] - a[0];
                    let dy = b[1] - a[1];
                    let len = dx.hypot(dy);
                    let steps = ((len / (spacing * 0.25).max(0.2)).ceil() as usize).clamp(2, 512);
                    let phase = if row_idx % 2 == 0 {
                        0.0
                    } else {
                        std::f64::consts::PI
                    };
                    let mut pts = Vec::with_capacity(steps + 1);
                    for i in 0..=steps {
                        let t = i as f64 / steps as f64;
                        let x = a[0] + t * dx;
                        let y = a[1] + t * dy + amp * (x * k + phase).sin();
                        if point_in_rings(remaining, [x, y]) {
                            pts.push([x, y]);
                        }
                    }
                    if pts.len() >= 2 {
                        budget.points(pts.len())?;
                        paths.push(Toolpath {
                            role,
                            points: pts,
                            closed: false,
                        });
                    }
                }
            }
            Ok(paths)
        }
        InfillPattern::HilbertCurve => generate_hilbert_infill(remaining, spacing, role, budget),
        InfillPattern::ArchimedeanChords => {
            generate_archimedean_infill(remaining, spacing, role, budget)
        }
        InfillPattern::CrossHatch => {
            let mut paths = hatch_angled(
                remaining,
                spacing,
                std::f64::consts::FRAC_PI_4,
                role,
                budget,
            )?;
            paths.extend(hatch_angled(
                remaining,
                spacing,
                -std::f64::consts::FRAC_PI_4,
                role,
                budget,
            )?);
            Ok(paths)
        }
        InfillPattern::TriHexagon => {
            let mut paths = hatch_angled(remaining, spacing * 1.25, 0.0, role, budget)?;
            paths.extend(hatch_angled(
                remaining,
                spacing * 1.25,
                std::f64::consts::FRAC_PI_3,
                role,
                budget,
            )?);
            paths.extend(hatch_angled(
                remaining,
                spacing * 1.25,
                -std::f64::consts::FRAC_PI_3,
                role,
                budget,
            )?);
            Ok(paths)
        }
        InfillPattern::CubicSubdivision => {
            generate_cubic_infill(remaining, spacing, 0.0, role, budget)
        }
    }
}

pub(crate) fn generate_cubic_infill(
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

fn point_in_rings(rings: &Rings, p: [f64; 2]) -> bool {
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

fn hilbert_d2xy(n: usize, mut d: usize) -> (usize, usize) {
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

fn generate_hilbert_infill(
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

fn generate_archimedean_infill(
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

/// Generates wavy phase-shifted $z$-dependent sinusoidal gyroid infill line segments
/// `([x0, y0], [x1, y1])` across the axis-aligned bounding box `bounds = ([min_x, min_y], [max_x, max_y])`
/// at height `z_mm` with row spacing `spacing_mm`.
pub fn generate_gyroid_infill_lines(
    bounds: ([f64; 2], [f64; 2]),
    z_mm: f64,
    spacing_mm: f64,
) -> Vec<([f64; 2], [f64; 2])> {
    let ([min_x, min_y], [max_x, max_y]) = bounds;
    if !min_x.is_finite()
        || !min_y.is_finite()
        || !max_x.is_finite()
        || !max_y.is_finite()
        || !z_mm.is_finite()
        || !spacing_mm.is_finite()
        || spacing_mm <= 1e-4
        || max_x <= min_x
        || max_y <= min_y
    {
        return Vec::new();
    }
    let width = max_x - min_x;
    let height = max_y - min_y;
    let rows = ((height / spacing_mm).floor() as usize).clamp(1, 2048);
    let step_x = (spacing_mm * 0.25).max(0.2);
    let steps = ((width / step_x).ceil() as usize).clamp(2, 1024);
    let period = (spacing_mm * 2.0).max(0.5);
    let k = std::f64::consts::TAU / period;
    let z_phase = z_mm * k;
    let amp = spacing_mm * 0.35 * z_phase.cos();
    let x_shift = 0.25 * period * z_phase.sin();
    let mut segments = Vec::with_capacity(rows * steps);
    for row in 0..rows {
        let base_y = min_y + (row as f64 + 0.5) * spacing_mm;
        if base_y > max_y {
            break;
        }
        let row_phase = if row % 2 == 0 {
            0.0
        } else {
            std::f64::consts::PI
        };
        let mut prev: Option<[f64; 2]> = None;
        for s in 0..=steps {
            let t = s as f64 / steps as f64;
            let x = if row % 2 == 0 {
                min_x + t * width
            } else {
                max_x - t * width
            };
            let y = (base_y + amp * ((x + x_shift) * k + row_phase).sin()).clamp(min_y, max_y);
            let pt = [x, y];
            if let Some(p0) = prev {
                segments.push((p0, pt));
            }
            prev = Some(pt);
        }
    }
    segments
}


