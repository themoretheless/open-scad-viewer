//! Bridge operations of the `brep` family.
use crate::*;
#[path="brep_closed_ss.rs"]
pub(super) mod closed_ss;

#[path="brep_section_sheets.rs"]
mod section_sheets;

#[path="brep_rim_caps.rs"]
mod rim_caps;
#[path="brep_inferred_rims.rs"]
mod inferred_rims;
#[path="brep_polyhedral_mass.rs"]
mod polyhedral_mass;
#[path="brep_section_queries.rs"]
mod section_queries;
#[path="brep_body_queries.rs"]
mod body_queries;
#[path="brep_swept_clearance.rs"]
mod swept_clearance;
#[path="brep_path_contacts.rs"]
mod path_contacts;
#[path="brep_assembly_path.rs"]
mod assembly_path;
#[path="brep_spatial_trace.rs"]
mod spatial_trace;
const OPS: &[&str] = &[
    "attach_expanded_rims","discover_expanded_rims","close_expanded_planes","build_expanded_plane_solid","build_outward_plane_solid","build_outward_inferred_solid",
    "inspect_inferred_solid","expanded_section_sheets","build_expanded_solid",
    "build_section_solid",
    "infer_rim_planes",
    "close_inferred_rims",
    "close_bounded_inferred_rims",
    "build_inferred_solid",
    "measure_polyhedral_volume","measure_polyhedral_centroid","measure_polyhedral_inertia","measure_polyhedral_face_areas","bound_polyhedral_area","bound_polyhedral_edge_length",
    "project_polyhedral_body","measure_polyhedral_frame","measure_polyhedral_axis_gap","classify_polyhedral_plane","intersect_polyhedral_edges",
    "section_polyhedral_faces","assemble_polyhedral_section_loops","measure_polyhedral_section_area","measure_polyhedral_section_centroid","bound_polyhedral_section_perimeter",
    "locate_section_point","measure_section_contour_distance","measure_section_material_distance","clip_section_segment","clip_section_polyline","measure_section_path_length",
    "clip_polyhedral_segment","locate_polyhedral_point","measure_polyhedral_boundary_distance","measure_polyhedral_signed_distance","sample_polyhedral_signed_distances","measure_polyhedral_ball_clearance",
    "measure_swept_ball_gap","measure_swept_path_gap","classify_swept_path_material","bound_swept_first_contact","bound_swept_face_ranges",
    "bound_path_material_length","clip_exterior_path","clip_interior_path","partition_solid_path","bound_path_length_breakdown",
    "find_path_face_contacts","recover_material_path","clip_path_to_material","find_path_boundary_events",
    "clip_common_path","clip_union_path","subtract_path_cutters","classify_path_membership","bound_overlap_lengths","recover_overlap_paths",
    "node_spatial_path","triangulate_planar_path","measure_path_second_moments","bound_path_principal_moments","measure_region_mass","intersect_transverse_profiles","measure_profile_overlap_area","measure_profile_overlap_centroid","classify_profile_overlap","decompose_profile_boolean","extract_profile_boolean_edges","assemble_profile_boolean_loops","triangulate_profile_boolean","measure_parallel_sweep_overlap","mesh_parallel_sweep_overlap","build_parallel_sweep_overlap","build_profile_boolean_solid","measure_profile_extrusion_inertia","locate_profile_extrusion_points","section_profile_extrusion","section_profile_extrusion_parallel","split_profile_extrusion_volume","clip_profile_extrusion_path","bound_profile_extrusion_area","extrude_profile_boolean_shell","decompose_profile_overlap","clip_path_to_perforated_region","measure_region_second_moments","measure_planar_path_area","measure_planar_path_centroid","locate_planar_path_points","clip_path_to_planar_region","project_to_planar_region","simplify_exact_path","project_point_to_path","intersect_spatial_paths","find_path_self_intersections","measure_spatial_path_distance","simplify_path_with_tolerance",
    "close_section_planes",
    "build_bounded_section_solid",
    "close_bounded_planes",
    "attach_section_rims","discover_section_rims","attach_bounded_rims","discover_bounded_rims",
    "section_brep_sheets","bounded_section_brep_sheets",
    "brep_partition_closed_ss", "brep_partition_all_closed_ss",
    "brep_audit_exact_periodic_trim_contour",
    "brep_measure_exact_periodic_trim_range",
    "brep_locate_exact_periodic_trim_contour_station",
    "brep_enclose_exact_periodic_trim_contour_station",
    "brep_divide_exact_periodic_trim_contour_by_length",
    "brep_bound_exact_periodic_trim_patch_area",
    "brep_measure_exact_periodic_trim_surface_area",
    "brep_measure_exact_periodic_trim_surface_mass_properties",
    "brep_measure_exact_periodic_trim_patch_set_mass_properties",
    "brep_measure_exact_periodic_trim_patch_set_density",
    "brep_measure_exact_periodic_trim_window_mass_properties",
    "brep_bound_exact_periodic_trim_directional_area",
    "locate_edge_parameter",
    "locate_input_parameter",
    "locate_input_range",
    "extract_input_range_curves",
    "extract_input_range_curve_pairs",
    "surface_trim_sheet_display",
    "surface_trim_sheet_step",
    "brep_build_exact_periodic_trim_sheet",
    "brep_build_exact_periodic_trim_sheet_with_bindings",
    "brep_build_exact_periodic_trim_sheet_with_input_bindings",
    "brep_qualify_periodic_band", "brep_intersect_periodic_stock_band",
    "brep_qualify_isolated_ss_contact",
    "brep_assemble_closed_ss_boolean", "brep_assemble_closed_ss_network_boolean",
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
    "brep_source_graph_g2_preview",
    "brep_frozen_graph_body_restore",
    "brep_frozen_graph_body_display",
    "brep_geometric_g2_audit",
    "brep_g2_annular_cylinder",

    "brep_session",
    "brep_boundary_certificate",
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
        "inspect_inferred_solid" => inferred_rims::inspect(&v),
        "build_expanded_solid" => rim_caps::solid(&v,"expanded"),
        op @ ("attach_expanded_rims"|"discover_expanded_rims") => rim_caps::attach(&v,op),
        "close_expanded_planes" => rim_caps::close_planes(&v,"expanded"),
        "build_expanded_plane_solid" => rim_caps::solid(&v,"expanded_planes"),
        "build_outward_plane_solid" => rim_caps::solid(&v,"outward_planes"),
        "build_outward_inferred_solid" => rim_caps::solid(&v,"outward_inferred"),
        op @ ("measure_polyhedral_volume"|"measure_polyhedral_centroid"|"measure_polyhedral_inertia"|"measure_polyhedral_face_areas"|"bound_polyhedral_area"|"bound_polyhedral_edge_length")=>polyhedral_mass::measure(&v,op),
        op @ ("project_polyhedral_body"|"measure_polyhedral_frame"|"measure_polyhedral_axis_gap"|"classify_polyhedral_plane"|"intersect_polyhedral_edges")=>polyhedral_mass::measure(&v,op),
        op @ ("section_polyhedral_faces"|"assemble_polyhedral_section_loops"|"measure_polyhedral_section_area"|"measure_polyhedral_section_centroid"|"bound_polyhedral_section_perimeter")=>polyhedral_mass::measure_section(&v,op),
        op @ ("locate_section_point"|"measure_section_contour_distance"|"measure_section_material_distance"|"clip_section_segment"|"clip_section_polyline"|"measure_section_path_length")=>section_queries::measure(&v,op),
        op @ ("clip_polyhedral_segment"|"locate_polyhedral_point"|"measure_polyhedral_boundary_distance"|"measure_polyhedral_signed_distance"|"sample_polyhedral_signed_distances"|"measure_polyhedral_ball_clearance")=>body_queries::measure(&v,op),
        op @ ("node_spatial_path"|"triangulate_planar_path"|"measure_path_second_moments"|"bound_path_principal_moments"|"measure_region_mass"|"intersect_transverse_profiles"|"measure_profile_overlap_area"|"measure_profile_overlap_centroid"|"classify_profile_overlap"|"decompose_profile_boolean"|"extract_profile_boolean_edges"|"assemble_profile_boolean_loops"|"triangulate_profile_boolean"|"measure_parallel_sweep_overlap"|"mesh_parallel_sweep_overlap"|"build_parallel_sweep_overlap"|"build_profile_boolean_solid"|"measure_profile_extrusion_inertia"|"locate_profile_extrusion_points"|"section_profile_extrusion"|"section_profile_extrusion_parallel"|"split_profile_extrusion_volume"|"clip_profile_extrusion_path"|"bound_profile_extrusion_area"|"extrude_profile_boolean_shell"|"decompose_profile_overlap"|"clip_path_to_perforated_region"|"measure_region_second_moments"|"measure_planar_path_area"|"measure_planar_path_centroid"|"locate_planar_path_points"|"clip_path_to_planar_region"|"project_to_planar_region"|"simplify_exact_path"|"project_point_to_path"|"intersect_spatial_paths"|"find_path_self_intersections"|"measure_spatial_path_distance"|"simplify_path_with_tolerance")=>spatial_trace::measure(&v,op),
        op @ ("clip_common_path"|"clip_union_path"|"subtract_path_cutters"|"classify_path_membership"|"bound_overlap_lengths"|"recover_overlap_paths")=>assembly_path::measure(&v,op),
        op @ ("bound_path_material_length"|"clip_exterior_path"|"clip_interior_path"|"partition_solid_path"|"bound_path_length_breakdown"|"find_path_face_contacts"|"recover_material_path"|"clip_path_to_material"|"find_path_boundary_events")=>path_contacts::measure(&v,op),
        op @ ("measure_swept_ball_gap"|"measure_swept_path_gap"|"classify_swept_path_material"|"bound_swept_first_contact"|"bound_swept_face_ranges")=>swept_clearance::measure(&v,op),
        "build_section_solid" => rim_caps::solid(&v,"exact"),
        op @ ("infer_rim_planes" | "close_inferred_rims" | "close_bounded_inferred_rims") => inferred_rims::infer(&v,op),
        "build_inferred_solid" => rim_caps::solid(&v,"inferred"),
        "close_section_planes" => rim_caps::close_planes(&v,"exact"),
        "build_bounded_section_solid" => rim_caps::solid(&v,"bounded"),
        "close_bounded_planes" => rim_caps::close_planes(&v,"bounded"),
        op @ ("attach_section_rims" | "discover_section_rims" | "attach_bounded_rims" | "discover_bounded_rims") => rim_caps::attach(&v,op),
        op @ ("expanded_section_sheets"|"section_brep_sheets"|"bounded_section_brep_sheets")=>section_sheets::build(&v,op),
        "brep_partition_closed_ss" => closed_ss::partition(v),
        "brep_bound_exact_periodic_trim_directional_area" => brep::exact_periodic_trim_directional_area_request(&v),
        "brep_measure_exact_periodic_trim_window_mass_properties" => brep::exact_periodic_trim_surface_window_mass_request(&v),
        "brep_measure_exact_periodic_trim_patch_set_density" => brep::exact_periodic_trim_patch_set_density_request(&v),
        "brep_measure_exact_periodic_trim_patch_set_mass_properties" => brep::exact_periodic_trim_patch_set_mass_request(&v),
        "brep_measure_exact_periodic_trim_surface_mass_properties" => brep::exact_periodic_trim_surface_mass_properties_request(&v),
        "brep_measure_exact_periodic_trim_surface_area" => brep::exact_periodic_trim_surface_area_request(&v),
        "brep_bound_exact_periodic_trim_patch_area" => brep::exact_periodic_trim_patch_area_request(&v),
        "brep_divide_exact_periodic_trim_contour_by_length" => brep::exact_periodic_trim_contour_division_request(&v),
        "brep_enclose_exact_periodic_trim_contour_station" => brep::exact_periodic_trim_contour_station_bounds_request(&v),
        "brep_locate_exact_periodic_trim_contour_station" => brep::exact_periodic_trim_contour_station_request(&v),
        "brep_measure_exact_periodic_trim_range" => brep::exact_periodic_trim_range_length_request(&v),
        "brep_audit_exact_periodic_trim_contour" => brep::exact_periodic_trim_contour_audit_request(&v),
        op @ ("locate_edge_parameter" | "locate_input_parameter" | "locate_input_range" | "extract_input_range_curves" | "extract_input_range_curve_pairs") => brep::periodic_trim_binding_query(&v,op),
        "surface_trim_sheet_display" => brep::periodic_sheet_delivery(&v,false),
        "surface_trim_sheet_step" => brep::periodic_sheet_delivery(&v,true),
        "brep_build_exact_periodic_trim_sheet" => brep::exact_periodic_trim_sheet_request(&v),
        "brep_build_exact_periodic_trim_sheet_with_input_bindings" => brep::exact_periodic_trim_sheet_input_bindings_request(&v),
        "brep_build_exact_periodic_trim_sheet_with_bindings" => brep::exact_periodic_trim_sheet_bindings_request(&v),
        "brep_qualify_periodic_band" => closed_ss::qualify_periodic_band(v),
        "brep_intersect_periodic_stock_band" => closed_ss::intersect_periodic_stock_band(v),
        "brep_qualify_isolated_ss_contact" => closed_ss::qualify_isolated_contact(v),
        "brep_assemble_closed_ss_boolean" => closed_ss::assemble_boolean(v, false),
        "brep_assemble_closed_ss_network_boolean" => closed_ss::assemble_boolean(v, true),
        "brep_partition_all_closed_ss" => closed_ss::network(v),
        "brep_boundary_certificate" => cad_boundary_certificate::inspect(v),
        "brep_nurbs_bezier_profiles_extrude"=>extrude_profiles(v),
        "minkowski_3d_certify" | "minkowski_3d_verify" | "minkowski_nurbs_certify" | "minkowski_nurbs_verify" | "nurbs_source_atlas_certify" | "nurbs_source_atlas_verify" => cad_minkowski3d::dispatch(v),
        "brep_closed_patch_network" => cad_geometric_g2::create_closed_patch_network(v),
        "brep_g2_surface_bridge" => cad_geometric_g2::create_surface_bridge(v),
        "brep_g2_runout_surface" => cad_runout::create(v),
        "brep_source_graph_g2_preview" => cad_runout::source_graph_preview(v),
        "brep_frozen_graph_body_restore" => cad_runout::restore_graph_body(v),
        "brep_frozen_graph_body_display" => cad_runout::display_graph_body(v),
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
