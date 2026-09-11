//! Generators and planar distortions for Bézier / polyline paths.
//!
//! Ported from Curvex path effects (MIT OR Apache-2.0), binary64 for polygon-core.
use crate::path::{BezierPath, PathSegment};
use crate::{check, Result};

const TAU: f64 = std::f64::consts::TAU;
const PI: f64 = std::f64::consts::PI;
const ELLIPSE_KAPPA: f64 = 0.552_285;
const MAX_COPIES: usize = 256;
const MAX_SAMPLES: usize = 4096;

fn lerp(a: [f64; 2], b: [f64; 2], t: f64) -> [f64; 2] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

fn add(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] + b[0], a[1] + b[1]]
}

fn sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn mul(a: [f64; 2], s: f64) -> [f64; 2] {
    [a[0] * s, a[1] * s]
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
    bbox_of(&path.flatten()?)
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
    map_path_points(path, |p| add(p, d))
}

fn rotate_about(p: [f64; 2], pivot: [f64; 2], radians: f64) -> [f64; 2] {
    let c = radians.cos();
    let s = radians.sin();
    let v = sub(p, pivot);
    add(pivot, [v[0] * c - v[1] * s, v[0] * s + v[1] * c])
}

fn deterministic_noise(seed: u64, i: usize) -> f64 {
    let mut x = seed
        .wrapping_add(i as u64)
        .wrapping_mul(0x9e37_79b9_7f4a_7c15);
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^= x >> 31;
    (x as f64) / (u64::MAX as f64) * 2.0 - 1.0
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
    check((1..=64).contains(&count), "Concentric count must be 1..=64")?;
    check(
        step.is_finite() && step.abs() > 1e-9,
        "Invalid concentric step",
    )?;
    let mut out = Vec::with_capacity(count);
    for k in 1..=count {
        let d = step * k as f64;
        match crate::rings::offset_join(rings, d, join, segments) {
            Ok(r) if !r.is_empty() => out.push(r),
            Ok(_) => {}
            Err(_) => {} // collapsed inset — skip this ring
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
        .map(|i| translate_path(path, mul(delta, i as f64)))
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

/// Scanline hatch segments clipped to a closed ring (even-odd).
pub fn hatch(
    ring: &[[f64; 2]],
    spacing: f64,
    angle_rad: f64,
    cross: bool,
) -> Result<Vec<BezierPath>> {
    check(ring.len() >= 3, "Hatch needs a closed ring")?;
    check(
        spacing > 1e-6 && spacing.is_finite(),
        "Invalid hatch spacing",
    )?;
    let mut lines = hatch_dir(ring, spacing, angle_rad)?;
    if cross {
        lines.extend(hatch_dir(ring, spacing, angle_rad + PI * 0.5)?);
    }
    Ok(lines)
}

fn hatch_dir(ring: &[[f64; 2]], spacing: f64, angle: f64) -> Result<Vec<BezierPath>> {
    let (min, max) = bbox_of(ring)?;
    let c = angle.cos();
    let s = angle.sin();
    // Local frame: x' along hatch, y' across.
    let corners = [
        [min[0], min[1]],
        [max[0], min[1]],
        [max[0], max[1]],
        [min[0], max[1]],
    ];
    let mut y_min = f64::INFINITY;
    let mut y_max = f64::NEG_INFINITY;
    let mut x_min = f64::INFINITY;
    let mut x_max = f64::NEG_INFINITY;
    for p in corners {
        let x = p[0] * c + p[1] * s;
        let y = -p[0] * s + p[1] * c;
        x_min = x_min.min(x);
        x_max = x_max.max(x);
        y_min = y_min.min(y);
        y_max = y_max.max(y);
    }
    let mut out = Vec::new();
    let mut y = y_min;
    let mut guard = 0;
    while y <= y_max + 1e-9 {
        check(guard < MAX_SAMPLES, "Hatch line budget exceeded")?;
        guard += 1;
        // Ray across bbox in world space.
        let a = [x_min * c - y * s, x_min * s + y * c];
        let b = [x_max * c - y * s, x_max * s + y * c];
        let mut ts = clip_segment_to_ring(a, b, ring);
        ts.sort_by(f64::total_cmp);
        for pair in ts.chunks_exact(2) {
            let p0 = lerp(a, b, pair[0]);
            let p1 = lerp(a, b, pair[1]);
            if (p0[0] - p1[0]).hypot(p0[1] - p1[1]) > 1e-9 {
                out.push(BezierPath::from_polyline(&[p0, p1], false)?);
            }
        }
        y += spacing;
    }
    Ok(out)
}

fn clip_segment_to_ring(a: [f64; 2], b: [f64; 2], ring: &[[f64; 2]]) -> Vec<f64> {
    use crate::rings::contains_point;
    let mut ts = vec![0.0, 1.0];
    for i in 0..ring.len() {
        let c = ring[i];
        let d = ring[(i + 1) % ring.len()];
        if let Some(t) = seg_intersect_t(a, b, c, d) {
            if t > 1e-9 && t < 1.0 - 1e-9 {
                ts.push(t);
            }
        }
    }
    ts.sort_by(f64::total_cmp);
    ts.dedup_by(|x, y| (*x - *y).abs() < 1e-10);
    let mut keep = Vec::new();
    for w in ts.windows(2) {
        let mid = (w[0] + w[1]) * 0.5;
        let p = lerp(a, b, mid);
        if contains_point(p, ring) {
            keep.push(w[0]);
            keep.push(w[1]);
        }
    }
    keep
}

fn seg_intersect_t(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> Option<f64> {
    let r = sub(b, a);
    let s = sub(d, c);
    let den = r[0] * s[1] - r[1] * s[0];
    if den.abs() < 1e-15 {
        return None;
    }
    let qp = sub(c, a);
    let t = (qp[0] * s[1] - qp[1] * s[0]) / den;
    let u = (qp[0] * r[1] - qp[1] * r[0]) / den;
    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
        Some(t)
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StippleKind {
    Dot,
    Ring,
    Cross,
    Square,
}

/// Regular stipple marks over a closed ring's bbox, kept if center is inside.
pub fn stipple(
    ring: &[[f64; 2]],
    spacing: f64,
    size: f64,
    kind: StippleKind,
) -> Result<Vec<BezierPath>> {
    use crate::rings::contains_point;
    check(spacing > 1e-6 && size > 1e-6, "Invalid stipple params")?;
    let (min, max) = bbox_of(ring)?;
    let mut out = Vec::new();
    let mut y = min[1];
    let mut guard = 0;
    while y <= max[1] + 1e-9 {
        let mut x = min[0];
        while x <= max[0] + 1e-9 {
            check(guard < MAX_SAMPLES, "Stipple budget exceeded")?;
            guard += 1;
            let p = [x, y];
            if contains_point(p, ring) {
                out.push(stipple_mark(p, size, kind)?);
            }
            x += spacing;
        }
        y += spacing;
    }
    Ok(out)
}

fn stipple_mark(center: [f64; 2], size: f64, kind: StippleKind) -> Result<BezierPath> {
    let h = size * 0.5;
    match kind {
        StippleKind::Dot | StippleKind::Ring => BezierPath::from_circle(center, h),
        StippleKind::Square => BezierPath::from_rect(
            [center[0] - h, center[1] - h],
            [center[0] + h, center[1] + h],
        ),
        StippleKind::Cross => {
            // Represent as a plus of two short open segments joined by a tiny compound polyline.
            BezierPath::from_polyline(
                &[
                    [center[0] - h, center[1]],
                    [center[0] + h, center[1]],
                    [center[0], center[1]],
                    [center[0], center[1] - h],
                    [center[0], center[1] + h],
                ],
                false,
            )
        }
    }
}

// ----- Distortions -----

pub fn zig_zag(path: &BezierPath, amplitude: f64, wavelength: f64) -> Result<BezierPath> {
    check(
        amplitude.is_finite() && wavelength > 1e-6,
        "Invalid zig-zag params",
    )?;
    let pts = path.flatten()?;
    check(pts.len() >= 2, "Path too short")?;
    let mut out = Vec::with_capacity(pts.len());
    let mut dist_acc = 0.0;
    out.push(pts[0]);
    for i in 1..pts.len() {
        let d = sub(pts[i], pts[i - 1]);
        let len = d[0].hypot(d[1]).max(1e-12);
        let n = [-d[1] / len, d[0] / len];
        dist_acc += len;
        let phase = (dist_acc / wavelength) * TAU;
        let offset = n[0] * amplitude * phase.sin();
        let offset_y = n[1] * amplitude * phase.sin();
        out.push([pts[i][0] + offset, pts[i][1] + offset_y]);
    }
    BezierPath::from_polyline(&out, path.closed)
}

pub fn pucker_bloat(path: &BezierPath, amount: f64) -> Result<BezierPath> {
    check(amount.is_finite() && amount.abs() <= 1.0, "Invalid amount")?;
    let (min, max) = path_bbox(path)?;
    let center = [(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5];
    Ok(map_path_points(path, |p| {
        let v = sub(p, center);
        add(center, mul(v, 1.0 + amount))
    }))
}

pub fn roughen(path: &BezierPath, amount: f64, seed: u64) -> Result<BezierPath> {
    check(amount.is_finite() && amount >= 0., "Invalid roughen amount")?;
    let mut i = 0usize;
    Ok(map_path_points(path, |p| {
        let nx = deterministic_noise(seed, i);
        let ny = deterministic_noise(seed ^ 0xdead_beef, i);
        i += 1;
        [p[0] + nx * amount, p[1] + ny * amount]
    }))
}

pub fn twist(path: &BezierPath, radians: f64) -> Result<BezierPath> {
    check(radians.is_finite(), "Invalid twist angle")?;
    let (min, max) = path_bbox(path)?;
    let center = [(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5];
    let extent = ((max[0] - min[0]).hypot(max[1] - min[1])).max(1e-9);
    Ok(map_path_points(path, |p| {
        let r = sub(p, center)[0].hypot(sub(p, center)[1]) / extent;
        rotate_about(p, center, radians * r)
    }))
}

pub fn scatter(path: &BezierPath, count: usize, radius: f64, seed: u64) -> Result<Vec<BezierPath>> {
    check((1..=MAX_COPIES).contains(&count), "Invalid scatter count")?;
    check(radius >= 0. && radius.is_finite(), "Invalid scatter radius")?;
    let (min, max) = path_bbox(path)?;
    let _ = (min, max);
    Ok((0..count)
        .map(|i| {
            let offset = [
                deterministic_noise(seed, i * 2) * radius,
                deterministic_noise(seed, i * 2 + 1) * radius,
            ];
            translate_path(path, offset)
        })
        .collect())
}

/// Blend two paths into `steps` intermediate open/closed paths (geometry only).
pub fn blend(a: &BezierPath, b: &BezierPath, steps: usize) -> Result<Vec<BezierPath>> {
    check((1..=64).contains(&steps), "Blend steps must be 1..=64")?;
    check(a.closed == b.closed, "Blend requires matching closedness")?;
    let ca = to_cubics(a);
    let cb = to_cubics(b);
    let (ca, cb) = resample_to_same_len(ca, cb)?;
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
    let w = (max[0] - min[0]).max(1e-9);
    let h = (max[1] - min[1]).max(1e-9);
    Ok(map_path_points(path, |p| {
        let u = (p[0] - min[0]) / w;
        let v = (p[1] - min[1]) / h;
        // bilinar: (1-u)(1-v) TL + u(1-v) TR + u v BR + (1-u) v BL
        let [tl, tr, br, bl] = quad;
        add(
            add(mul(tl, (1.0 - u) * (1.0 - v)), mul(tr, u * (1.0 - v))),
            add(mul(br, u * v), mul(bl, (1.0 - u) * v)),
        )
    }))
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
        start_deg.is_finite() && sweep_deg.is_finite() && sweep_deg.abs() >= 0.1,
        "Invalid arc angles",
    )?;
    let center = [(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5];
    let rx = ((max[0] - min[0]).abs() * 0.5).max(1e-6);
    let ry = ((max[1] - min[1]).abs() * 0.5).max(1e-6);
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
        // Approximate arc sector with one cubic (kappa scaled by sweep fraction of 90°).
        let k = ELLIPSE_KAPPA * ((a1 - a0).abs() / (PI * 0.5)).min(1.0);
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
            segs.push(PathSegment::Line { to: center });
            segs.push(PathSegment::Line { to: p0 });
            BezierPath::closed(p0, segs)
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
}
