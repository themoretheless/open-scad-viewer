//! Runtime-local native scene export sessions. Handles are never reused.
use super::{Result, Value, field, input, legacy_mesh_error, mesh_analysis};
#[cfg(test)]
use crate::Error;
use mesh_io::export_file::{Builder, MAX_BYTES};
use std::collections::BTreeMap;
use value_codec::json;
#[derive(Default)]
struct Registry {
    next: u32,
    builders: BTreeMap<u32, Builder>,
}
thread_local! {static REGISTRY:std::cell::RefCell<Registry>=std::cell::RefCell::new(Registry::default());}
pub fn poison(id: u32) {
    REGISTRY.with(|r| {
        if let Some(b) = r.borrow_mut().builders.get_mut(&id) {
            b.poison()
        }
    })
}
pub fn append(id: u32, vertices: &[f64], indices: &[u32], matrix: &[f64]) -> Result<()> {
    REGISTRY.with(|registry| {
        let mut r = registry.borrow_mut();
        let others: usize = r
            .builders
            .iter()
            .filter(|(key, _)| **key != id)
            .map(|(_, b)| b.len())
            .sum();
        r.builders
            .get_mut(&id)
            .ok_or_else(|| input("Unknown or disposed export session"))?
            .append(vertices, indices, matrix, MAX_BYTES.saturating_sub(others))
            .map_err(legacy_mesh_error)
    })
}
pub fn dispatch(v: Value) -> Result<Value> {
    REGISTRY.with(|registry| {
        let mut r = registry.borrow_mut();
        match v["action"].as_str() {
            Some("begin") => {
                let format: String = field(&v, "format")?;
                if format != "stl" && format != "obj" {
                    return Err(input("Unsupported scene export format"));
                }
                if r.builders.len() >= 8
                    || r.builders.values().map(Builder::len).sum::<usize>() > MAX_BYTES - 84
                {
                    return Err(input("Export session budget exceeded"));
                }
                let next = r
                    .next
                    .checked_add(1)
                    .ok_or_else(|| input("Export handle space exhausted"))?;
                r.builders.insert(
                    next,
                    Builder::new(format == "stl", &field::<String>(&v, "name")?),
                );
                r.next = next;
                Ok(json!(next))
            }
            Some("finish") => {
                let id: u32 = field(&v, "handle")?;
                let bytes = r
                    .builders
                    .remove(&id)
                    .ok_or_else(|| input("Unknown or disposed export session"))?
                    .finish()
                    .map_err(legacy_mesh_error)?;
                Ok(json!(mesh_analysis::store(
                    mesh_analysis::AnalysisBuffers::Bytes { bytes }
                )))
            }
            Some("dispose") => {
                r.builders.remove(&field::<u32>(&v, "handle")?);
                Ok(Value::Null)
            }
            _ => Err(input("Unknown export session action")),
        }
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_session_cannot_finish_and_handles_do_not_recur() {
        let id = dispatch(json!({"action":"begin","format":"obj","name":""}))
            .unwrap()
            .as_u64()
            .unwrap() as u32;
        poison(id);
        assert!(dispatch(json!({"action":"finish","handle":id})).is_err());
        assert!(append(id, &[], &[], &[]).is_err());
        let next = dispatch(json!({"action":"begin","format":"stl","name":""}))
            .unwrap()
            .as_u64()
            .unwrap() as u32;
        assert!(next > id);
        dispatch(json!({"action":"dispose","handle":next})).unwrap();
    }
}

#[cfg(test)]
mod wire_tests {
    use super::*;
    #[test]
    fn invalid_scene_data_preserves_error_and_poison_contract() {
        let id = dispatch(json!({"action":"begin","format":"stl","name":""}))
            .unwrap()
            .as_u64()
            .unwrap() as u32;
        let error = append(id, &[1.], &[], &[0.; 16]).unwrap_err();
        assert_eq!(error.code, "POLYGON_INVALID_INPUT");
        assert_eq!(
            error.message,
            "Mesh export requires finite affine triangle data"
        );
        let error = dispatch(json!({"action":"finish","handle":id})).unwrap_err();
        assert_eq!(error.code, "POLYGON_INVALID_INPUT");
        assert!(error.message.contains("failed export session"));
        assert_eq!(
            legacy_mesh_error(Error::new("MESH_EXPORT_TOO_MANY_TRIANGLES", "budget")).code,
            "MESH_EXPORT_TOO_MANY_TRIANGLES"
        );
    }
}
