use brep_core::face_cut;
use nurbs_core::surface::Surface;
#[path = "../examples/support/fiber_cut_fixture.rs"]
mod fixture;
fn pair(scale: f64) -> (brep_core::Model, brep_core::Model) {
    let a = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..2)
                    .map(|j| vec![i as f64 / 2., j as f64, if i == 2 { scale } else { 0. }])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let mut b = a.clone();
    for i in 0..3 {
        for j in 0..2 {
            b.control_points[i][j][1] = -0.5 + 2. * j as f64;
            b.control_points[i][j][2] += i as f64 / 2. - 0.5;
        }
    }
    (fixture::sheet(a), fixture::sheet(b))
}
#[test]
fn nonlinear_original_sheets_are_automatically_cut_into_saved_brep_sectors() {
    for scale in [1., 16.] {
        let (source, cutter) = pair(scale);
        let before = (source.clone(), cutter.clone());
        let result = face_cut::partition_fiber_at_face_automatic(
            &source, 0, &cutter, 0, 1e-8, 100000, 1000000,
        )
        .unwrap();
        assert_eq!(result.partition.model.faces.len(), 2);
        assert!(result.boundary_coverage.iter().all(|(_, r)| r.complete));
        assert!(result.root_error_upper_mm <= source.tolerance_mm / 2.);
        result.partition.model.validate().unwrap();
        assert!(result.partition.model.persistent_naming_complete());
        for vertex in result.endpoints {
            let p = &result.partition.model.vertices[vertex].point;
            assert!((p[0] - 0.5).abs() <= source.tolerance_mm);
            assert!((p[2] - scale / 4.).abs() <= source.tolerance_mm);
        }
        assert_eq!((source, cutter), before);
    }
}
#[test]
fn exhausted_automatic_trim_never_publishes_partial_edits() {
    let (source, cutter) = pair(16.);
    let before = (source.clone(), cutter.clone());
    assert!(
        face_cut::partition_fiber_at_face_automatic(&source, 0, &cutter, 0, 1e-8, 32, 1000000)
            .is_err()
    );
    assert_eq!((source, cutter), before);
}
