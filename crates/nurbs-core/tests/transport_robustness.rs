//! Mutation robustness of the JSON transport: every operation name found in
//! the dispatcher receives generated requests built from valid geometry,
//! hostile numbers and wrong types. `execute` must always answer with a
//! response envelope, never panic. Deterministic and dependency-free.
#![cfg(feature = "transport")]
use std::panic::{AssertUnwindSafe, catch_unwind};
use value_codec::{Value, json};

/// String literals of the transport modules that look like operation names;
/// non-operations are harmless (they exercise the unknown-op path).
const OPS: &[&str] = &[
    "arc_length",
    "axis_scale",
    "axis_y",
    "axis_z",
    "boundary_tangents",
    "center_law",
    "circle_center",
    "circle_normal",
    "circle_radius",
    "circle_seam",
    "contact_parameter",
    "contact_profile",
    "corrected_frenet",
    "curve_affine",
    "curve_approximate_certified",
    "curve_archimedean_spiral",
    "curve_bezier",
    "curve_bounds",
    "curve_catenary",
    "curve_certify_foundation",
    "curve_chain_diagnostics",
    "curve_circle",
    "curve_circle_arc",
    "curve_clamped_spline",
    "curve_closed_spline",
    "curve_clothoid",
    "curve_compose",
    "curve_conical_helix",
    "curve_curve_intersect_certified",
    "curve_cycloid",
    "curve_decompose",
    "curve_decomposition_audit",
    "curve_decomposition_batch_audit",
    "curve_distance",
    "curve_elevate",
    "curve_elevate_certified",
    "curve_ellipse_arc",
    "curve_elliptic_helix",
    "curve_epicycloid",
    "curve_evaluate",
    "curve_evaluate_points",
    "curve_exact_ellipse",
    "curve_fit_certified",
    "curve_fit_cloud_certified",
    "curve_formula",
    "curve_helix",
    "curve_hermite",
    "curve_hyperbola",
    "curve_hypocycloid",
    "curve_insert",
    "curve_insert_certified",
    "curve_interpolate_certified",
    "curve_involute",
    "curve_line",
    "curve_lissajous",
    "curve_logarithmic_spiral",
    "curve_match_g1",
    "curve_materialize_reparameterization",
    "curve_materialize_reparameterization_bounded",
    "curve_miter_sections",
    "curve_natural_spline",
    "curve_offset_bevel_wire",
    "curve_offset_bounded",
    "curve_offset_trimmed_bevel",
    "curve_parabola",
    "curve_periodic_edit_certified",
    "curve_periodic_split_certified",
    "curve_polyline",
    "curve_polynomial_parametric",
    "curve_prepare_coons_weights",
    "curve_progressive_miter",
    "curve_progressive_miter_cap_domains",
    "curve_progressive_miter_cap_parallelism",
    "curve_progressive_miter_cap_projection",
    "curve_progressive_miter_level",
    "curve_progressive_miter_wall_audit",
    "curve_project_certified",
    "curve_project_section",
    "curve_project_section_authored_axis",
    "curve_rational_polynomial",
    "curve_rebuild_certified",
    "curve_reduce_certified",
    "curve_remove_certified",
    "curve_repair_circle_section",
    "curve_reparameterization_certify",
    "curve_reparameterize_exact",
    "curve_reparameterized_evaluate",
    "curve_reverse",
    "curve_round_polyline",
    "curve_sample",
    "curve_segmented_bezier_controls",
    "curve_spherical_spiral",
    "curve_split",
    "curve_surface_intersect_certified",
    "curve_toroidal_spiral",
    "curve_torus_knot",
    "curve_transition_polyline",
    "curve_trim",
    "curve_trim_point",
    "curve_trim_screen_point",
    "curve_trochoid",
    "curve_validate",
    "curve_variable_pitch_helix",
    "dependent_axes",
    "depth_limit",
    "end_axis_u",
    "end_axis_v",
    "end_center",
    "end_curvature",
    "end_degrees",
    "end_normal",
    "end_pitch",
    "end_radians",
    "end_radius",
    "end_seam",
    "end_tangent",
    "end_tangents",
    "end_x",
    "end_z",
    "error_max_cells",
    "error_max_products",
    "exceeds_tolerance",
    "fixed_normal",
    "fixed_radius",
    "frame_axis",
    "frame_normal",
    "guide_a",
    "guide_b",
    "guide_error_upper",
    "guide_order",
    "guide_parameters",
    "initial_sections",
    "initial_steps",
    "inner_radius",
    "invalid_output",
    "latitude_phase_degrees",
    "latitude_turns",
    "length_max_cells",
    "length_tolerance",
    "loft_aligned",
    "longitude_phase_degrees",
    "longitude_turns",
    "major_phase_degrees",
    "major_radius",
    "major_turns",
    "max_deviation",
    "max_sections",
    "max_steps",
    "minor_phase_degrees",
    "minor_radius",
    "minor_turns",
    "miter_limit",
    "not_representable",
    "nurbs_brush_curve",
    "nurbs_brush_surface",
    "nurbs_deform_curve",
    "nurbs_deform_surface",
    "nurbs_intersection_resource_probe",
    "nurbs_sculpt_curve",
    "nurbs_sculpt_surface",
    "nurbs_ss_resource_probe",
    "orientation_guide",
    "original_section_certificates",
    "outer_radius",
    "parameter_tolerance",
    "parameters_u",
    "parameters_v",
    "patches_affine",
    "phase_degrees",
    "phases_degrees",
    "precision_limit",
    "preview_sections",
    "preview_steps",
    "profile_section_bridge_chords",
    "radius_x",
    "radius_y",
    "rectangle_axis_u",
    "rectangle_axis_v",
    "rectangle_center",
    "resource_limit",
    "rmf_transport_steps",
    "rolling_radius",
    "section_error_upper",
    "section_mapping_certificates",
    "section_mappings",
    "side_a",
    "side_b",
    "start_axis_u",
    "start_axis_v",
    "start_center",
    "start_curvature",
    "start_degrees",
    "start_normal",
    "start_pitch",
    "start_radians",
    "start_radius",
    "start_seam",
    "start_tangent",
    "start_tangents",
    "start_x",
    "start_z",
    "surface_affine",
    "surface_auto_guided_loft",
    "surface_auto_guided_loft_cartesian",
    "surface_bezier",
    "surface_bilinear",
    "surface_boundary_fill",
    "surface_boundary_v_tangent_certify",
    "surface_bounds",
    "surface_catenoid",
    "surface_catenoid_patches",
    "surface_certify_foundation",
    "surface_circle_rectangle_transition",
    "surface_circle_transition",
    "surface_clamped_loft",
    "surface_closed_loft",
    "surface_cone",
    "surface_cone_frustum",
    "surface_control_tangent_loft",
    "surface_coons_patch",
    "surface_cylinder",
    "surface_distance",
    "surface_elevate",
    "surface_ellipse_transition",
    "surface_ellipsoid",
    "surface_elliptic_cylinder",
    "surface_evaluate",
    "surface_exact_ellipsoid",
    "surface_exact_frustum_side",
    "surface_exact_planar_region",
    "surface_exact_ring_torus",
    "surface_exact_strip_jets_audit",
    "surface_extrude_patches",
    "surface_fit_certified",
    "surface_fit_cloud_certified",
    "surface_formula",
    "surface_framed_sweep",
    "surface_gordon",
    "surface_gordon_cartesian",
    "surface_grid_spline",
    "surface_guided_loft",
    "surface_guided_loft_cartesian",
    "surface_helicoid",
    "surface_helicoid_patches",
    "surface_hermite_patch",
    "surface_hyperboloid_one_sheet",
    "surface_hyperboloid_two_sheet",
    "surface_insert",
    "surface_interpolate_certified",
    "surface_iso",
    "surface_linear_injectivity_audit",
    "surface_loft_match_ends",
    "surface_match_jets",
    "surface_natural_loft",
    "surface_natural_loft_checked",
    "surface_offset_bounds",
    "surface_offset_candidates",
    "surface_offset_contact_band",
    "surface_offset_contact_qualification",
    "surface_offset_contact_section",
    "surface_offset_contact_tangent",
    "surface_offset_continuous_deviation",
    "surface_offset_envelope",
    "surface_offset_envelope_fit",
    "surface_offset_evaluate",
    "surface_offset_jacobian_bounds",
    "surface_offset_source_boundary",
    "surface_offset_trimmed_contact_band",
    "surface_periodic_edit_certified",
    "surface_pipe",
    "surface_plane",
    "surface_polynomial_graph",
    "surface_polynomial_parametric",
    "surface_prepare_seams",
    "surface_profile_sweep",
    "surface_progressive_sweep",
    "surface_progressive_sweep_cap_domains",
    "surface_progressive_sweep_cap_projection",
    "surface_progressive_sweep_closed_frame_smoothness",
    "surface_progressive_sweep_closed_guided_frame_smoothness",
    "surface_progressive_sweep_closed_path_frame_smoothness",
    "surface_progressive_sweep_decomposition_joins",
    "surface_progressive_sweep_decomposition_smoothness",
    "surface_progressive_sweep_frame_smoothness",
    "surface_progressive_sweep_level",
    "surface_progressive_sweep_profile_join",
    "surface_progressive_sweep_profiles",
    "surface_progressive_sweep_spatial_rmf_error",
    "surface_progressive_sweep_station_seams",
    "surface_project_certified",
    "surface_projective_strip_jets_audit",
    "surface_quadratic_patch",
    "surface_rational_polynomial",
    "surface_rebuild_certified",
    "surface_reduce_certified",
    "surface_remove_certified",
    "surface_reverse",
    "surface_ribbon",
    "surface_scaled_sweep",
    "surface_screw",
    "surface_sphere",
    "surface_station_normal_scale",
    "surface_surface_intersect_certified",
    "surface_surface_verify_coverage",
    "surface_sweep",
    "surface_torus",
    "surface_triangular_patch",
    "surface_trim",
    "surface_twist_sweep",
    "surface_two_guide_sweep",
    "surface_validate",
    "surface_variable_pipe",
    "sweep_authored_frame_regularity",
    "sweep_boundary_certificate",
    "sweep_boundary_coverage_audit",
    "sweep_boundary_error_upper",
    "sweep_cap_boundary_audit",
    "sweep_cap_wall_audit",
    "sweep_coedge_agreement_audit",
    "sweep_coedge_exact_audit",
    "sweep_contour_audit",
    "sweep_correct_miter_sections",
    "sweep_degrees",
    "sweep_error_upper_compose",
    "sweep_exact_seam_set_audit",
    "sweep_exact_seams_audit",
    "sweep_filled_cap_error_upper",
    "sweep_fixed_normal_frame_regularity",
    "sweep_frenet_frame_regularity",
    "sweep_profile_regularity_audit",
    "sweep_project_miter_caps",
    "sweep_project_sections",
    "sweep_projective_seam_set_audit",
    "sweep_projective_seams_audit",
    "sweep_repair_circle_sections",
    "sweep_retained_body_coverage_audit",
    "sweep_retained_cap_contour_audit",
    "sweep_retained_patch_regularity",
    "sweep_retained_wall_coefficients_audit",
    "sweep_retained_wall_domain_audit",
    "sweep_retained_wall_family_audit",
    "sweep_seam_audit",
    "sweep_wall_audit",
    "tangent_u",
    "tangent_v",
    "tracing_radius",
    "transport_steps",
    "u_curves",
    "unproved_identity",
    "v_curves",
    "within_tolerance",
    "work_limit",
];
/// Every field name the dispatcher reads with `field` or `optional_field`.
const FIELDS: &[&str] = &["a", "amplitudes", "angle", "authoredFrame", "axialRadius", "axis", "axisU", "axisV", "axis_scale", "axis_y", "axis_z", "b", "base", "bottomRadius", "boundary", "boundary_tangents", "bounds", "brush", "budget", "candidate", "cap", "capCorrection", "caps", "center", "center_law", "circleCorrection", "circle_center", "circle_normal", "circle_radius", "circle_seam", "clearance", "closed", "coefficients", "contact_parameter", "contact_profile", "controlCount", "controls", "controlsU", "controlsV", "corners", "correctionDisplacementUpper", "count", "curve", "decompositionErrorUpper", "deformation", "degree", "degreeU", "degreeV", "distance", "distanceTolerance", "distances", "domain", "domains", "edited", "editedBoundary", "editedEnd", "end", "end_axis_u", "end_axis_v", "end_center", "end_curvature", "end_degrees", "end_normal", "end_pitch", "end_radians", "end_radius", "end_seam", "end_tangent", "end_tangents", "end_x", "end_z", "endpointContourErrorUpper", "errorBudget", "error_max_cells", "error_max_products", "fillRule", "first", "firstCoedges", "firstLoops", "firstOther", "fixed", "fixedAxis", "fixedInterval", "fixed_radius", "frameAxis", "frame_axis", "frame_normal", "frequencies", "growth", "guide_a", "guide_b", "height", "idealCapDomainsCertified", "includeEnd", "initial_sections", "initial_steps", "inner_radius", "intersectionToleranceMm", "keep", "kind", "latitude_phase_degrees", "latitude_turns", "leftPatch", "length", "length_max_cells", "length_tolerance", "longitude_phase_degrees", "longitude_turns", "longitudinal", "lower", "majorRadius", "major_phase_degrees", "major_radius", "major_turns", "matrix", "maxAgreementCells", "maxAngleDegrees", "maxBoxes", "maxCells", "maxControlRows", "maxControls", "maxDepth", "maxDeviation", "maxDistance", "maxDomainCells", "maxError", "maxExactWork", "maxFaces", "maxFitCells", "maxInjectivityCells", "maxMapEvaluations", "maxNormalCells", "maxNormalSpans", "maxPairCells", "maxPairs", "maxProducts", "maxSeams", "maxSineSquared", "maxSpans", "maxTangentPositionCells", "maxTrimCells", "maxUvCells", "maxWalls", "maxWitnessChecks", "maxWork", "max_deviation", "max_sections", "max_steps", "minor_phase_degrees", "minor_radius", "minor_turns", "miter_limit", "model", "normal", "normalScale", "offset", "op", "openPeriodic", "operation", "order", "orientation", "orientation_guide", "origin", "outer_radius", "p", "parallelPlanesCertified", "parameterTolerance", "parameter_tolerance", "parameters", "path", "phase_degrees", "phases_degrees", "point", "points", "preview_sections", "preview_steps", "profile", "projection", "projectionNormalDots", "q", "quantum", "radialRadius", "radii", "radius", "radiusX", "radiusY", "radius_x", "radius_y", "rectangle_axis_u", "rectangle_axis_v", "rectangle_center", "reference", "referenceBoundary", "referenceEnd", "retained", "retainedCapRegionsExact", "reverse", "reversed", "rightPatch", "rmf_transport_steps", "rolling_radius", "rootRefinements", "scale", "seams", "second", "secondCoedges", "secondDomain", "secondLoops", "sections", "segments", "setback", "side_a", "side_b", "source", "sourcePcurves", "spacing", "span", "start", "startDegrees", "start_axis_u", "start_axis_v", "start_center", "start_curvature", "start_degrees", "start_normal", "start_pitch", "start_radians", "start_radius", "start_seam", "start_tangent", "start_tangents", "start_x", "start_z", "surface", "sweepDegrees", "sweep_degrees", "tangent_u", "tangent_v", "tolerance", "toleranceMm", "toleranceUv", "topRadius", "tracing_radius", "transport_steps", "transverse", "transverseScale", "traversal", "turns", "twist", "u", "uv", "v", "vector", "wall", "weights", "width", "world"];

struct Lcg(u64);
impl Lcg {
    fn below(&mut self, bound: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 17) % bound.max(1) as u64) as usize
    }
}

fn curve() -> Value {
    json!({"degree": 2, "knots": [0., 0., 0., 0.5, 1., 1., 1.],
        "controlPoints": [[0., 0., 0.], [1., 2., 0.], [2., -1., 1.], [3., 0., 0.]],
        "weights": [1., 0.8, 1.2, 1.]})
}
fn surface() -> Value {
    json!({"degreeU": 1, "degreeV": 1, "knotsU": [0., 0., 1., 1.], "knotsV": [0., 0., 1., 1.],
        "controlPoints": [[[0., 0., 0.], [0., 1., 0.]], [[1., 0., 0.], [1., 1., 0.3]]],
        "weights": [[1., 1.], [1., 1.]]})
}

/// Values a field may receive. Budgets stay small so that requests which
/// happen to be valid finish quickly.
fn payload(rng: &mut Lcg) -> Value {
    match rng.below(22) {
        0 => curve(),
        1 => surface(),
        2 => json!([curve(), curve()]),
        3 => json!(0),
        4 => json!(1),
        5 => json!(16),
        6 => json!(-1),
        7 => json!(0.5),
        8 => json!(1e-9),
        9 => json!(1e300),
        10 => json!(-1e300),
        11 => json!([0., 0., 1.]),
        12 => json!([0., 0., 0.]),
        13 => json!([0., 1.]),
        14 => json!([[0., 0., 0.], [1., 0., 0.], [1., 1., 0.]]),
        15 => json!(null),
        16 => json!(true),
        17 => json!("x"),
        18 => json!([]),
        19 => json!({}),
        20 => json!([[0., 1.], [0., 1.]]),
        _ => json!(u64::MAX),
    }
}

/// Field that the dispatcher reported as missing or malformed, if any.
fn rejected_field(envelope: &Value) -> Option<String> {
    let message = envelope["error"]["message"].as_str()?;
    let rest = message.strip_prefix("Invalid ")?;
    Some(rest.split(':').next()?.to_string())
}

/// Run one request; a panic is returned as its message.
fn run(text: &str) -> Result<Value, String> {
    match catch_unwind(AssertUnwindSafe(|| nurbs_core::execute(text))) {
        Ok(response) => Ok(value_codec::from_str(&response)
            .unwrap_or_else(|e| panic!("response is not JSON: {e}: {response}"))),
        Err(payload) => Err(payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default()),
    }
}

fn clamp_budgets(request: &mut Value, rng: &mut Lcg) {
    for budget in ["maxWork", "maxCells", "maxSpans", "maxExactWork", "maxPairs", "maxProducts",
        "maxDomainCells", "maxBoxes", "maxDepth", "count", "segments", "turns", "maxFaces",
        "maxSeams", "maxWalls", "maxControls", "maxControlRows", "maxFitCells", "maxTrimCells",
        "maxUvCells", "maxNormalCells", "maxPairCells", "maxAgreementCells", "maxMapEvaluations",
        "maxInjectivityCells", "maxNormalSpans", "maxTangentPositionCells", "length_max_cells",
        "max_sections", "max_steps", "initial_sections", "initial_steps", "preview_sections",
        "preview_steps", "transport_steps", "rootRefinements", "controlCount"] {
        if request.get(budget).is_some_and(|v| v.as_f64().is_some()) {
            request[budget] = json!(1 + rng.below(32));
        }
    }
}

#[test]
fn transport_never_panics_on_generated_requests() {
    let rounds: usize = std::env::var("NURBS_TRANSPORT_MUTATION_ROUNDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(12);
    let trace = std::env::var_os("NURBS_TRANSPORT_TRACE").is_some();
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let mut rng = Lcg(0x7EA5);
    let mut panics = Vec::new();
    let (mut runs, mut accepted, mut deep) = (0, 0, 0);
    let mut record = |text: &str, result: Result<Value, String>, accepted: &mut usize| {
        match result {
            Ok(envelope) => {
                if envelope["ok"] == true {
                    *accepted += 1;
                }
                Some(envelope)
            }
            Err(message) => {
                panics.push(format!("{message} <- {text}"));
                None
            }
        }
    };
    for op in OPS {
        // Complete the request field by field, guided by the dispatcher's
        // own "Invalid <field>" messages, so mutations reach past parsing.
        let mut request = json!({"op": op});
        for _ in 0..16 {
            let text = request.to_string();
            runs += 1;
            if trace {
                eprintln!("TRACE {text}");
            }
            let Some(envelope) = record(&text, run(&text), &mut accepted) else {
                break;
            };
            let Some(name) = rejected_field(&envelope) else {
                break;
            };
            let mut fixed = false;
            for _ in 0..22 {
                let mut candidate = request.clone();
                candidate[name.as_str()] = payload(&mut rng);
                clamp_budgets(&mut candidate, &mut rng);
                let text = candidate.to_string();
                runs += 1;
                if trace {
                    eprintln!("TRACE {text}");
                }
                let Some(answer) = record(&text, run(&text), &mut accepted) else {
                    break;
                };
                if rejected_field(&answer).as_deref() != Some(name.as_str()) {
                    request = candidate;
                    fixed = true;
                    break;
                }
            }
            if !fixed {
                break;
            }
        }
        if request.as_object().is_some_and(|o| o.len() > 1) {
            deep += 1;
        }
        // Mutate the completed request.
        for _ in 0..rounds {
            let mut mutated = request.clone();
            let keys: Vec<String> = mutated
                .as_object()
                .map(|o| o.keys().filter(|k| *k != "op").cloned().collect())
                .unwrap_or_default();
            for _ in 0..1 + rng.below(3) {
                let name = if keys.is_empty() || rng.below(4) == 0 {
                    FIELDS[rng.below(FIELDS.len())].to_string()
                } else {
                    keys[rng.below(keys.len())].clone()
                };
                mutated[name.as_str()] = payload(&mut rng);
            }
            clamp_budgets(&mut mutated, &mut rng);
            let text = mutated.to_string();
            runs += 1;
            if trace {
                eprintln!("TRACE {text}");
            }
            record(&text, run(&text), &mut accepted);
        }
    }
    std::panic::set_hook(hook);
    eprintln!("{runs} requests, {accepted} answered ok, {deep} ops completed past parsing");
    panics.sort();
    panics.dedup();
    assert!(
        panics.is_empty(),
        "{} panics:\n{}",
        panics.len(),
        panics[..panics.len().min(15)].join("\n")
    );
}
