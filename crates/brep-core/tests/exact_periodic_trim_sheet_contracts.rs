use brep_core::{
    face_domain::FaceDomain, feature_ops::periodic_trim_sheet::build_exact_periodic_trim_sheet,
};
use nurbs_core::{
    curve::Curve,
    surface::{Surface, extrude},
    trim_domain::Location,
};

fn support() -> Surface {
    extrude(
        &Curve {
            degree: 1,
            knots: vec![-1., 0., 1., 2., 3., 4., 5.],
            control_points: vec![
                vec![0., 0., 0.],
                vec![1., 0., 0.],
                vec![1., 1., 0.],
                vec![0., 1., 0.],
                vec![0., 0., 0.],
            ],
            weights: vec![1.; 5],
            periodic: true,
        },
        [0., 0., 2.],
    )
    .unwrap()
}

#[test]
fn periodic_trim_sheet_retains_closed_wires_hole_and_open_shell() {
    let source = support();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let hole = [
        [0.125, 0.25],
        [0.375, 0.25],
        [0.375, 0.75],
        [0.125, 0.75],
        [0.125, 0.25],
    ];
    let report =
        build_exact_periodic_trim_sheet(&source, &[&outer, &hole], 1e-8, 1_000_000).unwrap();
    assert!(report.refusal.is_none());
    let model = report.model.unwrap();
    model.validate().unwrap();
    assert_eq!(model.faces.len(), 1);
    assert_eq!(model.faces[0].holes, vec![1]);
    assert_eq!(model.shells.len(), 1);
    assert!(!model.shells[0].closed);
    assert!(model.bodies.is_empty());
    let domain = FaceDomain::new(&model, 0, 1e-8).unwrap();
    for (point, location) in [
        ([4., 0.5], Location::Inside),
        ([4.25, 0.5], Location::Outside),
    ] {
        assert_eq!(
            domain
                .classify(point.map(|t| [t, t]), 1024)
                .unwrap()
                .location,
            location
        );
    }
}

#[test]
fn periodic_trim_sheet_withholds_invalid_contour_and_exhausted_work() {
    let source = support();
    let bow = [[0., 0.], [1., 1.], [0., 1.], [1., 0.], [0., 0.]];
    let invalid = build_exact_periodic_trim_sheet(&source, &[&bow], 1e-8, 1_000_000).unwrap();
    assert!(invalid.model.is_none());
    assert!(invalid.refusal.is_some());
    let square = [
        [3.5, 0.25],
        [0.5, 0.25],
        [0.5, 0.75],
        [3.5, 0.75],
        [3.5, 0.25],
    ];
    let result = build_exact_periodic_trim_sheet(&source, &[&square], 1e-8, 1_000_000).unwrap();
    assert!(result.model.is_some());
    assert!(
        build_exact_periodic_trim_sheet(&source, &[&square], 1e-8, result.work_used - 1)
            .unwrap()
            .model
            .is_none()
    );
}
