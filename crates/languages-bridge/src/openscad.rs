//! OpenSCAD compiler front-end ABI adapter (migration stage 1: parse only).
//! Input: `{"source": string, "profile": "openscad-viewer-subset@1" | "openscad/stable-2021.01"}`.
//! Output: `{"ok": true, "ast": [...]}` or
//! `{"ok": false, "diagnostics": [{code, message, start, end, line, column}]}`.
//! Diagnostics keep the exact codes and UTF-16 positions of the TypeScript
//! parser so the language conformance corpus gates parity.
use openscad_core::{LanguageProfile, MAX_SOURCE_LENGTH, ParseError};
use value_codec::{Value, json};

// Nonfinite numeric tokens are a transport detail, shared by every numeric plan.
fn read_numeric(value: &Value) -> Option<f64> {
    value.as_f64().or_else(|| match value.as_str()? {
        "NaN" => Some(f64::NAN),
        "Infinity" => Some(f64::INFINITY),
        "-Infinity" => Some(f64::NEG_INFINITY),
        _ => None,
    })
}
fn encode_numeric(value: f64) -> Value {
    if value.is_finite() {
        json!(value)
    } else {
        json!(if value.is_nan() {
            "NaN"
        } else if value > 0. {
            "Infinity"
        } else {
            "-Infinity"
        })
    }
}

pub fn scad_compile(value: &Value) -> Value {
    let Some(source) = value["source"].as_str() else {
        return input_error("Expected source string");
    };
    let profile_name = value["profile"]
        .as_str()
        .unwrap_or("openscad-viewer-subset@1");
    let Some(profile) = LanguageProfile::parse(profile_name) else {
        return input_error(format!("Unknown OpenSCAD language profile {profile_name}"));
    };
    // Mirrors the MAX_SOURCE_LENGTH guard in `openscadParser.ts` (positions
    // count UTF-16 code units like the TS parser).
    let units: Vec<u16> = source.encode_utf16().collect();
    if units.len() > MAX_SOURCE_LENGTH {
        let diagnostic = ParseError::new(0, "Source exceeds 250,000 characters").resolve(&units);
        return json!({"ok": false, "diagnostics": [openscad_core::serialize::diagnostic(&diagnostic)]});
    }
    match openscad_core::compile_units(&units, profile) {
        Ok(statements) => {
            json!({"ok": true, "ast": openscad_core::serialize::program(&statements)})
        }
        Err(diagnostic) => {
            json!({"ok": false, "diagnostics": [openscad_core::serialize::diagnostic(&diagnostic)]})
        }
    }
}

fn input_error(message: impl Into<String>) -> Value {
    json!({"ok": false, "error": {"code": "GEOMETRY_INVALID_INPUT", "message": message.into()}})
}

pub fn stable_box_plan(value: &Value) -> Value {
    use openscad_core::primitive_plan::{Size, box_plan};
    let cube = match value["kind"].as_str() {
        Some("cube") => true,
        Some("square") => false,
        _ => return input_error("Expected cube or square"),
    };
    let size = if value["missing"].as_bool() == Some(true) {
        Size::Missing
    } else if let Some(number) = read_numeric(&value["size"]) {
        Size::Scalar(number)
    } else if let Some(vector) = value["size"].as_array() {
        // Only exact 2/3-component vectors can affect the plan.
        if vector.len() > 3 {
            Size::Invalid
        } else {
            Size::Vector(vector.iter().map(read_numeric).collect())
        }
    } else {
        Size::Invalid
    };
    let plan = box_plan(&size, cube, value["center"].as_bool() == Some(true));
    let dimensions: Vec<Value> = plan
        .dimensions
        .iter()
        .copied()
        .map(encode_numeric)
        .collect();
    json!({"ok":true,"value":{"dimensions":dimensions,"center":plan.center,
        "sizeSource":plan.source,"empty":plan.empty,"defaulted":plan.defaulted}})
}

/// Stage-2 migration ABI: evaluate the source with the Rust value evaluator.
/// Geometry modules return accounted shape descriptors (`{name, dimension}`)
/// until stage 3 wires the shared cad handle store. Output:
/// `{"ok": true, "shapes": [...], "warnings": [...], "reduced": bool}` or
/// `{"ok": false, "diagnostics": [{code, message, start, end, line, column}]}`
/// (`aborted: true` marks host cancellation instead of a diagnostic).
pub fn scad_eval(value: &Value) -> Value {
    let Some(source) = value["source"].as_str() else {
        return input_error("Expected source string");
    };
    let profile_name = value["profile"]
        .as_str()
        .unwrap_or("openscad-viewer-subset@1");
    let Some(profile) = LanguageProfile::parse(profile_name) else {
        return input_error(format!("Unknown OpenSCAD language profile {profile_name}"));
    };
    let units: Vec<u16> = source.encode_utf16().collect();
    if units.len() > MAX_SOURCE_LENGTH {
        let diagnostic = ParseError::new(0, "Source exceeds 250,000 characters").resolve(&units);
        return json!({"ok": false, "diagnostics": [openscad_core::serialize::diagnostic(&diagnostic)]});
    }
    let statements = match openscad_core::compile_units(&units, profile) {
        Ok(statements) => statements,
        Err(diagnostic) => {
            return json!({"ok": false, "diagnostics": [openscad_core::serialize::diagnostic(&diagnostic)]});
        }
    };
    let options = openscad_core::eval::EvaluatorOptions {
        record_geometry: value["geometry"].as_bool().unwrap_or(false),
        quality: if value["quality"].as_str() == Some("preview") {
            openscad_core::eval::Quality::Preview
        } else {
            openscad_core::eval::Quality::Full
        },
        ..Default::default()
    };
    match openscad_core::eval::evaluate_program(&statements, &units, profile, options) {
        Ok(evaluation) => {
            let shapes: Vec<Value> = evaluation
                .shapes
                .iter()
                .map(|shape| json!({"name": shape.name, "dimension": shape.dimension}))
                .collect();
            let mut output = json!({"ok":true,"shapes":shapes,"warnings":evaluation.warnings,"reduced":evaluation.reduced});
            if let Some(program) = evaluation.geometry {
                output["program"] = json!(program);
            }
            output
        }
        Err(openscad_core::value::EvalFailure::Error(error)) => {
            json!({"ok": false, "diagnostics": [openscad_core::serialize::diagnostic(&error.resolve(&units))]})
        }
        Err(openscad_core::value::EvalFailure::Aborted) => json!({"ok": false, "aborted": true}),
        Err(openscad_core::value::EvalFailure::ViewportRoot(_)) => {
            unreachable!("root selection is caught by the top-level loop")
        }
    }
}

pub fn extrude_slices(value: &Value) -> Value {
    let read = |key: &str| read_numeric(&value[key]);
    let parsed = (|| {
        let points: Vec<[f64; 2]> = value_codec::from_value(value["points"].clone()).ok()?;
        let scale: [f64; 2] = value_codec::from_value(value["scale"].clone()).ok()?;
        Some(openscad_core::extrude_slices::automatic(
            &points,
            read("height")?,
            read("twist")?,
            scale,
            [read("fn").unwrap_or(f64::NAN), read("fa")?, read("fs")?],
        ))
    })();
    match parsed {
        Some(selection) => {
            json!({"ok":true,"value":{"slices":selection.slices,"source":selection.source.name()}})
        }
        None => input_error("Invalid extrusion subdivision parameters"),
    }
}

pub fn resize(value: &Value) -> Value {
    fn selection<const D: usize>(value: &Value) -> Option<Value> {
        let targets = value_codec::from_value::<[Option<f64>; D]>(value["targets"].clone()).ok()?;
        let raw_extents = if let Some(bounds) = value.get("bounds") {
            let bounds = bounds.as_array()?.iter().map(|bound| {
                let axis_values = |key: &str| -> Option<[f64; D]> {
                    let values = bound[key].as_array()?;
                    (0..D).map(|axis| read_numeric(values.get(axis)?))
                        .collect::<Option<Vec<_>>>()?.try_into().ok()
                };
                Some((axis_values("min")?, axis_values("max")?))
            }).collect::<Option<Vec<_>>>()?;
            geometry_ops::resize::aggregate_extents(&bounds)
        } else {
            value["extents"].as_array()?.iter().map(|extent| {
                if extent.is_null() { Some(f64::NAN) } else { read_numeric(extent) }
            })
                .collect::<Option<Vec<_>>>()?.try_into().ok()?
        };
        let extents = raw_extents.map(|extent| {
            (extent.is_finite() && extent > 0.).then_some(extent)
        });
        let automatic = value_codec::from_value::<[bool; D]>(value["automatic"].clone()).ok()?;
        let selected = geometry_ops::resize::resolve(targets, extents, automatic);
        let scales: Vec<Value> = selected
            .scales
            .iter()
            .map(|v| {
                if v.is_finite() {
                    json!(v)
                } else {
                    json!("Infinity")
                }
            })
            .collect();
        Some(json!({"ok":true,"value":{"scales":scales,"invalidAxis":selected.invalid_axis,"extents":raw_extents.map(encode_numeric)}}))
    }
    match value["targets"].as_array().map(Vec::len) {
        Some(2) => selection::<2>(value),
        Some(3) => selection::<3>(value),
        _ => None,
    }
    .unwrap_or_else(|| input_error("Invalid resize parameters"))
}

pub fn fragments(value: &Value) -> Value {
    let Some(maximum) = read_numeric(&value["maximum"])
        .filter(|m| m.is_finite() && *m >= 3. && m.fract() == 0. && *m <= 9007199254740991.)
    else {
        return input_error("Invalid fragment maximum");
    };
    let Some([radius, fn_, fa, fs]) = ["radius", "fn", "fa", "fs"]
        .map(|k| read_numeric(&value[k]))
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .and_then(|v| <[f64; 4]>::try_from(v).ok())
    else {
        return input_error("Invalid fragment parameters");
    };
    let r = openscad_core::fragments::resolve(radius, fn_, fa, fs, maximum);
    use openscad_core::fragments::WarningKind::*;
    let warnings=r.warnings.iter().map(|w|json!({"kind":match w.kind{NonFiniteFn=>"fn-nonfinite",NegativeFn=>"fn-negative",NanFa=>"fa-nan",NanFs=>"fs-nan",SmallFa=>"fa-small",SmallFs=>"fs-small",NonFiniteResolution=>"resolution-nonfinite",Clamped=>"clamped"},"value":encode_numeric(w.value)})).collect::<Vec<_>>();
    let mut result = json!({"fragments":r.fragments,"unboundedFragments":r.unbounded,"source":r.source,"effectiveFn":encode_numeric(r.effective_fn),"effectiveFa":encode_numeric(r.fa),"effectiveFs":encode_numeric(r.fs),"maximum":maximum,"reduced":r.reduced,"warnings":warnings});
    if let Some(degrees) = value.get("sweepDegrees") {
        let Some(degrees) = read_numeric(degrees) else {
            return input_error("Invalid fragment sweep");
        };
        let (degrees, count) = openscad_core::fragments::sweep(r.fragments, degrees);
        result["sweepDegrees"] = json!(degrees);
        result["sweepFragments"] = json!(count);
    }
    json!({"ok":true,"value":result})
}

pub fn legacy_segments(value: &Value) -> Value {
    let parsed = (|| {
        let requested = if value["requested"].is_null() {
            None
        } else {
            Some(value["requested"].as_f64()?)
        };
        let fallback = value["fallback"].as_f64()?;
        let minimum = value["minimum"].as_f64()?;
        let preview = value["preview"].as_bool()?;
        if requested.is_some_and(|v| !v.is_finite())
            || !fallback.is_finite()
            || !minimum.is_finite()
        {
            return None;
        }
        Some(openscad_core::fragments::legacy(
            requested, fallback, minimum, preview,
        ))
    })();
    match parsed {
        Some(r) => {
            json!({"ok":true,"value":{"requested":r.requested,"beforeCap":r.before_cap,"maximum":r.maximum,"segments":r.segments,"clamped":r.clamped,"reduced":r.reduced}})
        }
        None => input_error("Invalid legacy subdivision parameters"),
    }
}

pub fn scad_geometry_plan(value: &Value) -> Value {
    if value["source"].as_str().is_none() {
        return input_error("Expected source string");
    };
    let mut input = value.clone();
    input["geometry"] = json!(true);
    scad_eval(&input)
}

/// Stable radius plans; authored warning values stay with the caller.
pub fn stable_radial_plan(value: &Value) -> Value {
    use openscad_core::primitive_plan::{cylinder_plan, radial_empty, radius_pair};
    let common = radius_pair(read_numeric(&value["r"]), read_numeric(&value["d"]));
    match value["kind"].as_str() {
        Some("sphere" | "circle") => json!({"ok":true,"value":{
            "radius":encode_numeric(common.value),"radiusSource":common.source,
            "fragmentRadius":encode_numeric(common.value),"empty":radial_empty(common.value),
            "shadowed":[common.shadowed]}}),
        Some("cylinder") => {
            let low = radius_pair(read_numeric(&value["r1"]), read_numeric(&value["d1"]));
            let high = radius_pair(read_numeric(&value["r2"]), read_numeric(&value["d2"]));
            let plan = cylinder_plan(read_numeric(&value["h"]), common, low, high);
            json!({"ok":true,"value":{"height":encode_numeric(plan.height),
                "radius1":encode_numeric(plan.radius1),"radius2":encode_numeric(plan.radius2),
                "radius1Source":plan.source1,"radius2Source":plan.source2,
                "fragmentRadius":encode_numeric(plan.fragment_radius),"empty":plan.empty,
                "ambiguous":plan.ambiguous,"shadowed":[common.shadowed,low.shadowed,high.shadowed]}})
        }
        _ => input_error("Expected sphere, circle or cylinder"),
    }
}

pub fn stable_transform_analysis(value: &Value) -> Value {
    let Some(values) = value["matrix"].as_array().filter(|v| v.len() == 16) else {
        return input_error("Expected 16-component transform matrix");
    };
    let mut matrix = [0.; 16];
    for (index, value) in values.iter().enumerate() {
        let Some(number) = read_numeric(value) else {
            return input_error("Expected numeric matrix component");
        };
        matrix[index] = number;
    }
    let a = openscad_core::transform_plan::analyze(matrix);
    let matrix2d: Vec<Value> = a.matrix2d.into_iter().map(encode_numeric).collect();
    json!({"ok":true,"value":{"matrix2d":matrix2d,"dropsChildren":a.drops_children,
        "matrix3dSingular":a.singular3d,"matrix2dSingular":a.singular2d,
        "affine3d":a.affine3d,"affine2d":a.affine2d}})
}

fn matrix_plan(matrix: [f64; 16]) -> Value {
    let a = openscad_core::transform_plan::analyze(matrix);
    let matrix: Vec<Value> = matrix.into_iter().map(encode_numeric).collect();
    let matrix2d: Vec<Value> = a.matrix2d.into_iter().map(encode_numeric).collect();
    json!({"matrix":matrix,"analysis":{"matrix2d":matrix2d,"dropsChildren":a.drops_children,
        "matrix3dSingular":a.singular3d,"matrix2dSingular":a.singular2d,
        "affine3d":a.affine3d,"affine2d":a.affine2d}})
}

pub fn stable_euler_matrix(value: &Value) -> Value {
    let Some(values) = value["angles"].as_array().filter(|v| v.len() == 3) else {
        return input_error("Expected three Euler angles");
    };
    let mut angles = [0.; 3];
    for (index, value) in values.iter().enumerate() {
        let Some(number) = read_numeric(value) else {
            return input_error("Expected numeric Euler angle");
        };
        angles[index] = number;
    }
    json!({"ok":true,"value":matrix_plan(openscad_core::transform_plan::euler_matrix(angles))})
}

pub fn stable_mirror_matrix(value: &Value) -> Value {
    let Some(values) = value["normal"].as_array().filter(|v| v.len() == 3) else {
        return input_error("Expected three normal components");
    };
    let mut normal = [0.; 3];
    for (index, value) in values.iter().enumerate() {
        let Some(number) = read_numeric(value) else {
            return input_error("Expected numeric normal component");
        };
        normal[index] = number;
    }
    json!({"ok":true,"value":matrix_plan(openscad_core::transform_plan::mirror_matrix(normal))})
}

pub fn stable_axis_angle_matrix(value: &Value) -> Value {
    let Some(values) = value["parameters"].as_array().filter(|v| v.len() == 4) else {
        return input_error("Expected angle and three axis components");
    };
    let mut parameters = [0.; 4];
    for (index, value) in values.iter().enumerate() {
        let Some(number) = read_numeric(value) else {
            return input_error("Expected numeric axis-angle component");
        };
        parameters[index] = number;
    }
    if value["viewer"].as_bool() == Some(true) {
        let plan = openscad_core::transform_plan::viewer_axis_angle_matrix(parameters[0], [parameters[1],parameters[2],parameters[3]]).map(matrix_plan);
        return json!({"ok":true,"value":plan});
    }
    json!({"ok":true,"value":matrix_plan(openscad_core::transform_plan::axis_angle_matrix(parameters[0],[parameters[1],parameters[2],parameters[3]]))})
}

pub fn stable_vector_conversion(value: &Value) -> Value {
    let Some(initial) = value["initial"].as_array().filter(|v| v.len() == 3) else {
        return input_error("Expected initial vector");
    };
    let mut defaults = [0.; 3];
    for (index, value) in initial.iter().enumerate() {
        let Some(number) = read_numeric(value) else {
            return input_error("Expected numeric default");
        };
        defaults[index] = number;
    }
    let Some(z) = read_numeric(&value["defaultZ"]) else {
        return input_error("Expected numeric Z default");
    };
    let input = value["vector"]
        .as_array()
        .filter(|v| v.len() <= 3)
        .map(|v| v.iter().map(read_numeric).collect::<Vec<_>>());
    let converted =
        openscad_core::transform_plan::vector_with_default(input.as_deref(), defaults, z);
    let vector: Vec<Value> = converted.vector.into_iter().map(encode_numeric).collect();
    json!({"ok":true,"value":{"vector":vector,"converted":converted.converted}})
}

pub fn stable_euler_arguments(value: &Value) -> Value {
    let Some(length) = value["length"]
        .as_f64()
        .filter(|v| v.is_finite() && *v >= 0. && *v <= 4294967295. && v.fract() == 0.)
    else {
        return input_error("Expected vector length");
    };
    let length = length as usize;
    let Some(values) = value["components"]
        .as_array()
        .filter(|v| v.len() == length.min(3))
    else {
        return input_error("Expected bounded Euler components");
    };
    let components: Vec<Option<f64>> = values.iter().map(read_numeric).collect();
    let result = openscad_core::transform_plan::euler_arguments(&components, length);
    json!({"ok":true,"value":{"plan":matrix_plan(openscad_core::transform_plan::euler_matrix(result.angles)),"valid":result.valid}})
}

pub fn stable_authored_matrix(value: &Value) -> Value {
    let rows = value["rows"].as_array().map(|rows| {
        rows.iter()
            .take(4)
            .map(|row| {
                row.as_array()
                    .map(|cells| cells.iter().take(4).map(read_numeric).collect::<Vec<_>>())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
    });
    if value["legacy2d"].as_bool() == Some(true) {
        let matrix = openscad_core::transform_plan::authored_affine2d(rows.as_deref())
            .map(|matrix| matrix.into_iter().map(encode_numeric).collect::<Vec<_>>());
        return json!({"ok":true,"value":matrix});
    }
    let result = openscad_core::transform_plan::authored_matrix(rows.as_deref());
    json!({"ok":true,"value":{"plan":matrix_plan(result.matrix),"valid":result.valid}})
}

pub fn stable_vector_transform(value: &Value) -> Value {
    use openscad_core::transform_plan::{VectorTransform, vector_transform};
    let kind = match value["kind"].as_str() {
        Some("translate") => VectorTransform::Translate,
        Some("scale") => VectorTransform::Scale,
        Some("mirror") => VectorTransform::Mirror,
        _ => return input_error("Expected vector transform kind"),
    };
    let vector = value["vector"]
        .as_array()
        .filter(|v| v.len() <= 3)
        .map(|v| v.iter().map(read_numeric).collect::<Vec<_>>());
    let plan = vector_transform(kind, vector.as_deref(), read_numeric(&value["scalar"]));
    json!({"ok":true,"value":{"plan":matrix_plan(plan.matrix),"valid":plan.valid,"rangeWarning":plan.range_warning}})
}

pub fn stable_scalar_rotation(value: &Value) -> Value {
    let axis = value["axis"]
        .as_array()
        .filter(|v| v.len() <= 3)
        .map(|v| v.iter().map(read_numeric).collect::<Vec<_>>());
    let plan = openscad_core::transform_plan::scalar_rotation(
        read_numeric(&value["angle"]),
        axis.as_deref(),
        value["axisProvided"].as_bool() == Some(true),
    );
    json!({"ok":true,"value":{"plan":matrix_plan(plan.matrix),"valid":plan.valid,"angleValid":plan.angle_valid}})
}

fn indexed_vectors(value: &Value) -> Option<Vec<Vec<Option<f64>>>> {
    Some(
        value
            .as_array()?
            .iter()
            .map(|row| {
                row.as_array()
                    .map(|v| v.iter().map(read_numeric).collect())
                    .unwrap_or_default()
            })
            .collect(),
    )
}
fn indexed_count(value: &Value) -> Option<usize> {
    let number = value
        .as_f64()
        .filter(|v| v.is_finite() && *v >= 0. && *v <= 9007199254740991. && v.fract() == 0.)?;
    Some((number as u64).min(usize::MAX as u64) as usize)
}
fn indexed_events(events: &[openscad_core::indexed_primitive::FaceEvent]) -> Vec<Value> {
    use openscad_core::indexed_primitive::FaceEvent;
    events
        .iter()
        .map(|e| match e {
            FaceEvent::OutOfBounds { face, entry, index } => {
                json!({"kind":"bounds","face":face,"entry":entry,"index":index})
            }
            FaceEvent::InvalidPoint { index } => json!({"kind":"point","index":index}),
            FaceEvent::IndexLimit { actual } => json!({"kind":"limit","actual":actual}),
            FaceEvent::PathLimit { actual } => json!({"kind":"paths","actual":actual}),
        })
        .collect()
}

pub fn stable_polyhedron_faces(value: &Value) -> Value {
    use openscad_core::indexed_primitive::expand_faces;

    let Some(points) = indexed_vectors(&value["points"]) else {
        return input_error("Expected point vectors");
    };
    let Some(faces) = indexed_vectors(&value["faces"]) else {
        return input_error("Expected face vectors");
    };
    let Some(maximum) = indexed_count(&value["maximumIndices"]) else {
        return input_error("Expected index limit");
    };
    let result = expand_faces(&points, &faces, maximum);
    let events = indexed_events(&result.events);
    let packed = match geometry_ops::polygon_mesh::reversed_fans(&result.polygons) {
        Ok(packed) => packed,
        Err(error) => return input_error(&error.message),
    };
    json!({"ok":true,"value":{"vertices":packed.vertices,"indices":packed.indices,"usable":packed.usable,"polygons":result.polygons,"events":events,"aborted":result.aborted,"empty":result.empty}})
}

pub fn stable_polygon_paths(value: &Value) -> Value {
    use openscad_core::indexed_primitive::expand_polygon;

    let Some(points) = indexed_vectors(&value["points"]) else {
        return input_error("Expected point vectors");
    };
    let Some(paths) = indexed_vectors(&value["paths"]) else {
        return input_error("Expected paths");
    };
    let Some(path_count) = indexed_count(&value["pathCount"]) else {
        return input_error("Expected path count");
    };
    let Some(maximum_paths) = indexed_count(&value["maximumPaths"]) else {
        return input_error("Expected path limit");
    };
    let Some(maximum_indices) = indexed_count(&value["maximumIndices"]) else {
        return input_error("Expected index limit");
    };
    let result = expand_polygon(&points, &paths, path_count, maximum_paths, maximum_indices);
    let events = indexed_events(&result.events);
    let points: Vec<Value> = result
        .points
        .iter()
        .map(|p| json!([encode_numeric(p[0]), encode_numeric(p[1])]))
        .collect();
    let outlines: Vec<Value> = result
        .outlines
        .iter()
        .map(|o| {
            json!(
                o.iter()
                    .map(|p| json!([encode_numeric(p[0]), encode_numeric(p[1])]))
                    .collect::<Vec<_>>()
            )
        })
        .collect();
    json!({"ok":true,"value":{"points":points,"outlines":outlines,"events":events,"pathSource":if result.implicit{"implicit"}else{"explicit"},"aborted":result.aborted,"empty":result.empty}})
}

pub fn stable_indexed_policy(value: &Value) -> Value {
    use openscad_core::indexed_primitive::{AuthoredLimit, indexed_policy};
    let Some(values) = value["limits"].as_array().filter(|v| v.len() == 3) else {
        return input_error("Expected three indexed limits");
    };
    let limits = std::array::from_fn(|i| {
        if values[i]["missing"].as_bool() == Some(true) {
            AuthoredLimit::Missing
        } else {
            read_numeric(&values[i]["number"])
                .map(AuthoredLimit::Number)
                .unwrap_or(AuthoredLimit::Invalid)
        }
    });
    match indexed_policy(limits, read_numeric(&value["convexity"])) {
        Ok(p) => {
            json!({"ok":true,"value":{"limits":p.limits.into_iter().map(encode_numeric).collect::<Vec<_>>(),"convexity":p.convexity}})
        }
        Err(field) => json!({"ok":false,"field":field}),
    }
}

pub fn stable_revolution_parameters(value: &Value) -> Value {
    let Some(minimum) = read_numeric(&value["minimum"]).filter(|v| v.is_finite()) else {
        return input_error("Invalid revolution profile bounds");
    };
    let Some(maximum) = read_numeric(&value["maximum"]).filter(|v| v.is_finite()) else {
        return input_error("Invalid revolution profile bounds");
    };
    if minimum > maximum {
        return input_error("Invalid revolution profile bounds");
    }
    let plan = openscad_core::extrusion_plan::revolution_parameters(
        read_numeric(&value["angle"]),
        value["angleProvided"].as_bool() == Some(true),
        minimum,
        maximum,
    );
    json!({"ok":true,"value":{"angle":plan.angle,"angleDefaulted":plan.angle_defaulted,"profileSide":plan.side,"empty":plan.empty,"reflectProfileX":plan.reflect,"radius":plan.radius}})
}
pub fn stable_offset_parameters(value: &Value) -> Value {
    use openscad_core::offset_plan::{self, Mode, Join};
    let plan = offset_plan::resolve(read_numeric(&value["r"]), read_numeric(&value["delta"]), value["chamfer"].as_bool() == Some(true));
    json!({"ok":true,"value":{"mode":match plan.mode {Mode::Radius=>"radius",Mode::Delta=>"delta"},"distance":encode_numeric(plan.distance),"joinType":match plan.join {Join::Round=>"Round",Join::Miter=>"Miter",Join::Square=>"Square"},"chamfer":plan.chamfer}})
}

pub fn stable_extrusion_parameters(value: &Value) -> Value {
    use openscad_core::{extrusion_plan::parameters, primitive_plan::Size};
    let scale = if value["scaleMissing"].as_bool() == Some(true) {
        Size::Missing
    } else if let Some(number) = read_numeric(&value["scale"]) {
        Size::Scalar(number)
    } else if let Some(vector) = value["scale"].as_array().filter(|v| v.len() == 2) {
        Size::Vector(vector.iter().map(read_numeric).collect())
    } else {
        Size::Invalid
    };
    let plan = parameters(
        read_numeric(&value["height"]),
        value["heightProvided"].as_bool() == Some(true),
        &scale,
        read_numeric(&value["twist"]),
        value["center"].as_bool() == Some(true),
        read_numeric(&value["slices"]),
    );
    if value["resolveSlices"].as_bool() == Some(true) {
        let maximum = value["maximumSlices"]
            .as_u64()
            .filter(|v| *v > 0 && *v <= 9_007_199_254_740_991);
        let Some(maximum) = maximum.and_then(|v| usize::try_from(v).ok()) else {
            return input_error("Invalid extrusion slice maximum");
        };
        let (slices, unbounded, source, reduced) = if plan.empty {
            (0, 0., "empty", false)
        } else {
            let points: Vec<[f64; 2]> = match value_codec::from_value(value["points"].clone()) {
                Ok(points) => points,
                Err(_) => return input_error("Invalid extrusion profile points"),
            };
            let special = |name: &str, default: f64, minimum: Option<f64>| {
                let number = read_numeric(&value[name]).unwrap_or(default);
                if number.is_finite() {
                    minimum.map_or(number, |minimum| number.max(minimum))
                } else {
                    number
                }
            };
            let fragments = [
                special("fn", 0., None),
                special("fa", 12., Some(0.01)),
                special("fs", 2., Some(0.01)),
            ];
            let resolved = match openscad_core::extrude_slices::resolve(
                &points,
                plan.height,
                plan.twist,
                plan.scale,
                fragments,
                plan.explicit_slices,
                maximum,
            ) {
                Ok(value) => value,
                Err(error) => return input_error(error.message),
            };
            (
                resolved.slices,
                resolved.unbounded_slices,
                resolved
                    .automatic_source
                    .map_or("explicit", |source| source.name()),
                resolved.reduced,
            )
        };
        return json!({"ok":true,"value":{"height":plan.height,"scale":plan.scale,"twist":plan.twist,"center":plan.center,"empty":plan.empty,"heightDefaulted":plan.height_defaulted,"scaleDefaulted":plan.scale_defaulted,"slices":slices,"unboundedSlices":unbounded,"sliceSource":source,"manifoldNDivisions":slices.saturating_sub(1),"reduced":reduced}});
    }
    json!({"ok":true,"value":{"height":plan.height,"scale":plan.scale,"twist":plan.twist,"center":plan.center,"explicitSlices":plan.explicit_slices,"empty":plan.empty,"heightDefaulted":plan.height_defaulted,"scaleDefaulted":plan.scale_defaulted}})
}


pub fn stable_child_indices(value: &Value) -> Value {
    use openscad_core::children_selection::{select, Issue};
    if value["range"].as_array().is_some() {
        let Some(range) = value["range"].as_array().filter(|v| v.len() == 3) else { return input_error("Expected range triple"); };
        let Some(maximum) = value["maximum"].as_u64().filter(|v| *v > 0 && *v <= 9_007_199_254_740_991) else { return input_error("Invalid children range maximum"); };
        let result = openscad_core::children_selection::expand_range(read_numeric(&range[0]).unwrap_or(f64::NAN), read_numeric(&range[1]).unwrap_or(f64::NAN), read_numeric(&range[2]).unwrap_or(f64::NAN), maximum);
        return match result {
            Ok(values) => json!({"ok":true,"value":{"candidates":values.into_iter().map(encode_numeric).collect::<Vec<_>>(),"issue":null}}),
            Err(issue) => json!({"ok":true,"value":{"candidates":[],"issue":match issue { openscad_core::children_selection::RangeIssue::Invalid => "invalid", openscad_core::children_selection::RangeIssue::Limit => "limit"}}}),
        };
    }

    let Some(count) = value["childCount"].as_u64().filter(|n| *n <= 9_007_199_254_740_991) else { return input_error("Invalid child count"); };
    let Some(candidates) = value["candidates"].as_array() else { return input_error("Expected child index candidates"); };
    let candidates = candidates.iter().map(read_numeric).collect::<Vec<_>>();
    let selection = select(&candidates, count);
    let issues = selection.issues.into_iter().map(|issue| match issue {
        Issue::Invalid { candidate } => json!({"kind":"invalid","candidate":candidate}),
        Issue::OutOfBounds { index } => json!({"kind":"out-of-bounds","index":index}),
    }).collect::<Vec<_>>();
    json!({"ok":true,"value":{"indices":selection.indices,"issues":issues}})
}

/// Bounded degree-math batch, preserving signed zero and nonfinite numeric tokens.
pub fn degree_math(value: &Value) -> Value {
    let Some(calls)=value["calls"].as_array().filter(|calls|calls.len()<=1024) else {return input_error("Invalid degree math batch");};
    let mut values=Vec::with_capacity(calls.len());
    for call in calls {
        let Some(name)=call["function"].as_str() else {return input_error("Invalid degree math function");};
        let Some(args)=call["args"].as_array() else {return input_error("Invalid degree math arguments");};
        let expected=if name=="atan2" {2} else {1};
        if args.len()!=expected {return input_error("Invalid degree math argument count");}
        let Some(args)=args.iter().map(|arg|if arg.as_str()==Some("-0") {Some(-0.)} else {read_numeric(arg)}).collect::<Option<Vec<_>>>() else {return input_error("Invalid degree math number");};
        use openscad_core::degree_math::*;
        let result=match name {
            "sin"=>sin_degrees(args[0]),"cos"=>cos_degrees(args[0]),"tan"=>tan_degrees(args[0]),
            "asin"=>asin_degrees(args[0]),"acos"=>acos_degrees(args[0]),"atan"=>atan_degrees(args[0]),
            "atan2"=>atan2_degrees(args[0],args[1]),
            _=>return input_error("Unsupported degree math function"),
        };
        values.push(if result==0. && result.is_sign_negative() {json!("-0")} else {encode_numeric(result)});
    }
    json!({"ok":true,"value":values})
}
