//! Runtime-local native scene export sessions. Handles are never reused.
use super::{Result, Value, field, input, mesh_analysis};
use polygon_core::solid::export_file::{Builder, MAX_BYTES};
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
pub fn append(id: u32, vertices: &[f32], indices: &[u32], matrix: &[f32]) -> Result<()> {
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
                    .finish()?;
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
