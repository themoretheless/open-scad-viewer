use nurbs_core::{
    analysis::{
        planar_face_area::measure_projected_face_area,
        planar_trim_projection::{PlanarTrimOutcome, project_coplanar_trim_loops},
    },
    curve::Curve,
    surface::Surface,
};

fn contour(a: f64, b: f64, reverse: bool) -> Vec<Curve> {
    let mut points = vec![vec![a, a], vec![b, a], vec![b, b], vec![a, b], vec![a, a]];
    if reverse {
        points.reverse();
    }
    vec![Curve {
        degree: 1,
        knots: vec![0., 0., 1., 2., 3., 4., 4.],
        control_points: points,
        weights: vec![1.; 5],
        periodic: false,
    }]
}

fn face() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 1.]],
            vec![vec![1., 0., 1.], vec![1., 1., 2.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}

fn projected(reverse: bool) -> nurbs_core::analysis::planar_trim_projection::ProjectedTrimmedFace {
    let report = project_coplanar_trim_loops(
        &[face()],
        &[vec![
            contour(0., 1., reverse),
            contour(0.25, 0.75, !reverse),
        ]],
        1_000_000,
    )
    .unwrap();
    let PlanarTrimOutcome::Projected(mut charts) = report.outcome else {
        panic!("{:?}", report.outcome)
    };
    charts.remove(0)
}

#[test]
fn encloses_the_tilted_physical_area_and_subtracts_opposite_winding_holes() {
    for reverse in [false, true] {
        let chart = projected(reverse);
        let report = measure_projected_face_area(&chart, 1e-9, 128).unwrap();
        let magnitude = 0.75 * 3_f64.sqrt();
        let [lo, hi] = report.signed_area;
        assert!(report.converged && report.cells <= 128);
        assert!(hi - lo < 1e-9);
        assert!(lo <= magnitude && magnitude <= hi || lo <= -magnitude && -magnitude <= hi);
    }
    let a = measure_projected_face_area(&projected(false), 1e-9, 128).unwrap();
    let b = measure_projected_face_area(&projected(true), 1e-9, 128).unwrap();
    assert!(a.signed_area[0] * b.signed_area[0] < 0.);
}

#[test]
fn reserves_source_spans_for_every_contour_and_reports_precision_limits() {
    let chart = projected(false);
    assert!(measure_projected_face_area(&chart, 1e-9, 7).is_err());
    let report = measure_projected_face_area(&chart, 1e-30, 8).unwrap();
    assert_eq!(report.cells, 8);
    assert!(!report.converged);
    let [lo, hi] = report.signed_area;
    let magnitude = 0.75 * 3_f64.sqrt();
    assert!(lo <= magnitude && magnitude <= hi || lo <= -magnitude && -magnitude <= hi);
}
