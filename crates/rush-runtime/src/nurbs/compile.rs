use super::*;

pub(super) fn s<'a>(v: &'a J, key: &str) -> &'a str {
    v[key].as_str().unwrap_or("")
}
pub(super) fn err(path: &str, message: impl Into<String>) -> Error {
    Error::new("invalid_document", path, message)
}
pub(super) fn valid_id(s: &str) -> bool {
    let b = s.as_bytes();
    !b.is_empty()
        && b.len() <= 32
        && b[0].is_ascii_alphabetic()
        && b.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'_')
}
pub(super) fn normalize(v: &mut J, schema: &J, path: &str, depth: usize) -> Result<()> {
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
            "null" => v.is_null(),
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
            "null" => v.is_null(),
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
        // Small numeric enum literals are exactly representable as binary64;
        // source scalar evaluation may encode the same value as a float.
        && !(values.iter().all(|e|e.as_f64().is_some_and(|x|x.is_finite() && x.abs()<=16384.))
            && values.iter().any(|e|e.as_f64()==v.as_f64()))
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
        "null" if !v.is_null() => return Err(err(path, "Expected null.")),
        "boolean" if !v.is_boolean() => return Err(err(path, "Expected boolean.")),
        _ => {}
    }
    Ok(())
}
pub(super) fn schema() -> &'static J {
    static SCHEMA: OnceLock<J> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        value_codec::from_str(include_str!(
            "../../../../docs/languages/rush-nurbs-1.schema.json"
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
    pub(super) fn resolve(value: &J, params: &BTreeMap<&str, J>, path: &str, count: &mut usize) -> Result<J> {
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
    pub(super) fn visit<'a>(
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
        if ["progressive_sweep","brep_progressive_sweep","brep_progressive_miter_sweep"].contains(&s(node,"op")) {if let Some(id)=node["orientation_guide"].as_str(){refs.push(id);}}
        if s(node,"op")=="ribbon_surface" {refs.push(s(node,"width_law"));}
        if s(node,"op")=="variable_pipe_surface" {refs.push(s(node,"radius_law"));}
        if let Some(maps)=node.get("section_mappings") {
            if maps.as_array().map(Vec::len)!=node["inputs"].as_array().map(Vec::len) {
                return Err(err(&path,"Loft needs one mapping entry per section"));
            }
            if s(node,"op")=="guided_loft_surface" && s(node,"construction")!="cartesian" {
                return Err(err(&path,"Guided section maps require Cartesian construction"));
            }
        }
        if ["guided_loft_surface","auto_guided_loft_surface"].contains(&s(node,"op")) {
            if node.get("start_tangents").is_some()!=node.get("end_tangents").is_some() {
                return Err(err(&path,"Guided loft requires both endpoint tangent fields"));
            }
            let cartesian = s(node,"construction") == "cartesian";
            if !cartesian && ["error_budget","max_cells","max_map_evaluations"].iter().any(|k| node.get(*k).is_some()) {
                return Err(err(&path,"Numerical loft audit options require Cartesian construction"));
            }
            if s(node,"op") == "auto_guided_loft_surface" && !cartesian && node.get("start_tangents").is_some() {
                return Err(err(&path,"Automatic authored tangents require Cartesian construction"));
            }
            if s(node,"op") == "auto_guided_loft_surface" && node.get("error_budget").is_some() {
                return Err(err(&path,"Automatic Cartesian loft uses budget, not error_budget"));
            }
            if let Some(xs)=node["guides"].as_array(){refs.extend(xs.iter().filter_map(J::as_str));}
        }
        if ["loft_match_surface","brep_natural_loft","brep_capped_loft"].contains(&s(node,"op")) {
            fn collect<'a>(value:&'a J,refs:&mut Vec<&'a str>){
                if let Some(id)=value.as_str(){refs.push(id);}
                else if let Some(items)=value.as_array(){for item in items{collect(item,refs);}}
            }
            for field in ["sections","guides","start_reference","end_reference","start","end","sides","cap_surfaces","cap_trims"] {
                if let Some(value)=node.get(field){collect(value,&mut refs);}
            }
        }
        if s(node,"op")=="gordon_surface" {
            for key in ["u_curves","v_curves"] {
                if let Some(xs)=node[key].as_array(){refs.extend(xs.iter().filter_map(J::as_str));}
            }
        }
        if s(node, "op") == "tessellate" && node.get("trim_curves").is_some() {
            refs.push(s(&node["trim_curves"], "outer"));
            if let Some(holes) = node["trim_curves"]["holes"].as_array() {
                refs.extend(holes.iter().filter_map(J::as_str))
            }
        }
        if ["brep_extrude_curves","brep_progressive_sweep","brep_miter_sweep","brep_progressive_miter_sweep"].contains(&s(node,"op")) {
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
pub(super) fn text_error(path: &str, message: &str) -> Error {
    Error::new(
        "text_error",
        path,
        format!("Rush {path}: {message}"),
    )
}
pub(super) fn scalar(
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
pub(super) fn scalar_inner(
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
