use super::*;

/// Finite cylinder × sphere: Complete empty when separated; Complete circle when
/// sphere center lies on the cylinder axis and the section is a transverse circle
/// strictly between the caps; otherwise Incomplete / frozen refuse.
pub fn cylinder_sphere(
    cyl_origin: [f64; 3],
    cyl_dir: [f64; 3],
    cyl_radius: f64,
    cyl_height: [f64; 2],
    sphere_center: [f64; 3],
    sphere_radius: f64,
    options: Options,
) -> Result<Report<AnalyticSsComponent>> {
    let options = options.validate()?;
    let axis = normalize(cyl_dir)?;
    if !(cyl_radius.is_finite()
        && sphere_radius.is_finite()
        && cyl_radius > 0.
        && sphere_radius > 0.)
        || !cyl_origin
            .iter()
            .chain(&sphere_center)
            .all(|x| x.is_finite())
    {
        return Err(invalid(
            "Cylinder/sphere parameters must be finite and positive",
        ));
    }
    let delta = sub(sphere_center, cyl_origin);
    let axial = dot(delta, axis);
    let radial = sub(delta, mul(axis, axial));
    let separation = norm(radial);
    let tol = options.distance_tolerance;
    let outside_height =
        axial < cyl_height[0] - sphere_radius - tol || axial > cyl_height[1] + sphere_radius + tol;
    if outside_height || separation > cyl_radius + sphere_radius + tol {
        return Ok(complete_report(vec![AnalyticSsComponent::Empty]));
    }
    // Axial transverse circle (sphere center on axis, section between caps).
    if separation <= tol {
        let r2 = sphere_radius * sphere_radius - 0.;
        let _ = r2;
        // Intersection of sphere with cylinder wall: circle in plane ⊥ axis at
        // the sphere center when cyl_radius < sphere_radius and height admits.
        if (cyl_radius - sphere_radius).abs() <= tol {
            return Ok(incomplete_report(
                vec![cyl_height[0], cyl_height[1]],
                UnresolvedReason::NearCoincidence,
            ));
        }
        if cyl_radius + tol < sphere_radius
            && axial >= cyl_height[0] + tol
            && axial <= cyl_height[1] - tol
        {
            let half = (sphere_radius * sphere_radius - cyl_radius * cyl_radius).sqrt();
            if half.is_finite()
                && axial - half >= cyl_height[0] - tol
                && axial + half <= cyl_height[1] + tol
            {
                return Ok(complete_report(vec![AnalyticSsComponent::Circle {
                    center: add(cyl_origin, mul(axis, axial)),
                    normal: axis,
                    radius: cyl_radius,
                }]));
            }
        }
        return Ok(incomplete_report(
            vec![cyl_height[0], cyl_height[1]],
            UnresolvedReason::UnsupportedSurface,
        ));
    }
    Ok(incomplete_report(
        vec![cyl_height[0], cyl_height[1]],
        UnresolvedReason::UnsupportedSurface,
    ))
}

/// Coaxial / parallel finite cones (apex+axis+radii envelopes): Complete empty when
/// clearly separated; coincident/intersecting walls Incomplete (frozen refuse).
pub fn cone_cone(
    a: FiniteCone,
    b: FiniteCone,
    options: Options,
) -> Result<Report<AnalyticSsComponent>> {
    let FiniteCone {
        apex: a_apex,
        axis: a_axis,
        radius: a_radius,
        height: a_height,
    } = a;
    let FiniteCone {
        apex: b_apex,
        axis: b_axis,
        radius: b_radius,
        height: b_height,
    } = b;
    let options = options.validate()?;
    let a_dir = normalize(a_axis)?;
    let b_dir = normalize(b_axis)?;
    if !(a_radius > 0. && b_radius > 0. && a_height > 0. && b_height > 0.) {
        return Err(invalid("Cone parameters must be positive and finite"));
    }
    if norm(cross(a_dir, b_dir)) > 1e-12 {
        return Ok(incomplete_report(
            vec![0., a_height, 0., b_height],
            UnresolvedReason::UnsupportedSurface,
        ));
    }
    let delta = sub(b_apex, a_apex);
    let radial = sub(delta, mul(a_dir, dot(delta, a_dir)));
    let separation = norm(radial);
    let tol = options.distance_tolerance;
    let a_extent = a_radius + a_height;
    let b_extent = b_radius + b_height;
    if separation > a_extent + b_extent + tol {
        return Ok(complete_report(vec![AnalyticSsComponent::Empty]));
    }
    Ok(incomplete_report(
        vec![0., a_height, 0., b_height],
        UnresolvedReason::UnsupportedSurface,
    ))
}

/// Torus × torus: Complete empty when bounding spheres are disjoint; else Incomplete.
pub fn torus_torus(
    a_center: [f64; 3],
    a_major: f64,
    a_minor: f64,
    b_center: [f64; 3],
    b_major: f64,
    b_minor: f64,
    options: Options,
) -> Result<Report<AnalyticSsComponent>> {
    let options = options.validate()?;
    if !(a_major > a_minor && a_minor > 0. && b_major > b_minor && b_minor > 0.) {
        return Err(invalid("Torus requires major > minor > 0"));
    }
    let d = norm(sub(b_center, a_center));
    let tol = options.distance_tolerance;
    if d > a_major + a_minor + b_major + b_minor + tol {
        return Ok(complete_report(vec![AnalyticSsComponent::Empty]));
    }
    Ok(incomplete_report(
        vec![],
        UnresolvedReason::UnsupportedSurface,
    ))
}

/// Plane × torus (major R, minor r, axis Z at origin): Complete empty / equator circle / Incomplete.
pub fn plane_torus(
    plane: Plane,
    center: [f64; 3],
    axis_dir: [f64; 3],
    major: f64,
    minor: f64,
    options: Options,
) -> Result<Report<AnalyticSsComponent>> {
    let options = options.validate()?;
    let plane = plane.normalized()?;
    let axis = normalize(axis_dir)?;
    if !(major.is_finite() && minor.is_finite() && major > minor && minor > 0.)
        || !center.iter().all(|x| x.is_finite())
    {
        return Err(invalid(
            "Torus parameters must be finite with major > minor > 0",
        ));
    }
    let cos = dot(plane.normal, axis).abs();
    let tol = options.distance_tolerance;
    let d = plane.distance(center);
    // Only the mid-plane perpendicular cut (equator circle of radius major) is Complete.
    if (cos - 1.).abs() <= 1e-12 && d.abs() <= tol {
        return Ok(complete_report(vec![AnalyticSsComponent::Circle {
            center,
            normal: axis,
            radius: major,
        }]));
    }
    if d.abs() > major + minor + tol {
        return Ok(complete_report(vec![AnalyticSsComponent::Empty]));
    }
    Ok(incomplete_report(
        vec![],
        UnresolvedReason::UnsupportedSurface,
    ))
}
