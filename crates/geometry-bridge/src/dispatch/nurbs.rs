//! Bridge operations of the `nurbs` family.
use crate::*;

const OPS: &[&str] = &[
    "brep_nurbs_sketch_extrude",
    "brep_nurbs_section_loft_source_audit",
    "brep_nurbs_affine_lattice",
    "brep_nurbs_transform",
    "brep_nurbs_workplane",
    "brep_nurbs_box",
    "brep_nurbs_freeform_cuboid",
    "brep_nurbs_freeform_cuboid_bump",
    "brep_nurbs_revolve",
    "brep_nurbs_sphere",
    "brep_nurbs_sphere_exact",
    "brep_implicit_sphere_source_construct",
    "brep_implicit_sphere_source_audit",
    "brep_nurbs_gear",
    "brep_nurbs_torus",
    "brep_nurbs_push_face",
    "brep_nurbs_shell",
    "brep_nurbs_exact_analytic_shell",
    "brep_nurbs_close_topology_v1",
    "brep_nurbs_split",
    "brep_nurbs_mass_properties",
    "brep_nurbs_certified_mass_properties",
    "brep_nurbs_certified_freeform_mass_properties",
    "brep_nurbs_authorized_heal_v2",
    "brep_nurbs_cylinder",
    "brep_nurbs_frustum",
    "brep_nurbs_tube",
    "brep_nurbs_extrude_curves",
    "brep_nurbs_extrude_polygon",
    "brep_nurbs_capped_loft_with_caps",
    "brep_nurbs_natural_section_loft",
    "brep_nurbs_section_loft_surfaces",
    "brep_nurbs_smooth_polygon_station_body",
    "brep_nurbs_smooth_station_walls",
    "brep_nurbs_capped_loft_surfaces",
    "brep_nurbs_periodic_section_loft",
    "brep_nurbs_rational_section_loft",
    "brep_nurbs_progressive_profile_body",
    "brep_nurbs_ruled_loft",
    "brep_nurbs_faceted_loft",
    "brep_nurbs_faceted_sweep",
    "brep_nurbs_faceted_revolve",
    "brep_nurbs_faceted_cylinder",
    "brep_nurbs_faceted_sphere",
    "brep_nurbs_canonical_graph_solid_v3",
    "brep_nurbs_canonical_rational_graph_solid_v5",
    "brep_nurbs_canonical_multispan_graph_solid",
    "brep_nurbs_boolean_bezier_le3_v3",
    "brep_nurbs_boolean_bezier_le3_v4_unequal",
    "brep_nurbs_boolean_bezier_le3_v4_containment",
    "brep_nurbs_boolean_bezier_le3_v5_rational",
    "brep_nurbs_boolean_general",
    "brep_nurbs_boolean",
    "brep_nurbs_chamfer",
    "brep_nurbs_chamfer_edges",
    "brep_nurbs_fillet",
    "brep_nurbs_fillet_edges",
    "brep_nurbs_audited_multi_edge_fillet",
    "brep_nurbs_exact_convex_chamfer",
    "brep_nurbs_constant_fillet_family",
];

pub(crate) fn route(v: Value) -> Routed {
    if !OPS.contains(&v["op"].as_str().unwrap_or("")) {
        return Routed::Unhandled(v);
    }
    Routed::Handled(handle(v))
}

fn handle(v: Value) -> Result<Value> {
    match v["op"].as_str().unwrap_or("") {
        "brep_nurbs_sketch_extrude" => {
            let sketch = v.get("sketch").ok_or_else(|| input("Missing sketch"))?;
            let profile = match sketch.get("analytic") {
                Some(analytic)
                    if analytic.get("kind").and_then(Value::as_str) == Some("circle") =>
                {
                    brep_core::sketch::Profile::Circle {
                        center: field(analytic, "center")?,
                        radius: field(analytic, "radius")?,
                    }
                }
                _ => brep_core::sketch::Profile::Polygon(field(sketch, "points")?),
            };
            let plane = match sketch.get("plane") {
                None | Some(Value::Null) => None,
                Some(plane) => Some([
                    field(plane, "origin")?,
                    field(plane, "u")?,
                    field(plane, "v")?,
                ]),
            };
            encode(brep_core::sketch::extrude(
                profile,
                field(sketch, "closed")?,
                field(&v, "height")?,
                field(&v, "baseZ")?,
                plane,
            )?)
        }
        "brep_nurbs_section_loft_source_audit" => {
            let exact = brep_core::analytic::section_loft_source_matches(
                &field(&v, "model")?,
                &field::<Vec<Vec<Vec<Curve>>>>(&v, "sections")?,
                field(&v, "closed")?,
            )?;
            Ok(json!({"geometryAndTopologyIdentical":exact,"globalEmbeddingCertified":false}))
        }
        "brep_nurbs_affine_lattice" => {
            let report = brep_core::affine_lattice::place(
                &field(&v, "model")?,
                field(&v, "matrix")?,
                field(&v, "quantum")?,
                field(&v, "maxWork")?,
            )?;
            Ok(
                json!({"model":report.model,"operatorNormUpper":report.operator_norm_upper,"arithmeticErrorUpper":report.arithmetic_error_upper,"work":report.work,"reason":report.reason}),
            )
        }
        "brep_nurbs_transform" => encode(brep_core::transform::affine(
            &field(&v, "model")?,
            field(&v, "matrix")?,
        )?),
        "brep_nurbs_workplane" => encode(brep_core::transform::workplane(
            &field(&v, "model")?,
            field(&v, "origin")?,
            field(&v, "u")?,
            field(&v, "v")?,
            field(&v, "offset")?,
        )?),
        "brep_nurbs_box" => encode(brep_core::cuboid(field(&v, "min")?, field(&v, "max")?)?),
        "brep_nurbs_freeform_cuboid" => encode(brep_core::freeform_cuboid_solid(
            field(&v, "min")?,
            field(&v, "max")?,
        )?),
        "brep_nurbs_freeform_cuboid_bump" => encode(brep_core::freeform_cuboid_with_bump_face(
            field(&v, "min")?,
            field(&v, "max")?,
        )?),
        "brep_nurbs_revolve" => encode(brep_core::revolve_angle(
            &field::<Vec<[f64; 2]>>(&v, "profile")?,
            v.get("angleDegrees")
                .and_then(Value::as_f64)
                .unwrap_or(360.),
        )?),
        "brep_nurbs_sphere" | "brep_nurbs_sphere_exact" => cad_face_contacts::sphere(v),
        "brep_implicit_sphere_source_construct" => cad_implicit_body::sphere(v, false),
        "brep_implicit_sphere_source_audit" => cad_implicit_body::sphere(v, true),
        "brep_nurbs_gear" => {
            let number = |name: &str, default: f64| -> f64 {
                v.get(name).and_then(Value::as_f64).unwrap_or(default)
            };
            let flag = |name: &str| v.get(name).and_then(Value::as_bool).unwrap_or(false);
            let teeth: f64 = field(&v, "teeth")?;
            let spec = brep_core::GearSpec {
                module: field(&v, "module")?,
                teeth: teeth.round().max(0.) as usize,
                pressure_angle_deg: number("pressureAngle", 20.),
                height: field(&v, "height")?,
                helix_angle_deg: number("helixAngle", 0.),
                herringbone: flag("herringbone"),
                bore: number("bore", 0.),
                internal: flag("internal"),
                rim_width: number("rimWidth", 2.),
                clearance: number("clearance", 0.25),
                backlash: number("backlash", 0.),
            };
            encode(brep_core::gear(&spec)?)
        }
        "brep_nurbs_torus" => encode(brep_core::torus(
            field(&v, "majorRadius")?,
            field(&v, "minorRadius")?,
        )?),
        "brep_nurbs_push_face" => encode(brep_core::operations::push_planar_face(
            &field(&v, "model")?,
            field(&v, "face")?,
            field(&v, "distance")?,
        )?),
        "brep_nurbs_shell" => encode(brep_core::operations::shell_planar(
            &field(&v, "model")?,
            &field::<Vec<usize>>(&v, "openings")?,
            field(&v, "thickness")?,
        )?),
        "brep_nurbs_exact_analytic_shell" => {
            require_exact_fields(
                &v,
                &["op", "model", "openings", "thickness", "direction"],
                "exact analytic shell request",
            )?;
            encode(brep_core::exact_analytic_shell(
                &field(&v, "model")?,
                &field::<Vec<usize>>(&v, "openings")?,
                field(&v, "thickness")?,
                v.get("direction")
                    .and_then(Value::as_str)
                    .ok_or_else(|| input("Invalid shell direction"))?,
            )?)
        }
        "brep_nurbs_close_topology_v1" => close_topology_audit_value(&v),
        "brep_nurbs_split" => encode(brep_core::operations::split_planar(
            &field(&v, "model")?,
            field(&v, "normal")?,
            field(&v, "offset")?,
        )?),
        "brep_nurbs_mass_properties" => encode(brep_core::analysis::mass_properties(
            &field::<brep_core::Model>(&v, "model")?,
            v.get("relativeTolerance")
                .and_then(Value::as_f64)
                .unwrap_or(1e-7),
            v.get("maxEvaluations")
                .and_then(Value::as_u64)
                .unwrap_or(300000) as usize,
        )?),
        "brep_nurbs_certified_mass_properties" => {
            let model: brep_core::Model = field(&v, "model")?;
            encode(brep_core::analysis::certified_mass_properties(&model)?)
        }
        "brep_nurbs_certified_freeform_mass_properties" => {
            let model: brep_core::Model = field(&v, "model")?;
            encode(brep_core::analysis::certified_freeform_mass_properties(
                &model,
            )?)
        }
        "brep_nurbs_authorized_heal_v2" => {
            require_exact_fields(&v, &["op", "model", "operation"], "heal request")?;
            let model: brep_core::Model = field(&v, "model")?;
            let operation = v
                .get("operation")
                .ok_or_else(|| input("Missing operation"))?;
            let kind = operation
                .get("kind")
                .and_then(Value::as_str)
                .ok_or_else(|| input("Invalid operation kind"))?;
            match kind {
                "endpointSnap" => {
                    require_exact_fields(operation, &["kind", "vertex", "to"], "endpoint snap")?;
                    encode(brep_core::transactions::authorized_heal_endpoint(
                        &model,
                        field(operation, "vertex")?,
                        field(operation, "to")?,
                    )?)
                }
                "rationalRefit" => {
                    require_exact_fields(
                        operation,
                        &["kind", "edge", "replacement"],
                        "rational refit",
                    )?;
                    encode(brep_core::transactions::authorized_heal_refit(
                        &model,
                        field(operation, "edge")?,
                        field(operation, "replacement")?,
                    )?)
                }
                _ => Err(input("Unsupported authorized heal operation")),
            }
        }
        "brep_nurbs_cylinder" => encode(brep_core::cylinder(
            field(&v, "radius")?,
            field(&v, "height")?,
        )?),
        "brep_nurbs_frustum" => encode(brep_core::frustum(
            field(&v, "bottomRadius")?,
            field(&v, "topRadius")?,
            field(&v, "height")?,
        )?),
        "brep_nurbs_tube" => encode(brep_core::tube(
            field(&v, "outerRadius")?,
            field(&v, "innerRadius")?,
            field(&v, "height")?,
        )?),
        "brep_nurbs_extrude_curves" => encode(brep_core::prism::extrude(
            &field::<Vec<Vec<Curve>>>(&v, "loops")?,
            field(&v, "zMin")?,
            field(&v, "zMax")?,
        )?),
        "brep_nurbs_extrude_polygon" => {
            let holes = v
                .get("holes")
                .map(|_| field::<Vec<Vec<[f64; 2]>>>(&v, "holes"))
                .transpose()?
                .unwrap_or_default();
            encode(brep_core::extrude_polygon_with_holes(
                &field::<Vec<[f64; 2]>>(&v, "profile")?,
                &holes,
                field(&v, "zMin")?,
                field(&v, "zMax")?,
            )?)
        }
        "brep_nurbs_capped_loft_with_caps" => {
            let definitions: Vec<Value> = field(&v, "caps")?;
            if definitions.len() != 2 {
                return Err(input("Loft requires two authored caps"));
            }
            let caps = definitions
                .iter()
                .map(|c| {
                    Ok(brep_core::LoftCap {
                        surface: field(c, "surface")?,
                        trims: field(c, "trims")?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let l: Value = field(&v, "embeddingLimits")?;
            let limits = brep_core::boundary_embedding::Limits {
                exact_work: field(&l, "exactWork")?,
                trim_pairs: field(&l, "trimPairs")?,
                trim_cells: field(&l, "trimCells")?,
                trim_domain_cells: field(&l, "trimDomainCells")?,
                spans: field(&l, "spans")?,
                contacts: brep_core::face_contacts::Limits {
                    pairs: field(&l, "facePairs")?,
                    cells: field(&l, "faceCells")?,
                    domain_cells: field(&l, "faceDomainCells")?,
                    cells_per_pair: field(&l, "faceCellsPerPair")?,
                    domain_cells_per_pair: field(&l, "faceDomainCellsPerPair")?,
                },
            };
            encode(brep_core::capped_loft_with_caps_checked(
                &field::<Vec<Vec<Curve>>>(&v, "start")?,
                &field::<Vec<Vec<Curve>>>(&v, "end")?,
                &field::<Vec<Vec<Surface>>>(&v, "sides")?,
                [&caps[0], &caps[1]],
                field(&v, "toleranceUv")?,
                limits,
            )?)
        }
        "brep_nurbs_natural_section_loft" => encode(brep_core::natural_section_loft(
            &field::<Vec<Vec<Vec<Curve>>>>(&v, "sections")?,
            &field::<Vec<f64>>(&v, "parameters")?,
        )?),
        operation @ ("brep_nurbs_smooth_polygon_station_body"
        | "brep_nurbs_section_loft_surfaces"
        | "brep_nurbs_rational_section_loft") => {
            crate::polygon_station_body::dispatch(operation, &v)
        }
        "brep_nurbs_smooth_station_walls" => {
            let report = brep_core::analytic::smooth_station_walls(
                &field::<Vec<Vec<Vec<Curve>>>>(&v, "sections")?,
                &field::<Vec<usize>>(&v, "sharp")?,
                field(&v, "closed")?,
                field(&v, "quantum")?,
                field(&v, "tolerance")?,
                field(&v, "maxWork")?,
            )?;
            encode(json!({"sides": report.sides,
                "wallDisplacementUpper": report.wall_displacement_upper,
                "work": report.work, "reason": report.reason}))
        }
        "brep_nurbs_capped_loft_surfaces" => encode(brep_core::capped_loft_surfaces(
            &field::<Vec<Vec<Curve>>>(&v, "start")?,
            &field::<Vec<Vec<Curve>>>(&v, "end")?,
            &field::<Vec<Vec<Surface>>>(&v, "sides")?,
        )?),
        "brep_nurbs_periodic_section_loft" => encode(brep_core::periodic_section_loft(&field::<
            Vec<Vec<Vec<Curve>>>,
        >(
            &v, "sections",
        )?)?),
        "brep_nurbs_progressive_profile_body" => {
            use nurbs_core::progressive_sweep::{Options, Orientation, Spacing};
            let orientation = match field::<String>(&v, "orientation")?.as_str() {
                "rmf" => Orientation::RotationMinimizing,
                "fixed" | "authored" => Orientation::Fixed,
                "fixed_normal" => Orientation::FixedNormal,
                "frenet" => Orientation::Frenet,
                "corrected_frenet" => Orientation::CorrectedFrenet,
                _ => {
                    return Err(Error::new(
                        "BREP_RATIONAL_SWEEP_REFUSED",
                        "Unknown orientation",
                    ));
                }
            };
            let spacing = match field::<String>(&v, "spacing")?.as_str() {
                "parameter" => Spacing::Parameter,
                "arc_length" => Spacing::ArcLength {
                    tolerance: field(&v, "length_tolerance")?,
                    max_cells: field(&v, "length_max_cells")?,
                },
                _ => return Err(Error::new("BREP_RATIONAL_SWEEP_REFUSED", "Unknown spacing")),
            };
            let options = Options {
                normal: field(&v, "normal")?,
                orientation,
                spacing,
                initial_sections: field(&v, "initial_sections")?,
                max_sections: field(&v, "max_sections")?,
                max_deviation: field(&v, "max_deviation")?,
            };
            let loops = field::<Vec<Vec<Curve>>>(&v, "loops")?;
            let path = field::<Curve>(&v, "path")?;
            let scale = field::<Curve>(&v, "scale")?;
            let twist = field::<Curve>(&v, "twist")?;
            let axes = optional_field::<Curve>(&v, "axis_scale")?;
            let center = optional_field::<Curve>(&v, "center_law")?;
            let guide = optional_field::<Curve>(&v, "orientation_guide")?;
            let contact = optional_field::<f64>(&v, "contact_parameter")?;
            let contact_profile = optional_field::<usize>(&v, "contact_profile")?;
            if contact.is_some() && guide.is_none()
                || contact_profile.is_some() && contact.is_none()
            {
                return Err(Error::new(
                    "BREP_RATIONAL_SWEEP_REFUSED",
                    "Contact anchor requires orientation guide and parameter",
                ));
            }
            let authored = field::<String>(&v, "orientation")? == "authored";
            if authored && guide.is_some() {
                return Err(Error::new(
                    "BREP_RATIONAL_SWEEP_REFUSED",
                    "Orientation guide cannot be combined with authored frames",
                ));
            }
            let frame_axis = if authored {
                Some(field::<Curve>(&v, "frame_axis")?)
            } else {
                None
            };
            let frame_normal = if authored {
                Some(field::<Curve>(&v, "frame_normal")?)
            } else {
                None
            };
            let default_axes = nurbs_core::progressive_sweep::constant_vector_law([1.; 3])?;
            let default_center = nurbs_core::progressive_sweep::constant_vector_law([0.; 3])?;
            let affine = if authored || guide.is_some() || axes.is_some() || center.is_some() {
                Some((
                    axes.as_ref().unwrap_or(&default_axes),
                    center.as_ref().unwrap_or(&default_center),
                ))
            } else {
                None
            };
            let options = if authored {
                Options {
                    orientation: Orientation::Fixed,
                    ..options
                }
            } else {
                options
            };
            let tolerance = optional_field::<f64>(&v, "cap_correction_tolerance")?;
            let quantum = optional_field::<f64>(&v, "cap_correction_quantum")?;
            let max_work = optional_field::<u64>(&v, "cap_correction_max_work")?;
            if tolerance.is_none() && (quantum.is_some() || max_work.is_some()) {
                return Err(Error::new(
                    "BREP_RATIONAL_SWEEP_REFUSED",
                    "Cap correction requires a displacement tolerance",
                ));
            }
            let correction =
                tolerance.map(|tolerance| brep_core::analytic::EndpointCapCorrection {
                    tolerance,
                    quantum: quantum.unwrap_or(2_f64.powi(-40)),
                    max_work: max_work.unwrap_or(1000000),
                });
            let rmf_steps = optional_field::<usize>(&v, "rmf_transport_steps")?;
            let rmf_cells = optional_field::<usize>(&v, "error_max_cells")?;
            let rmf_products = optional_field::<usize>(&v, "error_max_products")?;
            let rmf_policy = match (rmf_steps, rmf_cells, rmf_products) {
                (None, None, None) => None,
                (Some(steps), Some(cells), Some(products))
                    if field::<String>(&v, "orientation")? == "rmf" =>
                {
                    Some((steps, cells, products))
                }
                _ => {
                    return Err(Error::new(
                        "BREP_RATIONAL_SWEEP_REFUSED",
                        "Spatial RMF policy requires unguided RMF and three proof budgets",
                    ));
                }
            };
            let result = brep_core::analytic::progressive_profile_body_with_rmf_policy(
                &loops,
                &path,
                &scale,
                &twist,
                affine,
                frame_axis.as_ref().zip(frame_normal.as_ref()),
                guide
                    .as_ref()
                    .map(|g| (g, contact.map(|p| (contact_profile.unwrap_or(0), p)))),
                options,
                correction,
                rmf_policy,
            )?;
            let retained_caps=result.retained_caps.as_ref().map(|r|json!({"exact":r.exact,
                "capErrorUpper":if r.exact {Some(0.)}else{None},"exactWork":r.exact_work,"inspectedEdges":r.inspected_edges,"reason":r.reason,
                "scope":"constructor-owned-retained-endpoint-regions","continuousBound":false,"globalEmbeddingCertified":false}));
            let cap_projection = result.cap_projection.as_ref().map(|r| {
                json!({
                "idealCapDomainsCertified":r.original.domains.ideal_endpoint_domains_certified,
                "normalDots":r.normal_dots,"reversesOrientation":r.reverses_orientation,
                "cells":r.cells,"exactWork":r.exact_work,"reason":r.reason,
                "scope":"constructor-owned-endpoint-plane-projection"})
            });
            let mut output = encode(
                json!({"model":result.model,"approximation":result.approximation,"retainedCaps":retained_caps,
                "capProjection":cap_projection,"filledCapErrorUpper":result.filled_cap_error_upper,
                "capCorrectionErrorUpper":result.cap_correction_error_upper,
                "bodyDecompositionErrorUpper":result.body_decomposition_error_upper,
                "bodyDecompositionProducts":result.body_decomposition_products,
                "retainedWalls":{"certified":result.retained_walls.certified,"faceCoverageCertified":result.retained_walls.face_coverage_certified,
                    "coefficientFamilyCertified":result.retained_walls.coefficient_family_certified,"inspectedFaces":result.retained_walls.inspected_faces,
                    "exactWork":result.retained_walls.exact_work,"reason":result.retained_walls.reason},
                "boundaryErrorUpper":result.boundary_error_upper,"boundaryContinuousBound":result.boundary_error_upper.is_some(),
                "boundaryErrorWithinBudget":result.boundary_error_within_budget,
                "boundaryErrorScope":"constructor-owned-retained-wall-and-cap-union","globalEmbeddingCertified":false}),
            )?;
            if let Some((_, cells, products)) = rmf_policy {
                output["approximation"]["report"]["errorCertificateMaxCells"] = json!(cells);
                output["approximation"]["report"]["decompositionMaxProducts"] = json!(products);
                if let Some(levels) = output["approximation"]["levels"].as_array_mut() {
                    for level in levels {
                        level["errorCertificateMaxCells"] = json!(cells);
                        level["decompositionMaxProducts"] = json!(products);
                    }
                }
            }
            Ok(output)
        }
        "brep_nurbs_ruled_loft" => encode(brep_core::ruled_loft(&field::<Vec<Vec<[f64; 3]>>>(
            &v, "sections",
        )?)?),
        "brep_nurbs_faceted_loft" => encode(brep_core::faceted_loft(
            &field::<Vec<Vec<[f64; 3]>>>(&v, "sections")?,
        )?),
        "brep_nurbs_faceted_sweep" => encode(brep_core::faceted_sweep(
            &field::<Vec<[f64; 2]>>(&v, "profile")?,
            &field::<Vec<[f64; 3]>>(&v, "path")?,
            field(&v, "up")?,
        )?),
        "brep_nurbs_faceted_revolve" => encode(brep_core::faceted_revolve(
            &field::<Vec<[f64; 2]>>(&v, "profile")?,
            field(&v, "segments")?,
        )?),
        "brep_nurbs_faceted_cylinder" => encode(brep_core::faceted_cylinder(
            field(&v, "radius")?,
            field(&v, "height")?,
            field(&v, "segments")?,
        )?),
        "brep_nurbs_faceted_sphere" => encode(brep_core::faceted_sphere(
            field(&v, "radius")?,
            field(&v, "radialSegments")?,
            field(&v, "latitudeSegments")?,
        )?),
        "brep_nurbs_canonical_graph_solid_v3" => encode(brep_core::canonical_bezier_graph_solid(
            field(&v, "degreeU")?,
            field(&v, "degreeV")?,
        )?),
        "brep_nurbs_canonical_rational_graph_solid_v5" => {
            encode(brep_core::canonical_rational_graph_solid(
                field(&v, "degreeU")?,
                field(&v, "degreeV")?,
            )?)
        }
        "brep_nurbs_canonical_multispan_graph_solid" => {
            require_exact_fields(&v, &["op", "spansU", "spansV"], "multispan graph request")?;
            encode(brep_core::canonical_multispan_graph_solid(
                field(&v, "spansU")?,
                field(&v, "spansV")?,
            )?)
        }
        "brep_nurbs_boolean_bezier_le3_v3" => {
            require_exact_fields(
                &v,
                &["op", "a", "b", "operation"],
                "V3 graph Boolean request",
            )?;
            let a: brep_core::Model = field(&v, "a")?;
            let b: brep_core::Model = field(&v, "b")?;
            let operation = v
                .get("operation")
                .and_then(Value::as_str)
                .ok_or_else(|| input("Invalid operation"))?;
            let (model, certificate) = brep_core::nurbs_boolean_graph_patch_v3(&a, &b, operation)?;
            curved_graph_boolean_value(model, certificate)
        }
        "brep_nurbs_boolean_bezier_le3_v4_unequal" => {
            require_exact_fields(
                &v,
                &["op", "a", "b", "operation"],
                "V4 unequal graph Boolean request",
            )?;
            let (model, certificate) = brep_core::nurbs_boolean_graph_patch_unequal_v4(
                &field(&v, "a")?,
                &field(&v, "b")?,
                v.get("operation")
                    .and_then(Value::as_str)
                    .ok_or_else(|| input("Invalid operation"))?,
            )?;
            curved_graph_boolean_value(model, certificate)
        }
        "brep_nurbs_boolean_bezier_le3_v4_containment" => {
            require_exact_fields(
                &v,
                &["op", "graph", "cutter", "operation"],
                "V4 containment graph Boolean request",
            )?;
            let (model, certificate) = brep_core::nurbs_boolean_graph_containment_v4(
                &field(&v, "graph")?,
                &field(&v, "cutter")?,
                v.get("operation")
                    .and_then(Value::as_str)
                    .ok_or_else(|| input("Invalid operation"))?,
            )?;
            contained_graph_boolean_value(model, certificate)
        }
        "brep_nurbs_boolean_bezier_le3_v5_rational" => {
            require_exact_fields(
                &v,
                &["op", "a", "b", "operation"],
                "V5 rational graph Boolean request",
            )?;
            let (model, certificate) = brep_core::nurbs_boolean_rational_graph_patch_v5(
                &field(&v, "a")?,
                &field(&v, "b")?,
                v.get("operation")
                    .and_then(Value::as_str)
                    .ok_or_else(|| input("Invalid operation"))?,
            )?;
            curved_graph_boolean_value(model, certificate)
        }
        "brep_nurbs_boolean_general" => {
            require_exact_fields(
                &v,
                &["op", "a", "b", "operation"],
                "general NURBS Boolean request",
            )?;
            let (model, certificate) = brep_core::author_general_nurbs_boolean(
                &field(&v, "a")?,
                &field(&v, "b")?,
                v.get("operation")
                    .and_then(Value::as_str)
                    .ok_or_else(|| input("Invalid operation"))?,
            )?;
            general_nurbs_boolean_value(model, certificate)
        }
        "brep_nurbs_boolean" => encode(brep_core::boolean(
            &field(&v, "a")?,
            &field(&v, "b")?,
            v.get("operation")
                .and_then(|value| value.as_str())
                .ok_or_else(|| input("Invalid operation"))?,
        )?),
        "brep_nurbs_chamfer" => encode(brep_core::chamfer(
            &field(&v, "model")?,
            field(&v, "edge")?,
            field(&v, "size")?,
        )?),
        "brep_nurbs_chamfer_edges" => encode(brep_core::chamfer_edges(
            &field(&v, "model")?,
            &field::<Vec<usize>>(&v, "edges")?,
            field(&v, "size")?,
        )?),
        "brep_nurbs_fillet" => encode(brep_core::fillet(
            &field(&v, "model")?,
            field(&v, "edge")?,
            field(&v, "radius")?,
            field(&v, "segments")?,
        )?),
        "brep_nurbs_fillet_edges" => encode(brep_core::fillet_edges(
            &field(&v, "model")?,
            &field::<Vec<usize>>(&v, "edges")?,
            field(&v, "radius")?,
            field(&v, "segments")?,
        )?),
        "brep_nurbs_audited_multi_edge_fillet" => {
            let model: brep_core::Model = field(&v, "model")?;
            encode(brep_core::audited_multi_edge_fillet(
                &model,
                &field::<Vec<usize>>(&v, "edges")?,
                field(&v, "radius")?,
            )?)
        }
        "brep_nurbs_exact_convex_chamfer" => {
            require_exact_fields(
                &v,
                &["op", "model", "edges", "distance"],
                "exact chamfer request",
            )?;
            encode(brep_core::exact_convex_chamfer(
                &field(&v, "model")?,
                &field::<Vec<usize>>(&v, "edges")?,
                field(&v, "distance")?,
            )?)
        }
        "brep_nurbs_constant_fillet_family" => encode(
            brep_core::feature_family::constant_fillet_family(
                &field(&v, "model")?,
                &field::<Vec<usize>>(&v, "edges")?,
            )?
            .name(),
        ),
        _ => unreachable!("route admits only OPS"),
    }
}
