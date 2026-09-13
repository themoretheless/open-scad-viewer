//! Native execution of admitted, reduced B-rep semantic geometry nodes.
use super::{Result, Value, field, input};
use nurbs_core::curve::Curve;
use value_codec::json;
fn solid(model: brep_core::Model) -> Result<Value> {
    Ok(json!({"kind":"solid","model":model}))
}
fn region(value: Value) -> Result<Value> {
    Ok(json!({"kind":"profile","profile":value}))
}
fn model(value: &Value) -> Result<brep_core::Model> {
    if value["kind"].as_str() != Some("solid") {
        return Err(input("Expected solid semantic operand"));
    }
    field(value, "model")
}
fn profile(value: &Value) -> Result<&Value> {
    if value["kind"].as_str() != Some("profile") {
        return Err(input("Expected profile semantic operand"));
    }
    value.get("profile").ok_or_else(|| input("Missing profile"))
}
fn unsupported(message: &str) -> super::Error {
    super::Error::new("BREP_UNSUPPORTED_SEMANTIC", message)
}
fn is_empty(value: &Value) -> Result<bool> {
    if value["kind"].as_str() == Some("solid") {
        Ok(model(value)?.is_empty())
    } else {
        Ok(field::<Vec<Vec<Curve>>>(profile(value)?, "loops")?.is_empty())
    }
}
fn empty_result(space: &str, operand: Option<&Value>) -> Result<Value> {
    let tolerance = match operand {
        Some(value) if value["kind"].as_str() == Some("solid") => model(value)?.tolerance_mm,
        Some(value) => field(profile(value)?, "toleranceMm")?,
        None => 1e-7,
    };
    if space == "d2" {
        region(super::brep_profile::profile(Vec::new(), tolerance)?)
    } else {
        solid(brep_core::Model::empty(tolerance)?)
    }
}
pub fn execute(v: Value) -> Result<Value> {
    let node = v
        .get("node")
        .ok_or_else(|| input("Missing semantic node"))?;
    let inputs: Vec<Value> = field(&v, "inputs")?;
    if inputs.len() > 512 {
        return Err(input("Semantic operand budget exceeded"));
    }
    let kind = field::<String>(node, "kind")?;
    let space = node["valueType"]["space"].as_str();
    let expected = match kind.as_str() {
        "box"
        | "sphere-analytic"
        | "cylinder-analytic"
        | "linear-extrude"
        | "rotate-extrude-analytic" => "d3",
        "rectangle" | "circle-analytic" | "polygon" | "projection" | "offset" => "d2",
        "transform" | "boolean" | "hull" => match space {
            Some("d2") => "d2",
            Some("d3") => "d3",
            _ => {
                return Err(input(
                    "Semantic operation requires an explicit d2 or d3 result space",
                ));
            }
        },
        _ => return Err(unsupported("Unsupported B-rep semantic node")),
    };
    if space != Some(expected) {
        return Err(input("Semantic result space does not match the operation"));
    }
    let primitive = matches!(
        kind.as_str(),
        "box"
            | "sphere-analytic"
            | "cylinder-analytic"
            | "rectangle"
            | "circle-analytic"
            | "polygon"
    );
    if primitive && !inputs.is_empty() {
        return Err(input("Primitive semantic nodes cannot consume operands"));
    }
    let unary_operation = matches!(
        kind.as_str(),
        "transform" | "linear-extrude" | "rotate-extrude-analytic" | "projection" | "offset"
    );
    if unary_operation && inputs.len() != 1 {
        return Err(input("Expected one reduced semantic operand"));
    }
    let input_kind = if kind == "projection" {
        "solid"
    } else if matches!(kind.as_str(), "linear-extrude" | "rotate-extrude-analytic")
        || expected == "d2"
    {
        "profile"
    } else {
        "solid"
    };
    if inputs
        .iter()
        .any(|operand| operand["kind"].as_str() != Some(input_kind))
    {
        return Err(input("Semantic operand kind does not match the operation"));
    }
    // Empty algebra precedes feature construction, after kind/arity admission.
    if unary_operation && is_empty(&inputs[0])? {
        return empty_result(expected, Some(&inputs[0]));
    }
    if kind == "hull"
        && inputs
            .iter()
            .map(is_empty)
            .collect::<Result<Vec<_>>>()?
            .iter()
            .all(|empty| *empty)
    {
        return empty_result(expected, inputs.first());
    }
    let d2 = expected == "d2";
    let unary = || {
        if inputs.len() == 1 {
            Ok(&inputs[0])
        } else {
            Err(input("Expected one reduced semantic operand"))
        }
    };
    match kind.as_str() {
        "box" => {
            let size: [f64; 3] = field(node, "size")?;
            let center: bool = field(node, "center")?;
            let min = size.map(|x| if center { -x / 2. } else { 0. });
            solid(brep_core::cuboid(
                min,
                std::array::from_fn(|i| min[i] + size[i]),
            )?)
        }
        "sphere-analytic" => solid(brep_core::sphere(field(node, "radius")?)?),
        "cylinder-analytic" => {
            let h: f64 = field(node, "height")?;
            let m = brep_core::frustum(field(node, "radiusBottom")?, field(node, "radiusTop")?, h)?;
            solid(if field::<bool>(node, "center")? {
                brep_core::transform::workplane(
                    &m,
                    [0.; 3],
                    [1., 0., 0.],
                    [0., 1., 0.],
                    [0., 0., -h / 2.],
                )?
            } else {
                m
            })
        }
        "rectangle" | "circle-analytic" | "polygon" => {
            let mut args = node.clone();
            args.as_object_mut()
                .ok_or_else(|| input("Invalid node"))?
                .insert("op".into(), json!("brep_profile_author"));
            if kind == "circle-analytic" {
                args.as_object_mut()
                    .unwrap()
                    .insert("kind".into(), json!("circle"));
            }
            region(super::brep_profile::dispatch(args)?)
        }
        "transform" => {
            let value = unary()?;
            let matrix: [f64; 16] = field(node, "matrix")?;
            if d2 {
                let p = profile(value)?;
                let tolerance = field(p, "toleranceMm")?;
                region(super::brep_profile::profile(
                    brep_core::transform::profile(
                        &field::<Vec<Vec<Curve>>>(p, "loops")?,
                        matrix,
                        tolerance,
                    )?,
                    tolerance,
                )?)
            } else {
                solid(brep_core::transform::affine(
                    &model(value)?,
                    std::array::from_fn(|i| std::array::from_fn(|j| matrix[j * 4 + i])),
                )?)
            }
        }
        "boolean" => {
            let op: String = field(node, "operation")?;
            if !matches!(op.as_str(), "union" | "intersection" | "difference" | "xor") {
                return Err(unsupported("Unsupported semantic Boolean operation"));
            }
            // An authored zero-operand Boolean denotes the typed empty set.
            // Validate the operation first, so emptiness cannot admit unknown ops.
            if inputs.is_empty() {
                return if d2 {
                    region(super::brep_profile::profile(Vec::new(), 1e-7)?)
                } else {
                    solid(brep_core::Model::empty(1e-7)?)
                };
            }
            if d2 {
                let first = profile(&inputs[0])?;
                let mut loops: Vec<Vec<Curve>> = field(first, "loops")?;
                let mut tolerance: f64 = field(first, "toleranceMm")?;
                for next in &inputs[1..] {
                    let p = profile(next)?;
                    tolerance = tolerance.max(field(p, "toleranceMm")?);
                    loops = brep_core::planar_trim::boolean(
                        &loops,
                        &field::<Vec<Vec<Curve>>>(p, "loops")?,
                        &op,
                        tolerance,
                    )?;
                }
                region(super::brep_profile::profile(loops, tolerance)?)
            } else {
                let mut result = model(&inputs[0])?;
                for next in &inputs[1..] {
                    result = brep_core::boolean(&result, &model(next)?, &op)?;
                }
                solid(result)
            }
        }
        "rotate-extrude-analytic" => {
            let p = profile(unary()?)?;
            let loops: Vec<Vec<Curve>> = field(p, "loops")?;
            if loops.len() != 1 {
                let angle: f64 = field(node, "angleDegrees")?;
                return solid(brep_core::revolve_region_angle(
                    &loops,
                    field(p, "toleranceMm")?,
                    angle,
                )?);
            }
            solid(brep_core::revolve_wire_angle(
                &loops[0],
                field(p, "toleranceMm")?,
                field(node, "angleDegrees")?,
            )?)
        }
        "linear-extrude" => {
            let p = profile(unary()?)?;
            let twist: f64 = field(node, "twistDegrees")?;
            let scale: [f64; 2] = field(node, "scale")?;
            if twist != 0. || scale != [1., 1.] {
                return Err(unsupported(
                    "B-rep linear extrusion currently requires zero twist and unit scale",
                ));
            }
            let h: f64 = field(node, "height")?;
            let z = if field::<bool>(node, "center")? {
                -h / 2.
            } else {
                0.
            };
            let loops: Vec<Vec<Curve>> = field(p, "loops")?;
            if loops.is_empty() {
                return solid(brep_core::Model::empty(field(p, "toleranceMm")?)?);
            }
            solid(brep_core::prism::extrude(&loops, z, z + h)?)
        }
        _ => Err(unsupported("Unsupported B-rep semantic node")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_polygon_revolution_uses_native_closed_topology() {
        let p = execute(json!({"node":{"kind":"rectangle","valueType":{"space":"d2"},"size":[2,3],"center":false},"inputs":[]})).unwrap();
        for angle in [360., 90., -90.] {
            let result = execute(json!({"node":{"kind":"rotate-extrude-analytic","valueType":{"space":"d3"},"angleDegrees":angle},"inputs":[p.clone()]})).unwrap();
            let actual = model(&result).unwrap();
            actual.validate().unwrap();
            assert_eq!(actual.bodies.len(), 1);
            assert!(actual.shells.iter().all(|s| s.closed));
            let expected =
                brep_core::revolve_angle(&[[0., 0.], [2., 0.], [2., 3.], [0., 3.]], angle).unwrap();
            assert_eq!(solid(actual).unwrap(), solid(expected).unwrap());
        }
        let circle = execute(json!({"node":{"kind":"circle-analytic","valueType":{"space":"d2"},"radius":1},"inputs":[]})).unwrap();
        assert!(execute(json!({"node":{"kind":"rotate-extrude-analytic","valueType":{"space":"d3"},"angleDegrees":360},"inputs":[circle]})).is_err());
        assert!(execute(json!({"node":{"kind":"rotate-extrude-analytic","valueType":{"space":"d3"},"angleDegrees":0},"inputs":[p]})).is_err());
    }
    #[test]
    fn typed_empty_algebra_precedes_feature_limits() {
        let profile = execute(json!({"node":{"kind":"boolean","operation":"union","valueType":{"space":"d2"}},"inputs":[]})).unwrap();
        let empty_solid = execute(json!({"node":{"kind":"boolean","operation":"union","valueType":{"space":"d3"}},"inputs":[]})).unwrap();
        for (kind, space, operand) in [
            ("transform", "d2", profile.clone()),
            ("linear-extrude", "d3", profile.clone()),
            ("rotate-extrude-analytic", "d3", profile.clone()),
            ("offset", "d2", profile.clone()),
            ("projection", "d2", empty_solid.clone()),
            ("hull", "d3", empty_solid),
        ] {
            let result = execute(
                json!({"node":{"kind":kind,"valueType":{"space":space}},"inputs":[operand]}),
            )
            .unwrap();
            assert!(is_empty(&result).unwrap());
            assert_eq!(
                result["kind"].as_str(),
                Some(if space == "d2" { "profile" } else { "solid" })
            );
        }
        assert!(execute(json!({"node":{"kind":"projection","valueType":{"space":"d2"}},"inputs":[profile.clone()]})).is_err());
        assert!(
            execute(
                json!({"node":{"kind":"unknown","valueType":{"space":"d2"}},"inputs":[profile]})
            )
            .is_err()
        );
        assert!(
            execute(json!({"node":{"kind":"offset","valueType":{"space":"d2"}},"inputs":[]}))
                .is_err()
        );
    }
    #[test]
    fn native_geometry_nodes_compose_profile_transform_and_extrusion() {
        let profile =
            execute(json!({"node":{"kind":"rectangle","valueType":{"space":"d2"},"size":[2,3],"center":true},"inputs":[]}))
                .unwrap();
        assert_eq!(profile["profile"]["areaMm2"].as_f64(), Some(6.));
        let moved=execute(json!({"node":{"kind":"transform","valueType":{"space":"d2"},"matrix":[-1,0,0,0,0,1,0,0,0,0,1,0,5,7,0,1]},"inputs":[profile]})).unwrap();
        let solid=execute(json!({"node":{"kind":"linear-extrude","valueType":{"space":"d3"},"height":4,"center":true,"twistDegrees":0,"scale":[1,1]},"inputs":[moved]})).unwrap();
        let m = model(&solid).unwrap();
        m.validate().unwrap();
        assert!(
            m.vertices
                .iter()
                .all(|v| v.point[2] == -2. || v.point[2] == 2.)
        );
        assert!(
            m.vertices
                .iter()
                .all(|v| v.point[0] >= 4. && v.point[0] <= 6.)
        );
        assert!(execute(json!({"node":{"kind":"transform","valueType":{"space":"d3"},"matrix":[1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1]},"inputs":[]})).is_err());
    }
    #[test]
    fn native_semantic_boolean_keeps_authored_operand_order() {
        let a = execute(json!({"node":{"kind":"box","valueType":{"space":"d3"},"size":[2,2,2],"center":false},"inputs":[]}))
            .unwrap();
        let b = execute(json!({"node":{"kind":"box","valueType":{"space":"d3"},"size":[1,1,1],"center":false},"inputs":[]}))
            .unwrap();
        let removed=execute(json!({"node":{"kind":"boolean","valueType":{"space":"d3"},"operation":"difference"},"inputs":[b.clone(),a.clone()]})).unwrap();
        assert!(model(&removed).unwrap().bodies.is_empty());
        let retained=execute(json!({"node":{"kind":"boolean","valueType":{"space":"d3"},"operation":"difference"},"inputs":[a,b]})).unwrap();
        assert!(!model(&retained).unwrap().bodies.is_empty());
    }
}

#[cfg(test)]
mod admission_tests {
    use super::*;
    #[test]
    fn native_admission_refuses_ignored_inputs_and_space_mismatches() {
        let base = json!({"kind":"box","valueType":{"space":"d3"},"size":[1,1,1],"center":false});
        assert!(execute(json!({"node":base.clone(),"inputs":[{"kind":"solid"}]})).is_err());
        let mut wrong = base.clone();
        wrong["valueType"] = json!({"space":"d2"});
        assert!(execute(json!({"node":wrong,"inputs":[]})).is_err());
        let transform = json!({"kind":"transform","valueType":{"space":"d3"},"matrix":[1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1]});
        assert!(execute(json!({"node":transform,"inputs":[{"kind":"profile"}]})).is_err());
        let no_space = json!({"kind":"boolean","operation":"union"});
        assert!(execute(json!({"node":no_space,"inputs":[]})).is_err());
        assert!(execute(json!({"node":base,"inputs":[]})).is_ok());
    }
}

#[cfg(test)]
mod boolean_admission_tests {
    use super::*;
    #[test]
    fn singleton_boolean_cannot_bypass_operation_admission() {
        let solid=execute(json!({"node":{"kind":"box","valueType":{"space":"d3"},"size":[1,1,1],"center":false},"inputs":[]})).unwrap();
        let region=execute(json!({"node":{"kind":"circle-analytic","valueType":{"space":"d2"},"radius":2},"inputs":[]})).unwrap();
        for (space, operand) in [("d3", solid), ("d2", region)] {
            let bad=execute(json!({"node":{"kind":"boolean","valueType":{"space":space},"operation":"not-a-boolean"},"inputs":[operand.clone()]})).unwrap_err();
            assert_eq!(bad.code, "BREP_UNSUPPORTED_SEMANTIC");
            for operation in ["union", "intersection", "difference", "xor"] {
                let result=execute(json!({"node":{"kind":"boolean","valueType":{"space":space},"operation":operation},"inputs":[operand.clone()]})).unwrap();
                assert_eq!(
                    value_codec::to_string(&result).unwrap(),
                    value_codec::to_string(&operand).unwrap()
                );
            }
        }
    }
}
