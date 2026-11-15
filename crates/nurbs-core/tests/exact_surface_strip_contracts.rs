use nurbs_core::{
    editing::exact_surface_merge::{Refusal, StripRefusal, merge_surface_strip},
    surface::{Axis, Surface},
};

const WORK: u64 = 1_000_000;

fn patch(a: f64, b: f64, z: impl Fn(f64) -> f64) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![a, a, b, b],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [a, b]
            .map(|u| vec![vec![u, 0., z(u)], vec![u, 1., z(u) + 2.]])
            .to_vec(),
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}

#[test]
fn long_strip_stays_compact_before_reaching_the_tensor_control_limit() {
    let inputs: Vec<_> = (0..260)
        .map(|i| patch(i as f64, i as f64 + 1., |u| u))
        .collect();
    let report = merge_surface_strip(&inputs, Axis::U, WORK).unwrap();
    assert!(report.refusal.is_none(), "{report:?}");
    assert_eq!(report.removed_knots, 259);
    let merged = report.surface.unwrap();
    assert_eq!(merged.knots_u, [0., 0., 260., 260.]);
    assert_eq!(merged.control_points.len(), 2);
    assert_eq!(merged.control_points[1][1], [260., 1., 262.]);
}

#[test]
fn smooth_seams_are_removed_but_a_genuine_crease_stays() {
    let inputs = [
        patch(0., 1., |_| 0.),
        patch(1., 2., |u| u - 1.),
        patch(2., 3., |u| u - 1.),
    ];
    let report = merge_surface_strip(&inputs, Axis::U, WORK).unwrap();
    assert_eq!(report.removed_knots, 1);
    let merged = report.surface.unwrap();
    assert_eq!(merged.knots_u, [0., 0., 1., 3., 3.]);
    for u in [0., 0.5_f64, 1., 1.5, 2., 3.] {
        let exact_z = (u - 1.).max(0.) + 0.5;
        assert!((merged.evaluate(u, 0.25).unwrap().point[2] - exact_z).abs() < 1e-14);
    }
}

#[test]
fn late_join_or_budget_failure_never_returns_a_prefix() {
    let inputs = [
        patch(0., 1., |u| u),
        patch(1., 2., |u| u),
        patch(3., 4., |u| u),
    ];
    let report = merge_surface_strip(&inputs, Axis::U, WORK).unwrap();
    assert!(report.surface.is_none());
    assert_eq!(
        report.refusal,
        Some(StripRefusal::Join {
            patch: 2,
            reason: Refusal::NonAdjacentDomain
        })
    );
    let complete = merge_surface_strip(&inputs[..2], Axis::U, WORK).unwrap();
    let short = merge_surface_strip(&inputs[..2], Axis::U, complete.work_used - 1).unwrap();
    assert!(short.surface.is_none());
    assert_eq!(short.refusal, Some(StripRefusal::ResourceLimit));
}

#[test]
fn linear_seam_proof_preserves_nonconstant_weights_in_both_axes() {
    let make = |a, b| {
        let mut s = patch(a, b, |_| 0.);
        for (row, u) in [a, b].into_iter().enumerate() {
            let weight = 1. + 2. * u;
            s.weights[row].fill(weight);
            for point in &mut s.control_points[row] {
                point[0] = 3. * u / weight;
            }
        }
        s
    };
    let mut patches = [make(0., 0.5), make(0.5, 1.)];
    for axis in [Axis::U, Axis::V] {
        if matches!(axis, Axis::V) {
            for s in &mut patches {
                std::mem::swap(&mut s.knots_u, &mut s.knots_v);
                let point = s.control_points[0][1].clone();
                s.control_points[0][1] = s.control_points[1][0].clone();
                s.control_points[1][0] = point;
                let weight = s.weights[0][1];
                s.weights[0][1] = s.weights[1][0];
                s.weights[1][0] = weight;
            }
        }
        let report = merge_surface_strip(&patches, axis, WORK).unwrap();
        assert_eq!(report.removed_knots, 1, "{report:?}");
        let s = report.surface.unwrap();
        let (u, v) = if matches!(axis, Axis::U) {
            (0.25, 0.5)
        } else {
            (0.5, 0.25)
        };
        let point = s.evaluate(u, v).unwrap().point;
        assert!((point[0] - 0.5).abs() < 1e-14);
        assert_eq!(s.weights[0][0], 1.);
        assert_eq!(s.weights[1][1], 3.);
    }
}
