use super::*;

#[derive(Clone, Debug)]
pub enum PlaneCylinderComponent {
    /// Plane perpendicular to the axis: the exact circle (full, or clipped
    /// to the finite plane domain) with lifts on both surfaces.
    Circle {
        /// Exact rational quadratic sweep (full circle: four 90-degree arcs,
        /// weights cos(pi/4), knots 0..=4).
        curve: Curve,
        center: [f64; 3],
        radius: f64,
        /// Unit plane normal.
        normal: [f64; 3],
        /// True when the circle lies fully inside the plane patch domain.
        full: bool,
        plane_uv: Vec<PlanePatchCurve>,
        /// Iso-v circle arcs on the side patches.
        cylinder_uv: Vec<CylinderPatchCurve>,
        max_sample_residual: f64,
    },
    /// Plane parallel to the axis: one exact straight ruling clipped by the
    /// finite height and the patch rectangle.
    Line {
        /// Exact degree-1 line, knots [0,0,1,1], unit weights, start -> end.
        curve: Curve,
        start: [f64; 3],
        end: [f64; 3],
        /// Unit cylinder axis (the line direction).
        direction: [f64; 3],
        /// Clip endpoints sit on cap planes or the patch boundary.
        contact: Contact,
        plane_uv: Vec<PlanePatchCurve>,
        /// Iso-u segments on the cylinder side patches (two entries when the
        /// ruling sits on a quadrant seam).
        cylinder_uv: Vec<CylinderPatchCurve>,
        max_sample_residual: f64,
    },
    /// Oblique plane: the exact ellipse (full, or clipped to the finite
    /// height and patch domain) with the exact plane-UV lift. The cylinder
    /// side lift is null: the unrolled ellipse is a cosine curve with no
    /// exact low-degree rational UV representation (documented in the module
    /// header); the 3D ellipse is exact regardless.
    Ellipse {
        /// Exact rational quadratic sweep over the eccentric-angle interval.
        curve: Curve,
        center: [f64; 3],
        /// Semi-major R / |axis.n| along `major`.
        semi_major: f64,
        /// Semi-minor R along `minor`.
        semi_minor: f64,
        /// Unit in-plane direction of the axis projection (semi-major axis).
        major: [f64; 3],
        /// Unit in-plane direction perpendicular to it (semi-minor axis).
        minor: [f64; 3],
        /// Unit plane normal.
        normal: [f64; 3],
        /// True when the full ellipse survives every clip.
        full: bool,
        plane_uv: Vec<PlanePatchCurve>,
        cylinder_uv: Option<Vec<CylinderPatchCurve>>,
        max_sample_residual: f64,
    },
}


/// Plane UV coordinates of a 3D point under the rectangular patch frame.
pub(super) fn plane_uv(plane: &CanonicalPlane, point: [f64; 3]) -> [f64; 2] {
    let rel = sub(point, plane.origin);
    [
        dot(rel, plane.u) / (plane.u_len * plane.u_len),
        dot(rel, plane.v) / (plane.v_len * plane.v_len),
    ]
}

/// Rectangle half-planes of the patch domain with per-axis bands.
pub(super) fn rect_halfplanes(plane: &CanonicalPlane, band: f64) -> [Halfplane2; 4] {
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

/// Worst residual over `count` curve samples against the cylinder side
/// equation and the plane equation.
pub(super) fn side_plane_residuals(
    curve: &Curve,
    cylinder: &CanonicalCylinder,
    plane: &CanonicalPlane,
    count: usize,
) -> Result<f64> {
    let [lo, hi] = curve.domain();
    let mut worst = 0_f64;
    for i in 0..count {
        let t = lo + (hi - lo) * i as f64 / (count - 1) as f64;
        let p = point3(&curve.evaluate(t)?.point);
        let rel = sub(p, cylinder.center);
        let a = dot(rel, cylinder.axis);
        let perp = sub(rel, cylinder.axis.map(|x| x * a));
        let side = (perp[0].hypot(perp[1]).hypot(perp[2]) - cylinder.radius).abs();
        let planar = dot(plane.normal, sub(p, plane.origin)).abs();
        worst = worst.max(side).max(planar);
    }
    Ok(worst)
}

/// Clipped iso-v circle lifts on the side patches for a section circle whose
/// swept parameter intervals are `arcs`. `point_at(phi)` evaluates the exact
/// 3D circle point; the angular direction per arc is fixed by the exact
/// midpoint image, and the per-quadrant trims use the exact rational-arc
/// parameter (a quadratic solve in tan(theta), never a linear fraction).
pub(super) fn lift_circle_side(
    cylinder: &CanonicalCylinder,
    center: [f64; 3],
    v0: f64,
    arcs: &[(f64, f64)],
    point_at: impl Fn(f64) -> [f64; 3],
) -> Vec<CylinderPatchCurve> {
    let mut lifted = Vec::new();
    for &(a, b) in arcs {
        let sweep = b - a;
        if !sweep.is_finite() || sweep <= 0. {
            continue;
        }
        let theta = |phi: f64| {
            let rel = sub(point_at(phi), center);
            dot(rel, cylinder.frame[1])
                .atan2(dot(rel, cylinder.frame[0]))
                .rem_euclid(TAU)
        };
        let (ta, tb, tm) = (theta(a), theta(b), theta(a + sweep / 2.));
        let forward = (tb - ta).rem_euclid(TAU);
        let image = if (tm - ta).rem_euclid(TAU) <= forward {
            (ta, forward)
        } else {
            (tb, (ta - tb).rem_euclid(TAU))
        };
        for (quadrant, patch) in cylinder.sides.iter().enumerate() {
            let span = (quadrant as f64 * QUARTER, QUARTER);
            for (s0, sw) in ccw_intersect(span, image) {
                let rel0 = s0 - quadrant as f64 * QUARTER;
                let u0 = arc_parameter(rel0);
                let u1 = arc_parameter(rel0 + sw);
                if u1 - u0 <= 1e-12 {
                    continue;
                }
                lifted.push(CylinderPatchCurve {
                    patch: *patch,
                    arcs: vec![Curve {
                        degree: 1,
                        knots: vec![0., 0., 1., 1.],
                        control_points: vec![[u0, v0].to_vec(), [u1, v0].to_vec()],
                        weights: vec![1., 1.],
                        periodic: false,
                    }],
                });
            }
        }
    }
    lifted
}

/// Iso-v lines on all four side patches (full circle at height v0).
pub(super) fn lift_circle_side_full(cylinder: &CanonicalCylinder, v0: f64) -> Vec<CylinderPatchCurve> {
    cylinder
        .sides
        .iter()
        .map(|&patch| CylinderPatchCurve {
            patch,
            arcs: vec![Curve {
                degree: 1,
                knots: vec![0., 0., 1., 1.],
                control_points: vec![[0., v0].to_vec(), [1., v0].to_vec()],
                weights: vec![1., 1.],
                periodic: false,
            }],
        })
        .collect()
}

/// Clip the line `foot_uv + t * dir_uv` (t in model units along the axis) to
/// `[lo0, hi0]` and the unit square. Returns the surviving interval, or the
/// reason it fails: misses are silent, boundary-coincident lines, band-thin
/// clips and patch-corner touches are tangencies.
pub(super) enum LineClip {
    Miss,
    Tangent,
    Span(f64, f64),
}

pub(super) fn clip_line_rect(
    foot_uv: [f64; 2],
    dir_uv: [f64; 2],
    lo0: f64,
    hi0: f64,
    band: f64,
    band_uv: [f64; 2],
) -> LineClip {
    let mut lo = lo0;
    let mut hi = hi0;
    for k in 0..2 {
        let f = foot_uv[k];
        let d = dir_uv[k];
        if d.abs() <= 1e-30 {
            // Line parallel to this axis' domain edges.
            if f < -band_uv[k] || f > 1. + band_uv[k] {
                return LineClip::Miss;
            }
            if f.abs() <= band_uv[k] || (f - 1.).abs() <= band_uv[k] {
                // The line runs along the patch boundary within the band.
                return LineClip::Tangent;
            }
            continue;
        }
        let t0 = -f / d;
        let t1 = (1. - f) / d;
        lo = lo.max(t0.min(t1));
        hi = hi.min(t0.max(t1));
    }
    if hi < lo - band {
        return LineClip::Miss;
    }
    if hi - lo <= band {
        // Band-thin clip: a boundary touch, never a degenerate segment.
        return LineClip::Tangent;
    }
    // Patch-corner touch: a clip endpoint within the band of an edge in both
    // UV axes is a tangency, never a guessed corner segment endpoint.
    for t in [lo, hi] {
        let q = [foot_uv[0] + t * dir_uv[0], foot_uv[1] + t * dir_uv[1]];
        let near_u = q[0].abs() <= band_uv[0] || (q[0] - 1.).abs() <= band_uv[0];
        let near_v = q[1].abs() <= band_uv[1] || (q[1] - 1.).abs() <= band_uv[1];
        if near_u && near_v {
            return LineClip::Tangent;
        }
    }
    LineClip::Span(lo, hi)
}

/// Analytic plane/cylinder intersection of a canonical planar patch and a
/// canonical cylinder solid. Non-canonical operands are explicit
/// `UnsupportedSurface` regions, never a numerical fallback; tangencies
/// (d == R, cap-plane coincidence, domain-edge and cap-plane touches) stay
/// unresolved — tangent contacts are never guessed.
pub fn intersect_plane_cylinder(
    plane_model: &Model,
    cylinder_model: &Model,
    options: Options,
) -> Result<Report<PlaneCylinderComponent>> {
    let options = options.validate()?;
    let _ = options;
    let mut report = Report::default();
    let domain = vec![0., 1., 0., 1., 0., 1., 0., 1.];
    let (Some(plane), Some(cylinder)) = (
        recognize_plane(plane_model)?,
        recognize_cylinder(cylinder_model)?,
    ) else {
        report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    };
    report.boxes_visited = 1;
    let g = dot(cylinder.axis, plane.normal);
    let terms = cylinder.radius
        + cylinder.half_height
        + plane.u_len
        + plane.v_len
        + sub(cylinder.center, plane.origin)
            .iter()
            .map(|v| v.abs())
            .fold(0., f64::max)
        + 1.;
    // Outward binary64 classification band: both recognition deviations plus
    // a rounding allowance on the section arithmetic and the dimensions.
    let band = plane.error + cylinder.error + 16. * f64::EPSILON * terms;
    // Angular certification: pure-rounding tilts snap, recognition-scale
    // tilts report near_coincidence, clearly oblique angles are exact.
    let angular_snap = 64. * f64::EPSILON
        + plane.error / plane.u_len.min(plane.v_len)
        + cylinder.error / (2. * cylinder.half_height);
    let ga = g.abs();
    let rect = rect_halfplanes(&plane, band);
    if ga >= 1. - angular_snap {
        // Plane perpendicular to the axis: exact circle at the section
        // height, clipped by the patch domain.
        let axial = dot(sub(plane.origin, cylinder.center), plane.normal) / g;
        let half = cylinder.half_height;
        if axial.abs() > half + band {
            return Ok(report);
        }
        if (axial.abs() - half).abs() <= band {
            // The plane coincides with a cap plane within the band: the
            // coincident cap/rim region is never a guessed circle.
            report.unresolved(domain, UnresolvedReason::CoincidentTrim);
            return Ok(report);
        }
        let center = std::array::from_fn(|k| cylinder.center[k] + axial * cylinder.axis[k]);
        let r = cylinder.radius;
        let e1 = plane.u.map(|x| x / plane.u_len);
        let e2 = plane.v.map(|x| x / plane.v_len);
        let [uc, vc] = plane_uv(&plane, center);
        let w1 = [r / plane.u_len, 0.];
        let w2 = [0., r / plane.v_len];
        let (clip, tangent) = clip_ellipse([uc, vc], w1, w2, &rect);
        if tangent {
            report.unresolved(domain.clone(), UnresolvedReason::TangencyOrMultipleRoot);
        }
        let v0 = (axial + half) / (2. * half);
        let point_at = |phi: f64| {
            std::array::from_fn::<f64, 3, _>(|k| {
                center[k] + r * (phi.cos() * e1[k] + phi.sin() * e2[k])
            })
        };
        let we1 = e1.map(|x| x * r);
        let we2 = e2.map(|x| x * r);
        match clip {
            EllipseClip::Empty => {}
            EllipseClip::Full if !tangent => {
                let curve = conic_sweep(center, we1, we2, 0., TAU);
                let max_sample_residual = side_plane_residuals(&curve, &cylinder, &plane, 16)?;
                report.components.push(PlaneCylinderComponent::Circle {
                    curve,
                    center,
                    radius: r,
                    normal: plane.normal,
                    full: true,
                    plane_uv: vec![PlanePatchCurve {
                        patch: 0,
                        arcs: conic_arcs_2d([uc, vc], w1, w2, 0., TAU),
                    }],
                    cylinder_uv: lift_circle_side_full(&cylinder, v0),
                    max_sample_residual,
                });
            }
            EllipseClip::Arcs(intervals) => {
                for (a, b) in intervals {
                    let curve = conic_sweep(center, we1, we2, a, b);
                    let max_sample_residual = side_plane_residuals(&curve, &cylinder, &plane, 9)?;
                    report.components.push(PlaneCylinderComponent::Circle {
                        curve,
                        center,
                        radius: r,
                        normal: plane.normal,
                        full: false,
                        plane_uv: vec![PlanePatchCurve {
                            patch: 0,
                            arcs: conic_arcs_2d([uc, vc], w1, w2, a, b),
                        }],
                        cylinder_uv: lift_circle_side(&cylinder, center, v0, &[(a, b)], point_at),
                        max_sample_residual,
                    });
                }
            }
            // Tangency within the band: the full circle is withheld; the
            // unresolved region above carries the contact.
            EllipseClip::Full => {}
        }
        return Ok(report);
    }
    if ga <= angular_snap {
        // Plane parallel to the axis: two exact rulings at chord distance.
        let s0 = dot(plane.normal, sub(cylinder.center, plane.origin));
        let d = s0.abs();
        let r = cylinder.radius;
        if d > r + band {
            return Ok(report);
        }
        if (d - r).abs() <= band {
            report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
            return Ok(report);
        }
        let h2 = r * r - d * d;
        if !h2.is_finite() || h2 <= 0. {
            report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
            return Ok(report);
        }
        let h = h2.sqrt();
        let e = cross(cylinder.axis, plane.normal);
        let e_len = e[0].hypot(e[1]).hypot(e[2]);
        if !e_len.is_finite() || e_len <= 0. {
            report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
            return Ok(report);
        }
        let e = e.map(|x| x / e_len);
        let half = cylinder.half_height;
        let band_uv = [band / plane.u_len, band / plane.v_len];
        let du = dot(cylinder.axis, plane.u) / (plane.u_len * plane.u_len);
        let dv = dot(cylinder.axis, plane.v) / (plane.v_len * plane.v_len);
        for sign in [-1., 1.] {
            let foot: [f64; 3] = std::array::from_fn(|k| {
                cylinder.center[k] - s0 * plane.normal[k] + sign * h * e[k]
            });
            let foot_uv = plane_uv(&plane, foot);
            match clip_line_rect(foot_uv, [du, dv], -half, half, band, band_uv) {
                LineClip::Miss => {}
                LineClip::Tangent => {
                    report.unresolved(domain.clone(), UnresolvedReason::TangencyOrMultipleRoot);
                }
                LineClip::Span(lo, hi) => {
                    let start: [f64; 3] = std::array::from_fn(|k| foot[k] + lo * cylinder.axis[k]);
                    let end: [f64; 3] = std::array::from_fn(|k| foot[k] + hi * cylinder.axis[k]);
                    let curve = Curve {
                        degree: 1,
                        knots: vec![0., 0., 1., 1.],
                        control_points: vec![start.to_vec(), end.to_vec()],
                        weights: vec![1., 1.],
                        periodic: false,
                    };
                    let cylinder_uv = lift_line(&cylinder, foot, cylinder.axis, lo, hi);
                    let plane_uv = vec![PlanePatchCurve {
                        patch: 0,
                        arcs: vec![Curve {
                            degree: 1,
                            knots: vec![0., 0., 1., 1.],
                            control_points: vec![
                                [foot_uv[0] + lo * du, foot_uv[1] + lo * dv].to_vec(),
                                [foot_uv[0] + hi * du, foot_uv[1] + hi * dv].to_vec(),
                            ],
                            weights: vec![1., 1.],
                            periodic: false,
                        }],
                    }];
                    let max_sample_residual = side_plane_residuals(&curve, &cylinder, &plane, 9)?;
                    report.components.push(PlaneCylinderComponent::Line {
                        curve,
                        start,
                        end,
                        direction: cylinder.axis,
                        contact: Contact::Boundary,
                        plane_uv,
                        cylinder_uv,
                        max_sample_residual,
                    });
                }
            }
        }
        return Ok(report);
    }
    if ga >= 1. - RECOGNITION || ga <= RECOGNITION {
        // Recognition-scale tilt: parallelism/perpendicularity cannot be
        // certified against the recognition deviations — never forced.
        report.unresolved(domain, UnresolvedReason::NearCoincidence);
        return Ok(report);
    }
    // Clearly oblique: the exact ellipse with semi-minor R and semi-major
    // R / |axis.n|, clipped by the finite height and the patch domain in one
    // exact eccentric-angle clip (every constraint is linear in plane UV).
    let m = sub(cylinder.axis, plane.normal.map(|x| x * g));
    let s = m[0].hypot(m[1]).hypot(m[2]);
    if !s.is_finite() || s <= 0. {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(report);
    }
    let major = m.map(|x| x / s);
    let minor = cross(plane.normal, major);
    let semi_major = cylinder.radius / ga;
    let semi_minor = cylinder.radius;
    let t = dot(sub(plane.origin, cylinder.center), plane.normal) / g;
    let center = std::array::from_fn(|k| cylinder.center[k] + t * cylinder.axis[k]);
    let uh = plane.u.map(|x| x / plane.u_len);
    let vh = plane.v.map(|x| x / plane.v_len);
    let w1 = [
        semi_major * dot(major, uh) / plane.u_len,
        semi_major * dot(major, vh) / plane.v_len,
    ];
    let w2 = [
        semi_minor * dot(minor, uh) / plane.u_len,
        semi_minor * dot(minor, vh) / plane.v_len,
    ];
    let c2 = plane_uv(&plane, center);
    // The finite height as two half-planes in patch UV: the axial coordinate
    // of origin + u*U + v*V is a0 + u*au + v*av (model units).
    let a0 = dot(sub(plane.origin, cylinder.center), cylinder.axis);
    let au = dot(plane.u, cylinder.axis);
    let av = dot(plane.v, cylinder.axis);
    let half = cylinder.half_height;
    let mut halfplanes = rect.to_vec();
    halfplanes.push(Halfplane2 {
        normal: [au, av],
        offset: half - a0,
        band,
    });
    halfplanes.push(Halfplane2 {
        normal: [-au, -av],
        offset: half + a0,
        band,
    });
    let (clip, tangent) = clip_ellipse(c2, w1, w2, &halfplanes);
    if tangent {
        report.unresolved(domain.clone(), UnresolvedReason::TangencyOrMultipleRoot);
    }
    let we1 = major.map(|x| x * semi_major);
    let we2 = minor.map(|x| x * semi_minor);
    let push_ellipse = |report: &mut Report<PlaneCylinderComponent>,
                        start: f64,
                        end: f64,
                        full: bool|
     -> Result<()> {
        let curve = conic_sweep(center, we1, we2, start, end);
        let max_sample_residual = side_plane_residuals(&curve, &cylinder, &plane, 9)?;
        report.components.push(PlaneCylinderComponent::Ellipse {
            curve,
            center,
            semi_major,
            semi_minor,
            major,
            minor,
            normal: plane.normal,
            full,
            plane_uv: vec![PlanePatchCurve {
                patch: 0,
                arcs: conic_arcs_2d(c2, w1, w2, start, end),
            }],
            cylinder_uv: None,
            max_sample_residual,
        });
        Ok(())
    };
    match clip {
        EllipseClip::Empty => {}
        EllipseClip::Full if !tangent => push_ellipse(&mut report, 0., TAU, true)?,
        EllipseClip::Arcs(intervals) => {
            for (a, b) in intervals {
                push_ellipse(&mut report, a, b, false)?;
            }
        }
        // Tangency within the band: the full ellipse is withheld; the
        // unresolved region above carries the contact.
        EllipseClip::Full => {}
    }
    Ok(report)
}
