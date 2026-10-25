use super::*;

#[derive(Clone, Debug)]
pub enum CylinderTorusComponent {
    /// Transverse intersection: the exact circle plus both UV lifts.
    Circle {
        /// Exact rational full circle: degree 2, knots 0..=4, four 90-degree
        /// arcs with weights cos(pi/4).
        curve: Curve,
        center: [f64; 3],
        radius: f64,
        /// Unit shared axis (the circle plane normal).
        normal: [f64; 3],
        cylinder_uv: Vec<CylinderPatchCurve>,
        torus_uv: Vec<TorusPatchCurve>,
        /// Worst residual over 16 circle samples against the cylinder side
        /// or cap plane equation and the scaled torus implicit equation.
        max_sample_residual: f64,
    },
}

/// Where a resolved circle sits on the cylinder boundary.
#[derive(Clone, Copy)]
pub(super) enum CircleSite {
    /// On the side wall: radius is the cylinder radius.
    Side,
    /// On a cap disk: cylinder axial position and circle radius.
    Cap(f64, f64),
}

/// Exact circle component at torus-axial height `z` above the torus center
/// plane, with UV lifts on both surfaces and a 16-sample residual bound.
pub(super) fn circle_component(
    cylinder: &CanonicalCylinder,
    torus: &CanonicalTorus,
    z: f64,
    site: CircleSite,
) -> Result<CylinderTorusComponent> {
    let center = std::array::from_fn(|k| torus.center[k] + z * torus.axis[k]);
    let (radius, cylinder_uv) = match site {
        CircleSite::Side => {
            // Side patches: u sweeps the quadrant, v runs bottom to top, so
            // the circle is the iso-v line v = (axial + h/2) / h on each.
            let axial = dot(sub(center, cylinder.center), cylinder.axis);
            let v0 = (axial + cylinder.half_height) / (2. * cylinder.half_height);
            let arcs = vec![Curve {
                degree: 1,
                knots: vec![0., 0., 1., 1.],
                control_points: vec![[0., v0].to_vec(), [1., v0].to_vec()],
                weights: vec![1., 1.],
                periodic: false,
            }];
            (
                cylinder.radius,
                cylinder
                    .sides
                    .iter()
                    .map(|&patch| CylinderPatchCurve {
                        patch,
                        arcs: arcs.clone(),
                    })
                    .collect::<Vec<_>>(),
            )
        }
        CircleSite::Cap(axial, rho) => {
            let face = if axial > 0. {
                cylinder.caps[1]
            } else {
                cylinder.caps[0]
            };
            // Cap UV maps the plane as [1/2 + x/(2R), 1/2 + y/(2R)]: the
            // circle is an exact UV circle of radius rho/(2R) about [1/2,1/2].
            (
                rho,
                vec![CylinderPatchCurve {
                    patch: face,
                    arcs: circle_arcs([0.5, 0.5], rho / (2. * cylinder.radius), 0., TAU),
                }],
            )
        }
    };
    let curve = circle_curve(center, radius, cylinder.frame[0], cylinder.frame[1]);
    // Torus lift: the iso-v parallel at the exact profile angle.
    let profile = z.atan2(radius - torus.major).rem_euclid(TAU);
    let torus_uv = lift_parallel(torus, profile, &[(0., TAU)]);
    let mut max_sample_residual = 0_f64;
    for i in 0..16 {
        let jet = curve.evaluate(i as f64 / 4.)?;
        let point = [jet.point[0], jet.point[1], jet.point[2]];
        let rel = sub(point, cylinder.center);
        let axial = dot(rel, cylinder.axis);
        let cylinder_residual = match site {
            CircleSite::Side => {
                let perp = sub(rel, cylinder.axis.map(|x| x * axial));
                (perp[0].hypot(perp[1]).hypot(perp[2]) - cylinder.radius).abs()
            }
            CircleSite::Cap(q, _) => (axial - q).abs(),
        };
        max_sample_residual = max_sample_residual
            .max(cylinder_residual)
            .max(torus_residual(torus, point));
    }
    Ok(CylinderTorusComponent::Circle {
        curve,
        center,
        radius,
        normal: cylinder.axis,
        cylinder_uv,
        torus_uv,
        max_sample_residual,
    })
}

/// Analytic cylinder/torus intersection of a canonical cylinder solid and a
/// canonical ring-torus solid, coaxial configuration only. Non-canonical
/// operands and clearly tilted or off-axis pairs are explicit unsupported
/// regions, never a numerical fallback; recognition-scale near-coaxial
/// bands, the meridian line/circle tangency, rim contacts and cap-plane
/// touches stay unresolved — tangent contacts are never guessed. Provable
/// misses (the line missing the meridian circle, circles beyond the finite
/// height, cap circles outside the cap disks) resolve empty.
pub fn intersect_cylinder_torus(
    cylinder_model: &Model,
    torus_model: &Model,
    options: Options,
) -> Result<Report<CylinderTorusComponent>> {
    let options = options.validate()?;
    let _ = options;
    let mut report = Report::default();
    let domain = vec![0., 1., 0., 1., 0., 1., 0., 1.];
    let (Some(cylinder), Some(torus)) = (
        recognize_cylinder(cylinder_model)?,
        recognize_torus(torus_model)?,
    ) else {
        report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    };
    report.boxes_visited = 1;
    let delta = sub(torus.center, cylinder.center);
    let terms = cylinder.half_height
        + cylinder.radius
        + torus.major
        + torus.minor
        + delta.iter().map(|v| v.abs()).fold(0., f64::max)
        + 1.;
    // Outward binary64 classification band: both recognition deviations plus
    // a rounding allowance on the axial coordinates and radii.
    let band = cylinder.error + torus.error + 16. * f64::EPSILON * terms;
    // Axis parallelism is certified, never forced: pure-rounding direction
    // disagreement snaps to parallel, recognition-scale tilt reports
    // near_coincidence, and a clearly tilted pair is the unsupported general
    // (quartic) case.
    let direction = cross(cylinder.axis, torus.axis);
    let tilt = direction[0].hypot(direction[1]).hypot(direction[2]);
    let axis_snap = 64. * f64::EPSILON
        + cylinder.error / (2. * cylinder.half_height)
        + torus.error / (2. * torus.major);
    if tilt > axis_snap {
        let reason = if tilt <= RECOGNITION {
            UnresolvedReason::NearCoincidence
        } else {
            UnresolvedReason::UnsupportedSurface
        };
        report.unresolved(domain, reason);
        return Ok(report);
    }
    let axis = cylinder.axis;
    let along = dot(delta, axis);
    let perp = sub(delta, axis.map(|x| x * along));
    let pscale = perp.iter().map(|v| v.abs()).fold(0., f64::max);
    let d_perp = if pscale == 0. {
        0.
    } else {
        let scaled = perp.map(|x| x / pscale);
        scaled[0].hypot(scaled[1]).hypot(scaled[2]) * pscale
    };
    // Coaxiality: a center-line distance at pure rounding scale snaps to
    // zero; a recognition-scale offset is near_coincidence, never forced;
    // a clear offset is the unsupported general (quartic) configuration.
    let dist_snap = 64. * f64::EPSILON * terms + cylinder.error + torus.error;
    if d_perp > dist_snap {
        let reason = if d_perp <= RECOGNITION * terms {
            UnresolvedReason::NearCoincidence
        } else {
            UnresolvedReason::UnsupportedSurface
        };
        report.unresolved(domain, reason);
        return Ok(report);
    }
    let (r, big_r, r_c, half) = (
        torus.minor,
        torus.major,
        cylinder.radius,
        cylinder.half_height,
    );
    let mut tangency = false;
    let mut found: Vec<(f64, CircleSite)> = Vec::new();
    // Meridian line rho = R_c against the circle (rho - R)^2 + z^2 = r^2.
    let a = r_c - big_r;
    let aa = a.abs();
    if aa > r + band {
        // Provable miss: the line clears the meridian circle.
    } else if (aa - r).abs() <= band {
        // Tangent line: the double root at z == 0 revolves into a circle but
        // stays unresolved — never a guessed circle.
        tangency = true;
    } else {
        // |a| < r provably: two distinct roots z = +-sqrt(r^2 - a^2). The
        // roots coincide only at the tangent, so a resolved branch always
        // yields two distinct circles (no single-circle configuration).
        let z2 = r * r - a * a;
        if !z2.is_finite() || z2 <= 0. {
            // Defensive: the band comparisons above should have caught every
            // tangency; a non-positive discriminant here is rounding spill
            // and stays unresolved rather than guessed.
            tangency = true;
        } else {
            let z0 = z2.sqrt();
            for sign in [1., -1.] {
                let z = sign * z0;
                let center = std::array::from_fn(|k| torus.center[k] + z * torus.axis[k]);
                let axial = dot(sub(center, cylinder.center), axis);
                if axial.abs() > half + band {
                    // Provably beyond a cap plane: honestly absent.
                    continue;
                }
                if (axial.abs() - half).abs() <= band {
                    // Rim tangency: the circle lands on the cap plane.
                    tangency = true;
                    continue;
                }
                found.push((z, CircleSite::Side));
            }
        }
    }
    // Cap-plane contacts: the torus tube pokes through the cap plane at
    // torus-axial height h_c, cutting the circles of radii R +- sqrt(r^2 -
    // h_c^2) — the plane/torus perpendicular branch, reduced to the disk.
    for &q in &[half, -half] {
        let cap = std::array::from_fn(|k| cylinder.center[k] + q * axis[k]);
        let h_c = dot(sub(cap, torus.center), torus.axis);
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
        let s = s2.sqrt();
        if big_r - s <= band {
            // Inner-radius collapse (a pole degeneracy): unreachable for a
            // canonical strict ring torus (R - r >= 1e-5), kept unresolved,
            // never guessed.
            tangency = true;
            continue;
        }
        for side in [1., -1.] {
            let rho = big_r + side * s;
            if rho > r_c + band {
                // Provably outside the cap disk: honestly absent.
                continue;
            }
            if (rho - r_c).abs() <= band {
                // Rim tangency: the cap circle meets the side wall.
                tangency = true;
                continue;
            }
            found.push((h_c, CircleSite::Cap(q, rho)));
        }
    }
    if tangency {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
    }
    // Deterministic order: by torus-axial height, then by radius.
    let radius_of = |site: &CircleSite| match site {
        CircleSite::Side => r_c,
        CircleSite::Cap(_, rho) => *rho,
    };
    found.sort_by(|x, y| {
        x.0.total_cmp(&y.0)
            .then(radius_of(&x.1).total_cmp(&radius_of(&y.1)))
    });
    for (z, site) in found {
        report
            .components
            .push(circle_component(&cylinder, &torus, z, site)?);
    }
    Ok(report)
}
