//! Affine transforms, parametric curves/surfaces, splines, formulas, patches, and primitives.
use super::*;

#[path = "trimmed_plane_sections.rs"]
mod trimmed_plane_sections;

#[path = "world_mass_requests.rs"]
mod world_mass_requests;

#[path = "wire_moment_requests.rs"]
mod wire_moment_requests;

#[path = "planar_wire_requests.rs"]
mod planar_wire_requests;

#[path = "planar_chart_requests.rs"]
mod planar_chart_requests;

#[path = "arc_metrology.rs"]
mod arc_metrology;

#[path = "intersection_measurements.rs"]
mod intersection_measurements;

#[path = "surface_seam_measurements.rs"]
mod surface_seam_measurements;

#[path = "uv_loop_queries.rs"]
mod uv_loop_queries;

#[path = "planar_path_selection.rs"]
mod planar_path_selection;

#[path = "plane_branch_queries.rs"]
mod plane_branch_queries;

#[path = "section_cap_queries.rs"]
mod section_cap_queries;

#[path = "plane_side_queries.rs"]
mod plane_side_queries;

#[path = "parallel_section_queries.rs"]
mod parallel_section_queries;

#[path = "bounded_mesh_construction.rs"]
mod bounded_mesh_construction;

#[path = "shared_patch_mesh.rs"]
mod shared_patch_mesh;

#[path = "coordinate_analysis.rs"]
mod coordinate_analysis;

#[path = "optical_surface_analysis.rs"]
mod optical_surface_analysis;

#[path = "cloud_fit_surfaces.rs"]
mod cloud_fit_surfaces;

#[path = "terminal_span_edit.rs"]
mod terminal_span_edit;

#[path = "airfoil_section_requests.rs"]
mod airfoil_section_requests;

#[path = "signed_curvature_fit.rs"]
mod signed_curvature_fit;

#[path = "exact_merge_requests.rs"]
mod exact_merge_requests;

#[path = "exact_field_requests.rs"]
mod exact_field_requests;

#[path = "interpolation_error_requests.rs"]
mod interpolation_error_requests;

#[path = "oriented_volume_request.rs"]
mod oriented_volume_request;

#[path = "umbilic_requests.rs"]
mod umbilic_requests;

#[path = "curvature_contour_requests.rs"]
mod curvature_contour_requests;

#[path = "mobius_clock_requests.rs"]
mod mobius_clock_requests;

#[path = "pcurve_seam_requests.rs"]
mod pcurve_seam_requests;

#[path = "pcurve_agreement_requests.rs"]
mod pcurve_agreement_requests;

pub(super) fn try_dispatch(v: &Value, op: &str) -> Option<Result<Value>> {
    if matches!(op,"store_section_edge" | "optimize_section_edge" | "store_section_edges" | "build_material_segments" | "material_segment_curve" | "classify_section_gaps" | "assemble_section_material" | "measure_section_material" | "partition_face_events" | "enclose_face_events" | "match_quadratic_contacts" | "shared_boundary_spans" | "shared_boundary_contacts" | "refine_boundary_section" | "resolve_boundary_section" | "bound_boundary_section" | "classify_face_section" | "clip_plane_section" | "build_plane_section_segments") {return Some(trimmed_plane_sections::apply(v,op));}
    if op=="intersect_support_planes" {return Some(planar_chart_requests::support_line(v));}
    if matches!(op,"exact_wire_area" | "bound_wire_moments" | "exact_wire_mass" | "exact_wire_inertia") {return Some(wire_moment_requests::apply(v,op));}
    if matches!(op,"bound_world_face_mass" | "bound_principal_face_moments" | "bound_principal_face_directions" | "world_face_mass" | "principal_face_moments" | "principal_face_directions") {return Some(world_mass_requests::apply(v,op));}
    if matches!(op,"signed_planar_face_area" | "coplanar_ray_contacts" | "coplanar_ray_partition" | "cast_trimmed_face_ray" | "planar_face_perimeter" | "planar_boundary_distance" | "planar_material_distance") {return Some(planar_wire_requests::measure_face(v,op));}
    if matches!(op,"project_planar_wire" | "lift_planar_wire") {return Some(planar_wire_requests::apply(v,op));}
    if matches!(op,"active_surface_hull" | "project_plane_chart" | "lift_plane_chart" | "shared_plane_chart") {return Some(planar_chart_requests::apply(v,op));}
    if matches!(op,"bounded_surface_partial_field" | "surface_partial_field" | "bounded_surface_area_normal_field" | "surface_area_normal_field" | "bounded_surface_gaussian_field" | "bounded_surface_mean_squared_field" | "bounded_surface_principal_gap_squared_field" | "bounded_surface_metric_field" | "surface_gaussian_field" | "surface_mean_squared_field" | "surface_principal_gap_squared_field" | "surface_metric_field" | "bounded_surface_mixed_partial_field" | "surface_mixed_partial_field") { return Some(exact_field_requests::surface_field(v,op)); }
    if matches!(op,"bounded_curve_derivative_field" | "curve_derivative_field" | "bounded_curve_torsion_field" | "bounded_curve_speed_squared_field" | "curve_torsion_field" | "curve_speed_squared_field" | "bounded_curve_binormal_field" | "curve_binormal_field" | "bounded_curve_curvature_squared_field" | "curve_curvature_squared_field") { return Some(exact_field_requests::curve_field(v,op)); }
    if matches!(op,"affine_clock_curve" | "piecewise_clock_curve" | "relayout_clock_curve" | "affine_clock_surface" | "piecewise_clock_surface" | "relayout_clock_surface" | "mobius_bezier_curve" | "mobius_spline_curve" | "fit_mobius_curve" | "mobius_bezier_surface" | "mobius_spline_surface" | "fit_mobius_surface") { return Some(mobius_clock_requests::apply(v,op)); }
    if matches!(op,"relayout_trim_surface" | "relayout_trim_piece" | "affine_trim_surface" | "transpose_trim_surface" | "remap_trim_surface" | "reclock_trim_surface" | "affine_trim_piece" | "transpose_trim_piece" | "remap_trim_piece" | "reclock_trim_piece" | "certify_uv_chart") { return Some(mobius_clock_requests::trim(v,op)); }
    if matches!(op,"classify_uv_seam" | "isolate_uv_line" | "isolate_uv_seam" | "split_uv_seam") { return Some(pcurve_seam_requests::apply(v,op)); }
    if matches!(op,"check_seam_gap" | "check_seam_angle" | "isolate_seam_gap" | "isolate_seam_angle" | "verify_uv_edge" | "compare_seam_orientations" | "compare_uv_traces" | "compare_uv_tangents") { return Some(pcurve_agreement_requests::apply(v,op)); }
    if op == "lift_uv_edge" { return Some(mobius_clock_requests::lift(v)); }
    if matches!(op,"transfer_uv_grid_piece" | "pullback_uv_clock") { return Some(mobius_clock_requests::pullback(v,op)); }
    if op == "projective_curve_exact" {
        return Some((|| {
            let report = crate::editing::exact_projective::curve(&field(v,"source")?,field(v,"coefficients")?,field(v,"max_work")?)?;
            let geometry = report.geometry.ok_or_else(|| input(format!("Exact projective transformation refused: {:?}",report.refusal)))?;
            Ok(json!({"curve":geometry,"workUsed":report.work_used,"finiteChart":true}))
        })());
    }
    if op == "projective_surface_exact" {
        return Some((|| {
            let report = crate::editing::exact_projective::surface(&field(v,"source")?,field(v,"coefficients")?,field(v,"max_work")?)?;
            let geometry = report.geometry.ok_or_else(|| input(format!("Exact projective transformation refused: {:?}",report.refusal)))?;
            Ok(json!({"surface":geometry,"workUsed":report.work_used,"finiteChart":true}))
        })());
    }
    if matches!(op,"surface_cell_interpolation_error" | "surface_triangle_interpolation_error" | "surface_grid_interpolation_error") { return Some(interpolation_error_requests::inspect(v,op)); }
    if op == "signed_shell_volume" { return Some(oriented_volume_request::volume(v)); }
    if matches!(op,"surface_umbilic_cells" | "surface_umbilic_regions" | "surface_gap_comparison" | "surface_gap_ranges" | "surface_gap_regions") { return Some(umbilic_requests::inspect(v,op)); }
    if matches!(op,"surface_gap_edge_roots" | "surface_gap_contour_graph" | "surface_gap_contour_samples" | "surface_gap_world_polyline" | "surface_gap_world_length" | "surface_gap_pair_distance" | "surface_gap_plane_section" | "surface_gap_world_distance" | "surface_gap_world_bounds" | "surface_gap_uv_polyline" | "surface_gap_adaptive_polyline" | "surface_gap_second_derivative" | "surface_gap_second_order_polyline" | "surface_gap_contour_slope") { return Some(curvature_contour_requests::inspect(v,op)); }
    if op == "surface_quadric_support" {
        return Some((|| {
            let r = crate::analysis::implicit_quadric::inspect(&field(v,"surface")?,field(v,"quadric")?,field(v,"max_work")?,field(v,"max_cells")?)?;
            Ok(json!({"exactMembership":r.exact_membership,"gradientNonzero":r.gradient_nonzero,
                "regularImplicitSupport":r.regular_implicit_support,"exactWork":r.exact_work,"cells":r.cells,
                "gradientLower":r.gradient_lower,"reason":r.reason}))
        })());
    }
    if matches!(op,"merge_patch_strip" | "merge_patch_grid" | "inspect_patch_grid") { return Some(exact_merge_requests::grid(v,op)); }
    if matches!(op,"split_tensor_surface" | "tile_tensor_surface") { return Some(exact_merge_requests::split(v,op)); }
    if matches!(op,"compress_tensor_surface" | "compress_tensor_surface_batch") { return Some(exact_merge_requests::compress(v,op)); }
    if matches!(op,"join_affine_c1" | "join_affine_c2" | "join_projective_c2" | "join_stationary_c2" | "join_mixed_degree" | "compact_join_seam" | "compare_parametric_seam" | "compare_geometric_seam" | "merge_weighted_curves" | "merge_weighted_chain" | "merge_curve_exact" | "merge_surface_exact") { return Some(exact_merge_requests::merge(v,op)); }
    if matches!(op,"fit_cst_sides" | "naca_five_digit_profile" | "cst_spline_profile" | "cst_section_integrals") { return Some(airfoil_section_requests::build(v,op)); }
    if matches!(op,"fit_sphere_cloud" | "fit_cylinder_cloud" | "fit_cone_cloud" | "fit_torus_cloud" | "fit_plane_cloud") { return Some(cloud_fit_surfaces::build(v,op)); }
    if matches!(op,"surface_highlight_lines" | "surface_reflection_lines" | "surface_isophote_analysis" | "surface_dent_detection") { return Some(optical_surface_analysis::inspect(v,op)); }
    if matches!(op,"curve_from_curvature_samples" | "fit_biarc_piece") { return Some(signed_curvature_fit::build(v,op)); }
    if matches!(op,"simplify_linear_curve" | "extend_terminal_curve" | "extend_surface_boundary") { return Some(terminal_span_edit::apply(v,op)); }
    if matches!(op,"affine_control_support" | "surface_trace_agreement" | "geometry_bounds_union" | "geometry_conditioning" | "coordinate_frame_local" | "coordinate_frame_world") {
        return Some(coordinate_analysis::apply(v,op));
    }
    if op == "shared_patch_mesh" { return Some(shared_patch_mesh::build(v)); }
    if matches!(op,"bounded_curve_chord" | "bounded_surface_mesh") {
        return Some(bounded_mesh_construction::build(v,op));
    }
    if matches!(op,"split_periodic_plane_strip"|"close_plane_strip"|"store_closed_plane_piece"|"trace_plane_strip"|"stitch_plane_strip"|"split_cell_by_plane"|"store_plane_branch"|"pair_plane_branch"|"sample_plane_branch"|"plane_branch_uv_polyline"|"plane_branch_xyz_polyline"){return Some(plane_branch_queries::inspect(v,op));}
    if matches!(op,"cap_section_wire"|"cap_section_holes"|"arrange_section_caps"){return Some(section_cap_queries::build(v,op));}
    if matches!(op,"audit_plane_side"|"refine_plane_side"|"audit_plane_graphs"){return Some(plane_side_queries::inspect(v,op));}
    if matches!(op,"parallel_plane_sections" | "parallel_section_piece") {
        return Some(parallel_section_queries::inspect(v,op));
    }
    if matches!(op,"clean_offset_boundary" | "planar_toolpath_edge") {
        return Some(planar_path_selection::select(v,op));
    }
    if op == "embed_xy_curve" {
        return Some((|| {
            let mut curve: crate::curve::Curve = field(v,"curve")?;
            let height: f64 = field(v,"height")?;
            curve.validate()?;
            check(curve.control_points[0].len()==2, "XY embedding requires a two-dimensional curve")?;
            for point in &mut curve.control_points { point.push(height); }
            curve.validate()?;
            encode(curve)
        })());
    }
    if op == "planar_knot_miter" {
        return Some((|| {
            let join = crate::curve_offset_join::miter_at_knot(&field(v,"curve")?, field(v,"knot")?,
                field(v,"distance_mm")?, field(v,"extension_limit_mm")?, field(v,"tolerance_mm")?)?;
            let curve = crate::curve::Curve::from_polyline(join.points.iter().map(|p| p.to_vec()).collect())?;
            Ok(json!({"curve":curve,"points":join.points,"errorUpperMm":join.error_upper_mm}))
        })());
    }
    if op == "phase_aligned_section" {
        return Some((|| {
            let curves: Vec<crate::curve::Curve> = field(v,"curves")?;
            let piece: usize = field(v,"piece")?;
            check(piece < curves.len(), "Selected section index exceeds the section count")?;
            let landmarks: Vec<Vec<f64>> = field(v,"landmarks")?;
            let result = crate::section_phase::align(&curves, &landmarks, field(v,"search_tolerance_mm")?,
                field(v,"shape_tolerance_mm")?, field(v,"max_cells")?)?;
            let phases = result.phases.iter().map(|p| json!({
                "status":format!("{:?}",p.status),"parameter":p.parameter,"searchBoundsMm":p.nearest.distance_interval,
                "shapeErrorUpperMm":p.shape.as_ref().map(|proof|proof.error_upper),
                "finalDistanceMm":p.final_distance_interval,"minimumGapUpperMm":p.final_minimum_gap_upper
            })).collect::<Vec<_>>();
            Ok(json!({"curve":result.curves.as_ref().map(|curves| &curves[piece]),"phases":phases}))
        })());
    }
    if matches!(op, "compatible_section" | "oriented_section") {
        return Some((|| {
            let curves: Vec<crate::curve::Curve> = field(v, "curves")?;
            let piece: usize = field(v, "piece")?;
            check(piece < curves.len(), "Selected section index exceeds the section count")?;
            let (sections, orientation) = if op == "compatible_section" {
                (crate::sections::compatible(&curves)?, None)
            } else {
                let oriented = crate::sections::orient(&curves, field(v,"tolerance_mm")?)?;
                (oriented.curves, Some(format!("{:?}", oriented.orientations[piece])))
            };
            let curve = &sections[piece];
            Ok(json!({"curve":curve,"degree":curve.degree,"knots":curve.knots,"orientation":orientation}))
        })());
    }
    if matches!(op, "uv_loop_closure" | "uv_loop_simplicity" | "uv_trim_location") {
        return Some(uv_loop_queries::inspect(v, op));
    }
    if op == "curve_edit_deviation" {
        return Some((|| {
            let report = crate::curve_deviation::inspect(
                &field(v, "curve")?, &field(v, "target")?, field(v, "tolerance_mm")?, field(v, "max_cells")?,
            )?;
            Ok(json!({"boundsMm":report.bounds,"converged":report.converged,"cells":report.cells,"reason":format!("{:?}",report.reason)}))
        })());
    }
    if op == "surface_edit_deviation" {
        return Some((|| {
            let upper = crate::continuity::deviation::positional_upper(&field(v,"surface")?, &field(v,"target")?)?;
            Ok(json!({"upperMm":upper}))
        })());
    }
    if matches!(op, "surface_seam_g0" | "surface_seam_g1" | "surface_seam_g2") {
        return Some(surface_seam_measurements::inspect(v, op));
    }
    if matches!(op, "curve_plane_roots" | "curve_pair_crossings" | "curve_plane_chart_crossings" | "surface_ray_hits") {
        return Some(intersection_measurements::inspect(v, op));
    }
    if matches!(op, "curve_length_bounds" | "curve_station_at_length" | "curve_divide_by_length" | "surface_area_bounds") {
        return Some(arc_metrology::measure(op, v));
    }
    if let Some(result) = engineering_profile(v, op) { return Some(result); }
    if let Some(res) = try_dispatch_affine_pipes_and_helicoids(v, op) {
        return Some(res);
    }
    if let Some(res) = try_dispatch_spirals_and_helices(v, op) {
        return Some(res);
    }
    if let Some(res) = try_dispatch_splines_and_section_repairs(v, op) {
        return Some(res);
    }
    try_dispatch_polynomials_and_quadrics(v, op)
}

fn engineering_profile(v: &Value, op: &str) -> Option<Result<Value>> {
    use crate::engineering_profiles as P;
    Some(match op {
        "rounded_rectangle_profile" => (|| encode(P::rounded_rectangle(field(v, "center")?, field(v, "width")?, field(v, "height")?, field(v, "radius")?)?))(),
        "capsule_profile" => (|| encode(P::capsule(field(v, "center")?, field(v, "straight")?, field(v, "radius")?)?))(),
        "annular_sector_surface" => (|| encode(P::annular_sector(field(v, "center")?, field(v, "normal")?, field(v, "inner_radius")?, field(v, "outer_radius")?, field(v, "start_degrees")?, field(v, "sweep_degrees")?)?))(),
        "keyway_profile" => (|| encode(P::keyway(field(v, "mouth")?, field(v, "width")?, field(v, "depth")?)?))(),
        "t_slot_profile" => (|| encode(P::t_slot(field(v, "mouth")?, field(v, "neck_width")?, field(v, "flange_width")?, field(v, "neck_depth")?, field(v, "total_depth")?)?))(),
        "dovetail_profile" => (|| encode(P::dovetail(field(v, "mouth")?, field(v, "mouth_width")?, field(v, "base_width")?, field(v, "depth")?)?))(),
        "poly_v_belt_profile" => (|| encode(P::poly_v_belt(field(v, "center")?, field(v, "ribs")?, field(v, "pitch")?, field(v, "back_thickness")?, field(v, "rib_height")?)?))(),
        "toothed_belt_profile" => (|| encode(P::toothed_belt(field(v, "center")?, P::ToothedBelt { teeth: field(v, "teeth")?, pitch: field(v, "pitch")?, base_width: field(v, "base_width")?, tip_width: field(v, "tip_width")?, back_thickness: field(v, "back_thickness")?, tooth_height: field(v, "tooth_height")? })?))(),
        "o_ring_groove_profile" => (|| encode(P::o_ring_groove(field(v, "mouth")?, field(v, "width")?, field(v, "depth")?, field(v, "radius")?)?))(),
        "retaining_ring_profile" => (|| encode(P::retaining_ring(field(v, "center")?, field(v, "inner_radius")?, field(v, "outer_radius")?, field(v, "gap_degrees")?)?))(),
        "hex_fastener_profile" => (|| encode(P::hex_fastener(field(v, "center")?, field(v, "across_flats")?)?))(),
        "square_fastener_profile" => (|| encode(P::square_fastener(field(v, "center")?, field(v, "across_flats")?)?))(),
        _ => return None,
    })
}

fn try_dispatch_affine_pipes_and_helicoids(v: &Value, op: &str) -> Option<Result<Value>> {
    let res = match op {
        "curve_affine" => field::<curve::Curve>(v, "curve")
            .and_then(|c| Ok(affine::curve(&c, &field(v, "matrix")?)?))
            .and_then(encode),
        "surface_affine" => field::<surface::Surface>(v, "surface")
            .and_then(|s| Ok(affine::surface(&s, &field(v, "matrix")?)?))
            .and_then(encode),
        "patches_affine" => field::<Vec<surface::Surface>>(v, "patches")
            .and_then(|p| Ok(affine::patches(&p, &field(v, "matrix")?)?))
            .and_then(encode),
        "surface_circle_rectangle_transition" => handle_circle_rectangle_transition(v),
        "surface_ellipse_transition" => handle_ellipse_transition(v),
        "surface_circle_transition" => handle_circle_transition(v),
        "surface_ribbon" => (|| {
            encode(ribbon::checked(
                &field::<curve::Curve>(v, "path")?,
                &field::<curve::Curve>(v, "width")?,
                field(v, "normal")?,
                field(v, "sections")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "surface_variable_pipe" => (|| {
            encode(pipe::checked_variable(
                &field::<curve::Curve>(v, "path")?,
                &field::<curve::Curve>(v, "radius")?,
                field(v, "normal")?,
                field(v, "sections")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "surface_pipe" => (|| {
            encode(pipe::checked(
                &field::<curve::Curve>(v, "path")?,
                field(v, "radius")?,
                field(v, "normal")?,
                field(v, "sections")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "surface_catenoid_patches" => (|| {
            encode(catenoid::approximate_patches(
                field(v, "center")?,
                field(v, "scale")?,
                field(v, "start_z")?,
                field(v, "end_z")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "surface_screw" => (|| {
            encode(screw_surface::approximate(
                &field::<curve::Curve>(v, "profile")?,
                field(v, "origin")?,
                field(v, "axis")?,
                field(v, "height")?,
                field(v, "turns")?,
                field(v, "phase_degrees")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "curve_clothoid" => (|| {
            encode(clothoid::approximate(
                field(v, "center")?,
                field(v, "length")?,
                field(v, "start_curvature")?,
                field(v, "end_curvature")?,
                field(v, "phase_degrees")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "curve_spherical_spiral" => (|| {
            encode(spherical_spiral::approximate(
                field(v, "center")?,
                field(v, "radius")?,
                field(v, "longitude_turns")?,
                field(v, "latitude_turns")?,
                field(v, "longitude_phase_degrees")?,
                field(v, "latitude_phase_degrees")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "surface_extrude_patches" => (|| {
            encode(extrusion_patches::extrude(
                &field::<curve::Curve>(v, "curve")?,
                field(v, "vector")?,
            )?)
        })(),
        "curve_toroidal_spiral" => (|| {
            encode(toroidal_spiral::approximate(
                toroidal_spiral::Torus { center: field(v, "center")?, major_radius: field(v, "major_radius")?, minor_radius: field(v, "minor_radius")? },
                toroidal_spiral::TorusWinding { major_turns: field(v, "major_turns")?, minor_turns: field(v, "minor_turns")?, major_phase_degrees: field(v, "major_phase_degrees")?, minor_phase_degrees: field(v, "minor_phase_degrees")? },
                field(v, "max_deviation")?,
            )?)
        })(),
        "curve_torus_knot" => (|| {
            encode(toroidal_spiral::approximate_knot(
                toroidal_spiral::Torus { center: field(v, "center")?, major_radius: field(v, "major_radius")?, minor_radius: field(v, "minor_radius")? },
                field(v, "p")?,
                field(v, "q")?,
                [field(v, "major_phase_degrees")?, field(v, "minor_phase_degrees")?],
                field(v, "max_deviation")?,
            )?)
        })(),
        "surface_helicoid_patches" => (|| {
            encode(helicoid::approximate_patches(
                field(v, "center")?,
                field(v, "inner_radius")?,
                field(v, "outer_radius")?,
                field(v, "height")?,
                field(v, "turns")?,
                field(v, "phase_degrees")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "surface_helicoid" => (|| {
            encode(helicoid::approximate(
                field(v, "center")?,
                field(v, "inner_radius")?,
                field(v, "outer_radius")?,
                field(v, "height")?,
                field(v, "turns")?,
                field(v, "phase_degrees")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "superellipsoid_patches" => (|| {
            use crate::superellipsoid::{approximate, Request, Stop};
            let result = approximate(Request {
                center: field(v, "center")?, radii: field(v, "radii")?,
                exponents: field(v, "exponents")?, tolerance: field(v, "tolerance")?,
                max_cells: field(v, "max_cells")?,
            })?;
            let (surfaces, domains): (Vec<_>, Vec<_>) = result.patches.into_iter()
                .map(|patch| (patch.surface, json!({"signs":patch.signs, "domain":patch.domain}))).unzip();
            Ok(json!({
                "patches": encode(surfaces)?,
                "report": {
                    "domains":domains, "cells":result.cells, "subdivisions":result.subdivisions,
                    "analyticInterpolationEstimateMm":result.analytic_interpolation_estimate,
                    "roundingCertified":false, "continuousBound":false,
                    "stop":match result.stop {
                        Stop::AnalyticEstimateReached => "analyticEstimateReached",
                        Stop::CellBudget => "cellBudget",
                    }
                }
            }))
        })(),
        "corrugated_bellows" => (|| {
            let shell = crate::bellows::cubic(crate::bellows::Request {
                center: field(v, "center")?,
                mean_radius: field(v, "mean_radius")?,
                amplitude: field(v, "amplitude")?,
                height: field(v, "height")?,
                radial_thickness: field(v, "radial_thickness")?,
                periods: field(v, "periods")?,
            })?;
            let mut patches = shell.outer;
            patches.extend(shell.inner);
            patches.extend([shell.bottom, shell.top]);
            encode(patches)
        })(),
        "optical_dome" => (|| encode(crate::optical_shapes::dome(
            field(v, "center")?, field(v, "base_axes")?, field(v, "height")?,
        )?))(),
        "optical_lens" => (|| {
            let lens = crate::optical_shapes::lens(
                field(v, "center")?, field(v, "base_axes")?,
                field(v, "sags")?, field(v, "vertex_thickness")?,
            )?;
            let mut patches = vec![lens.upper, lens.lower];
            patches.extend(lens.barrel);
            encode(patches)
        })(),
        "cubic_wave_surface" => (|| {
            encode(crate::wave_surface::cubic(
                field(v, "origin")?,
                field(v, "width")?,
                field(v, "length")?,
                field(v, "amplitude")?,
                field(v, "periods")?,
            )?)
        })(),
        "surface_catenoid" => (|| {
            encode(catenoid::approximate(
                field(v, "center")?,
                field(v, "scale")?,
                field(v, "start_z")?,
                field(v, "end_z")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        _ => return None,
    };
    Some(res)
}

fn try_dispatch_spirals_and_helices(v: &Value, op: &str) -> Option<Result<Value>> {
    let res = match op {
        "curve_catenary" => (|| {
            encode(catenary::approximate(
                field(v, "center")?,
                field(v, "scale")?,
                field(v, "start_x")?,
                field(v, "end_x")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "curve_archimedean_spiral" => (|| {
            encode(archimedean_spiral::approximate(
                field(v, "center")?,
                field(v, "start_radius")?,
                field(v, "end_radius")?,
                field(v, "start_degrees")?,
                field(v, "end_degrees")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "curve_epicycloid" => (|| {
            encode(circular_rolling::approximate_epicycloid(
                field(v, "center")?,
                field(v, "fixed_radius")?,
                field(v, "rolling_radius")?,
                field(v, "start_radians")?,
                field(v, "end_radians")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "curve_hypocycloid" => (|| {
            encode(circular_rolling::approximate_hypocycloid(
                field(v, "center")?,
                field(v, "fixed_radius")?,
                field(v, "rolling_radius")?,
                field(v, "start_radians")?,
                field(v, "end_radians")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "curve_trochoid" => (|| {
            encode(trochoid::approximate(
                field(v, "center")?,
                field(v, "rolling_radius")?,
                field(v, "tracing_radius")?,
                field(v, "start_radians")?,
                field(v, "end_radians")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "curve_cycloid" => (|| {
            encode(trochoid::approximate_cycloid(
                field(v, "center")?,
                field(v, "radius")?,
                field(v, "start_radians")?,
                field(v, "end_radians")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "curve_lissajous" => (|| {
            encode(lissajous::approximate(
                field(v, "center")?,
                field(v, "amplitudes")?,
                field(v, "frequencies")?,
                field(v, "phases_degrees")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "curve_logarithmic_spiral" => (|| {
            encode(logarithmic_spiral::approximate(
                field(v, "center")?,
                field(v, "radius")?,
                field(v, "growth")?,
                field(v, "start_radians")?,
                field(v, "end_radians")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "curve_involute" => (|| {
            encode(involute::approximate(
                field(v, "center")?,
                field(v, "radius")?,
                field(v, "start_radians")?,
                field(v, "end_radians")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "curve_variable_pitch_helix" => (|| {
            encode(helix::approximate_variable_pitch(
                field(v, "center")?,
                field(v, "radius")?,
                field(v, "height")?,
                field(v, "turns")?,
                [field(v, "start_pitch")?, field(v, "end_pitch")?],
                field(v, "phase_degrees")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "curve_elliptic_helix" => (|| {
            encode(helix::approximate_elliptic(
                field(v, "center")?,
                field(v, "radius_x")?,
                field(v, "radius_y")?,
                field(v, "height")?,
                field(v, "turns")?,
                field(v, "phase_degrees")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "curve_conical_helix" => (|| {
            encode(helix::approximate_conical(
                field(v, "center")?,
                field(v, "start_radius")?,
                field(v, "end_radius")?,
                field(v, "height")?,
                field(v, "turns")?,
                field(v, "phase_degrees")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        "curve_helix" => (|| {
            encode(helix::approximate(
                field(v, "center")?,
                field(v, "radius")?,
                field(v, "height")?,
                field(v, "turns")?,
                field(v, "phase_degrees")?,
                field(v, "max_deviation")?,
            )?)
        })(),
        _ => return None,
    };
    Some(res)
}

fn try_dispatch_splines_and_section_repairs(v: &Value, op: &str) -> Option<Result<Value>> {
    let res = match op {
        "surface_boundary_fill" => (|| {
            encode(boundary_fill::fan(
                &field::<Vec<curve::Curve>>(v, "boundaries")?,
                field(v, "center")?,
            )?)
        })(),
        "surface_triangular_patch" => (|| {
            encode(triangular_patch::patch(
                &field(v, "base")?,
                &field(v, "side_a")?,
                &field(v, "side_b")?,
            )?)
        })(),
        "surface_grid_spline" => (|| {
            encode(grid_spline::interpolate(
                &field::<Vec<Vec<[f64; 3]>>>(v, "points")?,
                &field::<Vec<f64>>(v, "parameters_u")?,
                &field::<Vec<f64>>(v, "parameters_v")?,
            )?)
        })(),
        "surface_hermite_patch" => (|| {
            encode(hermite_patch::patch(
                field(v, "corners")?,
                field(v, "tangent_u")?,
                field(v, "tangent_v")?,
                field(v, "twist")?,
            )?)
        })(),
        "curve_closed_spline" => (|| {
            encode(closed_spline::interpolate(
                &field::<Vec<[f64; 3]>>(v, "points")?,
                &field::<Vec<f64>>(v, "parameters")?,
            )?)
        })(),
        "curve_clamped_spline" => (|| {
            encode(natural_spline::clamped(
                &field::<Vec<[f64; 3]>>(v, "points")?,
                &field::<Vec<f64>>(v, "parameters")?,
                field(v, "start_tangent")?,
                field(v, "end_tangent")?,
            )?)
        })(),
        "curve_natural_spline" => (|| {
            encode(natural_spline::interpolate(
                &field::<Vec<[f64; 3]>>(v, "points")?,
                &field::<Vec<f64>>(v, "parameters")?,
            )?)
        })(),
        "curve_hermite" => (|| {
            encode(hermite::interpolate(
                &field::<Vec<[f64; 3]>>(v, "points")?,
                &field::<Vec<[f64; 3]>>(v, "tangents")?,
                &field::<Vec<f64>>(v, "parameters")?,
            )?)
        })(),
        "surface_formula" => (|| {
            encode(formula::surface(
                field(v, "domain")?,
                &formula::tokens(&field::<Vec<Value>>(v, "expressions")?, true)?,
            )?)
        })(),
        "curve_formula" => (|| {
            encode(formula::curve(
                field(v, "domain")?,
                &formula::tokens(&field::<Vec<Value>>(v, "expressions")?, false)?,
            )?)
        })(),
        "surface_sphere" => (|| encode(primitives::sphere(field(v, "center")?, field(v, "radius")?)?))(),
        "surface_cylinder" => (|| {
            encode(primitives::cylinder(
                field(v, "center")?,
                field(v, "radius")?,
                field(v, "height")?,
            )?)
        })(),
        "surface_cone" => (|| {
            encode(primitives::cone(
                field(v, "center")?,
                field(v, "radius")?,
                field(v, "height")?,
            )?)
        })(),
        "surface_bezier" => (|| encode(patches::bezier(field(v, "points")?, optional_field(v, "weights")?)?))(),
        "surface_bilinear" => (|| encode(patches::bilinear(field(v, "corners")?)?))(),
        "surface_plane" => (|| {
            encode(patches::plane(
                field(v, "origin")?,
                field(v, "axisU")?,
                field(v, "axisV")?,
            )?)
        })(),
        "curve_bezier" => (|| encode(paths::bezier(field(v, "points")?, optional_field(v, "weights")?)?))(),
        "curve_repair_circle_section" => (|| {
            let r = crate::section_circle_repair::repair(
                &field::<Vec<curve::Curve>>(v, "curves")?,
                field(v, "quantum")?,
                field(v, "tolerance")?,
                field(v, "maxWork")?,
            )?;
            Ok(json!({
                "curves": r.curves,
                "displacementUpper": r.displacement_upper,
                "work": r.work,
                "reason": r.reason
            }))
        })(),
        "curve_project_section_authored_axis" => (|| {
            let r = crate::section_projection::project_authored_axis(
                &field::<Vec<curve::Curve>>(v, "curves")?,
                &field::<curve::Curve>(v, "frameAxis")?,
                field(v, "traversal")?,
                field(v, "quantum")?,
                field(v, "tolerance")?,
                field(v, "maxWork")?,
            )?;
            Ok(json!({
                "curves": r.curves,
                "displacementUpper": r.displacement_upper,
                "exactPlanar": r.exact_planar,
                "work": r.work,
                "reason": r.reason
            }))
        })(),
        "curve_project_section" => (|| {
            let r = crate::section_projection::project(
                &field::<Vec<curve::Curve>>(v, "curves")?,
                field(v, "axis")?,
                field(v, "coefficients")?,
                field(v, "offset")?,
                field(v, "quantum")?,
                field(v, "tolerance")?,
                field(v, "maxWork")?,
            )?;
            Ok(json!({
                "curves": r.curves,
                "displacementUpper": r.displacement_upper,
                "exactPlanar": r.exact_planar,
                "work": r.work,
                "reason": r.reason
            }))
        })(),
        _ => return None,
    };
    Some(res)
}

fn try_dispatch_polynomials_and_quadrics(v: &Value, op: &str) -> Option<Result<Value>> {
    let res = match op {
        "curve_transition_polyline" => (|| {
            let closed = optional_field::<bool>(v, "closed")?.unwrap_or(false);
            let f = if closed { paths::closed_transition_polyline } else { paths::transition_polyline };
            encode(f(&field::<Vec<[f64; 3]>>(v, "points")?, field(v, "setback")?)?)
        })(),
        "curve_round_polyline" => (|| {
            let closed = optional_field::<bool>(v, "closed")?.unwrap_or(false);
            let f = if closed { paths::closed_round_polyline } else { paths::round_polyline };
            encode(f(&field::<Vec<[f64; 3]>>(v, "points")?, field(v, "radius")?)?)
        })(),
        "curve_compose" => (|| encode(paths::compose(&field::<Vec<curve::Curve>>(v, "curves")?)?))(),
        "curve_line" => (|| encode(primitives::line(field(v, "start")?, field(v, "end")?)?))(),
        "curve_polyline" => (|| {
            encode(primitives::polyline(
                &field::<Vec<[f64; 3]>>(v, "points")?,
                optional_field(v, "closed")?.unwrap_or(false),
            )?)
        })(),
        "curve_circle" => (|| {
            encode(primitives::circle(
                field(v, "center")?,
                field(v, "normal")?,
                field(v, "radius")?,
            )?)
        })(),
        "curve_circle_arc" => (|| {
            encode(primitives::circle_arc(
                field(v, "center")?,
                field(v, "normal")?,
                field(v, "radius")?,
                field(v, "startDegrees")?,
                field(v, "sweepDegrees")?,
            )?)
        })(),
        "curve_rational_polynomial" => (|| {
            encode(polynomial::rational_curve(
                field(v, "domain")?,
                &field::<Vec<[f64; 4]>>(v, "coefficients")?,
            )?)
        })(),
        "surface_rational_polynomial" => (|| {
            encode(polynomial::rational_surface(
                field(v, "domain")?,
                &field::<Vec<Vec<[f64; 4]>>>(v, "coefficients")?,
            )?)
        })(),
        "curve_polynomial_parametric" => (|| {
            encode(polynomial::parametric_curve(
                field(v, "domain")?,
                &field::<Vec<[f64; 3]>>(v, "coefficients")?,
            )?)
        })(),
        "surface_polynomial_parametric" => (|| {
            encode(polynomial::parametric_surface(
                field(v, "domain")?,
                &field::<Vec<Vec<[f64; 3]>>>(v, "coefficients")?,
            )?)
        })(),
        "surface_polynomial_graph" => (|| {
            encode(polynomial::graph(
                field(v, "bounds")?,
                &field::<Vec<Vec<f64>>>(v, "coefficients")?,
            )?)
        })(),
        "surface_hyperboloid_one_sheet" => (|| {
            encode(primitives::hyperboloid_one_sheet(
                field(v, "center")?,
                field(v, "radii")?,
                field(v, "start")?,
                field(v, "end")?,
            )?)
        })(),
        "surface_hyperboloid_two_sheet" => (|| {
            encode(primitives::hyperboloid_two_sheet(
                field(v, "center")?,
                field(v, "radii")?,
                field(v, "start")?,
                field(v, "end")?,
                optional_field(v, "lower")?.unwrap_or(false),
            )?)
        })(),
        "surface_quadratic_patch" => (|| {
            encode(primitives::quadratic_patch(
                field(v, "bounds")?,
                field(v, "coefficients")?,
            )?)
        })(),
        "curve_parabola" | "curve_hyperbola" => (|| {
            let constructor = if op == "curve_parabola" {
                primitives::parabola
            } else {
                primitives::hyperbola
            };
            encode(constructor(
                field(v, "center")?,
                field(v, "axisU")?,
                field(v, "axisV")?,
                field(v, "start")?,
                field(v, "end")?,
            )?)
        })(),
        "surface_elliptic_cylinder" => (|| {
            encode(primitives::elliptic_cylinder(
                field(v, "center")?,
                field(v, "radiusX")?,
                field(v, "radiusY")?,
                field(v, "height")?,
            )?)
        })(),
        "surface_cone_frustum" => (|| {
            encode(primitives::cone_frustum(
                field(v, "center")?,
                field(v, "bottomRadius")?,
                field(v, "topRadius")?,
                field(v, "height")?,
            )?)
        })(),
        "curve_ellipse_arc" => (|| {
            encode(primitives::ellipse_arc(
                field(v, "center")?,
                field(v, "axisU")?,
                field(v, "axisV")?,
                field(v, "startDegrees")?,
                field(v, "sweepDegrees")?,
            )?)
        })(),
        "surface_ellipsoid" => (|| {
            encode(primitives::ellipsoid(
                field(v, "center")?,
                field(v, "radii")?,
            )?)
        })(),
        "surface_torus" => (|| {
            encode(primitives::torus(
                field(v, "center")?,
                field(v, "majorRadius")?,
                field(v, "radialRadius")?,
                field(v, "axialRadius")?,
            )?)
        })(),
        _ => return None,
    };
    Some(res)
}

fn handle_circle_rectangle_transition(v: &Value) -> Result<Value> {
    let circle = circle_transition::CircleSection {
        center: field(v, "circle_center")?,
        normal: field(v, "circle_normal")?,
        seam: field(v, "circle_seam")?,
        radius: field(v, "circle_radius")?,
    };
    let rectangle = circle_rectangle_transition::RectangleSection {
        center: field(v, "rectangle_center")?,
        axis_u: field(v, "rectangle_axis_u")?,
        axis_v: field(v, "rectangle_axis_v")?,
    };
    encode(circle_rectangle_transition::ruled(&circle, &rectangle)?)
}

fn handle_ellipse_transition(v: &Value) -> Result<Value> {
    let start = ellipse_transition::EllipseSection {
        center: field(v, "start_center")?,
        axis_u: field(v, "start_axis_u")?,
        axis_v: field(v, "start_axis_v")?,
    };
    let end = ellipse_transition::EllipseSection {
        center: field(v, "end_center")?,
        axis_u: field(v, "end_axis_u")?,
        axis_v: field(v, "end_axis_v")?,
    };
    encode(ellipse_transition::ruled(&start, &end)?)
}

fn handle_circle_transition(v: &Value) -> Result<Value> {
    let start = circle_transition::CircleSection {
        center: field(v, "start_center")?,
        normal: field(v, "start_normal")?,
        seam: field(v, "start_seam")?,
        radius: field(v, "start_radius")?,
    };
    let end = circle_transition::CircleSection {
        center: field(v, "end_center")?,
        normal: field(v, "end_normal")?,
        seam: field(v, "end_seam")?,
        radius: field(v, "end_radius")?,
    };
    encode(circle_transition::ruled(&start, &end)?)
}
