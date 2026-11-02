//! Bridge operations of the `intersections` family.
use crate::*;

const OPS: &[&str] = &[
    "nurbs_surface_self_intersection_audit",
    "nurbs_surface_iso_self_intersection_audit",
    "nurbs_surface_iso_target_window",
    "nurbs_original_affine_patch_contacts",
    "brep_intersect_surface_surface",
    "brep_intersect_curve_segment",
    "brep_intersect_curve_plane",
    "brep_intersect_surface_plane",
    "brep_intersect_curve_surface",
    "brep_intersect_curve_ruled_surface",
    "brep_intersect_curve_curve",
    "brep_intersect_sphere_sphere",
    "brep_intersect_sphere_cylinder",
    "brep_intersect_sphere_cone",
    "brep_intersect_cone_cone",
    "brep_intersect_cylinder_cylinder",
    "brep_intersect_plane_sphere",
    "brep_intersect_plane_cylinder",
    "brep_intersect_plane_cone",
    "brep_intersect_plane_torus",
    "brep_intersect_sphere_torus",
    "brep_intersect_cylinder_torus",
    "brep_intersect_cone_torus",
    "brep_intersect_torus_torus",
    "brep_intersection_trace_curve",
    "brep_intersection_trace_curve_segments",
    "brep_intersection_trace_evaluate",
];

pub(crate) fn route(v: Value) -> Routed {
    if !OPS.contains(&v["op"].as_str().unwrap_or("")) {
        return Routed::Unhandled(v);
    }
    Routed::Handled(handle(v))
}

fn handle(v: Value) -> Result<Value> {
    match v["op"].as_str().unwrap_or("") {
        "nurbs_original_affine_patch_contacts" => intersections::original_patch_contacts::run(v),
        "nurbs_surface_iso_target_window" => intersections::surface_iso_target::run(v),
        "nurbs_surface_self_intersection_audit" => intersections::self_surface::run(v),
        "nurbs_surface_iso_self_intersection_audit" => intersections::self_surface::run_iso(v),
        "brep_intersect_surface_surface"
        | "brep_intersect_curve_segment"
        | "brep_intersect_curve_plane"
        | "brep_intersect_surface_plane"
        | "brep_intersect_curve_surface"
        | "brep_intersect_curve_ruled_surface"
        | "brep_intersect_curve_curve"
        | "brep_intersect_sphere_sphere"
        | "brep_intersect_sphere_cylinder"
        | "brep_intersect_sphere_cone"
        | "brep_intersect_cone_cone"
        | "brep_intersect_cylinder_cylinder"
        | "brep_intersect_plane_sphere"
        | "brep_intersect_plane_cylinder"
        | "brep_intersect_plane_cone"
        | "brep_intersect_plane_torus"
        | "brep_intersect_sphere_torus"
        | "brep_intersect_cylinder_torus"
        | "brep_intersect_cone_torus"
        | "brep_intersect_torus_torus"
        | "brep_intersection_trace_curve"
        | "brep_intersection_trace_curve_segments"
        | "brep_intersection_trace_evaluate" => intersections::dispatch(v),
        _ => unreachable!("route admits only OPS"),
    }
}
