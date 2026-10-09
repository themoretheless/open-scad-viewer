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
fn schema() -> &'static J {
    static SCHEMA: OnceLock<J> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        value_codec::from_str(include_str!(
            "../../../docs/languages/rush-nurbs-1.schema.json"
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
fn text_error(path: &str, message: &str) -> Error {
    Error::new(
        "text_error",
        path,
        format!("Rush {path}: {message}"),
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
            if ["brep_extrude_curves","brep_progressive_sweep","brep_miter_sweep","brep_progressive_miter_sweep"].contains(&s(node,"op")) {
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
            "sphere_surface", "cylinder_surface", "cone_surface",
            "plane_patch", "bilinear_patch", "bezier_surface",
            "bezier_curve", "curve_compose", "round_polyline_curve", "transition_polyline_curve", "brep_miter_sweep","brep_progressive_miter_sweep",
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
            "ribbon_surface", "variable_pipe_surface", "pipe_surface", "screw_surface", "clothoid_curve", "spherical_spiral_curve", "toroidal_spiral_curve", "torus_knot_curve", "helicoid_patches", "circle_rectangle_transition", "ellipse_transition_surface", "circle_transition_surface", "helicoid_surface", "catenoid_patches", "catenoid_surface", "catenary_curve", "archimedean_spiral_curve", "epicycloid_curve", "hypocycloid_curve", "trochoid_curve", "cycloid_curve", "lissajous_curve", "logarithmic_spiral_curve", "involute_curve", "elliptic_helix_curve", "variable_pitch_helix_curve", "conical_helix_curve", "helix_curve", "surface_sweep", "brep_progressive_sweep", "progressive_sweep", "profile_sweep", "framed_sweep", "scaled_sweep", "two_guide_sweep", "twist_sweep",
            "surface_loft",
            "ruled_surface",
            "coons_patch",
            "guided_loft_surface", "auto_guided_loft_surface", "loft_match_surface", "brep_natural_loft", "brep_capped_loft", "control_tangent_loft_surface", "clamped_loft_surface", "boundary_fill", "triangular_patch", "gordon_surface", "closed_loft_surface", "natural_loft_surface", "grid_spline_surface", "hermite_patch", "closed_spline_curve", "clamped_spline_curve", "natural_spline_curve", "hermite_curve", "formula_curve", "formula_surface",
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
            "brep_gear",
            "brep_extrude",
            "brep_extrude_curves",
            "brep_revolve",
            "brep_boolean",
            "brep_chamfer",
            "brep_fillet",
            "brep_tessellate",
            "brep_smooth_miter_stations",
            "surface",
            "curve",
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
        let line_coordinates=s(&node,"op")=="line_curve";
        let scaled_sweep=["scaled_sweep","profile_sweep","progressive_sweep","brep_progressive_sweep","brep_progressive_miter_sweep"].contains(&s(&node,"op"));
        let catenary=s(&node,"op")=="catenary_curve";
        let circle_rectangle=s(&node,"op")=="circle_rectangle_transition";
        let ellipse_transition=s(&node,"op")=="ellipse_transition_surface";
        let circle_transition=s(&node,"op")=="circle_transition_surface";
        let catenoid=["catenoid_surface","catenoid_patches"].contains(&s(&node,"op"));
        let clothoid=s(&node,"op")=="clothoid_curve";
        let pipe=["profile_sweep","framed_sweep","pipe_surface","variable_pipe_surface","ribbon_surface"].contains(&s(&node,"op"));
        let loft_refs=["loft_match_surface","brep_natural_loft","brep_capped_loft"].contains(&s(&node,"op"));
        for (key, value) in node.as_object_mut().unwrap() {
            if loft_refs && ["sections","guides","start_reference","end_reference","start","end","sides","start_boundary","end_boundary","start_reverse","end_reverse","cap_surfaces","cap_trims"].contains(&key.as_str()){continue;}
            if ["id", "op", "orientation", "spacing", "width_law", "radius_law", "orientation_guide", "input", "inputs", "u_curves", "v_curves", "guides", "operation", "loops", "lower", "closed", "periodic", "construction", "cap_correction_authored_frame"].contains(&key.as_str()) {
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
                let dimension = if progressive_sweep && key=="length_tolerance" {LENGTH} else if circle_rectangle {if ["circle_normal","circle_seam"].contains(&key.as_str()) {SCALAR} else {LENGTH}} else if ellipse_transition {LENGTH} else if circle_transition && ["start_center","end_center","start_radius","end_radius"].contains(&key.as_str()) {LENGTH} else if pipe && key=="sections" {SCALAR} else if clothoid && ["start_curvature","end_curvature"].contains(&key.as_str()) {[-1,0]} else if clothoid && key=="length" {LENGTH} else if catenoid && ["scale","start_z","end_z"].contains(&key.as_str()) || catenary && ["scale","start_x","end_x"].contains(&key.as_str()) || line_coordinates && ["start","end"].contains(&key.as_str()) || [
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
                    "wall_tolerance", "quantum", "max_deviation", "budget", "error_budget", "cap_correction_tolerance", "cap_correction_quantum", "circle_correction_tolerance", "circle_correction_quantum",
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

#[cfg(test)]
#[path = "tests/nurbs.rs"]
mod tests;

#[cfg(test)]
mod cartesian_loft_tests {
    use super::*;
    fn document(operation: &str) -> J {
        let mut loft=json!({"id":"loft","op":operation,"inputs":["a","b"],
            "parameters":[2.,7.],"guides":["g"],"construction":"cartesian",
            "max_cells":50000,"max_map_evaluations":200000,
            "start_tangents":[[0.,0.,0.4],[0.,0.,0.4]],
            "end_tangents":[[0.,0.,0.1],[0.,0.,0.1]]});
        if operation=="guided_loft_surface" {loft["guide_parameters"]=json!([0.]);loft["error_budget"]=json!(1e-6);}
        else {loft["budget"]=json!(1e-6);}
        json!({"language":"rush/nurbs-1","units":"mm","nodes":[
            {"id":"a","op":"line_curve","start":[0.,0.,0.],"end":[1.,0.,0.]},
            {"id":"b","op":"line_curve","start":[0.,0.,1.],"end":[1.,0.,1.]},
            {"id":"g","op":"line_curve","start":[0.,0.,0.],"end":[0.,0.,1.]},loft],"root":"loft"})
    }
    #[test]
    fn cartesian_loft_graph_retains_audit_options_and_refuses_ignored_fields() {
        for operation in ["guided_loft_surface","auto_guided_loft_surface"] {
            let input=document(operation);
            assert!(compile(input.clone()).is_ok());
            let nodes=input["nodes"].as_array().unwrap().clone();
            assert!(compile_text(nodes,&[],"loft".into()).is_ok());
            let mut wrong=input.clone();
            wrong["nodes"][3]["construction"]=json!("homogeneous");
            assert!(compile(wrong).is_err());
            let mut depleted=input.clone();
            depleted["nodes"][3]["max_cells"]=json!(0);
            assert!(compile(depleted).is_err());
            let mut unpaired=input;
            unpaired["nodes"][3].as_object_mut().unwrap().remove("end_tangents");
            assert!(compile(unpaired).is_err());
        }
    }
    #[test]
    fn section_maps_resolve_dimensionless_values_and_validate_nullable_shapes() {
        let mut input=document("guided_loft_surface");
        let leaf=json!({"pieces":[{"domain":[0.,1.],"range":[0.,1.],"controlValues":[0.,{"param":"q"},1.],"weights":[1.,1.,1.]}]});
        input["parameters"]=json!([{"id":"q","value":0.2}]);
        input["nodes"][3]["section_mappings"]=json!([J::Null,{"composition":[leaf.clone(),leaf.clone()]}]);
        let compiled=compile(input.clone()).unwrap();
        assert_eq!(compiled["resolved_document"]["nodes"][3]["section_mappings"][1]["composition"][0]["pieces"][0]["controlValues"][1],json!(0.2));
        let text=compile_text(input["nodes"].as_array().unwrap().clone(),input["parameters"].as_array().unwrap(),"loft".into()).unwrap();
        assert_eq!(text["document"]["nodes"][3]["section_mappings"][0],J::Null);
        let mut wrong=input.clone();
        wrong["nodes"][3]["section_mappings"]=json!([J::Null,{"unknown":1}]);
        assert!(compile(wrong).is_err());
        let mut wrong=input.clone();
        wrong["nodes"][3]["section_mappings"]=json!([leaf.clone()]);
        assert!(compile(wrong).is_err());
        let mut wrong=input;
        wrong["nodes"][3]["section_mappings"][1]["composition"][0]["pieces"][0]["controlValues"][1]=json!({"op":"quantity","value":0.2,"unit":"mm"});
        assert!(compile_text(wrong["nodes"].as_array().unwrap().clone(),&[],"loft".into()).is_err());
    }

}
