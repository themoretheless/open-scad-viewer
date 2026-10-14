use super::*;

/// One sphere patch's share of the intersection circle in that patch's UV.
#[derive(Clone, Debug)]
pub struct SpherePatchCircle {
    /// Face index in the source model.
    pub patch: usize,
    /// Exact 2D rational quadratic arcs (or one degree-1 segment) clipped to
    /// the patch quarter-disk, each with knots [0,0,0,1,1,1] or [0,0,1,1].
    pub arcs: Vec<Curve>,
    /// Source-preserving algebraic pullback; not fitted NURBS arcs.
    pub implicit: Option<ImplicitSphereSection>,
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
pub(super) struct PatchFrame {
    pub(super) a: [f64; 3],
    pub(super) b: [f64; 3],
    /// Signed unit vector toward this patch's pole.
    pub(super) pole: [f64; 3],
}

#[derive(Clone, Debug)]
pub struct CanonicalSphere {
    pub center: [f64; 3],
    pub radius: f64,
    /// Observed structural deviation bound; feeds the outward classification.
    pub error: f64,
    pub(super) patches: Vec<PatchFrame>,
    pub(super) pole_surfaces: Option<Vec<nurbs_core::surface::Surface>>,
}

pub(crate) fn unit_quarter_arc(curve: &Curve) -> bool {
    curve.degree == 2
        && curve.knots == [0., 0., 0., 1., 1., 1.]
        && curve.control_points == [[1., 0.].to_vec(), [1., 1.].to_vec(), [0., 1.].to_vec()]
        && (curve.weights == [1., ARC_WEIGHT, 1.] || curve.weights == [1., 1., 2.])
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
pub fn recognize(model: &Model) -> Result<Option<CanonicalSphere>> {
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
    frames::finish(center,radius,error,frames,None)
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
    #[doc(hidden)] pub fn invert_uv(&self, patch: usize, point: [f64; 3]) -> Option<[f64; 2]> {
        if self.pole_surfaces.is_some() { return pole_charts::invert(self,&self.patches[patch],point); }
        let uv = invert_patch(self, &self.patches[patch], point);
        (uv[0].abs() < 1e29 && uv[1].abs() < 1e29).then_some(uv)
    }
}

/// Exact 3D circle: four 90-degree rational arcs, knots 0..=4, weights cos(pi/4).
#[doc(hidden)] pub fn circle_curve(center: [f64; 3], radius: f64, e1: [f64; 3], e2: [f64; 3]) -> Curve {
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
pub(super) fn in_quarter_disk(p: [f64; 2]) -> bool {
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
#[doc(hidden)] pub fn circle_arcs(center: [f64; 2], rho: f64, start: f64, end: f64) -> Vec<Curve> {
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
            if !rho2.is_finite() || rho2 <= 0. {
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
#[doc(hidden)] pub fn lift(
    sphere: &CanonicalSphere,
    normal: [f64; 3],
    middle: [f64; 3],
) -> Vec<SpherePatchCircle> {
    if sphere.pole_surfaces.is_some() { return pole_charts::sections(sphere,normal,middle,None); }
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
            SpherePatchCircle { patch, arcs, implicit:None }
        })
        .collect()
}

/// Intersection of two ccw arcs `(start, sweep)` on the circle, `start` in
/// `[0, TAU)`, `sweep` in `(0, TAU)`. Returns up to two components as
/// `(unwrapped_start, sweep)` pairs; a full-circle argument returns the
/// other arc unchanged.
#[doc(hidden)] pub fn ccw_intersect(a: (f64, f64), b: (f64, f64)) -> Vec<(f64, f64)> {
    if !a.1.is_finite() || a.1 <= 0. || !b.1.is_finite() || b.1 <= 0. {
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
