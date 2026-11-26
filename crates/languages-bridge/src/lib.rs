//! OpenSCAD and RushGraph frontends behind their own linear-memory ABI.
//!
//! Split out of `geometry-bridge` so the geometry kernel ships without a language frontend: this crate
//! only parses and compiles source into documents, and the host runs the geometry operations they name.
// Adapters turn bad input into typed errors instead of panicking (refactor plan L6).
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
pub mod abi;
mod openscad;
use value_codec::{Value, json};

/// Source-to-graph frontend shared by browser workers and native callers.
pub fn compile_rush_frontend(source: &str) -> String {
    match rush_frontend::compile_document(source) {
        Ok(value) => json!({"ok":true,"value":value}).to_string(),
        Err(error) => json!({"ok":false,"message":error.message}).to_string(),
    }
}

/// Canonical graph preparation; errors retain the public RushGraph code/path/details.
pub fn compile_rush(input: &str) -> String {
    runtime_response(
        parse_graph_input(input).and_then(rush_runtime::compile),
        None,
    )
}
pub fn compile_rush_nurbs(input: &str) -> String {
    runtime_response(
        parse_graph_input(input).and_then(rush_runtime::nurbs::compile),
        None,
    )
}
fn parse_graph_input(input: &str) -> rush_runtime::Result<Value> {
    if input.len() > 2 * 1024 * 1024 {
        return Err(rush_runtime::Error::new(
            "input_limit",
            "/",
            "Document exceeds transport limit.",
        ));
    }
    value_codec::from_str(input)
        .map_err(|e| rush_runtime::Error::new("invalid_document", "/", e.to_string()))
}
fn runtime_response(
    result: rush_runtime::Result<Value>,
    customizer: Option<Value>,
) -> String {
    runtime_value(result, customizer).to_string()
}
pub(crate) fn runtime_value(
    result: rush_runtime::Result<Value>,
    customizer: Option<Value>,
) -> Value {
    match result {
        Ok(value) => json!({"ok":true,"value":value}),
        Err(error) => json!({"ok":false,"error":error,"customizer":customizer}),
    }
}
/// Fused source -> authoring graph -> evaluated graph, with no intermediate JS graph.
pub fn execute_rush_frontend(source: &str) -> String {
    execute_text_value(source).to_string()
}
pub(crate) fn execute_text_value(source: &str) -> Value {
    let mut graph = match rush_frontend::compile_document(source) {
        Ok(graph) => graph,
        Err(error) => {
            return runtime_value(
                Err(rush_runtime::Error::new(
                    "text_error",
                    "",
                    error.message,
                )),
                None,
            );
        }
    };
    let controls = graph["customizer"].take();
    let result = (|| -> rush_runtime::Result<Value> {
        let nodes = graph["nodes"].take();
        let node_list = nodes.as_array().ok_or_else(|| {
            rush_runtime::Error::new("invalid_document", "/nodes", "Expected nodes.")
        })?;
        let own = node_list.iter().any(|n| {
            [
                "rounded_rectangle_profile",
                "capsule_profile",
                "annular_sector_surface",
                "keyway_profile",
                "t_slot_profile",
                "dovetail_profile",
                "poly_v_belt_profile",
                "toothed_belt_profile",
                "o_ring_groove_profile",
                "retaining_ring_profile",
                "hex_fastener_profile",
                "square_fastener_profile",
                "exact_ellipse", "exact_ellipsoid", "exact_ring_torus",
                "exact_frustum_side", "exact_planar_region",
                "polygon_profile",
                "polygon_loft",
                "triangle_mesh",
                "subdivision",
                "sdf_sphere",
                "sdf_box",
                "sdf_torus",
                "brep_box",
                "brep_sphere",
                "brep_torus",
                "brep_cylinder",
                "brep_frustum",
                "brep_tube",
                "rack_profile", "brep_rack", "planet_gear_occurrences","planet_occurrences_step","planetary_gear_set_step","verified_bevel_gear", "radial_cam_profile", "radial_cam_walls", "finite_external_thread", "finite_internal_thread", "finite_za_worm", "finite_za_wheel",
                "brep_gear",
                "brep_rational_loft",
                "brep_extrude_curves",
                "parabola_curve",
                "hyperbola_curve",
                "elliptic_cylinder_surface",
                "cone_frustum_surface",
                "quadratic_patch",
                "ribbon_surface", "variable_pipe_surface", "pipe_surface", "screw_surface", "clothoid_curve", "spherical_spiral_curve", "toroidal_spiral_curve", "torus_knot_curve", "helicoid_patches", "circle_rectangle_transition", "ellipse_transition_surface", "circle_transition_surface", "helicoid_surface", "catenoid_patches", "curve_from_curvature_samples", "fit_cst_sides", "naca_five_digit_profile", "cst_spline_profile", "node_spatial_path","triangulate_planar_path","measure_path_second_moments","bound_path_principal_moments","measure_region_mass","intersect_transverse_profiles","measure_profile_overlap_area","measure_profile_overlap_centroid","classify_profile_overlap","decompose_profile_boolean","extract_profile_boolean_edges","assemble_profile_boolean_loops","triangulate_profile_boolean","measure_parallel_sweep_overlap","mesh_parallel_sweep_overlap","build_parallel_sweep_overlap","build_profile_boolean_solid","measure_profile_extrusion_inertia","locate_profile_extrusion_points","section_profile_extrusion","section_profile_extrusion_parallel","split_profile_extrusion_volume","clip_profile_extrusion_path","bound_profile_extrusion_area","extrude_profile_boolean_shell","decompose_profile_overlap","clip_path_to_perforated_region","measure_region_second_moments","measure_planar_path_area","measure_planar_path_centroid","locate_planar_path_points","clip_path_to_planar_region","project_to_planar_region","simplify_exact_path","project_point_to_path","intersect_spatial_paths","find_path_self_intersections","measure_spatial_path_distance","simplify_path_with_tolerance","cst_section_integrals", "fit_sphere_cloud", "fit_cylinder_cloud", "fit_cone_cloud", "fit_torus_cloud", "fit_plane_cloud", "superellipsoid_patches", "corrugated_bellows", "surface_highlight_lines", "surface_reflection_lines", "surface_isophote_analysis", "surface_dent_detection", "fit_biarc_piece", "simplify_linear_curve", "extend_terminal_curve", "extend_surface_boundary", "affine_control_support", "surface_trace_agreement", "signed_shell_volume", "geometry_bounds_union", "geometry_conditioning", "coordinate_frame_local", "coordinate_frame_world", "shared_patch_mesh", "bounded_curve_chord", "bounded_surface_mesh", "store_plane_branch", "split_cell_by_plane", "pair_plane_branch", "close_plane_strip", "split_periodic_plane_strip", "store_closed_plane_piece", "stitch_plane_strip", "trace_plane_strip", "sample_plane_branch", "plane_branch_uv_polyline", "plane_branch_xyz_polyline", "audit_plane_side", "refine_plane_side", "audit_plane_graphs", "parallel_plane_sections", "parallel_section_piece", "clean_offset_boundary", "planar_toolpath_edge", "embed_xy_curve", "planar_offset_piece", "planar_knot_miter", "phase_aligned_section", "compatible_section", "oriented_section", "uv_loop_closure", "uv_loop_simplicity", "uv_trim_location", "curve_edit_deviation", "surface_edit_deviation", "surface_seam_g0", "surface_seam_g1", "surface_seam_g2", "curve_join_continuity", "curve_self_contacts", "surface_self_contacts", "bounded_surface_partial_field", "surface_partial_field", "bounded_surface_area_normal_field", "surface_area_normal_field", "bounded_surface_gaussian_field", "bounded_surface_mean_squared_field", "bounded_surface_principal_gap_squared_field", "bounded_surface_metric_field", "surface_gaussian_field", "surface_mean_squared_field", "surface_principal_gap_squared_field", "surface_metric_field", "bounded_surface_mixed_partial_field", "surface_mixed_partial_field", "bounded_curve_derivative_field", "curve_derivative_field", "bounded_curve_torsion_field", "bounded_curve_speed_squared_field", "curve_torsion_field", "curve_speed_squared_field", "bounded_curve_binormal_field", "curve_binormal_field", "bounded_curve_curvature_squared_field", "curve_curvature_squared_field", "projective_curve_exact", "projective_surface_exact", "surface_cell_interpolation_error", "surface_triangle_interpolation_error", "surface_grid_interpolation_error", "surface_umbilic_cells", "surface_umbilic_regions", "surface_gap_comparison", "surface_gap_ranges", "surface_gap_regions", "surface_gap_edge_roots", "surface_gap_contour_graph", "surface_gap_contour_samples", "surface_gap_world_polyline", "surface_gap_world_length", "merge_patch_strip", "merge_patch_grid", "inspect_patch_grid", "split_tensor_surface", "tile_tensor_surface", "mobius_bezier_curve", "mobius_bezier_surface", "mobius_spline_curve", "mobius_spline_surface", "fit_mobius_curve", "fit_mobius_surface", "affine_clock_curve", "piecewise_clock_curve", "relayout_clock_curve", "affine_clock_surface", "piecewise_clock_surface", "relayout_clock_surface", "pullback_uv_clock", "affine_trim_surface", "transpose_trim_surface", "remap_trim_surface", "reclock_trim_surface", "affine_trim_piece", "transpose_trim_piece", "remap_trim_piece", "reclock_trim_piece", "certify_uv_chart", "classify_uv_seam", "isolate_uv_line", "isolate_uv_seam", "split_uv_seam", "lift_uv_edge", "transfer_uv_grid_piece", "relayout_trim_surface", "relayout_trim_piece", "verify_uv_edge", "compare_seam_orientations", "compare_uv_traces", "compare_uv_tangents", "check_seam_gap", "check_seam_angle", "isolate_seam_gap", "isolate_seam_angle", "active_surface_hull", "project_plane_chart", "store_section_edge", "optimize_section_edge", "store_section_edges", "build_material_segments", "material_segment_curve", "classify_section_gaps", "assemble_section_material", "measure_section_material", "partition_face_events", "enclose_face_events", "match_quadratic_contacts", "shared_boundary_spans", "shared_boundary_contacts", "refine_boundary_section", "resolve_boundary_section", "bound_boundary_section", "classify_face_section", "clip_plane_section", "build_plane_section_segments", "intersect_support_planes", "lift_plane_chart", "exact_wire_area", "bound_wire_moments", "cap_section_wire", "attach_expanded_rims","attach_section_rims", "discover_expanded_rims","discover_section_rims", "attach_bounded_rims", "build_expanded_plane_solid","build_outward_plane_solid","build_section_solid","infer_rim_planes","close_inferred_rims","close_bounded_inferred_rims","inspect_inferred_solid","build_outward_inferred_solid","build_expanded_solid","build_inferred_solid","close_expanded_planes","project_polyhedral_body","measure_polyhedral_frame","measure_polyhedral_axis_gap","section_polyhedral_faces","assemble_polyhedral_section_loops","measure_polyhedral_section_area","measure_polyhedral_section_centroid","bound_polyhedral_section_perimeter","clip_polyhedral_segment","locate_polyhedral_point","measure_polyhedral_boundary_distance","measure_polyhedral_signed_distance","sample_polyhedral_signed_distances","measure_swept_ball_gap","bound_path_material_length","clip_exterior_path","clip_interior_path","partition_solid_path","bound_path_length_breakdown","find_path_face_contacts","clip_common_path","clip_union_path","subtract_path_cutters","classify_path_membership","bound_overlap_lengths","recover_overlap_paths","recover_material_path","clip_path_to_material","find_path_boundary_events","measure_swept_path_gap","classify_swept_path_material","bound_swept_first_contact","bound_swept_face_ranges","measure_polyhedral_ball_clearance","locate_section_point","measure_section_contour_distance","measure_section_material_distance","clip_section_segment","clip_section_polyline","measure_section_path_length","classify_polyhedral_plane","intersect_polyhedral_edges","measure_polyhedral_volume","measure_polyhedral_centroid","measure_polyhedral_inertia","measure_polyhedral_face_areas","bound_polyhedral_area","bound_polyhedral_edge_length","close_section_planes","build_bounded_section_solid","close_bounded_planes","discover_bounded_rims", "expanded_section_sheets","section_brep_sheets", "bounded_section_brep_sheets", "cap_section_holes", "arrange_section_caps", "exact_wire_mass", "exact_wire_inertia", "bound_world_face_mass", "bound_principal_face_moments", "bound_principal_face_directions", "world_face_mass", "principal_face_moments", "principal_face_directions", "coplanar_ray_contacts", "coplanar_ray_partition", "cast_trimmed_face_ray", "planar_face_perimeter", "planar_boundary_distance", "planar_material_distance", "signed_planar_face_area", "project_planar_wire", "lift_planar_wire", "shared_plane_chart", "compress_tensor_surface", "compress_tensor_surface_batch", "surface_gap_pair_distance", "surface_gap_plane_section", "surface_gap_world_distance", "surface_gap_world_bounds", "surface_gap_uv_polyline", "surface_gap_adaptive_polyline", "surface_gap_second_derivative", "surface_gap_second_order_polyline", "surface_gap_contour_slope", "surface_quadric_support", "surface_span_regularity", "curve_plane_roots", "curve_pair_crossings", "curve_plane_chart_crossings", "surface_ray_hits", "curve_curvature_torsion", "surface_principal_curvatures", "curve_nearest_point", "surface_nearest_point", "join_affine_c1" , "join_affine_c2" , "join_projective_c2" , "join_stationary_c2" , "store_periodic_curve" , "rotate_periodic_knot" , "insert_periodic_seam" , "open_periodic_curve" , "trim_periodic_curve" , "trim_periodic_turns" , "partition_periodic_curve" , "combine_surface_measures", "combine_surface_densities", "mass_weighted_centroid", "physical_principal_moments", "physical_moment_gaps", "find_principal_axis", "bound_axis_projector", "certify_principal_direction", "minimum_mixed_gap","mixed_path_clearance","violation_path_length","violation_obstacle_lengths","violation_path_duration","chain_violation_duration","last_chain_violation","chain_clearance_timeline","longest_chain_violation","minimum_synchronized_gap","synchronized_contact_intervals","minimum_tool_pair_gap","tool_pair_intervals","first_tool_pair_contact","last_tool_pair_contact","tool_contact_duration","longest_tool_contact","peak_tool_pair_count","simultaneous_tool_duration","simultaneous_tool_windows","peak_tool_contacts","tool_pairs_at_time","minimum_scheduled_gap","scheduled_contact_intervals","held_contact_intervals","minimum_held_gap","minimum_affine_gap","affine_contact_intervals","first_chain_violation","minimum_chain_gap","mixed_margin_clearance","variable_sphere_clearance","minimum_variable_sphere_gap","variable_scene_clearance","minimum_variable_scene_gap","profiled_scene_clearance","minimum_profiled_scene_gap","minimum_sphere_gap", "minimum_box_gap", "minimum_oriented_box_gap", "minimum_capsule_gap", "angular_speed_limit","angular_acceleration_limit","angular_jerk_limit","orientation_sector","relative_orientation_sector","relative_angular_limits","orientation_winding","curve_winding_at_point","contour_fill_at_point","curve_winding_over_box","contour_box_distance","contour_disk_distance","contour_capsule_distance","contour_rectangle_distance","planar_contour_sets_distance","contour_placement","contour_nesting","orient_fill_contour","contour_fill_area","contour_fill_centroid","contour_central_moments","contour_principal_moments","contour_principal_axes","contour_nonzero_area","contour_fill_perimeter","contour_fill_topology","contour_boundary_signed_distance","contour_hierarchy","contour_fill_orientations","step_ray_crossings","step_first_ray_crossing","step_ray_candidates","step_ray_intervals","step_ray_length","qualify_step_instance","step_ray_coverage","step_ray_multiplicity","step_pair_clearance","step_clearance_checks","step_minimum_clearance","step_polyhedral_sections","make_step_assembly","step_part_tolerances","set_scaled_step_placement","step_occurrence_bounds","step_proximity_pairs","step_common_material","step_interferences","step_material_volume","step_material_centroid","step_material_inertia","step_material_area","step_principal_inertia","step_mass_properties","open_step_file","read_shared_step","replace_step_part","replace_step_instance","place_step_instance","remove_step_instance","add_step_instance","insert_step_solid","append_step_assembly","deduplicate_step_parts","extract_step_instances","signed_contour_distance","contour_fill_over_box","signed_contour_box_distance","angular_travel","relative_angular_travel","angular_motion_limits","angular_slowdown_bounds","angular_duration_bounds","angular_motion_deadline","sphere_path_clearance", "capsule_path_clearance", "box_path_clearance", "oriented_box_path_clearance", "first_sphere_contact", "minimum_sphere_partition_gap", "point_workspace", "point_obstacles", "ball_obstacles", "profiled_ball_workspace","minimum_profiled_ball_workspace","profiled_hull_workspace","minimum_profiled_hull_workspace","rotating_hull_workspace","minimum_rotating_hull_gap","rotating_hull_obstacles","rounded_rotating_obstacles","rotating_face_obstacles","rotating_corner_obstacles","rotating_asymmetric_obstacles","rotating_skew_obstacles","rotating_chord_obstacles","rotating_clipped_chord_obstacles","rotating_exact_chord_obstacles","profiled_rotating_obstacles","profiled_rotating_workspace","minimum_profiled_rotating_gap","ball_workspace", "capsule_workspace", "hull_workspace", "hull_obstacles", "partition_axis_position", "partition_axis_positions", "partition_trajectory_bounds", "check_timed_chain", "inspect_knot_turns", "inspect_join_turn", "inspect_chain_turns", "inspect_knot_jumps", "inspect_timed_knot_jumps", "inspect_motion_join", "inspect_motion_chain", "tensor_principal_bounds", "tensor_axis_bounds", "trim_set_axis_bounds", "periodic_trim_set_area", "periodic_trim_set_centroid", "join_mixed_degree" , "compact_join_seam" , "compare_parametric_seam" , "compare_geometric_seam" , "merge_weighted_curves", "join_projective_chain" , "compact_projective_chain" , "close_projective_chain" , "merge_weighted_chain", "merge_curve_exact", "merge_surface_exact", "curve_pair_distance", "curve_surface_distance", "surface_pair_distance", "curve_length_bounds", "curve_station_at_length", "curve_divide_by_length", "surface_area_bounds", "optical_dome", "optical_lens", "cubic_wave_surface", "catenoid_surface", "catenary_curve", "archimedean_spiral_curve", "epicycloid_curve", "hypocycloid_curve", "trochoid_curve", "cycloid_curve", "lissajous_curve", "logarithmic_spiral_curve", "involute_curve", "elliptic_helix_curve", "variable_pitch_helix_curve", "conical_helix_curve", "helix_curve", "twist_sweep", "two_guide_sweep", "scaled_sweep", "clamped_loft_surface", "boundary_fill", "triangular_patch", "gordon_surface", "closed_loft_surface", "natural_loft_surface", "grid_spline_surface", "hermite_patch", "closed_spline_curve", "clamped_spline_curve", "natural_spline_curve", "hermite_curve", "framed_sweep", "formula_curve", "formula_surface", "coons_patch", "ruled_surface", "sphere_surface", "cylinder_surface", "cone_surface",
            "plane_patch", "bilinear_patch", "bezier_surface",
                "bezier_curve", "curve_compose", "round_polyline_curve", "transition_polyline_curve", "brep_miter_sweep","brep_progressive_miter_sweep",
                "line_curve", "polyline_curve", "circle_curve", "circle_arc",
                "hyperboloid_one_sheet", "hyperboloid_two_sheet", "polynomial_graph",
                "polynomial_curve", "polynomial_surface", "rational_polynomial_curve", "rational_polynomial_surface",
                "ellipse_arc",
                "ellipsoid_surface",
                "torus_surface",
                "nurbs_surface",
                "nurbs_curve",
                "mesh_boolean",
            ]
            .contains(&n["op"].as_str().unwrap_or(""))
        });
        let mut compiled = if own {
            let has_constraints = graph["constraints"]
                .as_array()
                .is_some_and(|items| !items.is_empty());
            let has_checks = graph["checks"]
                .as_array()
                .is_some_and(|items| !items.is_empty());
            if has_constraints || has_checks {
                return Err(rush_runtime::Error::new(
                    "text_error",
                    "",
                    "NURBS text checks are not supported; use the build topology report",
                ));
            }
            if graph.get("segments").is_some() {
                return Err(rush_runtime::Error::new(
                    "text_error",
                    "",
                    "segments applies only to legacy geometry; use explicit tessellation arguments for own geometry",
                ));
            }
            let Value::Array(nodes) = nodes else {
                return Err(rush_runtime::Error::new("invalid_document", "/nodes", "Expected nodes."));
            };
            let parameters = graph["parameters"].as_array().ok_or_else(|| {
                rush_runtime::Error::new("invalid_document", "/parameters", "Expected parameters.")
            })?;
            let root = graph["root"].as_str().ok_or_else(|| {
                rush_runtime::Error::new("invalid_document", "/root", "Expected root.")
            })?;
            let mut compiled = rush_runtime::nurbs::compile_text(
                nodes,
                parameters,
                root.into(),
            )?;
            compiled["source"] = json!("");
            for key in [
                "source_map",
                "geometry_assertions",
                "constraint_report",
                "sketch_solutions",
                "assembly_components",
                "mechanical_reports",
                "mechanical_parts",
            ] {
                compiled[key] = json!([]);
            }
            compiled
        } else {
            let mut document = value_codec::Map::new();
            document.insert("language".into(), json!("rush/ir-1"));
            document.insert("units".into(), json!("mm"));
            document.insert("nodes".into(), nodes);
            for key in ["parameters", "root"] {
                document.insert(key.into(), graph[key].take());
            }
            if let Some(segments) = graph.get_mut("segments") {
                document.insert("segments".into(), segments.take());
            }
            for (source, target) in [
                ("constraints", "constraints"),
                ("checks", "geometry_assertions"),
            ] {
                if graph[source].as_array().is_some_and(|items| !items.is_empty()) {
                    document.insert(target.into(), graph[source].take());
                }
            }
            rush_runtime::compile(Value::Object(document))?
        };
        compiled["customizer"] = controls.clone();
        Ok(compiled)
    })();
    runtime_value(result, Some(controls))
}

pub fn compile_rush_frontend_nurbs(input: &str) -> String {
    let result = parse_graph_input(input).and_then(|v| {
        let nodes = v["nodes"].as_array().ok_or_else(|| {
            rush_runtime::Error::new("invalid_document", "/nodes", "Expected nodes.")
        })?;
        let parameters = v["parameters"].as_array().ok_or_else(|| {
            rush_runtime::Error::new(
                "invalid_document",
                "/parameters",
                "Expected parameters.",
            )
        })?;
        let root = v["root"].as_str().ok_or_else(|| {
            rush_runtime::Error::new("invalid_document", "/root", "Expected root.")
        })?;
        rush_runtime::nurbs::compile_text(nodes.clone(), parameters, root.into())
    });
    runtime_response(result, None)
}

pub fn abi_language(op: u32, value: Value) -> Value {
    match op {
        1 => match value.as_str() {
            Some(s) => match rush_frontend::compile_document(s) {
                Ok(value) => json!({"ok":true,"value":value}),
                Err(error) => json!({"ok":false,"message":error.message}),
            },
            None => json!({"ok":false,"message":"Expected source string"}),
        },
        2 => runtime_value(rush_runtime::compile(value), None),
        3 => runtime_value(rush_runtime::nurbs::compile(value), None),
        4 => match value.as_str() {
            Some(s) => execute_text_value(s),
            None => runtime_value(
                Err(rush_runtime::Error::new(
                    "text_error",
                    "",
                    "Expected source string",
                )),
                None,
            ),
        },
        5 => runtime_value(
            (|| -> rush_runtime::Result<Value> {
                let nodes = value["nodes"].as_array().ok_or_else(|| {
                    rush_runtime::Error::new("invalid_document", "/nodes", "Expected nodes")
                })?;
                let parameters = value["parameters"].as_array().ok_or_else(|| {
                    rush_runtime::Error::new(
                        "invalid_document",
                        "/parameters",
                        "Expected parameters",
                    )
                })?;
                let root = value["root"].as_str().ok_or_else(|| {
                    rush_runtime::Error::new("invalid_document", "/root", "Expected root")
                })?;
                rush_runtime::nurbs::compile_text(nodes.clone(), parameters, root.into())
            })(),
            None,
        ),
        10 => openscad::scad_compile(&value),
        11 => openscad::scad_eval(&value),
        12 => openscad::extrude_slices(&value),
        13 => openscad::resize(&value),
        14 => openscad::fragments(&value),
        15 => openscad::legacy_segments(&value),
        16 => openscad::scad_geometry_plan(&value),
        17 => openscad::stable_box_plan(&value),
        18 => openscad::stable_radial_plan(&value),
        19 => openscad::stable_transform_analysis(&value),
        20 => openscad::stable_euler_matrix(&value),
        21 => openscad::stable_mirror_matrix(&value),
        22 => openscad::stable_axis_angle_matrix(&value),
        23 => openscad::stable_vector_conversion(&value),
        24 => openscad::stable_euler_arguments(&value),
        25 => openscad::stable_authored_matrix(&value),
        26 => openscad::stable_vector_transform(&value),
        27 => openscad::stable_scalar_rotation(&value),
        28 => openscad::stable_polyhedron_faces(&value),
        29 => openscad::stable_polygon_paths(&value),
        30 => openscad::stable_indexed_policy(&value),
        31 => openscad::stable_extrusion_parameters(&value),
        32 => openscad::stable_revolution_parameters(&value),
        33 => openscad::stable_offset_parameters(&value),
        34 => openscad::stable_child_indices(&value),
        35 => openscad::degree_math(&value),
        _ => {
            json!({"ok":false,"error":{"code":"GEOMETRY_INVALID_INPUT","message":"Unknown ABI operation"}})
        }
    }
}

#[cfg(test)]
mod progressive_body_correction_tests {
    #[test]
    fn corrected_progressive_body_passes_rust_text_and_strict_runtime_schema(){
        for source in [include_str!("../../../examples/rush/arc-length-curved-fixed-normal-body-boundary.r"),
            include_str!("../../../examples/rush/arc-length-curved-fixed-normal-folded-body-boundary.r")]{
            let result=super::execute_text_value(source);
            assert_eq!(result["ok"].as_bool(),Some(true),"{result}");
        }
    }
}
