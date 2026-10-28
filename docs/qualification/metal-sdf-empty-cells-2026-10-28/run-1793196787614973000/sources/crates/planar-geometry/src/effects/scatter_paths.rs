use super::*;

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

pub(super) fn selection_bbox(paths: &[BezierPath], copies: usize) -> Result<([f64; 2], [f64; 2])> {
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

pub(super) fn to_cubics(path: &BezierPath) -> Vec<Cubic4> {
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

pub(super) fn split_cubic(c: Cubic4, t: f64) -> (Cubic4, Cubic4) {
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
