use super::*;

const REQUIRED_ALL_SIGNATURE_OPS: &[&str] = &[
    "iso_curve",
    "curve_remove_knot", "curve_reduce_degree", "curve_rebuild", "surface_remove_knot", "surface_reduce_degree", "surface_rebuild",
    "surface_split",
    "surface_transpose",
    "surface_bezier_patch",
    "curve_edit_weights", "curve_relocate_seam", "surface_edit_weights", "surface_relocate_seam", "curve_exact_insert_knot", "curve_exact_remove_knot", "curve_exact_reduce_degree", "surface_exact_insert_knot", "surface_exact_remove_knot", "surface_exact_reduce_degree", "curve_normalize_weights", "curve_cancel_factor", "surface_cancel_factor", "curve_exact_split", "curve_exact_restrict", "curve_exact_partition", "curve_exact_elevate", "rotate_periodic_surface_at_exact_knot", "insert_and_rotate_periodic_surface_seam", "open_exact_periodic_surface_axis", "restrict_exact_periodic_surface_axis", "restrict_exact_periodic_surface_over_turns", "restrict_exact_doubly_periodic_surface", "restrict_exact_doubly_periodic_surface_over_turns", "partition_exact_periodic_surface_axis", "partition_exact_periodic_surface_over_turns", "partition_exact_doubly_periodic_surface", "partition_exact_doubly_periodic_surface_over_turns", "remove_exact_periodic_surface_knot", "simplify_exact_periodic_surface_axis", "remove_exact_periodic_surface_seam_knot", "simplify_exact_doubly_periodic_surface", "reduce_exact_periodic_surface_period", "repeat_exact_periodic_surface_period", "curve_reparameterize",
    "curve_split",
    "curve_bezier_span",
    "rounded_rectangle_profile", "capsule_profile", "annular_sector_surface", "keyway_profile", "t_slot_profile", "dovetail_profile", "poly_v_belt_profile", "toothed_belt_profile", "o_ring_groove_profile", "retaining_ring_profile", "hex_fastener_profile", "square_fastener_profile",
    "exact_ellipse", "exact_ellipsoid", "exact_ring_torus", "exact_frustum_side", "exact_planar_region",
    "helix_curve",
    "ribbon_surface",
    "variable_pipe_surface",
    "pipe_surface",
    "screw_surface",
    "clothoid_curve",
    "spherical_spiral_curve",
    "toroidal_spiral_curve",
    "torus_knot_curve",
    "helicoid_patches",
    "circle_rectangle_transition",
    "ellipse_transition_surface",
    "circle_transition_surface",
    "helicoid_surface",
    "catenoid_patches",
    "curve_from_curvature_samples", "fit_cst_sides", "naca_five_digit_profile", "cst_spline_profile", "node_spatial_path","triangulate_planar_path","measure_path_second_moments","bound_path_principal_moments","measure_region_mass","intersect_transverse_profiles","measure_profile_overlap_area","measure_profile_overlap_centroid","classify_profile_overlap","decompose_profile_boolean","extract_profile_boolean_edges","assemble_profile_boolean_loops","triangulate_profile_boolean","measure_parallel_sweep_overlap","mesh_parallel_sweep_overlap","build_parallel_sweep_overlap","build_profile_boolean_solid","measure_profile_extrusion_inertia","locate_profile_extrusion_points","section_profile_extrusion","section_profile_extrusion_parallel","split_profile_extrusion_volume","clip_profile_extrusion_path","bound_profile_extrusion_area","extrude_profile_boolean_shell","decompose_profile_overlap","clip_path_to_perforated_region","measure_region_second_moments","measure_planar_path_area","measure_planar_path_centroid","locate_planar_path_points","clip_path_to_planar_region","project_to_planar_region","simplify_exact_path","project_point_to_path","intersect_spatial_paths","find_path_self_intersections","measure_spatial_path_distance","simplify_path_with_tolerance","cst_section_integrals", "fit_sphere_cloud", "fit_cylinder_cloud", "fit_cone_cloud", "fit_torus_cloud", "fit_plane_cloud", "superellipsoid_patches", "corrugated_bellows", "curve_nearest_point", "surface_nearest_point", "join_affine_c1" , "join_affine_c2" , "join_projective_c2" , "join_stationary_c2" , "store_periodic_curve" , "rotate_periodic_knot" , "insert_periodic_seam" , "open_periodic_curve" , "trim_periodic_curve" , "trim_periodic_turns" , "partition_periodic_curve" , "combine_surface_measures", "combine_surface_densities", "mass_weighted_centroid", "physical_principal_moments", "physical_moment_gaps", "find_principal_axis", "bound_axis_projector", "certify_principal_direction", "minimum_mixed_gap","mixed_path_clearance","violation_path_length","violation_obstacle_lengths","violation_path_duration","chain_violation_duration","last_chain_violation","chain_clearance_timeline","longest_chain_violation","first_chain_violation","minimum_chain_gap","mixed_margin_clearance","minimum_sphere_gap", "minimum_box_gap", "minimum_oriented_box_gap", "minimum_capsule_gap", "sphere_path_clearance", "capsule_path_clearance", "box_path_clearance", "oriented_box_path_clearance", "first_sphere_contact", "minimum_sphere_partition_gap", "point_workspace", "point_obstacles", "ball_obstacles", "ball_workspace", "capsule_workspace", "hull_workspace", "hull_obstacles", "partition_axis_position", "partition_axis_positions", "partition_trajectory_bounds", "check_timed_chain", "inspect_knot_turns", "inspect_join_turn", "inspect_chain_turns", "inspect_knot_jumps", "inspect_timed_knot_jumps", "inspect_motion_join", "inspect_motion_chain", "tensor_principal_bounds", "tensor_axis_bounds", "trim_set_axis_bounds", "periodic_trim_set_area", "periodic_trim_set_centroid", "join_mixed_degree" , "compact_join_seam" , "compare_parametric_seam" , "compare_geometric_seam" , "merge_weighted_curves", "join_projective_chain" , "compact_projective_chain" , "close_projective_chain" , "merge_weighted_chain", "merge_curve_exact", "merge_surface_exact", "curve_pair_distance", "curve_surface_distance", "surface_pair_distance", "curve_length_bounds", "curve_station_at_length", "curve_divide_by_length", "surface_area_bounds", "optical_dome", "optical_lens", "cubic_wave_surface", "catenoid_surface",
    "catenary_curve",
    "archimedean_spiral_curve",
    "epicycloid_curve",
    "hypocycloid_curve",
    "trochoid_curve",
    "cycloid_curve",
    "lissajous_curve",
    "logarithmic_spiral_curve",
    "involute_curve",
    "elliptic_helix_curve",
    "conical_helix_curve",
    "variable_pitch_helix_curve",
];

const MODIFIER_OPS: &[&str] = &[
    "surface_highlight_lines", "surface_reflection_lines", "surface_isophote_analysis", "surface_dent_detection", "fit_biarc_piece", "simplify_linear_curve", "extend_terminal_curve", "extend_surface_boundary", "affine_control_support", "surface_trace_agreement", "signed_shell_volume", "geometry_bounds_union", "geometry_conditioning", "coordinate_frame_local", "coordinate_frame_world", "shared_patch_mesh", "bounded_curve_chord", "bounded_surface_mesh", "store_plane_branch", "split_cell_by_plane", "pair_plane_branch", "close_plane_strip", "split_periodic_plane_strip", "store_closed_plane_piece", "stitch_plane_strip", "trace_plane_strip", "sample_plane_branch", "plane_branch_uv_polyline", "plane_branch_xyz_polyline", "audit_plane_side", "refine_plane_side", "audit_plane_graphs", "parallel_plane_sections", "parallel_section_piece", "clean_offset_boundary", "planar_toolpath_edge", "embed_xy_curve", "planar_offset_piece", "planar_knot_miter", "phase_aligned_section", "compatible_section", "oriented_section", "uv_loop_closure", "uv_loop_simplicity", "uv_trim_location", "curve_edit_deviation", "surface_edit_deviation", "surface_seam_g0", "surface_seam_g1", "surface_seam_g2", "curve_join_continuity", "curve_self_contacts", "surface_self_contacts", "bounded_surface_partial_field", "surface_partial_field", "bounded_surface_area_normal_field", "surface_area_normal_field", "bounded_surface_gaussian_field", "bounded_surface_mean_squared_field", "bounded_surface_principal_gap_squared_field", "bounded_surface_metric_field", "surface_gaussian_field", "surface_mean_squared_field", "surface_principal_gap_squared_field", "surface_metric_field", "bounded_surface_mixed_partial_field", "surface_mixed_partial_field", "bounded_curve_derivative_field", "curve_derivative_field", "bounded_curve_torsion_field", "bounded_curve_speed_squared_field", "curve_torsion_field", "curve_speed_squared_field", "bounded_curve_binormal_field", "curve_binormal_field", "bounded_curve_curvature_squared_field", "curve_curvature_squared_field", "projective_curve_exact", "projective_surface_exact", "surface_cell_interpolation_error", "surface_triangle_interpolation_error", "surface_grid_interpolation_error", "surface_umbilic_cells", "surface_umbilic_regions", "surface_gap_comparison", "surface_gap_ranges", "surface_gap_regions", "surface_gap_edge_roots", "surface_gap_contour_graph", "surface_gap_contour_samples", "surface_gap_world_polyline", "surface_gap_world_length", "merge_patch_strip", "merge_patch_grid", "inspect_patch_grid", "split_tensor_surface", "tile_tensor_surface", "mobius_bezier_curve", "mobius_bezier_surface", "mobius_spline_curve", "mobius_spline_surface", "fit_mobius_curve", "fit_mobius_surface", "affine_clock_curve", "piecewise_clock_curve", "relayout_clock_curve", "affine_clock_surface", "piecewise_clock_surface", "relayout_clock_surface", "pullback_uv_clock", "affine_trim_surface", "transpose_trim_surface", "remap_trim_surface", "reclock_trim_surface", "affine_trim_piece", "transpose_trim_piece", "remap_trim_piece", "reclock_trim_piece", "certify_uv_chart", "classify_uv_seam", "isolate_uv_line", "isolate_uv_seam", "split_uv_seam", "lift_uv_edge", "transfer_uv_grid_piece", "relayout_trim_surface", "relayout_trim_piece", "verify_uv_edge", "compare_seam_orientations", "compare_uv_traces", "compare_uv_tangents", "check_seam_gap", "check_seam_angle", "isolate_seam_gap", "isolate_seam_angle", "active_surface_hull", "project_plane_chart", "store_section_edge", "optimize_section_edge", "store_section_edges", "build_material_segments", "material_segment_curve", "classify_section_gaps", "assemble_section_material", "measure_section_material", "partition_face_events", "enclose_face_events", "match_quadratic_contacts", "shared_boundary_spans", "shared_boundary_contacts", "refine_boundary_section", "resolve_boundary_section", "bound_boundary_section", "classify_face_section", "clip_plane_section", "build_plane_section_segments", "intersect_support_planes", "lift_plane_chart", "exact_wire_area", "bound_wire_moments", "cap_section_wire", "attach_expanded_rims","attach_section_rims", "discover_expanded_rims","discover_section_rims", "attach_bounded_rims", "build_expanded_plane_solid","build_outward_plane_solid","build_section_solid","infer_rim_planes","close_inferred_rims","close_bounded_inferred_rims","inspect_inferred_solid","build_outward_inferred_solid","build_expanded_solid","build_inferred_solid","close_expanded_planes","project_polyhedral_body","measure_polyhedral_frame","measure_polyhedral_axis_gap","section_polyhedral_faces","assemble_polyhedral_section_loops","measure_polyhedral_section_area","measure_polyhedral_section_centroid","bound_polyhedral_section_perimeter","clip_polyhedral_segment","locate_polyhedral_point","measure_polyhedral_boundary_distance","measure_polyhedral_signed_distance","sample_polyhedral_signed_distances","measure_swept_ball_gap","bound_path_material_length","clip_exterior_path","clip_interior_path","partition_solid_path","bound_path_length_breakdown","find_path_face_contacts","clip_common_path","clip_union_path","subtract_path_cutters","classify_path_membership","bound_overlap_lengths","recover_overlap_paths","recover_material_path","clip_path_to_material","find_path_boundary_events","measure_swept_path_gap","classify_swept_path_material","bound_swept_first_contact","bound_swept_face_ranges","measure_polyhedral_ball_clearance","locate_section_point","measure_section_contour_distance","measure_section_material_distance","clip_section_segment","clip_section_polyline","measure_section_path_length","classify_polyhedral_plane","intersect_polyhedral_edges","measure_polyhedral_volume","measure_polyhedral_centroid","measure_polyhedral_inertia","measure_polyhedral_face_areas","bound_polyhedral_area","bound_polyhedral_edge_length","close_section_planes","build_bounded_section_solid","close_bounded_planes","discover_bounded_rims", "expanded_section_sheets","section_brep_sheets", "bounded_section_brep_sheets", "cap_section_holes", "arrange_section_caps", "exact_wire_mass", "exact_wire_inertia", "bound_world_face_mass", "bound_principal_face_moments", "bound_principal_face_directions", "world_face_mass", "principal_face_moments", "principal_face_directions", "coplanar_ray_contacts", "coplanar_ray_partition", "cast_trimmed_face_ray", "planar_face_perimeter", "planar_boundary_distance", "planar_material_distance", "signed_planar_face_area", "project_planar_wire", "lift_planar_wire", "shared_plane_chart", "compress_tensor_surface", "compress_tensor_surface_batch", "surface_gap_pair_distance", "surface_gap_plane_section", "surface_gap_world_distance", "surface_gap_world_bounds", "surface_gap_uv_polyline", "surface_gap_adaptive_polyline", "surface_gap_second_derivative", "surface_gap_second_order_polyline", "surface_gap_contour_slope", "surface_quadric_support", "surface_span_regularity", "curve_plane_roots", "curve_pair_crossings", "curve_plane_chart_crossings", "surface_ray_hits", "curve_curvature_torsion", "surface_principal_curvatures",
    "curve_nearest_point", "surface_nearest_point", "join_affine_c1" , "join_affine_c2" , "join_projective_c2" , "join_stationary_c2" , "store_periodic_curve" , "rotate_periodic_knot" , "insert_periodic_seam" , "open_periodic_curve" , "trim_periodic_curve" , "trim_periodic_turns" , "partition_periodic_curve" , "combine_surface_measures", "combine_surface_densities", "mass_weighted_centroid", "physical_principal_moments", "physical_moment_gaps", "find_principal_axis", "bound_axis_projector", "certify_principal_direction", "minimum_mixed_gap","mixed_path_clearance","violation_path_length","violation_obstacle_lengths","violation_path_duration","chain_violation_duration","last_chain_violation","chain_clearance_timeline","longest_chain_violation","first_chain_violation","minimum_chain_gap","mixed_margin_clearance","minimum_sphere_gap", "minimum_box_gap", "minimum_oriented_box_gap", "minimum_capsule_gap", "sphere_path_clearance", "capsule_path_clearance", "box_path_clearance", "oriented_box_path_clearance", "first_sphere_contact", "minimum_sphere_partition_gap", "point_workspace", "point_obstacles", "ball_obstacles", "ball_workspace", "capsule_workspace", "hull_workspace", "hull_obstacles", "partition_axis_position", "partition_axis_positions", "partition_trajectory_bounds", "check_timed_chain", "inspect_knot_turns", "inspect_join_turn", "inspect_chain_turns", "inspect_knot_jumps", "inspect_timed_knot_jumps", "inspect_motion_join", "inspect_motion_chain", "tensor_principal_bounds", "tensor_axis_bounds", "trim_set_axis_bounds", "periodic_trim_set_area", "periodic_trim_set_centroid", "join_mixed_degree" , "compact_join_seam" , "compare_parametric_seam" , "compare_geometric_seam" , "merge_weighted_curves", "join_projective_chain" , "compact_projective_chain" , "close_projective_chain" , "merge_weighted_chain", "merge_curve_exact", "merge_surface_exact", "curve_pair_distance", "curve_surface_distance", "surface_pair_distance",
    "curve_length_bounds", "curve_station_at_length", "curve_divide_by_length", "surface_area_bounds",
    "curve_axis_motion_partition_speed", "curve_axis_motion_partition_motion", "curve_axis_motion_partition_axes", "curve_axis_motion_limits", "curve_axis_motion_slowdown", "curve_axis_motion_multi_slowdown", "curve_axis_motion_duration", "curve_axis_motion_deadline", "curve_directional_travel_full", "curve_directional_travel_signed", "curve_check_backtracking", "surface_trim_contour_audit", "surface_trim_contour_station", "surface_trim_contour_station_box", "surface_trim_contour_divide", "surface_trim_patch_area_vector", "surface_trim_patch_projected_area", "surface_trim_range_length", "evaluate_exact_wrapped_surface_point", "evaluate_exact_wrapped_surface_points", "lift_exact_periodic_surface_path", "exact_periodic_surface_path_winding", "exact_periodic_surface_path_signed_area", "exact_periodic_surface_path_centroid", "exact_periodic_surface_path_inertia", "classify_exact_periodic_surface_path_point", "classify_exact_periodic_surface_path_points", "classify_exact_periodic_surface_region_points", "certify_exact_periodic_surface_path_simplicity", "check_exact_periodic_surface_path_contact", "relate_exact_periodic_surface_paths", "exact_periodic_surface_contour_forest", "exact_periodic_surface_region_area", "exact_periodic_surface_region_component_centroids", "exact_periodic_surface_region_component_inertia", "measure_input_contour_length", "measure_input_segment_length", "measure_input_segment_range_length", "locate_input_segment_at_length", "measure_input_contour_prefix_lengths", "locate_input_contour_at_lengths", "divide_input_contour_by_length", "exact_input_contour_projected_area", "exact_input_contour_polynomial_projected_area", "bound_input_contour_projected_area", "bound_input_patch_max_projected_area", "bound_input_patch_max_projection_direction", "surface_trim_sheet_display", "surface_trim_sheet_step", "surface_trim_sheet", "surface_trim_sheet_edge_bindings", "surface_trim_sheet_input_bindings", "locate_input_parameter", "locate_edge_parameter", "locate_input_range", "extract_input_range_curves", "extract_input_range_curve_pairs", "evaluate_input_point", "evaluate_input_tangents", "classify_input_tangent_join", "classify_input_vertex_join", "lift_exact_periodic_surface_region_components", "build_exact_periodic_surface_trim_components", "build_exact_periodic_surface_trim_patches", "build_exact_periodic_surface_trim_edges", "trimmed_surface_area_in_window", "trimmed_surface_centroid", "trimmed_surface_area_inertia", "surface_trim_area", "surface_trim_mass_properties", "surface_trim_window_properties", "surface_trim_set_properties", "surface_measure_trimmed_mass", "curve_axial_derivative_peak", "curve_measure_directional_travel", "curve_measure_directional_travel_batch",
    "polygon_extrude",
    "polygon_sweep",
    "mesh_to_nurbs_brep",
    "mesh_to_sdf",
    "mesh_to_subdivision",
    "subdivision_tessellate",
    "mesh_to_nurbs",
    "mesh_fit_nurbs",
    "nurbs_patches_tessellate",
    "sdf_tessellate",
    "sdf_offset",
    "sdf_translate",
    "brep_tessellate",
    "brep_smooth_miter_stations",
    "brep_smooth_sweep_stations",
    "brep_extrude",
    "brep_revolve",
    "brep_chamfer",
    "brep_fillet",
    "brep_full_round_exact",
    "ribbon_surface",
    "variable_pipe_surface",
    "pipe_surface",
    "screw_surface",
    "surface_extrude_patches",
    "iso_curve",
    "curve_remove_knot", "curve_reduce_degree", "curve_rebuild", "surface_remove_knot", "surface_reduce_degree", "surface_rebuild",
    "surface_split",
    "surface_transpose",
    "surface_bezier_patch",
    "curve_edit_weights", "curve_relocate_seam", "surface_edit_weights", "surface_relocate_seam", "curve_exact_insert_knot", "curve_exact_remove_knot", "curve_exact_reduce_degree", "surface_exact_insert_knot", "surface_exact_remove_knot", "surface_exact_reduce_degree", "curve_normalize_weights", "curve_cancel_factor", "surface_cancel_factor", "curve_exact_split", "curve_exact_restrict", "curve_exact_partition", "curve_exact_elevate", "rotate_periodic_surface_at_exact_knot", "insert_and_rotate_periodic_surface_seam", "open_exact_periodic_surface_axis", "restrict_exact_periodic_surface_axis", "restrict_exact_periodic_surface_over_turns", "restrict_exact_doubly_periodic_surface", "restrict_exact_doubly_periodic_surface_over_turns", "partition_exact_periodic_surface_axis", "partition_exact_periodic_surface_over_turns", "partition_exact_doubly_periodic_surface", "partition_exact_doubly_periodic_surface_over_turns", "remove_exact_periodic_surface_knot", "simplify_exact_periodic_surface_axis", "remove_exact_periodic_surface_seam_knot", "simplify_exact_doubly_periodic_surface", "reduce_exact_periodic_surface_period", "repeat_exact_periodic_surface_period", "curve_reparameterize",
    "curve_split",
    "curve_bezier_span",
    "curve_edit",
    "surface_edit",
    "surface_extrude",
    "surface_revolve",
    "tessellate",
    "thicken",
    "transform",
    "extrude",
    "revolve",
    "translate",
    "rotate",
    "scale",
    "offset",
    "mirror",
];

impl Compiler {
    pub(super) fn signature_operation(
        &mut self,
        name: &str,
        input: Option<String>,
        mut args: Vec<(Option<String>, V)>,
    ) -> R<V> {
        normalize_shorthand_args(name, &mut args)?;
        if name == "offset" && args.iter().any(|(n, _)| n.as_deref() == Some("delta")) {
            if input.is_none() || args.len() != 1 || args[0].0.as_deref() != Some("delta") {
                return Err(crate::error(
                    "offset(delta: distance) requires one argument and piped geometry",
                ));
            }
            return self.add(
                json!({"op":"offset","input":input,"distance":raw(args.remove(0).1)?,"mode":"delta"}),
            );
        }
        let sig =
            signature(name).ok_or_else(|| crate::error(format!("Unknown operation {name}")))?;
        let mut node = json!({"op":name});
        for (i, (n, v)) in args.into_iter().enumerate() {
            let key = n
                .as_deref()
                .or_else(|| sig.get(i).copied())
                .ok_or_else(|| crate::error("Unknown or duplicate argument"))?;
            if !sig.contains(&key) || node.get(key).is_some() {
                return Err(crate::error("Unknown or duplicate argument"));
            }
            node[key] = self.lower_signature_argument(name, key, v)?;
        }
        if REQUIRED_ALL_SIGNATURE_OPS.contains(&name) {
            for key in sig {
                if node.get(*key).is_none() {
                    return Err(crate::error(format!("{name} requires {key}")));
                }
            }
        }
        self.finish_piped_or_standalone_node(name, input, node)
    }

    fn lower_signature_argument(&mut self, name: &str, key: &str, v: V) -> R<J> {
        if key == "continuity_class" {
            let value=raw(v)?;
            return match value.as_str().or_else(||value.get("text").and_then(J::as_str)) {
                Some(s @ ("G1" | "G2" | "C2"))=>Ok(J::String(s.to_owned())),
                _=>Err(crate::error("Expected G1, G2 or C2 for the extension continuity class")),
            };
        }
        if matches!(key, "source_edge" | "target_edge") {
            let value = raw(v)?;
            let edge = value.as_str().or_else(|| value.get("text").and_then(J::as_str));
            return match edge {
                Some(s @ ("u_min" | "u_max" | "v_min" | "v_max")) => Ok(J::String(s.to_owned())),
                _ => Err(crate::error("Expected u_min, u_max, v_min or v_max")),
            };
        }
        if matches!(key, "source_end" | "target_end") {
            let value = raw(v)?;
            let end = value.as_str().or_else(|| value.get("text").and_then(J::as_str));
            return match end {
                Some(s @ ("start" | "end")) => Ok(J::String(s.to_owned())),
                _ => Err(crate::error("Expected start or end for the curve endpoint")),
            };
        }
        if key == "boolean_mode" {
            let value=raw(v)?;
            return match value.as_str().or_else(||value.get("text").and_then(J::as_str)) {
                Some(s @ ("intersection"|"union"|"difference"|"symmetric_difference"))=>Ok(J::String(s.to_owned())),
                _=>Err(crate::error("Expected a profile Boolean mode"))
            };
        }
        if matches!(key, "query_side" | "u_side" | "v_side") {
            let value = raw(v)?;
            let side = value.as_str().or_else(|| value.get("text").and_then(J::as_str));
            return match side {
                Some(s @ ("automatic" | "left" | "right")) => Ok(J::String(s.to_owned())),
                _ => Err(crate::error("Expected automatic, left or right as the derivative side")),
            };
        }
        if key == "laws" && matches!(name, "radial_cam_profile" | "radial_cam_walls") {
            let V::Array(entries)=v else {return Err(crate::error("Cam motion laws must be a list"));};
            let laws=entries.into_iter().map(|entry|{
                let value=raw(entry)?;
                value.as_str().or_else(||value.get("text").and_then(J::as_str))
                    .map(|s|J::String(s.to_owned())).ok_or_else(||crate::error("Expected a cam motion law name"))
            }).collect::<R<Vec<_>>>()?;
            return Ok(J::Array(laws));
        }
        if matches!(key, "query_point_mm" | "other_sweep_mm" | "cut_normal" | "cut_offset" | "samples_mm" | "sweep_mm" | "query_path_mm" | "other_holes_mm" | "holes_mm" | "deviation_mm" | "other_path_mm" | "query_radius_mm") {
            fn coordinate_values(v: V) -> R<J> {
                match v {
                    V::Array(values) => values.into_iter().map(coordinate_values).collect::<R<Vec<_>>>().map(J::Array),
                    value => {
                        let j=raw(value)?;
                        Ok(j.get("text").cloned().unwrap_or(j))
                    }
                }
            }
            return coordinate_values(v);
        }
        if matches!(key, "range_start" | "range_end" | "exact_parameter" | "edge_id") {
            let value = raw(v)?;
            let ratio = value.as_str().or_else(|| value.get("text").and_then(J::as_str))
                .ok_or_else(|| crate::error("Expected a rational parameter as numerator/denominator text"))?;
            return Ok(J::String(ratio.to_owned()));
        }
        if key == "inputs" && matches!(name,"chain_violation_duration" | "last_chain_violation" | "chain_clearance_timeline" | "longest_chain_violation" | "first_chain_violation" | "minimum_chain_gap" | "check_timed_chain" | "inspect_join_turn" | "inspect_chain_turns" | "inspect_motion_join" | "inspect_motion_chain" | "combine_surface_measures" | "combine_surface_densities" | "join_projective_chain" | "compact_projective_chain" | "close_projective_chain" | "merge_weighted_chain" | "clip_common_path" | "clip_union_path" | "subtract_path_cutters" | "classify_path_membership" | "bound_overlap_lengths" | "recover_overlap_paths" | "check_seam_gap" | "check_seam_angle" | "isolate_seam_gap" | "isolate_seam_angle" | "verify_uv_edge" | "compare_seam_orientations" | "compare_uv_traces" | "compare_uv_tangents" | "merge_patch_strip" | "merge_patch_grid" | "inspect_patch_grid" | "project_planar_wire"  | "exact_wire_area" | "bound_wire_moments" | "cap_section_wire" | "expanded_section_sheets"|"section_brep_sheets" | "bounded_section_brep_sheets" | "cap_section_holes" | "arrange_section_caps" | "exact_wire_mass" | "exact_wire_inertia" | "shared_plane_chart" | "compress_tensor_surface_batch" | "surface_trace_agreement" | "signed_shell_volume" | "geometry_bounds_union" | "shared_patch_mesh" | "clean_offset_boundary" | "planar_toolpath_edge" | "phase_aligned_section" | "compatible_section" | "oriented_section") {
            let V::Array(curves) = v else { return Err(crate::error("Expected a list of additional sections")); };
            let maximum = if name=="geometry_bounds_union" {2047} else if matches!(name,"cap_section_wire"|"expanded_section_sheets"|"section_brep_sheets"|"bounded_section_brep_sheets"|"cap_section_holes"|"arrange_section_caps"){255} else if matches!(name, "clean_offset_boundary" | "planar_toolpath_edge") {255} else {31};
            if curves.len()>maximum { return Err(crate::error("Too many additional curve references")); }
            return curves.into_iter().map(|curve| self.geometry(curve).map(J::String)).collect::<R<Vec<_>>>().map(J::Array);
        }
        if key == "target_geometry" && matches!(name,"join_affine_c1" | "join_affine_c2" | "join_projective_c2" | "join_stationary_c2" | "join_mixed_degree" | "compact_join_seam" | "compare_parametric_seam" | "compare_geometric_seam" | "merge_weighted_curves" | "measure_polyhedral_axis_gap" | "check_seam_gap" | "check_seam_angle" | "isolate_seam_gap" | "isolate_seam_angle" | "verify_uv_edge" | "compare_seam_orientations" | "compare_uv_traces" | "compare_uv_tangents" | "store_section_edge" | "optimize_section_edge" | "store_section_edges" | "build_material_segments" | "material_segment_curve" | "classify_section_gaps" | "assemble_section_material" | "measure_section_material" | "partition_face_events" | "enclose_face_events" | "match_quadratic_contacts" | "shared_boundary_spans" | "shared_boundary_contacts" | "refine_boundary_section" | "resolve_boundary_section" | "bound_boundary_section" | "classify_face_section" | "clip_plane_section" | "build_plane_section_segments" | "intersect_support_planes" | "lift_plane_chart" | "lift_uv_edge" | "surface_gap_pair_distance" | "curve_edit_deviation" | "surface_edit_deviation" | "surface_seam_g0" | "surface_seam_g1" | "surface_seam_g2" | "curve_join_continuity" | "curve_pair_crossings" | "curve_plane_chart_crossings" | "merge_curve_exact" | "merge_surface_exact" | "curve_pair_distance" | "curve_surface_distance" | "surface_pair_distance") {
            return Ok(J::String(self.geometry(v)?));
        }
        if (name == "ribbon_surface" && key == "width_law")
            || (name == "variable_pipe_surface" && key == "radius_law")
        {
            return Ok(J::String(self.geometry(v)?));
        }
        if key == "loops" && matches!(name, "brep_extrude_curves" | "bound_wire_moments" | "cap_section_wire" | "expanded_section_sheets"|"section_brep_sheets" | "bounded_section_brep_sheets" | "cap_section_holes" | "arrange_section_caps" | "exact_wire_mass" | "exact_wire_inertia" | "bound_world_face_mass" | "bound_principal_face_moments" | "bound_principal_face_directions" | "world_face_mass" | "principal_face_moments" | "principal_face_directions" | "coplanar_ray_contacts" | "coplanar_ray_partition" | "store_section_edge" | "optimize_section_edge" | "store_section_edges" | "build_material_segments" | "material_segment_curve" | "classify_section_gaps" | "assemble_section_material" | "measure_section_material" | "partition_face_events" | "enclose_face_events" | "match_quadratic_contacts" | "shared_boundary_spans" | "shared_boundary_contacts" | "refine_boundary_section" | "resolve_boundary_section" | "bound_boundary_section" | "classify_face_section" | "clip_plane_section" | "build_plane_section_segments" | "cast_trimmed_face_ray" | "planar_face_perimeter" | "planar_boundary_distance" | "planar_material_distance" | "signed_planar_face_area" | "project_planar_wire" | "lift_planar_wire" | "relayout_trim_surface" | "relayout_trim_piece" | "affine_trim_surface" | "transpose_trim_surface" | "remap_trim_surface" | "reclock_trim_surface" | "affine_trim_piece" | "transpose_trim_piece" | "remap_trim_piece" | "reclock_trim_piece" | "uv_loop_closure" | "uv_loop_simplicity" | "uv_trim_location" | "trimmed_surface_area_in_window" | "trimmed_surface_centroid" | "trimmed_surface_area_inertia") {
            return self.lower_extrude_curve_loops(v);
        }
        if [
            "herringbone",
            "verified_profile", "return_field", "inspect_spans",
            "positive_side", "use_pcurve",
            "internal",
            "principal_axes",
            "static_unbalance",
            "couple_unbalance",
            "radius_of_gyration",
            "even_odd",
            "left_handed",
            "lower",
            "closed",
            "periodic",
            "periodic_u",
            "periodic_v",
            "reverse",
            "reverse_u",
            "reverse_v",
        ]
        .contains(&key)
        {
            // Text has no boolean literal: true/false lower to 1/0, and the
            // canonical schema wants a JSON boolean for these flags.
            return match raw(v)? {
                J::Bool(b) => Ok(J::Bool(b)),
                J::Number(n) => Ok(J::Bool(n.as_f64().is_some_and(|x| x != 0.))),
                _ => Err(crate::error(format!("{key} expects true or false"))),
            };
        }
        raw(v)
    }

    fn lower_extrude_curve_loops(&mut self, v: V) -> R<J> {
        let V::Array(loops) = v else {
            return Err(crate::error("Curve extrusion requires nested curve lists"));
        };
        if loops.len() > 64 {
            return Err(crate::error("Curve extrusion accepts at most 64 loops"));
        }
        let mut count = 0;
        let lowered = loops
            .into_iter()
            .map(|wire| {
                let V::Array(curves) = wire else {
                    return Err(crate::error("Curve extrusion requires nested curve lists"));
                };
                count += curves.len();
                if curves.is_empty() || count > 254 {
                    return Err(crate::error(
                        "Curve extrusion requires nonempty loops and at most 254 curve references",
                    ));
                }
                curves
                    .into_iter()
                    .map(|curve| {
                        if matches!(curve, V::Array(_)) {
                            return Err(crate::error("Each loop item must be one curve"));
                        }
                        self.geometry(curve).map(J::String)
                    })
                    .collect::<R<Vec<_>>>()
                    .map(J::Array)
            })
            .collect::<R<_>>()?;
        Ok(J::Array(lowered))
    }

    fn finish_piped_or_standalone_node(
        &mut self,
        name: &str,
        input: Option<String>,
        mut node: J,
    ) -> R<V> {
        let modifier = MODIFIER_OPS.contains(&name);
        if modifier && input.is_none() {
            return Err(crate::error(format!("{name} requires piped geometry")));
        }
        if !modifier && input.is_some() {
            return Err(crate::error(format!(
                "{name} does not accept piped geometry"
            )));
        }
        if let Some(i) = input {
            let collection = self.collections.contains(&i);
            if collection && !["translate", "rotate", "scale", "mirror"].contains(&name) {
                return Err(crate::error(
                    "Transform each generated part inside the generator, or explicitly union the collection first",
                ));
            }
            node["input"] = json!(i);
            let result = self.add(node)?;
            if collection {
                let id = self.geometry(result.clone())?;
                self.collections.insert(id);
            }
            return Ok(result);
        }
        self.add(node)
    }
}

fn normalize_shorthand_args(name: &str, args: &mut Vec<(Option<String>, V)>) -> R<()> {
    if ["box", "translate"].contains(&name)
        && args.len() == 3
        && args.iter().all(|(arg_name, _)| arg_name.is_none())
    {
        let values = std::mem::take(args)
            .into_iter()
            .map(|(_, value)| scalar(value).map(V::Json))
            .collect::<R<Vec<_>>>()?;
        *args = vec![(
            Some(if name == "box" { "size" } else { "vector" }.into()),
            V::Array(values),
        )];
    }
    if ["translate", "rotate", "scale"].contains(&name)
        && args
            .iter()
            .any(|(n, _)| matches!(n.as_deref(), Some("x" | "y" | "z")))
    {
        let mut names = Set::new();
        if args.iter().any(|(n, _)| {
            !matches!(n.as_deref(), Some("x" | "y" | "z")) || !names.insert(n.clone())
        }) {
            return Err(crate::error("Use unique x/y/z arguments or one vector"));
        }
        let vec = ["x", "y", "z"]
            .iter()
            .map(|axis| {
                args.iter()
                    .find(|(n, _)| n.as_deref() == Some(axis))
                    .map_or(
                        V::Json(json!(if name == "scale" { 1 } else { 0 })),
                        |(_, v)| v.clone(),
                    )
            })
            .collect();
        *args = vec![(Some("vector".into()), V::Array(vec))];
    }
    Ok(())
}
