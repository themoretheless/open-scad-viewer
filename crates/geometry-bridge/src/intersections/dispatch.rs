use super::*;

pub fn dispatch(v: Value) -> Result<Value> {
    if v["op"].as_str() == Some("brep_intersection_trace_curve_segments") {
        let trace: SurfaceTrace = field(&v, "trace")?;
        return encode(trace.to_curve_segments()?);
    }
    if v["op"].as_str() == Some("brep_intersection_trace_curve") {
        let trace: SurfaceTrace = field(&v, "trace")?;
        return encode(trace.to_curve()?);
    }
    if v["op"].as_str() == Some("brep_intersection_trace_evaluate") {
        let trace: SurfaceTrace = field(&v, "trace")?;
        return encode(trace.evaluate(field(&v, "fraction")?)?);
    }
    let options: Options = match v.get("options") {
        Some(value) => value_codec::from_value(value.clone())
            .map_err(|e| input(format!("Invalid intersection options: {e}")))?,
        None => Options::default(),
    };
    match v["op"].as_str() {
        Some("brep_intersect_surface_surface") => encode(queries::surface_surface(
            &field(&v, "first")?,
            &field(&v, "second")?,
            options,
        )?),
        Some("brep_intersect_curve_segment") => encode(queries::curve_segment(
            &field(&v, "curve")?,
            field(&v, "start")?,
            field(&v, "end")?,
            options,
        )?),
        Some("brep_intersect_curve_plane") => encode(queries::curve_plane(
            &field(&v, "curve")?,
            field(&v, "plane")?,
            options,
        )?),
        Some("brep_intersect_surface_plane") => encode(queries::surface_plane(
            &field(&v, "surface")?,
            field(&v, "plane")?,
            options,
        )?),
        Some("brep_intersect_curve_curve") => encode(queries::curve_curve(
            &field(&v, "first")?,
            &field(&v, "second")?,
            options,
        )?),
        Some("brep_intersect_curve_surface") => encode(queries::curve_surface(
            &field(&v, "curve")?,
            &field(&v, "surface")?,
            options,
        )?),
        Some("brep_intersect_curve_ruled_surface") => encode(queries::curve_ruled_surface(
            &field(&v, "curve")?,
            &field(&v, "surface")?,
            options,
        )?),
        Some("brep_intersect_sphere_sphere") => encode(brep_intersect::intersect_sphere_sphere(
            &field(&v, "first")?,
            &field(&v, "second")?,
            brep_core::intersections::Options {distance_tolerance:options.distance_tolerance,parameter_tolerance:options.parameter_tolerance,max_depth:options.max_depth,max_boxes:options.max_boxes},
        )?),
        Some("brep_intersect_sphere_cylinder") => encode(brep_intersect::intersect_sphere_cylinder(
            &field(&v, "first")?,
            &field(&v, "second")?,
            options,
        )?),
        Some("brep_intersect_sphere_cone") => encode(brep_intersect::intersect_sphere_cone(
            &field(&v, "first")?,
            &field(&v, "second")?,
            options,
        )?),
        Some("brep_intersect_cone_cone") => encode(brep_intersect::intersect_cone_cone(
            &field(&v, "first")?,
            &field(&v, "second")?,
            options,
        )?),
        Some("brep_intersect_cylinder_cylinder") => {
            encode(brep_intersect::intersect_cylinder_cylinder(
                &field(&v, "first")?,
                &field(&v, "second")?,
                options,
            )?)
        }
        Some("brep_intersect_plane_sphere") => encode(brep_intersect::intersect_plane_sphere(
            &field(&v, "first")?,
            &field(&v, "second")?,
            options,
        )?),
        Some("brep_intersect_plane_cylinder") => encode(brep_intersect::intersect_plane_cylinder(
            &field(&v, "first")?,
            &field(&v, "second")?,
            options,
        )?),
        Some("brep_intersect_plane_cone") => encode(brep_intersect::intersect_plane_cone(
            &field(&v, "first")?,
            &field(&v, "second")?,
            options,
        )?),
        Some("brep_intersect_plane_torus") => encode(brep_intersect::intersect_plane_torus(
            &field(&v, "first")?,
            &field(&v, "second")?,
            options,
        )?),
        Some("brep_intersect_sphere_torus") => encode(brep_intersect::intersect_sphere_torus(
            &field(&v, "first")?,
            &field(&v, "second")?,
            options,
        )?),
        Some("brep_intersect_cylinder_torus") => encode(brep_intersect::intersect_cylinder_torus(
            &field(&v, "first")?,
            &field(&v, "second")?,
            options,
        )?),
        Some("brep_intersect_cone_torus") => encode(brep_intersect::intersect_cone_torus(
            &field(&v, "first")?,
            &field(&v, "second")?,
            options,
        )?),
        Some("brep_intersect_torus_torus") => encode(brep_intersect::intersect_torus_torus(
            &field(&v, "first")?,
            &field(&v, "second")?,
            options,
        )?),
        _ => Err(input("Unknown intersection query")),
    }
}
