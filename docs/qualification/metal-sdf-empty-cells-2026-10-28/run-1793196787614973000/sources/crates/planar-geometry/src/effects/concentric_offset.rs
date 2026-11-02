use super::*;

pub(super) use crate::lerp;

pub(super) fn bbox_of(points: &[[f64; 2]]) -> Result<([f64; 2], [f64; 2])> {
    check(!points.is_empty(), "Empty point set")?;
    let mut min = points[0];
    let mut max = points[0];
    for p in points {
        min[0] = min[0].min(p[0]);
        min[1] = min[1].min(p[1]);
        max[0] = max[0].max(p[0]);
        max[1] = max[1].max(p[1]);
    }
    Ok((min, max))
}

pub(super) fn path_bbox(path: &BezierPath) -> Result<([f64; 2], [f64; 2])> {
    // Curvex's editable Bézier bounds include control handles. The same bounds
    // must drive its warp and copy pivots, otherwise a migrated document jumps.
    validate_effect_path(path)?;
    let mut points = Vec::with_capacity(path.segments.len() * 3 + 1);
    points.push(path.start);
    for segment in &path.segments {
        match *segment {
            PathSegment::Line { to } => points.push(to),
            PathSegment::Cubic { c1, c2, to } => points.extend([c1, c2, to]),
        }
    }
    bbox_of(&points)
}

pub(super) fn map_path_points(path: &BezierPath, mut f: impl FnMut([f64; 2]) -> [f64; 2]) -> BezierPath {
    let start = f(path.start);
    let segments = path
        .segments
        .iter()
        .map(|seg| match *seg {
            PathSegment::Line { to } => PathSegment::Line { to: f(to) },
            PathSegment::Cubic { c1, c2, to } => PathSegment::Cubic {
                c1: f(c1),
                c2: f(c2),
                to: f(to),
            },
        })
        .collect();
    BezierPath {
        start,
        segments,
        closed: path.closed,
    }
}

pub(super) fn translate_path(path: &BezierPath, d: [f64; 2]) -> BezierPath {
    map_path_points(path, |p| add2(p, d))
}

pub(super) fn rotate_about(p: [f64; 2], pivot: [f64; 2], radians: f64) -> [f64; 2] {
    let c = radians.cos();
    let s = radians.sin();
    let v = sub2(p, pivot);
    add2(pivot, [v[0] * c - v[1] * s, v[0] * s + v[1] * c])
}

// ----- Generators -----

/// Nested offsets of a closed region: distances `step, 2*step, …, count*step`.
pub fn concentric_offset(
    rings: &crate::rings::Rings,
    count: usize,
    step: f64,
    join: &str,
    segments: usize,
) -> Result<Vec<crate::rings::Rings>> {
    concentric_offset_with_options(
        rings,
        count,
        &crate::path_offset::OffsetOptions {
            distance: step,
            join: crate::path_offset::parse_join(join),
            segments,
            ..Default::default()
        },
    )
}

/// Every offset is measured from the original region; `options.distance` is
/// the signed step. Collapsed insets stop the series, invalid inputs fail.
pub fn concentric_offset_with_options(
    rings: &crate::rings::Rings,
    count: usize,
    options: &crate::path_offset::OffsetOptions,
) -> Result<Vec<crate::rings::Rings>> {
    check((1..=64).contains(&count), "Concentric count must be 1..=64")?;
    check(
        options.distance.is_finite() && options.distance.abs() > 1e-9,
        "Invalid concentric step",
    )?;
    let mut out = Vec::with_capacity(count);
    for k in 1..=count {
        let options = crate::path_offset::OffsetOptions {
            distance: options.distance * k as f64,
            ..*options
        };
        match crate::path_offset::offset_closed_rings(rings, &options) {
            Ok(r) if !r.is_empty() => out.push(r),
            Ok(_) => break,
            Err(err) if options.distance < 0. && err.message == "Offset collapsed the region" => {
                break;
            }
            Err(err) => return Err(err),
        }
    }
    check(!out.is_empty(), "Concentric offset produced no rings")?;
    Ok(out)
}

/// Archimedean-like spiral inscribed in axis-aligned bbox `[min, max]`.
pub fn spiral(
    min: [f64; 2],
    max: [f64; 2],
    turns: f64,
    inner_ratio: f64,
    clockwise: bool,
) -> Result<BezierPath> {
    let turns = if turns.is_finite() {
        turns.clamp(0.1, 50.0)
    } else {
        0.1
    };
    let inner_ratio = if inner_ratio.is_finite() {
        inner_ratio.clamp(0.0, 0.95)
    } else {
        0.0
    };
    let center = [(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5];
    let rx = ((max[0] - min[0]).abs() * 0.5).max(1e-3);
    let ry = ((max[1] - min[1]).abs() * 0.5).max(1e-3);
    let dir = if clockwise { 1.0 } else { -1.0 };
    let n = ((turns * 48.0).ceil() as usize).clamp(24, MAX_SAMPLES);
    let sample = |i: usize| -> [f64; 2] {
        let t = i as f64 / (n - 1) as f64;
        let ang = dir * t * turns * TAU;
        let frac = inner_ratio + (1.0 - inner_ratio) * t;
        [
            center[0] + frac * rx * ang.cos(),
            center[1] + frac * ry * ang.sin(),
        ]
    };
    let pts: Vec<[f64; 2]> = (0..n).map(sample).collect();
    BezierPath::from_polyline(&pts, false)
}

/// Concentric ellipse rings + radial spokes inscribed in bbox.
pub fn polar_grid(
    min: [f64; 2],
    max: [f64; 2],
    circles: usize,
    spokes: usize,
    inner_ratio: f64,
    full_spokes: bool,
) -> Result<Vec<BezierPath>> {
    check(circles <= 64 && spokes <= 128, "Polar grid budget exceeded")?;
    let center = [(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5];
    let outer_rx = (max[0] - min[0]).abs() * 0.5;
    let outer_ry = (max[1] - min[1]).abs() * 0.5;
    if outer_rx <= 1e-4 && outer_ry <= 1e-4 {
        return Ok(Vec::new());
    }
    let inner = inner_ratio.clamp(0.0, 0.95);
    let mut out = Vec::new();
    for k in 0..circles {
        let f = if circles <= 1 {
            1.0
        } else {
            inner + (1.0 - inner) * (k as f64) / ((circles - 1) as f64)
        };
        if f * outer_rx <= 1e-4 && f * outer_ry <= 1e-4 {
            continue;
        }
        out.push(BezierPath::from_ellipse(
            center,
            outer_rx * f,
            outer_ry * f,
        )?);
    }
    let rim = |theta: f64| -> [f64; 2] {
        [
            center[0] + outer_rx * theta.cos(),
            center[1] + outer_ry * theta.sin(),
        ]
    };
    for s in 0..spokes {
        let theta = (s as f64) / (spokes.max(1) as f64) * TAU;
        if full_spokes {
            if theta >= PI - 1e-4 {
                continue;
            }
            let antipode = [
                center[0] - outer_rx * theta.cos(),
                center[1] - outer_ry * theta.sin(),
            ];
            out.push(BezierPath::from_polyline(&[antipode, rim(theta)], false)?);
        } else {
            let inner_pt = [
                center[0] + outer_rx * inner * theta.cos(),
                center[1] + outer_ry * inner * theta.sin(),
            ];
            out.push(BezierPath::from_polyline(&[inner_pt, rim(theta)], false)?);
        }
    }
    Ok(out)
}

pub fn step_and_repeat(
    path: &BezierPath,
    count: usize,
    delta: [f64; 2],
) -> Result<Vec<BezierPath>> {
    check((1..=MAX_COPIES).contains(&count), "Invalid repeat count")?;
    check(delta.iter().all(|x| x.is_finite()), "Invalid delta")?;
    Ok((0..count)
        .map(|i| translate_path(path, scale2(delta, i as f64)))
        .collect())
}

pub fn radial_repeat(path: &BezierPath, count: usize, center: [f64; 2]) -> Result<Vec<BezierPath>> {
    check((1..=MAX_COPIES).contains(&count), "Invalid radial count")?;
    check(center.iter().all(|x| x.is_finite()), "Invalid center")?;
    Ok((0..count)
        .map(|i| {
            let ang = TAU * (i as f64) / (count as f64);
            map_path_points(path, |p| rotate_about(p, center, ang))
        })
        .collect())
}

pub fn grid_array(
    path: &BezierPath,
    rows: usize,
    cols: usize,
    spacing: [f64; 2],
) -> Result<Vec<BezierPath>> {
    check(
        rows >= 1 && cols >= 1 && rows * cols <= MAX_COPIES,
        "Invalid grid size",
    )?;
    check(spacing.iter().all(|x| x.is_finite()), "Invalid spacing")?;
    let mut out = Vec::with_capacity(rows * cols);
    for r in 0..rows {
        for c in 0..cols {
            out.push(translate_path(
                path,
                [spacing[0] * c as f64, spacing[1] * r as f64],
            ));
        }
    }
    Ok(out)
}

/// Curvex copy-only repeat: first copy is one delta from the source. Members
/// retain selection order and callers keep the original paths separately.
pub fn step_and_repeat_paths(
    paths: &[BezierPath],
    count: usize,
    delta: [f64; 2],
) -> Result<Vec<BezierPath>> {
    check(
        (1..=MAX_COPIES).contains(&count) && delta.iter().all(|v| v.is_finite()),
        "Invalid repeat params",
    )?;
    selection_bbox(paths, count)?;
    let mut result = Vec::with_capacity(paths.len() * count);
    for step in 1..=count {
        for path in paths {
            let copy = translate_path(path, scale2(delta, step as f64));
            validate_effect_path(&copy)?;
            result.push(copy);
        }
    }
    Ok(result)
}

/// Curvex polar copies with an explicit angle step. `rotate_copies=false`
/// orbits each member's bounds center while preserving its orientation.
pub fn radial_repeat_paths(
    paths: &[BezierPath],
    count: usize,
    center: [f64; 2],
    angle_step: f64,
    rotate_copies: bool,
) -> Result<Vec<BezierPath>> {
    check(
        (1..=MAX_COPIES).contains(&count)
            && center.iter().all(|v| v.is_finite())
            && angle_step.is_finite(),
        "Invalid radial repeat params",
    )?;
    selection_bbox(paths, count)?;
    let mut result = Vec::with_capacity(paths.len() * count);
    for step in 1..=count {
        let angle = angle_step * step as f64;
        for path in paths {
            let copy = if rotate_copies {
                map_path_points(path, |p| rotate_about(p, center, angle))
            } else {
                let (min, max) = path_bbox(path)?;
                let own_center = add2(scale2(min, 0.5), scale2(max, 0.5));
                translate_path(
                    path,
                    sub2(rotate_about(own_center, center, angle), own_center),
                )
            };
            validate_effect_path(&copy)?;
            result.push(copy);
        }
    }
    Ok(result)
}

/// Curvex grid copies, row-major, omitting the (0,0) source cell.
pub fn grid_array_paths(
    paths: &[BezierPath],
    rows: usize,
    cols: usize,
    spacing: [f64; 2],
) -> Result<Vec<BezierPath>> {
    let cells = rows.checked_mul(cols);
    check(
        rows > 0
            && cols > 0
            && cells.is_some_and(|v| v <= MAX_COPIES)
            && spacing.iter().all(|v| v.is_finite()),
        "Invalid grid params",
    )?;
    let copies = cells.unwrap() - 1;
    selection_bbox(paths, copies.max(1))?;
    let mut result = Vec::with_capacity(paths.len() * copies);
    for row in 0..rows {
        for col in 0..cols {
            if row == 0 && col == 0 {
                continue;
            }
            for path in paths {
                let copy = translate_path(path, [spacing[0] * col as f64, spacing[1] * row as f64]);
                validate_effect_path(&copy)?;
                result.push(copy);
            }
        }
    }
    Ok(result)
}

/// Scanline hatch segments clipped to a closed ring (nonzero winding).
pub fn hatch(
    ring: &[[f64; 2]],
    spacing: f64,
    angle_rad: f64,
    cross: bool,
) -> Result<Vec<BezierPath>> {
    hatch_rings(
        &[ring.to_vec()],
        crate::tessellation::FillRule::NonZero,
        spacing,
        angle_rad,
        cross,
    )
}

/// Hatch clipped to multi-contour `rings` under `fill_rule` (holes / compound).
pub fn hatch_rings(
    rings: &[Vec<[f64; 2]>],
    fill_rule: crate::tessellation::FillRule,
    spacing: f64,
    angle_rad: f64,
    cross: bool,
) -> Result<Vec<BezierPath>> {
    check(!rings.is_empty(), "Hatch needs at least one ring")?;
    check(
        rings.iter().all(|r| r.len() >= 3),
        "Hatch ring needs ≥3 vertices",
    )?;
    check(
        spacing > 1e-6 && spacing.is_finite() && angle_rad.is_finite(),
        "Invalid hatch params",
    )?;
    let (min, max) = rings_bbox(rings)?;
    let width = max[0] - min[0];
    let height = max[1] - min[1];
    check(width > 1e-9 && height > 1e-9, "Degenerate hatch bbox")?;
    let half_diag = width.hypot(height) * 0.5 + spacing;
    let lines_per = (((2.0 * half_diag) / spacing).ceil() as usize).saturating_add(1);
    let families = if cross { 2 } else { 1 };
    check(
        lines_per.saturating_mul(families) <= HATCH_MAX_LINES,
        "Hatch line budget exceeded",
    )?;
    let edge_count: usize = rings.iter().map(|r| r.len()).sum();
    check(edge_count <= HATCH_MAX_EDGES, "Hatch edge budget exceeded")?;
    check(
        lines_per
            .saturating_mul(families)
            .saturating_mul(edge_count)
            <= HATCH_MAX_LINE_EDGE_TESTS,
        "Hatch work budget exceeded",
    )?;

    let mut out = Vec::new();
    let mut span_count = 0usize;
    let family = HatchFamily {
        rings,
        fill_rule,
        spacing,
        min,
        max,
        half_diag,
        lines_per,
    };
    emit_hatch_family(&family, angle_rad, &mut out, &mut span_count)?;
    if cross {
        emit_hatch_family(&family, angle_rad + PI * 0.5, &mut out, &mut span_count)?;
    }
    Ok(out)
}

/// Flatten a closed path (and optional hole paths) then hatch.
pub fn hatch_path(
    outer: &BezierPath,
    holes: &[BezierPath],
    fill_rule: crate::tessellation::FillRule,
    spacing: f64,
    angle_rad: f64,
    cross: bool,
    tolerance: f64,
) -> Result<Vec<BezierPath>> {
    check(outer.closed, "Hatch path must be closed")?;
    let mut rings = vec![outer.to_ring(tolerance)?];
    for h in holes {
        check(h.closed, "Hatch hole path must be closed")?;
        rings.push(h.to_ring(tolerance)?);
    }
    hatch_rings(&rings, fill_rule, spacing, angle_rad, cross)
}

struct HatchFamily<'a> {
    rings: &'a [Vec<[f64; 2]>],
    fill_rule: crate::tessellation::FillRule,
    spacing: f64,
    min: [f64; 2],
    max: [f64; 2],
    half_diag: f64,
    lines_per: usize,
}

fn emit_hatch_family(
    family: &HatchFamily<'_>,
    angle: f64,
    out: &mut Vec<BezierPath>,
    span_count: &mut usize,
) -> Result<()> {
    let (s, c) = angle.sin_cos();
    let dir = [c, s];
    let normal = [-s, c];
    let center = [
        (family.min[0] + family.max[0]) * 0.5,
        (family.min[1] + family.max[1]) * 0.5,
    ];
    for line_index in 1..family.lines_per {
        let offset = -family.half_diag + family.spacing * line_index as f64;
        if offset >= family.half_diag - 1e-9 {
            break;
        }
        let origin = [
            center[0] + normal[0] * offset,
            center[1] + normal[1] * offset,
        ];
        let spans = hatch_clip_line(family.rings, origin, dir, family.fill_rule)?;
        for (p0, p1) in spans {
            check(*span_count < HATCH_MAX_SPANS, "Hatch span budget exceeded")?;
            *span_count += 1;
            if (p0[0] - p1[0]).hypot(p0[1] - p1[1]) > 1e-9 {
                out.push(BezierPath::from_polyline(&[p0, p1], false)?);
            }
        }
    }
    Ok(())
}

/// Clip infinite line `origin + t·dir` (|dir|≈1) to interior spans under fill rule.
fn hatch_clip_line(
    rings: &[Vec<[f64; 2]>],
    origin: [f64; 2],
    dir: [f64; 2],
    fill_rule: crate::tessellation::FillRule,
) -> Result<Vec<([f64; 2], [f64; 2])>> {
    let mut crossings: Vec<(f64, i32)> = Vec::new();
    for ring in rings {
        let n = ring.len();
        for i in 0..n {
            let a = ring[i];
            let b = ring[(i + 1) % n];
            let perp = |p: [f64; 2]| {
                let dx = p[0] - origin[0];
                let dy = p[1] - origin[1];
                dir[0] * dy - dir[1] * dx
            };
            let fa = perp(a);
            let fb = perp(b);
            // Half-open: Negative|Zero → Positive adds +1 winding.
            let side = |f: f64| -> i8 {
                if f > 1e-15 {
                    1
                } else if f < -1e-15 {
                    -1
                } else {
                    0
                }
            };
            let sa = side(fa);
            let sb = side(fb);
            let winding_delta = if (sa == -1 || sa == 0) && sb == 1 {
                1
            } else if (sb == -1 || sb == 0) && sa == 1 {
                -1
            } else {
                continue;
            };
            let denom = fa - fb;
            if denom.abs() < 1e-18 {
                continue;
            }
            let u = fa / denom;
            if !(0.0..=1.0).contains(&u) {
                continue;
            }
            let hit = lerp(a, b, u);
            let t = dir[0] * (hit[0] - origin[0]) + dir[1] * (hit[1] - origin[1]);
            check(t.is_finite(), "Non-finite hatch crossing")?;
            crossings.push((t, winding_delta));
        }
    }
    if crossings.len() < 2 {
        return Ok(Vec::new());
    }
    crossings.sort_by(|a, b| a.0.total_cmp(&b.0));

    let mut spans = Vec::new();
    let mut parity = false;
    let mut winding = 0_i32;
    let mut span_start: Option<f64> = None;
    let mut index = 0;
    while index < crossings.len() {
        let t = crossings[index].0;
        let mut count = 0usize;
        let mut delta = 0_i32;
        while index < crossings.len() && (crossings[index].0 - t).abs() <= 1e-9 {
            count += 1;
            delta += crossings[index].1;
            index += 1;
        }
        let was_inside = match fill_rule {
            crate::tessellation::FillRule::EvenOdd => parity,
            crate::tessellation::FillRule::NonZero => winding != 0,
        };
        parity ^= count % 2 == 1;
        winding += delta;
        let is_inside = match fill_rule {
            crate::tessellation::FillRule::EvenOdd => parity,
            crate::tessellation::FillRule::NonZero => winding != 0,
        };
        match (was_inside, is_inside, span_start) {
            (false, true, _) => span_start = Some(t),
            (true, false, Some(start)) if (t - start).abs() > 1e-9 => {
                let p0 = [origin[0] + dir[0] * start, origin[1] + dir[1] * start];
                let p1 = [origin[0] + dir[0] * t, origin[1] + dir[1] * t];
                spans.push((p0, p1));
                span_start = None;
            }
            (true, false, _) => span_start = None,
            _ => {}
        }
    }
    Ok(spans)
}

pub(super) fn rings_bbox(rings: &[Vec<[f64; 2]>]) -> Result<([f64; 2], [f64; 2])> {
    let mut pts = Vec::new();
    for r in rings {
        pts.extend_from_slice(r);
    }
    bbox_of(&pts)
}
