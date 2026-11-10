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
    core_queries::dispatch(v, op_str)
}
