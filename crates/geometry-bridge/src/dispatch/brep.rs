//! Bridge operations of the `brep` family.
use crate::*;

const OPS: &[&str] = &[
    "brep_profile_normalize","brep_profile_cleanup","brep_nurbs_bezier_profiles_extrude",
    "minkowski_3d_certify",
    "minkowski_3d_verify",
    "minkowski_nurbs_certify",
    "minkowski_nurbs_verify",
    "nurbs_source_atlas_certify",
    "nurbs_source_atlas_verify",
    "brep_profile_certify",
    "brep_profile_certify_measure",
    "brep_curve_certify_agreement",
    "brep_curve_certify_image_agreement",
    "brep_curve_verify_image_agreement_certificate",
    "brep_profile_minkowski_sum",
    "brep_profile_minkowski_offset",
    "brep_profile_verify_minkowski_certificate",
    "brep_curve_certify_span_intersections",
    "brep_curve_verify_span_intersections_certificate",
    "brep_curve_certify_algebraic_intersections",
    "brep_curve_verify_algebraic_intersections_certificate",
    "brep_curve_certify_multispan_agreement",
    "brep_curve_verify_multispan_agreement_certificate",
    "brep_profile_offset_regions_certified",
    "brep_profile_verify_offset_regions_certificate",
    "brep_curve_certify_nonlinear_agreement",
    "brep_curve_verify_nonlinear_agreement_certificate",
    "brep_curve_certify_intersections",
    "brep_curve_verify_intersections_certificate",
    "brep_profile_offset_wire_certified",
    "brep_profile_verify_offset_wire_certificate",
    "brep_curve_certify_projective_agreement",
    "brep_curve_verify_projective_agreement_certificate",
    "brep_curve_certify_endpoint_contacts",
    "brep_curve_verify_endpoint_contacts_certificate",
    "brep_curve_offset_certified",
    "brep_curve_verify_offset_certificate",
    "brep_curve_verify_agreement_certificate",
    "brep_profile_verify_measure_certificate",
    "brep_profile_verify_certificate",
    "brep_closed_patch_network",
    "brep_g2_surface_bridge",
    "brep_g2_runout_surface",
    "brep_geometric_g2_audit",
    "brep_g2_annular_cylinder",

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
    "brep_profile_display",
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
        "brep_nurbs_bezier_profiles_extrude"=>extrude_profiles(v),
        "minkowski_3d_certify" | "minkowski_3d_verify" | "minkowski_nurbs_certify" | "minkowski_nurbs_verify" | "nurbs_source_atlas_certify" | "nurbs_source_atlas_verify" => cad_minkowski3d::dispatch(v),
        "brep_closed_patch_network" => cad_geometric_g2::create_closed_patch_network(v),
        "brep_g2_surface_bridge" => cad_geometric_g2::create_surface_bridge(v),
        "brep_g2_runout_surface" => cad_runout::create(v),
        "brep_geometric_g2_audit" => cad_geometric_g2::diagnose(v),
        "brep_g2_annular_cylinder" => cad_geometric_g2::create_annular_cylinder(v),

        "brep_session" => brep_session_abi::dispatch(v),
        "brep_graph" => brep_graph::dispatch(v),
        "brep_graph_report" => Ok(brep_graph::report(v)),
        "brep_scene_plan" => brep_scene_plan::plan(&v),
        "brep_semantic_geometry" => brep_semantic::execute(v),
        "brep_profile_certify"
        | "brep_profile_certify_measure"
        | "brep_curve_certify_agreement"
        | "brep_curve_certify_image_agreement"
        | "brep_curve_verify_image_agreement_certificate"
        | "brep_profile_minkowski_sum"
        | "brep_profile_minkowski_offset"
        | "brep_profile_verify_minkowski_certificate"
        | "brep_curve_certify_span_intersections"
        | "brep_curve_verify_span_intersections_certificate"
        | "brep_curve_certify_algebraic_intersections"
        | "brep_curve_verify_algebraic_intersections_certificate"
        | "brep_curve_certify_multispan_agreement"
        | "brep_curve_verify_multispan_agreement_certificate"
        | "brep_profile_offset_regions_certified"
        | "brep_profile_verify_offset_regions_certificate"
        | "brep_curve_certify_nonlinear_agreement"
        | "brep_curve_verify_nonlinear_agreement_certificate"
        | "brep_curve_certify_intersections"
        | "brep_curve_verify_intersections_certificate"
        | "brep_profile_offset_wire_certified"
        | "brep_profile_verify_offset_wire_certificate"
        | "brep_curve_certify_projective_agreement"
        | "brep_curve_verify_projective_agreement_certificate"
        | "brep_curve_certify_endpoint_contacts"
        | "brep_curve_verify_endpoint_contacts_certificate"
        | "brep_curve_offset_certified"
        | "brep_curve_verify_offset_certificate"
        | "brep_curve_verify_agreement_certificate"
        | "brep_profile_verify_measure_certificate"
        | "brep_profile_verify_certificate"
        | "brep_profile_normalize" | "brep_profile_cleanup" | "brep_profile_transform"
        | "brep_profile_author"
        | "brep_profile_validate"
        | "brep_profile_offset"
        | "brep_profile_boolean"
        | "brep_profile_intersections"
        | "brep_profile_signed_area"
        | "brep_profile_display" => brep_profile::dispatch(v),
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

fn extrude_profiles(v:Value)->Result<Value> {
            let sketches = v
                .get("sketches")
                .and_then(Value::as_array)
                .ok_or_else(|| input("Expected closed profile sketches"))?;
            let mut paths = Vec::new();
            let mut common_plane = None;
            for (index, sketch) in sketches.iter().enumerate() {
                if !field::<bool>(sketch, "closed")?
                    || sketch.get("retainedProfile").is_some_and(|p| !p.is_null())
                    || sketch.get("analytic").is_some_and(|p| !p.is_null())
                {
                    return Err(input(
                        "Compound authored profiles accept closed polynomial paths or polygons",
                    ));
                }
                let plane = match sketch.get("plane") {
                    None | Some(Value::Null) => [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
                    Some(p) => [field(p, "origin")?, field(p, "u")?, field(p, "v")?],
                };
                if index == 0 {
                    common_plane = Some(plane);
                } else if common_plane != Some(plane) {
                    return Err(input("All profile contours must share one exact workplane"));
                }
                paths.push(
                    if let Some(path) = sketch.get("editablePath").filter(|p| !p.is_null()) {
                        path2d::authored_path(path)?
                    } else {
                        planar_geometry::path::BezierPath::from_polygon(
                            &field::<Vec<[f64; 2]>>(sketch, "points")?,
                            true,
                        )?
                    },
                );
            }
            encode(brep_core::sketch::extrude_bezier_profiles(
                paths,
                field(&v, "height")?,
                field(&v, "baseZ")?,
                common_plane,
            )?)
        }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_router_executes_endpoint_certificate_and_rejects_forgery() {
        let curve=|y:f64|json!({"degree":2,"knots":[0.,0.,0.,1.,1.,1.],
            "controlPoints":[[0.,0.],[1.,y],[2.,0.]],"weights":[1.,1.,1.],"periodic":false});
        let first=curve(-1.);let second=curve(1.);
        let response:Value=value_codec::from_str(&crate::execute(&value_codec::to_string(&json!({
            "op":"brep_curve_certify_endpoint_contacts","first":first,"second":second
        })).unwrap())).unwrap();
        assert_eq!(response["ok"],json!(true),"{response:?}");
        let mut certificate=response["value"].clone();
        assert_eq!(certificate["commonEndpoints"],json!(2));
        certificate["commonEndpoints"]=json!(0);
        let refused:Value=value_codec::from_str(&crate::execute(&value_codec::to_string(&json!({
            "op":"brep_curve_verify_endpoint_contacts_certificate","first":first,"second":second,
            "certificate":certificate
        })).unwrap())).unwrap();
        assert_eq!(refused["ok"],json!(false),"{refused:?}");
    }
}
