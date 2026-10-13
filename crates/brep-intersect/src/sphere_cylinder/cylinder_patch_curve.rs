use super::*;

/// One cylinder face's share of an intersection circle in that face's UV.
#[derive(Clone, Debug)]
pub struct CylinderPatchCurve {
    /// Face index in the source cylinder model.
    pub patch: usize,
    /// Side patches carry one degree-1 iso-v segment; a cap patch carries
    /// four exact 90-degree rational arcs of the UV circle.
    pub arcs: Vec<Curve>,
}

#[derive(Clone, Debug)]
pub enum SphereCylinderComponent {
    /// Transverse intersection: the exact circle plus both UV lifts.
    Circle {
        /// Exact rational full circle: degree 2, knots 0..=4, four 90-degree
        /// arcs with weights cos(pi/4).
        curve: Curve,
        center: [f64; 3],
        radius: f64,
        /// Unit cylinder axis (the circle plane normal).
        normal: [f64; 3],
        sphere_uv: Vec<SpherePatchCircle>,
        cylinder_uv: Vec<CylinderPatchCurve>,
        /// Worst residual over 16 circle samples against the sphere equation
        /// and the cylinder side or cap plane equation.
        max_sample_residual: f64,
    },
}

/// Where a resolved circle sits on the cylinder boundary.
#[derive(Clone, Copy)]
enum CircleSite {
    /// On the side wall: radius is the cylinder radius.
    Side,
    /// On a cap disk with the given circle radius.
    Cap(f64),
}

/// Exact circle component at axial position `axial` relative to the cylinder
/// center, with UV lifts on both surfaces and a 16-sample residual bound.
fn circle_component(
    sphere: &CanonicalSphere,
    cylinder: &CanonicalCylinder,
    axial: f64,
    site: CircleSite,
) -> Result<SphereCylinderComponent> {
    let center = std::array::from_fn(|k| cylinder.center[k] + axial * cylinder.axis[k]);
    let (radius, cylinder_uv) = match site {
        CircleSite::Side => {
            // Side patches: u sweeps the quadrant, v runs bottom to top, so
            // the circle is the iso-v line v = (axial + h/2) / h on each.
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
        CircleSite::Cap(rho) => {
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
    let sphere_uv = lift(sphere, cylinder.axis, center);
    let mut max_sample_residual = 0_f64;
    for i in 0..16 {
        let point = point3(&curve.evaluate(i as f64 / 4.)?.point);
        let d = sub(point, sphere.center);
        let sphere_residual = (d[0].hypot(d[1]).hypot(d[2]) - sphere.radius).abs();
        let other = match site {
            CircleSite::Side => {
                let rel = sub(point, cylinder.center);
                let a = dot(rel, cylinder.axis);
                let perp = sub(rel, cylinder.axis.map(|x| x * a));
                (perp[0].hypot(perp[1]).hypot(perp[2]) - cylinder.radius).abs()
            }
            CircleSite::Cap(_) => (dot(sub(point, cylinder.center), cylinder.axis) - axial).abs(),
        };
        max_sample_residual = max_sample_residual.max(sphere_residual).max(other);
    }
    Ok(SphereCylinderComponent::Circle {
        curve,
        center,
        radius,
        normal: cylinder.axis,
        sphere_uv,
        cylinder_uv,
        max_sample_residual,
    })
}

/// Analytic sphere/cylinder intersection of a canonical sphere solid and a
/// canonical cylinder solid, axial configuration only. Non-canonical operands
/// and clearly off-axis pairs are explicit unsupported regions, never a
/// numerical fallback; near-axial offsets, the r == R band, rim and cap-plane
/// tangencies stay unresolved — tangent contacts are never guessed.
pub fn intersect_sphere_cylinder(
    sphere_model: &Model,
    cylinder_model: &Model,
    options: Options,
) -> Result<Report<SphereCylinderComponent>> {
    let options = options.validate()?;
    let _ = options;
    let mut report = Report::default();
    let domain = vec![0., 1., 0., 1., 0., 1., 0., 1.];
    let (Some(sphere), Some(cylinder)) = (
        sphere_sphere::recognize(sphere_model)?,
        recognize_cylinder(cylinder_model)?,
    ) else {
        report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    };
    report.boxes_visited = 1;
    let offset = sub(sphere.center, cylinder.center);
    let s = dot(offset, cylinder.axis);
    let perp = sub(offset, cylinder.axis.map(|x| x * s));
    let pscale = perp.iter().map(|v| v.abs()).fold(0., f64::max);
    let d_perp = if pscale == 0. {
        0.
    } else {
        let scaled = perp.map(|x| x / pscale);
        scaled[0].hypot(scaled[1]).hypot(scaled[2]) * pscale
    };
    let terms = s.abs() + cylinder.half_height + sphere.radius + cylinder.radius + 1.;
    // Outward binary64 classification band: recognition deviation plus a
    // rounding allowance on the axial coordinates and radii.
    let band = sphere.error + cylinder.error + 16. * f64::EPSILON * terms;
    // Axiality is certified, never forced: pure-rounding offsets snap to the
    // axis, recognition-scale offsets report near_coincidence, and anything
    // larger is the unsupported general (quartic) configuration.
    let snap = 64. * f64::EPSILON * terms;
    if d_perp > snap {
        let near = RECOGNITION * (sphere.radius + cylinder.radius + cylinder.half_height);
        let reason = if d_perp <= near {
            UnresolvedReason::NearCoincidence
        } else {
            UnresolvedReason::UnsupportedSurface
        };
        report.unresolved(domain, reason);
        return Ok(report);
    }
    let r = sphere.radius;
    let big_r = cylinder.radius;
    let half = cylinder.half_height;
    // Sphere provably beyond a cap plane: no contact in any radius relation.
    if s - r > half + band || -s - r > half + band {
        return Ok(report);
    }
    // r == R within the band: side contact degenerates to a tangent circle at
    // the sphere equator and the component structure cannot be certified —
    // a coincident-band region, never a guessed circle.
    if (r - big_r).abs() <= band {
        report.unresolved(domain, UnresolvedReason::CoincidentTrim);
        return Ok(report);
    }
    let mut tangency = false;
    let mut found: Vec<(f64, CircleSite)> = Vec::new();
    if r < big_r {
        // Sphere provably inside the side wall: cap-plane crossings only, and
        // every cap circle has radius <= r < R, so it sits inside the disk.
        for &q in &[half, -half] {
            let d = (q - s).abs();
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
            found.push((q, CircleSite::Cap(rho2.sqrt())));
        }
    } else {
        // r > R provably: side crossings at s +- sqrt(r^2 - R^2), clipped by
        // the finite height; a crossing on a cap plane is the rim tangency.
        let delta2 = r * r - big_r * big_r;
        if !delta2.is_finite() || delta2 <= 0. {
            report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
            return Ok(report);
        }
        let delta = delta2.sqrt();
        for sign in [1., -1.] {
            let p = s + sign * delta;
            if p.abs() > half + band {
                continue;
            }
            if (p.abs() - half).abs() <= band {
                tangency = true;
                continue;
            }
            found.push((p, CircleSite::Side));
        }
        // Cap-plane contacts: a circle inside the disk (which implies a side
        // crossing within the height), the rim band, or a plane tangency.
        for &q in &[half, -half] {
            let d = (q - s).abs();
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
            if rho > big_r + band {
                continue;
            }
            if (rho - big_r).abs() <= band {
                tangency = true;
                continue;
            }
            found.push((q, CircleSite::Cap(rho)));
        }
    }
    if tangency {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
    }
    found.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (axial, site) in found {
        report
            .components
            .push(circle_component(&sphere, &cylinder, axial, site)?);
    }
    Ok(report)
}

impl value_codec::Serialize for CylinderPatchCurve {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({"patch":self.patch,"arcs":self.arcs})
    }
}
impl value_codec::Serialize for SphereCylinderComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Circle {
                curve,
                center,
                radius,
                normal,
                sphere_uv,
                cylinder_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"circle","curve":curve,"center":center,"radius":radius,
                "normal":normal,"sphereUv":sphere_uv,"cylinderUv":cylinder_uv,
                "maxSampleResidual":max_sample_residual}),
        }
    }
}
