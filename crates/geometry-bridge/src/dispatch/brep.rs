//! Bridge operations of the `brep` family.
use crate::*;

const OPS: &[&str] = &[
    "brep_session",
    "brep_graph",
    "brep_graph_report",
    "brep_scene_plan",
    "brep_semantic_geometry",
    "brep_profile_transform",
    "brep_profile_author",
    "brep_profile_validate",
    "brep_profile_offset",
    "brep_profile_boolean",
    "brep_profile_intersections",
    "brep_profile_signed_area",
    "brep_polygon_from_mesh",
    "brep_polygon_inspect",
    "brep_polygon_tessellate",
];

pub(crate) fn route(v: Value) -> Routed {
    if !OPS.contains(&v["op"].as_str().unwrap_or("")) {
        return Routed::Unhandled(v);
    }
    Routed::Handled(handle(v))
}

fn handle(v: Value) -> Result<Value> {
    match v["op"].as_str().unwrap_or("") {
        "brep_session" => brep_session_abi::dispatch(v),
        "brep_graph" => brep_graph::dispatch(v),
        "brep_graph_report" => Ok(brep_graph::report(v)),
        "brep_scene_plan" => brep_scene_plan::plan(&v),
        "brep_semantic_geometry" => brep_semantic::execute(v),
        "brep_profile_transform"
        | "brep_profile_author"
        | "brep_profile_validate"
        | "brep_profile_offset"
        | "brep_profile_boolean"
        | "brep_profile_intersections"
        | "brep_profile_signed_area" => brep_profile::dispatch(v),
        "brep_polygon_from_mesh" => {
            let ids: Option<Vec<usize>> =
                v.get("faceIds").map(|_| field(&v, "faceIds")).transpose()?;
            encode(polygon_core::solid::brep::from_mesh(
                &field(&v, "mesh")?,
                ids.as_deref(),
            )?)
        }
        "brep_polygon_inspect" => {
            polygon_core::solid::brep::validate(&field(&v, "model")?)?;
            encode(json!({"topologyValid":true,"solidGeometryStatus":"not_certified"}))
        }
        "brep_polygon_tessellate" => encode(brep::polygons(&field(&v, "model")?)?),
        _ => unreachable!("route admits only OPS"),
    }
}
