use super::*;

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

pub(super) fn stipple_mark(center: [f64; 2], size: f64, kind: StippleKind) -> Result<BezierPath> {
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

pub(super) const HATCH_MAX_LINES: usize = 4_000;
pub(super) const HATCH_MAX_EDGES: usize = 100_000;
pub(super) const HATCH_MAX_LINE_EDGE_TESTS: usize = 8_000_000;
pub(super) const HATCH_MAX_SPANS: usize = 100_000;
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

pub(super) fn validate_effect_path(path: &BezierPath) -> Result<()> {
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
pub(super) fn curvex_noise(seed: u64) -> f64 {
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
