//! Numeric own-geometry graph preparation. Kernel execution remains separate.
use crate::units::{self, ANGLE, LENGTH, Numeric, SCALAR};
use crate::{Error, Result};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};
use value_codec::{Value as J, json};
fn s<'a>(v: &'a J, key: &str) -> &'a str {
    v[key].as_str().unwrap_or("")
}
fn err(path: &str, message: impl Into<String>) -> Error {
    Error::new("invalid_document", path, message)
}
fn valid_id(s: &str) -> bool {
    let b = s.as_bytes();
    !b.is_empty()
        && b.len() <= 32
        && b[0].is_ascii_alphabetic()
        && b.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'_')
}
fn normalize(v: &mut J, schema: &J, path: &str, depth: usize) -> Result<()> {
    if depth > 64 {
        return Err(err(path, "Document nesting exceeds 64."));
    }
    if let Some(choices) = schema
        .get("oneOf")
        .or_else(|| schema.get("anyOf"))
        .and_then(J::as_array)
    {
        // Node operations have a direct discriminator; scalar alternatives dispatch on shape.
        if choices
            .iter()
            .all(|choice| choice["properties"]["op"].get("const").is_some())
        {
            let choice = v.get("op").and_then(J::as_str).and_then(|op| {
                choices
                    .iter()
                    .find(|c| c["properties"]["op"]["const"] == op)
            });
            return match choice {
                Some(choice) => normalize(v, choice, path, depth + 1),
                None => Err(err(
                    &format!("{path}/op"),
                    "Invalid input: no matching discriminator.",
                )),
            };
        }
        // Numeric/parameter unions and the 2D/3D control-point union can be
        // selected without cloning a candidate subtree for speculative validation.
        let matches_shape = |choice: &&J| match s(choice, "type") {
            "number" | "integer" => v.is_number(),
            "object" => v.is_object(),
            "array" => v.as_array().is_some_and(|items| {
                choice["prefixItems"]
                    .as_array()
                    .is_none_or(|prefix| prefix.len() == items.len())
            }),
            _ => true,
        };
        let mut matching = choices.iter().filter(matches_shape);
        if let Some(choice) = matching.next()
            && matching.next().is_none()
        {
            return normalize(v, choice, path, depth + 1);
        }
        for choice in choices {
            let matches = match s(choice, "type") {
                "number" | "integer" => v.is_number(),
                "object" => v.is_object(),
                "array" => v.is_array(),
                _ => true,
            };
            if !matches {
                continue;
            }
            let mut candidate = v.clone();
            if normalize(&mut candidate, choice, path, depth + 1).is_ok() {
                *v = candidate;
                return Ok(());
            }
        }
        return Err(err(path, "Invalid input: no matching schema variant."));
    }
    if let Some(expected) = schema.get("const")
        && v != expected
    {
        return Err(err(path, format!("Expected {expected}.")));
    }
    if let Some(values) = schema["enum"].as_array()
        && !values.contains(v)
    {
        return Err(err(path, "Invalid enum value."));
    }
    match s(schema, "type") {
        "object" => {
            let object = v
                .as_object_mut()
                .ok_or_else(|| err(path, "Expected object."))?;
            let props = schema["properties"]
                .as_object()
                .ok_or_else(|| err(path, "Invalid object schema."))?;
            if schema["additionalProperties"] == false
                && let Some(key) = object.keys().find(|k| !props.contains_key(*k))
            {
                return Err(err(path, format!("Unrecognized key: {key}")));
            }
            for (key, sub) in props {
                if !object.contains_key(key)
                    && let Some(default) = sub.get("default")
                {
                    object.insert(key.clone(), default.clone());
                }
            }
            if let Some(required) = schema["required"].as_array() {
                for key in required {
                    let key = key.as_str().unwrap();
                    if !object.contains_key(key) {
                        return Err(err(&format!("{path}/{key}"), "Required field is missing."));
                    }
                }
            }
            for (key, value) in object {
                if let Some(sub) = props.get(key) {
                    normalize(value, sub, &format!("{path}/{key}"), depth + 1)?;
                }
            }
        }
        "array" => {
            let items = v
                .as_array_mut()
                .ok_or_else(|| err(path, "Expected array."))?;
            if schema["minItems"]
                .as_u64()
                .is_some_and(|n| items.len() < n as usize)
                || schema["maxItems"]
                    .as_u64()
                    .is_some_and(|n| items.len() > n as usize)
            {
                return Err(err(path, "Array length is outside schema bounds."));
            }
            if let Some(prefix) = schema["prefixItems"].as_array() {
                if items.len() != prefix.len() {
                    return Err(err(path, "Invalid tuple length."));
                }
                for (i, (v, sub)) in items.iter_mut().zip(prefix).enumerate() {
                    normalize(v, sub, &format!("{path}/{i}"), depth + 1)?
                }
            } else if schema["items"].is_object() {
                for (i, item) in items.iter_mut().enumerate() {
                    normalize(item, &schema["items"], &format!("{path}/{i}"), depth + 1)?
                }
            }
        }
        "number" | "integer" => {
            let n = v.as_f64().ok_or_else(|| err(path, "Expected number."))?;
            if !n.is_finite()
                || s(schema, "type") == "integer" && n.fract() != 0.0
                || schema["minimum"].as_f64().is_some_and(|min| n < min)
                || schema["maximum"].as_f64().is_some_and(|max| n > max)
            {
                return Err(err(path, "Number is outside schema bounds."));
            }
        }
        "string" => {
            let text = v.as_str().ok_or_else(|| err(path, "Expected string."))?;
            if schema.get("pattern").is_some() && !valid_id(text) {
                return Err(err(path, "Invalid identifier."));
            }
        }
        "boolean" if !v.is_boolean() => return Err(err(path, "Expected boolean.")),
        _ => {}
    }
    Ok(())
}
fn schema() -> &'static J {
    static SCHEMA: OnceLock<J> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        value_codec::from_str(include_str!(
            "../../../docs/languages/modelgraph-nurbs-1.schema.json"
        ))
        .expect("checked-in NURBS schema")
    })
}
pub fn compile(mut document: J) -> Result<J> {
    if crate::emit::json_string(&document).encode_utf16().count() > 250000 {
        return Err(Error::new(
            "document_limit",
            "/",
            "Document exceeds 250000 characters.",
        ));
    }
    normalize(&mut document, schema(), "", 0)?;
    let parameters = document["parameters"].as_array().unwrap();
    let params: BTreeMap<_, _> = parameters
        .iter()
        .map(|p| (s(p, "id"), p["value"].clone()))
        .collect();
    if params.len() != parameters.len() {
        return Err(Error::new(
            "duplicate_parameter",
            "/parameters",
            "Duplicate parameter ID.",
        ));
    }
    fn resolve(value: &J, params: &BTreeMap<&str, J>, path: &str, count: &mut usize) -> Result<J> {
        *count += 1;
        if *count > 30000 {
            return Err(Error::new(
                "value_limit",
                path,
                "Maximum 30000 document values.",
            ));
        }
        match value {
            J::Array(a) => a
                .iter()
                .enumerate()
                .map(|(i, v)| resolve(v, params, &format!("{path}/{i}"), count))
                .collect::<Result<Vec<_>>>()
                .map(J::Array),
            J::Object(o) => {
                if let Some(id) = o.get("param").and_then(J::as_str) {
                    return params.get(id).cloned().ok_or_else(|| {
                        Error::new("unknown_parameter", path, format!("Unknown parameter {id}"))
                    });
                }
                let mut out = value_codec::Map::new();
                for (k, v) in o {
                    out.insert(
                        k.clone(),
                        resolve(v, params, &format!("{path}/{k}"), count)?,
                    );
                }
                Ok(J::Object(out))
            }
            v => Ok(v.clone()),
        }
    }
    let resolved = resolve(&document, &params, "", &mut 0)?;
    let items = resolved["nodes"].as_array().unwrap();
    let nodes: BTreeMap<&str, &J> = items.iter().map(|n| (s(n, "id"), n)).collect();
    if nodes.len() != items.len() {
        return Err(Error::new("duplicate_node", "/nodes", "Duplicate node ID."));
    }
    fn visit<'a>(
        key: &'a str,
        depth: usize,
        nodes: &BTreeMap<&'a str, &'a J>,
        active: &mut BTreeSet<&'a str>,
        heights: &mut BTreeMap<&'a str, usize>,
    ) -> Result<usize> {
        let node = nodes
            .get(key)
            .ok_or_else(|| Error::new("unknown_node", "/nodes", format!("Unknown node {key}")))?;
        let path = format!("/nodes/{key}");
        if active.contains(key) {
            return Err(Error::new("cycle", path, "Cyclic NURBS graph."));
        }
        if let Some(height) = heights.get(key) {
            if depth + height - 1 > 32 {
                return Err(Error::new("depth_limit", path, "Maximum graph depth 32."));
            }
            return Ok(*height);
        }
        if depth > 32 {
            return Err(Error::new("depth_limit", path, "Maximum graph depth 32."));
        }
        active.insert(key);
        let mut refs = if let Some(xs) = node["inputs"].as_array() {
            xs.iter().filter_map(J::as_str).collect::<Vec<_>>()
        } else {
            node["input"].as_str().into_iter().collect()
        };
        if s(node, "op") == "tessellate" && node.get("trim_curves").is_some() {
            refs.push(s(&node["trim_curves"], "outer"));
            if let Some(holes) = node["trim_curves"]["holes"].as_array() {
                refs.extend(holes.iter().filter_map(J::as_str))
            }
        }
        if s(node, "op") == "brep_extrude_curves" {
            let curves = node["loops"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|wire| wire.as_array().unwrap())
                .collect::<Vec<_>>();
            if curves.len() > 254 {
                return Err(Error::new(
                    "reference_limit",
                    format!("{path}/loops"),
                    "Curve extrusion accepts at most 254 curve references across all loops.",
                ));
            }
            refs.extend(curves.into_iter().map(|curve| curve.as_str().unwrap()));
        }
        let mut height = 1;
        for key in refs {
            height = height.max(1 + visit(key, depth + 1, nodes, active, heights)?)
        }
        active.remove(key);
        heights.insert(key, height);
        Ok(height)
    }
    let mut heights = BTreeMap::new();
    visit(
        s(&document, "root"),
        1,
        &nodes,
        &mut BTreeSet::new(),
        &mut heights,
    )?;
    if heights.len() != nodes.len() {
        return Err(Error::new(
            "unreachable_node",
            "/nodes",
            "Every node must be reachable from root.",
        ));
    }
    Ok(json!({"document":document,"resolved_document":resolved,"execution_target":"own-nurbs"}))
}
fn text_error(path: &str, message: &str) -> Error {
    Error::new(
        "text_error",
        path,
        format!("ModelGraph Text {path}: {message}"),
    )
}
fn scalar(
    expr: &J,
    params: &BTreeMap<String, Numeric>,
    path: &str,
    depth: usize,
) -> Result<Numeric> {
    scalar_inner(expr, params, path, depth).map_err(|error| {
        if error.code == "text_error" {
            error
        } else {
            text_error(path, &error.message)
        }
    })
}
fn scalar_inner(
    expr: &J,
    params: &BTreeMap<String, Numeric>,
    path: &str,
    depth: usize,
) -> Result<Numeric> {
    if depth > 64 {
        return Err(text_error(path, "Expression depth exceeds 64"));
    }
    if let Some(n) = expr.as_f64() {
        return Ok(Numeric {
            value: units::field(
                Numeric {
                    value: n,
                    dimension: SCALAR,
                },
                SCALAR,
                path,
                false,
            )?,
            dimension: SCALAR,
        });
    }
    if !expr.is_object() {
        return Err(text_error(path, "Expected a scalar expression"));
    }
    if let Some(id) = expr["param"].as_str() {
        return params
            .get(id)
            .copied()
            .ok_or_else(|| text_error(path, "Unknown parameter"));
    }
    let sub = |v: &J| scalar(v, params, path, depth + 1);
    match s(expr, "op") {
        "checked" => {
            for c in expr["checks"]
                .as_array()
                .ok_or_else(|| text_error(path, "Invalid checks"))?
            {
                sub(c)?;
            }
            sub(&expr["value"])
        }
        "typed" => units::check_type(sub(&expr["value"])?, s(expr, "type"), path),
        "if" => sub(if units::scalar(sub(&expr["condition"])?, path)? != 0.0 {
            &expr["then"]
        } else {
            &expr["else"]
        }),
        "quantity" => units::quantity(
            expr["value"].as_f64().unwrap_or(f64::NAN),
            s(expr, "unit"),
            path,
        ),
        _ => {
            if let Some(args) = expr["args"].as_array()
                && args.len() == 2
            {
                return units::binary(s(expr, "op"), sub(&args[0])?, sub(&args[1])?, path);
            }
            if expr.get("value").is_some() {
                return units::unary(s(expr, "op"), sub(&expr["value"])?, path, false);
            }
            Err(text_error(
                path,
                "This sequence/expression is not supported in a NURBS field",
            ))
        }
    }
}
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
            if let Some(inputs) = node["inputs"].as_array() {
                n["inputs"] = J::Array(
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
            if s(node, "op") == "brep_extrude_curves" {
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
    fn field(
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
            "polygon_profile",
            "polygon_extrude",
            "polygon_sweep",
            "polygon_loft",
            "surface_sweep",
            "surface_loft",
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
            "brep_extrude",
            "brep_extrude_curves",
            "brep_revolve",
            "brep_boolean",
            "brep_chamfer",
            "brep_fillet",
            "brep_tessellate",
            "surface",
            "curve",
            "surface_extrude",
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
        for (key, value) in node.as_object_mut().unwrap() {
            if ["id", "op", "input", "inputs", "operation", "loops"].contains(&key.as_str()) {
                continue;
            }
            if key == "matrix" {
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
            } else {
                let dimension = if [
                    "height",
                    "z_min",
                    "z_max",
                    "outer",
                    "holes",
                    "sections",
                    "path",
                    "max_deviation",
                    "vertices",
                    "center",
                    "half_size",
                    "radius",
                    "bottom_radius",
                    "top_radius",
                    "inner_radius",
                    "outer_radius",
                    "size",
                    "major_radius",
                    "minor_radius",
                    "distance",
                    "min",
                    "max",
                    "control_points",
                    "vector",
                    "origin",
                ]
                .contains(&key.as_str())
                {
                    LENGTH
                } else if key == "angle" {
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
        json!({"language":"modelgraph/nurbs-1","units":"mm","parameters":[],"nodes":lowered,"root":root}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sphere() -> J {
        json!({"id":"Shape","op":"sdf_sphere","center":[0,0,0],"radius":10})
    }
    fn document(nodes: Vec<J>, root: &str) -> J {
        json!({"language":"modelgraph/nurbs-1","units":"mm","nodes":nodes,"root":root})
    }

    #[test]
    fn curve_extrusion_references_participate_in_graph_and_conditional_traversal() {
        let curve = json!({"id":"Curve","op":"curve","degree":1,"knots":[0,0,1,1],"control_points":[[0,0],[1,0]],"weights":[1,1]});
        let mut body = json!({"id":"Body","op":"brep_extrude_curves","loops":[["Curve"]],"z_min":-2,"z_max":3});
        assert!(compile(document(vec![curve.clone(), body.clone()], "Body")).is_ok());
        body["loops"] = json!([["Missing"]]);
        assert_eq!(
            compile(document(vec![curve.clone(), body.clone()], "Body"))
                .unwrap_err()
                .code,
            "unknown_node"
        );
        body["loops"] = json!([["Body"]]);
        assert_eq!(
            compile(document(vec![body.clone()], "Body"))
                .unwrap_err()
                .code,
            "cycle"
        );
        body["loops"] = json!([vec!["Curve"; 128], vec!["Curve"; 127]]);
        assert_eq!(
            compile(document(vec![curve.clone(), body.clone()], "Body"))
                .unwrap_err()
                .code,
            "reference_limit"
        );
        body["loops"] = json!([["Chosen"]]);
        let choice = json!({"id":"Chosen","op":"if","condition":1,"then":"Curve","else":"Missing"});
        let prepared = compile_text(vec![curve, choice, body], &[], "Body".into()).unwrap();
        assert_eq!(
            prepared["document"]["nodes"][0]["loops"],
            json!([["Curve"]])
        );
        assert_eq!(prepared["document"]["nodes"].as_array().unwrap().len(), 2);
        let body = json!({"id":"Empty","op":"brep_extrude_curves","loops":[],"z_min":{"op":"quantity","value":-2,"unit":"mm"},"z_max":{"op":"quantity","value":3,"unit":"mm"}});
        assert_eq!(
            compile_text(vec![body.clone()], &[], "Empty".into()).unwrap()["document"]["nodes"][0]
                ["z_min"],
            -2
        );
        let mut bad = body;
        bad["z_max"]["unit"] = json!("deg");
        assert!(compile_text(vec![bad], &[], "Empty".into()).is_err());
    }

    #[test]
    fn validates_defaults_and_resolves_parameters_without_changing_authoring_document() {
        let input = json!({"language":"modelgraph/nurbs-1","units":"mm","parameters":[{"id":"Radius","value":5}],"nodes":[{"id":"Curve","op":"curve","degree":1,"knots":[0,0,1,1],"control_points":[[0,0],[{"param":"Radius"},1]],"weights":[1,1]},{"id":"Body","op":"surface_extrude","input":"Curve","vector":[0,0,{"param":"Radius"}]}],"root":"Body"});
        let result = compile(input).unwrap();
        assert_eq!(result["document"]["nodes"][0]["periodic"], false);
        assert_eq!(
            result["document"]["nodes"][0]["control_points"][1][0],
            json!({"param":"Radius"})
        );
        assert_eq!(
            result["resolved_document"]["nodes"][0]["control_points"][1][0],
            5
        );
        assert_eq!(
            result["resolved_document"]["nodes"][1]["vector"],
            json!([0, 0, 5])
        );
        assert_eq!(
            compile(document(vec![sphere()], "Shape")).unwrap()["document"]["parameters"],
            json!([])
        );
    }

    #[test]
    fn rejects_bad_shapes_enums_and_scalar_records_without_union_backtracking() {
        let mut shape = sphere();
        shape["radius"] = json!({"param":"Radius","extra":1});
        assert_eq!(
            compile(document(vec![shape], "Shape")).unwrap_err().path,
            "/nodes/0/radius"
        );
        let mut shape = sphere();
        shape["center"] = json!([0, 0]);
        assert_eq!(
            compile(document(vec![shape], "Shape")).unwrap_err().code,
            "invalid_document"
        );
        let mut shape = sphere();
        shape["id"] = json!("invalid-id");
        assert_eq!(
            compile(document(vec![shape], "Shape")).unwrap_err().path,
            "/nodes/0/id"
        );
        let mut shape = sphere();
        shape["op"] = json!("unknown");
        assert_eq!(
            compile(document(vec![shape], "Shape")).unwrap_err().path,
            "/nodes/0/op"
        );
        let mut shape = sphere();
        shape["radius"] = json!(1000001);
        assert_eq!(
            compile(document(vec![shape], "Shape")).unwrap_err().code,
            "invalid_document"
        );
        let input = document(
            vec![
                json!({"id":"A","op":"curve","degree":1,"knots":[0,0,1,1],"control_points":[[0,0,0,0],[1,1]],"weights":[1,1]}),
            ],
            "A",
        );
        assert_eq!(compile(input).unwrap_err().code, "invalid_document");
    }

    #[test]
    fn rejects_duplicate_unknown_and_unreachable_references_with_original_paths() {
        let mut input = document(vec![sphere()], "Shape");
        input["parameters"] = json!([{"id":"R","value":1},{"id":"R","value":2}]);
        assert_eq!(compile(input).unwrap_err().code, "duplicate_parameter");
        let mut shape = sphere();
        shape["radius"] = json!({"param":"Missing"});
        let error = compile(document(vec![shape], "Shape")).unwrap_err();
        assert_eq!(
            (error.code.as_str(), error.path.as_str()),
            ("unknown_parameter", "/nodes/0/radius")
        );
        assert_eq!(
            compile(document(vec![sphere(), sphere()], "Shape"))
                .unwrap_err()
                .code,
            "duplicate_node"
        );
        let unused = json!({"id":"Other","op":"sdf_offset","input":"Shape","distance":1});
        assert_eq!(
            compile(document(vec![sphere(), unused], "Shape"))
                .unwrap_err()
                .code,
            "unreachable_node"
        );
        let cycle = json!({"id":"Cycle","op":"sdf_offset","input":"Cycle","distance":1});
        assert_eq!(
            compile(document(vec![cycle], "Cycle")).unwrap_err().path,
            "/nodes/Cycle"
        );
        assert_eq!(
            compile(document(vec![sphere()], "Missing"))
                .unwrap_err()
                .code,
            "unknown_node"
        );
    }

    #[test]
    fn graph_depth_budget_counts_cached_shared_branches() {
        let mut nodes = vec![sphere()];
        let mut input = "Shape".to_owned();
        for i in 0..31 {
            let id = format!("N{i}");
            nodes.push(json!({"id":id,"op":"sdf_offset","input":input,"distance":1}));
            input = id;
        }
        compile(document(nodes.clone(), &input)).unwrap();
        nodes.push(json!({"id":"Join","op":"sdf_union","inputs":["Shape",input]}));
        assert_eq!(
            compile(document(nodes, "Join")).unwrap_err().code,
            "depth_limit"
        );
    }

    #[test]
    fn value_and_raw_document_budgets_still_apply() {
        let vertices = vec![json!([0, 0, 0]); 6144];
        let triangles = vec![json!([0, 1, 2]); 2048];
        let input = document(
            vec![
                json!({"id":"Mesh","op":"triangle_mesh","vertices":vertices,"triangles":triangles}),
            ],
            "Mesh",
        );
        assert_eq!(compile(input).unwrap_err().code, "value_limit");
        let mut input = document(vec![sphere()], "Shape");
        input["extra"] = json!("x".repeat(250001));
        assert_eq!(compile(input).unwrap_err().code, "document_limit");
    }

    #[test]
    fn text_choices_are_lazy_and_unit_errors_preserve_text_diagnostics() {
        let mut bad = sphere();
        bad["id"] = json!("Bad");
        bad["radius"] = json!({"op":"quantity","value":3,"unit":"deg"});
        let choice = json!({"id":"Choice","op":"if","condition":{"param":"Pick"},"then":"Shape","else":"Bad"});
        let output = compile_text(
            vec![sphere(), bad.clone(), choice],
            &[json!({"id":"Pick","value":1})],
            "Choice".into(),
        )
        .unwrap();
        assert_eq!(output["document"]["root"], "Shape");
        assert_eq!(output["document"]["nodes"].as_array().unwrap().len(), 1);
        let error = compile_text(vec![bad], &[], "Bad".into()).unwrap_err();
        assert_eq!(error.code, "text_error");
        assert!(
            error
                .message
                .starts_with("ModelGraph Text radius: Expected length^1 angle^0")
        );
        let mut arithmetic = sphere();
        arithmetic["radius"] =
            json!({"op":"add","args":[{"op":"quantity","value":1,"unit":"mm"},1]});
        assert!(
            compile_text(vec![arithmetic], &[], "Shape".into())
                .unwrap_err()
                .message
                .starts_with("ModelGraph Text radius: Incompatible dimensions")
        );
    }

    #[test]
    fn text_matrix_translation_uses_lengths_and_linear_part_uses_scalars() {
        let transform = json!({"id":"Moved","op":"transform","input":"Shape","matrix":[[1,0,0,{"op":"quantity","value":2,"unit":"cm"}],[0,1,0,0],[0,0,1,0],[0,0,0,1]]});
        let result = compile_text(vec![sphere(), transform.clone()], &[], "Moved".into()).unwrap();
        assert_eq!(result["document"]["nodes"][1]["matrix"][0][3], 20.0);
        let mut bad = transform;
        bad["matrix"][0][0] = json!({"op":"quantity","value":1,"unit":"mm"});
        assert_eq!(
            compile_text(vec![sphere(), bad], &[], "Moved".into())
                .unwrap_err()
                .path,
            "matrix/0/0"
        );
    }
}
