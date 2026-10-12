use super::*;

#[derive(Clone, Debug)]
pub enum ConeTorusComponent {
    /// Transverse intersection: the exact circle plus both UV lifts.
    Circle {
        /// Exact rational full circle: degree 2, knots 0..=4, four 90-degree
        /// arcs with weights cos(pi/4).
        curve: Curve,
        center: [f64; 3],
        radius: f64,
        /// Unit cone axis (the circle plane normal).
        normal: [f64; 3],
        cone_uv: Vec<CylinderPatchCurve>,
        torus_uv: Vec<TorusPatchCurve>,
        /// Worst residual over 16 circle samples against the cone side
        /// profile or cap plane equation and the scaled torus implicit
        /// equation.
        max_sample_residual: f64,
    },
}

/// Where a resolved circle sits on the cone boundary.
#[derive(Clone, Copy)]
enum CircleSite {
    /// On the side wall: radius is the linearly interpolated rho(axial).
    Side,
    /// On a ring cap disk (slot 0 bottom, slot 1 top) with the circle radius.
    Cap(usize, f64),
}


/// Exact circle component at torus-axial height `z` (measured along the
/// torus's own axis), with UV lifts on both surfaces and a 16-sample
/// residual bound.
fn circle_component(
    cone: &CanonicalCone,
    torus: &CanonicalTorus,
    z: f64,
    site: CircleSite,
) -> Result<ConeTorusComponent> {
    let center = std::array::from_fn(|k| torus.center[k] + z * torus.axis[k]);
    let (radius, cone_uv) = match site {
        CircleSite::Side => {
            // Side patches: u sweeps the quadrant, v runs bottom ring to top
            // ring, so the circle is the iso-v line v = axial / h on each.
            let axial = dot(sub(center, cone.bottom), cone.axis);
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
            let face = cone.caps[slot].expect("cap circle needs a cap face");
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
    // Torus lift: the iso-v parallel at the exact profile angle.
    let profile = z.atan2(radius - torus.major).rem_euclid(TAU);
    let torus_uv = lift_parallel(torus, profile, &[(0., TAU)]);
    let mut max_sample_residual = 0_f64;
    for i in 0..16 {
        let point = point3(&curve.evaluate(i as f64 / 4.)?.point);
        let cone_residual = match site {
            CircleSite::Side => {
                let rel = sub(point, cone.bottom);
                let a = dot(rel, cone.axis);
                let perp = sub(rel, cone.axis.map(|x| x * a));
                (perp[0].hypot(perp[1]).hypot(perp[2]) - (cone.r_bottom + cone.slope * a)).abs()
            }
            CircleSite::Cap(slot, _) => {
                let q = if slot == 0 { 0. } else { cone.height };
                (dot(sub(point, cone.bottom), cone.axis) - q).abs()
            }
        };
        max_sample_residual = max_sample_residual
            .max(cone_residual)
            .max(torus_residual(torus, point));
    }
    Ok(ConeTorusComponent::Circle {
        curve,
        center,
        radius,
        normal: cone.axis,
        cone_uv,
        torus_uv,
        max_sample_residual,
    })
}

/// Analytic cone/torus intersection of a canonical conical frustum solid and
/// a canonical ring-torus solid, coaxial configuration only (either axis
/// orientation). Non-canonical operands and clearly tilted or off-axis pairs
/// are explicit unsupported regions, never a numerical fallback;
/// recognition-scale near-coaxial bands, the meridian line/circle tangency,
/// rim contacts and cap-plane tube tangencies stay unresolved — tangent
/// contacts are never guessed. Provable misses (the line missing the
/// meridian circle, roots beyond the finite height, cap circles outside the
/// ring disks) resolve empty.
pub fn intersect_cone_torus(
    cone_model: &Model,
    torus_model: &Model,
    options: Options,
) -> Result<Report<ConeTorusComponent>> {
    let options = options.validate()?;
    let _ = options;
    let mut report = Report::default();
    let domain = vec![0., 1., 0., 1., 0., 1., 0., 1.];
    let (Some(cone), Some(torus)) = (recognize_cone(cone_model)?, recognize_torus(torus_model)?)
    else {
        report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    };
    report.boxes_visited = 1;
    let delta = sub(torus.center, cone.bottom);
    let terms = cone.height
        + cone.r_bottom
        + cone.r_top
        + torus.major
        + torus.minor
        + delta.iter().map(|v| v.abs()).fold(0., f64::max)
        + 1.;
    // Outward binary64 classification band: both recognition deviations plus
    // a rounding allowance on the axial coordinates and radii.
    let band = cone.error + torus.error + 16. * f64::EPSILON * terms;
    // Axis parallelism is certified, never forced: pure-rounding direction
    // disagreement snaps to parallel, recognition-scale tilt reports
    // near_coincidence, and a clearly tilted pair is the unsupported general
    // (quartic) case.
    let direction = cross(cone.axis, torus.axis);
    let tilt = direction[0].hypot(direction[1]).hypot(direction[2]);
    let axis_snap =
        64. * f64::EPSILON + cone.error / cone.height + torus.error / (2. * torus.major);
    if tilt > axis_snap {
        let reason = if tilt <= RECOGNITION {
            UnresolvedReason::NearCoincidence
        } else {
            UnresolvedReason::UnsupportedSurface
        };
        report.unresolved(domain, reason);
        return Ok(report);
    }
    // Coaxiality: a center-line distance at pure rounding scale snaps to
    // zero; a recognition-scale offset is near_coincidence, never forced;
    // a clear offset is the unsupported general (quartic) configuration.
    let along = dot(delta, cone.axis);
    let perp = sub(delta, cone.axis.map(|x| x * along));
    let pscale = perp.iter().map(|v| v.abs()).fold(0., f64::max);
    let d_perp = if pscale == 0. {
        0.
    } else {
        let scaled = perp.map(|x| x / pscale);
        scaled[0].hypot(scaled[1]).hypot(scaled[2]) * pscale
    };
    let dist_snap = 64. * f64::EPSILON * terms + cone.error + torus.error;
    if d_perp > dist_snap {
        let reason = if d_perp <= RECOGNITION * terms {
            UnresolvedReason::NearCoincidence
        } else {
            UnresolvedReason::UnsupportedSurface
        };
        report.unresolved(domain, reason);
        return Ok(report);
    }
    let (r, big_r) = (torus.minor, torus.major);
    let (r_b, m, h) = (cone.r_bottom, cone.slope, cone.height);
    // Orientation: the torus-axial height of the point at cone-axial t is
    // z(t) = z_b + s t with s = +-1 the axes' relative orientation; the
    // meridian circle is symmetric in z, so the sign enters only the height
    // map and the torus profile angles.
    let s = if dot(cone.axis, torus.axis) >= 0. {
        1.
    } else {
        -1.
    };
    let z_b = dot(sub(cone.bottom, torus.center), torus.axis);
    let mut tangency = false;
    let mut found: Vec<(f64, CircleSite)> = Vec::new();
    // Meridian line (rho, z) = (r_b + m t, z_b + s t) against the circle
    // (rho - R)^2 + z^2 = r^2: the quadratic (1+m^2) t^2 + 2 B t + C = 0
    // with B = (r_b - R) m + z_b s and discriminant r^2 (1+m^2) - D^2,
    // D = (R - r_b) s + z_b m the signed cross of center-minus-line-point
    // with the line direction — the line passes the circle center at
    // distance |D| / sqrt(1+m^2).
    let k2 = 1. + m * m;
    let cross_d = (big_r - r_b) * s + z_b * m;
    let dist = cross_d.abs() / k2.sqrt();
    if dist > r + band {
        // Provable miss: the line clears the meridian circle.
    } else if (dist - r).abs() <= band {
        // Tangent line: a double root. Only a tangency whose foot lands on
        // the finite side segment belongs to this surface; beyond it the
        // rim bands below own the contact.
        let foot = ((big_r - r_b) * m - z_b * s) / k2;
        if (-band..=h + band).contains(&foot) {
            tangency = true;
        }
    } else {
        // Two distinct roots t = (-B +- sqrt(r^2 k2 - D^2)) / k2.
        let b = (r_b - big_r) * m + z_b * s;
        let disc2 = r * r * k2 - cross_d * cross_d;
        if !disc2.is_finite() || disc2 <= 0. {
            // Defensive: the band comparisons above should have caught every
            // tangency; a non-positive discriminant here is rounding spill
            // and stays unresolved rather than guessed.
            tangency = true;
        } else {
            let disc = disc2.sqrt();
            for sign in [1., -1.] {
                let t = (-b + sign * disc) / k2;
                if !(-band..=h + band).contains(&t) {
                    // Provably beyond a ring plane: honestly absent.
                    continue;
                }
                // A root within the band of a ring plane is the rim contact.
                if t.abs() <= band || (t - h).abs() <= band {
                    tangency = true;
                    continue;
                }
                // A root whose radius collapses into the band would be the
                // apex/pole degeneracy: unreachable for a canonical strict
                // ring torus (the meridian circle stays at rho >= R - r),
                // kept as an honest guard, never guessed through.
                let rho = r_b + m * t;
                if rho <= band {
                    tangency = true;
                    continue;
                }
                found.push((z_b + s * t, CircleSite::Side));
            }
        }
    }
    // Rim bands: a rim circle lying on the torus within the band —
    // (r_ring - R)^2 + h_q^2 == r^2 — is a tangent boundary contact, never a
    // guessed circle (independent of the side classification above; this
    // also owns the zero-radius apex ring).
    for slot in 0..2 {
        let t_ring = if slot == 0 { 0. } else { h };
        let r_ring = if slot == 0 { r_b } else { cone.r_top };
        let h_q = z_b + s * t_ring;
        let rim = ((r_ring - big_r) * (r_ring - big_r) + h_q * h_q).sqrt();
        if (rim - r).abs() <= band {
            tangency = true;
        }
    }
    // Cap-plane contacts: the ring plane at torus-axial height h_c cuts the
    // tube at the circles of radii R +- sqrt(r^2 - h_c^2) — the plane/torus
    // perpendicular branch, reduced to the ring disk.
    for slot in 0..2 {
        if cone.caps[slot].is_none() {
            continue;
        }
        let t_cap = if slot == 0 { 0. } else { h };
        let r_ring = if slot == 0 { r_b } else { cone.r_top };
        let h_c = z_b + s * t_cap;
        let ah = h_c.abs();
        if ah > r + band {
            // Provable miss: the whole band clears the minor radius.
            continue;
        }
        if ah >= (r - band).max(0.) {
            // Tangent plane, or a band straddling it: never a guessed circle.
            tangency = true;
            continue;
        }
        let s2 = r * r - h_c * h_c;
        if !s2.is_finite() || s2 <= 0. {
            tangency = true;
            continue;
        }
        let root = s2.sqrt();
        if big_r - root <= band {
            // Inner-radius collapse (a pole degeneracy): unreachable for a
            // canonical strict ring torus (R - r >= 1e-5), kept unresolved,
            // never guessed.
            tangency = true;
            continue;
        }
        for side in [1., -1.] {
            let rho = big_r + side * root;
            if rho > r_ring + band {
                // Provably outside the ring disk: honestly absent.
                continue;
            }
            if (rho - r_ring).abs() <= band {
                // Rim tangency: the cap circle meets the side wall.
                tangency = true;
                continue;
            }
            found.push((h_c, CircleSite::Cap(slot, rho)));
        }
    }
    if tangency {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
    }
    // Deterministic order: by torus-axial height, then by radius.
    let radius_of = |site: &CircleSite| match site {
        CircleSite::Side => 0.,
        CircleSite::Cap(_, rho) => *rho,
    };
    found.sort_by(|x, y| {
        x.0.total_cmp(&y.0)
            .then(radius_of(&x.1).total_cmp(&radius_of(&y.1)))
    });
    for (z, site) in found {
        report
            .components
            .push(circle_component(&cone, &torus, z, site)?);
    }
    Ok(report)
}

impl value_codec::Serialize for ConeTorusComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Circle {
                curve,
                center,
                radius,
                normal,
                cone_uv,
                torus_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"circle","curve":curve,"center":center,"radius":radius,
                "normal":normal,"coneUv":cone_uv,"torusUv":torus_uv,
                "maxSampleResidual":max_sample_residual}),
        }
    }
}
