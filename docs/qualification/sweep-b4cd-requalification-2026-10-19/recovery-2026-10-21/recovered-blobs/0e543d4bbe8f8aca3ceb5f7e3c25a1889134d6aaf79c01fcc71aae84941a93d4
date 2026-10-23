//! Surfaces of revolution and extruded side surfaces built on conic profiles:
//! cylinders, cones, spheres, ellipsoids, hyperboloids and tori. No caps.
use crate::{Result, check, curve::Curve, surface::{Surface, revolve}};
use super::conics::{ellipse_arc, hyperbola};

fn radii(values: &[f64]) -> Result<()> {
    check(
        values.iter().all(|v| v.is_finite() && *v > 0.),
        "Radii must be finite and positive",
    )
}

/// Circular cylinder side surface, without caps.
pub fn cylinder(center: [f64; 3], radius: f64, height: f64) -> Result<Surface> {
    elliptic_cylinder(center, radius, radius, height)
}

/// Circular cone side surface, with an intentional singular apex and no base cap.
pub fn cone(center: [f64; 3], radius: f64, height: f64) -> Result<Surface> {
    cone_frustum(center, radius, 0., height)
}

/// Rational sphere surface with singular poles; no solid topology is inferred.
pub fn sphere(center: [f64; 3], radius: f64) -> Result<Surface> {
    ellipsoid(center, [radius; 3])
}

/// Axis-aligned elliptical cylinder side surface, without caps.
pub fn elliptic_cylinder(center: [f64; 3], rx: f64, ry: f64, height: f64) -> Result<Surface> {
    radii(&[rx, ry, height])?;
    crate::surface::extrude(
        &ellipse_arc(center, [rx, 0., 0.], [0., ry, 0.], 0., 360.)?,
        [0., 0., height],
    )
}

/// Circular cone/frustum side surface. A zero end radius creates a singular apex.
pub fn cone_frustum(center: [f64; 3], bottom: f64, top: f64, height: f64) -> Result<Surface> {
    check(
        bottom.is_finite()
            && top.is_finite()
            && bottom >= 0.
            && top >= 0.
            && (bottom > 0. || top > 0.),
        "Cone radii must be nonnegative with at least one positive radius",
    )?;
    radii(&[height])?;
    check(
        center.iter().all(|x| x.is_finite()),
        "Cone center must be finite",
    )?;
    let profile = Curve::from_polyline(vec![vec![bottom, 0., 0.], vec![top, 0., height]])?;
    let mut result = revolve(&profile, [0.; 3], [0., 0., 1.], 360.)?;
    for row in &mut result.control_points {
        for p in row {
            for i in 0..3 {
                p[i] += center[i];
            }
        }
    }
    result.validate()?;
    Ok(result)
}

/// Axis-aligned ellipsoid. The two poles are intentional parameter singularities.
/// Returns a surface, not a certified solid or trimmed B-rep.
pub fn ellipsoid(center: [f64; 3], radius: [f64; 3]) -> Result<Surface> {
    radii(&radius)?;
    let profile = ellipse_arc(
        [0.; 3],
        [radius[0], 0., 0.],
        [0., 0., radius[2]],
        -90.,
        180.,
    )?;
    let mut result = revolve(&profile, [0.; 3], [0., 0., 1.], 360.)?;
    // Materialize exact poles instead of retaining cos(pi/2) roundoff.
    let last = result.control_points.len() - 1;
    for row in [0, last] {
        for p in &mut result.control_points[row] {
            p[0] = 0.;
            p[1] = 0.;
        }
    }
    for row in &mut result.control_points {
        for p in row {
            p[1] = (p[1] / radius[0]) * radius[1];
            for a in 0..3 {
                p[a] += center[a];
            }
        }
    }
    result.validate()?;
    Ok(result)
}

/// Elliptic one-sheet hyperboloid: X²/a²+Y²/b²-Z²/c²=1.
/// Finite analytic t interval; rational parameter is not affine in t.
pub fn hyperboloid_one_sheet(
    center: [f64; 3],
    radii_xyz: [f64; 3],
    start: f64,
    end: f64,
) -> Result<Surface> {
    radii(&radii_xyz)?;
    let [a, b, c] = radii_xyz;
    let profile = hyperbola([0.; 3], [a, 0., 0.], [0., 0., c], start, end)?;
    let mut result = revolve(&profile, [0.; 3], [0., 0., 1.], 360.)?;
    for row in &mut result.control_points {
        for p in row {
            p[1] = (p[1] / a) * b;
            for i in 0..3 {
                p[i] += center[i];
            }
        }
    }
    result.validate()?;
    Ok(result)
}

/// One sheet of Z²/c²-X²/a²-Y²/b²=1. lower selects negative Z.
/// Requires 0 <= start < end. start=0 includes a singular pole.
pub fn hyperboloid_two_sheet(
    center: [f64; 3],
    radii_xyz: [f64; 3],
    start: f64,
    end: f64,
    lower: bool,
) -> Result<Surface> {
    radii(&radii_xyz)?;
    check(
        start >= 0.,
        "Two-sheet hyperboloid interval must be nonnegative",
    )?;
    let [a, b, c] = radii_xyz;
    let profile = hyperbola(
        [0.; 3],
        [0., 0., if lower { -c } else { c }],
        [a, 0., 0.],
        start,
        end,
    )?;
    let mut result = revolve(&profile, [0.; 3], [0., 0., 1.], 360.)?;
    for row in &mut result.control_points {
        for p in row {
            p[1] = (p[1] / a) * b;
            for i in 0..3 {
                p[i] += center[i];
            }
        }
    }
    result.validate()?;
    Ok(result)
}

/// Ring torus about Z with an elliptical tube. Horn/spindle tori are rejected.
pub fn torus(center: [f64; 3], major: f64, radial: f64, axial: f64) -> Result<Surface> {
    radii(&[major, radial, axial])?;
    check(
        major > radial,
        "Ring torus requires major radius greater than radial tube radius",
    )?;
    let profile = ellipse_arc([major, 0., 0.], [radial, 0., 0.], [0., 0., axial], 0., 360.)?;
    let mut result = revolve(&profile, [0.; 3], [0., 0., 1.], 360.)?;
    for row in &mut result.control_points {
        for p in row {
            for a in 0..3 {
                p[a] += center[a];
            }
        }
    }
    result.validate()?;
    Ok(result)
}
