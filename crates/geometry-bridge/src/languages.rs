//! OpenSCAD and ModelGraph frontends. Optional: enable the `languages` feature.
use super::*;

/// Source-to-graph frontend shared by browser workers and native callers.
pub fn compile_modelgraph_text(source: &str) -> String {
    match modelgraph_text::compile(source) {
        Ok(value) => json!({"ok":true,"value":value}).to_string(),
        Err(error) => json!({"ok":false,"message":error.message}).to_string(),
    }
}

/// Canonical graph preparation; errors retain the public ModelGraph code/path/details.
pub fn compile_modelgraph(input: &str) -> String {
    runtime_response(
        parse_graph_input(input).and_then(modelgraph_runtime::compile),
        None,
    )
}
pub fn compile_modelgraph_nurbs(input: &str) -> String {
    runtime_response(
        parse_graph_input(input).and_then(modelgraph_runtime::nurbs::compile),
        None,
    )
}
fn parse_graph_input(input: &str) -> modelgraph_runtime::Result<Value> {
    if input.len() > 2 * 1024 * 1024 {
        return Err(modelgraph_runtime::Error::new(
            "input_limit",
            "/",
            "Document exceeds transport limit.",
        ));
    }
    value_codec::from_str(input)
        .map_err(|e| modelgraph_runtime::Error::new("invalid_document", "/", e.to_string()))
}
fn runtime_response(
    result: modelgraph_runtime::Result<Value>,
    customizer: Option<Value>,
) -> String {
    runtime_value(result, customizer).to_string()
}
pub(crate) fn runtime_value(
    result: modelgraph_runtime::Result<Value>,
    customizer: Option<Value>,
) -> Value {
    match result {
        Ok(value) => json!({"ok":true,"value":value}),
        Err(error) => json!({"ok":false,"error":error,"customizer":customizer}),
    }
}
/// Fused source -> authoring graph -> evaluated graph, with no intermediate JS graph.
pub fn execute_modelgraph_text(source: &str) -> String {
    execute_text_value(source).to_string()
}
pub(crate) fn execute_text_value(source: &str) -> Value {
    let mut graph = match modelgraph_text::compile(source) {
        Ok(graph) => graph,
        Err(error) => {
            return runtime_value(
                Err(modelgraph_runtime::Error::new(
                    "text_error",
                    "",
                    error.message,
                )),
                None,
            );
        }
    };
    let controls = graph["customizer"].take();
    let result = try {
        let nodes = graph["nodes"].take();
        let own = nodes.as_array().unwrap().iter().any(|n| {
            [
                "polygon_profile",
                "polygon_loft",
                "triangle_mesh",
                "subdivision",
                "sdf_sphere",
                "sdf_box",
                "sdf_torus",
                "brep_box",
                "nurbs_surface",
                "nurbs_curve",
                "mesh_boolean",
            ]
            .contains(&n["op"].as_str().unwrap_or(""))
        });
        let mut compiled = if own {
            if !graph["constraints"].as_array().unwrap().is_empty()
                || !graph["checks"].as_array().unwrap().is_empty()
            {
                do yeet modelgraph_runtime::Error::new(
                    "text_error",
                    "",
                    "NURBS text checks are not supported; use the build topology report",
                );
            }
            if graph.get("segments").is_some() {
                do yeet modelgraph_runtime::Error::new(
                    "text_error",
                    "",
                    "segments applies only to legacy geometry; use explicit tessellation arguments for own geometry",
                );
            }
            let Value::Array(nodes) = nodes else {
                unreachable!()
            };
            let mut compiled = modelgraph_runtime::nurbs::compile_text(
                nodes,
                graph["parameters"].as_array().unwrap(),
                graph["root"].as_str().unwrap().into(),
            )?;
            compiled["source"] = json!("");
            for key in [
                "source_map",
                "geometry_assertions",
                "constraint_report",
                "sketch_solutions",
                "assembly_components",
                "mechanical_reports",
                "mechanical_parts",
            ] {
                compiled[key] = json!([]);
            }
            compiled
        } else {
            let mut document = value_codec::Map::new();
            document.insert("language".into(), json!("modelgraph/1"));
            document.insert("units".into(), json!("mm"));
            document.insert("nodes".into(), nodes);
            for key in ["parameters", "root"] {
                document.insert(key.into(), graph[key].take());
            }
            if let Some(segments) = graph.get_mut("segments") {
                document.insert("segments".into(), segments.take());
            }
            for (source, target) in [
                ("constraints", "constraints"),
                ("checks", "geometry_assertions"),
            ] {
                if !graph[source].as_array().unwrap().is_empty() {
                    document.insert(target.into(), graph[source].take());
                }
            }
            modelgraph_runtime::compile(Value::Object(document))?
        };
        compiled["customizer"] = controls.clone();
        compiled
    };
    runtime_value(result, Some(controls))
}

pub fn compile_modelgraph_text_nurbs(input: &str) -> String {
    let result = parse_graph_input(input).and_then(|v| {
        let nodes = v["nodes"].as_array().ok_or_else(|| {
            modelgraph_runtime::Error::new("invalid_document", "/nodes", "Expected nodes.")
        })?;
        let parameters = v["parameters"].as_array().ok_or_else(|| {
            modelgraph_runtime::Error::new(
                "invalid_document",
                "/parameters",
                "Expected parameters.",
            )
        })?;
        let root = v["root"].as_str().ok_or_else(|| {
            modelgraph_runtime::Error::new("invalid_document", "/root", "Expected root.")
        })?;
        modelgraph_runtime::nurbs::compile_text(nodes.clone(), parameters, root.into())
    });
    runtime_response(result, None)
}

pub(crate) fn abi_language(op: u32, value: Value) -> Value {
    match op {
        1 => match value.as_str() {
            Some(s) => match modelgraph_text::compile(s) {
                Ok(value) => json!({"ok":true,"value":value}),
                Err(error) => json!({"ok":false,"message":error.message}),
            },
            None => json!({"ok":false,"message":"Expected source string"}),
        },
        2 => runtime_value(modelgraph_runtime::compile(value), None),
        3 => runtime_value(modelgraph_runtime::nurbs::compile(value), None),
        4 => match value.as_str() {
            Some(s) => execute_text_value(s),
            None => runtime_value(
                Err(modelgraph_runtime::Error::new(
                    "text_error",
                    "",
                    "Expected source string",
                )),
                None,
            ),
        },
        5 => runtime_value(
            try {
                let nodes = value["nodes"].as_array().ok_or_else(|| {
                    modelgraph_runtime::Error::new("invalid_document", "/nodes", "Expected nodes")
                })?;
                let parameters = value["parameters"].as_array().ok_or_else(|| {
                    modelgraph_runtime::Error::new(
                        "invalid_document",
                        "/parameters",
                        "Expected parameters",
                    )
                })?;
                let root = value["root"].as_str().ok_or_else(|| {
                    modelgraph_runtime::Error::new("invalid_document", "/root", "Expected root")
                })?;
                modelgraph_runtime::nurbs::compile_text(nodes.clone(), parameters, root.into())?
            },
            None,
        ),
        10 => crate::openscad::scad_compile(&value),
        11 => crate::openscad::scad_eval(&value),
        _ => {
            json!({"ok":false,"error":{"code":"GEOMETRY_INVALID_INPUT","message":"Unknown ABI operation"}})
        }
    }
}
