use super::*;
use value_codec::json;

fn sphere() -> u32 {
    crate::mesh::dispatch(json!({"action":"sphere", "radius":3, "segments":128}))
        .unwrap()
        .as_u64()
        .unwrap() as u32
}
fn delete(id: u32) {
    crate::mesh::dispatch(json!({"action":"delete", "ids":[id]})).unwrap();
}
fn start(id: u32) -> u32 {
    Z_RIGID_TEMPLATE.with(|slot| *slot.borrow_mut() = None);
    start_solid_analysis(id, 0.6, 0.8, 8).unwrap()
}
fn live() -> usize {
    JOBS.with(|jobs| jobs.borrow().jobs.iter().flatten().count())
}

#[test]
fn cancelled_jobs_drop_scratch_and_never_alias_new_jobs() {
    let id = sphere();
    for _ in 0..8 {
        let cancelled = start(id);
        assert_eq!(step_solid_analysis(cancelled).unwrap(), 0);
        assert_eq!(live(), 1);
        cancel_solid_analysis(cancelled);
        assert_eq!(live(), 0);
        let next = start(id);
        assert!(next > cancelled);
        cancel_solid_analysis(cancelled);
        assert!(step_solid_analysis(cancelled).is_err());
        assert_eq!(step_solid_analysis(next).unwrap(), 0);
        cancel_solid_analysis(next);
    }
    delete(id);
}

#[test]
fn cancellation_at_every_cooperative_step_leaves_no_jobs_or_results() {
    let id = sphere();
    let result_baseline = live_result_count();
    let completed = start(id);
    let mut steps = 0;
    let handle = loop {
        steps += 1;
        assert!(
            steps < 10_000,
            "analysis did not finish within its step budget"
        );
        let result = step_solid_analysis(completed).unwrap();
        if result != 0 {
            break result;
        }
    };
    assert!(steps > 2, "test requires distinct BVH and edge phases");
    free(handle);
    assert_eq!(live_result_count(), result_baseline);

    // Every prefix ends before the completing poll. This covers admission,
    // every BVH slice, the inter-phase yield and every edge-extraction slice.
    for completed_steps in 0..steps {
        let job = start(id);
        for _ in 0..completed_steps {
            assert_eq!(step_solid_analysis(job).unwrap(), 0);
        }
        assert_eq!(live(), 1);
        cancel_solid_analysis(job);
        cancel_solid_analysis(job); // idempotent, including stale IDs
        assert_eq!(live(), 0, "cancel after {completed_steps} steps");
        assert_eq!(live_result_count(), result_baseline);
        assert!(step_solid_analysis(job).is_err());
        let next = start(id);
        assert!(next > job, "job IDs must not be recycled");
        cancel_solid_analysis(job);
        assert_eq!(live(), 1, "stale cancellation removed a newer job");
        cancel_solid_analysis(next);
        assert_eq!(live(), 0);
        assert_eq!(live_result_count(), result_baseline);
    }
    eprintln!("verified cancellation at all {steps} cooperative-step prefixes");
    delete(id);
}

#[test]
fn cancellation_after_bvh_drops_edge_scratch_and_releases_admission() {
    use std::future::Future;
    let id = sphere();
    let mesh = crate::mesh::render_buffers(id, 0.6).unwrap();
    let mut bvh = std::pin::pin!(mesh_query::build_mesh_bvh_cooperative(
        &mesh.vertices,
        &mesh.indices,
        6,
        8
    ));
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    let mut bvh_polls = 1;
    while bvh.as_mut().poll(&mut cx).is_pending() {
        bvh_polls += 1;
    }
    for edge_steps in [1, 20, 80] {
        let job = start(id);
        // The poll completing BVH also reaches the pre-edge yield.
        for _ in 0..bvh_polls + edge_steps {
            assert_eq!(step_solid_analysis(job).unwrap(), 0);
        }
        assert_eq!(live(), 1);
        cancel_solid_analysis(job);
        assert_eq!(live(), 0);
        assert!(step_solid_analysis(job).is_err());
    }
    let a = start(id);
    let b = start(id);
    cancel_solid_analysis(a);
    cancel_solid_analysis(b);
    delete(id);
}

#[test]
fn completion_matches_sync_and_owns_snapshot_after_source_deletion() {
    let id = sphere();
    let expected = analyze_solid(id, 0.6, 0.8, 8).unwrap();
    let job = start(id);
    delete(id);
    let mut polls = 0;
    let handle = loop {
        polls += 1;
        assert!(polls < 10_000);
        let result = step_solid_analysis(job).unwrap();
        if result != 0 {
            break result;
        }
    };
    assert!(polls > 2);
    assert_eq!(live(), 0);
    assert!(step_solid_analysis(job).is_err());
    cancel_solid_analysis(job);
    RESULTS.with(|results| {
        let results = results.borrow();
        let Some(AnalysisBuffers::Solid(actual)) = &results[handle - 1] else {
            panic!("missing result")
        };
        assert_eq!(actual.mesh.vertices, expected.mesh.vertices);
        assert_eq!(actual.mesh.indices, expected.mesh.indices);
        let actual_bits: Vec<_> = actual.bvh.bounds.iter().map(|x| x.to_bits()).collect();
        let expected_bits: Vec<_> = expected.bvh.bounds.iter().map(|x| x.to_bits()).collect();
        assert_eq!(actual_bits, expected_bits);
        assert_eq!(actual.bvh.nodes, expected.bvh.nodes);
        assert_eq!(actual.bvh.triangles, expected.bvh.triangles);
        assert_eq!(actual.edges.indices, expected.edges.indices);
        assert_eq!(actual.edges.diagnostics, expected.edges.diagnostics);
    });
    free(handle);
}

#[test]
fn admission_is_bounded_and_failure_does_not_poison_registry() {
    let id = sphere();
    assert!(start_solid_analysis(id, f64::NAN, 0.8, 8).is_err());
    assert!(start_solid_analysis(id, 0.6, 0.8, 0).is_err());
    let a = start(id);
    let b = start(id);
    assert!(start_solid_analysis(id, 0.6, 0.8, 8).is_err());
    assert_eq!(live(), 2);
    cancel_solid_analysis(a);
    let c = start(id);
    cancel_solid_analysis(b);
    cancel_solid_analysis(c);
    delete(id);
    assert!(start_solid_analysis(id, 0.6, 0.8, 8).is_err());
    assert_eq!(live(), 0);
}
