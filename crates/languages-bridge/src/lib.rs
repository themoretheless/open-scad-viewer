//! OpenSCAD and RushGraph frontends behind their own linear-memory ABI.
//!
//! Split out of `geometry-bridge` so the geometry kernel ships without a language frontend: this crate
//! only parses and compiles source into documents, and the host runs the geometry operations they name.
#![feature(try_blocks, yeet_expr)]
#![allow(unused_features)]
pub mod abi;
mod openscad;
use value_codec::{Value, json};

/// Source-to-graph frontend shared by browser workers and native callers.
pub fn compile_rush_frontend(source: &str) -> String {
    match rush_frontend::compile_document(source) {
        Ok(value) => json!({"ok":true,"value":value}).to_string(),
        Err(error) => json!({"ok":false,"message":error.message}).to_string(),
    }
}

/// Canonical graph preparation; errors retain the public RushGraph code/path/details.
pub fn compile_rush(input: &str) -> String {
    runtime_response(
        parse_graph_input(input).and_then(rush_runtime::compile),
        None,
    )
}
pub fn compile_rush_nurbs(input: &str) -> String {
    runtime_response(
        parse_graph_input(input).and_then(rush_runtime::nurbs::compile),
        None,
    )
}
fn parse_graph_input(input: &str) -> rush_runtime::Result<Value> {
    if input.len() > 2 * 1024 * 1024 {
        return Err(rush_runtime::Error::new(
            "input_limit",
            "/",
            "Document exceeds transport limit.",
        ));
    }
    value_codec::from_str(input)
        .map_err(|e| rush_runtime::Error::new("invalid_document", "/", e.to_string()))
}
fn runtime_response(
    result: rush_runtime::Result<Value>,
    customizer: Option<Value>,
) -> String {
    runtime_value(result, customizer).to_string()
}
pub(crate) fn runtime_value(
    result: rush_runtime::Result<Value>,
    customizer: Option<Value>,
) -> Value {
    match result {
        Ok(value) => json!({"ok":true,"value":value}),
        Err(error) => json!({"ok":false,"error":error,"customizer":customizer}),
    }
}
/// Fused source -> authoring graph -> evaluated graph, with no intermediate JS graph.
pub fn execute_rush_frontend(source: &str) -> String {
    execute_text_value(source).to_string()
}
pub(crate) fn execute_text_value(source: &str) -> Value {
    let mut graph = match rush_frontend::compile_document(source) {
        Ok(graph) => graph,
        Err(error) => {
            return runtime_value(
                Err(rush_runtime::Error::new(
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
                "brep_sphere",
                "brep_torus",
                "brep_cylinder",
                "brep_frustum",
                "brep_tube",
                "brep_gear",
                "brep_extrude_curves",
                "parabola_curve",
                "hyperbola_curve",
                "elliptic_cylinder_surface",
                "cone_frustum_surface",
                "quadratic_patch",
                "ribbon_surface", "variable_pipe_surface", "pipe_surface", "screw_surface", "clothoid_curve", "spherical_spiral_curve", "toroidal_spiral_curve", "torus_knot_curve", "helicoid_patches", "circle_rectangle_transition", "ellipse_transition_surface", "circle_transition_surface", "helicoid_surface", "catenoid_patches", "catenoid_surface", "catenary_curve", "archimedean_spiral_curve", "epicycloid_curve", "hypocycloid_curve", "trochoid_curve", "cycloid_curve", "lissajous_curve", "logarithmic_spiral_curve", "involute_curve", "elliptic_helix_curve", "variable_pitch_helix_curve", "conical_helix_curve", "helix_curve", "twist_sweep", "two_guide_sweep", "scaled_sweep", "clamped_loft_surface", "boundary_fill", "triangular_patch", "gordon_surface", "closed_loft_surface", "natural_loft_surface", "grid_spline_surface", "hermite_patch", "closed_spline_curve", "clamped_spline_curve", "natural_spline_curve", "hermite_curve", "framed_sweep", "formula_curve", "formula_surface", "coons_patch", "ruled_surface", "sphere_surface", "cylinder_surface", "cone_surface",
            "plane_patch", "bilinear_patch", "bezier_surface",
                "bezier_curve", "curve_compose", "round_polyline_curve", "transition_polyline_curve", "brep_miter_sweep","brep_progressive_miter_sweep",
                "line_curve", "polyline_curve", "circle_curve", "circle_arc",
                "hyperboloid_one_sheet", "hyperboloid_two_sheet", "polynomial_graph",
                "polynomial_curve", "polynomial_surface", "rational_polynomial_curve", "rational_polynomial_surface",
                "ellipse_arc",
                "ellipsoid_surface",
                "torus_surface",
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
                do yeet rush_runtime::Error::new(
                    "text_error",
                    "",
                    "NURBS text checks are not supported; use the build topology report",
                );
            }
            if graph.get("segments").is_some() {
                do yeet rush_runtime::Error::new(
                    "text_error",
                    "",
                    "segments applies only to legacy geometry; use explicit tessellation arguments for own geometry",
                );
            }
            let Value::Array(nodes) = nodes else {
                unreachable!()
            };
            let mut compiled = rush_runtime::nurbs::compile_text(
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
            document.insert("language".into(), json!("rush/ir-1"));
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
            rush_runtime::compile(Value::Object(document))?
        };
        compiled["customizer"] = controls.clone();
        compiled
    };
    runtime_value(result, Some(controls))
}

pub fn compile_rush_frontend_nurbs(input: &str) -> String {
    let result = parse_graph_input(input).and_then(|v| {
        let nodes = v["nodes"].as_array().ok_or_else(|| {
            rush_runtime::Error::new("invalid_document", "/nodes", "Expected nodes.")
        })?;
        let parameters = v["parameters"].as_array().ok_or_else(|| {
            rush_runtime::Error::new(
                "invalid_document",
                "/parameters",
                "Expected parameters.",
            )
        })?;
        let root = v["root"].as_str().ok_or_else(|| {
            rush_runtime::Error::new("invalid_document", "/root", "Expected root.")
        })?;
        rush_runtime::nurbs::compile_text(nodes.clone(), parameters, root.into())
    });
    runtime_response(result, None)
}

pub fn abi_language(op: u32, value: Value) -> Value {
    match op {
        1 => match value.as_str() {
            Some(s) => match rush_frontend::compile_document(s) {
                Ok(value) => json!({"ok":true,"value":value}),
                Err(error) => json!({"ok":false,"message":error.message}),
            },
            None => json!({"ok":false,"message":"Expected source string"}),
        },
        2 => runtime_value(rush_runtime::compile(value), None),
        3 => runtime_value(rush_runtime::nurbs::compile(value), None),
        4 => match value.as_str() {
            Some(s) => execute_text_value(s),
            None => runtime_value(
                Err(rush_runtime::Error::new(
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
                    rush_runtime::Error::new("invalid_document", "/nodes", "Expected nodes")
                })?;
                let parameters = value["parameters"].as_array().ok_or_else(|| {
                    rush_runtime::Error::new(
                        "invalid_document",
                        "/parameters",
                        "Expected parameters",
                    )
                })?;
                let root = value["root"].as_str().ok_or_else(|| {
                    rush_runtime::Error::new("invalid_document", "/root", "Expected root")
                })?;
                rush_runtime::nurbs::compile_text(nodes.clone(), parameters, root.into())?
            },
            None,
        ),
        10 => openscad::scad_compile(&value),
        11 => openscad::scad_eval(&value),
        12 => openscad::extrude_slices(&value),
        13 => openscad::resize(&value),
        14 => openscad::fragments(&value),
        15 => openscad::legacy_segments(&value),
        16 => openscad::scad_geometry_plan(&value),
        17 => openscad::stable_box_plan(&value),
        18 => openscad::stable_radial_plan(&value),
        19 => openscad::stable_transform_analysis(&value),
        20 => openscad::stable_euler_matrix(&value),
        21 => openscad::stable_mirror_matrix(&value),
        22 => openscad::stable_axis_angle_matrix(&value),
        23 => openscad::stable_vector_conversion(&value),
        24 => openscad::stable_euler_arguments(&value),
        25 => openscad::stable_authored_matrix(&value),
        26 => openscad::stable_vector_transform(&value),
        27 => openscad::stable_scalar_rotation(&value),
        28 => openscad::stable_polyhedron_faces(&value),
        29 => openscad::stable_polygon_paths(&value),
        30 => openscad::stable_indexed_policy(&value),
        31 => openscad::stable_extrusion_parameters(&value),
        32 => openscad::stable_revolution_parameters(&value),
        33 => openscad::stable_offset_parameters(&value),
        34 => openscad::stable_child_indices(&value),
        35 => openscad::degree_math(&value),
        _ => {
            json!({"ok":false,"error":{"code":"GEOMETRY_INVALID_INPUT","message":"Unknown ABI operation"}})
        }
    }
}

#[cfg(test)]
mod progressive_body_correction_tests {
    #[test]
    fn corrected_progressive_body_passes_rust_text_and_strict_runtime_schema(){
        for source in [include_str!("../../../examples/rush/arc-length-curved-fixed-normal-body-boundary.r"),
            include_str!("../../../examples/rush/arc-length-curved-fixed-normal-folded-body-boundary.r")]{
            let result=super::execute_text_value(source);
            assert_eq!(result["ok"].as_bool(),Some(true),"{result}");
        }
    }
}
