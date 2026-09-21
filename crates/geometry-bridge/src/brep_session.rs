//! Native ownership for reduced semantic geometry. No process-global registry.
use super::{Result, Value, field, input};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Clone, Debug)]
pub struct Lease {
    owner: Arc<()>,
    node: usize,
}
#[derive(Clone, Debug)]
struct Entry {
    value_type: Value,
    empty: bool,
    geometry: Arc<Value>,
    bytes: usize,
    characters: usize,
}
/// Owned immutable results outlive the session only through a committed bundle.
#[derive(Debug)]
pub struct Committed {
    results: Vec<Entry>,
}
impl Committed {
    pub fn outcomes(&self) -> impl Iterator<Item = Outcome<'_>> {
        self.results.iter().map(|entry| {
            if entry.empty {
                Outcome::Empty {
                    value_type: &entry.value_type,
                }
            } else {
                Outcome::Value {
                    value_type: &entry.value_type,
                    geometry: entry.geometry.as_ref(),
                }
            }
        })
    }
    pub fn results(&self) -> impl Iterator<Item = &Value> {
        self.results.iter().map(|entry| entry.geometry.as_ref())
    }
}
/// Typed view of an owned result; empty geometry keeps its declared carrier.
#[derive(Debug)]
pub enum Outcome<'a> {
    Empty {
        value_type: &'a Value,
    },
    Value {
        value_type: &'a Value,
        geometry: &'a Value,
    },
}
pub struct Session {
    owner: Arc<()>,
    entries: BTreeMap<usize, Entry>,
    evaluated: BTreeSet<usize>,
    bytes: usize,
    max_bytes: usize,
    characters: usize,
    max_characters: Option<usize>,
    max_nodes: usize,
    failed: bool,
    closed: bool,
}
impl Session {
    pub fn new(max_nodes: usize, max_bytes: usize) -> Result<Self> {
        if max_nodes == 0 || max_nodes > 65536 || max_bytes == 0 || max_bytes > 64 * 1024 * 1024 {
            return Err(input("Invalid native session budget"));
        }
        Ok(Self {
            owner: Arc::new(()),
            entries: BTreeMap::new(),
            evaluated: BTreeSet::new(),
            bytes: 0,
            max_bytes,
            characters: 0,
            max_characters: None,
            max_nodes,
            failed: false,
            closed: false,
        })
    }
    pub fn with_character_limit(mut self, limit: usize) -> Result<Self> {
        if limit == 0 || limit > 4 * 1024 * 1024 {
            return Err(input("Invalid native character budget"));
        }
        self.max_characters = Some(limit);
        Ok(self)
    }
    fn open(&self) -> Result<()> {
        if self.closed || self.failed {
            Err(input("Native session is closed or failed"))
        } else {
            Ok(())
        }
    }
    fn entry(&self, lease: &Lease) -> Result<&Entry> {
        if !Arc::ptr_eq(&self.owner, &lease.owner) {
            return Err(input("Foreign native result lease"));
        }
        self.entries
            .get(&lease.node)
            .ok_or_else(|| input("Released native result lease"))
    }
    /// Borrow immutable geometry while its owner and lease are still live.
    pub fn snapshot(&self, lease: &Lease) -> Result<&Value> {
        self.open()?;
        Ok(self.entry(lease)?.geometry.as_ref())
    }
    pub fn outcome(&self, lease: &Lease) -> Result<Outcome<'_>> {
        self.open()?;
        let entry = self.entry(lease)?;
        Ok(if entry.empty {
            Outcome::Empty {
                value_type: &entry.value_type,
            }
        } else {
            Outcome::Value {
                value_type: &entry.value_type,
                geometry: entry.geometry.as_ref(),
            }
        })
    }
    pub fn retained_bytes(&self) -> usize {
        self.bytes
    }
    pub fn evaluate(&mut self, node: Value, inputs: &[Lease]) -> Result<Lease> {
        self.evaluate_reduced(node, None, inputs)
    }
    /// Preserve authored operand order when the executor removes typed empties.
    pub fn evaluate_reduced(
        &mut self,
        node: Value,
        reduced: Option<Vec<usize>>,
        inputs: &[Lease],
    ) -> Result<Lease> {
        self.open()?;
        let result = (|| {
            let id = node
                .get("id")
                .and_then(Value::as_u64)
                .and_then(|id| usize::try_from(id).ok())
                .ok_or_else(|| {
                    super::Error::new(
                        "BREP_SEMANTIC_CONTRACT",
                        "Native node identity must be a nonnegative integer",
                    )
                })?;
            if id >= self.max_nodes || self.evaluated.contains(&id) {
                return Err(super::Error::new(
                    "BREP_SEMANTIC_CONTRACT",
                    "Invalid or repeated native node identity",
                ));
            }
            let refs = references(&node)?;
            let refs = if let Some(reduced) = reduced {
                let mut authored = refs.iter();
                let ordered = reduced.iter().all(|id| authored.any(|source| source == id));
                if !ordered || (node["kind"].as_str() != Some("boolean") && reduced != refs) {
                    return Err(super::Error::new(
                        "BREP_SEMANTIC_CONTRACT",
                        "B-rep input order differs from the authored node",
                    ));
                }
                reduced
            } else {
                refs
            };
            if refs.len() != inputs.len() {
                return Err(input("Native input reference count mismatch"));
            }
            let mut geometry = Vec::with_capacity(inputs.len());
            for (reference, lease) in refs.iter().zip(inputs) {
                let entry = self.entry(lease)?;
                if *reference != lease.node || *reference >= id {
                    return Err(input(
                        "Native input reference mismatch or forward reference",
                    ));
                }
                geometry.push(entry.geometry.as_ref().clone());
            }
            let value_type = node
                .get("valueType")
                .cloned()
                .ok_or_else(|| input("Missing native result type"))?;
            validate_carrier(&value_type)?;
            let value =
                super::brep_semantic::execute(value_codec::json!({"node":node,"inputs":geometry}))?;
            let empty = match value["kind"].as_str() {
                Some("profile") => value["profile"]["loops"]
                    .as_array()
                    .is_some_and(Vec::is_empty),
                Some("solid") => ["vertices", "edges", "loops", "faces", "shells", "bodies"]
                    .iter()
                    .all(|key| value["model"][*key].as_array().is_some_and(Vec::is_empty)),
                _ => return Err(input("Unknown native result kind")),
            };
            if !empty {
                validate_result(&value_type, &value)?;
            }
            let characters = if empty {
                0
            } else {
                super::brep_json_size::characters(if value["kind"].as_str() == Some("solid") {
                    &value["model"]
                } else {
                    &value["profile"]
                })
            };
            if self
                .max_characters
                .is_some_and(|limit| characters > limit - self.characters)
            {
                return Err(super::Error::new(
                    "BREP_SEMANTIC_BUDGET",
                    "B-rep execution exceeds its aggregate retained geometry snapshot limit",
                ));
            }
            let type_bytes = value_codec::to_string(&value_type)
                .map_err(|e| input(e.to_string()))?
                .len();
            let bytes = value_codec::to_string(&value)
                .map_err(|e| input(e.to_string()))?
                .len()
                .checked_add(type_bytes)
                .ok_or_else(|| input("Native result size overflow"))?;
            if bytes > self.max_bytes - self.bytes {
                return Err(input("Native retained geometry budget exceeded"));
            }
            self.entries.insert(
                id,
                Entry {
                    value_type,
                    empty,
                    geometry: Arc::new(value),
                    bytes,
                    characters,
                },
            );
            self.evaluated.insert(id);
            self.bytes += bytes;
            self.characters += characters;
            Ok(Lease {
                owner: Arc::clone(&self.owner),
                node: id,
            })
        })();
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    pub fn release(&mut self, lease: &Lease) -> Result<()> {
        if !Arc::ptr_eq(&self.owner, &lease.owner) || !self.evaluated.contains(&lease.node) {
            return Err(input("Foreign native result lease"));
        }
        if self.closed {
            return Err(input("Native session is closed"));
        }
        if let Some(entry) = self.entries.remove(&lease.node) {
            self.bytes -= entry.bytes;
            self.characters -= entry.characters;
        }
        Ok(())
    }
    pub fn abort(&mut self) {
        self.entries.clear();
        self.bytes = 0;
        self.characters = 0;
        self.closed = true;
    }
    pub fn commit(&mut self, retained: &[Lease]) -> Result<Committed> {
        if let Err(error) = self.open() {
            self.abort();
            return Err(error);
        }
        let result = (|| {
            let mut ids = BTreeSet::new();
            let mut results = Vec::with_capacity(retained.len());
            for lease in retained {
                let entry = self.entry(lease)?;
                if !ids.insert(lease.node) {
                    return Err(input("Duplicate retained native lease"));
                }
                results.push(entry.clone());
            }
            Ok(Committed { results })
        })();
        self.abort();
        result
    }
}
pub(crate) fn references(node: &Value) -> Result<Vec<usize>> {
    Ok(match node["kind"].as_str() {
        Some("boolean" | "hull") => field(node, "inputs")?,
        Some(
            "transform" | "linear-extrude" | "rotate-extrude-analytic" | "projection" | "offset",
        ) => vec![field(node, "input")?],
        Some(
            "box" | "sphere-analytic" | "cylinder-analytic" | "rectangle" | "circle-analytic"
            | "polygon",
        ) => vec![],
        _ => {
            return Err(super::Error::new(
                "BREP_UNSUPPORTED_SEMANTIC",
                "Unsupported B-rep semantic node",
            ));
        }
    })
}

/// Native admission for the supported semantic carrier and evidence contract.
fn validate_carrier(value_type: &Value) -> Result<()> {
    let supported = matches!(
        (
            value_type["geometryKind"].as_str(),
            value_type["space"].as_str()
        ),
        (Some("solid" | "solid-set"), Some("d3")) | (Some("region"), Some("d2"))
    );
    if !supported
        || value_type["representation"].as_str() != Some("analytic-brep")
        || value_type["evidence"]["tag"].as_str() != Some("representation-preserving")
    {
        return Err(super::Error::new(
            "BREP_UNSUPPORTED_SEMANTIC",
            "Unsupported B-rep semantic carrier or evidence",
        ));
    }
    Ok(())
}

/// Validate before publishing a lease. This is topology admission, not a solid
/// certificate or a proof of numerical correctness.
fn validate_result(value_type: &Value, value: &Value) -> Result<()> {
    let profile = value["kind"].as_str() == Some("profile");
    if profile != (value_type["space"].as_str() == Some("d2")) {
        return Err(input(
            "B-rep result dimension does not match its semantic carrier",
        ));
    }
    if !profile {
        let model: brep_core::Model = field(value, "model")?;
        if model.bodies.is_empty()
            || model.shells.iter().any(|shell| !shell.closed)
            || (value_type["geometryKind"].as_str() == Some("solid") && model.bodies.len() != 1)
        {
            return Err(input("B-rep result does not satisfy its solid carrier"));
        }
        model.validate()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use value_codec::json;
    fn cube(id: usize) -> Value {
        json!({"id":id,"kind":"box","valueType":{"space":"d3","geometryKind":"solid","representation":"analytic-brep","evidence":{"tag":"representation-preserving"}},"size":[1,1,1],"center":false})
    }
    #[test]
    fn result_admission_checks_dimension_closed_shells_and_topology() {
        let result =
            super::super::brep_semantic::execute(json!({"node":cube(0),"inputs":[]})).unwrap();
        let carrier = json!({"space":"d3","geometryKind":"solid"});
        validate_result(&carrier, &result).unwrap();
        assert!(validate_result(&json!({"space":"d2"}), &result).is_err());
        let mut open = result.clone();
        open["model"]["shells"][0]["closed"] = json!(false);
        assert!(validate_result(&carrier, &open).is_err());
        let mut bodyless = result.clone();
        bodyless["model"]["bodies"] = json!([]);
        assert!(validate_result(&carrier, &bodyless).is_err());
        let mut invalid = result.clone();
        invalid["model"]["vertices"] = json!([]);
        assert!(validate_result(&carrier, &invalid).is_err());
        let mut multiple = result.clone();
        let body = multiple["model"]["bodies"][0].clone();
        multiple["model"]["bodies"] = json!([body.clone(), body]);
        assert!(validate_result(&carrier, &multiple).is_err());
    }
    #[test]
    fn ownership_release_and_commit_are_atomic() {
        let mut a = Session::new(8, 1_000_000).unwrap();
        let mut b = Session::new(8, 1_000_000).unwrap();
        let x = a.evaluate(cube(0), &[]).unwrap();
        let y = b.evaluate(cube(0), &[]).unwrap();
        assert!(a.release(&y).is_err());
        assert!(a.snapshot(&y).is_err());
        assert_eq!(a.snapshot(&x).unwrap()["kind"].as_str(), Some("solid"));
        let bytes = a.retained_bytes();
        assert!(bytes > 0);
        let bundle = a.commit(std::slice::from_ref(&x)).unwrap();
        assert_eq!(a.retained_bytes(), 0);
        assert_eq!(bundle.results().count(), 1);
        assert!(a.evaluate(cube(1), &[]).is_err());
        assert!(a.release(&x).is_err());
        assert!(a.snapshot(&x).is_err());
        b.release(&y).unwrap();
        assert!(b.snapshot(&y).is_err());
        b.release(&y).unwrap();
        assert_eq!(b.retained_bytes(), 0);
        assert!(b.commit(&[y]).is_err());
        assert_eq!(b.retained_bytes(), 0);
    }
    #[test]
    fn failed_evaluation_never_publishes_and_poisoned_session_cannot_commit() {
        let mut s = Session::new(8, 1).unwrap();
        assert!(s.evaluate(cube(0), &[]).is_err());
        assert_eq!(s.retained_bytes(), 0);
        assert!(s.commit(&[]).is_err());
        s.abort();
        let mut s = Session::new(8, 1_000_000).unwrap();
        let a = s.evaluate(cube(0), &[]).unwrap();
        let before = s.retained_bytes();
        assert!(s.evaluate(cube(0), &[]).is_err());
        assert_eq!(s.retained_bytes(), before);
        assert!(s.commit(&[a]).is_err());
        assert_eq!(s.retained_bytes(), 0);
        s.abort();
        assert_eq!(s.retained_bytes(), 0);
    }
    #[test]
    fn references_and_duplicate_commit_are_checked() {
        let mut s = Session::new(8, 1_000_000).unwrap();
        let a = s.evaluate(cube(0), &[]).unwrap();
        let node = json!({"id":1,"kind":"boolean","valueType":{"space":"d3","geometryKind":"solid","representation":"analytic-brep","evidence":{"tag":"representation-preserving"}},"operation":"union","inputs":[0]});
        let b = s.evaluate(node, std::slice::from_ref(&a)).unwrap();
        s.release(&a).unwrap();
        assert!(s.commit(&[b.clone(), b]).is_err());
        assert_eq!(s.retained_bytes(), 0);
    }
    #[test]
    fn evaluation_refuses_foreign_released_and_mismatched_references() {
        for case in 0..3 {
            let mut source = Session::new(8, 1_000_000).unwrap();
            let mut target = Session::new(8, 1_000_000).unwrap();
            let foreign = source.evaluate(cube(0), &[]).unwrap();
            let local = target.evaluate(cube(0), &[]).unwrap();
            if case == 1 {
                target.release(&local).unwrap();
            }
            let reference = if case == 2 { 1 } else { 0 };
            let node = json!({"id":2,"kind":"boolean","valueType":{"space":"d3","geometryKind":"solid","representation":"analytic-brep","evidence":{"tag":"representation-preserving"}},"operation":"union","inputs":[reference]});
            let lease = if case == 0 { &foreign } else { &local };
            let before = target.retained_bytes();
            assert!(target.evaluate(node, std::slice::from_ref(lease)).is_err());
            assert_eq!(target.retained_bytes(), before);
            assert!(target.commit(&[local]).is_err());
            assert_eq!(target.retained_bytes(), 0);
            // Failure in the receiving session must not revoke the source owner.
            assert_eq!(source.commit(&[foreign]).unwrap().results().count(), 1);
        }
    }
    #[test]
    fn aggregate_budget_failure_preserves_existing_entries_until_close() {
        let mut probe = Session::new(8, 1_000_000).unwrap();
        probe.evaluate(cube(0), &[]).unwrap();
        let one = probe.retained_bytes();
        let mut session = Session::new(8, one + 1).unwrap();
        let first = session.evaluate(cube(0), &[]).unwrap();
        assert!(session.evaluate(cube(1), &[]).is_err());
        assert_eq!(session.retained_bytes(), one);
        assert!(session.commit(&[first]).is_err());
        assert_eq!(session.retained_bytes(), 0);
    }
    #[test]
    fn typed_empty_survives_reuse_and_commit() {
        for space in ["d2", "d3"] {
            let mut session = Session::new(8, 1_000_000).unwrap();
            let node = if space == "d3" {
                cube(0)
            } else {
                json!({"id":0,"kind":"circle-analytic","valueType":{"space":"d2","geometryKind":"region","representation":"analytic-brep","evidence":{"tag":"representation-preserving"}},"radius":2})
            };
            let a = session.evaluate(node, &[]).unwrap();
            let ty = json!({"space":space,"geometryKind":if space=="d2" {"region"} else {"solid-set"},"representation":"analytic-brep","evidence":{"tag":"representation-preserving"}});
            let empty=session.evaluate(json!({"id":1,"kind":"boolean","valueType":ty.clone(),"operation":"difference","inputs":[0,0]}),&[a.clone(),a.clone()]).unwrap();
            match session.outcome(&empty).unwrap() {
                Outcome::Empty { value_type } => assert_eq!(value_type, &ty),
                _ => panic!("Expected typed empty result"),
            }
            let restored=session.evaluate(json!({"id":2,"kind":"boolean","valueType":ty.clone(),"operation":"union","inputs":[1,0]}),&[empty.clone(),a]).unwrap();
            assert!(matches!(
                session.outcome(&restored).unwrap(),
                Outcome::Value { .. }
            ));
            let committed = session.commit(&[empty, restored]).unwrap();
            let mut outcomes = committed.outcomes();
            assert!(matches!(outcomes.next(),Some(Outcome::Empty{value_type}) if value_type==&ty));
            assert!(matches!(outcomes.next(), Some(Outcome::Value { .. })));
            assert!(outcomes.next().is_none());
        }
    }
    #[test]
    fn node_identity_admission_is_native_and_stable_after_release() {
        for id in [json!(-1), json!(0.5), json!("0"), json!(8)] {
            let mut session = Session::new(8, 1_000_000).unwrap();
            let mut node = cube(0);
            node["id"] = id;
            assert_eq!(
                session.evaluate(node, &[]).unwrap_err().code,
                "BREP_SEMANTIC_CONTRACT"
            );
            assert_eq!(session.retained_bytes(), 0);
        }
        let mut session = Session::new(8, 1_000_000).unwrap();
        let lease = session.evaluate(cube(0), &[]).unwrap();
        session.release(&lease).unwrap();
        assert_eq!(
            session.evaluate(cube(0), &[]).unwrap_err().code,
            "BREP_SEMANTIC_CONTRACT"
        );
    }
    #[test]
    fn unsupported_carriers_and_evidence_never_publish() {
        for (key, value) in [
            ("geometryKind", json!("region")),
            ("space", json!("d2")),
            ("representation", json!("mesh")),
            ("evidence", json!({"tag":"certified"})),
            ("evidence", Value::Null),
        ] {
            let mut session = Session::new(8, 1_000_000).unwrap();
            let mut node = cube(0);
            node["valueType"][key] = value;
            assert_eq!(
                session.evaluate(node, &[]).unwrap_err().code,
                "BREP_UNSUPPORTED_SEMANTIC"
            );
            assert_eq!(session.retained_bytes(), 0);
            assert!(session.commit(&[]).is_err());
        }
    }
    #[test]
    fn reduced_operands_preserve_authored_order_and_multiplicity() {
        for (refs, valid) in [
            (vec![1, 0], true),
            (vec![1, 1], false),
            (vec![0, 0, 1], false),
        ] {
            let mut session = Session::new(8, 1_000_000).unwrap();
            let a = session.evaluate(cube(0), &[]).unwrap();
            let b = session.evaluate(cube(1), &[]).unwrap();
            let leases: Vec<_> = refs
                .iter()
                .map(|id| if *id == 0 { a.clone() } else { b.clone() })
                .collect();
            let node = json!({"id":2,"kind":"boolean","operation":"union","inputs":[0,1,0],"valueType":cube(0)["valueType"].clone()});
            let result = session.evaluate_reduced(node, Some(refs), &leases);
            if valid {
                assert!(result.is_ok());
            } else {
                assert_eq!(result.unwrap_err().code, "BREP_SEMANTIC_CONTRACT");
                assert!(session.commit(&[a]).is_err());
            }
        }
    }
    #[test]
    fn native_empty_boolean_algebra_and_extrusion_need_no_host_reduction() {
        for space in ["d2", "d3"] {
            for op in ["union", "intersection", "difference", "xor"] {
                for empty_first in [false, true] {
                    let mut session = Session::new(8, 1_000_000).unwrap();
                    let ty = json!({"space":space,"geometryKind":if space=="d2" {"region"} else {"solid-set"},"representation":"analytic-brep","evidence":{"tag":"representation-preserving"}});
                    let empty = session.evaluate(json!({"id":0,"kind":"boolean","operation":op,"inputs":[],"valueType":ty.clone()}), &[]).unwrap();
                    assert!(matches!(
                        session.outcome(&empty).unwrap(),
                        Outcome::Empty { .. }
                    ));
                    let mut primitive = if space == "d3" {
                        cube(1)
                    } else {
                        json!({"id":1,"kind":"circle-analytic","radius":2})
                    };
                    primitive["valueType"] = ty.clone();
                    let a = session.evaluate(primitive, &[]).unwrap();
                    let (refs, leases) = if empty_first {
                        (vec![0, 1], vec![empty.clone(), a])
                    } else {
                        (vec![1, 0], vec![a, empty.clone()])
                    };
                    let result = session.evaluate(json!({"id":2,"kind":"boolean","operation":op,"inputs":refs,"valueType":ty}), &leases).unwrap();
                    assert_eq!(
                        matches!(session.outcome(&result).unwrap(), Outcome::Empty { .. }),
                        op == "intersection" || (op == "difference" && empty_first)
                    );
                    if space == "d2" {
                        let solid_type = cube(0)["valueType"].clone();
                        let extruded = session.evaluate(json!({"id":3,"kind":"linear-extrude","input":0,"valueType":solid_type,"height":2,"center":false,"twistDegrees":0,"scale":[1,1]}), &[empty]).unwrap();
                        assert!(matches!(
                            session.outcome(&extruded).unwrap(),
                            Outcome::Empty { .. }
                        ));
                    }
                }
            }
        }
    }
}
