//! Transport adapter for the native editor API.
use crate::{Result, encode, field, input};
use polygon_core::{Mesh, mesh_editor as editor};
use value_codec::{Value, json};
pub fn dispatch(v: Value) -> Result<Value> {
    let action = v["action"].as_str().unwrap_or("");
    if action == "sphere" {
        return encode(editor::uv_sphere(
            field(&v, "radius")?,
            field(&v, "segments")?,
            field(&v, "rings")?,
        )?);
    }
    if action == "join" {
        return encode(editor::join(&field::<Vec<Mesh>>(&v, "meshes")?)?);
    }
    let mesh = field::<Mesh>(&v, "mesh")?;
    let selected = || field::<Vec<usize>>(&v, "ids");
    let optional = || {
        if v["ids"].is_null() {
            Ok(None)
        } else {
            selected().map(Some)
        }
    };
    match action {
        "project_faces" => encode(Value::Array(
            editor::project_faces(
                &mesh,
                field(&v, "yaw")?,
                field(&v, "pitch")?,
                field(&v, "orientation")?,
            )?
            .into_iter()
            .map(|(points, face, depth)| json!([points, face, depth]))
            .collect(),
        )),
        "centroid" => encode(editor::centroid(&mesh, &selected()?)?),
        "edges" => encode(editor::edges(&mesh)?),
        "transform" => encode(editor::transform(
            &mesh,
            field(&v, "delta")?,
            field(&v, "angle")?,
            field(&v, "scale")?,
        )?),
        "move" => encode(editor::move_vertices(
            &mesh,
            &selected()?,
            field(&v, "delta")?,
            if v["radius"].is_null() {
                None
            } else {
                Some(field(&v, "radius")?)
            },
        )?),
        "delete" => encode(editor::delete_faces(&mesh, &selected()?)?),
        "flip" => encode(editor::flip_faces(&mesh, optional()?.as_deref())?),
        "subdivide" => encode(editor::subdivide(&mesh, optional()?.as_deref())?),
        "extrude" => encode(editor::extrude(
            &mesh,
            &selected()?,
            field(&v, "distance")?,
        )?),
        "inset" => encode(editor::inset(&mesh, &selected()?, field(&v, "amount")?)?),
        "merge" => encode(editor::merge(&mesh, field(&v, "distance")?)?),
        "knife" => encode(editor::knife(&mesh, &selected()?)?),
        "symmetrize" => encode(editor::symmetrize(&mesh, field(&v, "axis")?)?),
        "separate" => {
            let (kept, separated) = editor::separate(&mesh, &selected()?)?;
            encode(json!({"kept":kept,"separated":separated}))
        }
        _ => Err(input("Unknown mesh editor action")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn compare(actual: &Value, expected: &Value) {
        if let (Some(a), Some(b)) = (actual.as_f64(), expected.as_f64()) {
            assert!((a - b).abs() <= 1e-12 * b.abs().max(1.), "{a} != {b}");
        } else if let (Some(a), Some(b)) = (actual.as_array(), expected.as_array()) {
            assert_eq!(a.len(), b.len());
            for (a, b) in a.iter().zip(b) {
                compare(a, b);
            }
        } else if let Some(b) = expected.as_object() {
            for (key, value) in b {
                compare(&actual[key], value);
            }
        } else {
            assert_eq!(actual, expected);
        }
    }
    #[test]
    fn matches_frozen_typescript_editor_corpus() {
        let cases: Value =
            value_codec::from_str(include_str!("../tests/fixtures/mesh-editor-parity-v1.json"))
                .unwrap();
        assert_eq!(cases.as_array().unwrap().len(), 22);
        for case in cases.as_array().unwrap() {
            compare(&dispatch(case.clone()).unwrap(), &case["expected"]);
        }
    }
    #[test]
    fn rejects_invalid_selection_and_budget_before_editing() {
        let mesh = json!({"positions":[0.,0.,0.,1.,0.,0.,0.,1.,0.],"indices":[0,1,2]});
        assert!(dispatch(json!({"action":"move","mesh":mesh,"ids":[3],"delta":[1,0,0]})).is_err());
        assert!(
            dispatch(json!({"action":"sphere","radius":1,"segments":1000000,"rings":1000000}))
                .is_err()
        );
        assert!(dispatch(json!({"action":"knife","mesh":mesh,"ids":[100]})).is_err());
    }
}
