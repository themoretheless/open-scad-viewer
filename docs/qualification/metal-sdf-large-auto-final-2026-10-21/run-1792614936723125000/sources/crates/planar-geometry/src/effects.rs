//! Generators and planar distortions for Bézier / polyline paths.
//!
//! Ported from Curvex path effects (MIT OR Apache-2.0), binary64 for polygon-core.
use crate::path::{BezierPath, PathSegment};
use crate::{Result, check};
use math_core::{add2, norm2, scale2, sub2};

const TAU: f64 = std::f64::consts::TAU;
const PI: f64 = std::f64::consts::PI;
const MAX_COPIES: usize = 256;
const MAX_SAMPLES: usize = 4096;

fn lerp(a: [f64; 2], b: [f64; 2], t: f64) -> [f64; 2] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

fn bbox_of(points: &[[f64; 2]]) -> Result<([f64; 2], [f64; 2])> {
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

fn path_bbox(path: &BezierPath) -> Result<([f64; 2], [f64; 2])> {
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

fn map_path_points(path: &BezierPath, mut f: impl FnMut([f64; 2]) -> [f64; 2]) -> BezierPath {
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

fn translate_path(path: &BezierPath, d: [f64; 2]) -> BezierPath {
    map_path_points(path, |p| add2(p, d))
}

fn rotate_about(p: [f64; 2], pivot: [f64; 2], radians: f64) -> [f64; 2] {
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

fn rings_bbox(rings: &[Vec<[f64; 2]>]) -> Result<([f64; 2], [f64; 2])> {
    let mut pts = Vec::new();
    for r in rings {
        pts.extend_from_slice(r);
    }
    bbox_of(&pts)
}

fn point_in_rings(
    p: [f64; 2],
    rings: &[Vec<[f64; 2]>],
    fill_rule: crate::tessellation::FillRule,
) -> bool {
    match fill_rule {
        crate::tessellation::FillRule::NonZero => {
            let mut winding = 0_i32;
            for r in rings {
                for i in 0..r.len() {
                    let a = r[i];
                    let b = r[(i + 1) % r.len()];
                    let c = (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
                    if a[1] <= p[1] && b[1] > p[1] && c > 0. {
                        winding += 1;
                    }
                    if a[1] > p[1] && b[1] <= p[1] && c < 0. {
                        winding -= 1;
                    }
                }
            }
            winding != 0
        }
        crate::tessellation::FillRule::EvenOdd => {
            let mut parity = false;
            for ring in rings {
                if even_odd_ring(p, ring) {
                    parity = !parity;
                }
            }
            parity
        }
    }
}

fn even_odd_ring(p: [f64; 2], ring: &[[f64; 2]]) -> bool {
    let mut inside = false;
    for i in 0..ring.len() {
        let a = ring[i];
        let b = ring[(i + 1) % ring.len()];
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1] + 0.0) + a[0]
        {
            inside = !inside;
        }
    }
    inside
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StippleKind {
    #[default]
    Dot,
    /// Circle geometry to be stroked with no fill by the caller.
    Ring,
    Cross,
    Square,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StippleOptions {
    pub spacing: f64,
    pub size: f64,
    pub kind: StippleKind,
    /// Max random offset per mark (mm); 0 = exact lattice.
    pub jitter: f64,
    pub seed: u64,
    pub fill_rule: crate::tessellation::FillRule,
}

impl Default for StippleOptions {
    fn default() -> Self {
        Self {
            spacing: 3.0,
            size: 0.6,
            kind: StippleKind::Dot,
            jitter: 0.0,
            seed: 1,
            fill_rule: crate::tessellation::FillRule::NonZero,
        }
    }
}

/// Regular stipple marks over a closed ring's bbox (nonzero, no jitter).
pub fn stipple(
    ring: &[[f64; 2]],
    spacing: f64,
    size: f64,
    kind: StippleKind,
) -> Result<Vec<BezierPath>> {
    stipple_rings(
        &[ring.to_vec()],
        &StippleOptions {
            spacing,
            size,
            kind,
            ..Default::default()
        },
    )
}

/// Stipple over multi-contour rings (holes / compound) with optional jitter.
pub fn stipple_rings(rings: &[Vec<[f64; 2]>], opts: &StippleOptions) -> Result<Vec<BezierPath>> {
    check(!rings.is_empty(), "Stipple needs at least one ring")?;
    check(
        rings.iter().all(|ring| ring.len() >= 3),
        "Stipple ring needs at least three vertices",
    )?;
    check(
        opts.spacing > 1e-6
            && opts.size > 1e-6
            && opts.jitter >= 0.
            && opts.jitter.is_finite()
            && opts.spacing.is_finite()
            && opts.size.is_finite(),
        "Invalid stipple params",
    )?;
    let (min, max) = rings_bbox(rings)?;
    let cols = (((max[0] - min[0]) / opts.spacing).floor() as usize).saturating_add(1);
    let rows = (((max[1] - min[1]) / opts.spacing).floor() as usize).saturating_add(1);
    let candidates = cols.saturating_mul(rows);
    check(
        candidates <= STIPPLE_MAX_MARKS,
        "Stipple candidate budget exceeded",
    )?;
    let edges = rings
        .iter()
        .try_fold(0usize, |n, ring| n.checked_add(ring.len()));
    check(
        edges
            .and_then(|n| n.checked_mul(candidates))
            .is_some_and(|n| n <= HATCH_MAX_LINE_EDGE_TESTS),
        "Stipple work budget exceeded",
    )?;
    let mut out = Vec::new();
    for row in 0..rows {
        let y = min[1] + row as f64 * opts.spacing;
        for col in 0..cols {
            let x = min[0] + col as f64 * opts.spacing;
            let cell = opts.seed
                ^ (col as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
                ^ (row as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f);
            let jx = if opts.jitter > 0. {
                curvex_noise(cell ^ 0x1111_1111_1111_1111) * opts.jitter
            } else {
                0.0
            };
            let jy = if opts.jitter > 0. {
                curvex_noise(cell ^ 0x2222_2222_2222_2222) * opts.jitter
            } else {
                0.0
            };
            let p = [x + jx, y + jy];
            if point_in_rings(p, rings, opts.fill_rule) {
                out.push(stipple_mark(p, opts.size, opts.kind)?);
            }
        }
    }
    Ok(out)
}

/// Flatten closed path (+ holes) then stipple.
pub fn stipple_path(
    outer: &BezierPath,
    holes: &[BezierPath],
    opts: &StippleOptions,
    tolerance: f64,
) -> Result<Vec<BezierPath>> {
    check(outer.closed, "Stipple path must be closed")?;
    let mut rings = vec![outer.to_ring(tolerance)?];
    for h in holes {
        check(h.closed, "Stipple hole path must be closed")?;
        rings.push(h.to_ring(tolerance)?);
    }
    stipple_rings(&rings, opts)
}

fn stipple_mark(center: [f64; 2], size: f64, kind: StippleKind) -> Result<BezierPath> {
    let h = size * 0.5;
    match kind {
        StippleKind::Dot => BezierPath::from_circle(center, h),
        // Dot and Ring have the same ellipse geometry. The caller fills a dot
        // and strokes a ring; polygonizing a ring does not create a hole.
        StippleKind::Ring => BezierPath::from_circle(center, h),
        StippleKind::Square => BezierPath::from_rect(
            [center[0] - h, center[1] - h],
            [center[0] + h, center[1] + h],
        ),
        StippleKind::Cross => BezierPath::from_polyline(
            &[
                [center[0] - h, center[1]],
                [center[0] + h, center[1]],
                [center[0], center[1]],
                [center[0], center[1] - h],
                [center[0], center[1] + h],
            ],
            false,
        ),
    }
}

const HATCH_MAX_LINES: usize = 4_000;
const HATCH_MAX_EDGES: usize = 100_000;
const HATCH_MAX_LINE_EDGE_TESTS: usize = 8_000_000;
const HATCH_MAX_SPANS: usize = 100_000;
const STIPPLE_MAX_MARKS: usize = 50_000;

// ----- Distortions -----
/// Curvex's ridge count is per flattened edge; open endpoints stay fixed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZigZagOptions {
    pub amplitude: f64,
    pub ridges: usize,
    pub smooth: bool,
    pub tolerance: f64,
}

impl Default for ZigZagOptions {
    fn default() -> Self {
        Self {
            amplitude: 1.,
            ridges: 4,
            smooth: false,
            tolerance: crate::path::FLATTEN_TOLERANCE,
        }
    }
}

fn validate_effect_path(path: &BezierPath) -> Result<()> {
    check(
        !path.segments.is_empty() && path.segments.len() <= MAX_SAMPLES,
        "Invalid effect path size",
    )?;
    let finite = |p: [f64; 2]| p.iter().all(|v| v.is_finite());
    check(
        finite(path.start)
            && path.segments.iter().all(|s| match *s {
                PathSegment::Line { to } => finite(to),
                PathSegment::Cubic { c1, c2, to } => finite(c1) && finite(c2) && finite(to),
            }),
        "Non-finite effect geometry",
    )?;
    check(
        !path.closed || norm2(sub2(path.segments.last().unwrap().end(), path.start)) <= 1e-9,
        "Closed effect path needs its closing segment",
    )
}

fn sampled_path(points: &[[f64; 2]], closed: bool, smooth: bool) -> Result<BezierPath> {
    check(
        points.len() >= 2 && points.len() <= MAX_SAMPLES,
        "Effect sample budget exceeded",
    )?;
    check(
        points.iter().flatten().all(|v| v.is_finite()),
        "Non-finite effect result",
    )?;
    if !smooth {
        return BezierPath::from_polyline(points, closed);
    }
    let n = points.len();
    let count = if closed { n } else { n - 1 };
    let mut segments = Vec::with_capacity(count);
    for i in 0..count {
        let p1 = points[i];
        let p2 = points[(i + 1) % n];
        let p0 = if i > 0 {
            points[i - 1]
        } else if closed {
            points[n - 1]
        } else {
            p1
        };
        let p3 = if i + 2 < n {
            points[i + 2]
        } else if closed {
            points[(i + 2) % n]
        } else {
            p2
        };
        segments.push(PathSegment::Cubic {
            c1: add2(p1, scale2(sub2(p2, p0), 1. / 6.)),
            c2: sub2(p2, scale2(sub2(p3, p1), 1. / 6.)),
            to: p2,
        });
    }
    let result = BezierPath {
        start: points[0],
        segments,
        closed,
    };
    validate_effect_path(&result)?;
    Ok(result)
}

fn dense_anchors(path: &BezierPath, detail: usize) -> Result<Vec<[f64; 2]>> {
    validate_effect_path(path)?;
    let anchors = path.anchors();
    check(anchors.len() >= 2, "Effect requires at least two anchors")?;
    let edge_count = if path.closed {
        anchors.len()
    } else {
        anchors.len() - 1
    };
    let count = edge_count
        .checked_mul(detail)
        .and_then(|v| v.checked_add(usize::from(!path.closed)));
    check(
        detail > 0 && count.is_some_and(|n| n <= MAX_SAMPLES),
        "Effect sample budget exceeded",
    )?;
    let mut points = Vec::with_capacity(count.unwrap());
    for i in 0..edge_count {
        for k in 0..detail {
            points.push(lerp(
                anchors[i],
                anchors[(i + 1) % anchors.len()],
                k as f64 / detail as f64,
            ));
        }
    }
    if !path.closed {
        points.push(*anchors.last().unwrap());
    }
    Ok(points)
}

fn centroid(points: &[[f64; 2]]) -> [f64; 2] {
    // Dividing before summing keeps large finite coordinates from overflowing.
    points.iter().fold([0., 0.], |c, p| {
        add2(c, scale2(*p, 1. / points.len() as f64))
    })
}

fn zig_zag_samples(
    path: &BezierPath,
    poly: &[[f64; 2]],
    amplitude: f64,
    ridges: &[usize],
    smooth: bool,
) -> Result<BezierPath> {
    let n = poly.len();
    check(n >= 2, "Path too short")?;
    let edge_count = if path.closed { n } else { n - 1 };
    let count = ridges.iter().try_fold(usize::from(!path.closed), |sum, r| {
        sum.checked_add(*r)?.checked_add(1)
    });
    check(
        ridges.len() == edge_count && count.is_some_and(|v| v <= MAX_SAMPLES),
        "Zig-zag sample budget exceeded",
    )?;
    let normal = |edge: usize| {
        let d = sub2(poly[(edge + 1) % n], poly[edge]);
        let len = norm2(d);
        (len > 1e-6).then(|| [-d[1] / len, d[0] / len])
    };
    let vertex_normal = |v: usize| match (normal((v + n - 1) % n), normal(v)) {
        (Some(a), Some(b)) => {
            let sum = add2(a, b);
            let len = norm2(sum);
            Some(if len > 1e-6 { scale2(sum, 1. / len) } else { a })
        }
        (Some(a), None) | (None, Some(a)) => Some(a),
        _ => None,
    };
    let mut sign = 1.;
    let mut displace = |p, normal: Option<[f64; 2]>| {
        let result = add2(p, scale2(normal.unwrap_or([0., 0.]), amplitude * sign));
        sign = -sign;
        result
    };
    let mut points = Vec::with_capacity(count.unwrap());
    for e in 0..edge_count {
        points.push(if !path.closed && e == 0 {
            poly[e]
        } else {
            displace(poly[e], vertex_normal(e))
        });
        for k in 1..=ridges[e] {
            points.push(displace(
                lerp(
                    poly[e],
                    poly[(e + 1) % n],
                    k as f64 / (ridges[e] + 1) as f64,
                ),
                normal(e),
            ));
        }
    }
    if !path.closed {
        points.push(poly[n - 1]);
    }
    sampled_path(&points, path.closed, smooth)
}

fn effect_polyline(path: &BezierPath, tolerance: f64) -> Result<Vec<[f64; 2]>> {
    validate_effect_path(path)?;
    check(
        tolerance.is_finite() && tolerance > 0.,
        "Invalid effect tolerance",
    )?;
    let mut points = path.flatten_tol(tolerance)?;
    if path.closed {
        points.pop();
    }
    Ok(points)
}

pub fn zig_zag_with_options(path: &BezierPath, options: &ZigZagOptions) -> Result<BezierPath> {
    check(
        options.amplitude.is_finite() && options.ridges <= 50,
        "Invalid zig-zag params",
    )?;
    let points = effect_polyline(path, options.tolerance)?;
    if options.amplitude.abs() <= 1e-4 {
        return Ok(path.clone());
    }
    let edges = if path.closed {
        points.len()
    } else {
        points.len().saturating_sub(1)
    };
    zig_zag_samples(
        path,
        &points,
        options.amplitude,
        &vec![options.ridges; edges],
        options.smooth,
    )
}

/// Compatibility API using distance between alternating peaks. Unlike the old
/// vertex-only displacement this samples straight edges as well as curves.
pub fn zig_zag(path: &BezierPath, amplitude: f64, wavelength: f64) -> Result<BezierPath> {
    check(
        amplitude.is_finite() && wavelength.is_finite() && wavelength > 1e-6,
        "Invalid zig-zag params",
    )?;
    let points = effect_polyline(path, crate::path::FLATTEN_TOLERANCE)?;
    if amplitude.abs() <= 1e-4 {
        return Ok(path.clone());
    }
    let edges = if path.closed {
        points.len()
    } else {
        points.len().saturating_sub(1)
    };
    let ridges = (0..edges)
        .map(|e| {
            (norm2(sub2(points[(e + 1) % points.len()], points[e])) / wavelength)
                .ceil()
                .max(1.) as usize
        })
        .collect::<Vec<_>>();
    zig_zag_samples(path, &points, amplitude, &ridges, false)
}

pub fn pucker_bloat(path: &BezierPath, amount: f64) -> Result<BezierPath> {
    check(amount.is_finite() && amount.abs() <= 1.0, "Invalid amount")?;
    validate_effect_path(path)?;
    if amount.abs() <= 1e-4 {
        return Ok(path.clone());
    }
    let anchors = path.anchors();
    check(
        anchors.len() >= 2,
        "Pucker/bloat requires at least two anchors",
    )?;
    let center = centroid(&anchors);
    let moved = anchors
        .iter()
        .map(|p| add2(center, scale2(sub2(*p, center), 1. + amount)))
        .collect::<Vec<_>>();
    let handle = |p| add2(p, scale2(sub2(p, center), amount * 0.5));
    let count = if path.closed {
        moved.len()
    } else {
        moved.len() - 1
    };
    let segments = (0..count)
        .map(|i| PathSegment::Cubic {
            c1: handle(moved[i]),
            c2: handle(moved[(i + 1) % moved.len()]),
            to: moved[(i + 1) % moved.len()],
        })
        .collect();
    let result = BezierPath {
        start: moved[0],
        segments,
        closed: path.closed,
    };
    validate_effect_path(&result)?;
    Ok(result)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoughenOptions {
    pub amplitude: f64,
    pub detail: usize,
    pub smooth: bool,
    /// Zero reproduces Curvex's fixed per-anchor noise sequence.
    pub seed: u64,
}

impl Default for RoughenOptions {
    fn default() -> Self {
        Self {
            amplitude: 1.,
            detail: 4,
            smooth: false,
            seed: 0,
        }
    }
}

/// SplitMix64 sequence used by Curvex. Keeping its high 24 bits allows an
/// existing Curvex document to regenerate the same jitter in binary64.
fn curvex_noise(seed: u64) -> f64 {
    let mut mixed = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
    mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    mixed ^= mixed >> 31;
    (mixed >> 40) as f64 / (1u64 << 24) as f64 * 2. - 1.
}

pub fn roughen_with_options(path: &BezierPath, options: &RoughenOptions) -> Result<BezierPath> {
    check(
        options.amplitude.is_finite()
            && options.amplitude >= 0.
            && (1..=50).contains(&options.detail),
        "Invalid roughen params",
    )?;
    validate_effect_path(path)?;
    if options.amplitude <= 1e-4 {
        return Ok(path.clone());
    }
    let mut points = dense_anchors(path, options.detail)?;
    let last = points.len() - 1;
    for (i, p) in points.iter_mut().enumerate() {
        if !path.closed && (i == 0 || i == last) {
            continue;
        }
        p[0] += curvex_noise(options.seed ^ (2 * i as u64 + 1)) * options.amplitude;
        p[1] += curvex_noise(options.seed ^ (2 * i as u64 + 2)) * options.amplitude;
    }
    sampled_path(&points, path.closed, options.smooth)
}

pub fn roughen(path: &BezierPath, amount: f64, seed: u64) -> Result<BezierPath> {
    roughen_with_options(
        path,
        &RoughenOptions {
            amplitude: amount,
            seed,
            ..Default::default()
        },
    )
}

pub fn twist(path: &BezierPath, radians: f64) -> Result<BezierPath> {
    check(radians.is_finite(), "Invalid twist angle")?;
    validate_effect_path(path)?;
    if radians.abs() <= 1e-4 {
        return Ok(path.clone());
    }
    let mut points = dense_anchors(path, 24)?;
    let center = centroid(&points);
    let radius = points
        .iter()
        .map(|p| norm2(sub2(*p, center)))
        .fold(0., f64::max);
    if radius > 1e-6 {
        for p in &mut points {
            *p = rotate_about(*p, center, radians * (norm2(sub2(*p, center)) / radius));
        }
    }
    sampled_path(&points, path.closed, false)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScatterOptions {
    pub count: usize,
    pub position: f64,
    pub rotation_radians: f64,
    /// Uniform factors, not percentages. Reversed bounds are accepted.
    pub scale_min: f64,
    pub scale_max: f64,
    pub seed: u64,
}

impl Default for ScatterOptions {
    fn default() -> Self {
        Self {
            count: 1,
            position: 0.,
            rotation_radians: 0.,
            scale_min: 1.,
            scale_max: 1.,
            seed: 0,
        }
    }
}

/// Scatter an entire selection as a rigid set. Results are copy-major, with
/// every member receiving the same scale, rotation and translation.
pub fn scatter_paths(paths: &[BezierPath], options: &ScatterOptions) -> Result<Vec<BezierPath>> {
    check(
        !paths.is_empty() && (1..=MAX_COPIES).contains(&options.count),
        "Invalid scatter count",
    )?;
    check(
        options.position.is_finite()
            && options.position >= 0.
            && options.rotation_radians.is_finite()
            && options.scale_min.is_finite()
            && options.scale_max.is_finite(),
        "Invalid scatter params",
    )?;
    let (min, max) = selection_bbox(paths, options.count)?;
    let pivot = add2(scale2(min, 0.5), scale2(max, 0.5));
    let lo = options.scale_min.min(options.scale_max);
    let hi = options.scale_min.max(options.scale_max);
    let mut result = Vec::with_capacity(paths.len() * options.count);
    for i in 0..options.count {
        let seed = options.seed ^ (i as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        let delta = [
            curvex_noise(seed ^ 0x1111_1111_1111_1111) * options.position,
            curvex_noise(seed ^ 0x2222_2222_2222_2222) * options.position,
        ];
        let angle = curvex_noise(seed ^ 0x3333_3333_3333_3333) * options.rotation_radians;
        let t = (curvex_noise(seed ^ 0x4444_4444_4444_4444) + 1.) * 0.5;
        let scale = lo * (1. - t) + hi * t;
        for path in paths {
            let transformed = map_path_points(path, |p| {
                add2(
                    rotate_about(add2(pivot, scale2(sub2(p, pivot), scale)), pivot, angle),
                    delta,
                )
            });
            validate_effect_path(&transformed)?;
            result.push(transformed);
        }
    }
    Ok(result)
}

fn selection_bbox(paths: &[BezierPath], copies: usize) -> Result<([f64; 2], [f64; 2])> {
    check(
        !paths.is_empty()
            && paths
                .len()
                .checked_mul(copies)
                .is_some_and(|n| n <= MAX_SAMPLES),
        "Copy shape budget exceeded",
    )?;
    let total_segments = paths
        .iter()
        .try_fold(0usize, |n, p| n.checked_add(p.segments.len()));
    check(
        total_segments
            .and_then(|n| n.checked_mul(copies))
            .is_some_and(|n| n <= 100_000),
        "Copy segment budget exceeded",
    )?;
    let mut bounds = Vec::with_capacity(paths.len() * 2);
    for path in paths {
        validate_effect_path(path)?;
        let (min, max) = path_bbox(path)?;
        bounds.extend([min, max]);
    }
    bbox_of(&bounds)
}

pub fn scatter(path: &BezierPath, count: usize, radius: f64, seed: u64) -> Result<Vec<BezierPath>> {
    scatter_paths(
        std::slice::from_ref(path),
        &ScatterOptions {
            count,
            position: radius,
            seed,
            ..Default::default()
        },
    )
}

/// Blend two paths into `steps` intermediate open/closed paths (geometry only).
pub fn blend(a: &BezierPath, b: &BezierPath, steps: usize) -> Result<Vec<BezierPath>> {
    check((1..=64).contains(&steps), "Blend steps must be 1..=64")?;
    validate_effect_path(a)?;
    validate_effect_path(b)?;
    check(a.closed == b.closed, "Blend requires matching closedness")?;
    let ca = to_cubics(a);
    let cb = to_cubics(b);
    let (ca, mut cb) = resample_to_same_len(ca, cb)?;
    check(
        ca.len().checked_mul(steps).is_some_and(|n| n <= 100_000),
        "Blend output budget exceeded",
    )?;
    if a.closed {
        cb = align_closed_cubics(&ca, &cb)?;
    }
    let mut out = Vec::with_capacity(steps);
    for i in 1..=steps {
        let t = i as f64 / (steps + 1) as f64;
        let start = lerp(ca[0].0, cb[0].0, t);
        let mut segs = Vec::with_capacity(ca.len());
        for (u, v) in ca.iter().zip(cb.iter()) {
            segs.push(PathSegment::Cubic {
                c1: lerp(u.1, v.1, t),
                c2: lerp(u.2, v.2, t),
                to: lerp(u.3, v.3, t),
            });
        }
        out.push(if a.closed {
            BezierPath::closed(start, segs)?
        } else {
            BezierPath::open(start, segs)?
        });
    }
    Ok(out)
}

type Cubic4 = ([f64; 2], [f64; 2], [f64; 2], [f64; 2]);

fn align_closed_cubics(a: &[Cubic4], b: &[Cubic4]) -> Result<Vec<Cubic4>> {
    let reversed = b
        .iter()
        .rev()
        .map(|c| (c.3, c.2, c.1, c.0))
        .collect::<Vec<_>>();
    let mut best = (false, 0);
    let mut score = f64::INFINITY;
    let mut work = 0usize;
    for (reverse, candidate) in [(false, b), (true, reversed.as_slice())] {
        for rotation in 0..a.len() {
            let mut current = 0.;
            for (i, anchor) in a.iter().enumerate() {
                work += 1;
                check(work <= 32_000_000, "Blend alignment work budget exceeded")?;
                let delta = sub2(anchor.0, candidate[(i + rotation) % b.len()].0);
                current += delta[0] * delta[0] + delta[1] * delta[1];
                if current >= score {
                    break;
                }
            }
            if current < score {
                score = current;
                best = (reverse, rotation);
            }
        }
    }
    check(score.is_finite(), "Blend alignment overflow")?;
    let source = if best.0 { reversed.as_slice() } else { b };
    Ok((0..source.len())
        .map(|i| source[(i + best.1) % source.len()])
        .collect())
}

fn to_cubics(path: &BezierPath) -> Vec<Cubic4> {
    let mut cur = path.start;
    let mut out = Vec::new();
    for seg in &path.segments {
        match *seg {
            PathSegment::Line { to } => {
                out.push((cur, lerp(cur, to, 1.0 / 3.0), lerp(cur, to, 2.0 / 3.0), to));
                cur = to;
            }
            PathSegment::Cubic { c1, c2, to } => {
                out.push((cur, c1, c2, to));
                cur = to;
            }
        }
    }
    out
}

fn resample_to_same_len(
    mut a: Vec<Cubic4>,
    mut b: Vec<Cubic4>,
) -> Result<(Vec<Cubic4>, Vec<Cubic4>)> {
    check(!a.is_empty() && !b.is_empty(), "Empty blend path")?;
    while a.len() < b.len() {
        let i = (0..a.len())
            .max_by(|&i, &j| chord(&a[i]).total_cmp(&chord(&a[j])))
            .unwrap();
        let (l, r) = split_cubic(a[i], 0.5);
        a[i] = l;
        a.insert(i + 1, r);
    }
    while b.len() < a.len() {
        let i = (0..b.len())
            .max_by(|&i, &j| chord(&b[i]).total_cmp(&chord(&b[j])))
            .unwrap();
        let (l, r) = split_cubic(b[i], 0.5);
        b[i] = l;
        b.insert(i + 1, r);
    }
    Ok((a, b))
}

fn chord(c: &Cubic4) -> f64 {
    (c.0[0] - c.3[0]).hypot(c.0[1] - c.3[1])
}

fn split_cubic(c: Cubic4, t: f64) -> (Cubic4, Cubic4) {
    let (p0, p1, p2, p3) = c;
    let q0 = lerp(p0, p1, t);
    let q1 = lerp(p1, p2, t);
    let q2 = lerp(p2, p3, t);
    let r0 = lerp(q0, q1, t);
    let r1 = lerp(q1, q2, t);
    let s = lerp(r0, r1, t);
    ((p0, q0, r0, s), (s, r1, q2, p3))
}

/// Bilinear free distort: map path into destination quad (TL, TR, BR, BL).
pub fn free_distort(path: &BezierPath, quad: [[f64; 2]; 4]) -> Result<BezierPath> {
    check(
        quad.iter().all(|p| p.iter().all(|x| x.is_finite())),
        "Invalid distort quad",
    )?;
    let (min, max) = path_bbox(path)?;
    let w = max[0] - min[0];
    let h = max[1] - min[1];
    validate_effect_path(path)?;
    let result = map_path_points(path, |p| {
        let u = if w.abs() <= 1e-4 {
            0.
        } else {
            (p[0] - min[0]) / w
        };
        let v = if h.abs() <= 1e-4 {
            0.
        } else {
            (p[1] - min[1]) / h
        };
        // bilinar: (1-u)(1-v) TL + u(1-v) TR + u v BR + (1-u) v BL
        let [tl, tr, br, bl] = quad;
        add2(
            add2(scale2(tl, (1.0 - u) * (1.0 - v)), scale2(tr, u * (1.0 - v))),
            add2(scale2(br, u * v), scale2(bl, (1.0 - u) * v)),
        )
    });
    validate_effect_path(&result)?;
    Ok(result)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArcMode {
    Arc,
    Pie,
    Segment,
}

/// Arc / pie / chord segment from ellipse inscribed in bbox.
pub fn arc_path(
    min: [f64; 2],
    max: [f64; 2],
    start_deg: f64,
    sweep_deg: f64,
    mode: ArcMode,
) -> Result<BezierPath> {
    check(
        min.iter().chain(max.iter()).all(|v| v.is_finite())
            && start_deg.is_finite()
            && sweep_deg.is_finite()
            && sweep_deg.abs() >= 0.1,
        "Invalid arc angles",
    )?;
    let center = [(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5];
    let rx = (max[0] - min[0]).abs() * 0.5;
    let ry = (max[1] - min[1]).abs() * 0.5;
    check(rx > 1e-4 && ry > 1e-4, "Degenerate arc bounds")?;
    let start = start_deg.to_radians();
    let sweep = sweep_deg.clamp(-360.0, 360.0).to_radians();
    let steps = ((sweep.abs() / (PI * 0.5)).ceil() as usize).clamp(1, 8);
    let mut pts = Vec::new();
    let mut segs = Vec::new();
    let point = |ang: f64| -> [f64; 2] { [center[0] + rx * ang.cos(), center[1] + ry * ang.sin()] };
    let p0 = point(start);
    pts.push(p0);
    let mut cur = p0;
    for i in 0..steps {
        let a0 = start + sweep * (i as f64) / (steps as f64);
        let a1 = start + sweep * ((i + 1) as f64) / (steps as f64);
        let p1 = point(a1);
        // Signed circular arc formula: negative sweeps need negative handles.
        // Linear interpolation of the quarter-circle kappa is inaccurate for
        // partial sectors (e.g. 45 degrees).
        let k = (4. / 3.) * ((a1 - a0) * 0.25).tan();
        let t0 = [-rx * a0.sin(), ry * a0.cos()];
        let t1 = [-rx * a1.sin(), ry * a1.cos()];
        let c1 = [cur[0] + t0[0] * k, cur[1] + t0[1] * k];
        let c2 = [p1[0] - t1[0] * k, p1[1] - t1[1] * k];
        segs.push(PathSegment::Cubic { c1, c2, to: p1 });
        cur = p1;
        pts.push(p1);
    }
    match mode {
        ArcMode::Arc => BezierPath::open(p0, segs),
        ArcMode::Pie => {
            segs.insert(0, PathSegment::Line { to: p0 });
            segs.push(PathSegment::Line { to: center });
            BezierPath::closed(center, segs)
        }
        ArcMode::Segment => {
            segs.push(PathSegment::Line { to: p0 });
            BezierPath::closed(p0, segs)
        }
    }
}

/// Star polygon inscribed in bbox (`points` tips, `inner_ratio` valley radius).
pub fn star(min: [f64; 2], max: [f64; 2], points: usize, inner_ratio: f64) -> Result<BezierPath> {
    check((3..=64).contains(&points), "Star points must be 3..=64")?;
    let inner = inner_ratio.clamp(0.05, 0.95);
    let center = [(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5];
    let rx = ((max[0] - min[0]).abs() * 0.5).max(1e-6);
    let ry = ((max[1] - min[1]).abs() * 0.5).max(1e-6);
    let n = points * 2;
    let mut pts = Vec::with_capacity(n);
    for i in 0..n {
        let ang = -PI * 0.5 + TAU * (i as f64) / (n as f64);
        let r = if i % 2 == 0 { 1.0 } else { inner };
        pts.push([
            center[0] + rx * r * ang.cos(),
            center[1] + ry * r * ang.sin(),
        ]);
    }
    BezierPath::from_polyline(&pts, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: [f64; 2], b: [f64; 2]) {
        assert!(norm2(sub2(a, b)) < 1e-9, "{a:?} != {b:?}");
    }

    #[test]
    fn zig_zag_creates_ridges_on_a_straight_line_and_pins_endpoints() {
        let path = BezierPath::from_polyline(&[[0., 0.], [8., 0.]], false).unwrap();
        let options = ZigZagOptions {
            amplitude: 1.,
            ridges: 3,
            ..Default::default()
        };
        let sharp = zig_zag_with_options(&path, &options).unwrap();
        assert_eq!(
            sharp.anchors(),
            vec![[0., 0.], [2., 1.], [4., -1.], [6., 1.], [8., 0.]]
        );
        let smooth = zig_zag_with_options(
            &path,
            &ZigZagOptions {
                smooth: true,
                ..options
            },
        )
        .unwrap();
        assert_eq!(smooth.anchors(), sharp.anchors());
        assert!(
            smooth
                .segments
                .iter()
                .all(|s| matches!(s, PathSegment::Cubic { .. }))
        );
        for pair in smooth.segments.windows(2) {
            if let [
                PathSegment::Cubic { c2, to, .. },
                PathSegment::Cubic { c1, .. },
            ] = pair
            {
                near(sub2(*to, *c2), sub2(*c1, *to));
            }
        }
        assert!(zig_zag(&path, 1., 2.).unwrap().segments.len() > 2);
    }

    #[test]
    fn zig_zag_closed_seam_has_the_same_treatment_as_other_corners() {
        let path = BezierPath::from_rect([0., 0.], [8., 8.]).unwrap();
        let result = zig_zag_with_options(
            &path,
            &ZigZagOptions {
                amplitude: 1.,
                ridges: 1,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(result.closed);
        assert_eq!(result.anchor_count(), 8);
        near(
            result.start,
            [
                std::f64::consts::FRAC_1_SQRT_2,
                std::f64::consts::FRAC_1_SQRT_2,
            ],
        );
        assert_eq!(result.segments.last().unwrap().end(), result.start);
        assert!(result.anchors().iter().flatten().all(|v| v.is_finite()));
    }

    #[test]
    fn pucker_bloat_bows_rectangle_sides_and_uses_anchor_centroid() {
        let rect = BezierPath::from_rect([0., 0.], [4., 2.]).unwrap();
        let bloat = pucker_bloat(&rect, 0.5).unwrap();
        assert_eq!(bloat.start, [-1., -0.5]);
        assert_eq!(
            bloat.segments[0],
            PathSegment::Cubic {
                c1: [-1.75, -0.875],
                c2: [5.75, -0.875],
                to: [5., -0.5]
            }
        );
        let (_, middle) = split_cubic(to_cubics(&bloat)[0], 0.5);
        near(middle.0, [2., -0.78125]);
        let asym = BezierPath::from_polyline(&[[0., 0.], [6., 0.], [0., 3.]], true).unwrap();
        let result = pucker_bloat(&asym, 0.5).unwrap();
        near(result.start, [-1., -0.5]); // centroid (2,1), not bounds center (3,1.5)
        let pucker = pucker_bloat(&rect, -0.5).unwrap();
        assert_eq!(pucker.start, [1., 0.5]);
        assert!(matches!(pucker.segments[0], PathSegment::Cubic { c1, .. } if c1[1] > 0.5));
        assert_eq!(pucker_bloat(&rect, 0.).unwrap(), rect);
    }

    #[test]
    fn roughen_densifies_pins_open_ends_and_can_smooth() {
        let line = BezierPath::from_polyline(&[[0., 0.], [8., 0.]], false).unwrap();
        let options = RoughenOptions {
            amplitude: 0.5,
            detail: 4,
            smooth: false,
            seed: 0,
        };
        let result = roughen_with_options(&line, &options).unwrap();
        assert_eq!(result.anchor_count(), 5);
        assert_eq!(result.start, line.start);
        assert_eq!(result.segments.last().unwrap().end(), [8., 0.]);
        for (i, p) in result.anchors().iter().enumerate().skip(1).take(3) {
            assert!((p[0] - 2. * i as f64).abs() <= 0.5 && p[1].abs() <= 0.5);
            assert_ne!(p[1], 0.);
        }
        assert_eq!(result, roughen_with_options(&line, &options).unwrap());
        assert_ne!(
            result,
            roughen_with_options(
                &line,
                &RoughenOptions {
                    seed: 42,
                    ..options
                }
            )
            .unwrap()
        );
        let smooth = roughen_with_options(
            &line,
            &RoughenOptions {
                smooth: true,
                ..options
            },
        )
        .unwrap();
        assert_eq!(result.anchors(), smooth.anchors());
        assert!(
            smooth
                .segments
                .iter()
                .all(|s| matches!(s, PathSegment::Cubic { .. }))
        );
    }

    #[test]
    fn roughen_closed_contour_has_one_consistent_seam() {
        let rect = BezierPath::from_rect([0., 0.], [4., 2.]).unwrap();
        let result = roughen_with_options(
            &rect,
            &RoughenOptions {
                detail: 3,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(result.anchor_count(), 12);
        assert_ne!(result.start, rect.start);
        assert_eq!(result.segments.last().unwrap().end(), result.start);
    }

    #[test]
    fn twist_bends_straight_edges_and_rotates_the_outer_radius_by_full_angle() {
        let rect = BezierPath::from_rect([-2., -1.], [2., 1.]).unwrap();
        let result = twist(&rect, PI * 0.5).unwrap();
        assert_eq!(result.anchor_count(), 4 * 24);
        near(result.start, [1., -2.]);
        assert_eq!(result.segments.last().unwrap().end(), result.start);
        // Midpoint radius is smaller than a corner radius and rotates less.
        let mid = result.anchors()[12];
        assert!(mid[0] > 0. && mid[0] < 1. && mid[1] < 0.);
        assert_eq!(twist(&rect, 0.).unwrap(), rect);
    }

    #[test]
    fn scatter_preserves_group_geometry_with_scale_rotation_and_seed() {
        let a = BezierPath::from_rect([0., 0.], [2., 1.]).unwrap();
        let b = translate_path(&a, [5., 0.]);
        let source = [a, b];
        let options = ScatterOptions {
            count: 3,
            position: 3.,
            rotation_radians: PI,
            scale_min: 2.,
            scale_max: 2.,
            seed: 13,
        };
        let result = scatter_paths(&source, &options).unwrap();
        assert_eq!(result, scatter_paths(&source, &options).unwrap());
        assert_eq!(result.len(), 6);
        assert_ne!(
            result,
            scatter_paths(
                &source,
                &ScatterOptions {
                    seed: 14,
                    ..options
                }
            )
            .unwrap()
        );
        for pair in result.as_chunks::<2>().0 {
            let offset = sub2(pair[1].start, pair[0].start);
            assert!((norm2(offset) - 10.).abs() < 1e-9);
            assert!(offset[1].abs() > 0.1); // rotation really happened
            for (p, q) in pair[0].anchors().into_iter().zip(pair[1].anchors()) {
                near(sub2(q, p), offset);
            }
            let edge = sub2(pair[0].segments[0].end(), pair[0].start);
            assert!((norm2(edge) - 4.).abs() < 1e-9);
        }
    }

    #[test]
    fn blend_aligns_both_seam_and_winding_before_interpolation() {
        let a = BezierPath::from_rect([0., 0.], [4., 2.]).unwrap();
        let b =
            BezierPath::from_polyline(&[[14., 2.], [14., 0.], [10., 0.], [10., 2.]], true).unwrap();
        let result = blend(&a, &b, 1).unwrap().remove(0);
        assert_eq!(
            result.anchors(),
            vec![[5., 0.], [9., 0.], [9., 2.], [5., 2.]]
        );
        for c in to_cubics(&result) {
            near(c.1, lerp(c.0, c.3, 1. / 3.));
            near(c.2, lerp(c.0, c.3, 2. / 3.));
        }
        assert_eq!(result.segments.last().unwrap().end(), result.start);
    }

    #[test]
    fn partial_and_negative_arc_sweeps_follow_the_ellipse() {
        for sweep in [-135., -45., 45., 135.] {
            let path = arc_path([-2., -1.], [2., 1.], 0., sweep, ArcMode::Arc).unwrap();
            let cubics = to_cubics(&path);
            for c in cubics {
                for k in 0..=10 {
                    let (_, right) = split_cubic(c, k as f64 / 10.);
                    let p = right.0;
                    let ellipse = (p[0] * 0.5).powi(2) + p[1].powi(2);
                    assert!((ellipse - 1.).abs() < 0.0006, "sweep {sweep}: {ellipse}");
                    assert!(p[1] * sweep >= -1e-12);
                }
            }
        }
    }

    #[test]
    fn grouped_repeats_omit_sources_and_support_upright_orbits() {
        let path = BezierPath::from_rect([2., -0.5], [4., 0.5]).unwrap();
        let source = [path.clone()];
        let linear = step_and_repeat_paths(&source, 2, [10., 0.]).unwrap();
        assert_eq!(linear[0].start, [12., -0.5]);
        assert_eq!(linear[1].start, [22., -0.5]);
        let grid = grid_array_paths(&source, 2, 2, [10., 20.]).unwrap();
        assert_eq!(grid.len(), 3);
        assert_eq!(
            grid.iter().map(|p| p.start).collect::<Vec<_>>(),
            vec![[12., -0.5], [2., 19.5], [12., 19.5]]
        );
        assert!(
            grid_array_paths(&source, 1, 1, [0., 0.])
                .unwrap()
                .is_empty()
        );
        let upright = radial_repeat_paths(&source, 1, [0., 0.], PI * 0.5, false).unwrap();
        near(upright[0].start, [-1., 2.5]);
        near(
            sub2(upright[0].segments[0].end(), upright[0].start),
            [2., 0.],
        );
        let rotated = radial_repeat_paths(&source, 1, [0., 0.], PI * 0.5, true).unwrap();
        near(
            sub2(rotated[0].segments[0].end(), rotated[0].start),
            [0., 2.],
        );
    }

    #[test]
    fn effects_reject_unbounded_and_non_finite_inputs() {
        let line = BezierPath::from_polyline(&[[0., 0.], [100., 0.]], false).unwrap();
        assert!(zig_zag(&line, 1., 0.000002).is_err());
        assert!(zig_zag(&line, 1., f64::INFINITY).is_err());
        assert!(
            roughen_with_options(
                &line,
                &RoughenOptions {
                    detail: usize::MAX,
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert!(
            scatter_paths(
                std::slice::from_ref(&line),
                &ScatterOptions {
                    count: usize::MAX,
                    ..Default::default()
                }
            )
            .is_err()
        );
        let mut invalid = line;
        invalid.start[0] = f64::NAN;
        assert!(pucker_bloat(&invalid, 0.5).is_err());
        assert!(blend(&invalid, &invalid, 1).is_err());
    }

    #[test]
    fn stipple_bounds_candidate_work_even_when_no_mark_would_survive() {
        let empty_region = vec![
            vec![
                [0., 0.],
                [1000000., 0.],
                [1000000., 1000000.],
                [0., 1000000.]
            ];
            2
        ];
        assert!(
            stipple_rings(
                &empty_region,
                &StippleOptions {
                    spacing: 0.01,
                    fill_rule: crate::tessellation::FillRule::EvenOdd,
                    ..Default::default()
                }
            )
            .is_err()
        );
        let ring = stipple_mark([2., 3.], 4., StippleKind::Ring).unwrap();
        let dot = stipple_mark([2., 3.], 4., StippleKind::Dot).unwrap();
        assert_eq!(ring, dot); // appearance, not a different polygon, makes a ring
        assert_eq!(ring.segments.len(), 4);
    }

    #[test]
    fn free_distort_uses_the_editable_control_bounds_and_handles_flat_axes() {
        let path = BezierPath::open(
            [0., 0.],
            vec![PathSegment::Cubic {
                c1: [2., 5.],
                c2: [7., -3.],
                to: [10., 0.],
            }],
        )
        .unwrap();
        let identity = free_distort(&path, [[0., -3.], [10., -3.], [10., 5.], [0., 5.]]).unwrap();
        for (a, b) in to_cubics(&path).into_iter().zip(to_cubics(&identity)) {
            near(a.0, b.0);
            near(a.1, b.1);
            near(a.2, b.2);
            near(a.3, b.3);
        }
        let line = BezierPath::from_polyline(&[[0., 2.], [10., 2.]], false).unwrap();
        let warped = free_distort(&line, [[1., 4.], [20., 8.], [20., 20.], [1., 20.]]).unwrap();
        near(warped.start, [1., 4.]);
        near(warped.segments[0].end(), [20., 8.]);
    }

    #[test]
    fn spiral_and_polar_grid() {
        let s = spiral([-10., -10.], [10., 10.], 2., 0.1, true).unwrap();
        assert!(!s.closed);
        assert!(s.segments.len() > 20);
        let g = polar_grid([-5., -5.], [5., 5.], 3, 8, 0.2, true).unwrap();
        assert!(g.len() >= 3 + 4); // rings + half spokes
    }

    #[test]
    fn arrays_and_hatch() {
        let p = BezierPath::from_rect([0., 0.], [1., 1.]).unwrap();
        assert_eq!(step_and_repeat(&p, 3, [2., 0.]).unwrap().len(), 3);
        assert_eq!(radial_repeat(&p, 4, [0., 0.]).unwrap().len(), 4);
        assert_eq!(grid_array(&p, 2, 3, [2., 2.]).unwrap().len(), 6);
        let ring = [[0., 0.], [10., 0.], [10., 10.], [0., 10.]];
        let h = hatch(&ring, 2., 0., false).unwrap();
        assert!(!h.is_empty());
        let st = stipple(&ring, 3., 0.5, StippleKind::Dot).unwrap();
        assert!(!st.is_empty());
    }

    #[test]
    fn hatch_skips_donut_hole() {
        use crate::tessellation::FillRule;
        let outer = vec![[0., 0.], [20., 0.], [20., 20.], [0., 20.]];
        let hole = vec![[6., 6.], [14., 6.], [14., 14.], [6., 14.]];
        // Opposite winding so nonzero treats hole as empty.
        let hole_cw: Vec<[f64; 2]> = hole.into_iter().rev().collect();
        let rings = vec![outer, hole_cw];
        let lines = hatch_rings(&rings, FillRule::NonZero, 2., 0., false).unwrap();
        assert!(!lines.is_empty());
        for line in &lines {
            let pts = line.flatten().unwrap();
            for p in pts {
                // No hatch geometry should sit deep inside the hole.
                assert!(
                    !(p[0] > 7. && p[0] < 13. && p[1] > 7. && p[1] < 13.),
                    "hatch point in hole {p:?}"
                );
            }
        }
    }

    #[test]
    fn stipple_jitter_is_deterministic() {
        use crate::tessellation::FillRule;
        let ring = vec![vec![[0., 0.], [12., 0.], [12., 12.], [0., 12.]]];
        let opts = StippleOptions {
            spacing: 4.,
            size: 0.4,
            kind: StippleKind::Ring,
            jitter: 0.8,
            seed: 42,
            fill_rule: FillRule::NonZero,
        };
        let a = stipple_rings(&ring, &opts).unwrap();
        let b = stipple_rings(&ring, &opts).unwrap();
        assert_eq!(a.len(), b.len());
        assert!(!a.is_empty());
        assert!(a.iter().all(|m| m.closed));
        let starts: Vec<_> = a.iter().map(|m| m.start).collect();
        let starts_b: Vec<_> = b.iter().map(|m| m.start).collect();
        assert_eq!(starts, starts_b);
        let no_jitter = stipple_rings(
            &ring,
            &StippleOptions {
                jitter: 0.,
                kind: StippleKind::Dot,
                ..opts
            },
        )
        .unwrap();
        assert_ne!(
            a.iter().map(|m| m.start).collect::<Vec<_>>(),
            no_jitter.iter().map(|m| m.start).collect::<Vec<_>>()
        );
    }

    #[test]
    fn hatch_path_and_cross() {
        use crate::tessellation::FillRule;
        let outer = BezierPath::from_rect([0., 0.], [10., 8.]).unwrap();
        let hole = BezierPath::from_rect([3., 2.], [7., 6.]).unwrap().reverse();
        let lines = hatch_path(
            &outer,
            &[hole],
            FillRule::NonZero,
            1.5,
            PI * 0.25,
            true,
            0.25,
        )
        .unwrap();
        assert!(lines.len() >= 4);
    }

    #[test]
    fn distortions_and_blend() {
        let p = BezierPath::from_rect([0., 0.], [4., 2.]).unwrap();
        assert!(zig_zag(&p, 0.2, 1.).unwrap().segments.len() >= 3);
        assert!(pucker_bloat(&p, 0.2).unwrap().closed);
        assert!(roughen(&p, 0.1, 1).unwrap().closed);
        assert!(twist(&p, 0.5).unwrap().closed);
        assert_eq!(scatter(&p, 5, 2., 7).unwrap().len(), 5);
        let q = BezierPath::from_circle([2., 1.], 1.).unwrap();
        let b = blend(&p, &q, 3).unwrap();
        assert_eq!(b.len(), 3);
        let d = free_distort(&p, [[0., 0.], [5., -1.], [6., 3.], [-1., 4.]]).unwrap();
        assert!(d.closed);
    }

    #[test]
    fn arc_and_star() {
        let a = arc_path([-1., -1.], [1., 1.], 0., 90., ArcMode::Pie).unwrap();
        assert!(a.closed);
        let s = star([-2., -2.], [2., 2.], 5, 0.4).unwrap();
        assert_eq!(s.anchor_count(), 10);
    }

    #[test]
    fn concentric_offset_rings() {
        let ring = vec![vec![[0., 0.], [10., 0.], [10., 10.], [0., 10.]]];
        let rings = concentric_offset(&ring, 3, 1., "Miter", 8).unwrap();
        assert_eq!(rings.len(), 3);
    }

    #[test]
    fn concentric_offsets_preserve_evenodd_holes_and_reject_invalid_options() {
        use crate::path_offset::OffsetOptions;
        use crate::tessellation::FillRule;
        let rings = vec![
            vec![[0., 0.], [20., 0.], [20., 20.], [0., 20.]],
            vec![[6., 6.], [14., 6.], [14., 14.], [6., 14.]],
        ];
        let options = OffsetOptions {
            distance: 1.,
            fill_rule: FillRule::EvenOdd,
            ..Default::default()
        };
        let result = concentric_offset_with_options(&rings, 2, &options).unwrap();
        assert_eq!(
            result.iter().map(|r| r.len()).collect::<Vec<_>>(),
            vec![2, 2]
        );
        for (i, rings) in result.iter().enumerate() {
            let area: f64 = rings.iter().map(|r| crate::rings::area(r)).sum();
            let d = (i + 1) as f64;
            assert!((area - ((20. + 2. * d).powi(2) - (8. - 2. * d).powi(2))).abs() < 1e-7);
        }
        assert!(
            concentric_offset_with_options(
                &rings,
                2,
                &OffsetOptions {
                    tolerance: 0.,
                    ..options
                }
            )
            .is_err()
        );
        let small = vec![vec![[0., 0.], [4., 0.], [4., 4.], [0., 4.]]];
        assert_eq!(
            concentric_offset_with_options(
                &small,
                4,
                &OffsetOptions {
                    distance: -1.,
                    ..options
                }
            )
            .unwrap()
            .len(),
            1
        );
    }
}
