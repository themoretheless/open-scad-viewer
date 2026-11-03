use nurbs_core::{ss_intersection as ss, surface::Surface};
use std::sync::Arc;
#[path = "common/periodic_extrusion.rs"]
mod fixture;
fn wire() -> ss::OriginalSsRootWire {
    let plane = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![-2., -2., 1.], vec![-2., 2., 1.]],
            vec![vec![2., -2., 1.], vec![2., 2., 1.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let source = ss::FiberSources::new(&fixture::source(1), &plane).unwrap();
    let curves: Vec<_> = (0..4)
        .map(|i| {
            let d = [
                [i as f64 - 0.05, i as f64 + 1.05],
                [0.4, 0.6],
                [0.1, 0.9],
                [0.1, 0.9],
            ];
            let result = source
                .certify_quotient(d, ss::FiberAxis::FirstU, Default::default())
                .unwrap();
            let report = format!("{:?}", result.report());
            Arc::new(ss::OriginalSsRootCurve::from_quotient(
                result.into_fiber().unwrap_or_else(|| panic!("{report}")),
            ))
        })
        .collect();
    let points: Vec<_> = (0..4)
        .map(|i| Arc::new(ss::OriginalSsRootPoint::new(curves[i].clone(), i as f64).unwrap()))
        .collect();
    let edges = (0..4)
        .map(|i| {
            let a = ss::bind_original_ss_port(
                points[i].clone(),
                curves[i].clone(),
                [0; 4],
                Default::default(),
            )
            .unwrap();
            let phase = if i == 3 { [1, 0, 0, 0] } else { [0; 4] };
            let b = ss::bind_original_ss_port(
                points[(i + 1) % 4].clone(),
                curves[i].clone(),
                phase,
                Default::default(),
            )
            .unwrap();
            ss::OriginalSsRootEdge::new(a, b).unwrap()
        })
        .collect();
    ss::OriginalSsRootWire::new(edges).unwrap()
}
#[test]
fn noncontractible_periodic_boundary_has_relative_material_transitions() {
    let wire = wire();
    wire.reverify().unwrap();
    assert!(ss::qualify_original_ss_winding(&wire, 0, [0.25, 0.25], Default::default()).is_err());
    for (a, b, odd) in [
        ([0.25, 0.25], [0.25, 0.75], true),
        ([3.99, 0.25], [4.01, 0.75], true),
        ([0.1, 0.25], [3.9, 0.25], false),
        ([8.25, 0.25], [8.25, 0.75], true),
    ] {
        let proof =
            ss::qualify_original_ss_path_parity(&wire, 0, a, b, Default::default()).unwrap();
        assert_eq!(proof.odd_crossings(), odd, "{a:?} -> {b:?}");
        proof.reverify().unwrap();
        let reversed =
            ss::qualify_original_ss_path_parity(&wire, 0, b, a, Default::default()).unwrap();
        assert_eq!(reversed.odd_crossings(), odd);
    }
    assert!(
        ss::qualify_original_ss_path_parity(
            &wire,
            0,
            [0.25, 0.5],
            [0.25, 0.75],
            ss::OriginalSsPathLimits {
                max_segments: 4,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        ss::qualify_original_ss_path_parity(
            &wire,
            0,
            [0.25, 0.25],
            [8.25, 0.75],
            ss::OriginalSsPathLimits {
                max_periodic_copies: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
}
