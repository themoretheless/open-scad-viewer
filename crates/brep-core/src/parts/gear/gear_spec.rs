use super::*;
pub(super) fn invalid(message: impl Into<String>) -> Error {
    Error::new("BREP_INVALID_SIZE", message)
}

#[derive(Clone, Debug)]
pub struct GearSpec {
    pub module: f64,
    pub teeth: usize,
    pub pressure_angle_deg: f64,
    pub height: f64,
    /// Helix angle at the pitch circle, signed (positive twists
    /// counter-clockwise with height); zero is a spur gear.
    pub helix_angle_deg: f64,
    /// Two mirrored helical halves.
    pub herringbone: bool,
    /// Central bore diameter of an external gear (zero for none).
    pub bore: f64,
    /// Internal (ring) gear: teeth point inward, `rim_width` of material
    /// outside the root circle.
    pub internal: bool,
    pub rim_width: f64,
    pub clearance: f64,
    pub backlash: f64,
}

impl Default for GearSpec {
    fn default() -> Self {
        Self {
            module: 1.,
            teeth: 20,
            pressure_angle_deg: 20.,
            height: 5.,
            helix_angle_deg: 0.,
            herringbone: false,
            bore: 0.,
            internal: false,
            rim_width: 2.,
            clearance: 0.25,
            backlash: 0.,
        }
    }
}

/// Derived radii and angles of a spec, mirroring the sampled generator.
#[derive(Clone, Debug)]
pub struct GearGeometry {
    pub pitch_radius: f64,
    pub base_radius: f64,
    pub tip_radius: f64,
    pub root_radius: f64,
    pub outside_radius: f64,
    /// Continuous stored-curve error bound. When flank_rounding_certified is
    /// true, this includes nominal GearSpec parameter derivation; otherwise
    /// it refers to stored derived radii and angles. Point evaluation is separate.
    pub flank_deviation: f64,
    /// Whether the flank bound also covers complete nominal GearSpec radius,
    /// pressure/placement and roll derivation, including arithmetic rounding.
    /// Root transitions, topology and helical walls remain separate.
    pub flank_rounding_certified: bool,
    /// Inclusive curve bound is certified against the stored derived radius
    /// and placement angles. This does not certify GearSpec parameter derivation.
    pub flank_stored_parameters_rounding_certified: bool,
    /// Continuous wall error. If sweep_rounding_certified, includes saved-net
    /// rounding and nominal GearSpec twist derivation, relative to the saved
    /// profile. Flank/root profile error remains separate.
    pub sweep_deviation: f64,
    pub sweep_rounding_certified: bool,
    pub face_count: usize,
    /// Continuous bound on canonical endpoint adjustment of lines/arcs.
    /// Certified involute poles are retained exactly. Does not prove embedding.
    pub profile_join_adjustment_upper: f64,
    /// Full original arc radial deviation, including nominal radius derivation
    /// and canonical joins. Does not bound angular placement or radial lines.
    pub arc_radius_error_upper: Option<f64>,
    pub arc_radius_rounding_certified: bool,
    /// Continuous same-parameter error of saved arcs and radial transition lines
    /// against nominal GearSpec components. Includes angular/radius derivation.
    pub transition_error_upper: Option<f64>,
    pub transition_rounding_certified: bool,
    /// Continuous componentwise two-sided profile error versus the nominal
    /// sharp GearSpec profile, only when every flank/arc/line is admitted.
    pub profile_error_upper: Option<f64>,
    /// Profile error plus saved-wall/nominal-twist error. No solid embedding claim.
    pub ideal_wall_error_upper: Option<f64>,
}

// ---------------------------------------------------------------------------
// 2D outline
// ---------------------------------------------------------------------------

pub(super) fn polar(r: f64, a: f64) -> [f64; 2] {
    [r * a.cos(), r * a.sin()]
}

pub(super) fn line2(a: [f64; 2], b: [f64; 2]) -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a.to_vec(), b.to_vec()],
        weights: vec![1., 1.],
        periodic: false,
    }
}

/// Smooth quadratic Bezier corner fillet between `p0` and `p2` with corner
/// control point `p1`, tangent to both adjoining segments.
pub(super) fn corner_fillet2(p0: [f64; 2], p1: [f64; 2], p2: [f64; 2]) -> Curve {
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![p0.to_vec(), p1.to_vec(), p2.to_vec()],
        weights: vec![1., 1., 1.],
        periodic: false,
    }
}

/// Computes the $G^1$ tangent-continuous quadratic Bezier control point `p1`
/// connecting `p0` (with unit/non-zero outgoing tangent `t0`) to `p2` (with
/// incoming tangent `t2` pointing towards `p2`), falling back to `fallback_p1`
/// if the rays do not intersect cleanly inside the corner.
pub(super) fn tangent_fillet_control_point(
    p0: [f64; 2],
    t0: [f64; 2],
    p2: [f64; 2],
    t2: [f64; 2],
    fallback_p1: [f64; 2],
) -> [f64; 2] {
    let chord = (p2[0] - p0[0]).hypot(p2[1] - p0[1]);
    let l0 = t0[0].hypot(t0[1]);
    let l2 = t2[0].hypot(t2[1]);
    if chord <= 1e-9 || l0 <= 1e-12 || l2 <= 1e-12 {
        return fallback_p1;
    }
    let d0 = [t0[0] / l0, t0[1] / l0];
    let d2 = [t2[0] / l2, t2[1] / l2];
    // Solve p0 + s0 * d0 = p2 - s2 * d2  =>  s0 * d0 + s2 * d2 = p2 - p0
    let det = d0[0] * d2[1] - d0[1] * d2[0];
    let dx = p2[0] - p0[0];
    let dy = p2[1] - p0[1];
    if det.abs() > 1e-4 {
        let s0 = (dx * d2[1] - dy * d2[0]) / det;
        let s2 = (d0[0] * dy - d0[1] * dx) / det;
        if s0 > 0.05 * chord && s0 < 1.5 * chord && s2 > 0.05 * chord && s2 < 1.5 * chord {
            return [p0[0] + s0 * d0[0], p0[1] + s0 * d0[1]];
        }
    }
    fallback_p1
}

/// Smoothly rounds the outer tip corners of a quintic involute flank (`u = 1`
/// for a rising flank, `u = 0` for a falling flank) so the flank transitions
/// tangentially into the shortened tip arc `[center - tip_arc_half, center + tip_arc_half]`.
pub(super) fn round_flank_tip_corner(
    curve: &mut Curve,
    high: f64,
    center: f64,
    tip_arc_half: f64,
    is_rising: bool,
) {
    let n = curve.control_points.len();
    if n < 6 {
        return;
    }
    let tip_angle = if is_rising {
        center - tip_arc_half
    } else {
        center + tip_arc_half
    };
    let end = polar(high, tip_angle);
    // Tangent direction along the CCW tip circle at `tip_angle`
    let tangent = [-tip_angle.sin(), tip_angle.cos()];
    if is_rising {
        let p_in = [curve.control_points[n - 3][0], curve.control_points[n - 3][1]];
        let chord = (end[0] - p_in[0]).hypot(end[1] - p_in[1]);
        let handle = (chord * 0.42).max(1e-6);
        let p_prev = [
            end[0] - handle * tangent[0],
            end[1] - handle * tangent[1],
        ];
        curve.control_points[n - 1] = end.to_vec();
        curve.control_points[n - 2] = p_prev.to_vec();
        curve.control_points[n - 3] = [
            0.5 * (p_in[0] + p_prev[0]),
            0.5 * (p_in[1] + p_prev[1]),
        ]
        .to_vec();
    } else {
        let p_in = [curve.control_points[2][0], curve.control_points[2][1]];
        let chord = (p_in[0] - end[0]).hypot(p_in[1] - end[1]);
        let handle = (chord * 0.42).max(1e-6);
        let p_next = [
            end[0] + handle * tangent[0],
            end[1] + handle * tangent[1],
        ];
        curve.control_points[0] = end.to_vec();
        curve.control_points[1] = p_next.to_vec();
        curve.control_points[2] = [
            0.5 * (p_in[0] + p_next[0]),
            0.5 * (p_in[1] + p_next[1]),
        ]
        .to_vec();
    }
}

/// Smoothly rounds the inner corners of a quintic involute flank on an internal
/// gear (`start == low`) so both the tip (`low`) and root (`high`) junctions
/// transition smoothly into their circular arcs.
pub(super) fn round_internal_flank_corners(
    curve: &mut Curve,
    low: f64,
    high: f64,
    center: f64,
    low_arc_half: f64,
    high_arc_half: f64,
    is_rising: bool,
) {
    let n = curve.control_points.len();
    if n < 6 {
        return;
    }
    let sign = if is_rising { -1. } else { 1. };
    let low_angle = center + sign * low_arc_half;
    let high_angle = center + sign * high_arc_half;
    let p_low = polar(low, low_angle);
    let p_high = polar(high, high_angle);
    let t_low = [-low_angle.sin(), low_angle.cos()];
    let t_high = [-high_angle.sin(), high_angle.cos()];
    if is_rising {
        // u=0 is at `low` (after preceding low arc), u=1 is at `high` (before high arc)
        let p2 = [curve.control_points[2][0], curve.control_points[2][1]];
        let h0 = ((p2[0] - p_low[0]).hypot(p2[1] - p_low[1]) * 0.42).max(1e-6);
        let p1 = [p_low[0] + h0 * t_low[0], p_low[1] + h0 * t_low[1]];
        curve.control_points[0] = p_low.to_vec();
        curve.control_points[1] = p1.to_vec();
        curve.control_points[2] = [0.5 * (p1[0] + p2[0]), 0.5 * (p1[1] + p2[1])].to_vec();

        let p_in = [curve.control_points[n - 3][0], curve.control_points[n - 3][1]];
        let h1 = ((p_high[0] - p_in[0]).hypot(p_high[1] - p_in[1]) * 0.42).max(1e-6);
        let p_prev = [p_high[0] - h1 * t_high[0], p_high[1] - h1 * t_high[1]];
        curve.control_points[n - 1] = p_high.to_vec();
        curve.control_points[n - 2] = p_prev.to_vec();
        curve.control_points[n - 3] = [
            0.5 * (p_in[0] + p_prev[0]),
            0.5 * (p_in[1] + p_prev[1]),
        ]
        .to_vec();
    } else {
        // u=0 is at `high` (after high arc), u=1 is at `low` (before low arc)
        let p2 = [curve.control_points[2][0], curve.control_points[2][1]];
        let h0 = ((p2[0] - p_high[0]).hypot(p2[1] - p_high[1]) * 0.42).max(1e-6);
        let p1 = [p_high[0] + h0 * t_high[0], p_high[1] + h0 * t_high[1]];
        curve.control_points[0] = p_high.to_vec();
        curve.control_points[1] = p1.to_vec();
        curve.control_points[2] = [0.5 * (p1[0] + p2[0]), 0.5 * (p1[1] + p2[1])].to_vec();

        let p_in = [curve.control_points[n - 3][0], curve.control_points[n - 3][1]];
        let h1 = ((p_low[0] - p_in[0]).hypot(p_low[1] - p_in[1]) * 0.42).max(1e-6);
        let p_prev = [p_low[0] - h1 * t_low[0], p_low[1] - h1 * t_low[1]];
        curve.control_points[n - 1] = p_low.to_vec();
        curve.control_points[n - 2] = p_prev.to_vec();
        curve.control_points[n - 3] = [
            0.5 * (p_in[0] + p_prev[0]),
            0.5 * (p_in[1] + p_prev[1]),
        ]
        .to_vec();
    }
}

/// Exact circular arc about the origin from `from` to `to` (sweep below a
/// half turn, either sense).
pub(super) fn arc2(r: f64, from: f64, to: f64) -> Result<Curve> {
    let sweep = to - from;
    if !(sweep.abs() > 1e-12 && sweep.abs() < ARC_WEIGHT_MAX_SWEEP) {
        return Err(invalid("Gear arc must sweep less than a half turn"));
    }
    let mid = (from + to) / 2.;
    let w = (sweep / 2.).cos();
    Ok(Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            polar(r, from).to_vec(),
            polar(r / w, mid).to_vec(),
            polar(r, to).to_vec(),
        ],
        weights: vec![1., w, 1.],
        periodic: false,
    })
}

/// Full circle as `arcs` exact arcs (at least four), counter-clockwise
/// from +x.
pub(super) fn circle2(r: f64, arcs: usize) -> Result<Vec<Curve>> {
    let arcs = arcs.max(4);
    (0..arcs)
        .map(|q| {
            arc2(
                r,
                q as f64 * TAU / arcs as f64,
                (q + 1) as f64 * TAU / arcs as f64,
            )
        })
        .collect()
}

/// Arc count for a circle of radius `r` that must stay outside radius
/// `inner` even when each arc is displayed as two chords: the inscribed
/// radius of the chord polygon must clear `inner`.
pub(super) fn arcs_clearing(r: f64, inner: f64) -> usize {
    if inner <= 0. || inner >= r {
        return 4;
    }
    let half = (inner / r).acos();
    ((PI / (2. * half)).ceil() as usize).clamp(4, 64)
}

/// Arcs per full circle on a gear: one arc for every two teeth, so the rim
/// and the bore are displayed as round as the teeth at any tessellation
/// level, while the arc count stays exact and bounded.
pub fn circle_arcs(teeth: usize) -> usize {
    (teeth / 2).clamp(4, 64)
}

/// Involute of the base circle `base` between roll parameters `t0..t1`,
/// placed at the tooth centre with half-angle function `half`; `sign` picks
/// the flank side. Quintic Hermite approximation with an ideal continuous bound.
pub(super) fn involute(
    base: f64,
    t0: f64,
    t1: f64,
    center: f64,
    sign: f64,
    half: &dyn Fn(f64) -> f64,
) -> Result<(Curve, f64)> {
    let angle = center + sign * half(base);
    let (curve, certificate) = involute_flank(base, t0, t1, angle, -sign)?;
    Ok((curve, certificate.error_upper))
}

/// Build a quintic involute flank and certify the actual stored poles and
/// knots, including trigonometric placement and reverse-parameter rounding.
/// Radius and angles are the supplied binary64 values, interpreted as exact
/// nominal geometry. Their derivation from a complete GearSpec is not covered.
/// Reflection is +/-1. Angles must be within -8..8; at most 32 controls are
/// retained, and the inclusive geometric error must be <= 1e-6.
pub fn involute_flank(
    base: f64,
    t0: f64,
    t1: f64,
    angle: f64,
    reflection: f64,
) -> Result<(Curve, nurbs_core::involute::QuinticCertificate)> {
    if !angle.is_finite() || angle.abs() > 8. || !(reflection == 1. || reflection == -1.) {
        return Err(invalid("Flank needs a bounded angle and reflection +/-1"));
    }
    // Reserve half of the requested allowance for stored pole/knot placement.
    let approximation = nurbs_core::involute::approximate_quintic(
        [0.; 3],
        base,
        t0.min(t1),
        t0.max(t1),
        FLANK_FIT * 0.5,
    )?;
    let mut curve = approximation.curve;
    if curve.control_points.len() > 32 {
        return Err(invalid(
            "Gear flank needs more than 32 controls at the requested tolerance",
        ));
    }
    let (sin, cos) = angle.sin_cos();
    for point in &mut curve.control_points {
        let (x, y) = (point[0], reflection * point[1]);
        *point = vec![cos * x - sin * y, sin * x + cos * y];
    }
    if t1 < t0 {
        curve = curve.reverse()?;
    }
    let certificate = nurbs_core::involute::certify_quintic_placed(
        &curve, [0.; 3], base, t0, t1, angle, reflection,
    )?;
    if certificate.error_upper > FLANK_FIT {
        return Err(invalid(
            "Stored flank exceeds the rounding-inclusive error allowance",
        ));
    }
    Ok((curve, certificate))
}

pub(super) fn reverse2(c: &Curve) -> Curve {
    let mut r = c.clone();
    r.control_points.reverse();
    r.weights.reverse();
    let [a, b] = c.domain();
    r.knots = c.knots.iter().rev().map(|k| a + b - k).collect();
    r
}

/// Closed loops of the outline (each a connected CCW or CW chain of 2D
/// curves, material on the left), the geometry report, and the fit.
/// Original planar closed profile, with exact endpoint ownership and unchanged
/// certified involute poles. Returns candidate geometry, not an embedding proof.
pub fn profile_with_report(spec: &GearSpec) -> Result<(Vec<Vec<Curve>>, GearGeometry)> {
    use super::profile_stages::{ToothBuilder, ToothFrame, certify_outline, outline_loops, validate_spec};
    validate_spec(spec)?;
    let frame = ToothFrame::new(spec)?;
    let ideal = nurbs_core::involute::derive_gear_parameter_bounds(
        spec.module,
        spec.teeth,
        spec.pressure_angle_deg,
        spec.clearance,
        spec.backlash,
        spec.internal,
    )
    .ok();
    let mut builder = ToothBuilder::new(spec, &frame, ideal.as_ref());
    for i in 0..spec.teeth {
        builder.push_tooth(i)?;
    }
    let (tooth, tally) = builder.finish();
    let mut loops = outline_loops(spec, &frame, tooth)?;
    let geometry = certify_outline(spec, &frame, ideal.as_ref(), &mut loops, tally)?;
    Ok((loops, geometry))
}

pub(super) fn rotate2(c: &Curve, theta: f64) -> Curve {
    let (s, co) = theta.sin_cos();
    let mut r = c.clone();
    for p in &mut r.control_points { let (x, y) = (p[0], p[1]); p[0] = co * x - s * y; p[1] = s * x + co * y; }
    r
}

pub(super) fn lift(c: &Curve, z: f64) -> Curve {
    let mut r = c.clone();
    for p in &mut r.control_points { p.push(z); }
    r
}

/// Wall surface of one outline curve between `z0` and `z1`, rotated by
/// `twist` radians over the height: ruled for zero twist, else quintic
/// endpoint-Hermite spans with an ideal continuous bound at most SWEEP_FIT.
pub(super) fn wall(
    c: &Curve, z0: f64, z1: f64, theta0: f64, twist: f64, loop_budget: f64, nominal: Option<([f64; 2], [f64; 2])>,
) -> Result<(Surface, f64, bool)> {
    let source = lift(c, z0);
    let approximation = nurbs_core::helical_sweep::approximate(&source, z1 - z0, theta0, twist, loop_budget)?;
    if let Some((start, turn)) = nominal {
        if let Ok(certificate) = nurbs_core::helical_sweep::certify_stored(&source, &approximation.surface, [z1 - z0; 2], start, turn) {
            if certificate.error_upper <= SWEEP_FIT {
                return Ok((approximation.surface, certificate.error_upper, true));
            }
        }
    }
    Ok((approximation.surface, approximation.real_arithmetic_error_estimate, false))
}

/// Iso-curve of a surface at u = 0 or u = 1 (the shared vertical edge).
pub(super) fn column(surface: &Surface, last: bool) -> Curve {
    let i = if last { surface.control_points.len() - 1 } else { 0 };
    Curve {
        degree: surface.degree_v, knots: surface.knots_v.clone(),
        control_points: surface.control_points[i].clone(), weights: surface.weights[i].clone(), periodic: false,
    }
}

/// Planar cap over the bounding box of the outline curves at height `z`:
/// the surface and the affine pcurve map.
pub(super) fn cap(curves: &[&Curve], z: f64) -> (Surface, [f64; 4]) {
    let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for c in curves {
        for p in &c.control_points {
            for k in 0..2 { lo[k] = lo[k].min(p[k]); hi[k] = hi[k].max(p[k]); }
        }
    }
    let surface = Surface {
        degree_u: 1, degree_v: 1, knots_u: vec![0., 0., 1., 1.], knots_v: vec![0., 0., 1., 1.],
        control_points: vec![vec![vec![lo[0], lo[1], z], vec![lo[0], hi[1], z]], vec![vec![hi[0], lo[1], z], vec![hi[0], hi[1], z]]],
        weights: vec![vec![1., 1.], vec![1., 1.]], periodic_u: false, periodic_v: false,
    };
    (surface, [lo[0], lo[1], hi[0] - lo[0], hi[1] - lo[1]])
}

pub(super) fn cap_pcurve(c: &Curve, map: [f64; 4]) -> Curve {
    let mut p = c.clone();
    for point in &mut p.control_points {
        let (x, y) = (point[0], point[1]);
        *point = vec![(x - map[0]) / map[2], (y - map[1]) / map[3]];
    }
    p
}

pub(super) fn unit_line(a: [f64; 2], b: [f64; 2]) -> Curve { line2(a, b) }
