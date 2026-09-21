//! Analytic sphere/sphere intersection for canonical stereographic sphere solids.
//!
//! Both operands must be exact canonical spheres as built by `analytic::sphere`
//! (eight regular rational biquadratic stereographic patches over ordinary pole
//! vertices, quarter-disk UV trims), optionally carried through a rigid affine
//! placement. Anything else is an explicit `UnsupportedSurface` region — this
//! cell never falls back to numerical surface/surface subdivision. Center
//! classification uses outward binary64 interval arithmetic: separate and
//! contained-without-contact pairs resolve empty, concentric equal (and
//! concentric near-equal within the recognition band) pairs report a
//! `CoincidentTrim` region, and every tangency or within-error tangency band
//! reports `TangencyOrMultipleRoot` — tangent contacts are never reported as
//! point components, matching the numerical queries' tangency discipline.
//! A transverse pair yields the exact rational circle (four 90-degree arcs,
//! weights cos(pi/4)) plus, for each sphere, the circle's exact per-patch UV
//! lifts: planar sections of a stereographic patch are exact circles (or lines
//! through the UV origin when the plane contains the patch pole) clipped to
//! the patch quarter-disk. Nothing here authorizes a topology change.
use super::*;
use crate::Model;

/// Recognition tolerance relative to radius for the canonical structure
/// screen (vertex distances, frame orthonormality, control points).
pub(crate) const RECOGNITION: f64 = 1e-9;
/// Relative scale under which the UV circle degenerates to a UV line (the
/// section plane contains the patch pole, e.g. a great circle whose plane is
/// perpendicular to the sphere axis).
const UV_LINE: f64 = 1e-12;
const TAU: f64 = std::f64::consts::TAU;
const LINEAR: [f64; 3] = [0., 0.5, 1.];
const SQUARE: [f64; 3] = [0., 0., 1.];
pub(crate) const ARC_WEIGHT: f64 = std::f64::consts::FRAC_1_SQRT_2;

/// One sphere patch's share of the intersection circle in that patch's UV.
#[derive(Clone, Debug)]
pub struct SpherePatchCircle {
    /// Face index in the source model.
    pub patch: usize,
    /// Exact 2D rational quadratic arcs (or one degree-1 segment) clipped to
    /// the patch quarter-disk, each with knots [0,0,0,1,1,1] or [0,0,1,1].
    pub arcs: Vec<Curve>,
}

#[derive(Clone, Debug)]
pub enum SphereSphereComponent {
    /// Transverse intersection: the exact circle plus its UV lifts.
    Circle {
        /// Exact rational full circle: degree 2, knots 0..=4, four 90-degree
        /// arcs with weights cos(pi/4).
        curve: Curve,
        center: [f64; 3],
        radius: f64,
        /// Unit sphere-center axis from the first sphere toward the second.
        normal: [f64; 3],
        first_uv: Vec<SpherePatchCircle>,
        second_uv: Vec<SpherePatchCircle>,
        /// Worst |.|p|-r|. over 16 circle samples against both spheres.
        max_sample_residual: f64,
    },
}

#[derive(Clone, Copy, Debug)]
struct PatchFrame {
    a: [f64; 3],
    b: [f64; 3],
    /// Signed unit vector toward this patch's pole.
    pole: [f64; 3],
}

#[derive(Clone, Debug)]
pub(crate) struct CanonicalSphere {
    pub(crate) center: [f64; 3],
    pub(crate) radius: f64,
    /// Observed structural deviation bound; feeds the outward classification.
    pub(crate) error: f64,
    patches: Vec<PatchFrame>,
}

pub(crate) fn unit_quarter_arc(curve: &Curve) -> bool {
    curve.degree == 2
        && curve.knots == [0., 0., 0., 1., 1., 1.]
        && curve.control_points == [[1., 0.].to_vec(), [1., 1.].to_vec(), [0., 1.].to_vec()]
        && curve.weights == [1., ARC_WEIGHT, 1.]
}
pub(crate) fn axis_line(curve: &Curve, from: [f64; 2], to: [f64; 2]) -> bool {
    curve.degree == 1
        && curve.knots == [0., 0., 1., 1.]
        && curve.control_points == [from.to_vec(), to.to_vec()]
        && curve.weights == [1., 1.]
}

/// Recognizes a canonical stereographic sphere solid, certifying every patch
/// control point and weight against the exact construction in its recovered
/// frame, the exact quarter-disk trim pcurves, and a globally consistent
/// pole/quadrant tiling (each (hemisphere, quadrant) pair exactly once).
pub(crate) fn recognize(model: &Model) -> Result<Option<CanonicalSphere>> {
    model.validate()?;
    if model.bodies.len() != 1
        || model.shells.len() != 1
        || model.faces.len() != 8
        || model.vertices.len() != 6
    {
        return Ok(None);
    }
    let center = {
        let mut c = [0.; 3];
        for vertex in &model.vertices {
            for axis in 0..3 {
                c[axis] += vertex.point[axis] / 6.;
            }
        }
        c
    };
    let mut radius = 0.;
    for vertex in &model.vertices {
        let d = sub(vertex.point, center);
        radius += d[0].hypot(d[1]).hypot(d[2]) / 6.;
    }
    if !radius.is_finite() || !(1e-5..=1e6).contains(&radius) {
        return Ok(None);
    }
    let mut error: f64 = 0.;
    for vertex in &model.vertices {
        let d = sub(vertex.point, center);
        error = error.max((d[0].hypot(d[1]).hypot(d[2]) - radius).abs());
    }
    if error > RECOGNITION * radius {
        return Ok(None);
    }
    let mut frames = Vec::with_capacity(8);
    for face in &model.faces {
        let surface = &face.surface;
        if surface.degree_u != 2
            || surface.degree_v != 2
            || surface.periodic_u
            || surface.periodic_v
            || surface.knots_u != [0., 0., 0., 1., 1., 1.]
            || surface.knots_v != [0., 0., 0., 1., 1., 1.]
            || surface.control_points.len() != 3
            || surface.control_points.iter().any(|row| row.len() != 3)
            || surface.weights.len() != 3
            || surface.weights.iter().any(|row| row.len() != 3)
            || !face.holes.is_empty()
        {
            return Ok(None);
        }
        for (i, row) in surface.weights.iter().enumerate() {
            for (j, w) in row.iter().enumerate() {
                if *w != 1. + SQUARE[i] + SQUARE[j] {
                    return Ok(None);
                }
            }
        }
        let loop_ = &model.loops[face.outer];
        if loop_.coedges.len() != 3 {
            return Ok(None);
        }
        let mut trims = [false; 3];
        for coedge in &loop_.coedges {
            let pcurve = &coedge.pcurve;
            if unit_quarter_arc(pcurve) {
                trims[0] = true;
            } else if axis_line(pcurve, [0., 0.], [1., 0.]) {
                trims[1] = true;
            } else if axis_line(pcurve, [0., 1.], [0., 0.]) {
                trims[2] = true;
            }
        }
        if !trims.into_iter().all(|hit| hit) {
            return Ok(None);
        }
        let pole_point = surface.evaluate(0., 0.)?.point;
        let a_point = surface.evaluate(1., 0.)?.point;
        let b_point = surface.evaluate(0., 1.)?.point;
        let pole = sub(pole_point, center).map(|x| x / radius);
        let a = sub(a_point, center).map(|x| x / radius);
        let b = sub(b_point, center).map(|x| x / radius);
        for (x, y) in [(pole, a), (pole, b), (a, b)] {
            if dot(x, y).abs() > RECOGNITION {
                return Ok(None);
            }
        }
        for x in [pole, a, b] {
            if (dot(x, x) - 1.).abs() > RECOGNITION {
                return Ok(None);
            }
        }
        for i in 0..3 {
            for j in 0..3 {
                let w = 1. + SQUARE[i] + SQUARE[j];
                let expected: [f64; 3] = std::array::from_fn(|axis| {
                    center[axis]
                        + (2. * radius * (LINEAR[i] * a[axis] + LINEAR[j] * b[axis])
                            + radius * (1. - SQUARE[i] - SQUARE[j]) * pole[axis])
                            / w
                });
                let actual = &surface.control_points[i][j];
                if actual.len() != 3 {
                    return Ok(None);
                }
                let deviation = (0..3).map(|k| actual[k] - expected[k]).collect::<Vec<_>>();
                let deviation = deviation[0].hypot(deviation[1]).hypot(deviation[2]);
                if !deviation.is_finite() || deviation > RECOGNITION * radius + 1e-12 {
                    return Ok(None);
                }
                error = error.max(deviation);
            }
        }
        frames.push(PatchFrame { a, b, pole });
    }
    // Global frame: poles must be +/- one axis, four patches per hemisphere;
    // equator bases must tile the four quadrants consistently in both.
    let reference = frames[0].pole;
    let mut axis = [0.; 3];
    let mut north = 0;
    for frame in &frames {
        let alignment = dot(frame.pole, reference);
        if alignment.abs() < 1. - RECOGNITION {
            return Ok(None);
        }
        let sign = alignment.signum();
        north += (sign > 0.) as usize;
        for k in 0..3 {
            axis[k] += sign * frame.pole[k] / 8.;
        }
    }
    if north != 4 {
        return Ok(None);
    }
    let axis_length = dot(axis, axis).sqrt();
    if !(axis_length > 0.) {
        return Ok(None);
    }
    let axis = axis.map(|x| x / axis_length);
    // X from any equator basis direction projected off the axis.
    let seed = frames[0].a;
    let axial = dot(seed, axis);
    let x_dir = sub(seed, axis.map(|x| x * axial));
    let x_length = dot(x_dir, x_dir).sqrt();
    if !(x_length > 0.) {
        return Ok(None);
    }
    let x_dir = x_dir.map(|x| x / x_length);
    let y_dir = cross(axis, x_dir);
    let quarter = std::f64::consts::FRAC_PI_2;
    let mut seen = [false; 8];
    let mut snapped = Vec::with_capacity(8);
    for frame in &frames {
        let hemisphere = dot(frame.pole, axis).signum();
        let angle = dot(frame.a, y_dir).atan2(dot(frame.a, x_dir));
        let quadrant = (angle / quarter).round() as i64;
        let quadrant = quadrant.rem_euclid(4) as usize;
        let residual = (angle - quadrant as f64 * quarter + std::f64::consts::PI).rem_euclid(TAU)
            - std::f64::consts::PI;
        if residual.abs() > RECOGNITION {
            return Ok(None);
        }
        let slot = if hemisphere > 0. {
            quadrant
        } else {
            4 + quadrant
        };
        if seen[slot] {
            return Ok(None);
        }
        seen[slot] = true;
        let direction = |q: usize| {
            let (sin, cos) = (q as f64 * quarter).sin_cos();
            std::array::from_fn(|k| cos * x_dir[k] + sin * y_dir[k])
        };
        let snapped_a = direction(quadrant);
        let snapped_b = direction((quadrant + 1) % 4);
        if dot(snapped_b, frame.b) < 1. - RECOGNITION {
            return Ok(None);
        }
        snapped.push(PatchFrame {
            a: snapped_a,
            b: snapped_b,
            pole: axis.map(|x| x * hemisphere),
        });
    }
    if seen.into_iter().any(|hit| !hit) {
        return Ok(None);
    }
    Ok(Some(CanonicalSphere {
        center,
        radius,
        error,
        patches: snapped,
    }))
}

impl CanonicalSphere {
    /// Snapped exact frame of one patch: (equator basis a, equator basis b,
    /// signed pole). Patch indices are source model face indices.
    pub(crate) fn frame(&self, patch: usize) -> ([f64; 3], [f64; 3], [f64; 3]) {
        let frame = &self.patches[patch];
        (frame.a, frame.b, frame.pole)
    }
    /// Exact rational UV inversion of a 3D point on one patch; None at or
    /// beyond the patch antipode. Uses the snapped certified frame, so the
    /// residual against the authored surface is bounded by `self.error`.
    pub(crate) fn invert_uv(&self, patch: usize, point: [f64; 3]) -> Option<[f64; 2]> {
        let uv = invert_patch(self, &self.patches[patch], point);
        (uv[0].abs() < 1e29 && uv[1].abs() < 1e29).then_some(uv)
    }
}

/// Exact 3D circle: four 90-degree rational arcs, knots 0..=4, weights cos(pi/4).
pub(crate) fn circle_curve(center: [f64; 3], radius: f64, e1: [f64; 3], e2: [f64; 3]) -> Curve {
    let point = |x: f64, y: f64| {
        std::array::from_fn::<f64, 3, _>(|k| center[k] + radius * (x * e1[k] + y * e2[k])).to_vec()
    };
    let endpoints = [[1., 0.], [0., 1.], [-1., 0.], [0., -1.], [1., 0.]];
    let mid_signs = [[1., 1.], [-1., 1.], [-1., -1.], [1., -1.]];
    let mut control_points = Vec::with_capacity(9);
    let mut weights = Vec::with_capacity(9);
    for arc in 0..4 {
        if arc == 0 {
            control_points.push(point(endpoints[0][0], endpoints[0][1]));
            weights.push(1.);
        }
        // The tangent-intersection control of a 90-degree arc with weight
        // cos(pi/4) sits at radius/cos(pi/4) along (cos45, sin45): distance
        // radius*sqrt(2)*(sqrt(2)/2) = radius along (+-e1+-e2).
        control_points.push(point(mid_signs[arc][0], mid_signs[arc][1]));
        weights.push(ARC_WEIGHT);
        control_points.push(point(endpoints[arc + 1][0], endpoints[arc + 1][1]));
        weights.push(1.);
    }
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.],
        control_points,
        weights,
        periodic: false,
    }
}

/// Quarter-disk membership of a UV point with a small closure margin.
fn in_quarter_disk(p: [f64; 2]) -> bool {
    const MARGIN: f64 = 1e-12;
    p[0] >= -MARGIN && p[1] >= -MARGIN && p[0] * p[0] + p[1] * p[1] <= 1. + MARGIN
}

/// Swept angle interval of a UV circle inside the quarter disk. The quarter
/// disk is convex, so the inside set is one arc; tangent-touch splits merge.
fn clip_circle(center: [f64; 2], rho: f64) -> Option<(f64, f64)> {
    if !rho.is_finite() || rho <= 0. || !center.iter().all(|v| v.is_finite()) {
        return None;
    }
    let point = |phi: f64| [center[0] + rho * phi.cos(), center[1] + rho * phi.sin()];
    let mut cuts = Vec::new();
    let mut push = |phi: f64| cuts.push(phi.rem_euclid(TAU));
    let x = -center[0] / rho;
    if x.abs() <= 1. + 1e-12 {
        let alpha = x.clamp(-1., 1.).acos();
        push(alpha);
        push(-alpha);
    }
    let y = -center[1] / rho;
    if y.abs() <= 1. + 1e-12 {
        let alpha = y.clamp(-1., 1.).asin();
        push(alpha);
        push(std::f64::consts::PI - alpha);
    }
    let q2 = center[0] * center[0] + center[1] * center[1];
    if q2 > 0. {
        let q = q2.sqrt();
        let nu = (1. - rho * rho - q2) / (2. * rho * q);
        if nu.abs() <= 1. + 1e-12 {
            let base = center[1].atan2(center[0]);
            let delta = nu.clamp(-1., 1.).acos();
            push(base + delta);
            push(base - delta);
        }
    }
    cuts.sort_by(f64::total_cmp);
    cuts.dedup_by(|a, b| (*a - *b).abs() <= 1e-12);
    if cuts.is_empty() {
        return if in_quarter_disk(point(0.)) {
            Some((0., TAU))
        } else {
            None
        };
    }
    let mut spans: Vec<(f64, f64)> = Vec::new();
    for i in 0..cuts.len() {
        let start = cuts[i];
        let end = if i + 1 < cuts.len() {
            cuts[i + 1]
        } else {
            cuts[0] + TAU
        };
        if end - start > 1e-14 && in_quarter_disk(point((start + end) / 2.)) {
            spans.push((start, end));
        }
    }
    if spans.is_empty() {
        return None;
    }
    // Merge spans separated only by touch-scale gaps, across the wraparound.
    let mut merged = vec![spans[0]];
    for &span in &spans[1..] {
        let last = merged.last_mut().unwrap();
        if span.0 - last.1 <= 1e-9 {
            last.1 = span.1;
        } else {
            merged.push(span);
        }
    }
    if merged.len() > 1 && merged[0].0 + TAU - merged.last().unwrap().1 <= 1e-9 {
        let first = merged.remove(0);
        let last = merged.last_mut().unwrap();
        // The merged span wraps the angle origin: keep the swept-interval
        // contract (ccw, end - start in (0, TAU]) by unwrapping the end past
        // TAU instead of emitting a negative sweep (which circle_arcs would
        // turn into a complementary negative-weight arc Curve::evaluate
        // rejects).
        last.1 = first.1 + TAU;
    }
    if merged.len() != 1 {
        return None;
    }
    Some(merged[0])
}

/// Segment of the UV line `normal.p + offset = 0` inside the quarter disk.
fn clip_line(normal: [f64; 2], offset: f64) -> Option<([f64; 2], [f64; 2])> {
    let n2 = normal[0] * normal[0] + normal[1] * normal[1];
    if !n2.is_finite() || n2 <= 0. || !offset.is_finite() {
        return None;
    }
    let length = n2.sqrt();
    let foot = [-normal[0] * offset / n2, -normal[1] * offset / n2];
    let dir = [-normal[1] / length, normal[0] / length];
    let mut lo = f64::NEG_INFINITY;
    let mut hi = f64::INFINITY;
    for axis in 0..2 {
        if dir[axis] > 0. {
            lo = lo.max(-foot[axis] / dir[axis]);
        } else if dir[axis] < 0. {
            hi = hi.min(-foot[axis] / dir[axis]);
        } else if foot[axis] < 0. {
            return None;
        }
    }
    let along = foot[0] * dir[0] + foot[1] * dir[1];
    let disc = along * along - (foot[0] * foot[0] + foot[1] * foot[1] - 1.);
    if disc < 0. {
        return None;
    }
    let root = disc.sqrt();
    lo = lo.max(-along - root);
    hi = hi.min(-along + root);
    if !lo.is_finite() || !hi.is_finite() || hi - lo <= 1e-12 {
        return None;
    }
    Some((
        [foot[0] + lo * dir[0], foot[1] + lo * dir[1]],
        [foot[0] + hi * dir[0], foot[1] + hi * dir[1]],
    ))
}

/// Exact rational quadratic arcs covering the swept interval, each <= 90 degrees.
pub(crate) fn circle_arcs(center: [f64; 2], rho: f64, start: f64, end: f64) -> Vec<Curve> {
    let pieces = ((end - start) / std::f64::consts::FRAC_PI_2).ceil().max(1.) as usize;
    (0..pieces)
        .map(|i| {
            let a0 = start + (end - start) * i as f64 / pieces as f64;
            let a1 = start + (end - start) * (i + 1) as f64 / pieces as f64;
            let half = (a1 - a0) / 2.;
            let weight = half.cos();
            let point =
                |phi: f64| [center[0] + rho * phi.cos(), center[1] + rho * phi.sin()].to_vec();
            let middle = (a0 + a1) / 2.;
            let shoulder = [
                center[0] + rho / weight * middle.cos(),
                center[1] + rho / weight * middle.sin(),
            ]
            .to_vec();
            Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: vec![point(a0), shoulder, point(a1)],
                weights: vec![1., weight, 1.],
                periodic: false,
            }
        })
        .collect()
}

/// One patch's UV section of a plane cut, already clipped to the patch
/// quarter disk: either a UV circle with its swept interval (ccw from
/// `start` to `end`, `end - start` in (0, TAU]) or a UV line segment.
#[derive(Clone, Copy, Debug)]
pub(crate) enum PatchUvSection {
    Circle {
        center: [f64; 2],
        radius: f64,
        start: f64,
        end: f64,
    },
    Line {
        start: [f64; 2],
        end: [f64; 2],
    },
}

/// Per-patch UV sections of the plane `n.x = n.m`, exposed with their
/// quarter-disk spans so callers can sub-clip them further. A plane section
/// of the stereographic patch `S(u,v) = c + (2r(ua+vb) + r(1-u^2-v^2)p) /
/// (1+u^2+v^2)` under `n.x = n.m` is `A(u^2+v^2) - 2Bu - 2Cv - D = 0` with
/// `A = r(n.p) + k`, `B = r(n.a)`, `C = r(n.b)`, `D = r(n.p) - k`,
/// `k = n.(m - c)`: an exact UV circle, or a UV line when A vanishes.
pub(crate) fn patch_sections(
    sphere: &CanonicalSphere,
    normal: [f64; 3],
    middle: [f64; 3],
) -> Vec<(usize, PatchUvSection)> {
    let k = dot(normal, sub(middle, sphere.center));
    let mut sections = Vec::new();
    for (patch, frame) in sphere.patches.iter().enumerate() {
        let na = dot(normal, frame.a);
        let nb = dot(normal, frame.b);
        let np = dot(normal, frame.pole);
        let aa = sphere.radius * np + k;
        let bb = sphere.radius * na;
        let cc = sphere.radius * nb;
        let dd = sphere.radius * np - k;
        let scale = sphere.radius + k.abs();
        let section = if aa.abs() <= UV_LINE * scale {
            clip_line([2. * bb, 2. * cc], dd)
                .map(|(start, end)| PatchUvSection::Line { start, end })
        } else {
            let center = [bb / aa, cc / aa];
            let rho2 = (bb * bb + cc * cc + dd * aa) / (aa * aa);
            if !(rho2 > 0.) {
                None
            } else {
                clip_circle(center, rho2.sqrt()).map(|(start, end)| PatchUvSection::Circle {
                    center,
                    radius: rho2.sqrt(),
                    start,
                    end,
                })
            }
        };
        if let Some(section) = section {
            sections.push((patch, section));
        }
    }
    sections
}

/// Lift the section circle into every patch UV of one sphere: the exact
/// per-patch UV circles (or lines through the UV origin when the plane
/// contains the patch pole) clipped to the patch quarter-disk.
pub(crate) fn lift(
    sphere: &CanonicalSphere,
    normal: [f64; 3],
    middle: [f64; 3],
) -> Vec<SpherePatchCircle> {
    patch_sections(sphere, normal, middle)
        .into_iter()
        .map(|(patch, section)| {
            let arcs = match section {
                PatchUvSection::Circle {
                    center,
                    radius,
                    start,
                    end,
                } => circle_arcs(center, radius, start, end),
                PatchUvSection::Line { start, end } => vec![Curve {
                    degree: 1,
                    knots: vec![0., 0., 1., 1.],
                    control_points: vec![start.to_vec(), end.to_vec()],
                    weights: vec![1., 1.],
                    periodic: false,
                }],
            };
            SpherePatchCircle { patch, arcs }
        })
        .collect()
}

/// Intersection of two ccw arcs `(start, sweep)` on the circle, `start` in
/// `[0, TAU)`, `sweep` in `(0, TAU)`. Returns up to two components as
/// `(unwrapped_start, sweep)` pairs; a full-circle argument returns the
/// other arc unchanged.
pub(crate) fn ccw_intersect(a: (f64, f64), b: (f64, f64)) -> Vec<(f64, f64)> {
    if !(a.1 > 0.) || !(b.1 > 0.) {
        return Vec::new();
    }
    if a.1 >= TAU {
        return vec![b];
    }
    if b.1 >= TAU {
        return vec![a];
    }
    let mut out = Vec::new();
    for k in -1..=1 {
        let b0 = b.0 + k as f64 * TAU;
        let lo = a.0.max(b0);
        let hi = (a.0 + a.1).min(b0 + b.1);
        if hi - lo > 1e-14 {
            out.push((lo, hi - lo));
        }
    }
    out
}

/// Exact rational inversion of the stereographic patch: with `d = p - c`,
/// `u = d.a / (r + d.pole)`, `v = d.b / (r + d.pole)`. Points at or beyond
/// the patch antipode (outside this patch's hemisphere) return a far-away
/// sentinel so downstream span membership tests reject them.
fn invert_patch(sphere: &CanonicalSphere, frame: &PatchFrame, point: [f64; 3]) -> [f64; 2] {
    let d = sub(point, sphere.center);
    let denom = sphere.radius + dot(d, frame.pole);
    if !(denom > UV_LINE * sphere.radius) {
        return [1e30, 1e30];
    }
    [dot(d, frame.a) / denom, dot(d, frame.b) / denom]
}

/// Like `lift`, but further clipped to the given parameter intervals of the
/// exact 3D section circle. `arcs` holds ccw parameter intervals and
/// `point_at(phi)` must evaluate the exact 3D section point at that
/// parameter; the parameterization must be monotonic with the geometric
/// angle of the section circle. Interval endpoints and midpoints are mapped
/// into each patch UV through the exact rational inversion, which fixes both
/// the sub-span trims and the angular direction without any fitting.
pub(crate) fn lift_clipped(
    sphere: &CanonicalSphere,
    normal: [f64; 3],
    middle: [f64; 3],
    arcs: &[(f64, f64)],
    point_at: impl Fn(f64) -> [f64; 3],
) -> Vec<SpherePatchCircle> {
    let mut lifted = Vec::new();
    for (patch, section) in patch_sections(sphere, normal, middle) {
        let frame = &sphere.patches[patch];
        let mut out: Vec<Curve> = Vec::new();
        for &(a, b) in arcs {
            let sweep = b - a;
            if !(sweep > 0.) {
                continue;
            }
            let qa = invert_patch(sphere, frame, point_at(a));
            let qb = invert_patch(sphere, frame, point_at(b));
            let qm = invert_patch(sphere, frame, point_at(a + sweep / 2.));
            match section {
                PatchUvSection::Circle {
                    center,
                    radius,
                    start,
                    end,
                } => {
                    if qa.iter().chain(&qb).chain(&qm).any(|x| x.abs() >= 1e30) {
                        continue;
                    }
                    let psi = |q: [f64; 2]| (q[1] - center[1]).atan2(q[0] - center[0]);
                    let (pa, pb, pm) = (
                        psi(qa).rem_euclid(TAU),
                        psi(qb).rem_euclid(TAU),
                        psi(qm).rem_euclid(TAU),
                    );
                    // The stereographic map is a Möbius map of the section
                    // circle: the image of the parameter interval is one of
                    // the two ccw arcs between the endpoint images; the
                    // midpoint image decides the direction.
                    let forward = (pb - pa).rem_euclid(TAU);
                    let image = if (pm - pa).rem_euclid(TAU) <= forward {
                        (pa, forward)
                    } else {
                        (pb, (pa - pb).rem_euclid(TAU))
                    };
                    let span = (start, (end - start).min(TAU));
                    for (s0, sw) in ccw_intersect(span, image) {
                        // Defensive: the emitted sub-arc midpoint must lie in
                        // the quarter disk (the intersection above is exact,
                        // so this only filters antipode-degenerate cases).
                        let mid_angle = s0 + sw / 2.;
                        let q = [
                            center[0] + radius * mid_angle.cos(),
                            center[1] + radius * mid_angle.sin(),
                        ];
                        if in_quarter_disk(q) {
                            out.extend(circle_arcs(center, radius, s0, s0 + sw));
                        }
                    }
                }
                PatchUvSection::Line { start, end } => {
                    let d = [end[0] - start[0], end[1] - start[1]];
                    let dd = d[0] * d[0] + d[1] * d[1];
                    if !(dd > 0.) {
                        continue;
                    }
                    let t =
                        |q: [f64; 2]| ((q[0] - start[0]) * d[0] + (q[1] - start[1]) * d[1]) / dd;
                    let (ta, tb, tm) = (t(qa), t(qb), t(qm));
                    if !(-1e-9..=1. + 1e-9).contains(&tm) {
                        continue;
                    }
                    let lo = ta.min(tb).max(0.);
                    let hi = ta.max(tb).min(1.);
                    if hi - lo > 1e-12 {
                        let p = |s: f64| [start[0] + s * d[0], start[1] + s * d[1]];
                        out.push(Curve {
                            degree: 1,
                            knots: vec![0., 0., 1., 1.],
                            control_points: vec![p(lo).to_vec(), p(hi).to_vec()],
                            weights: vec![1., 1.],
                            periodic: false,
                        });
                    }
                }
            }
        }
        if !out.is_empty() {
            lifted.push(SpherePatchCircle { patch, arcs: out });
        }
    }
    lifted
}

/// Analytic sphere/sphere intersection of two canonical sphere solids.
/// Non-canonical operands are explicit `UnsupportedSurface` regions, never a
/// numerical fallback; tangency bands and coincident spheres stay unresolved.
pub fn intersect_sphere_sphere(
    first: &Model,
    second: &Model,
    options: Options,
) -> Result<Report<SphereSphereComponent>> {
    let options = options.validate()?;
    let _ = options;
    let mut report = Report::default();
    let domain = vec![0., 1., 0., 1., 0., 1., 0., 1.];
    let (Some(a), Some(b)) = (recognize(first)?, recognize(second)?) else {
        report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    };
    report.boxes_visited = 1;
    // Outward binary64 classification band: recognition deviation plus a
    // rounding allowance on the center difference, its norm, and the radii.
    let delta = sub(b.center, a.center);
    let scale = delta.iter().map(|v| v.abs()).fold(0., f64::max);
    let distance = if scale == 0. {
        0.
    } else {
        let scaled = delta.map(|x| x / scale);
        scaled[0].hypot(scaled[1]).hypot(scaled[2]) * scale
    };
    let band =
        a.error + b.error + 16. * f64::EPSILON * (distance + scale + a.radius + b.radius + 1.);
    let d_lo = (distance - band).max(0.);
    let d_hi = distance + band;
    let sum = a.radius + b.radius;
    let sum_lo = sum - band;
    let sum_hi = sum + band;
    let diff = (a.radius - b.radius).abs();
    let diff_lo = (diff - band).max(0.);
    let diff_hi = diff + band;
    // Separate: the whole distance band clears the radius sum.
    if d_lo > sum_hi {
        return Ok(report);
    }
    // Contained without contact, including concentric unequal radii.
    if d_hi < diff_lo {
        return Ok(report);
    }
    let zero = band.max(64. * f64::EPSILON * sum);
    // Concentric with radii equal within the band: coincident surfaces (or an
    // inseparable near-coincidence), never a fabricated curve.
    if d_hi <= zero && diff_lo <= zero {
        report.unresolved(domain, UnresolvedReason::CoincidentTrim);
        return Ok(report);
    }
    // External or internal tangency, or a classification band straddling one:
    // tangent contacts are never reported as points (house tangency rule).
    if d_hi >= sum_lo || d_lo <= diff_hi {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(report);
    }
    // Transverse pair: the circle exists with clear margin on both sides.
    let normal = delta.map(|x| x / distance);
    let along = (a.radius * a.radius - b.radius * b.radius + distance * distance) / (2. * distance);
    let h2 = a.radius * a.radius - along * along;
    if !h2.is_finite() || h2 <= 0. {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(report);
    }
    let radius = h2.sqrt();
    let center = std::array::from_fn(|i| a.center[i] + along * normal[i]);
    // Circle basis: cross with the coordinate axis least aligned with normal.
    let mut axis = 0;
    for candidate in 1..3 {
        if normal[candidate].abs() < normal[axis].abs() {
            axis = candidate;
        }
    }
    let unit = std::array::from_fn(|i| (i == axis) as u8 as f64);
    let e1 = cross(normal, unit);
    let e1_length = dot(e1, e1).sqrt();
    if !e1_length.is_finite() || e1_length <= 0. {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(report);
    }
    let e1 = e1.map(|x| x / e1_length);
    let e2 = cross(normal, e1);
    let curve = circle_curve(center, radius, e1, e2);
    let first_uv = lift(&a, normal, center);
    let second_uv = lift(&b, normal, center);
    let mut max_sample_residual = 0_f64;
    for i in 0..16 {
        let point = curve.evaluate(i as f64 / 4.)?.point;
        let point: [f64; 3] = [point[0], point[1], point[2]];
        for sphere in [&a, &b] {
            let d = sub(point, sphere.center);
            let residual = (d[0].hypot(d[1]).hypot(d[2]) - sphere.radius).abs();
            max_sample_residual = max_sample_residual.max(residual);
        }
    }
    report.components.push(SphereSphereComponent::Circle {
        curve,
        center,
        radius,
        normal,
        first_uv,
        second_uv,
        max_sample_residual,
    });
    Ok(report)
}

impl value_codec::Serialize for SpherePatchCircle {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({"patch":self.patch,"arcs":self.arcs})
    }
}
impl value_codec::Serialize for SphereSphereComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Circle {
                curve,
                center,
                radius,
                normal,
                first_uv,
                second_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"circle","curve":curve,"center":center,"radius":radius,
                "normal":normal,"firstUv":first_uv,"secondUv":second_uv,
                "maxSampleResidual":max_sample_residual}),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn translated(model: &Model, offset: [f64; 3]) -> Model {
        crate::transform::affine(
            model,
            [
                [1., 0., 0., offset[0]],
                [0., 1., 0., offset[1]],
                [0., 0., 1., offset[2]],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap()
    }
    fn rotated_translated(model: &Model, angle: f64, offset: [f64; 3]) -> Model {
        let (sin, cos) = angle.sin_cos();
        crate::transform::affine(
            model,
            [
                [1., 0., 0., offset[0]],
                [0., cos, -sin, offset[1]],
                [0., sin, cos, offset[2]],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap()
    }
    type SphereSphereCircle<'a> = (
        &'a Curve,
        [f64; 3],
        f64,
        [f64; 3],
        &'a [SpherePatchCircle],
        &'a [SpherePatchCircle],
        f64,
    );

    fn only_circle(report: &Report<SphereSphereComponent>) -> SphereSphereCircle<'_> {
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        assert!(report.unresolved.is_empty(), "{report:?}");
        assert!(!report.permits_topology_change());
        let [
            SphereSphereComponent::Circle {
                curve,
                center,
                radius,
                normal,
                first_uv,
                second_uv,
                max_sample_residual,
            },
        ] = &report.components[..]
        else {
            panic!("expected one circle component: {report:?}")
        };
        (
            curve,
            *center,
            *radius,
            *normal,
            first_uv,
            second_uv,
            *max_sample_residual,
        )
    }
    fn sphere_residual(point: [f64; 3], center: [f64; 3], radius: f64) -> f64 {
        (sub(point, center)[0]
            .hypot(sub(point, center)[1])
            .hypot(sub(point, center)[2])
            - radius)
            .abs()
    }
    /// Worst both-sphere residual over 16 circle samples.
    fn circle_samples(curve: &Curve, c1: [f64; 3], r1: f64, c2: [f64; 3], r2: f64) -> f64 {
        let mut worst = 0_f64;
        for i in 0..16 {
            let p = curve.evaluate(i as f64 / 4.).unwrap().point;
            let p = [p[0], p[1], p[2]];
            worst = worst
                .max(sphere_residual(p, c1, r1))
                .max(sphere_residual(p, c2, r2));
        }
        worst
    }
    /// Worst residual of lifted UV arcs evaluated through their patch surface.
    fn uv_samples(
        model: &Model,
        lifts: &[SpherePatchCircle],
        own: ([f64; 3], f64),
        other: ([f64; 3], f64),
    ) -> f64 {
        let mut worst = 0_f64;
        for lift in lifts {
            let surface = &model.faces[lift.patch].surface;
            for arc in &lift.arcs {
                for k in 0..=8 {
                    let uv = arc.evaluate(k as f64 / 8.).unwrap().point;
                    assert!(uv[0] >= -1e-9 && uv[1] >= -1e-9);
                    assert!(uv[0] * uv[0] + uv[1] * uv[1] <= 1. + 1e-9);
                    let p = surface
                        .evaluate(uv[0].clamp(0., 1.), uv[1].clamp(0., 1.))
                        .unwrap()
                        .point;
                    let p = [p[0], p[1], p[2]];
                    worst = worst
                        .max(sphere_residual(p, own.0, own.1))
                        .max(sphere_residual(p, other.0, other.1));
                }
            }
        }
        worst
    }

    #[test]
    fn equal_spheres_intersect_in_exact_circle() {
        let first = crate::analytic::sphere(2.).unwrap();
        let second = translated(&first, [2., 0., 0.]);
        let report = intersect_sphere_sphere(&first, &second, Options::default()).unwrap();
        let (curve, center, radius, normal, first_uv, second_uv, sampled) = only_circle(&report);
        // Oracle: circle radius sqrt(r^2 - d^2/4) = sqrt(3), center on the axis.
        let oracle = (4_f64 - 1.).sqrt();
        assert!((radius - oracle).abs() <= 1e-12, "{radius} vs {oracle}");
        assert!(
            sub(center, [1., 0., 0.]).iter().all(|x| x.abs() <= 1e-12),
            "{center:?}"
        );
        assert!(
            sub(normal, [1., 0., 0.]).iter().all(|x| x.abs() <= 1e-12),
            "{normal:?}"
        );
        // Exact rational circle: four 90-degree arcs, weights cos(pi/4).
        assert_eq!(curve.degree, 2);
        assert_eq!(
            curve.knots,
            vec![0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.]
        );
        assert_eq!(curve.control_points.len(), 9);
        assert_eq!(
            curve.weights,
            vec![
                1., ARC_WEIGHT, 1., ARC_WEIGHT, 1., ARC_WEIGHT, 1., ARC_WEIGHT, 1.
            ]
        );
        // Sixteen sampled points satisfy both sphere equations.
        let worst = circle_samples(curve, [0., 0., 0.], 2., [2., 0., 0.], 2.);
        assert!(worst <= 1e-12, "{worst}");
        assert!(sampled <= 1e-12, "{sampled}");
        // The circle plane x=1 is seen from either center within a +-60-degree
        // cone around +-x: it crosses quadrants 0,3 (both hemispheres) of the
        // first sphere and quadrants 1,2 of the second.
        assert_eq!(
            first_uv.iter().map(|l| l.patch).collect::<Vec<_>>(),
            [0, 3, 4, 7]
        );
        assert_eq!(
            second_uv.iter().map(|l| l.patch).collect::<Vec<_>>(),
            [1, 2, 5, 6]
        );
        assert!(uv_samples(&first, first_uv, ([0., 0., 0.], 2.), ([2., 0., 0.], 2.)) <= 1e-12);
        assert!(uv_samples(&second, second_uv, ([2., 0., 0.], 2.), ([0., 0., 0.], 2.)) <= 1e-12);
        // Operand swap keeps the circle, flips the axis and the UV roles.
        let swapped = intersect_sphere_sphere(&second, &first, Options::default()).unwrap();
        let (_, s_center, s_radius, s_normal, s_first, s_second, _) = only_circle(&swapped);
        assert!((s_radius - radius).abs() <= 1e-15);
        assert!(sub(s_center, center).iter().all(|x| x.abs() <= 1e-12));
        assert!(
            s_normal
                .iter()
                .zip(normal)
                .all(|(x, y)| (x + y).abs() <= 1e-12)
        );
        assert_eq!(s_first.len(), second_uv.len());
        assert_eq!(s_second.len(), first_uv.len());
    }

    #[test]
    fn concentric_equal_spheres_report_coincident_unresolved() {
        let first = crate::analytic::sphere(2.).unwrap();
        let second = crate::analytic::sphere(2.).unwrap();
        let report = intersect_sphere_sphere(&first, &second, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(report.unresolved.len(), 1);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::CoincidentTrim
        );
        assert!(!report.permits_topology_change());
    }

    #[test]
    fn concentric_unequal_and_contained_spheres_are_empty_resolved() {
        let small = crate::analytic::sphere(2.).unwrap();
        let large = crate::analytic::sphere(3.).unwrap();
        let report = intersect_sphere_sphere(&small, &large, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
        // Contained without contact: d = 1 < |3 - 2| is false; use radii 1 and 3.
        let tiny = crate::analytic::sphere(1.).unwrap();
        let shifted = translated(&large, [1., 0., 0.]);
        let report = intersect_sphere_sphere(&tiny, &shifted, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
    }

    #[test]
    fn separate_spheres_are_empty_resolved() {
        let first = crate::analytic::sphere(2.).unwrap();
        let second = translated(&first, [5., 0., 0.]);
        let report = intersect_sphere_sphere(&first, &second, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
    }

    #[test]
    fn external_tangency_band_stays_unresolved() {
        let first = crate::analytic::sphere(2.).unwrap();
        // Exact tangency and inside the outward classification band: no point.
        for gap in [0., -3e-14] {
            let second = translated(&first, [4. + gap, 0., 0.]);
            let report = intersect_sphere_sphere(&first, &second, Options::default()).unwrap();
            assert!(report.components.is_empty(), "{gap} {report:?}");
            assert_eq!(report.coverage, Coverage::Incomplete);
            assert_eq!(
                report.unresolved[0].reason,
                UnresolvedReason::TangencyOrMultipleRoot
            );
        }
        // Just outside the band the pair resolves empty; just inside, a circle.
        let apart = translated(&first, [4. + 1e-12, 0., 0.]);
        let report = intersect_sphere_sphere(&first, &apart, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        let closer = translated(&first, [4. - 1e-12, 0., 0.]);
        let report = intersect_sphere_sphere(&first, &closer, Options::default()).unwrap();
        let (_, _, radius, _, _, _, _) = only_circle(&report);
        // h^2 = r^2 - (d/2)^2 ~ 2e-12 loses four decimal digits to binary64
        // cancellation in any evaluation order; compare squared radii.
        let d = 4_f64 - 1e-12;
        let oracle = 4_f64 - (d / 2.).powi(2);
        assert!(
            (radius * radius - oracle).abs() <= 2e-15,
            "{} vs {oracle}",
            radius * radius
        );
    }

    #[test]
    fn internal_tangency_band_stays_unresolved() {
        let small = crate::analytic::sphere(1.).unwrap();
        let large = translated(&crate::analytic::sphere(3.).unwrap(), [2., 0., 0.]);
        let report = intersect_sphere_sphere(&small, &large, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot
        );
        // Clearly inside the band boundary: d below |r1-r2| is containment
        // (empty, resolved), d above it is a transverse circle.
        let large = translated(&crate::analytic::sphere(3.).unwrap(), [2. - 1e-9, 0., 0.]);
        let report = intersect_sphere_sphere(&small, &large, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        let large = translated(&crate::analytic::sphere(3.).unwrap(), [2. + 1e-9, 0., 0.]);
        let report = intersect_sphere_sphere(&small, &large, Options::default()).unwrap();
        only_circle(&report);
    }

    #[test]
    fn axis_aligned_lifted_uv_radius_matches_polar_oracle() {
        let first = crate::analytic::sphere(3.).unwrap();
        let second = translated(&first, [0., 0., 2.]);
        let report = intersect_sphere_sphere(&first, &second, Options::default()).unwrap();
        let (curve, center, radius, normal, first_uv, second_uv, _) = only_circle(&report);
        assert!((radius - 8_f64.sqrt()).abs() <= 1e-12, "{radius}");
        assert!(
            sub(center, [0., 0., 1.]).iter().all(|x| x.abs() <= 1e-12),
            "{center:?}"
        );
        assert!(
            sub(normal, [0., 0., 1.]).iter().all(|x| x.abs() <= 1e-12),
            "{normal:?}"
        );
        // The parallel at height t = 1 above the equator lifts on sphere 1's
        // north patches (faces 0..4) to UV circles of radius sqrt((r-t)/(r+t));
        // on sphere 2 it sits at t = -1, lifting to the south patches (4..8).
        assert_eq!(
            first_uv.iter().map(|l| l.patch).collect::<Vec<_>>(),
            [0, 1, 2, 3]
        );
        assert_eq!(
            second_uv.iter().map(|l| l.patch).collect::<Vec<_>>(),
            [4, 5, 6, 7]
        );
        let t = 1_f64;
        let r = 3_f64;
        let rho = ((r - t) / (r + t)).sqrt();
        // asin-based oracle: polar angle phi = asin(t/r), stereographic radius
        // rho = tan(pi/4 - phi/2).
        let phi = (t / r).asin();
        let rho_asin = (std::f64::consts::FRAC_PI_4 - phi / 2.).tan();
        assert!((rho - rho_asin).abs() <= 1e-15);
        for lift in first_uv.iter().chain(second_uv) {
            assert_eq!(lift.arcs.len(), 1, "{lift:?}");
            let arc = &lift.arcs[0];
            assert_eq!(arc.degree, 2);
            assert_eq!(arc.weights, vec![1., ARC_WEIGHT, 1.]);
            for endpoint in [&arc.control_points[0], &arc.control_points[2]] {
                let modulus = endpoint[0].hypot(endpoint[1]);
                assert!((modulus - rho).abs() <= 1e-12, "{modulus} vs {rho}");
            }
        }
        assert!(uv_samples(&first, first_uv, ([0., 0., 0.], 3.), ([0., 0., 2.], 3.)) <= 1e-12);
        assert!(uv_samples(&second, second_uv, ([0., 0., 2.], 3.), ([0., 0., 0.], 3.)) <= 1e-12);
        let worst = circle_samples(curve, [0., 0., 0.], 3., [0., 0., 2.], 3.);
        assert!(worst <= 1e-12, "{worst}");
    }

    #[test]
    fn rotated_translated_spheres_keep_exact_circle() {
        let first = crate::analytic::sphere(2.).unwrap();
        let second = rotated_translated(
            &crate::analytic::sphere(1.5).unwrap(),
            0.5,
            [1.5, 0.3, -0.2],
        );
        let c2: [f64; 3] = [1.5, 0.3, -0.2];
        let report = intersect_sphere_sphere(&first, &second, Options::default()).unwrap();
        let (curve, center, radius, _, first_uv, second_uv, sampled) = only_circle(&report);
        // Independent binary64 oracle from the two sphere definitions.
        let d = c2[0].hypot(c2[1]).hypot(c2[2]);
        let along = (4. - 2.25 + d * d) / (2. * d);
        let oracle = (4_f64 - along * along).sqrt();
        assert!((radius - oracle).abs() <= 1e-12, "{radius} vs {oracle}");
        let expected_center = [along * c2[0] / d, along * c2[1] / d, along * c2[2] / d];
        assert!(
            sub(center, expected_center)
                .iter()
                .all(|x| x.abs() <= 1e-12),
            "{center:?} vs {expected_center:?}"
        );
        let worst = circle_samples(curve, [0., 0., 0.], 2., c2, 1.5);
        assert!(worst <= 1e-12, "{worst}");
        assert!(sampled <= 1e-12, "{sampled}");
        assert!(!first_uv.is_empty() && !second_uv.is_empty());
        assert!(uv_samples(&first, first_uv, ([0., 0., 0.], 2.), (c2, 1.5)) <= 1e-12);
        assert!(uv_samples(&second, second_uv, (c2, 1.5), ([0., 0., 0.], 2.)) <= 1e-12);
    }

    #[test]
    fn noncanonical_surfaces_are_explicit_unsupported_regions() {
        let sphere = crate::analytic::sphere(1.).unwrap();
        for other in [
            crate::analytic::cylinder(1., 2.).unwrap(),
            crate::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap(),
        ] {
            for (a, b) in [(&sphere, &other), (&other, &sphere)] {
                let report = intersect_sphere_sphere(a, b, Options::default()).unwrap();
                assert!(report.components.is_empty(), "{report:?}");
                assert_eq!(report.coverage, Coverage::Incomplete);
                assert_eq!(report.unresolved.len(), 1);
                assert_eq!(
                    report.unresolved[0].reason,
                    UnresolvedReason::UnsupportedSurface
                );
                assert_eq!(
                    report.unresolved[0].parameter_box,
                    vec![0., 1., 0., 1., 0., 1., 0., 1.]
                );
            }
        }
        // A sphere with a perturbed patch is no longer a valid model at all:
        // like every other query, invalid input is a hard error, not a region.
        let mut perturbed = crate::analytic::sphere(1.).unwrap();
        perturbed.faces[0].surface.weights[1][1] = 1.5;
        assert!(intersect_sphere_sphere(&sphere, &perturbed, Options::default()).is_err());
    }
}
