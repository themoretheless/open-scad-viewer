use super::*;

/// Box/sphere Boolean entry: `Ok(None)` unless one operand is the canonical
/// cuboid and the other the canonical sphere. Admitted pairs return the
/// regularized result; everything else is an explicit refusal.
pub(crate) fn boolean(a: &Model, b: &Model, operation: &str) -> Result<Option<Model>> {
    if !matches!(operation, "union" | "difference" | "intersection") {
        return Ok(None);
    }
    let (mate_index, cuboid, sphere) = match (
        recognize_box(a)?,
        sphere_sphere::recognize(b)?,
        recognize_box(b)?,
        sphere_sphere::recognize(a)?,
    ) {
        (Some(cuboid), Some(sphere), _, _) => (0usize, cuboid, sphere),
        (_, _, Some(cuboid), Some(sphere)) => (1usize, cuboid, sphere),
        _ => return Ok(None),
    };
    let band = shared_band(&cuboid, &sphere);
    sphere_mate::run(a, b, mate_index, &cuboid, sphere, band, operation).map(Some)
}

fn shared_band(cuboid: &CanonicalBox<'_>, sphere: &CanonicalSphere) -> f64 {
    let scale = cuboid
        .faces
        .iter()
        .map(|f| f.u_len.max(f.v_len))
        .fold(sphere.radius, f64::max)
        + norm(sub(sphere.center, cuboid.faces[0].origin))
        + 1.;
    cuboid.error + sphere.error + 1e-9 * scale + 64. * f64::EPSILON * scale
}
