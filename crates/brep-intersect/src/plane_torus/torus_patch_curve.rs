use super::*;

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
pub(super) fn uv_line(from: [f64; 2], to: [f64; 2]) -> Curve {
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
pub(super) fn snap_unit(x: f64) -> f64 {
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
pub(super) fn circle_residuals(
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


/// Plane-UV ellipse parameterization of the circle
/// `p(phi) = center + cos(phi) w1 + sin(phi) w2`.
pub(super) fn plane_uv_metric(
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
pub(super) fn arc_parameter(local: f64) -> f64 {
    if (local - QUARTER).abs() <= 64. * f64::EPSILON * 4. {
        return 1.;
    }
    let t = (local / 2.).tan();
    let s = std::f64::consts::SQRT_2 * t / (1. - t);
    s / (1. + s)
}

/// Quadrant-local exact arc parameter of an angle, snapped to the patch
/// seam at pure rounding scale: returns (quadrant, local parameter in [0,1]).
pub(super) fn quadrant_local(angle: f64) -> (usize, f64) {
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
pub(super) fn lift_meridian(
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
pub(super) fn circle_components(
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
    lift: &TorusLift<'_>,
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
    if !m_len.is_finite() || m_len <= 0. {
        report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    }
    let m = m.map(|x| x / m_len);
    let r = torus.minor;
    for side in [1., -1.] {
        let inward = m.map(|x| x * side);
        let circle_center = std::array::from_fn(|k| torus.center[k] + side * torus.major * m[k]);
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
