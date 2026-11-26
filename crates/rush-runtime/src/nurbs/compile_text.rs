use super::*;

pub fn compile_text(nodes: Vec<J>, parameters: &[J], mut root: String) -> Result<J> {
    let params = parameters
        .iter()
        .map(|p| {
            let value = p["value"].as_f64().unwrap_or(f64::NAN);
            let numeric = if let Some(unit) = p["unit"].as_str() {
                units::quantity(value, unit, s(p, "id"))
                    .map_err(|error| text_error(s(p, "id"), &error.message))?
            } else {
                Numeric {
                    value,
                    dimension: SCALAR,
                }
            };
            Ok((s(p, "id").to_owned(), numeric))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    let nodes = if nodes.iter().any(|n| s(n, "op") == "if") {
        let by_id: BTreeMap<String, J> = nodes
            .into_iter()
            .map(|n| (s(&n, "id").to_owned(), n))
            .collect();
        let mut selected = Vec::new();
        let mut found = BTreeSet::new();
        fn select(
            id: &str,
            depth: usize,
            by_id: &BTreeMap<String, J>,
            params: &BTreeMap<String, Numeric>,
            found: &mut BTreeSet<String>,
            selected: &mut Vec<J>,
        ) -> Result<String> {
            if depth > 64 {
                return Err(text_error(id, "Conditional geometry depth exceeds 64"));
            }
            let node = by_id
                .get(id)
                .ok_or_else(|| text_error(id, "Unknown geometry node"))?;
            if s(node, "op") == "if" {
                let condition = units::scalar(scalar(&node["condition"], params, id, 0)?, id)
                    .map_err(|error| text_error(id, &error.message))?;
                return select(
                    s(node, if condition != 0.0 { "then" } else { "else" }),
                    depth + 1,
                    by_id,
                    params,
                    found,
                    selected,
                );
            }
            if !found.insert(id.into()) {
                return Ok(id.into());
            }
            let slot = selected.len();
            selected.push(node.clone());
            let mut n = node.clone();
            if let Some(input) = node["input"].as_str() {
                n["input"] = json!(select(input, depth + 1, by_id, params, found, selected)?)
            }
            if let Some(target) = node["target_geometry"].as_str() {
                n["target_geometry"] = json!(select(target, depth+1, by_id, params, found, selected)?);
            }
            if s(node,"op")=="ribbon_surface" {n["width_law"]=json!(select(s(node,"width_law"),depth+1,by_id,params,found,selected)?);}
            if s(node,"op")=="variable_pipe_surface" {
                n["radius_law"]=json!(select(s(node,"radius_law"),depth+1,by_id,params,found,selected)?);
            }
            if ["progressive_sweep","brep_progressive_sweep","brep_progressive_miter_sweep"].contains(&s(node,"op")) {
                if let Some(id)=node["orientation_guide"].as_str(){n["orientation_guide"]=json!(select(id,depth+1,by_id,params,found,selected)?);}
            }
            for reference_key in ["inputs","u_curves","v_curves"] {
            if let Some(inputs) = node[reference_key].as_array() {
                n[reference_key] = J::Array(
                    inputs
                        .iter()
                        .map(|i| {
                            select(
                                i.as_str().unwrap_or(""),
                                depth + 1,
                                by_id,
                                params,
                                found,
                                selected,
                            )
                            .map(J::String)
                        })
                        .collect::<Result<_>>()?,
                )
            }
            }
            if ["brep_extrude_curves","brep_progressive_sweep","brep_miter_sweep","brep_progressive_miter_sweep"].contains(&s(node,"op")) || (node.get("loops").is_some() && matches!(s(node,"op"),"bound_wire_moments" | "cap_section_wire" | "expanded_section_sheets"|"section_brep_sheets" | "bounded_section_brep_sheets" | "cap_section_holes" | "arrange_section_caps" | "exact_wire_mass" | "exact_wire_inertia" | "bound_world_face_mass" | "bound_principal_face_moments" | "bound_principal_face_directions" | "world_face_mass" | "principal_face_moments" | "principal_face_directions" | "coplanar_ray_contacts" | "coplanar_ray_partition" | "store_section_edge" | "optimize_section_edge" | "store_section_edges" | "build_material_segments" | "material_segment_curve" | "classify_section_gaps" | "assemble_section_material" | "measure_section_material" | "partition_face_events" | "enclose_face_events" | "match_quadratic_contacts" | "shared_boundary_spans" | "shared_boundary_contacts" | "refine_boundary_section" | "resolve_boundary_section" | "bound_boundary_section" | "classify_face_section" | "clip_plane_section" | "build_plane_section_segments" | "cast_trimmed_face_ray" | "planar_face_perimeter" | "planar_boundary_distance" | "planar_material_distance" | "signed_planar_face_area" | "project_planar_wire" | "lift_planar_wire" | "relayout_trim_surface" | "relayout_trim_piece" | "affine_trim_surface" | "transpose_trim_surface" | "remap_trim_surface" | "reclock_trim_surface" | "affine_trim_piece" | "transpose_trim_piece" | "remap_trim_piece" | "reclock_trim_piece" | "uv_loop_closure" | "uv_loop_simplicity" | "uv_trim_location" | "trimmed_surface_area_in_window" | "trimmed_surface_centroid" | "trimmed_surface_area_inertia")) {
                n["loops"] = J::Array(
                    node["loops"]
                        .as_array()
                        .ok_or_else(|| {
                            text_error(id, "Curve extrusion requires nested curve lists")
                        })?
                        .iter()
                        .map(|wire| {
                            wire.as_array()
                                .ok_or_else(|| {
                                    text_error(id, "Curve extrusion requires nested curve lists")
                                })?
                                .iter()
                                .map(|curve| {
                                    select(
                                        curve.as_str().unwrap_or(""),
                                        depth + 1,
                                        by_id,
                                        params,
                                        found,
                                        selected,
                                    )
                                    .map(J::String)
                                })
                                .collect::<Result<Vec<_>>>()
                                .map(J::Array)
                        })
                        .collect::<Result<_>>()?,
                );
            }
            selected[slot] = n;
            Ok(id.into())
        }
        root = select(&root, 0, &by_id, &params, &mut found, &mut selected)?;
        selected
    } else {
        nodes
    };
    pub(super) fn field(
        v: &J,
        dimension: [i8; 2],
        params: &BTreeMap<String, Numeric>,
        path: &str,
    ) -> Result<J> {
        if let Some(a) = v.as_array() {
            return a
                .iter()
                .enumerate()
                .map(|(i, v)| field(v, dimension, params, &format!("{path}/{i}")))
                .collect::<Result<Vec<_>>>()
                .map(J::Array);
        }
        Ok(json!(
            units::field(scalar(v, params, path, 0)?, dimension, path, false)
                .map_err(|error| text_error(path, &error.message))?
        ))
    }
    let mut lowered = Vec::new();
    for mut node in nodes {
        if s(&node, "op") == "nurbs_surface" {
            node["op"] = json!("surface")
        }
        if s(&node, "op") == "nurbs_curve" {
            node["op"] = json!("curve")
        }
        let supported = [
            "sphere_surface", "cylinder_surface", "cone_surface",
            "plane_patch", "bilinear_patch", "bezier_surface",
            "bezier_curve", "curve_compose", "round_polyline_curve", "transition_polyline_curve", "brep_miter_sweep","brep_progressive_miter_sweep",
            "rounded_rectangle_profile", "capsule_profile", "annular_sector_surface", "keyway_profile", "t_slot_profile", "dovetail_profile", "poly_v_belt_profile", "toothed_belt_profile", "o_ring_groove_profile", "retaining_ring_profile", "hex_fastener_profile", "square_fastener_profile",
            "exact_ellipse", "exact_ellipsoid", "exact_ring_torus", "exact_frustum_side", "exact_planar_region",
            "line_curve", "polyline_curve", "circle_curve", "circle_arc",
            "hyperboloid_one_sheet", "hyperboloid_two_sheet", "polynomial_graph",
            "polynomial_curve", "polynomial_surface", "rational_polynomial_curve", "rational_polynomial_surface",
            "parabola_curve",
            "hyperbola_curve",
            "elliptic_cylinder_surface",
            "cone_frustum_surface",
            "quadratic_patch",

            "ellipse_arc", "ellipsoid_surface", "torus_surface",
            "polygon_profile",
            "polygon_extrude",
            "polygon_sweep",
            "polygon_loft",
            "ribbon_surface", "variable_pipe_surface", "pipe_surface", "screw_surface", "clothoid_curve", "spherical_spiral_curve", "toroidal_spiral_curve", "torus_knot_curve", "helicoid_patches", "circle_rectangle_transition", "ellipse_transition_surface", "circle_transition_surface", "helicoid_surface", "catenoid_patches", "curve_from_curvature_samples", "fit_cst_sides", "naca_five_digit_profile", "cst_spline_profile", "node_spatial_path","triangulate_planar_path","measure_path_second_moments","bound_path_principal_moments","measure_region_mass","clip_path_to_perforated_region","measure_region_second_moments","measure_planar_path_area","measure_planar_path_centroid","locate_planar_path_points","clip_path_to_planar_region","project_to_planar_region","simplify_exact_path","project_point_to_path","intersect_spatial_paths","find_path_self_intersections","measure_spatial_path_distance","simplify_path_with_tolerance","cst_section_integrals", "fit_sphere_cloud", "fit_cylinder_cloud", "fit_cone_cloud", "fit_torus_cloud", "fit_plane_cloud", "superellipsoid_patches", "corrugated_bellows", "surface_highlight_lines", "surface_reflection_lines", "surface_isophote_analysis", "surface_dent_detection", "fit_biarc_piece", "simplify_linear_curve", "extend_terminal_curve", "extend_surface_boundary", "affine_control_support", "surface_trace_agreement", "signed_shell_volume", "geometry_bounds_union", "geometry_conditioning", "coordinate_frame_local", "coordinate_frame_world", "shared_patch_mesh", "bounded_curve_chord", "bounded_surface_mesh", "store_plane_branch", "split_cell_by_plane", "pair_plane_branch", "close_plane_strip", "split_periodic_plane_strip", "store_closed_plane_piece", "stitch_plane_strip", "trace_plane_strip", "sample_plane_branch", "plane_branch_uv_polyline", "plane_branch_xyz_polyline", "audit_plane_side", "refine_plane_side", "audit_plane_graphs", "parallel_plane_sections", "parallel_section_piece", "clean_offset_boundary", "planar_toolpath_edge", "embed_xy_curve", "planar_offset_piece", "planar_knot_miter", "phase_aligned_section", "compatible_section", "oriented_section", "uv_loop_closure", "uv_loop_simplicity", "uv_trim_location", "curve_edit_deviation", "surface_edit_deviation", "surface_seam_g0", "surface_seam_g1", "surface_seam_g2", "curve_join_continuity", "curve_self_contacts", "surface_self_contacts", "bounded_surface_partial_field", "surface_partial_field", "bounded_surface_area_normal_field", "surface_area_normal_field", "bounded_surface_gaussian_field", "bounded_surface_mean_squared_field", "bounded_surface_principal_gap_squared_field", "bounded_surface_metric_field", "surface_gaussian_field", "surface_mean_squared_field", "surface_principal_gap_squared_field", "surface_metric_field", "bounded_surface_mixed_partial_field", "surface_mixed_partial_field", "bounded_curve_derivative_field", "curve_derivative_field", "bounded_curve_torsion_field", "bounded_curve_speed_squared_field", "curve_torsion_field", "curve_speed_squared_field", "bounded_curve_binormal_field", "curve_binormal_field", "bounded_curve_curvature_squared_field", "curve_curvature_squared_field", "projective_curve_exact", "projective_surface_exact", "surface_cell_interpolation_error", "surface_triangle_interpolation_error", "surface_grid_interpolation_error", "surface_umbilic_cells", "surface_umbilic_regions", "surface_gap_comparison", "surface_gap_ranges", "surface_gap_regions", "surface_gap_edge_roots", "surface_gap_contour_graph", "surface_gap_contour_samples", "surface_gap_world_polyline", "surface_gap_world_length", "merge_patch_strip", "merge_patch_grid", "inspect_patch_grid", "split_tensor_surface", "tile_tensor_surface", "mobius_bezier_curve", "mobius_bezier_surface", "mobius_spline_curve", "mobius_spline_surface", "fit_mobius_curve", "fit_mobius_surface", "affine_clock_curve", "piecewise_clock_curve", "relayout_clock_curve", "affine_clock_surface", "piecewise_clock_surface", "relayout_clock_surface", "pullback_uv_clock", "affine_trim_surface", "transpose_trim_surface", "remap_trim_surface", "reclock_trim_surface", "affine_trim_piece", "transpose_trim_piece", "remap_trim_piece", "reclock_trim_piece", "certify_uv_chart", "classify_uv_seam", "isolate_uv_line", "isolate_uv_seam", "split_uv_seam", "lift_uv_edge", "transfer_uv_grid_piece", "relayout_trim_surface", "relayout_trim_piece", "verify_uv_edge", "compare_seam_orientations", "compare_uv_traces", "compare_uv_tangents", "check_seam_gap", "check_seam_angle", "isolate_seam_gap", "isolate_seam_angle", "active_surface_hull", "project_plane_chart", "store_section_edge", "optimize_section_edge", "store_section_edges", "build_material_segments", "material_segment_curve", "classify_section_gaps", "assemble_section_material", "measure_section_material", "partition_face_events", "enclose_face_events", "match_quadratic_contacts", "shared_boundary_spans", "shared_boundary_contacts", "refine_boundary_section", "resolve_boundary_section", "bound_boundary_section", "classify_face_section", "clip_plane_section", "build_plane_section_segments", "intersect_support_planes", "lift_plane_chart", "exact_wire_area", "bound_wire_moments", "cap_section_wire", "attach_expanded_rims","attach_section_rims", "discover_expanded_rims","discover_section_rims", "attach_bounded_rims", "build_expanded_plane_solid","build_outward_plane_solid","build_section_solid","infer_rim_planes","close_inferred_rims","close_bounded_inferred_rims","inspect_inferred_solid","build_outward_inferred_solid","build_expanded_solid","build_inferred_solid","close_expanded_planes","project_polyhedral_body","measure_polyhedral_frame","measure_polyhedral_axis_gap","section_polyhedral_faces","assemble_polyhedral_section_loops","measure_polyhedral_section_area","measure_polyhedral_section_centroid","bound_polyhedral_section_perimeter","clip_polyhedral_segment","locate_polyhedral_point","measure_polyhedral_boundary_distance","measure_polyhedral_signed_distance","sample_polyhedral_signed_distances","measure_swept_ball_gap","bound_path_material_length","clip_exterior_path","clip_interior_path","partition_solid_path","bound_path_length_breakdown","find_path_face_contacts","clip_common_path","clip_union_path","subtract_path_cutters","classify_path_membership","bound_overlap_lengths","recover_overlap_paths","recover_material_path","clip_path_to_material","find_path_boundary_events","measure_swept_path_gap","classify_swept_path_material","bound_swept_first_contact","bound_swept_face_ranges","measure_polyhedral_ball_clearance","locate_section_point","measure_section_contour_distance","measure_section_material_distance","clip_section_segment","clip_section_polyline","measure_section_path_length","classify_polyhedral_plane","intersect_polyhedral_edges","measure_polyhedral_volume","measure_polyhedral_centroid","measure_polyhedral_inertia","measure_polyhedral_face_areas","bound_polyhedral_area","bound_polyhedral_edge_length","close_section_planes","build_bounded_section_solid","close_bounded_planes","discover_bounded_rims", "expanded_section_sheets","section_brep_sheets", "bounded_section_brep_sheets", "cap_section_holes", "arrange_section_caps", "exact_wire_mass", "exact_wire_inertia", "bound_world_face_mass", "bound_principal_face_moments", "bound_principal_face_directions", "world_face_mass", "principal_face_moments", "principal_face_directions", "coplanar_ray_contacts", "coplanar_ray_partition", "cast_trimmed_face_ray", "planar_face_perimeter", "planar_boundary_distance", "planar_material_distance", "signed_planar_face_area", "project_planar_wire", "lift_planar_wire", "shared_plane_chart", "compress_tensor_surface", "compress_tensor_surface_batch", "surface_gap_pair_distance", "surface_gap_plane_section", "surface_gap_world_distance", "surface_gap_world_bounds", "surface_gap_uv_polyline", "surface_gap_adaptive_polyline", "surface_gap_second_derivative", "surface_gap_second_order_polyline", "surface_gap_contour_slope", "surface_quadric_support", "surface_span_regularity", "curve_plane_roots", "curve_pair_crossings", "curve_plane_chart_crossings", "surface_ray_hits", "curve_curvature_torsion", "surface_principal_curvatures", "curve_nearest_point", "surface_nearest_point", "merge_curve_exact", "merge_surface_exact", "curve_pair_distance", "curve_surface_distance", "surface_pair_distance", "curve_length_bounds", "curve_station_at_length", "curve_divide_by_length", "surface_area_bounds", "optical_dome", "optical_lens", "cubic_wave_surface", "catenoid_surface", "catenary_curve", "archimedean_spiral_curve", "epicycloid_curve", "hypocycloid_curve", "trochoid_curve", "cycloid_curve", "lissajous_curve", "logarithmic_spiral_curve", "involute_curve", "elliptic_helix_curve", "variable_pitch_helix_curve", "conical_helix_curve", "helix_curve", "surface_sweep", "brep_progressive_sweep", "progressive_sweep", "profile_sweep", "framed_sweep", "scaled_sweep", "two_guide_sweep", "twist_sweep",
            "surface_loft",
            "ruled_surface",
            "coons_patch",
            "guided_loft_surface", "auto_guided_loft_surface", "loft_match_surface", "brep_rational_loft","brep_natural_loft", "brep_capped_loft", "control_tangent_loft_surface", "clamped_loft_surface", "boundary_fill", "triangular_patch", "gordon_surface", "closed_loft_surface", "natural_loft_surface", "grid_spline_surface", "hermite_patch", "closed_spline_curve", "clamped_spline_curve", "natural_spline_curve", "hermite_curve", "formula_curve", "formula_surface",
            "extrude",
            "revolve",
            "triangle_mesh",
            "mesh_to_nurbs_brep",
            "mesh_to_sdf",
            "mesh_to_subdivision",
            "subdivision_tessellate",
            "mesh_to_nurbs",
            "mesh_fit_nurbs",
            "nurbs_patches_tessellate",
            "subdivision",
            "sdf_sphere",
            "sdf_box",
            "sdf_torus",
            "sdf_union",
            "sdf_intersection",
            "sdf_difference",
            "sdf_smooth_union",
            "sdf_offset",
            "sdf_translate",
            "sdf_tessellate",
            "brep_box",
            "brep_sphere",
            "brep_torus",
            "brep_cylinder",
            "brep_frustum",
            "brep_tube",
            "radial_cam_profile",
            "radial_cam_walls",
            "rack_profile",
            "brep_rack",
            "finite_za_worm",
            "finite_za_wheel",
            "finite_external_thread",
            "finite_internal_thread",
            "verified_bevel_gear",
            "brep_gear",
            "brep_extrude",
            "brep_extrude_curves",
            "brep_revolve",
            "brep_boolean",
            "brep_chamfer",
            "brep_fillet",
            "brep_full_round_exact",
            "brep_tessellate",
            "brep_smooth_miter_stations", "brep_smooth_sweep_stations",
            "surface",
            "curve",
            "curve_edit", "curve_edit_weights", "curve_relocate_seam", "surface_edit_weights", "surface_relocate_seam", "curve_exact_insert_knot", "curve_exact_remove_knot", "curve_exact_reduce_degree", "surface_exact_insert_knot", "surface_exact_remove_knot", "surface_exact_reduce_degree", "curve_normalize_weights", "curve_cancel_factor", "surface_cancel_factor", "curve_exact_split", "curve_exact_restrict", "curve_exact_partition", "curve_exact_elevate", "assemble_exact_periodic_surface_axis", "assemble_periodic_surface_axis_with_exact_weight_scaling", "assemble_periodic_surface_axis_with_exact_knot_union", "assemble_periodic_surface_axis_with_exact_basis_matching", "assemble_periodic_surface_axis_with_exact_degree_matching", "assemble_exact_doubly_periodic_surface", "assemble_doubly_periodic_surface_with_exact_weight_scaling", "rotate_periodic_surface_at_exact_knot", "insert_and_rotate_periodic_surface_seam", "open_exact_periodic_surface_axis", "restrict_exact_periodic_surface_axis", "restrict_exact_periodic_surface_over_turns", "restrict_exact_doubly_periodic_surface", "restrict_exact_doubly_periodic_surface_over_turns", "partition_exact_periodic_surface_axis", "partition_exact_periodic_surface_over_turns", "partition_exact_doubly_periodic_surface", "partition_exact_doubly_periodic_surface_over_turns", "remove_exact_periodic_surface_knot", "simplify_exact_periodic_surface_axis", "remove_exact_periodic_surface_seam_knot", "simplify_exact_doubly_periodic_surface", "reduce_exact_periodic_surface_period", "repeat_exact_periodic_surface_period", "curve_axis_motion_partition_speed", "curve_axis_motion_partition_motion", "curve_axis_motion_partition_axes", "curve_axis_motion_limits", "curve_axis_motion_slowdown", "curve_axis_motion_multi_slowdown", "curve_axis_motion_duration", "curve_axis_motion_deadline", "curve_directional_travel_full", "curve_directional_travel_signed", "curve_check_backtracking", "surface_trim_contour_audit", "surface_trim_contour_station", "surface_trim_contour_station_box", "surface_trim_contour_divide", "surface_trim_patch_area_vector", "surface_trim_patch_projected_area", "surface_trim_range_length", "evaluate_exact_wrapped_surface_point", "evaluate_exact_wrapped_surface_points", "lift_exact_periodic_surface_path", "exact_periodic_surface_path_winding", "exact_periodic_surface_path_signed_area", "exact_periodic_surface_path_centroid", "exact_periodic_surface_path_inertia", "classify_exact_periodic_surface_path_point", "classify_exact_periodic_surface_path_points", "classify_exact_periodic_surface_region_points", "certify_exact_periodic_surface_path_simplicity", "check_exact_periodic_surface_path_contact", "relate_exact_periodic_surface_paths", "exact_periodic_surface_contour_forest", "exact_periodic_surface_region_area", "exact_periodic_surface_region_component_centroids", "exact_periodic_surface_region_component_inertia", "measure_input_contour_length", "measure_input_segment_length", "measure_input_segment_range_length", "locate_input_segment_at_length", "measure_input_contour_prefix_lengths", "locate_input_contour_at_lengths", "divide_input_contour_by_length", "exact_input_contour_projected_area", "exact_input_contour_polynomial_projected_area", "bound_input_contour_projected_area", "bound_input_patch_max_projected_area", "bound_input_patch_max_projection_direction", "surface_trim_sheet_display", "surface_trim_sheet_step", "surface_trim_sheet", "surface_trim_sheet_edge_bindings", "surface_trim_sheet_input_bindings", "locate_input_parameter", "locate_edge_parameter", "locate_input_range", "extract_input_range_curves", "extract_input_range_curve_pairs", "evaluate_input_point", "evaluate_input_tangents", "classify_input_tangent_join", "classify_input_vertex_join", "lift_exact_periodic_surface_region_components", "build_exact_periodic_surface_trim_components", "build_exact_periodic_surface_trim_patches", "build_exact_periodic_surface_trim_edges", "trimmed_surface_area_in_window", "trimmed_surface_centroid", "trimmed_surface_area_inertia", "surface_trim_area", "surface_trim_mass_properties", "surface_trim_window_properties", "surface_trim_set_properties", "surface_measure_trimmed_mass", "curve_axial_derivative_peak", "curve_measure_directional_travel", "curve_measure_directional_travel_batch", "curve_reparameterize", "surface_edit", "surface_split", "surface_transpose", "surface_bezier_patch", "curve_remove_knot", "curve_reduce_degree", "curve_rebuild", "surface_remove_knot", "surface_reduce_degree", "surface_rebuild", "iso_curve", "curve_split", "curve_bezier_span",
            "surface_extrude_patches", "surface_extrude",
            "surface_revolve",
            "tessellate",
            "thicken",
            "mesh_boolean",
            "transform",
        ];
        if !supported.contains(&s(&node, "op")) {
            return Err(text_error(
                s(&node, "id"),
                &format!(
                    "Operation {} cannot be mixed with own NURBS/mesh operations",
                    s(&node, "op")
                ),
            ));
        }
        let progressive_sweep=["progressive_sweep","brep_progressive_sweep","brep_progressive_miter_sweep"].contains(&s(&node,"op"));
        let needs_direction = matches!(s(&node, "op"), "store_plane_branch" | "split_cell_by_plane" | "pair_plane_branch" | "close_plane_strip" | "split_periodic_plane_strip" | "store_closed_plane_piece" | "stitch_plane_strip" | "trace_plane_strip" | "sample_plane_branch" | "plane_branch_uv_polyline" | "plane_branch_xyz_polyline" | "classify_uv_seam" | "isolate_uv_seam" | "split_uv_seam" | "split_tensor_surface" | "merge_patch_strip" | "surface_gap_contour_graph" | "surface_gap_contour_samples" | "surface_gap_world_polyline" | "surface_gap_world_length" | "surface_gap_pair_distance" | "surface_gap_plane_section" | "surface_gap_world_distance" | "surface_gap_world_bounds" | "surface_gap_uv_polyline" | "surface_gap_adaptive_polyline" | "surface_gap_second_derivative" | "surface_gap_second_order_polyline" | "surface_gap_contour_slope" | "bounded_surface_partial_field" | "surface_partial_field" | "merge_surface_exact" | "iso_curve" | "surface_exact_insert_knot" | "surface_exact_remove_knot" | "surface_exact_reduce_degree" | "surface_cancel_factor" | "rotate_periodic_surface_at_exact_knot" | "insert_and_rotate_periodic_surface_seam" | "open_exact_periodic_surface_axis" | "restrict_exact_periodic_surface_axis" | "restrict_exact_periodic_surface_over_turns" | "partition_exact_periodic_surface_axis" | "partition_exact_periodic_surface_over_turns" | "remove_exact_periodic_surface_knot" | "simplify_exact_periodic_surface_axis" | "remove_exact_periodic_surface_seam_knot" | "reduce_exact_periodic_surface_period" | "repeat_exact_periodic_surface_period" | "assemble_exact_periodic_surface_axis" | "assemble_periodic_surface_axis_with_exact_weight_scaling" | "assemble_periodic_surface_axis_with_exact_knot_union" | "assemble_periodic_surface_axis_with_exact_basis_matching" | "assemble_periodic_surface_axis_with_exact_degree_matching" | "surface_relocate_seam" | "surface_split" | "surface_remove_knot" | "surface_reduce_degree" | "surface_rebuild");
        let engineering_profile = ["rounded_rectangle_profile", "capsule_profile", "annular_sector_surface", "keyway_profile", "t_slot_profile", "dovetail_profile", "poly_v_belt_profile", "toothed_belt_profile", "o_ring_groove_profile", "retaining_ring_profile", "hex_fastener_profile", "square_fastener_profile"].contains(&s(&node, "op"));
        let exact_primitive = ["exact_ellipse", "exact_ellipsoid", "exact_ring_torus", "exact_frustum_side", "exact_planar_region"].contains(&s(&node,"op"));
        let exact_ellipsoid = s(&node,"op") == "exact_ellipsoid";
        let exact_frustum = s(&node,"op") == "exact_frustum_side";
        let line_coordinates=s(&node,"op")=="line_curve";
        let scaled_sweep=["scaled_sweep","profile_sweep","progressive_sweep","brep_progressive_sweep","brep_progressive_miter_sweep"].contains(&s(&node,"op"));
        let catenary=s(&node,"op")=="catenary_curve";
        let circle_rectangle=s(&node,"op")=="circle_rectangle_transition";
        let ellipse_transition=s(&node,"op")=="ellipse_transition_surface";
        let circle_transition=s(&node,"op")=="circle_transition_surface";
        let superellipsoid = s(&node, "op") == "superellipsoid_patches";
        let optical = matches!(s(&node, "op"), "optical_dome" | "optical_lens");
        let periodic_profile = matches!(s(&node, "op"), "cubic_wave_surface" | "corrugated_bellows");
        let catenoid=["catenoid_surface","catenoid_patches"].contains(&s(&node,"op"));
        let clothoid=s(&node,"op")=="clothoid_curve";
        let pipe=["profile_sweep","framed_sweep","pipe_surface","variable_pipe_surface","ribbon_surface"].contains(&s(&node,"op"));
        let mechanical_profile=["finite_za_wheel","finite_za_worm","brep_rack","rack_profile","verified_bevel_gear","radial_cam_profile","radial_cam_walls","finite_external_thread","finite_internal_thread"].contains(&s(&node,"op"));
        let loft_refs=["loft_match_surface","brep_rational_loft","brep_natural_loft","brep_capped_loft"].contains(&s(&node,"op"));
        for (key, value) in node.as_object_mut().unwrap() {
            if exact_primitive && key == "axes" && exact_frustum {
                for (i, axis) in value.as_array_mut().ok_or_else(|| text_error(key,"Expected three axes"))?.iter_mut().enumerate() {
                    *axis = field(axis, if i == 2 { LENGTH } else { SCALAR }, &params, key)?;
                }
                continue;
            }
            if needs_direction && matches!(key.as_str(), "direction" | "target_direction") {
                let axis = value.as_str().or_else(|| value.get("text").and_then(J::as_str));
                if !matches!(axis, Some("u" | "v")) {
                    return Err(text_error(key, "Expected parameter direction u or v"));
                }
                *value = json!(axis);
                continue;
            }
            if ["reverse", "reverse_u", "reverse_v"].contains(&key.as_str()) {
                continue;
            }
            if ["insert_knots", "insert_knots_u", "insert_knots_v"].contains(&key.as_str()) {
                for knot in value.as_array_mut().ok_or_else(|| text_error(key, "Expected knot list"))? {
                    let entry = knot.as_object_mut().ok_or_else(|| text_error(key, "Expected knot record"))?;
                    for field_name in ["value", "count"] {
                        let item = entry.get_mut(field_name).ok_or_else(|| text_error(key, "Expected knot value and count"))?;
                        *item = field(item, SCALAR, &params, key)?;
                    }
                }
                continue;
            }
            if loft_refs && ["sections","guides","start_reference","end_reference","start","end","sides","start_boundary","end_boundary","start_reverse","end_reverse","cap_surfaces","cap_trims"].contains(&key.as_str()){continue;}
            if ["id", "op", "orientation", "spacing", "continuity_class", "source_edge", "target_edge", "source_end", "target_end", "query_side", "u_side", "v_side", "target_geometry", "width_law", "radius_law", "orientation_guide", "input", "inputs", "u_curves", "v_curves", "guides", "operation", "loops", "laws", "even_odd", "left_handed", "verified_profile", "return_field", "inspect_spans", "herringbone", "internal", "positive_side", "use_pcurve", "edge_id", "exact_parameter", "range_start", "range_end", "principal_axes", "static_unbalance", "couple_unbalance", "radius_of_gyration", "lower", "closed", "periodic", "periodic_u", "periodic_v", "construction", "cap_correction_authored_frame"].contains(&key.as_str()) {
                continue;
            }
            if matches!(key.as_str(), "query_point_mm" | "query_path_mm" | "holes_mm" | "other_path_mm" | "deviation_mm" | "query_radius_mm") {
                fn coordinates(value: &J, params: &BTreeMap<String, Numeric>, path: &str) -> Result<J> {
                    if value.as_str().is_some() { return Ok(value.clone()); }
                    if let Some(items) = value.as_array() {
                        return items.iter().enumerate().map(|(i,item)|coordinates(item,params,&format!("{path}/{i}")))
                            .collect::<Result<Vec<_>>>().map(J::Array);
                    }
                    field(value,LENGTH,params,path)
                }
                *value = coordinates(value,&params,key)?;
                continue;
            }
            if key=="section_mappings" {
                fn maps(value:&J,params:&BTreeMap<String,Numeric>,path:&str)->Result<J> {
                    if value.is_null() {return Ok(J::Null);}
                    if let Some(items)=value.as_array() {return Ok(J::Array(items.iter().enumerate().map(|(i,v)|maps(v,params,&format!("{path}/{i}"))).collect::<Result<_>>()?));}
                    if let Some(object)=value.as_object() {
                        if object.contains_key("pieces") || object.contains_key("composition") || object.contains_key("controlValues") {
                            return Ok(J::Object(object.iter().map(|(k,v)|Ok((k.clone(),maps(v,params,&format!("{path}/{k}"))?))).collect::<Result<_>>()?));
                        }
                    }
                    field(value,SCALAR,params,path)
                }
                *value=maps(value,&params,key)?;
                continue;
            }
            if key=="embedding_limits" {
                for (name,limit) in value.as_object_mut().ok_or_else(||text_error(key,"Expected embedding limits"))? {
                    *limit=field(limit,SCALAR,&params,&format!("{key}/{name}"))?;
                }
                continue;
            }
            if key == "edges" && value.is_object() {
                // B-rep edge query: names and enum text stay; lengths are mm, the rest scalar.
                fn query(value: &J, params: &BTreeMap<String, Numeric>, path: &str) -> Result<J> {
                    let object = value
                        .as_object()
                        .ok_or_else(|| text_error(path, "Expected an edge or face query"))?;
                    let mut out = value_codec::Map::new();
                    for (name, v) in object {
                        let at = format!("{path}/{name}");
                        let lowered = match name.as_str() {
                            "face" => query(v, params, &at)?,
                            "convexity" | "type" => v
                                .as_str()
                                .or_else(|| v.get("text").and_then(J::as_str))
                                .map(|t| J::String(t.to_owned()))
                                .ok_or_else(|| text_error(&at, "Expected text"))?,
                            "min_length" | "max_length" => field(v, LENGTH, params, &at)?,
                            _ => field(v, SCALAR, params, &at)?,
                        };
                        out.insert(name.clone(), lowered);
                    }
                    Ok(J::Object(out))
                }
                *value = query(value, &params, key)?;
                continue;
            }
            if key == "expressions" {
                let groups=value.as_array().ok_or_else(||text_error(key,"Expected three formula token lists"))?;
                *value=J::Array(groups.iter().enumerate().map(|(axis,group)| {
                    if let Some(text)=group.as_str().or_else(||group.get("text").and_then(J::as_str)){return Ok(J::String(text.to_owned()));}
                    let tokens=group.as_array().ok_or_else(||text_error(key,"Expected formula text or a token list"))?;
                    tokens.iter().enumerate().map(|(i,token)| {
                        if let Some(text)=token.as_str().or_else(||token.get("text").and_then(J::as_str)) {Ok(J::String(text.to_owned()))}
                        else {field(token,SCALAR,&params,&format!("{key}/{axis}/{i}"))}
                    }).collect::<Result<Vec<_>>>().map(J::Array)
                }).collect::<Result<Vec<_>>>()?);
            } else if key == "matrix" {
                let rows = value
                    .as_array()
                    .ok_or_else(|| text_error(key, "Expected a matrix"))?;
                let mut matrix = Vec::new();
                for (i, row) in rows.iter().enumerate() {
                    let row = row
                        .as_array()
                        .ok_or_else(|| text_error(key, "Expected a matrix"))?;
                    matrix.push(J::Array(
                        row.iter()
                            .enumerate()
                            .map(|(j, v)| {
                                field(
                                    v,
                                    if i < 3 && j == 3 { LENGTH } else { SCALAR },
                                    &params,
                                    &format!("{key}/{i}/{j}"),
                                )
                            })
                            .collect::<Result<_>>()?,
                    ))
                }
                *value = J::Array(matrix)
            } else if progressive_sweep && ["axis_scale","center_law","frame_axis","frame_normal"].contains(&key.as_str()) {
                let law=value.as_object_mut().ok_or_else(||text_error(key,"Sweep vector law must be an object"))?;
                for (component,entry) in law {
                    *entry=field(entry,if key=="center_law" && component=="values" {LENGTH} else {SCALAR},&params,&format!("{key}/{component}"))?;
                }
            } else if key == "twist" && progressive_sweep {
                let law=value.as_object_mut().ok_or_else(||text_error(key,"Sweep twist must be an object"))?;
                for (component,entry) in law {
                    *entry=field(entry,if component=="values" {ANGLE} else {SCALAR},&params,&format!("twist/{component}"))?;
                }
            } else if key == "scale" && scaled_sweep {
                let law=value.as_object_mut().ok_or_else(||text_error(key,"Sweep scale must be an object"))?;
                for (component,entry) in law {
                    *entry=field(entry,SCALAR,&params,&format!("scale/{component}"))?;
                }
            } else {
                let dimension = if key == "tolerance_mm3" { [3,0] } else if matches!(key.as_str(),"residual_per_mm2" | "gap_level_per_mm2" | "target_gap_level_per_mm2") { [-2,0] } else if matches!(key.as_str(),"curvature_tolerance_per_mm" | "curvatures_per_mm") { [-1,0] } else if key == "tolerance_mm2" { [2,0] } else if superellipsoid { match key.as_str() { "exponents" | "max_cells" => SCALAR, _ => LENGTH } } else if optical || periodic_profile && key != "periods" { LENGTH } else if mechanical_profile && ["y_breaks","stock_radius","axial_shift","diameter","pitch","addendum","dedendum","backing_height","face_width","base_radius","roller_radius","end_lifts"].contains(&key.as_str()) { LENGTH } else if mechanical_profile && ["pitch_cone_angle","end_angles","phase"].contains(&key.as_str()) { ANGLE } else if exact_primitive { match key.as_str() { "max_work" => SCALAR, "axes" if !exact_ellipsoid => SCALAR, _ => LENGTH } } else if engineering_profile { match key.as_str() { "normal" | "teeth" | "ribs" => SCALAR, "gap_degrees" | "start_degrees" | "sweep_degrees" => ANGLE, _ => LENGTH } } else if progressive_sweep && key=="length_tolerance" {LENGTH} else if circle_rectangle {if ["circle_normal","circle_seam"].contains(&key.as_str()) {SCALAR} else {LENGTH}} else if ellipse_transition {LENGTH} else if circle_transition && ["start_center","end_center","start_radius","end_radius"].contains(&key.as_str()) {LENGTH} else if pipe && key=="sections" {SCALAR} else if clothoid && ["start_curvature","end_curvature"].contains(&key.as_str()) {[-1,0]} else if clothoid && key=="length" {LENGTH} else if catenoid && ["scale","start_z","end_z"].contains(&key.as_str()) || catenary && ["scale","start_x","end_x"].contains(&key.as_str()) || line_coordinates && ["start","end"].contains(&key.as_str()) || [
                    "height",
                    "width",
                    "z_min",
                    "z_max",
                    "outer",
                    "holes",
                    "sections",
                    "path",
                    "tangent_u", "tangent_v", "twist", "points", "tangents", "start_tangent", "end_tangent", "start_tangents", "end_tangents",
                    "corners",
                    "upper_points", "lower_points", "chord_vector", "landmarks", "search_tolerance_mm", "shape_tolerance_mm", "extension_limit_mm", "tool_radius_mm", "stock_mm", "stepover_mm", "chord_mm", "stations_mm", "plane_offset_mm", "ray_origin", "distances_mm", "distance_mm", "probe_point", "balance_tolerance", "plane_offsets", "weight_radii", "plane_point", "axis_point", "reference_point", "tolerance_mm", "reverse_limit", "wall_tolerance", "quantum", "max_deviation", "budget", "error_budget", "cap_correction_tolerance", "cap_correction_quantum", "circle_correction_tolerance", "circle_correction_quantum",
                    "vertices",
                    "axis_u", "axis_v", "radii", "radial_radius", "axial_radius",
                    "radius_x", "radius_y", "bounds",
                    "fixed_radius", "rolling_radius", "tracing_radius", "amplitudes", "start_radius", "end_radius", "start_pitch", "end_pitch",
                    "center",
                    "half_size",
                    "radius",
                    "bottom_radius",
                    "top_radius",
                    "inner_radius",
                    "outer_radius",
                    "measured_span_mm",
                    "module",
                    "bore",
                    "rim_width",
                    "clearance",
                    "backlash",
                    "size",
                    "major_radius",
                    "minor_radius",
                    "distance", "setback",
                    "min",
                    "max",
                    "control_points",
                    "vector",
                    "origin",
                ]
                .contains(&key.as_str())
                {
                    LENGTH
                } else if ["angle", "pressure_angle", "helix_angle", "start_degrees", "sweep_degrees", "phase_degrees", "end_degrees", "phases_degrees", "major_phase_degrees", "minor_phase_degrees", "longitude_phase_degrees", "latitude_phase_degrees"].contains(&key.as_str()) {
                    ANGLE
                } else {
                    SCALAR
                };
                *value = field(value, dimension, &params, key)?
            }
        }
        lowered.push(node)
    }
    compile(
        json!({"language":"rush/nurbs-1","units":"mm","parameters":[],"nodes":lowered,"root":root}),
    )
}
