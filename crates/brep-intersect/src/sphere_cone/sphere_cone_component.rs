use super::*;

#[derive(Clone, Debug)]
pub enum SphereConeComponent {
    /// Transverse intersection: the exact circle plus both UV lifts.
    Circle {
        /// Exact rational full circle: degree 2, knots 0..=4, four 90-degree
        /// arcs with weights cos(pi/4).
        curve: Curve,
        center: [f64; 3],
        radius: f64,
        /// Unit cone axis (the circle plane normal).
        normal: [f64; 3],
        sphere_uv: Vec<SpherePatchCircle>,
        cone_uv: Vec<CylinderPatchCurve>,
        /// Worst residual over 16 circle samples against the sphere equation
        /// and the cone side profile or cap plane equation.
        max_sample_residual: f64,
    },
    /// Off-axis contact of a sphere strictly between the cap planes: a
    /// closed quartic loop on the side wall, traced into cubic pieces and
    /// certified within `distance_bound` of both surfaces; `complete` says
    /// whether the component set is certified complete. No UV lifts yet.
    Traced {
        pieces: Vec<Curve>,
        distance_bound: f64,
        spans: usize,
        complete: bool,
    },
}

/// Where a resolved circle sits on the cone boundary.
#[derive(Clone, Copy)]
pub(super) enum CircleSite {
    /// On the side wall: radius is the linearly interpolated rho(axial).
    Side,
    /// On a ring cap disk (slot 0 bottom, slot 1 top) with the circle radius.
    Cap(usize, f64),
}


/// Exact circle component at axial position `axial` relative to the bottom
/// ring center, with UV lifts on both surfaces and a 16-sample residual
/// bound.
pub(super) fn circle_component(
    sphere: &CanonicalSphere,
    cone: &CanonicalCone,
    axial: f64,
    site: CircleSite,
) -> Result<SphereConeComponent> {
    let center = std::array::from_fn(|k| cone.bottom[k] + axial * cone.axis[k]);
    let (radius, cone_uv) = match site {
        CircleSite::Side => {
            // Side patches: u sweeps the quadrant, v runs bottom ring to top
            // ring, so the circle is the iso-v line v = axial / h on each.
            let v0 = axial / cone.height;
            let radius = cone.r_bottom + cone.slope * axial;
            let arcs = vec![Curve {
                degree: 1,
                knots: vec![0., 0., 1., 1.],
                control_points: vec![[0., v0].to_vec(), [1., v0].to_vec()],
                weights: vec![1., 1.],
                periodic: false,
            }];
            (
                radius,
                cone.sides
                    .iter()
                    .map(|&patch| CylinderPatchCurve {
                        patch,
                        arcs: arcs.clone(),
                    })
                    .collect::<Vec<_>>(),
            )
        }
        CircleSite::Cap(slot, rho) => {
            let face = cone.caps[slot].ok_or_else(|| invalid("cap circle needs a cap face"))?;
            let ring_r = if slot == 0 { cone.r_bottom } else { cone.r_top };
            // Cap UV maps the plane as [1/2 + x/(2R), 1/2 + y/(2R)] at that
            // ring's radius: an exact UV circle of radius rho/(2R).
            (
                rho,
                vec![CylinderPatchCurve {
                    patch: face,
                    arcs: circle_arcs([0.5, 0.5], rho / (2. * ring_r), 0., TAU),
                }],
            )
        }
    };
    let curve = circle_curve(center, radius, cone.frame[0], cone.frame[1]);
    let sphere_uv = lift(sphere, cone.axis, center);
    let mut max_sample_residual = 0_f64;
    for i in 0..16 {
        let point = point3(&curve.evaluate(i as f64 / 4.)?.point);
        let d = sub(point, sphere.center);
        let sphere_residual = (d[0].hypot(d[1]).hypot(d[2]) - sphere.radius).abs();
        let other = match site {
            CircleSite::Side => {
                let rel = sub(point, cone.bottom);
                let a = dot(rel, cone.axis);
                let perp = sub(rel, cone.axis.map(|x| x * a));
                (perp[0].hypot(perp[1]).hypot(perp[2]) - (cone.r_bottom + cone.slope * a)).abs()
            }
            CircleSite::Cap(..) => (dot(sub(point, cone.bottom), cone.axis) - axial).abs(),
        };
        max_sample_residual = max_sample_residual.max(sphere_residual).max(other);
    }
    Ok(SphereConeComponent::Circle {
        curve,
        center,
        radius,
        normal: cone.axis,
        sphere_uv,
        cone_uv,
        max_sample_residual,
    })
}

/// Analytic sphere/cone intersection of a canonical sphere solid and a
/// canonical conical frustum solid, axial configuration only. Non-canonical
/// operands and clearly off-axis pairs are explicit unsupported regions,
/// never a numerical fallback; near-axial offsets, the side tangency
/// (double-root discriminant), rim contacts, apex contacts and cap-plane
/// tangencies stay unresolved — tangent contacts are never guessed.
pub fn intersect_sphere_cone(
    sphere_model: &Model,
    cone_model: &Model,
    options: Options,
) -> Result<Report<SphereConeComponent>> {
    let options = options.validate()?;
    let _ = options;
    let mut report = Report::default();
    let domain = vec![0., 1., 0., 1., 0., 1., 0., 1.];
    let (Some(sphere), Some(cone)) = (
        sphere_sphere::recognize_any(sphere_model)?,
        recognize_cone(cone_model)?,
    ) else {
        report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    };
    report.boxes_visited = 1;
    let offset = sub(sphere.center, cone.bottom);
    let s0 = dot(offset, cone.axis);
    let perp = sub(offset, cone.axis.map(|x| x * s0));
    let pscale = perp.iter().map(|v| v.abs()).fold(0., f64::max);
    let d_perp = if pscale == 0. {
        0.
    } else {
        let scaled = perp.map(|x| x / pscale);
        scaled[0].hypot(scaled[1]).hypot(scaled[2]) * pscale
    };
    let terms = s0.abs() + cone.height + sphere.radius + cone.r_bottom + cone.r_top + 1.;
    // Outward binary64 classification band: both recognition deviations plus
    // a rounding allowance on the axial coordinates and radii.
    let band = sphere.error + cone.error + 16. * f64::EPSILON * terms;
    // Axiality is certified, never forced: pure-rounding offsets snap to the
    // axis, recognition-scale offsets report near_coincidence, and anything
    // larger is the unsupported general (quartic) configuration.
    let snap = 64. * f64::EPSILON * terms;
    if d_perp > snap {
        let near = RECOGNITION * (sphere.radius + cone.r_bottom + cone.r_top + cone.height);
        if d_perp <= near {
            report.unresolved(domain, UnresolvedReason::NearCoincidence);
            return Ok(report);
        }
        return off_axis(report, domain, sphere_model, &sphere, &cone, s0, d_perp, band, terms);
    }
    let r = sphere.radius;
    let h = cone.height;
    let m = cone.slope;
    // Sphere provably beyond the whole axial extent: no contact in any
    // radius relation (every boundary point sits in [0, h]).
    if s0 - r > h + band || s0 + r < -band {
        return Ok(report);
    }
    let mut tangency = false;
    let mut found: Vec<(f64, CircleSite)> = Vec::new();
    // Side contacts: rho(t) = r_bottom + m t; with q = t - s0 the sphere
    // equation is (1+m^2) q^2 + 2 rho_c m q + (rho_c^2 - r^2) = 0, whose
    // discriminant 4 (r^2 (1+m^2) - rho_c^2) compares r against the distance
    // |rho_c| / sqrt(1+m^2) from the sphere center to the side line.
    let rho_c = cone.r_bottom + m * s0;
    let k2 = 1. + m * m;
    let dist = rho_c.abs() / k2.sqrt();
    if (r - dist).abs() <= band {
        // Tangent to the side line: a double root. Only a tangency whose
        // foot lands on the finite side segment belongs to this surface;
        // beyond it the rim bands below own the contact.
        let foot = s0 - rho_c * m / k2;
        if (-band..=h + band).contains(&foot) {
            tangency = true;
        }
    } else if r > dist {
        let disc = (r * r * k2 - rho_c * rho_c).max(0.).sqrt();
        for sign in [1., -1.] {
            let q = (-rho_c * m + sign * disc) / k2;
            let t = s0 + q;
            if !(-band..=h + band).contains(&t) {
                continue;
            }
            // A root within the band of a ring plane is the rim contact.
            if t.abs() <= band || (t - h).abs() <= band {
                tangency = true;
                continue;
            }
            // A root whose radius collapses into the band is the apex
            // contact: the sphere passes through the singular apex point.
            let rho = cone.r_bottom + m * t;
            if rho <= band {
                tangency = true;
                continue;
            }
            found.push((t, CircleSite::Side));
        }
    }
    // Rim bands: the whole ring circle lies on the sphere — a tangent
    // boundary contact, never a guessed circle (independent of which branch
    // above ran).
    for &(t_ring, r_ring) in &[(0., cone.r_bottom), (h, cone.r_top)] {
        let d = t_ring - s0;
        let rim = (d * d + r_ring * r_ring).sqrt();
        if (rim - r).abs() <= band {
            tangency = true;
        }
    }
    // Cap-plane contacts: a circle strictly inside the disk is exact; a
    // plane touch or a rim equality stays a tangency region.
    for slot in 0..2 {
        if cone.caps[slot].is_none() {
            continue;
        }
        let t_cap = if slot == 0 { 0. } else { h };
        let r_ring = if slot == 0 { cone.r_bottom } else { cone.r_top };
        let d = (t_cap - s0).abs();
        if d > r + band {
            continue;
        }
        if (d - r).abs() <= band {
            tangency = true;
            continue;
        }
        let rho2 = r * r - d * d;
        if !rho2.is_finite() || rho2 <= 0. {
            tangency = true;
            continue;
        }
        let rho = rho2.sqrt();
        if rho > r_ring + band {
            continue;
        }
        if (rho - r_ring).abs() <= band {
            tangency = true;
            continue;
        }
        found.push((t_cap, CircleSite::Cap(slot, rho)));
    }
    if tangency {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
    }
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (axial, site) in found {
        report
            .components
            .push(circle_component(&sphere, &cone, axial, site)?);
    }
    Ok(report)
}

/// Off-axis pair. Only a sphere strictly between the cap planes is handled:
/// the caps (and the apex, and the other nappe) stay untouched, so every
/// contact is a closed loop on the side wall. Provable misses resolve empty.
#[allow(clippy::too_many_arguments)]
fn off_axis(
    mut report: Report<SphereConeComponent>,
    domain: Vec<f64>,
    sphere_model: &Model,
    sphere: &CanonicalSphere,
    cone: &CanonicalCone,
    s0: f64,
    d_perp: f64,
    band: f64,
    terms: f64,
) -> Result<Report<SphereConeComponent>> {
    let r = sphere.radius;
    if s0 - r <= band || s0 + r >= cone.height - band {
        report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    }
    let radius_at = |z: f64| cone.r_bottom + cone.slope * z;
    let (c_lo, c_hi) = {
        let (a, b) = (radius_at(s0 - r), radius_at(s0 + r));
        (a.min(b), a.max(b))
    };
    // Points of the sphere are within [d_perp - r, d_perp + r] of the axis.
    if d_perp - r > c_hi + band || d_perp + r < c_lo - band {
        return Ok(report);
    }
    let frame = [cone.frame[0], cone.frame[1], cone.axis];
    let s_implicit = crate::implicit_trace::SphereImplicit {
        center: sphere.center,
        radius: r,
    };
    let c_implicit = crate::implicit_trace::ConeImplicit {
        bottom: cone.bottom,
        frame,
        r_bottom: cone.r_bottom,
        slope: cone.slope,
    };
    let seed = |theta: f64, phi: f64| -> [f64; 3] {
        let (st, ct) = theta.sin_cos();
        let (sp, cp) = phi.sin_cos();
        std::array::from_fn(|k| {
            sphere.center[k] + r * (st * cp * frame[0][k] + st * sp * frame[1][k] + ct * frame[2][k])
        })
    };
    match crate::implicit_trace::trace_intersection(
        &s_implicit,
        &c_implicit,
        seed,
        terms + d_perp,
        r.min(c_lo.max(1e-3)),
        crate::TRACE_TOLERANCE,
    ) {
        Ok(loops) if !loops.is_empty() => {
            let all: Vec<Vec<Curve>> = loops.iter().map(|l| l.pieces.clone()).collect();
            let complete = crate::sphere_torus::completeness::certify_complete_against(
                sphere,
                &crate::sphere_torus::completeness::Quadric::Cone {
                    bottom: cone.bottom,
                    frame,
                    r_bottom: cone.r_bottom,
                    slope: cone.slope,
                },
                sphere_model,
                &all,
            );
            report.components.extend(loops.into_iter().map(|l| SphereConeComponent::Traced {
                pieces: l.pieces,
                distance_bound: l.distance_bound,
                spans: l.spans,
                complete,
            }));
        }
        Ok(_) => report.unresolved(domain, UnresolvedReason::UnsupportedSurface),
        Err(e) if e.message.contains("tangency") => {
            report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot)
        }
        Err(_) => report.unresolved(domain, UnresolvedReason::UnsupportedSurface),
    }
    Ok(report)
}
