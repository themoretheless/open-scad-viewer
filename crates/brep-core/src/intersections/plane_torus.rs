//! Analytic plane/torus intersection for a canonical planar patch operand
//! and a canonical ring-torus solid, in the axial configurations only.
//!
//! The plane operand is the canonical finite rectangular planar patch of
//! `plane_sphere::recognize_plane` (one-face open model, exact bilinear
//! affine surface over the unit square, plane operand first). The torus
//! operand must be the exact canonical ring torus of `analytic::torus`:
//! sixteen rational biquadratic patches tiling four profile quadrants times
//! four revolution quadrants, with exact tensor-product weights, exact
//! unit-square boundary trims and sixteen ring/quadrant vertices, optionally
//! carried through a rigid affine placement (`recognize_torus` below). The
//! canonical constructor admits only strict ring tori (R - r >= 1e-5): horn
//! (r == R) and spindle (r > R) tori are refused at construction, so the
//! recognizer never certifies one and this cell never sees a
//! self-intersecting torus. Anything else is an explicit
//! `UnsupportedSurface` region — this cell never falls back to numerical
//! surface/surface subdivision.
//!
//! Only axial plane placements are analytic here:
//! - Plane perpendicular to the torus axis at axial height h from the center
//!   plane: |h| > r (clear beyond the outward band) resolves empty; |h| == r
//!   within the band is a tangency and stays `TangencyOrMultipleRoot`, never
//!   a guessed point or circle; |h| < r yields the two exact circles of
//!   radii R +- sqrt(r^2 - h^2) centered on the axis in the plane (h == 0 is
//!   the equator pair R +- r of the same branch). The circles are clipped by
//!   the patch's finite rectangular domain with exact eccentric-angle trims
//!   (`plane_sphere::clip_ellipse`); a tangency to a domain edge stays
//!   unresolved. An inner radius R - sqrt(r^2 - h^2) collapsing into the
//!   band would be an inner-tangency/horn configuration and stays
//!   unresolved (unreachable for a canonical strict ring torus, where
//!   R - r >= 1e-5; kept as an honest guard, never guessed through).
//! - Plane containing the axis (normal perpendicular to the axis, center in
//!   the plane at pure-rounding scale): the two exact meridian circles of
//!   radius r centered at +-R along the in-plane line where the plane meets
//!   the center plane, clipped the same way. A parallel-to-axis plane at a
//!   certified nonzero distance cuts a Cassini oval, not a circle pair:
//!   clearly offset planes are `UnsupportedSurface`, and an offset inside
//!   the recognition-scale band reports `NearCoincidence` — never snapped.
//! - Oblique planes are a quartic section with Villarceau degeneracies:
//!   clearly oblique planes are `UnsupportedSurface`, tilts inside the
//!   recognition-scale band off either axial configuration report
//!   `NearCoincidence`.
//! A plane can never coincide with a curved torus patch, so `CoincidentTrim`
//! is deliberately unused by this cell.
//!
//! Resolved contacts are exact rational circles (full: four 90-degree arcs,
//! weights cos(pi/4), knots 0..=4; clipped: exact swept arcs) with UV lifts
//! on both operands: on the plane, exact rational quadratic ellipse arcs per
//! the `recognize_plane` metric; on the torus the perpendicular-plane
//! circles are iso-v parallels (constant exact v over the profile quadrant,
//! u clipped per revolution quadrant) and the through-axis circles are
//! iso-u meridians (constant exact u over the revolution quadrant, v
//! clipped per profile quadrant) — both are degree-1 UV line segments on
//! the exact patch grid, one per covered patch. Classification uses outward
//! binary64 bands widened by both recognition deviations; the angular snap
//! is 64 binary64 epsilons plus the recognition-error angular allowances,
//! and recognition-scale tilts report `NearCoincidence`, never forced.
//! Nothing here authorizes a topology change.
use super::plane_sphere::{
    CanonicalPlane, EllipseClip, Halfplane2, PlanePatchCurve, clip_ellipse, conic_arcs_2d,
    conic_sweep, recognize_plane,
};
use super::sphere_sphere::{ARC_WEIGHT, RECOGNITION, ccw_intersect, circle_curve};
use super::*;
use crate::Model;

const TAU: f64 = std::f64::consts::TAU;
const QUARTER: f64 = std::f64::consts::FRAC_PI_2;
const QUADRANTS: [[f64; 2]; 4] = [[1., 0.], [0., 1.], [-1., 0.], [0., -1.]];

/// A recognized canonical ring torus: the exact sixteen-patch solid of
/// `analytic::torus`, optionally rigidly placed.
#[derive(Clone, Debug)]
pub(crate) struct CanonicalTorus {
    /// Torus center (the center of the symmetry-plane circle of radius R).
    pub(crate) center: [f64; 3],
    /// Unit symmetry axis; the profile circle rises toward +axis.
    pub(crate) axis: [f64; 3],
    pub(crate) major: f64,
    pub(crate) minor: f64,
    /// Observed structural deviation bound; feeds the outward classification.
    pub(crate) error: f64,
    /// In-plane orthonormal ring frame: x = quadrant-0 direction, y = axis x x.
    pub(crate) frame: [[f64; 3]; 2],
    /// Face index by [profile quadrant][revolution quadrant].
    pub(crate) patches: [[usize; 4]; 4],
}

/// Degree-1 pcurve exactly from `from` to `to` with unit knots/weights.
fn unit_edge(curve: &Curve, from: [f64; 2], to: [f64; 2]) -> bool {
    curve.degree == 1
        && curve.knots == [0., 0., 1., 1.]
        && curve.control_points == [from.to_vec(), to.to_vec()]
        && curve.weights == [1., 1.]
}

/// Profile-span control points (radial, axial) of the canonical torus
/// profile circle of radius `minor` centered at radial `major`: four
/// 90-degree arcs starting at the outer equator point, turning toward +axis.
fn profile_controls(major: f64, minor: f64) -> [[[f64; 2]; 3]; 4] {
    let (r, m) = (major, minor);
    [
        [[r + m, 0.], [r + m, m], [r, m]],
        [[r, m], [r - m, m], [r - m, 0.]],
        [[r - m, 0.], [r - m, -m], [r, -m]],
        [[r, -m], [r + m, -m], [r + m, 0.]],
    ]
}

/// Recognizes a canonical ring-torus solid as built by `analytic::torus`,
/// certifying the 4x4 quadrant patch tiling, the exact tensor-product
/// weights [[1,w,1],[w,w^2,w],[1,w,1]] with w = cos(pi/4), the unit-square
/// boundary trims, every control point against the exact construction in the
/// recovered ring frame, and all sixteen ring/quadrant vertices. Rigid
/// affine placement is admitted; anything else returns `None`. Only strict
/// ring tori exist canonically (the constructor refuses horn and spindle
/// tori), and the recovered radii must keep R - r clearly positive.
pub(crate) fn recognize_torus(model: &Model) -> Result<Option<CanonicalTorus>> {
    model.validate()?;
    if model.bodies.len() != 1
        || model.shells.len() != 1
        || model.faces.len() != 16
        || model.vertices.len() != 16
        || model.edges.len() != 32
        || model.loops.len() != 16
    {
        return Ok(None);
    }
    let shell = &model.shells[0];
    if !shell.closed || shell.faces.len() != 16 {
        return Ok(None);
    }
    let w2 = ARC_WEIGHT * ARC_WEIGHT;
    let expected_weights = [
        [1., ARC_WEIGHT, 1.],
        [ARC_WEIGHT, w2, ARC_WEIGHT],
        [1., ARC_WEIGHT, 1.],
    ];
    let boundary = [
        ([0., 0.], [1., 0.]),
        ([1., 0.], [1., 1.]),
        ([1., 1.], [0., 1.]),
        ([0., 1.], [0., 0.]),
    ];
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
            || surface
                .control_points
                .iter()
                .flatten()
                .any(|p| p.len() != 3)
            || surface.weights != expected_weights
            || !face.holes.is_empty()
        {
            return Ok(None);
        }
        let loop_ = &model.loops[face.outer];
        if loop_.coedges.len() != 4 {
            return Ok(None);
        }
        let mut seen = [false; 4];
        for coedge in &loop_.coedges {
            let mut hit = false;
            for (k, &(a, b)) in boundary.iter().enumerate() {
                if !seen[k] && unit_edge(&coedge.pcurve, a, b) {
                    seen[k] = true;
                    hit = true;
                    break;
                }
            }
            if !hit {
                return Ok(None);
            }
        }
        if seen.into_iter().any(|hit| !hit) {
            return Ok(None);
        }
    }
    // Center: each profile ring's four quadrant vertices average to the same
    // axis point, so all sixteen vertices average to the torus center.
    let mut center = [0.; 3];
    for vertex in &model.vertices {
        for k in 0..3 {
            center[k] += vertex.point[k] / 16.;
        }
    }
    let mut error: f64 = 0.;
    // The outer equator ring (rho = R + r, axial 0) is strictly the farthest
    // vertex set from the center for a ring torus: (R+r)^2 > R^2 + r^2.
    let dmax = model
        .vertices
        .iter()
        .map(|v| {
            let d = sub(v.point, center);
            d[0].hypot(d[1]).hypot(d[2])
        })
        .fold(0., f64::max);
    if !dmax.is_finite() || !(2e-5..=1e6).contains(&dmax) {
        return Ok(None);
    }
    let scale = dmax;
    let outer: Vec<[f64; 3]> = model
        .vertices
        .iter()
        .map(|v| v.point)
        .filter(|p| {
            let d = sub(*p, center);
            (d[0].hypot(d[1]).hypot(d[2]) - dmax).abs() <= RECOGNITION * scale
        })
        .collect();
    if outer.len() != 4 {
        return Ok(None);
    }
    // Axis from the outer ring plane; every outer vertex must lie in the
    // center plane within the band.
    let axis_raw = cross(sub(outer[1], outer[0]), sub(outer[2], outer[0]));
    let axis_len = axis_raw[0].hypot(axis_raw[1]).hypot(axis_raw[2]);
    if !(axis_len > 0.) || !axis_len.is_finite() {
        return Ok(None);
    }
    let axis = axis_raw.map(|x| x / axis_len);
    for p in &outer {
        let deviation = dot(sub(*p, center), axis).abs();
        if !deviation.is_finite() || deviation > RECOGNITION * scale {
            return Ok(None);
        }
        error = error.max(deviation);
    }
    // r is the extreme axial vertex coordinate; R = (R + r) - r.
    let minor = model
        .vertices
        .iter()
        .map(|v| dot(sub(v.point, center), axis).abs())
        .fold(0., f64::max);
    let major = dmax - minor;
    if !minor.is_finite() || !(1e-5..=1e6).contains(&minor) || !(major > minor) {
        return Ok(None);
    }
    // Ring frame: x from the first outer-ring vertex, y = axis x x.
    let x_dir = sub(outer[0], center).map(|x| x / dmax);
    let y_dir = cross(axis, x_dir);
    let frame = [x_dir, y_dir];
    let profile = profile_controls(major, minor);
    // Quadrant of a ring-frame point; both angular residuals must sit at
    // pure rounding/recognition scale.
    let quadrant_of = |point: [f64; 3]| -> Option<(usize, usize, f64)> {
        let d = sub(point, center);
        let axial = dot(d, axis);
        let perp = sub(d, axis.map(|x| x * axial));
        let rho = perp[0].hypot(perp[1]).hypot(perp[2]);
        let alpha = axial.atan2(rho - major);
        let theta = dot(perp, y_dir).atan2(dot(perp, x_dir));
        let residual = |angle: f64, q: i64| {
            (angle - q as f64 * QUARTER + std::f64::consts::PI).rem_euclid(TAU)
                - std::f64::consts::PI
        };
        let jq = (alpha / QUARTER).round() as i64;
        let iq = (theta / QUARTER).round() as i64;
        if residual(alpha, jq).abs() > RECOGNITION || residual(theta, iq).abs() > RECOGNITION {
            return None;
        }
        Some((jq.rem_euclid(4) as usize, iq.rem_euclid(4) as usize, rho))
    };
    // Per-patch quadrant tiling and exact control-point certification.
    let mut seen = [[false; 4]; 4];
    let mut patches = [[0usize; 4]; 4];
    for (index, face) in model.faces.iter().enumerate() {
        let surface = &face.surface;
        let p00 = surface.evaluate(0., 0.)?.point;
        let Some((j, i, _)) = quadrant_of([p00[0], p00[1], p00[2]]) else {
            return Ok(None);
        };
        if seen[j][i] {
            return Ok(None);
        }
        seen[j][i] = true;
        patches[j][i] = index;
        let qa = QUADRANTS[i];
        let qb = QUADRANTS[(i + 1) % 4];
        let pattern = [
            [qa[0], qa[1]],
            [qa[0] + qb[0], qa[1] + qb[1]],
            [qb[0], qb[1]],
        ];
        for (a, dir) in pattern.iter().enumerate() {
            for (b, profile_point) in profile[j].iter().enumerate() {
                let expected: [f64; 3] = std::array::from_fn(|k| {
                    center[k]
                        + profile_point[0] * (dir[0] * x_dir[k] + dir[1] * y_dir[k])
                        + profile_point[1] * axis[k]
                });
                let actual = &surface.control_points[a][b];
                let d = [
                    actual[0] - expected[0],
                    actual[1] - expected[1],
                    actual[2] - expected[2],
                ];
                let deviation = d[0].hypot(d[1]).hypot(d[2]);
                if !deviation.is_finite() || deviation > RECOGNITION * scale + 1e-12 {
                    return Ok(None);
                }
                error = error.max(deviation);
            }
        }
    }
    if seen.iter().flatten().any(|hit| !hit) {
        return Ok(None);
    }
    // Every vertex is a ring/quadrant vertex, each exactly once, certified
    // radially and axially against the exact profile ring starts.
    let mut used = [[false; 4]; 4];
    for vertex in &model.vertices {
        let Some((j, _, rho)) = quadrant_of(vertex.point) else {
            return Ok(None);
        };
        let d = sub(vertex.point, center);
        let axial = dot(d, axis);
        let perp = sub(d, axis.map(|x| x * axial));
        let theta = dot(perp, y_dir).atan2(dot(perp, x_dir));
        let iq = (theta / QUARTER).round() as i64;
        let i = iq.rem_euclid(4) as usize;
        if used[j][i] {
            return Ok(None);
        }
        used[j][i] = true;
        let start = profile[j][0];
        let deviation = (rho - start[0]).abs().max((axial - start[1]).abs());
        if !deviation.is_finite() || deviation > RECOGNITION * scale + 1e-12 {
            return Ok(None);
        }
        error = error.max(deviation);
    }
    if used.iter().flatten().any(|hit| !hit) {
        return Ok(None);
    }
    Ok(Some(CanonicalTorus {
        center,
        axis,
        major,
        minor,
        error,
        frame,
        patches,
    }))
}

/// One torus face's share of an intersection circle in that face's UV.
#[derive(Clone, Debug)]
pub struct TorusPatchCurve {
    /// Face index in the source torus model.
    pub patch: usize,
    /// Degree-1 iso-parameter segments: iso-v lines for parallels, iso-u
    /// lines for meridians, clipped per patch quadrant.
    pub arcs: Vec<Curve>,
}

#[derive(Clone, Debug)]
pub enum PlaneTorusComponent {
    /// Transverse axial section: the exact circle (full, or clipped to the
    /// finite plane domain with exact eccentric-angle trims) plus both UV
    /// lifts. Perpendicular planes yield the parallel pair
    /// R +- sqrt(r^2 - h^2); through-axis planes yield the meridian pair of
    /// radius r at +-R.
    Circle {
        /// Exact rational quadratic sweep: the full circle (four 90-degree
        /// arcs, weights cos(pi/4), knots 0..=4) when `full`, else the
        /// clipped arc pieces over the swept interval.
        curve: Curve,
        center: [f64; 3],
        radius: f64,
        /// Unit plane normal (the circle plane normal).
        normal: [f64; 3],
        /// True when the circle lies fully inside the plane patch domain.
        full: bool,
        plane_uv: Vec<PlanePatchCurve>,
        torus_uv: Vec<TorusPatchCurve>,
        /// Worst residual over curve samples against the torus implicit
        /// equation (scaled by 4(R+r)^3 to a length) and the plane equation.
        max_sample_residual: f64,
    },
}

/// UV line segment with unit knots/weights.
fn uv_line(from: [f64; 2], to: [f64; 2]) -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![from.to_vec(), to.to_vec()],
        weights: vec![1., 1.],
        periodic: false,
    }
}

/// Snap a quadrant-local parameter to a patch boundary at pure rounding
/// scale (64 binary64 epsilons of the unit parameter interval).
fn snap_unit(x: f64) -> f64 {
    let snap = 64. * f64::EPSILON;
    if x.abs() <= snap {
        0.
    } else if (x - 1.).abs() <= snap {
        1.
    } else {
        x
    }
}

/// Length-scaled torus implicit residual at a point:
/// |(p^2 + R^2 - r^2)^2 - 4 R^2 rho^2| / (4 (R + r)^3), computed in the
/// torus frame so placements do not change the value.
pub(crate) fn torus_residual(torus: &CanonicalTorus, point: [f64; 3]) -> f64 {
    let d = sub(point, torus.center);
    let axial = dot(d, torus.axis);
    let perp = sub(d, torus.axis.map(|x| x * axial));
    let rho2 = dot(perp, perp);
    let total = rho2 + axial * axial;
    let (r, m) = (torus.major, torus.minor);
    let base = total + r * r - m * m;
    let f = base * base - 4. * r * r * rho2;
    f.abs() / (4. * (r + m).powi(3))
}

/// Worst scaled torus/plane residual over `count` curve samples.
fn circle_residuals(
    curve: &Curve,
    torus: &CanonicalTorus,
    plane: &CanonicalPlane,
    count: usize,
) -> Result<f64> {
    let [lo, hi] = curve.domain();
    let mut worst = 0_f64;
    for i in 0..count {
        let t = lo + (hi - lo) * i as f64 / (count - 1) as f64;
        let p = curve.evaluate(t)?.point;
        let p = [p[0], p[1], p[2]];
        worst = worst
            .max(torus_residual(torus, p))
            .max(dot(plane.normal, sub(p, plane.origin)).abs());
    }
    Ok(worst)
}

/// Patch-UV half-planes of the canonical unit-square domain.
fn rect_halfplanes(plane: &CanonicalPlane, band: f64) -> [Halfplane2; 4] {
    [
        Halfplane2 {
            normal: [-1., 0.],
            offset: 0.,
            band: band / plane.u_len,
        },
        Halfplane2 {
            normal: [1., 0.],
            offset: 1.,
            band: band / plane.u_len,
        },
        Halfplane2 {
            normal: [0., -1.],
            offset: 0.,
            band: band / plane.v_len,
        },
        Halfplane2 {
            normal: [0., 1.],
            offset: 1.,
            band: band / plane.v_len,
        },
    ]
}

/// Plane-UV ellipse parameterization of the circle
/// `p(phi) = center + cos(phi) w1 + sin(phi) w2`.
fn plane_uv_metric(
    plane: &CanonicalPlane,
    center: [f64; 3],
    w1: [f64; 3],
    w2: [f64; 3],
) -> ([f64; 2], [f64; 2], [f64; 2]) {
    let rel = sub(center, plane.origin);
    let project = |w: [f64; 3]| {
        [
            dot(w, plane.u) / (plane.u_len * plane.u_len),
            dot(w, plane.v) / (plane.v_len * plane.v_len),
        ]
    };
    (
        [
            dot(rel, plane.u) / (plane.u_len * plane.u_len),
            dot(rel, plane.v) / (plane.v_len * plane.v_len),
        ],
        project(w1),
        project(w2),
    )
}

/// Exact parameter of a canonical 90-degree rational arc (weights cos(pi/4))
/// at swept angle `local` in [0, pi/2]. Substituting s = t/(1-t) into the
/// homogeneous arc gives tan(phi/2) = (s^2 + 2ws)/(s^2 + 4ws + 2) with
/// w = cos(pi/4), whose discriminant is exactly 2; the nonnegative root is
/// s = sqrt(2) tan(phi/2) / (1 - tan(phi/2)) and t = s/(1+s).
fn arc_parameter(local: f64) -> f64 {
    if (local - QUARTER).abs() <= 64. * f64::EPSILON * 4. {
        return 1.;
    }
    let t = (local / 2.).tan();
    let s = std::f64::consts::SQRT_2 * t / (1. - t);
    s / (1. + s)
}

/// Quadrant-local exact arc parameter of an angle, snapped to the patch
/// seam at pure rounding scale: returns (quadrant, local parameter in [0,1]).
fn quadrant_local(angle: f64) -> (usize, f64) {
    let qv = angle / QUARTER;
    let nearest = qv.round();
    if (qv - nearest).abs() <= 64. * f64::EPSILON * 4. {
        (nearest.rem_euclid(4.) as usize, 0.)
    } else {
        let j = qv.floor().clamp(0., 3.) as usize;
        (j, snap_unit(arc_parameter(angle - j as f64 * QUARTER)))
    }
}

/// Torus lift of a parallel circle at profile angle `profile`: an iso-v
/// line on the profile-quadrant patch row, clipped per revolution quadrant
/// to the swept eccentric intervals (the circle parameter is the revolution
/// angle).
pub(crate) fn lift_parallel(
    torus: &CanonicalTorus,
    profile: f64,
    arcs: &[(f64, f64)],
) -> Vec<TorusPatchCurve> {
    let (j, v0) = quadrant_local(profile);
    let mut out = Vec::new();
    for (i, &patch) in torus.patches[j].iter().enumerate() {
        let mut lines = Vec::new();
        for &(a, b) in arcs {
            for (s0, sweep) in ccw_intersect((i as f64 * QUARTER, QUARTER), (a, b - a)) {
                let u0 = snap_unit(arc_parameter(s0 - i as f64 * QUARTER));
                let u1 = snap_unit(arc_parameter(s0 + sweep - i as f64 * QUARTER));
                if u1 - u0 > 1e-14 {
                    lines.push(uv_line([u0, v0], [u1, v0]));
                }
            }
        }
        if !lines.is_empty() {
            out.push(TorusPatchCurve { patch, arcs: lines });
        }
    }
    out
}

/// Torus lift of a meridian circle at revolution angle `revolution`: an
/// iso-u line on the revolution-quadrant patch column, clipped per profile
/// quadrant to the swept eccentric intervals (the circle parameter is the
/// profile angle).
fn lift_meridian(
    torus: &CanonicalTorus,
    revolution: f64,
    arcs: &[(f64, f64)],
) -> Vec<TorusPatchCurve> {
    let (i, u0) = quadrant_local(revolution);
    let mut out = Vec::new();
    for (j, row) in torus.patches.iter().enumerate() {
        let mut lines = Vec::new();
        for &(a, b) in arcs {
            for (s0, sweep) in ccw_intersect((j as f64 * QUARTER, QUARTER), (a, b - a)) {
                let v0 = snap_unit(arc_parameter(s0 - j as f64 * QUARTER));
                let v1 = snap_unit(arc_parameter(s0 + sweep - j as f64 * QUARTER));
                if v1 - v0 > 1e-14 {
                    lines.push(uv_line([u0, v0], [u0, v1]));
                }
            }
        }
        if !lines.is_empty() {
            out.push(TorusPatchCurve {
                patch: row[i],
                arcs: lines,
            });
        }
    }
    out
}

/// One resolved circle component family: full or clipped exact sweep, both
/// UV lifts, and the sampled residual bound. `w1`/`w2` are the exact 3D
/// circle basis vectors (length = radius); the circle parameter coincides
/// with the torus revolution angle (parallel) or profile angle (meridian).
#[allow(clippy::too_many_arguments)]
fn circle_components(
    torus: &CanonicalTorus,
    plane: &CanonicalPlane,
    center: [f64; 3],
    radius: f64,
    w1: [f64; 3],
    w2: [f64; 3],
    clip: EllipseClip,
    uv_center: [f64; 2],
    uv_w1: [f64; 2],
    uv_w2: [f64; 2],
    lift: &dyn Fn(&CanonicalTorus, &[(f64, f64)]) -> Vec<TorusPatchCurve>,
) -> Result<Vec<PlaneTorusComponent>> {
    let build = |start: f64, end: f64, full: bool| -> Result<PlaneTorusComponent> {
        let curve = if full {
            circle_curve(
                center,
                radius,
                w1.map(|x| x / radius),
                w2.map(|x| x / radius),
            )
        } else {
            conic_sweep(center, w1, w2, start, end)
        };
        let torus_uv = lift(torus, &[(start, end)]);
        let plane_uv = vec![PlanePatchCurve {
            patch: 0,
            arcs: conic_arcs_2d(uv_center, uv_w1, uv_w2, start, end),
        }];
        let max_sample_residual = circle_residuals(&curve, torus, plane, 16)?;
        Ok(PlaneTorusComponent::Circle {
            curve,
            center,
            radius,
            normal: plane.normal,
            full,
            plane_uv,
            torus_uv,
            max_sample_residual,
        })
    };
    match clip {
        EllipseClip::Empty => Ok(Vec::new()),
        EllipseClip::Full => Ok(vec![build(0., TAU, true)?]),
        EllipseClip::Arcs(intervals) => intervals
            .into_iter()
            .map(|(a, b)| build(a, b, false))
            .collect(),
    }
}

/// Analytic plane/torus intersection of a canonical planar patch and a
/// canonical ring-torus solid, in the axial configurations only.
/// Non-canonical operands, oblique planes and parallel-to-axis offset
/// (Cassini) planes are explicit `UnsupportedSurface` regions — never a
/// numerical fallback; tangencies (|h| == r, domain-edge touches, the
/// inner-radius collapse band) and recognition-scale near-axial bands stay
/// unresolved — tangent contacts are never guessed circles.
pub fn intersect_plane_torus(
    plane_model: &Model,
    torus_model: &Model,
    options: Options,
) -> Result<Report<PlaneTorusComponent>> {
    let options = options.validate()?;
    let _ = options;
    let mut report = Report::default();
    let domain = vec![0., 1., 0., 1., 0., 1., 0., 1.];
    let (Some(plane), Some(torus)) = (recognize_plane(plane_model)?, recognize_torus(torus_model)?)
    else {
        report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    };
    report.boxes_visited = 1;
    let scale = plane.u_len
        + plane.v_len
        + torus.major
        + torus.minor
        + sub(torus.center, plane.origin)
            .iter()
            .map(|v| v.abs())
            .fold(0., f64::max)
        + 1.;
    // Outward binary64 classification band: both recognition deviations plus
    // a rounding allowance on the distance arithmetic.
    let band = plane.error + torus.error + 16. * f64::EPSILON * scale;
    // Angular certification (the plane/cylinder discipline): pure-rounding
    // tilts snap, recognition-scale tilts report near_coincidence, clearly
    // oblique angles are refused as unsupported.
    let angular_snap = 64. * f64::EPSILON
        + plane.error / plane.u_len.min(plane.v_len)
        + torus.error / (2. * torus.major);
    let g = dot(plane.normal, torus.axis);
    let ga = g.abs();
    let perpendicular = ga >= 1. - angular_snap;
    let parallel = ga <= angular_snap;
    if !perpendicular && !parallel {
        if ga >= 1. - RECOGNITION || ga <= RECOGNITION {
            // Recognition-scale tilt off an axial configuration: never forced.
            report.unresolved(domain, UnresolvedReason::NearCoincidence);
        } else {
            // The general oblique plane/torus section is a quartic with
            // Villarceau degeneracies — out of scope, honestly refused.
            report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        }
        return Ok(report);
    }
    let rect = rect_halfplanes(&plane, band);
    if perpendicular {
        // Plane (anti-)perpendicular to the axis: axial height of the plane
        // above the center plane (constant on the plane when n || axis).
        let h = dot(torus.axis, sub(plane.origin, torus.center));
        let ah = h.abs();
        let r = torus.minor;
        if ah > r + band {
            // Provable miss: the whole band clears the minor radius.
            return Ok(report);
        }
        if ah >= (r - band).max(0.) {
            // Tangent plane, or a band straddling it: never a guessed circle.
            report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
            return Ok(report);
        }
        let s2 = r * r - h * h;
        if !s2.is_finite() || s2 <= 0. {
            report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
            return Ok(report);
        }
        let s = s2.sqrt();
        if torus.major - s <= band {
            // Inner-radius collapse (inner tangency / horn): unreachable for
            // a canonical strict ring torus, kept unresolved, never guessed.
            report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
            return Ok(report);
        }
        let circle_center = std::array::from_fn(|k| torus.center[k] + h * torus.axis[k]);
        for side in [1., -1.] {
            let rho = torus.major + side * s;
            // Profile angle: cos(phi) = side*s/r, sin(phi) = h/r.
            let profile = h.atan2(side * s).rem_euclid(TAU);
            let w1 = torus.frame[0].map(|x| x * rho);
            let w2 = torus.frame[1].map(|x| x * rho);
            let (uv_center, uv_w1, uv_w2) = plane_uv_metric(&plane, circle_center, w1, w2);
            let (clip, tangent) = clip_ellipse(uv_center, uv_w1, uv_w2, &rect);
            if tangent {
                // The circle touches the patch boundary within the band:
                // unresolved alongside any resolved arc components; a full
                // circle is withheld, never guessed through the touch.
                report.unresolved(domain.clone(), UnresolvedReason::TangencyOrMultipleRoot);
            }
            let clip = match (&clip, tangent) {
                (EllipseClip::Full, true) => EllipseClip::Empty,
                (clip, _) => clip.clone(),
            };
            report.components.extend(circle_components(
                &torus,
                &plane,
                circle_center,
                rho,
                w1,
                w2,
                clip,
                uv_center,
                uv_w1,
                uv_w2,
                &|t: &CanonicalTorus, arcs: &[(f64, f64)]| lift_parallel(t, profile, arcs),
            )?);
        }
        return Ok(report);
    }
    // Parallel to the axis: the through-axis meridian pair is analytic only
    // when the plane contains the axis (center in the plane at pure rounding
    // scale). A certified offset plane cuts a Cassini oval: refused.
    let d = dot(plane.normal, sub(torus.center, plane.origin));
    let ad = d.abs();
    if ad > angular_snap * scale {
        if ad <= band + RECOGNITION * scale {
            report.unresolved(domain, UnresolvedReason::NearCoincidence);
        } else {
            report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        }
        return Ok(report);
    }
    let m = cross(torus.axis, plane.normal);
    let m_len = m[0].hypot(m[1]).hypot(m[2]);
    if !(m_len > 0.) || !m_len.is_finite() {
        report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    }
    let m = m.map(|x| x / m_len);
    let r = torus.minor;
    for side in [1., -1.] {
        let inward = m.map(|x| x * side);
        let circle_center =
            std::array::from_fn(|k| torus.center[k] + side * torus.major * m[k]);
        // Circle basis (outward radial, axis): the circle parameter is the
        // profile angle exactly.
        let w1 = inward.map(|x| x * r);
        let w2 = torus.axis.map(|x| x * r);
        let revolution = dot(inward, torus.frame[1])
            .atan2(dot(inward, torus.frame[0]))
            .rem_euclid(TAU);
        let (uv_center, uv_w1, uv_w2) = plane_uv_metric(&plane, circle_center, w1, w2);
        let (clip, tangent) = clip_ellipse(uv_center, uv_w1, uv_w2, &rect);
        if tangent {
            report.unresolved(domain.clone(), UnresolvedReason::TangencyOrMultipleRoot);
        }
        let clip = match (&clip, tangent) {
            (EllipseClip::Full, true) => EllipseClip::Empty,
            (clip, _) => clip.clone(),
        };
        report.components.extend(circle_components(
            &torus,
            &plane,
            circle_center,
            r,
            w1,
            w2,
            clip,
            uv_center,
            uv_w1,
            uv_w2,
            &|t: &CanonicalTorus, arcs: &[(f64, f64)]| lift_meridian(t, revolution, arcs),
        )?);
    }
    Ok(report)
}

impl value_codec::Serialize for TorusPatchCurve {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({"patch":self.patch,"arcs":self.arcs})
    }
}
impl value_codec::Serialize for PlaneTorusComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Circle {
                curve,
                center,
                radius,
                normal,
                full,
                plane_uv,
                torus_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"circle","curve":curve,"center":center,"radius":radius,
                "normal":normal,"full":full,"planeUv":plane_uv,"torusUv":torus_uv,
                "maxSampleResidual":max_sample_residual}),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::plane_sphere::plane_patch;
    use super::*;

    /// (curve, center, radius, full, plane_uv, torus_uv, sampled) — Copy view.
    type CircleView<'a> = (
        &'a Curve,
        [f64; 3],
        f64,
        bool,
        &'a [PlanePatchCurve],
        &'a [TorusPatchCurve],
        f64,
    );

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
    /// Torus implicit residual (unscaled quartic) in the canonical frame.
    fn implicit(point: [f64; 3], major: f64, minor: f64) -> f64 {
        let total = dot(point, point);
        let rho2 = point[0] * point[0] + point[1] * point[1];
        let base = total + major * major - minor * minor;
        (base * base - 4. * major * major * rho2).abs()
    }
    fn only_circles(report: &Report<PlaneTorusComponent>, count: usize) -> Vec<CircleView<'_>> {
        assert!(report.unresolved.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        assert!(!report.permits_topology_change());
        assert_eq!(report.components.len(), count, "{report:?}");
        report
            .components
            .iter()
            .map(|c| {
                let PlaneTorusComponent::Circle {
                    curve,
                    center,
                    radius,
                    full,
                    plane_uv,
                    torus_uv,
                    max_sample_residual,
                    ..
                } = c;
                (
                    curve,
                    *center,
                    *radius,
                    *full,
                    plane_uv.as_slice(),
                    torus_uv.as_slice(),
                    *max_sample_residual,
                )
            })
            .collect()
    }
    /// Worst residual of lifted UV arcs on both operands, evaluated through
    /// their own surfaces, against a caller-supplied implicit checker.
    fn uv_samples(
        plane_model: &Model,
        torus_model: &Model,
        circles: &[CircleView<'_>],
        check: &dyn Fn([f64; 3]) -> f64,
    ) -> f64 {
        let mut worst = 0_f64;
        for (_, _, _, _, plane_uv, torus_uv, _) in circles {
            for lift in plane_uv.iter() {
                let surface = &plane_model.faces[lift.patch].surface;
                for arc in &lift.arcs {
                    for k in 0..=8 {
                        let uv = arc.evaluate(k as f64 / 8.).unwrap().point;
                        assert!((-1e-9..=1. + 1e-9).contains(&uv[0]), "{uv:?}");
                        assert!((-1e-9..=1. + 1e-9).contains(&uv[1]), "{uv:?}");
                        let p = surface
                            .evaluate(uv[0].clamp(0., 1.), uv[1].clamp(0., 1.))
                            .unwrap()
                            .point;
                        worst = worst.max(check([p[0], p[1], p[2]]));
                    }
                }
            }
            for lift in torus_uv.iter() {
                let surface = &torus_model.faces[lift.patch].surface;
                for arc in &lift.arcs {
                    for k in 0..=8 {
                        let uv = arc.evaluate(k as f64 / 8.).unwrap().point;
                        assert!((-1e-9..=1. + 1e-9).contains(&uv[0]), "{uv:?}");
                        assert!((-1e-9..=1. + 1e-9).contains(&uv[1]), "{uv:?}");
                        let p = surface
                            .evaluate(uv[0].clamp(0., 1.), uv[1].clamp(0., 1.))
                            .unwrap()
                            .point;
                        worst = worst.max(check([p[0], p[1], p[2]]));
                    }
                }
            }
        }
        worst
    }

    #[test]
    fn perpendicular_section_matches_the_two_circle_oracle() {
        // Torus R=3, r=1; plane z = 0.4, patch [-5,5]^2: the exact parallels
        // of radii 3 +- sqrt(1 - 0.16), fully inside the patch.
        let torus = crate::analytic::torus(3., 1.).unwrap();
        let plane = plane_patch([-5., -5., 0.4], [10., 0., 0.], [0., 10., 0.]);
        let report = intersect_plane_torus(&plane, &torus, Options::default()).unwrap();
        let circles = only_circles(&report, 2);
        let s = (1_f64 - 0.16).sqrt();
        let expected = [(3. + s, 0usize), (3. - s, 1)];
        for k in 0..2 {
            let (curve, center, radius, full, plane_uv, torus_uv, sampled) = circles[k];
            assert!(
                (radius - expected[k].0).abs() <= 1e-12,
                "{radius} vs {}",
                expected[k].0
            );
            assert!(
                sub(center, [0., 0., 0.4]).iter().all(|x| x.abs() <= 1e-12),
                "{center:?}"
            );
            assert!(full);
            // Exact rational circle: four 90-degree arcs, weights cos(pi/4).
            assert_eq!(curve.degree, 2);
            assert_eq!(
                curve.knots,
                vec![0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.]
            );
            assert_eq!(curve.control_points.len(), 9);
            // Sixteen samples on both implicit equations.
            let mut worst = 0_f64;
            for i in 0..16 {
                let p = curve.evaluate(i as f64 / 4.).unwrap().point;
                let p = [p[0], p[1], p[2]];
                worst = worst.max(implicit(p, 3., 1.)).max((p[2] - 0.4).abs());
            }
            assert!(worst <= 1e-10, "{worst}");
            assert!(sampled <= 1e-10, "{sampled}");
            // Torus UV: one iso-v line on each revolution quadrant patch of
            // the profile quadrant row, at the exact v parameter.
            assert_eq!(torus_uv.len(), 4, "{torus_uv:?}");
            let phi = 0.4_f64.atan2(if k == 0 { s } else { -s }).rem_euclid(TAU);
            let v0 = arc_parameter(phi - expected[k].1 as f64 * QUARTER);
            for lift in torus_uv {
                assert_eq!(lift.arcs.len(), 1);
                let arc = &lift.arcs[0];
                assert_eq!(arc.degree, 1);
                assert_eq!(arc.control_points[0], vec![0., v0]);
                assert_eq!(arc.control_points[1], vec![1., v0]);
            }
            // Plane UV: four exact ellipse arcs inside the unit square.
            assert_eq!(plane_uv.len(), 1);
            assert_eq!(plane_uv[0].arcs.len(), 4);
            for arc in &plane_uv[0].arcs {
                assert_eq!(arc.weights, vec![1., ARC_WEIGHT, 1.]);
            }
        }
        let uv_worst = uv_samples(&plane, &torus, &circles, &|p| {
            implicit(p, 3., 1.) / (4. * 64.) + (p[2] - 0.4).abs()
        });
        assert!(uv_worst <= 1e-9, "{uv_worst}");
        // Operand order is fixed: the torus as plane operand refuses.
        let swapped = intersect_plane_torus(&torus, &plane, Options::default()).unwrap();
        assert_eq!(swapped.unresolved[0].reason, UnresolvedReason::UnsupportedSurface);
    }

    #[test]
    fn symmetry_plane_section_is_the_equator_pair() {
        // Plane z = 0: the exact equator circles R + r = 4 and R - r = 2,
        // landing exactly on profile quadrant seams (v = 0 on rows 0 and 2).
        let torus = crate::analytic::torus(3., 1.).unwrap();
        let recognized = recognize_torus(&torus).unwrap().unwrap();
        let plane = plane_patch([-5., -5., 0.], [10., 0., 0.], [0., 10., 0.]);
        let report = intersect_plane_torus(&plane, &torus, Options::default()).unwrap();
        let circles = only_circles(&report, 2);
        assert!((circles[0].2 - 4.).abs() <= 1e-12, "{}", circles[0].2);
        assert!((circles[1].2 - 2.).abs() <= 1e-12, "{}", circles[1].2);
        for (k, row) in [0usize, 2].iter().enumerate() {
            assert_eq!(circles[k].5.len(), 4);
            for lift in circles[k].5 {
                assert_eq!(lift.arcs[0].control_points[0], vec![0., 0.]);
                assert_eq!(lift.arcs[0].control_points[1], vec![1., 0.]);
                assert!(recognized.patches[*row].contains(&lift.patch));
            }
        }
        let uv_worst = uv_samples(&plane, &torus, &circles, &|p| {
            implicit(p, 3., 1.) / (4. * 64.) + p[2].abs()
        });
        assert!(uv_worst <= 1e-9, "{uv_worst}");
        // The near-horn limit admitted by the constructor still resolves:
        // R - r = 1e-5 keeps the inner equator circle clearly above the band.
        let slim = crate::analytic::torus(1.00001, 1.).unwrap();
        let report = intersect_plane_torus(&plane, &slim, Options::default()).unwrap();
        let circles = only_circles(&report, 2);
        assert!((circles[0].2 - 2.00001).abs() <= 1e-12, "{}", circles[0].2);
        assert!((circles[1].2 - 1e-5).abs() <= 1e-12, "{}", circles[1].2);
        // Horn and spindle tori are refused by the canonical constructor:
        // the recognizer can never certify one.
        assert!(crate::analytic::torus(2., 2.).is_err());
        assert!(crate::analytic::torus(1., 2.).is_err());
    }

    #[test]
    fn miss_tangency_and_band_classification() {
        let torus = crate::analytic::torus(3., 1.).unwrap();
        let at = |z: f64| plane_patch([-5., -5., z], [10., 0., 0.], [0., 10., 0.]);
        // Miss: |h| = 2 > r, empty and resolved.
        let report = intersect_plane_torus(&at(2.), &torus, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
        // Exact tangency and within the outward band: never a point/circle.
        for z in [1., 1. - 3e-14, -1.] {
            let report = intersect_plane_torus(&at(z), &torus, Options::default()).unwrap();
            assert!(report.components.is_empty(), "{z} {report:?}");
            assert_eq!(report.coverage, Coverage::Incomplete);
            assert_eq!(
                report.unresolved[0].reason,
                UnresolvedReason::TangencyOrMultipleRoot
            );
        }
        // Just clear of the band: a thin parallel pair resolves.
        let report = intersect_plane_torus(&at(1. - 1e-9), &torus, Options::default()).unwrap();
        let circles = only_circles(&report, 2);
        let h = 1. - 1e-9;
        let s = (1_f64 - h * h).sqrt();
        assert!(
            (circles[0].2 - (3. + s)).abs() <= 1e-12,
            "{} vs {}",
            circles[0].2,
            3. + s
        );
        assert!((circles[1].2 - (3. - s)).abs() <= 1e-12);
    }

    #[test]
    fn through_axis_section_matches_the_meridian_pair() {
        // Plane y = 0 contains the Z axis: the exact meridian circles of
        // radius r = 1 centered at (+-3, 0, 0). The patch normal is
        // (0,-1,0), so m = axis x normal = (+1,0,0): side +1 sits at
        // revolution angle 0 (quadrant column 0), side -1 at pi (column 2).
        let torus = crate::analytic::torus(3., 1.).unwrap();
        let recognized = recognize_torus(&torus).unwrap().unwrap();
        let plane = plane_patch([-5., 0., -5.], [10., 0., 0.], [0., 0., 10.]);
        let report = intersect_plane_torus(&plane, &torus, Options::default()).unwrap();
        let circles = only_circles(&report, 2);
        let mut signed = Vec::new();
        for (curve, center, radius, full, plane_uv, torus_uv, sampled) in &circles {
            assert!((radius - 1.).abs() <= 1e-12, "{radius}");
            assert!(center[1].abs() <= 1e-12 && center[2].abs() <= 1e-12, "{center:?}");
            assert!((center[0].abs() - 3.).abs() <= 1e-12, "{center:?}");
            signed.push(center[0]);
            assert!(full);
            let mut worst = 0_f64;
            for i in 0..16 {
                let p = curve.evaluate(i as f64 / 4.).unwrap().point;
                let p = [p[0], p[1], p[2]];
                worst = worst.max(implicit(p, 3., 1.)).max(p[1].abs());
            }
            assert!(worst <= 1e-10, "{worst}");
            assert!(*sampled <= 1e-10, "{sampled}");
            // Torus UV: one full iso-u line per profile quadrant patch of the
            // meridian's revolution column, exactly on the quadrant seam.
            assert_eq!(torus_uv.len(), 4, "{torus_uv:?}");
            let column = if center[0] > 0. { 0 } else { 2 };
            for lift in torus_uv.iter() {
                assert_eq!(lift.arcs.len(), 1);
                let arc = &lift.arcs[0];
                assert_eq!(arc.degree, 1);
                assert_eq!(arc.control_points[0], vec![0., 0.]);
                assert_eq!(arc.control_points[1], vec![0., 1.]);
                assert!(
                    (0..4).any(|j| recognized.patches[j][column] == lift.patch),
                    "{}",
                    lift.patch
                );
            }
            assert_eq!(plane_uv[0].arcs.len(), 4);
        }
        assert!(signed[0] * signed[1] < 0., "{signed:?}");
        let uv_worst = uv_samples(&plane, &torus, &circles, &|p| {
            implicit(p, 3., 1.) / (4. * 64.) + p[1].abs()
        });
        assert!(uv_worst <= 1e-9, "{uv_worst}");
    }

    #[test]
    fn offset_parallel_and_oblique_planes_are_honestly_refused() {
        let torus = crate::analytic::torus(3., 1.).unwrap();
        // Parallel to the axis at distance d: a Cassini-oval section, not a
        // circle pair — unsupported beyond the band, near_coincidence inside
        // the recognition-scale band, snapped only at pure rounding scale.
        let at = |y: f64| plane_patch([-5., y, -5.], [10., 0., 0.], [0., 0., 10.]);
        let refused = intersect_plane_torus(&at(0.5), &torus, Options::default()).unwrap();
        assert!(refused.components.is_empty());
        assert_eq!(refused.unresolved[0].reason, UnresolvedReason::UnsupportedSurface);
        let near = intersect_plane_torus(&at(1e-12), &torus, Options::default()).unwrap();
        assert!(near.components.is_empty());
        assert_eq!(near.unresolved[0].reason, UnresolvedReason::NearCoincidence);
        // Pure rounding-scale offset snaps to through-axis and resolves.
        let snapped = intersect_plane_torus(&at(1e-16), &torus, Options::default()).unwrap();
        assert_eq!(snapped.components.len(), 2, "{snapped:?}");
        // Oblique plane: the quartic with Villarceau degeneracies is out of
        // scope — unsupported, never a numerical fallback.
        let s = std::f64::consts::FRAC_1_SQRT_2;
        let oblique = plane_patch(
            [-5., -5. * s, -5. * s],
            [10., 0., 0.],
            [0., 10. * s, 10. * s],
        );
        let report = intersect_plane_torus(&oblique, &torus, Options::default()).unwrap();
        assert!(report.components.is_empty());
        assert_eq!(report.unresolved[0].reason, UnresolvedReason::UnsupportedSurface);
        // Recognition-scale tilts off either axial configuration report
        // near_coincidence, never forced: 1 - cos(a) ~ a^2/2 <= 1e-9 off
        // perpendicular, sin(b) ~ b <= 1e-9 off parallel.
        let a: f64 = 1e-6;
        let tilted_perp = plane_patch(
            [-5., -5. * a.cos(), -5. * a.sin()],
            [10., 0., 0.],
            [0., 10. * a.cos(), 10. * a.sin()],
        );
        let report = intersect_plane_torus(&tilted_perp, &torus, Options::default()).unwrap();
        assert_eq!(report.unresolved[0].reason, UnresolvedReason::NearCoincidence);
        let b: f64 = 1e-10;
        let tilted_par = plane_patch(
            [-5., -5. * b.sin(), -5.],
            [10., 0., 0.],
            [0., 10. * b.sin(), 10. * b.cos()],
        );
        let report = intersect_plane_torus(&tilted_par, &torus, Options::default()).unwrap();
        assert_eq!(report.unresolved[0].reason, UnresolvedReason::NearCoincidence);
    }

    #[test]
    fn domain_clipping_has_exact_trims_and_clipped_lifts() {
        // Plane z = 0.4, patch covering only x >= 0 (y in [-5, 5]): both
        // parallels clip to their x >= 0 half arcs with exact endpoints at
        // (0, +-rho, 0.4); the half circle covers revolution quadrants 3 and
        // 0 completely, so each torus lift spans u in [0, 1] on two patches.
        let torus = crate::analytic::torus(3., 1.).unwrap();
        let plane = plane_patch([0., -5., 0.4], [5., 0., 0.], [0., 10., 0.]);
        let report = intersect_plane_torus(&plane, &torus, Options::default()).unwrap();
        let circles = only_circles(&report, 2);
        let s = (1_f64 - 0.16).sqrt();
        for k in 0..2 {
            let (curve, _, radius, full, plane_uv, torus_uv, sampled) = circles[k];
            let rho = 3. + if k == 0 { s } else { -s };
            assert!((radius - rho).abs() <= 1e-12);
            assert!(!full);
            // Half circle: swept [3pi/2, 5pi/2] in the (x, y) basis, two
            // 90-degree pieces.
            assert_eq!(curve.knots, vec![0., 0., 0., 1., 1., 2., 2., 2.]);
            let start = curve.evaluate(0.).unwrap().point;
            let end = curve.evaluate(2.).unwrap().point;
            assert!(start[0].abs() <= 1e-12 && (start[1] + rho).abs() <= 1e-12, "{start:?}");
            assert!(end[0].abs() <= 1e-12 && (end[1] - rho).abs() <= 1e-12, "{end:?}");
            let mut worst = 0_f64;
            for i in 0..=16 {
                let p = curve.evaluate(i as f64 / 8.).unwrap().point;
                let p = [p[0], p[1], p[2]];
                assert!(p[0] >= -1e-12, "{p:?}");
                worst = worst.max(implicit(p, 3., 1.)).max((p[2] - 0.4).abs());
            }
            assert!(worst <= 1e-10, "{worst}");
            assert!(sampled <= 1e-10, "{sampled}");
            // Torus UV: two patches (revolution quadrants 3 and 0), each a
            // full-sweep iso-v line.
            assert_eq!(torus_uv.len(), 2, "{torus_uv:?}");
            for lift in torus_uv {
                assert_eq!(lift.arcs.len(), 1);
                let arc = &lift.arcs[0];
                let u0 = arc.control_points[0][0];
                let u1 = arc.control_points[1][0];
                assert!((u0, u1) == (0., 1.), "{u0} {u1}");
                assert_eq!(arc.control_points[0][1], arc.control_points[1][1]);
            }
            // Plane UV: two exact clipped ellipse arcs.
            assert_eq!(plane_uv[0].arcs.len(), 2);
        }
        let uv_worst = uv_samples(&plane, &torus, &circles, &|p| {
            implicit(p, 3., 1.) / (4. * 64.) + (p[2] - 0.4).abs()
        });
        assert!(uv_worst <= 1e-9, "{uv_worst}");
        // A circle clearly outside the rectangle: empty and resolved.
        let outside = plane_patch([4.5, -5., 0.4], [5., 0., 0.], [0., 10., 0.]);
        let report = intersect_plane_torus(&outside, &torus, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        // The outer circle tangent to the patch edge x = 3 + s: tangency
        // region, the circle withheld, never guessed; the inner circle
        // (radius 3 - s, center x = 0 < 3 + s) is provably outside too.
        let tangent = plane_patch([3. + s, -5., 0.4], [5., 0., 0.], [0., 10., 0.]);
        let report = intersect_plane_torus(&tangent, &torus, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert!(report.components.is_empty(), "{report:?}");
        assert!(
            report
                .unresolved
                .iter()
                .any(|u| u.reason == UnresolvedReason::TangencyOrMultipleRoot)
        );
    }

    #[test]
    fn rigid_placement_keeps_the_exact_circles() {
        let angle = 0.5;
        let offset = [0.3, -0.2, 1.1];
        let torus = rotated_translated(&crate::analytic::torus(3., 1.).unwrap(), angle, offset);
        let plane = rotated_translated(
            &plane_patch([-5., -5., 0.4], [10., 0., 0.], [0., 10., 0.]),
            angle,
            offset,
        );
        // The recognizer recovers the placed axis, center and radii.
        let recognized = recognize_torus(&torus).unwrap().unwrap();
        let (sin, cos) = angle.sin_cos();
        assert!((recognized.major - 3.).abs() <= 1e-12, "{}", recognized.major);
        assert!((recognized.minor - 1.).abs() <= 1e-12, "{}", recognized.minor);
        assert!(
            sub(recognized.axis, [0., -sin, cos])
                .iter()
                .all(|x| x.abs() <= 1e-12),
            "{:?}",
            recognized.axis
        );
        assert!(
            sub(recognized.center, offset).iter().all(|x| x.abs() <= 1e-12),
            "{:?}",
            recognized.center
        );
        let report = intersect_plane_torus(&plane, &torus, Options::default()).unwrap();
        let circles = only_circles(&report, 2);
        let s = (1_f64 - 0.16).sqrt();
        let placed = |p: [f64; 3]| {
            [
                p[0] + offset[0],
                cos * p[1] - sin * p[2] + offset[1],
                sin * p[1] + cos * p[2] + offset[2],
            ]
        };
        let placed_normal = sub(placed([0., 0., 2.4]), placed([0., 0., 1.4]));
        // Un-place a world point back into the canonical torus frame.
        let unplaced = |p: [f64; 3]| {
            [
                p[0] - offset[0],
                cos * (p[1] - offset[1]) + sin * (p[2] - offset[2]),
                -sin * (p[1] - offset[1]) + cos * (p[2] - offset[2]),
            ]
        };
        for k in 0..2 {
            let (curve, center, radius, _, _, _, sampled) = circles[k];
            let rho = 3. + if k == 0 { s } else { -s };
            assert!((radius - rho).abs() <= 1e-12, "{radius}");
            assert!(
                sub(center, placed([0., 0., 0.4]))
                    .iter()
                    .all(|x| x.abs() <= 1e-12),
                "{center:?}"
            );
            let mut worst = 0_f64;
            for i in 0..16 {
                let p = curve.evaluate(i as f64 / 4.).unwrap().point;
                let p = [p[0], p[1], p[2]];
                worst = worst
                    .max(implicit(unplaced(p), 3., 1.))
                    .max(dot(placed_normal, sub(p, center)).abs());
            }
            assert!(worst <= 1e-10, "{worst}");
            assert!(sampled <= 1e-10, "{sampled}");
        }
        let placed_origin = placed([-5., -5., 0.4]);
        let uv_worst = uv_samples(&plane, &torus, &circles, &|p| {
            implicit(unplaced(p), 3., 1.) / (4. * 64.)
                + dot(placed_normal, sub(p, placed_origin)).abs()
        });
        assert!(uv_worst <= 1e-9, "{uv_worst}");
    }

    #[test]
    fn noncanonical_operands_are_explicit_unsupported_regions() {
        let plane = plane_patch([-5., -5., 0.4], [10., 0., 0.], [0., 10., 0.]);
        let torus = crate::analytic::torus(3., 1.).unwrap();
        for (a, b) in [
            (plane.clone(), crate::analytic::cylinder(1., 2.).unwrap()),
            (plane.clone(), crate::analytic::sphere(2.).unwrap()),
            (plane.clone(), crate::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap()),
            (crate::analytic::cylinder(1., 2.).unwrap(), torus.clone()),
            (crate::cuboid([0., 0., 0.], [4., 4., 4.]).unwrap(), torus.clone()),
        ] {
            let report = intersect_plane_torus(&a, &b, Options::default()).unwrap();
            assert!(report.components.is_empty(), "{report:?}");
            assert_eq!(report.coverage, Coverage::Incomplete);
            assert_eq!(report.unresolved.len(), 1);
            assert_eq!(report.unresolved[0].reason, UnresolvedReason::UnsupportedSurface);
            assert_eq!(
                report.unresolved[0].parameter_box,
                vec![0., 1., 0., 1., 0., 1., 0., 1.]
            );
            assert!(!report.permits_topology_change());
        }
        // A skewed (non-rectangular) affine patch is not canonical.
        let skewed = plane_patch([-5., -5., 0.4], [10., 0., 0.], [3., 10., 0.]);
        let report = intersect_plane_torus(&skewed, &torus, Options::default()).unwrap();
        assert_eq!(report.unresolved[0].reason, UnresolvedReason::UnsupportedSurface);
        // A structurally perturbed torus (one moved control point) fails the
        // exact certification and refuses.
        let mut perturbed = torus.clone();
        perturbed.faces[0].surface.control_points[1][1][0] += 1e-6;
        perturbed.rebuild_topology_ids();
        let report = intersect_plane_torus(&plane, &perturbed, Options::default()).unwrap();
        assert_eq!(report.unresolved[0].reason, UnresolvedReason::UnsupportedSurface);
        // A valid canonical pair still resolves.
        let report = intersect_plane_torus(&plane, &torus, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
    }
}
