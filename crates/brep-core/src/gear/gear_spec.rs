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
    if ![
        spec.module,
        spec.pressure_angle_deg,
        spec.height,
        spec.helix_angle_deg,
        spec.bore,
        spec.rim_width,
        spec.clearance,
        spec.backlash,
    ]
    .iter()
    .all(|x| x.is_finite())
    {
        return Err(invalid(
            "Gear dimensions, angles and allowances must be finite",
        ));
    }
    let teeth = spec.teeth as f64;
    if !(3..=256).contains(&spec.teeth) {
        return Err(invalid("Gear teeth must be 3..256"));
    }
    if !(0.05..=100.).contains(&spec.module) {
        return Err(invalid("Gear module must be 0.05..100 mm"));
    }
    if !(10.0..=35.).contains(&spec.pressure_angle_deg) {
        return Err(invalid("Gear pressure angle must be 10..35 degrees"));
    }
    if !(0.05..=1000.).contains(&spec.height) {
        return Err(invalid("Gear height must be 0.05..1000 mm"));
    }
    if !spec.helix_angle_deg.is_finite() || spec.helix_angle_deg.abs() >= 80. {
        return Err(invalid("Gear helix angle must be within -80..80 degrees"));
    }
    if spec.backlash < 0.
        || spec.backlash > spec.module / 2.
        || spec.clearance < 0.
        || spec.clearance > spec.module
    {
        return Err(invalid(
            "Gear backlash must be 0..module/2 and clearance 0..module",
        ));
    }
    let alpha = spec.pressure_angle_deg.to_radians();
    let pitch = spec.module * teeth / 2.;
    let base = pitch * alpha.cos();
    let (tip, root) = if spec.internal {
        (pitch - spec.module, pitch + spec.module + spec.clearance)
    } else {
        (pitch + spec.module, pitch - spec.module - spec.clearance)
    };
    let outside = if spec.internal {
        root + spec.rim_width
    } else {
        tip
    };
    if root <= 0. || outside > 10000. {
        return Err(invalid(
            "Gear root radius must be positive and outside radius at most 10000 mm",
        ));
    }
    if spec.internal {
        if tip <= base + 1e-9 {
            return Err(invalid(
                "Internal gear tip must stay outside the base circle",
            ));
        }
        if spec.bore != 0. {
            return Err(invalid(
                "Internal gear has a toothed opening; bore must be zero",
            ));
        }
        if spec.rim_width < spec.module / 4. {
            return Err(invalid(
                "Internal gear rim must be at least a quarter module",
            ));
        }
    } else if spec.bore < 0. || (spec.bore > 0. && spec.bore > 2. * root - spec.module / 2.) {
        return Err(invalid(
            "Gear bore must leave a quarter module inside the root circle",
        ));
    }
    let low = if spec.internal { tip } else { root };
    let high = if spec.internal { root } else { tip };
    let start = base.max(low);
    let inv_pitch = alpha.tan() - alpha;
    let half_pitch = PI / (2. * teeth)
        + if spec.internal {
            spec.backlash / (2. * pitch)
        } else {
            -spec.backlash / (2. * pitch)
        };
    let half = move |r: f64| -> f64 {
        let t = ((r / base).powi(2) - 1.).max(0.).sqrt();
        half_pitch + inv_pitch - (t - t.atan())
    };
    let low_half = half(start);
    let high_half = half(high);
    let step = TAU / teeth;
    if high_half <= 1e-6 || low_half >= step / 2. - 1e-6 {
        return Err(invalid(
            "Gear tooth profile collapses or overlaps; change teeth, pressure angle, backlash or clearance",
        ));
    }
    let t0 = ((start / base).powi(2) - 1.).max(0.).sqrt();
    let t1 = ((high / base).powi(2) - 1.).max(0.).sqrt();
    let mut flank_deviation: f64 = 0.;
    let ideal = nurbs_core::involute::derive_gear_parameter_bounds(
        spec.module,
        spec.teeth,
        spec.pressure_angle_deg,
        spec.clearance,
        spec.backlash,
        spec.internal,
    )
    .ok();
    let mut full_flank_certified = ideal.is_some();
    let mut full_flank_deviation = 0_f64;
    // One tooth outline, counter-clockwise, starting at the low radius on
    // the leading side.
    let mut tooth: Vec<Curve> = Vec::new();
    for i in 0..spec.teeth {
        let center = i as f64 * step + if spec.internal { step / 2. } else { 0. };
        if start > low + 1e-12 {
            tooth.push(line2(
                polar(low, center - low_half),
                polar(start, center - low_half),
            ));
        }
        let (rising, d1) = involute(base, t0, t1, center, -1., &half)?;
        let (falling, d2) = involute(base, t1, t0, center, 1., &half)?;
        flank_deviation = flank_deviation.max(d1).max(d2);
        if spec.internal {
            // The hole reverses each flank a second time. Recheck the actual
            // saved reverse knots rather than assuming subtraction is exact.
            for (curve, from, to, sign) in [(&rising, t1, t0, -1.), (&falling, t0, t1, 1.)] {
                let saved = reverse2(curve);
                let proof = nurbs_core::involute::certify_quintic_placed(
                    &saved,
                    [0.; 3],
                    base,
                    from,
                    to,
                    center + sign * half(base),
                    -sign,
                )?;
                if proof.error_upper > FLANK_FIT {
                    return Err(invalid(
                        "Reversed internal flank exceeds the inclusive allowance",
                    ));
                }
                flank_deviation = flank_deviation.max(proof.error_upper);
            }
        }
        if let Some(ideal) = &ideal {
            use nurbs_core::interval_eval::Interval as I;
            let step = I::new(ideal.tooth_step[0], ideal.tooth_step[1])?;
            let nominal_center =
                step.mul(I::point(i as f64 + if spec.internal { 0.5 } else { 0. }))?;
            let half = I::new(ideal.half_base_angle[0], ideal.half_base_angle[1])?;
            for (curve, from, to, sign) in [
                (&rising, ideal.roll_start, ideal.roll_end, -1.),
                (&falling, ideal.roll_end, ideal.roll_start, 1.),
            ] {
                let angle = nominal_center.add(half.mul(I::point(sign))?)?;
                let reversed = spec.internal.then(|| reverse2(curve));
                let curve = reversed.as_ref().unwrap_or(curve);
                let (from, to) = if spec.internal {
                    (to, from)
                } else {
                    (from, to)
                };
                match nurbs_core::involute::certify_quintic_parameter_box(
                    curve,
                    [0.; 3],
                    ideal.base_radius,
                    from,
                    to,
                    [angle.lo, angle.hi],
                    -sign,
                ) {
                    Ok(certificate) => {
                        full_flank_deviation = full_flank_deviation.max(certificate.error_upper);
                        full_flank_certified &= certificate.error_upper <= FLANK_FIT;
                    }
                    Err(_) => full_flank_certified = false,
                }
            }
        }
        tooth.push(rising);
        tooth.push(arc2(high, center - high_half, center + high_half)?);
        tooth.push(falling);
        if start > low + 1e-12 {
            tooth.push(line2(
                polar(start, center + low_half),
                polar(low, center + low_half),
            ));
        }
        tooth.push(arc2(low, center + low_half, center + step - low_half)?);
    }
    let mut loops = Vec::new();
    if spec.internal {
        // Material outside: the outer circle is the CCW boundary, the tooth
        // outline runs clockwise as a hole.
        loops.push(circle2(
            outside,
            arcs_clearing(outside, root).max(circle_arcs(spec.teeth)),
        )?);
        let mut hole: Vec<Curve> = tooth.iter().rev().map(reverse2).collect();
        hole.iter_mut().for_each(|_| {});
        loops.push(hole);
    } else {
        loops.push(tooth);
        if spec.bore > 0. {
            let mut hole: Vec<Curve> = circle2(spec.bore / 2., circle_arcs(spec.teeth))?
                .iter()
                .rev()
                .map(reverse2)
                .collect();
            hole.iter_mut().for_each(|_| {});
            loops.push(hole);
        }
    }
    let profile_join_adjustment_upper = profile_join::close(&mut loops, 1e-7)?;
    let arc_radius_error_upper = ideal
        .as_ref()
        .and_then(|bounds| arc_radius::certify(spec, &loops, bounds, start > low + 1e-12).ok());
    let arc_radius_rounding_certified = arc_radius_error_upper.is_some_and(|e| e <= FLANK_FIT);
    let transition_error_upper = ideal.as_ref().and_then(|bounds| {
        profile_parameter::certify(spec, &loops, bounds, start > low + 1e-12).ok()
    });
    let transition_rounding_certified = transition_error_upper.is_some_and(|e| e <= FLANK_FIT);
    if full_flank_certified {
        flank_deviation = flank_deviation.max(full_flank_deviation);
    }
    let profile_error_upper = if full_flank_certified && transition_rounding_certified {
        transition_error_upper.map(|e| e.max(flank_deviation))
    } else {
        None
    };
    let geometry = GearGeometry {
        pitch_radius: pitch,
        base_radius: base,
        tip_radius: tip,
        root_radius: root,
        outside_radius: outside,
        flank_deviation,
        flank_rounding_certified: full_flank_certified,
        flank_stored_parameters_rounding_certified: true,
        sweep_deviation: 0.,
        sweep_rounding_certified: false,
        face_count: 0,
        profile_join_adjustment_upper,
        arc_radius_error_upper,
        arc_radius_rounding_certified,
        transition_error_upper,
        transition_rounding_certified,
        profile_error_upper,
        ideal_wall_error_upper: None,
    };
    Ok((loops, geometry))
}

// ---------------------------------------------------------------------------
// Solid
// ---------------------------------------------------------------------------

pub(super) fn rotate2(c: &Curve, theta: f64) -> Curve {
    let (s, co) = theta.sin_cos();
    let mut r = c.clone();
    for p in &mut r.control_points {
        let (x, y) = (p[0], p[1]);
        p[0] = co * x - s * y;
        p[1] = s * x + co * y;
    }
    r
}

pub(super) fn lift(c: &Curve, z: f64) -> Curve {
    let mut r = c.clone();
    for p in &mut r.control_points {
        p.push(z);
    }
    r
}

/// Wall surface of one outline curve between `z0` and `z1`, rotated by
/// `twist` radians over the height: ruled for zero twist, else quintic
/// endpoint-Hermite spans with an ideal continuous bound at most SWEEP_FIT.
pub(super) fn wall(
    c: &Curve,
    z0: f64,
    z1: f64,
    theta0: f64,
    twist: f64,
    nominal: Option<([f64; 2], [f64; 2])>,
) -> Result<(Surface, f64, bool)> {
    let source = lift(c, z0);
    let approximation =
        nurbs_core::helical_sweep::approximate(&source, z1 - z0, theta0, twist, SWEEP_FIT)?;
    if let Some((start, turn)) = nominal {
        if let Ok(certificate) = nurbs_core::helical_sweep::certify_stored(
            &source,
            &approximation.surface,
            [z1 - z0; 2],
            start,
            turn,
        ) {
            if certificate.error_upper <= SWEEP_FIT {
                return Ok((approximation.surface, certificate.error_upper, true));
            }
        }
    }
    Ok((
        approximation.surface,
        approximation.real_arithmetic_error_estimate,
        false,
    ))
}

/// Iso-curve of a surface at u = 0 or u = 1 (the shared vertical edge).
pub(super) fn column(surface: &Surface, last: bool) -> Curve {
    let i = if last {
        surface.control_points.len() - 1
    } else {
        0
    };
    Curve {
        degree: surface.degree_v,
        knots: surface.knots_v.clone(),
        control_points: surface.control_points[i].clone(),
        weights: surface.weights[i].clone(),
        periodic: false,
    }
}

/// Planar cap over the bounding box of the outline curves at height `z`:
/// the surface and the affine pcurve map.
pub(super) fn cap(curves: &[&Curve], z: f64) -> (Surface, [f64; 4]) {
    let mut lo = [f64::INFINITY; 2];
    let mut hi = [f64::NEG_INFINITY; 2];
    for c in curves {
        for p in &c.control_points {
            for k in 0..2 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
    }
    let surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![lo[0], lo[1], z], vec![lo[0], hi[1], z]],
            vec![vec![hi[0], lo[1], z], vec![hi[0], hi[1], z]],
        ],
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
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

pub(super) fn unit_line(a: [f64; 2], b: [f64; 2]) -> Curve {
    line2(a, b)
}

/// Builds the gear solid. Returns the model and its geometry report.

pub(super) fn outline(spec:&GearSpec)->Result<(Vec<Vec<Curve>>,GearGeometry)> { profile_with_report(spec) }
