use super::*;

/// Analytic plane/cone intersection of a canonical planar patch and a
/// canonical conical frustum solid (true-apex cone admitted). Non-canonical
/// operands — including an equal-radius frustum (a cylinder) — are explicit
/// `UnsupportedSurface` regions, never a numerical fallback; tangencies
/// (ring planes, the apex, the parabola threshold band, boundary touches)
/// stay unresolved — tangent contacts are never guessed.
pub fn intersect_plane_cone(
    plane_model: &Model,
    cone_model: &Model,
    options: Options,
) -> Result<Report<PlaneConeComponent>> {
    let options = options.validate()?;
    let _ = options;
    let mut report = Report::default();
    let domain = vec![0., 1., 0., 1., 0., 1., 0., 1.];
    let (Some(plane), Some(cone)) = (recognize_plane(plane_model)?, recognize_cone(cone_model)?)
    else {
        report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    };
    report.boxes_visited = 1;
    let h = cone.height;
    let m = cone.slope;
    let k2 = 1. + m * m;
    let sigma = m.abs() / k2.sqrt();
    let g = dot(cone.axis, plane.normal);
    let ga = g.abs();
    let terms = cone.r_bottom
        + cone.r_top
        + h
        + plane.u_len
        + plane.v_len
        + sub(cone.bottom, plane.origin)
            .iter()
            .map(|v| v.abs())
            .fold(0., f64::max)
        + 1.;
    // Outward binary64 classification band: both recognition deviations plus
    // a rounding allowance on the section arithmetic and the dimensions.
    let band = plane.error + cone.error + 16. * f64::EPSILON * terms;
    // Angular certification: pure-rounding tilts snap, recognition-scale
    // tilts report near_coincidence, clearly oblique angles are exact.
    let angular_snap =
        64. * f64::EPSILON + plane.error / plane.u_len.min(plane.v_len) + cone.error / h;
    let rect = rect_halfplanes(&plane, band);
    // Axial coordinate of the apex from the bottom ring center.
    let t_apex = -cone.r_bottom / m;
    let apex: [f64; 3] = std::array::from_fn(|i| cone.bottom[i] + t_apex * cone.axis[i]);
    if ga >= 1. - angular_snap {
        // Plane perpendicular to the axis: the exact circle at the section
        // height, radius linearly interpolated between the rings.
        let t_s = dot(sub(plane.origin, cone.bottom), plane.normal) / g;
        if !t_s.is_finite() {
            report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
            return Ok(report);
        }
        if t_s < -band || t_s > h + band {
            return Ok(report);
        }
        if t_s.abs() <= band {
            // Bottom ring plane within the band: a coincident cap/rim region
            // (nonzero ring) or the apex contact (apex ring).
            let reason = if cone.r_bottom > band {
                UnresolvedReason::CoincidentTrim
            } else {
                UnresolvedReason::TangencyOrMultipleRoot
            };
            report.unresolved(domain, reason);
            return Ok(report);
        }
        if (t_s - h).abs() <= band {
            let reason = if cone.r_top > band {
                UnresolvedReason::CoincidentTrim
            } else {
                UnresolvedReason::TangencyOrMultipleRoot
            };
            report.unresolved(domain, reason);
            return Ok(report);
        }
        let r_s = cone.r_bottom + m * t_s;
        if !r_s.is_finite() || r_s <= band {
            // Section at the apex level: a point contact, never guessed.
            report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
            return Ok(report);
        }
        let center = std::array::from_fn(|i| cone.bottom[i] + t_s * cone.axis[i]);
        let e1 = plane.u.map(|x| x / plane.u_len);
        let e2 = plane.v.map(|x| x / plane.v_len);
        let [uc, vc] = plane_uv(&plane, center);
        let w1 = [r_s / plane.u_len, 0.];
        let w2 = [0., r_s / plane.v_len];
        let (clip, tangent) = clip_ellipse([uc, vc], w1, w2, &rect);
        if tangent {
            report.unresolved(domain.clone(), UnresolvedReason::TangencyOrMultipleRoot);
        }
        let v0 = t_s / h;
        let point_at = |phi: f64| {
            std::array::from_fn::<f64, 3, _>(|i| {
                center[i] + r_s * (phi.cos() * e1[i] + phi.sin() * e2[i])
            })
        };
        let we1 = e1.map(|x| x * r_s);
        let we2 = e2.map(|x| x * r_s);
        match clip {
            EllipseClip::Empty => {}
            EllipseClip::Full if !tangent => {
                let curve = conic_sweep(center, we1, we2, 0., TAU);
                let max_sample_residual = side_plane_residuals(&curve, &cone, &plane, 16)?;
                report.components.push(PlaneConeComponent::Circle {
                    curve,
                    center,
                    radius: r_s,
                    normal: plane.normal,
                    full: true,
                    plane_uv: vec![PlanePatchCurve {
                        patch: 0,
                        arcs: conic_arcs_2d([uc, vc], w1, w2, 0., TAU),
                    }],
                    cone_uv: lift_circle_side_full(&cone, v0),
                    max_sample_residual,
                });
            }
            EllipseClip::Arcs(intervals) => {
                for (a, b) in intervals {
                    let curve = conic_sweep(center, we1, we2, a, b);
                    let max_sample_residual = side_plane_residuals(&curve, &cone, &plane, 9)?;
                    report.components.push(PlaneConeComponent::Circle {
                        curve,
                        center,
                        radius: r_s,
                        normal: plane.normal,
                        full: false,
                        plane_uv: vec![PlanePatchCurve {
                            patch: 0,
                            arcs: conic_arcs_2d([uc, vc], w1, w2, a, b),
                        }],
                        cone_uv: lift_circle_side(&cone, center, v0, &[(a, b)], point_at),
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
    // Conic kind classification on the outward bands.
    #[derive(Clone, Copy, PartialEq)]
    enum Conic {
        Hyperbola,
        Parabola,
        Ellipse,
    }
    let mut g = g;
    let mut s = (1. - g * g).max(0.).sqrt();
    let kind: Conic;
    if ga <= angular_snap {
        // Plane parallel to the axis. Through the axis (the apex on the
        // plane within the band): the two exact straight rulings.
        let d0 = dot(plane.normal, sub(plane.origin, apex));
        if d0.abs() <= band {
            let e2 = cross(plane.normal, cone.axis);
            let e2_len = norm(e2);
            if !e2_len.is_finite() || e2_len <= 0. {
                report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
                return Ok(report);
            }
            let e2 = e2.map(|x| x / e2_len);
            let wlen = k2.sqrt();
            let band_uv = [band / plane.u_len, band / plane.v_len];
            let apex_uv = plane_uv(&plane, apex);
            for sign in [-1., 1.] {
                let w: [f64; 3] = std::array::from_fn(|i| cone.axis[i] + sign * m * e2[i]);
                let dir_uv = [
                    dot(w, plane.u) / (plane.u_len * plane.u_len),
                    dot(w, plane.v) / (plane.v_len * plane.v_len),
                ];
                match clip_line_rect(apex_uv, dir_uv, -t_apex, h - t_apex, band / wlen, band_uv) {
                    LineClip::Miss => {}
                    LineClip::Tangent => {
                        report.unresolved(domain.clone(), UnresolvedReason::TangencyOrMultipleRoot);
                    }
                    LineClip::Span(lo, hi) => {
                        let start: [f64; 3] = std::array::from_fn(|i| apex[i] + lo * w[i]);
                        let end: [f64; 3] = std::array::from_fn(|i| apex[i] + hi * w[i]);
                        let curve = Curve {
                            degree: 1,
                            knots: vec![0., 0., 1., 1.],
                            control_points: vec![start.to_vec(), end.to_vec()],
                            weights: vec![1., 1.],
                            periodic: false,
                        };
                        let cone_uv = lift_ruling(&cone, w, lo, hi);
                        let plane_uv = vec![PlanePatchCurve {
                            patch: 0,
                            arcs: vec![Curve {
                                degree: 1,
                                knots: vec![0., 0., 1., 1.],
                                control_points: vec![
                                    [apex_uv[0] + lo * dir_uv[0], apex_uv[1] + lo * dir_uv[1]]
                                        .to_vec(),
                                    [apex_uv[0] + hi * dir_uv[0], apex_uv[1] + hi * dir_uv[1]]
                                        .to_vec(),
                                ],
                                weights: vec![1., 1.],
                                periodic: false,
                            }],
                        }];
                        let max_sample_residual = side_plane_residuals(&curve, &cone, &plane, 9)?;
                        report.components.push(PlaneConeComponent::Line {
                            curve,
                            start,
                            end,
                            direction: w.map(|x| x / wlen),
                            contact: Contact::Boundary,
                            plane_uv,
                            cone_uv,
                            max_sample_residual,
                        });
                    }
                }
            }
            return Ok(report);
        }
        // Off-axis parallel plane: the exact hyperbola with g = 0, s = 1.
        g = 0.;
        s = 1.;
        kind = Conic::Hyperbola;
    } else {
        let off_para = (ga - sigma).abs();
        if off_para <= angular_snap {
            // Pure-rounding distance from the parabola threshold: snap to the
            // exact parabola (a plane built exactly parallel to a ruling).
            g = sigma.copysign(g);
            s = (1. - sigma * sigma).sqrt();
            kind = Conic::Parabola;
        } else {
            let near = RECOGNITION + plane.error / plane.u_len.min(plane.v_len) + cone.error / h;
            if ga >= 1. - near || ga <= near {
                // Recognition-scale tilt: cannot be certified — never forced.
                report.unresolved(domain, UnresolvedReason::NearCoincidence);
                return Ok(report);
            }
            if off_para <= near {
                // The parabola threshold band: a degenerate multiple-root
                // boundary, never a guessed conic.
                report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
                return Ok(report);
            }
            kind = if ga > sigma {
                Conic::Ellipse
            } else {
                Conic::Hyperbola
            };
        }
    }
    // Shared oblique geometry, apex-based: plane points are
    // P = Q + x e1 + y e2 with Q the apex projection, e1 the in-plane axis
    // projection; the cone side is A2 x^2 - L x + y^2 + C = 0.
    let d0 = dot(plane.normal, sub(plane.origin, apex));
    if d0.abs() <= band {
        // Plane through the apex region: the apex is a singular point of
        // the side surface — a multiple-root contact, never guessed (the
        // exact through-axis ruling case returned above).
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(report);
    }
    let e1_raw = sub(cone.axis, plane.normal.map(|x| x * g));
    let e1_len = norm(e1_raw);
    if !e1_len.is_finite() || e1_len <= 0. {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(report);
    }
    let e1 = e1_raw.map(|x| x / e1_len);
    let e2 = cross(plane.normal, e1);
    let q: [f64; 3] = std::array::from_fn(|i| apex[i] + d0 * plane.normal[i]);
    let a2 = k2 * (g * g - sigma * sigma);
    let ll = 2. * k2 * s * g * d0;
    let cc = d0 * d0 * (1. - k2 * g * g);
    if !a2.is_finite() || !ll.is_finite() || !cc.is_finite() {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(report);
    }
    match kind {
        Conic::Ellipse => {
            if a2 <= 0. {
                report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
                return Ok(report);
            }
            let x_c = ll / (2. * a2);
            let r2 = ll * ll / (4. * a2) - cc;
            if !r2.is_finite() {
                report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
                return Ok(report);
            }
            if r2 <= band * band {
                if r2 < -band * band {
                    // Provable miss of the double cone: resolved empty.
                    return Ok(report);
                }
                // Degenerate point contact: never a guessed point.
                report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
                return Ok(report);
            }
            let ae = (r2 / a2).sqrt();
            let be = r2.sqrt();
            let center3: [f64; 3] = std::array::from_fn(|i| q[i] + x_c * e1[i]);
            // Finite height, nappe and patch constraints are all linear in
            // the plane UV: one exact eccentric-angle clip.
            let x_q = dot(sub(plane.origin, q), e1);
            let xu = dot(plane.u, e1);
            let xv = dot(plane.v, e1);
            let mut halfplanes = rect.to_vec();
            halfplanes.push(Halfplane2 {
                normal: [s * xu, s * xv],
                offset: h - t_apex - g * d0 - s * x_q,
                band,
            });
            halfplanes.push(Halfplane2 {
                normal: [-s * xu, -s * xv],
                offset: t_apex + g * d0 + s * x_q,
                band,
            });
            halfplanes.push(Halfplane2 {
                normal: [-m * s * xu, -m * s * xv],
                offset: m * (g * d0 + s * x_q),
                band,
            });
            let c2 = plane_uv(&plane, center3);
            let w1 = [
                ae * dot(e1, plane.u) / (plane.u_len * plane.u_len),
                ae * dot(e1, plane.v) / (plane.v_len * plane.v_len),
            ];
            let w2 = [
                be * dot(e2, plane.u) / (plane.u_len * plane.u_len),
                be * dot(e2, plane.v) / (plane.v_len * plane.v_len),
            ];
            let (clip, tangent) = clip_ellipse(c2, w1, w2, &halfplanes);
            if tangent {
                report.unresolved(domain.clone(), UnresolvedReason::TangencyOrMultipleRoot);
            }
            let (semi_major, major, semi_minor, minor) = if ae >= be {
                (ae, e1, be, e2)
            } else {
                (be, e2, ae, e1)
            };
            let we1 = e1.map(|x| x * ae);
            let we2 = e2.map(|x| x * be);
            let mut push_ellipse = |start: f64, end: f64, full: bool| -> Result<()> {
                let curve = conic_sweep(center3, we1, we2, start, end);
                let max_sample_residual = side_plane_residuals(&curve, &cone, &plane, 9)?;
                report.components.push(PlaneConeComponent::Ellipse {
                    curve,
                    center: center3,
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
                    cone_uv: None,
                    max_sample_residual,
                });
                Ok(())
            };
            match clip {
                EllipseClip::Empty => {}
                EllipseClip::Full if !tangent => push_ellipse(0., TAU, true)?,
                EllipseClip::Arcs(intervals) => {
                    for (a, b) in intervals {
                        push_ellipse(a, b, false)?;
                    }
                }
                EllipseClip::Full => {}
            }
            Ok(report)
        }
        Conic::Parabola => {
            // y^2 = L x - C: the exact polynomial quadratic
            // p(t) = (x0 + t^2 / L, t), weights 1.
            if ll.abs() <= 1e-30 * (k2 * s * d0.abs() + 1.) {
                report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
                return Ok(report);
            }
            let x0 = cc / ll;
            let num = [[x0, 0., 1. / ll], [0., 1., 0.]];
            let den = [1., 0., 0.];
            // Finite height, nappe and patch half-planes in (x, y).
            let hps = conic_halfplanes(&plane, &cone, &rect, q, e1, e2, g, s, d0, t_apex, band);
            let big = 1e6 * terms;
            let t_band = band + 1e-12 * terms;
            let (intervals, tangent) = clip_conic_t(num, den, -big, big, &hps, t_band);
            if tangent {
                report.unresolved(domain.clone(), UnresolvedReason::TangencyOrMultipleRoot);
            }
            let vertex: [f64; 3] = std::array::from_fn(|i| q[i] + x0 * e1[i]);
            let direction = if ll > 0. { e1 } else { e1.map(|x| -x) };
            for (t0, t1) in intervals {
                let curve = bezier_rational_3d(conic_num3(q, e1, e2, num, den), den, t0, t1);
                let plane_uv = vec![PlanePatchCurve {
                    patch: 0,
                    arcs: vec![bezier_rational_2d(
                        conic_num_uv(&plane, q, e1, e2, num, den),
                        den,
                        t0,
                        t1,
                    )],
                }];
                let max_sample_residual = side_plane_residuals(&curve, &cone, &plane, 9)?;
                report.components.push(PlaneConeComponent::Parabola {
                    curve,
                    vertex,
                    direction,
                    focal_length: ll.abs() / 4.,
                    normal: plane.normal,
                    plane_uv,
                    cone_uv: None,
                    max_sample_residual,
                });
            }
            Ok(report)
        }
        Conic::Hyperbola => {
            if a2 >= 0. {
                report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
                return Ok(report);
            }
            let x_c = ll / (2. * a2);
            let hh = cc - a2 * x_c * x_c;
            if !hh.is_finite() {
                report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
                return Ok(report);
            }
            if hh <= band * band {
                if hh < -band * band {
                    // Provable miss of the double cone: resolved empty.
                    return Ok(report);
                }
                report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
                return Ok(report);
            }
            let ah = (hh / (-a2)).sqrt();
            let bh = hh.sqrt();
            let center3: [f64; 3] = std::array::from_fn(|i| q[i] + x_c * e1[i]);
            let hps = conic_halfplanes(&plane, &cone, &rect, q, e1, e2, g, s, d0, t_apex, band);
            for br in [-1., 1.] {
                // x = x_c + br a (1+t^2)/(1-t^2), y = 2 b t/(1-t^2), |t| < 1.
                let num = [[x_c + br * ah, 0., br * ah - x_c], [0., 2. * bh, 0.]];
                let den = [1., 0., -1.];
                let (intervals, tangent) = clip_conic_t(num, den, -1., 1., &hps, 1e-9);
                if tangent {
                    report.unresolved(domain.clone(), UnresolvedReason::TangencyOrMultipleRoot);
                }
                for (t0, t1) in intervals {
                    if t0 < -1. + 1e-9 || t1 > 1. - 1e-9 {
                        // Arc runs into the asymptote: never a guessed
                        // unbounded trim.
                        report.unresolved(domain.clone(), UnresolvedReason::TangencyOrMultipleRoot);
                        continue;
                    }
                    let curve = bezier_rational_3d(conic_num3(q, e1, e2, num, den), den, t0, t1);
                    let plane_uv = vec![PlanePatchCurve {
                        patch: 0,
                        arcs: vec![bezier_rational_2d(
                            conic_num_uv(&plane, q, e1, e2, num, den),
                            den,
                            t0,
                            t1,
                        )],
                    }];
                    let max_sample_residual = side_plane_residuals(&curve, &cone, &plane, 9)?;
                    report.components.push(PlaneConeComponent::Hyperbola {
                        curve,
                        center: center3,
                        semi_transverse: ah,
                        semi_conjugate: bh,
                        transverse: e1.map(|x| x * br),
                        conjugate: e2,
                        normal: plane.normal,
                        plane_uv,
                        cone_uv: None,
                        max_sample_residual,
                    });
                }
            }
            Ok(report)
        }
    }
}

/// Finite height, nappe and patch-rectangle half-planes in the plane's
/// (x, y) coordinates (x along the in-plane axis projection e1, y along
/// e2 = normal x e1, origin at the apex projection Q).
#[allow(clippy::too_many_arguments)]
pub(super) fn conic_halfplanes(
    plane: &CanonicalPlane,
    cone: &CanonicalCone,
    rect: &[Halfplane2; 4],
    q: [f64; 3],
    e1: [f64; 3],
    e2: [f64; 3],
    g: f64,
    s: f64,
    d0: f64,
    t_apex: f64,
    band: f64,
) -> Vec<Halfplane2> {
    let m = cone.slope;
    let mut hps = vec![
        // Height: 0 <= t_apex + g d0 + s x <= h.
        Halfplane2 {
            normal: [s, 0.],
            offset: cone.height - t_apex - g * d0,
            band,
        },
        Halfplane2 {
            normal: [-s, 0.],
            offset: t_apex + g * d0,
            band,
        },
        // Nappe: m (g d0 + s x) >= 0.
        Halfplane2 {
            normal: [-m * s, 0.],
            offset: m * g * d0,
            band,
        },
    ];
    // Patch rectangle edges, affine in (x, y): u = u_q + x au + y bu.
    let u2 = plane.u_len * plane.u_len;
    let v2 = plane.v_len * plane.v_len;
    let au = dot(e1, plane.u) / u2;
    let bu = dot(e2, plane.u) / u2;
    let av = dot(e1, plane.v) / v2;
    let bv = dot(e2, plane.v) / v2;
    let u_q = dot(sub(q, plane.origin), plane.u) / u2;
    let v_q = dot(sub(q, plane.origin), plane.v) / v2;
    let _ = rect;
    hps.push(Halfplane2 {
        normal: [au, bu],
        offset: 1. - u_q,
        band: band / plane.u_len,
    });
    hps.push(Halfplane2 {
        normal: [-au, -bu],
        offset: u_q,
        band: band / plane.u_len,
    });
    hps.push(Halfplane2 {
        normal: [av, bv],
        offset: 1. - v_q,
        band: band / plane.v_len,
    });
    hps.push(Halfplane2 {
        normal: [-av, -bv],
        offset: v_q,
        band: band / plane.v_len,
    });
    hps
}

/// Homogeneous 3D numerator of Q + x(t) e1 + y(t) e2 with
/// (x, y) = num / den.
pub(super) fn conic_num3(
    q: [f64; 3],
    e1: [f64; 3],
    e2: [f64; 3],
    num: [[f64; 3]; 2],
    den: [f64; 3],
) -> [[f64; 3]; 3] {
    std::array::from_fn(|k| {
        [
            q[k] * den[0] + e1[k] * num[0][0] + e2[k] * num[1][0],
            q[k] * den[1] + e1[k] * num[0][1] + e2[k] * num[1][1],
            q[k] * den[2] + e1[k] * num[0][2] + e2[k] * num[1][2],
        ]
    })
}
