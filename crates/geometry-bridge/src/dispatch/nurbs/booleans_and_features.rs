//! Booleans, fillets, chamfers, shells, planar edits, mass properties, and healing.
use crate::*;

pub(super) const OPS: &[&str] = &[
    "brep_nurbs_push_face",
    "brep_nurbs_shell",
    "brep_nurbs_exact_analytic_shell",
    "brep_nurbs_close_topology_v1",
    "brep_nurbs_split",
    "brep_nurbs_mass_properties",
    "brep_nurbs_certified_mass_properties",
    "brep_nurbs_certified_freeform_mass_properties",
    "brep_nurbs_authorized_heal_v2",
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

pub(super) fn handle(op: &str, v: Value) -> Result<Value> {
    match op {
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
                .unwrap_or(300_000) as usize,
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
        "brep_nurbs_authorized_heal_v2" => handle_authorized_heal(&v),
        "brep_nurbs_boolean_bezier_le3_v3"
        | "brep_nurbs_boolean_bezier_le3_v4_unequal"
        | "brep_nurbs_boolean_bezier_le3_v4_containment"
        | "brep_nurbs_boolean_bezier_le3_v5_rational"
        | "brep_nurbs_boolean_general"
        | "brep_nurbs_boolean" => handle_boolean(op, &v),
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
        _ => unreachable!("booleans_and_features admits only its OPS"),
    }
}

fn handle_authorized_heal(v: &Value) -> Result<Value> {
    require_exact_fields(v, &["op", "model", "operation"], "heal request")?;
    let model: brep_core::Model = field(v, "model")?;
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

fn operation_str(v: &Value) -> Result<&str> {
    v.get("operation")
        .and_then(Value::as_str)
        .ok_or_else(|| input("Invalid operation"))
}

fn handle_boolean(op: &str, v: &Value) -> Result<Value> {
    match op {
        "brep_nurbs_boolean_bezier_le3_v3" => {
            require_exact_fields(v, &["op", "a", "b", "operation"], "V3 graph Boolean request")?;
            let (model, certificate) = brep_core::nurbs_boolean_graph_patch_v3(
                &field(v, "a")?,
                &field(v, "b")?,
                operation_str(v)?,
            )?;
            curved_graph_boolean_value(model, certificate)
        }
        "brep_nurbs_boolean_bezier_le3_v4_unequal" => {
            require_exact_fields(
                v,
                &["op", "a", "b", "operation"],
                "V4 unequal graph Boolean request",
            )?;
            let (model, certificate) = brep_core::nurbs_boolean_graph_patch_unequal_v4(
                &field(v, "a")?,
                &field(v, "b")?,
                operation_str(v)?,
            )?;
            curved_graph_boolean_value(model, certificate)
        }
        "brep_nurbs_boolean_bezier_le3_v4_containment" => {
            require_exact_fields(
                v,
                &["op", "graph", "cutter", "operation"],
                "V4 containment graph Boolean request",
            )?;
            let (model, certificate) = brep_core::nurbs_boolean_graph_containment_v4(
                &field(v, "graph")?,
                &field(v, "cutter")?,
                operation_str(v)?,
            )?;
            contained_graph_boolean_value(model, certificate)
        }
        "brep_nurbs_boolean_bezier_le3_v5_rational" => {
            require_exact_fields(
                v,
                &["op", "a", "b", "operation"],
                "V5 rational graph Boolean request",
            )?;
            let (model, certificate) = brep_core::nurbs_boolean_rational_graph_patch_v5(
                &field(v, "a")?,
                &field(v, "b")?,
                operation_str(v)?,
            )?;
            curved_graph_boolean_value(model, certificate)
        }
        "brep_nurbs_boolean_general" => {
            require_exact_fields(
                v,
                &["op", "a", "b", "operation"],
                "general NURBS Boolean request",
            )?;
            let (model, certificate) = brep_core::author_general_nurbs_boolean(
                &field(v, "a")?,
                &field(v, "b")?,
                operation_str(v)?,
            )?;
            general_nurbs_boolean_value(model, certificate)
        }
        "brep_nurbs_boolean" => encode(brep_core::boolean(
            &field(v, "a")?,
            &field(v, "b")?,
            operation_str(v)?,
        )?),
        _ => unreachable!("handle_boolean admits only Boolean OPS"),
    }
}
