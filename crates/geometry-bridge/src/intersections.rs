//! Read-only native intersection queries; reports never authorize topology edits.
use super::{Result, Value, encode, field, input};
use brep_core::intersections::{self, Options, SurfaceTrace};

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
        Some("brep_intersect_surface_surface") => encode(intersections::surface_surface(
            &field(&v, "first")?,
            &field(&v, "second")?,
            options,
        )?),
        Some("brep_intersect_curve_segment") => encode(intersections::curve_segment(
            &field(&v, "curve")?,
            field(&v, "start")?,
            field(&v, "end")?,
            options,
        )?),
        Some("brep_intersect_curve_plane") => encode(intersections::curve_plane(
            &field(&v, "curve")?,
            field(&v, "plane")?,
            options,
        )?),
        Some("brep_intersect_surface_plane") => encode(intersections::surface_plane(
            &field(&v, "surface")?,
            field(&v, "plane")?,
            options,
        )?),
        Some("brep_intersect_curve_surface") => encode(intersections::curve_surface(
            &field(&v, "curve")?,
            &field(&v, "surface")?,
            options,
        )?),
        _ => Err(input("Unknown intersection query")),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn serialized_intersection_report_is_explicitly_uncertified() {
        let curve =
            nurbs_core::curve::Curve::from_polyline(vec![vec![0., 0., -1.], vec![0., 0., 1.]])
                .unwrap();
        let result = super::dispatch(value_codec::json!({"op":"brep_intersect_curve_plane","curve":curve,"plane":{"normal":[0.,0.,1.],"offset":0.}})).unwrap();
        assert_eq!(result["coverage"].as_str(), Some("numerically_resolved"));
        assert_eq!(result["permitsTopologyChange"].as_bool(), Some(false));
        assert_eq!(
            result["components"][0]["curve"]["parameter"].as_f64(),
            Some(0.5)
        );
    }
}
