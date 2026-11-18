use nurbs_core::{
    analysis::{
        planar_face_perimeter::bound_projected_face_perimeter,
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

fn contains(bounds: [f64; 2], value: f64) {
    assert!(
        bounds[0] <= value && value <= bounds[1],
        "{bounds:?} excludes {value}"
    );
}

#[test]
fn adds_hole_perimeter_and_applies_the_tilted_metric() {
    for reversed in [false, true] {
        let chart = projected(reversed);
        let report = bound_projected_face_perimeter(&chart, 1e-9, 128).unwrap();
        assert!(report.within_tolerance);
        contains(report.perimeter, 6. * 2_f64.sqrt());
        contains(report.contours[0], 4. * 2_f64.sqrt());
        contains(report.contours[1], 2. * 2_f64.sqrt());
    }
}

#[test]
fn encloses_the_boundary_without_storing_nonbinary_world_poles() {
    use nurbs_core::analysis::{
        exact_affine_hull::{PlanarProjectionOutcome, project_planar_surface},
        planar_trim_projection::{
            PlanarContourLiftOutcome, ProjectedTrimmedFace, lift_planar_trim_loops,
        },
    };
    let mut patch = face();
    patch.control_points = vec![
        vec![vec![0., 0., 0.], vec![0., 3., 1.]],
        vec![vec![3., 0., 1.], vec![3., 3., 2.]],
    ];
    let report = project_planar_surface(&patch, 1_000_000).unwrap();
    let PlanarProjectionOutcome::Projected(projection) = report.outcome else {
        panic!()
    };
    let chart = ProjectedTrimmedFace {
        projection,
        loops: vec![contour(0., 1., false)],
    };
    let lifted = lift_planar_trim_loops(&chart.projection, &chart.loops, 1_000_000).unwrap();
    assert!(matches!(
        lifted.outcome,
        PlanarContourLiftOutcome::Refused { .. }
    ));
    let result = bound_projected_face_perimeter(&chart, 1e-9, 128).unwrap();
    assert!(result.within_tolerance);
    contains(result.perimeter, 4. * 10_f64.sqrt() / 3.);
}

#[test]
fn retains_a_bound_at_the_original_span_budget() {
    let chart = projected(false);
    assert!(bound_projected_face_perimeter(&chart, 1e-20, 7).is_err());
    let report = bound_projected_face_perimeter(&chart, 1e-20, 8).unwrap();
    assert_eq!(report.cells, 8);
    assert!(!report.within_tolerance);
    contains(report.perimeter, 6. * 2_f64.sqrt());
}
