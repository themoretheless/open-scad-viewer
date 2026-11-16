use super::*;
use crate::mesh::{BooleanJobOp, FACE_ID_COMPUTATIONS};

fn call(v: Value) -> Value {
    crate::mesh::dispatch(v).unwrap()
}
fn id(v: Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn translated(source: u32, x: f64) -> u32 {
    let t = id(call(json!({"action":"transform","id":source,"operation":"translate","offset":[x,0.,0.]})));
    delete(&[source]);
    t
}
fn sphere(r: f64, x: f64) -> u32 {
    translated(id(call(json!({"action":"sphere","radius":r,"segments":24}))), x)
}
fn cube(size: f64, x: f64) -> u32 {
    translated(id(call(json!({"action":"cube","size":[size,size,size],"center":true}))), x)
}
fn delete(ids: &[u32]) {
    call(json!({"action":"delete","ids":ids}));
}
fn mesh_of(id: u32) -> (Vec<f64>, Vec<u32>, Vec<u32>) {
    let b = crate::mesh::export_buffers(id).unwrap();
    (b.positions, b.indices, b.face_ids)
}
fn run(job: u32, budget: u32) -> (Value, u64) {
    let mut calls = 0;
    loop {
        calls += 1;
        let v = step(job, budget).unwrap();
        if v["state"] == "complete" {
            return (v["result"].clone(), calls);
        }
        assert_eq!(v["state"], "pending");
        assert!(v["done"].as_u64().unwrap() <= v["total"].as_u64().unwrap());
    }
}
fn operands() -> Vec<u32> {
    // Overlapping and separated operands exercise bound groups and batches.
    vec![cube(4., 0.), sphere(1.5, 1.5), sphere(1., -1.8), cube(1., 9.), sphere(0.8, 9.4)]
}

#[test]
fn boolean_steps_match_one_shot_combine_for_every_operation_and_budget() {
    for (op, name) in [
        (BooleanJobOp::Union, "union"),
        (BooleanJobOp::Difference, "difference"),
        (BooleanJobOp::Intersection, "intersection"),
    ] {
        let ids = operands();
        let expected = id(call(json!({"action":"combine","ids":ids,"dimension":3,"operation":name})));
        let expected_mesh = mesh_of(expected);
        for budget in [1, 2, 1000] {
            let job = start_boolean(&ids, op).unwrap();
            let (result, _) = run(job, budget);
            let got = id(result);
            assert_eq!(mesh_of(got), expected_mesh, "{name} budget {budget}");
            delete(&[got]);
        }
        delete(&[expected]);
        delete(&ids);
    }
    assert_eq!(live_jobs(), 0);
}

#[test]
fn boolean_step_reports_bounded_progress_and_cancel_frees_state() {
    let ids = operands();
    let job = start_boolean(&ids, BooleanJobOp::Union).unwrap();
    let first = step(job, 1).unwrap();
    assert_eq!(first["state"], "pending");
    assert_eq!(first["kind"], "mesh-boolean");
    assert_eq!(first["steps"].as_u64(), Some(1));
    assert!(first["total"].as_u64().unwrap() >= 2);
    assert_eq!(live_jobs(), 1);
    cancel(job);
    assert_eq!(live_jobs(), 0);
    assert!(step(job, 1).is_err(), "cancelled handles refuse");
    cancel(job); // idempotent
    // Handles are never recycled, and a later job still runs to the same result.
    let next = start_boolean(&ids, BooleanJobOp::Union).unwrap();
    assert!(next > job);
    let (result, _) = run(next, 1);
    delete(&[id(result)]);
    delete(&ids);
}

#[test]
fn cancellation_at_every_step_leaves_no_jobs() {
    let ids = operands();
    let job = start_boolean(&ids, BooleanJobOp::Difference).unwrap();
    let (result, total_calls) = run(job, 1);
    delete(&[id(result)]);
    assert!(total_calls >= 2);
    for cut in 0..total_calls - 1 {
        let job = start_boolean(&ids, BooleanJobOp::Difference).unwrap();
        for _ in 0..cut {
            assert_eq!(step(job, 1).unwrap()["state"], "pending");
        }
        cancel(job);
        assert_eq!(live_jobs(), 0);
    }
    delete(&ids);
}

#[test]
fn render_job_matches_one_shot_render_and_owns_its_source() {
    let s = id(call(json!({"action":"sphere","radius":3,"segments":160})));
    let expected = crate::mesh::render_buffers(s, 0.6).unwrap();
    let results = crate::mesh_analysis::live_result_count();
    let job = start_render(s, 0.6).unwrap();
    let first = step(job, 1).unwrap();
    assert_eq!(first["kind"], "render-prepare");
    assert_eq!(first["state"], "pending");
    // Deleting the source mid-job cannot invalidate the owned snapshot.
    delete(&[s]);
    let (result, calls) = run(job, 1);
    assert!(calls >= 2, "render preparation yields between batches");
    let got = crate::mesh_analysis::take_render(result.as_u64().unwrap() as usize).unwrap();
    assert_eq!(got.vertices, expected.vertices);
    assert_eq!(got.indices, expected.indices);
    assert_eq!(got.merge_from, expected.merge_from);
    assert_eq!(got.merge_to, expected.merge_to);
    assert_eq!(got.face_ids, expected.face_ids);
    assert_eq!(crate::mesh_analysis::live_result_count(), results);
    assert_eq!(live_jobs(), 0);
}

#[test]
fn face_ids_are_computed_once_per_solid_across_export_and_render() {
    let s = id(call(json!({"action":"sphere","radius":2,"segments":32})));
    let before = FACE_ID_COMPUTATIONS.with(|c| c.get());
    let exported = crate::mesh::export_buffers(s).unwrap();
    let rendered = crate::mesh::render_buffers(s, 0.6).unwrap();
    let job = start_render(s, 0.6).unwrap();
    let (result, _) = run(job, 1000);
    let cooperative = crate::mesh_analysis::take_render(result.as_u64().unwrap() as usize).unwrap();
    assert_eq!(FACE_ID_COMPUTATIONS.with(|c| c.get()) - before, 1);
    assert_eq!(exported.face_ids, rendered.face_ids);
    assert_eq!(rendered.face_ids, cooperative.face_ids);
    // A different solid computes its own ids (no stale reuse).
    let other = id(call(json!({"action":"cube","size":[1,1,1],"center":false})));
    let other_ids = crate::mesh::export_buffers(other).unwrap().face_ids;
    assert_eq!(FACE_ID_COMPUTATIONS.with(|c| c.get()) - before, 2);
    assert_eq!(other_ids.len(), 12);
    assert_eq!(*other_ids.iter().max().unwrap(), 5);
    delete(&[s, other]);
}

#[test]
fn invalid_starts_refuse_without_admitting_jobs() {
    assert!(start_boolean(&[], BooleanJobOp::Union).is_err());
    assert!(start_boolean(&[u32::MAX], BooleanJobOp::Union).is_err());
    assert!(start_render(u32::MAX, 0.6).is_err());
    assert!(start_render(1, f64::NAN).is_err());
    assert!(step(u32::MAX, 1).is_err());
    assert_eq!(live_jobs(), 0);
}
