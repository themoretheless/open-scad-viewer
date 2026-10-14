use super::*;

#[derive(Clone, Debug)]
pub enum ConeConeComponent {
    /// Transverse side crossing: the exact circle plus both UV lifts.
    Circle {
        /// Exact rational full circle: degree 2, knots 0..=4, four 90-degree
        /// arcs with weights cos(pi/4).
        curve: Curve,
        center: [f64; 3],
        radius: f64,
        /// Unit shared axis (the first cone's axis, the circle plane normal).
        normal: [f64; 3],
        first_uv: Vec<CylinderPatchCurve>,
        second_uv: Vec<CylinderPatchCurve>,
        /// Worst residual over 16 circle samples against both cones' side
        /// profile equations.
        max_sample_residual: f64,
    },
}

/// The second cone's linear radius profile oriented into the shared axial
/// coordinate s (first cone's bottom ring center at s = 0, axis +axis).
pub(super) struct OrientedProfile<'a> {
    pub(super) cone: &'a CanonicalCone,
    /// Shared coordinate of this cone's own bottom ring center.
    pub(super) origin: f64,
    /// +1 when its axis agrees with the shared axis, -1 when flipped.
    pub(super) sign: f64,
}
impl OrientedProfile<'_> {
    pub(super) fn lo(&self) -> f64 {
        if self.sign > 0. {
            self.origin
        } else {
            self.origin - self.cone.height
        }
    }
    pub(super) fn hi(&self) -> f64 {
        if self.sign > 0. {
            self.origin + self.cone.height
        } else {
            self.origin
        }
    }
    /// Radius profile rho(s) in the shared coordinate; linear between rings.
    pub(super) fn radius(&self, s: f64) -> f64 {
        self.cone.r_bottom + self.cone.slope * self.sign * (s - self.origin)
    }
    /// Side-patch v parameter of the shared height s (iso-v lift line).
    pub(super) fn v(&self, s: f64) -> f64 {
        self.sign * (s - self.origin) / self.cone.height
    }
    /// Dimensionless taper of the radius profile in the shared coordinate.
    pub(super) fn taper(&self) -> f64 {
        self.cone.slope * self.sign
    }
}


/// Scaled Euclidean norm that never overflows or flushes subnormals.
pub(super) fn norm3(v: [f64; 3]) -> f64 {
    let scale = v.iter().map(|x| x.abs()).fold(0., f64::max);
    if scale == 0. {
        return 0.;
    }
    let scaled = v.map(|x| x / scale);
    scaled[0].hypot(scaled[1]).hypot(scaled[2]) * scale
}

/// Exact circle component at the shared axial position `s`, with iso-v
/// lifts on all four side patches of both cones and a 16-sample residual
/// bound against both side profile equations.
pub(super) fn circle_component(
    first: &CanonicalCone,
    profile: &OrientedProfile,
    s: f64,
) -> Result<ConeConeComponent> {
    let center = std::array::from_fn(|k| first.bottom[k] + s * first.axis[k]);
    let radius = first.r_bottom + first.slope * s;
    let iso_v = |v: f64| {
        vec![Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![[0., v].to_vec(), [1., v].to_vec()],
            weights: vec![1., 1.],
            periodic: false,
        }]
    };
    let first_uv = first
        .sides
        .iter()
        .map(|&patch| CylinderPatchCurve {
            patch,
            arcs: iso_v(s / first.height),
        })
        .collect();
    let second_uv = profile
        .cone
        .sides
        .iter()
        .map(|&patch| CylinderPatchCurve {
            patch,
            arcs: iso_v(profile.v(s)),
        })
        .collect();
    let curve = circle_curve(center, radius, first.frame[0], first.frame[1]);
    let second = profile.cone;
    let mut max_sample_residual = 0_f64;
    for i in 0..16 {
        let point = point3(&curve.evaluate(i as f64 / 4.)?.point);
        let side_residual = |cone: &CanonicalCone| {
            let rel = sub(point, cone.bottom);
            let a = dot(rel, cone.axis);
            let perp = sub(rel, cone.axis.map(|x| x * a));
            (norm3(perp) - (cone.r_bottom + cone.slope * a)).abs()
        };
        max_sample_residual = max_sample_residual
            .max(side_residual(first))
            .max(side_residual(second));
    }
    Ok(ConeConeComponent::Circle {
        curve,
        center,
        radius,
        normal: first.axis,
        first_uv,
        second_uv,
        max_sample_residual,
    })
}

/// Analytic cone/cone intersection of two canonical conical frustum solids,
/// coaxial configuration only (either axis orientation). Non-canonical
/// operands and clearly non-parallel or off-axis pairs are explicit
/// unsupported regions, never a numerical fallback; recognition-scale tilts
/// and offsets report near_coincidence, ring-plane coincidences, rim
/// contacts, apex meetings, equal-taper profile coincidence and
/// ill-conditioned near-equal-taper crossings stay unresolved — tangent
/// contacts and coincident trims are never guessed into components.
pub fn intersect_cone_cone(
    first_model: &Model,
    second_model: &Model,
    options: Options,
) -> Result<Report<ConeConeComponent>> {
    let options = options.validate()?;
    let _ = options;
    let mut report = Report::default();
    let domain = vec![0., 1., 0., 1., 0., 1., 0., 1.];
    let (Some(first), Some(second)) = (recognize_cone(first_model)?, recognize_cone(second_model)?)
    else {
        report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    };
    report.boxes_visited = 1;
    let offset = sub(second.bottom, first.bottom);
    let s0 = dot(offset, first.axis);
    let terms = s0.abs()
        + first.height
        + second.height
        + first.r_bottom
        + first.r_top
        + second.r_bottom
        + second.r_top
        + 1.;
    // Outward binary64 classification band: both recognition deviations plus
    // a rounding allowance on the axial coordinates and radii.
    let band = first.error + second.error + 16. * f64::EPSILON * terms;
    let scale = first.height
        + second.height
        + first.r_bottom
        + first.r_top
        + second.r_bottom
        + second.r_top;
    // Parallelism is certified on the cross-product magnitude (~ sin of the
    // axis angle), never forced: pure-rounding tilts snap, recognition-scale
    // tilts report near_coincidence, larger tilts are unsupported.
    let cross = [
        first.axis[1] * second.axis[2] - first.axis[2] * second.axis[1],
        first.axis[2] * second.axis[0] - first.axis[0] * second.axis[2],
        first.axis[0] * second.axis[1] - first.axis[1] * second.axis[0],
    ];
    let tilt = norm3(cross);
    let angle_snap = 64. * f64::EPSILON * 8.;
    if tilt > angle_snap {
        let reason = if tilt <= RECOGNITION {
            UnresolvedReason::NearCoincidence
        } else {
            UnresolvedReason::UnsupportedSurface
        };
        report.unresolved(domain, reason);
        return Ok(report);
    }
    // Axiality of the two axis lines: perpendicular offset of the second
    // cone's bottom ring center from the first cone's axis line.
    let perp = sub(offset, first.axis.map(|x| x * s0));
    let d_perp = norm3(perp);
    let snap = 64. * f64::EPSILON * terms;
    if d_perp > snap {
        let reason = if d_perp <= RECOGNITION * scale {
            UnresolvedReason::NearCoincidence
        } else {
            UnresolvedReason::UnsupportedSurface
        };
        report.unresolved(domain, reason);
        return Ok(report);
    }
    let sign = if dot(first.axis, second.axis) >= 0. {
        1.
    } else {
        -1.
    };
    let profile = OrientedProfile {
        cone: &second,
        origin: s0,
        sign,
    };
    let lo = 0_f64.max(profile.lo());
    let hi = first.height.min(profile.hi());
    // Ring-plane coincidences (rim/rim, rim-on-cap, apex-on-cap): coaxial
    // disks at the same plane always share at least the smaller disk — a
    // tangent boundary contact, never a guessed circle. Independent of the
    // side classification below.
    let mut tangency = false;
    for p in [0., first.height] {
        for q in [profile.lo(), profile.hi()] {
            if (p - q).abs() <= band {
                tangency = true;
            }
        }
    }
    // Clearly separated axially: no shared point is possible.
    if hi < lo - band {
        if tangency {
            report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        }
        return Ok(report);
    }
    // Overlap collapsed into the band: the facing ring planes touch.
    if hi - lo <= band {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(report);
    }
    let d = |s: f64| (first.r_bottom + first.slope * s) - profile.radius(s);
    let d_lo = d(lo);
    let d_hi = d(hi);
    // Equal-taper test: the taper difference is dimensionless, so the
    // rounding allowance scales with the tapers and the recognition
    // deviations enter through the heights they were measured over.
    let slope_band = 16. * f64::EPSILON * (first.slope.abs() + second.slope.abs() + 1.)
        + (first.error + second.error) / first.height.min(second.height);
    let dm = first.slope - profile.taper();
    if dm.abs() <= slope_band {
        // Parallel profiles: d is constant up to the band over the overlap.
        if d_lo.abs() <= band && d_hi.abs() <= band {
            // Coincident side profiles over the whole overlap: a coincident
            // trim, never a surface component.
            report.unresolved(domain, UnresolvedReason::CoincidentTrim);
        } else if d_lo.abs() <= band || d_hi.abs() <= band || d_lo.signum() != d_hi.signum() {
            // Equality at a single ring plane only (rim tangency), or a
            // crossing whose position is ill-conditioned at equal taper.
            report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        } else if tangency {
            // Strictly separated parallel profiles, but a ring-plane
            // coincidence carries its own boundary contact.
            report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        }
        // Otherwise strictly separated parallel profiles resolve empty.
        return Ok(report);
    }
    // Different tapers: at most one side root s* = lo + d_lo (hi-lo)/(d_lo-d_hi).
    if d_lo.signum() == d_hi.signum() && d_lo.abs() > band && d_hi.abs() > band {
        // Strict same sign at both overlap ends: the linear root is clipped
        // by a finite height — honestly absent.
        if tangency {
            report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        }
        return Ok(report);
    }
    if d_lo.abs() <= band || d_hi.abs() <= band {
        // The root sits within the band of an overlap boundary — every
        // overlap boundary is a ring plane: a rim contact, unresolved.
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(report);
    }
    let s = lo + d_lo * (hi - lo) / (d_lo - d_hi);
    // A root within the band of any of the four ring planes is a rim
    // contact; a root whose radius collapses into the band is the
    // apex-meeting contact — both stay unresolved.
    if [0., first.height, profile.lo(), profile.hi()]
        .iter()
        .any(|&p| (s - p).abs() <= band)
    {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(report);
    }
    let radius = first.r_bottom + first.slope * s;
    if radius <= band {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(report);
    }
    report
        .components
        .push(circle_component(&first, &profile, s)?);
    if tangency {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
    }
    Ok(report)
}
