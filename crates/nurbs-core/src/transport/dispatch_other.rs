use super::*;

const SWEEP_REQUEST_OPS: &[&str] = &[
    "sweep_error_upper_compose",
    "sweep_exact_seam_set_audit",
    "sweep_projective_seam_set_audit",
    "surface_progressive_sweep_spatial_rmf_error",
    "surface_progressive_sweep",
    "surface_progressive_sweep_profiles",
    "surface_progressive_sweep_level",
    "surface_progressive_sweep_cap_domains",
    "surface_progressive_sweep_cap_projection",
    "surface_progressive_sweep_station_seams",
    "surface_progressive_sweep_profile_join",
    "surface_progressive_sweep_decomposition_joins",
    "surface_progressive_sweep_decomposition_smoothness",
    "surface_progressive_sweep_frame_smoothness",
    "surface_progressive_sweep_closed_frame_smoothness",
    "surface_progressive_sweep_closed_path_frame_smoothness",
    "surface_progressive_sweep_closed_guided_frame_smoothness",
    "surface_twist_sweep",
    "surface_two_guide_sweep",
    "sweep_repair_circle_sections",
    "sweep_retained_patch_regularity",
    "sweep_frenet_frame_regularity",
    "sweep_fixed_normal_frame_regularity",
    "sweep_authored_frame_regularity",
    "sweep_correct_miter_sections",
    "sweep_project_miter_caps",
    "sweep_project_sections",
    "curve_miter_sections",
    "surface_scaled_sweep",
    "surface_profile_sweep",
    "surface_framed_sweep",
    "surface_sweep",
];

const LOFT_REQUEST_OPS: &[&str] = &[
    "surface_auto_guided_loft",
    "surface_auto_guided_loft_cartesian",
    "surface_guided_loft_cartesian",
    "surface_natural_loft_checked",
    "surface_gordon_cartesian",
    "surface_gordon",
    "surface_closed_loft",
    "surface_loft_match_ends",
    "surface_guided_loft",
    "surface_control_tangent_loft",
    "surface_clamped_loft",
    "surface_natural_loft",
    "loft_aligned",
    "loft",
];

const CERTIFICATE_REQUEST_OPS: &[&str] = &[
    "curve_decomposition_audit",
    "surface_boundary_v_tangent_certify",
    "curve_certify_foundation",
    "surface_certify_foundation",
    "curve_project_certified",
    "surface_project_certified",
    "curve_interpolate_certified",
    "curve_approximate_certified",
    "curve_insert_certified",
    "curve_elevate_certified",
    "curve_remove_certified",
    "curve_rebuild_certified",
    "curve_reduce_certified",
    "surface_rebuild_certified",
    "surface_remove_certified",
    "surface_reduce_certified",
    "curve_periodic_edit_certified",
    "curve_periodic_split_certified",
    "surface_periodic_edit_certified",
    "curve_reparameterization_certify",
    "curve_fit_certified",
    "surface_interpolate_certified",
    "surface_fit_certified",
    "curve_fit_cloud_certified",
    "surface_fit_cloud_certified",
    "curve_curve_intersect_certified",
    "curve_surface_intersect_certified",
    "surface_surface_intersect_certified",
];

pub(super) fn dispatch_other(v: Value, op: String) -> Result<Value> {
    let op_str = op.as_str();
    if SWEEP_REQUEST_OPS.contains(&op_str) {
        return sweep_requests::dispatch(v, op);
    }
    if LOFT_REQUEST_OPS.contains(&op_str) {
        return loft_requests::dispatch(v, op);
    }
    if CERTIFICATE_REQUEST_OPS.contains(&op_str) {
        return certificate_requests::dispatch(v, op);
    }
    if let Some(res) = parametric_primitives::try_dispatch(&v, op_str) {
        return res;
    }
    if let Some(res) = surface_offset_requests::try_dispatch(&v, op_str) {
        return res;
    }
    if let Some(res) = editing_and_continuity::try_dispatch(&v, op_str) {
        return res;
    }
    if matches!(op_str,
        "rotate_periodic_surface_at_exact_knot" |
        "insert_and_rotate_periodic_surface_seam" |
        "open_exact_periodic_surface_axis" |
        "restrict_exact_periodic_surface_axis" |
        "restrict_exact_periodic_surface_over_turns" |
        "restrict_exact_doubly_periodic_surface" |
        "restrict_exact_doubly_periodic_surface_over_turns" |
        "partition_exact_periodic_surface_axis" |
        "partition_exact_periodic_surface_over_turns" |
        "partition_exact_doubly_periodic_surface" |
        "partition_exact_doubly_periodic_surface_over_turns" |
        "remove_exact_periodic_surface_knot" |
        "simplify_exact_periodic_surface_axis" |
        "remove_exact_periodic_surface_seam_knot" |
        "simplify_exact_doubly_periodic_surface" |
        "reduce_exact_periodic_surface_period" |
        "repeat_exact_periodic_surface_period"
    ) {
        return periodic_surface_edits::dispatch(&v, op_str);
    }
    if periodic_chart_measurements::OPS.contains(&op_str) {
        return periodic_chart_measurements::dispatch(&v, op_str);
    }
    if trajectory_ranges::OPS.contains(&op_str) { return trajectory_ranges::dispatch(&v,op_str); }
    if motion_turns::OPS.contains(&op_str) { return motion_turns::dispatch(&v,op_str); }
    if motion_jumps::OPS.contains(&op_str) { return motion_jumps::dispatch(&v,op_str); }
    if trimmed_surface_integrals::OPS.contains(&op_str) { return trimmed_surface_integrals::dispatch(&v, op_str); }
    if periodic_trim_components::OPS.contains(&op_str) { return periodic_trim_components::dispatch(&v, op_str); }
    if trim_contour_measurements::OPS.contains(&op_str) {return trim_contour_measurements::dispatch(&v,op_str);}
    if periodic_surface_assembly::OPS.contains(&op_str) {
        return periodic_surface_assembly::dispatch(&v, op_str);
    }
    core_queries::dispatch(v, op_str)
}
