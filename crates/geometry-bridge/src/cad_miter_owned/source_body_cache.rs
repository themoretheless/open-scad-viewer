//! Bounded reuse of immutable, native constructor-owned source bodies.
//! Placement, actual-model binding, contacts and material admission stay fresh.
use super::{Rc, Value, body_value, sweep_miter_owned};
use std::{cell::RefCell, collections::VecDeque};

type Body = Rc<sweep_miter_owned::BoundaryBody>;
type Entry = (Vec<u8>, Body, usize);
const MAX_ENTRIES: usize = 2;
const MAX_SOURCE_BYTES: usize = 1 << 20;
const MAX_PAYLOAD_BYTES: usize = 4 << 20;
thread_local! {
    static ENTRIES: RefCell<VecDeque<Entry>> = const { RefCell::new(VecDeque::new()) };
}

pub(super) fn key(mut source: Value) -> Option<Vec<u8>> {
    source.as_object_mut()?.remove("op");
    let bytes = value_codec::encode_binary(&source).ok()?;
    (bytes.len() <= MAX_SOURCE_BYTES).then_some(bytes)
}

pub(super) fn get(source: &[u8]) -> Option<Body> {
    ENTRIES.with(|entries| {
        entries
            .borrow()
            .iter()
            .find(|(key, _, _)| key == source)
            .map(|(_, body, _)| body.clone())
    })
}

pub(super) fn insert(key: Vec<u8>, body: Body) {
    if body.complete_boundary().is_none() {
        return;
    }
    // Bound retained serialized payload as well as entry count. Include the
    // correction's full sections, which the public summary deliberately omits.
    // This is a payload limit, not a claim about allocator overhead in bytes.
    let Ok(value) = body_value(&body, body.model(), body.boundary(), false, None) else {
        return;
    };
    let Ok(bytes) = value_codec::encode_binary(&value) else {
        return;
    };
    let correction = body.correction().and_then(|r| r.sections.as_ref());
    let Ok(extra) = value_codec::encode_binary(&value_codec::json!(correction)) else {
        return;
    };
    let weight = key
        .len()
        .saturating_add(bytes.len())
        .saturating_add(extra.len());
    if weight > MAX_PAYLOAD_BYTES {
        return;
    }
    ENTRIES.with(|entries| {
        let mut entries = entries.borrow_mut();
        while entries.len() >= MAX_ENTRIES
            || entries.iter().map(|(_, _, weight)| weight).sum::<usize>() + weight
                > MAX_PAYLOAD_BYTES
        {
            entries.pop_front();
        }
        entries.push_back((key, body, weight));
    });
}

#[cfg(test)]
mod tests {
    use super::super::{construct_body, place};
    use super::*;
    use value_codec::json;

    fn source() -> Value {
        let curve = nurbs_core::primitives::circle([0.; 3], [0., 0., 1.], 0.5).unwrap();
        json!({"op":"brep_miter_owned_construct","direct":true,"loops":[[curve]],
            "points":[[0.,0.,0.],[0.,0.,10.]],"normal":[1.,0.,0.],
            "closed":false,"miterLimit":2.,"maxDeviation":0.01})
    }
    fn clear() {
        ENTRIES.with(|entries| entries.borrow_mut().clear());
    }
    fn bytes(body: &Body) -> Vec<u8> {
        value_codec::encode_binary(
            &body_value(body, body.model(), body.boundary(), true, None).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn exact_source_reuse_preserves_fresh_geometry_and_all_reported_work() {
        clear();
        let request = source();
        let first = construct_body(request.clone()).unwrap();
        assert!(first.complete_boundary().is_some());
        let mut nested = request.clone();
        nested.as_object_mut().unwrap().remove("op");
        let repeated = construct_body(nested).unwrap();
        assert!(Rc::ptr_eq(&first, &repeated));
        assert_eq!(bytes(&first), bytes(&repeated));
        clear();
        let fresh = construct_body(request).unwrap();
        assert!(!Rc::ptr_eq(&first, &fresh));
        assert_eq!(bytes(&first), bytes(&fresh));
    }

    #[test]
    fn source_bits_limits_and_forbidden_evidence_cannot_reuse_positive_body() {
        clear();
        let request = source();
        let original = construct_body(request.clone()).unwrap();
        let mut changed = request.clone();
        changed["points"][0][0] = json!(-0.0_f64);
        assert_ne!(key(request.clone()), key(changed.clone()));
        assert!(!Rc::ptr_eq(&original, &construct_body(changed).unwrap()));
        let mut tighter = request.clone();
        tighter["maxDeviation"] = json!(0.005);
        assert!(!Rc::ptr_eq(&original, &construct_body(tighter).unwrap()));
        let mut short = request.clone();
        short["limits"] = json!({"maxProducts":100000,"maxFaces":1024,"exactWork":0,
            "domainTolerance":0.001,"domainPairs":1000,"domainCells":10000,"projectionCells":10000,
            "capRegions":{"maxWalls":1024,"maxExactWork":0,"maxChartCells":1000,
                "maxTrimPairs":100000,"maxTrimCells":100000,"maxTrimDomainCells":1000000}});
        assert!(get(&key(short.clone()).unwrap()).is_none());
        let result = construct_body(short.clone());
        assert!(match &result {
            Err(_) => true,
            Ok(body) => body.complete_boundary().is_none(),
        });
        assert!(get(&key(short).unwrap()).is_none());
        for name in [
            "model",
            "boundaryCertificate",
            "retainedWallCharts",
            "sourceSections",
            "retainedSmoothness",
        ] {
            let mut injected = request.clone();
            injected[name] = Value::Null;
            assert!(construct_body(injected).is_err(), "{name}");
        }
    }

    #[test]
    fn cached_source_still_replays_placement_and_binds_actual_result_model() {
        clear();
        let source = source();
        let original = construct_body(source.clone()).unwrap();
        let request = json!({"source":source,
            "matrix":[[2.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.],[0.,0.,0.,1.]],
            "quantum":2_f64.powi(-40),"maxWork":100000,"budget":1.});
        let result = place(request.clone()).unwrap();
        assert!(!result["placement"]["model"].is_null());
        let mut bound = request.clone();
        bound["expectedResultModel"] = result["placement"]["model"].clone();
        assert_eq!(place(bound.clone()).unwrap()["resultModelBound"], true);
        bound["expectedResultModel"]["faces"][0]["surface"]["controlPoints"][0][0][0] = json!(123.);
        assert!(place(bound).is_err());
        let mut zero = request.clone();
        zero["maxWork"] = json!(0);
        assert!(place(zero).unwrap()["placement"]["model"].is_null());
        assert!(Rc::ptr_eq(
            &original,
            &construct_body(request["source"].clone()).unwrap()
        ));
    }

    #[test]
    fn retained_entries_and_payload_are_bounded_and_large_sources_bypass_reuse() {
        clear();
        for z in [10., 11., 12.] {
            let mut request = source();
            request["points"][1][2] = json!(z);
            assert!(
                construct_body(request)
                    .unwrap()
                    .complete_boundary()
                    .is_some()
            );
        }
        ENTRIES.with(|entries| {
            let entries = entries.borrow();
            assert_eq!(entries.len(), MAX_ENTRIES);
            assert!(
                entries.iter().map(|(_, _, weight)| weight).sum::<usize>() <= MAX_PAYLOAD_BYTES
            );
        });
        let mut oversized = source();
        oversized["extra"] = json!("x".repeat(MAX_SOURCE_BYTES));
        assert!(key(oversized).is_none());
    }
}
