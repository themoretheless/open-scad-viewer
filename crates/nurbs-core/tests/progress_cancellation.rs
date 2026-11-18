//! Roadmap N19: cooperative cancellation, monotonic progress and bounded
//! allocation in the tessellation and approximation loops.
use nurbs_core::{
    curve::Curve,
    curve_tessellation,
    foundation::{fit_curve_cloud_certified_report, fit_curve_cloud_certified_report_with},
    numerics::progress::{
        AllocationBudget, CANCELLED, CancellationToken, Control, RecordedProgress, is_cancelled,
    },
    shared_tessellation::{self, Seam, Side},
    surface::Surface,
    surface_tessellation,
    tessellation::{shading::Orientation, watertight},
};

fn curve() -> Curve {
    Curve {
        degree: 2,
        knots: vec![2., 2., 2., 7., 7., 7.],
        control_points: vec![vec![0., 0.], vec![2., 3.], vec![4., 0.]],
        weights: vec![1., 2., 1.],
        periodic: false,
    }
}

fn panel() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 7., 7.],
        knots_v: vec![-3., -3., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 4., 0.]],
            vec![vec![4., 0., 0.], vec![4., 4., 2.]],
        ],
        weights: vec![vec![1., 2.], vec![3., 6.]],
        periodic_u: false,
        periodic_v: false,
    }
}

fn patch(x: f64) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![x, 0., 0.], vec![x, 1., 0.]],
            vec![vec![x + 1., 0., 0.], vec![x + 1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}

const SEAM: Seam = Seam {
    patches: [0, 1],
    sides: [Side::U1, Side::U0],
    reversed: false,
};

fn cloud() -> Vec<Vec<f64>> {
    (0..200)
        .map(|i| {
            let t = i as f64 / 199.;
            vec![t, (3. * t).sin(), t * t]
        })
        .collect()
}

/// Within a phase `done` never decreases and never exceeds `total`; the last
/// sample of a completed call is `done == total`.
fn assert_progress(rec: &RecordedProgress) {
    assert!(!rec.0.is_empty());
    let mut previous: Option<(&str, u64)> = None;
    for (phase, done, total, _) in &rec.0 {
        assert!(done <= total, "{phase}: {done} > {total}");
        if let Some((p, d)) = previous
            && p == *phase
            && *done != 0
        {
            assert!(*done >= d, "{phase} went back from {d} to {done}");
        }
        previous = Some((phase, *done));
    }
    let last = rec.0.last().unwrap();
    assert_eq!(last.1, last.2, "final sample must reach total: {last:?}");
}

/// Runs `f` with a token that cancels on poll `k + 1` (interval 1) and checks
/// the typed error and that the loop stopped at that poll.
fn assert_cancels<T>(k: usize, f: impl FnOnce(&mut Control<'_>) -> nurbs_core::Result<T>) {
    let mut rec = RecordedProgress::default();
    let result = {
        let mut ctl = Control::new(CancellationToken::cancel_after(k), &mut rec);
        f(&mut ctl)
    };
    let Err(error) = result else {
        panic!("expected cancellation after {k} polls")
    };
    assert!(is_cancelled(&error) && error.code == CANCELLED, "{error:?}");
    // Every sample follows a successful poll, so at most k samples exist.
    assert!(rec.0.len() <= k, "loop kept reporting after cancellation: {}", rec.0.len());
}

#[test]
fn curve_tessellation_cancels_promptly_and_matches_plain_call() {
    let c = curve();
    let plain = curve_tessellation::tessellate(&c, 1e-4, 10000).unwrap();
    let mut rec = RecordedProgress::default();
    let with = curve_tessellation::tessellate_with(
        &c,
        1e-4,
        10000,
        &mut Control::new(CancellationToken::new(), &mut rec),
    )
    .unwrap();
    assert_eq!(format!("{plain:?}"), format!("{with:?}"));
    assert_progress(&rec);
    for k in [0, 1, 5, 20] {
        assert_cancels(k, |ctl| curve_tessellation::tessellate_with(&c, 1e-4, 10000, ctl));
    }
    // A token cancelled before the call refuses at once.
    let t = CancellationToken::new();
    t.cancel();
    let mut none = RecordedProgress::default();
    let e = curve_tessellation::tessellate_with(&c, 1e-4, 10000, &mut Control::new(t, &mut none))
        .unwrap_err();
    assert!(is_cancelled(&e) && none.0.is_empty());
}

#[test]
fn surface_tessellation_cancels_promptly_and_matches_plain_call() {
    let s = panel();
    let before = s.clone();
    let plain = surface_tessellation::tessellate(&s, 1e-3, 2000).unwrap();
    let mut rec = RecordedProgress::default();
    let with = surface_tessellation::tessellate_with(
        &s,
        1e-3,
        2000,
        &mut Control::new(CancellationToken::new(), &mut rec).with_interval(7),
    )
    .unwrap();
    assert_eq!(format!("{plain:?}"), format!("{with:?}"));
    assert_progress(&rec);
    for k in [0, 3, 50] {
        assert_cancels(k, |ctl| surface_tessellation::tessellate_with(&s, 1e-3, 2000, ctl));
    }
    assert_eq!(format!("{s:?}"), format!("{before:?}"), "input must stay untouched");
}

#[test]
fn shared_and_watertight_tessellation_cancel_promptly_and_match_plain_calls() {
    let input = vec![patch(0.), patch(1.)];
    let plain = shared_tessellation::tessellate(&input, &[SEAM], 8, 0.3, 10000, 10000).unwrap();
    let mut rec = RecordedProgress::default();
    let with = shared_tessellation::tessellate_with(
        &input,
        &[SEAM],
        8,
        0.3,
        10000,
        10000,
        &mut Control::new(CancellationToken::new(), &mut rec),
    )
    .unwrap();
    assert_eq!(format!("{plain:?}"), format!("{with:?}"));
    assert_progress(&rec);
    for k in [0, 1, 2, 40, 150] {
        assert_cancels(k, |ctl| {
            shared_tessellation::tessellate_with(&input, &[SEAM], 8, 0.3, 10000, 10000, ctl)
        });
    }

    let o = [Orientation::Natural; 2];
    let plain =
        watertight::tessellate_lods(&input, &[SEAM], &o, &[0.5, 0.1], 100000, 1000000).unwrap();
    let mut rec = RecordedProgress::default();
    let with = watertight::tessellate_lods_with(
        &input,
        &[SEAM],
        &o,
        &[0.5, 0.1],
        100000,
        1000000,
        &mut Control::new(CancellationToken::never(), &mut rec),
    )
    .unwrap();
    assert_eq!(format!("{plain:?}"), format!("{with:?}"));
    assert_progress(&rec);
    for k in [0, 10, 30] {
        assert_cancels(k, |ctl| {
            watertight::tessellate_lods_with(&input, &[SEAM], &o, &[0.5, 0.1], 100000, 1000000, ctl)
        });
    }
}

#[test]
fn cloud_fit_cancels_promptly_and_matches_plain_call() {
    let plain = fit_curve_cloud_certified_report(cloud(), 8, None).unwrap();
    let mut rec = RecordedProgress::default();
    let with = fit_curve_cloud_certified_report_with(
        cloud(),
        8,
        None,
        &mut Control::new(CancellationToken::new(), &mut rec),
    )
    .unwrap();
    assert_eq!(format!("{:?}", plain.curve), format!("{:?}", with.curve));
    assert_eq!(
        plain.certificate.evidence.hausdorff_error_upper.to_bits(),
        with.certificate.evidence.hausdorff_error_upper.to_bits()
    );
    assert_progress(&rec);
    for k in [0, 1, 100] {
        assert_cancels(k, |ctl| fit_curve_cloud_certified_report_with(cloud(), 8, None, ctl));
    }
}

#[test]
fn loops_refuse_oversized_buffers_with_typed_resource_errors() {
    let resource = |e: nurbs_core::Error| assert_eq!(e.code, "NURBS_RESOURCE_LIMIT", "{e:?}");
    let tiny = AllocationBudget { max_bytes: 4096 };
    let mut ctl = Control::none().with_memory(tiny);
    resource(curve_tessellation::tessellate_with(&curve(), 1e-4, 100000, &mut ctl).unwrap_err());
    let mut ctl = Control::none().with_memory(tiny);
    resource(surface_tessellation::tessellate_with(&panel(), 1e-3, 100000, &mut ctl).unwrap_err());
    let input = vec![patch(0.), patch(1.)];
    let mut ctl = Control::none().with_memory(tiny);
    resource(
        shared_tessellation::tessellate_with(&input, &[SEAM], 64, 0.3, 100000, 10000, &mut ctl)
            .unwrap_err(),
    );
    let mut ctl = Control::none().with_memory(AllocationBudget { max_bytes: 1024 });
    resource(fit_curve_cloud_certified_report_with(cloud(), 8, None, &mut ctl).err().unwrap());
    // The default budget admits the same workloads.
    assert!(
        curve_tessellation::tessellate_with(&curve(), 1e-4, 100000, &mut Control::none()).is_ok()
    );
}
