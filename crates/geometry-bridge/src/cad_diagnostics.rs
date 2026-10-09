//! Read-only section coordinates are computed in Rust, including plane placement.
use super::{Result, Value, field, input};
use mesh_section::MeshSectionIndex;
use polygon_core::Mesh;
use value_codec::json;
type V = [f64; 3];
fn dot(a: V, b: V) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
fn cross(a: V, b: V) -> V {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn unit(a: V) -> Result<V> {
    let length = dot(a, a).sqrt();
    if !length.is_finite() || length <= 1e-12 {
        return Err(input("Section normal must be finite and nonzero."));
    }
    Ok(a.map(|x| x / length))
}
pub fn section(v: Value) -> Result<Value> {
    let mesh: Mesh = field(&v, "mesh")?;
    let normal = unit(field(&v, "normal")?)?;
    let offset: f64 = field(&v, "offset")?;
    if !offset.is_finite() {
        return Err(input("Section offset must be finite."));
    }
    let helper = if normal[0].abs() < 0.8 {
        [1., 0., 0.]
    } else {
        [0., 1., 0.]
    };
    let u = unit(cross(helper, normal))?;
    let w = cross(normal, u);
    let mut local = mesh.clone();
    for point in local.positions.chunks_exact_mut(3) {
        let p = [point[0], point[1], point[2]];
        point.copy_from_slice(&[dot(p, u), dot(p, w), dot(p, normal)]);
    }
    let (section, collapsed) = MeshSectionIndex::new(&local.view())
        .map_err(crate::legacy_mesh_error)?
        .section_for_display(offset)
        .map_err(crate::legacy_mesh_error)?;
    let contours = section
        .contours
        .iter()
        .map(|contour| {
            let mut points = contour
                .points
                .iter()
                .map(|p| {
                    std::array::from_fn::<_, 3, _>(|i| {
                        u[i] * p[0] + w[i] * p[1] + normal[i] * offset
                    })
                })
                .collect::<Vec<_>>();
            // The section kernel closes implicitly; UI polylines require the first point again.
            if let Some(first) = points.first().copied() {
                points.push(first);
            }
            json!({"points":points,"sourceTriangles":contour.source_triangles})
        })
        .collect::<Vec<_>>();
    Ok(
        json!({"normal":normal,"offsetMm":offset,"candidateTriangles":section.candidate_triangles,"contours":contours,"collapsedSegmentTriangles":collapsed}),
    )
}
pub fn measure_points(v: Value) -> Result<Value> {
    let a: V = field(&v, "a")?;
    let b: V = field(&v, "b")?;
    if !a.iter().chain(&b).all(|x| x.is_finite()) {
        return Err(input("Measurement points must be finite."));
    }
    let delta: V = std::array::from_fn(|i| b[i] - a[i]);
    let distance = delta[0].hypot(delta[1]).hypot(delta[2]);
    if !distance.is_finite() {
        return Err(input("Measurement exceeded numeric range."));
    }
    Ok(json!({"a":a,"b":b,"deltaMm":delta,"distanceMm":distance}))
}
pub fn curvature(v: Value) -> Result<Value> {
    let curve: nurbs_core::curve::Curve = field(&v, "curve")?;
    curve.validate()?;
    if curve.control_points[0].len() != 3 {
        return Err(input("Curvature measurement requires a 3D curve."));
    }
    let t: f64 = field(&v, "t")?;
    if !t.is_finite() || !(0.0..=1.0).contains(&t) {
        return Err(input("Curve parameter must be between 0 and 1."));
    }
    let [lo, hi] = curve.domain();
    let evaluated = curve.evaluate(lo + (hi - lo) * t)?;
    let d1 = evaluated
        .d1
        .ok_or_else(|| input("Curve derivative is unavailable at this parameter."))?;
    let d2 = evaluated.d2.ok_or_else(|| {
        input("Curve curvature is undefined at this knot. Choose a nearby parameter.")
    })?;
    let a = [d1[0], d1[1], d1[2]];
    let b = [d2[0], d2[1], d2[2]];
    let speed = dot(a, a).sqrt();
    let cross_ab = cross(a, b);
    let numerator = dot(cross_ab, cross_ab).sqrt();
    if speed == 0. || !speed.is_finite() || !numerator.is_finite() {
        return Err(input("Curve curvature exceeds the regular numeric domain."));
    }
    let denominator = speed * speed * speed;
    if !denominator.is_finite() || denominator == 0. {
        return Err(input("Curve curvature exceeded numeric range."));
    }
    let k = numerator / denominator;
    if numerator > 0. && k == 0. {
        return Err(input("Curve curvature underflowed numeric range."));
    }
    if !k.is_finite() {
        return Err(input("Curve curvature exceeded numeric range."));
    }
    let radius = if k == 0. { None } else { Some(1. / k) };
    if radius.is_some_and(|r| !r.is_finite()) {
        return Err(input("Curvature radius exceeded numeric range."));
    }
    Ok(json!({"point":evaluated.point,"curvaturePerMm":k,"radiusMm":radius,"parameter":t}))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_planes() {
        assert!(unit([0.; 3]).is_err());
        assert!(unit([f64::NAN, 0., 1.]).is_err());
        assert_eq!(unit([0., 0., 3.]).unwrap(), [0., 0., 1.]);
    }
    #[test]
    fn measures_distance_and_circle_curvature() {
        let measured = measure_points(json!({"a":[0.,0.,0.],"b":[3.,4.,12.]})).unwrap();
        assert_eq!(field::<f64>(&measured, "distanceMm").unwrap(), 13.);
        let circle = json!({"degree":2,"knots":[0.,0.,0.,1.,1.,1.],"controlPoints":[[3.,0.,0.],[3.,3.,0.],[0.,3.,0.]],"weights":[1.,std::f64::consts::FRAC_1_SQRT_2,1.]});
        for t in [0.1, 0.5, 0.9] {
            let result = curvature(json!({"curve":circle.clone(),"t":t})).unwrap();
            assert!((field::<f64>(&result, "radiusMm").unwrap() - 3.).abs() < 1e-12);
        }
        assert!(curvature(json!({"curve":circle,"t":2.})).is_err());
    }
}

pub fn locations(v: Value) -> Result<Value> {
    let mesh: Mesh = field(&v, "mesh")?;
    let d = mesh.diagnostic_locations()?;
    Ok(
        json!({"boundaryEdges":d.boundary_edges,"nonManifoldEdges":d.non_manifold_edges,"orientationEdges":d.orientation_edges,"degenerateTriangles":d.degenerate_triangles}),
    )
}

pub fn intersection(v: Value) -> Result<Value> {
    let mesh: Mesh = field(&v, "mesh")?;
    let tolerance: f64 = field(&v, "relativeTolerance")?;
    let work: usize = field(&v, "maxWork")?;
    let hit = polygon_core::solid::boolean::first_mesh_intersection(&mesh, tolerance, work)?;
    let contact=hit.map(|h|json!({"triangles":h.triangles,"point":h.point,"sharedVertices":h.shared_vertices,"toleranceMm":h.tolerance_mm}));
    Ok(
        json!({"contact":contact,"relativeTolerance":tolerance,"scope":"display-mesh-first-contact"}),
    )
}

#[cfg(test)]
mod intersection_tests {
    use super::*;
    #[test]
    fn serializes_first_contact_and_rejects_invalid_budget() {
        let mesh = json!({"positions":[0.,0.,0.,2.,0.,0.,0.,2.,0.],"indices":[0,1,2,0,1,2]});
        let result =
            intersection(json!({"mesh":mesh.clone(),"relativeTolerance":1e-9,"maxWork":1000}))
                .unwrap();
        assert_eq!(result["contact"]["triangles"], json!([0, 1]));
        assert_eq!(result["scope"], json!("display-mesh-first-contact"));
        assert!(intersection(json!({"mesh":mesh,"relativeTolerance":1e-9,"maxWork":0})).is_err());
    }
}

/// Full pair enumeration within explicit work and output budgets.
pub fn intersections(v: Value) -> Result<Value> {
    let mesh: Mesh = field(&v, "mesh")?;
    let tolerance: f64 = field(&v, "relativeTolerance")?;
    let work: usize = field(&v, "maxWork")?;
    let max_contacts: usize = field(&v, "maxContacts")?;
    let report =
        polygon_core::solid::boolean::mesh_intersections(&mesh, tolerance, work, max_contacts)?;
    let contacts=report.contacts.into_iter().map(|h|json!({"triangles":h.triangles,"point":h.point,"sharedVertices":h.shared_vertices,"toleranceMm":h.tolerance_mm})).collect::<Vec<_>>();
    Ok(
        json!({"contacts":contacts,"complete":report.complete,"stopReason":report.stop_reason,"work":report.work,"maxWork":work,"maxContacts":max_contacts,"relativeTolerance":tolerance,"scope":"display-mesh-all-contacts"}),
    )
}

#[cfg(test)]
mod all_intersection_tests {
    use super::*;
    #[test]
    fn serializes_completion_and_output_limit_without_claiming_a_clean_mesh() {
        let mesh = json!({"positions":[0.,0.,0.,2.,0.,0.,0.,2.,0.],"indices":[0,1,2,0,1,2,0,1,2]});
        let result = intersections(
            json!({"mesh":mesh.clone(),"relativeTolerance":1e-9,"maxWork":1000,"maxContacts":100}),
        )
        .unwrap();
        assert_eq!(result["contacts"].as_array().unwrap().len(), 3);
        assert_eq!(result["complete"], json!(true));
        let partial = intersections(
            json!({"mesh":mesh,"relativeTolerance":1e-9,"maxWork":1,"maxContacts":100}),
        )
        .unwrap();
        assert_eq!(partial["complete"], json!(false));
        assert_eq!(partial["stopReason"], json!("work-limit"));
        assert_eq!(partial["contacts"], json!([]));
    }
}
