use brep_core::face_cut;
use nurbs_core::{
    curve_surface_agreement::Status,
    surface::{Axis, Surface},
};
#[test]
fn rational_curved_section_is_generated_and_a_probe_only_match_refuses() {
    let source = Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: [0., 1.]
            .iter()
            .map(|&x| vec![vec![x, 0., 0.], vec![x, 0.5, 0.], vec![x, 1., 1.]])
            .collect(),
        weights: vec![vec![1., 0.5, 1.]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let plane = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0.1, -1., -1.], vec![0.1, -1., 2.]],
            vec![vec![0.1, 2., -1.], vec![0.1, 2., 2.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let before = (source.clone(), plane.clone());
    let out = face_cut::generate_iso_intersection_curve(
        [&source, &plane],
        Axis::U,
        0,
        None,
        1e-7,
        100000,
    )
    .unwrap();
    assert!(out.probe_roots.complete);
    assert!((out.fixed_parameter - 0.1).abs() < 1e-10);
    assert!(out.cut.curve.degree >= 2);
    assert!(
        out.cut
            .curve
            .weights
            .iter()
            .any(|w| *w != out.cut.curve.weights[0])
    );
    assert!(
        out.cut
            .agreements
            .iter()
            .all(|r| r.status == Status::WithinTolerance)
    );
    assert!(out.cut.cells <= 100000);
    let transposed = Surface {
        degree_u: source.degree_v,
        degree_v: source.degree_u,
        knots_u: source.knots_v.clone(),
        knots_v: source.knots_u.clone(),
        control_points: (0..3)
            .map(|j| {
                (0..2)
                    .map(|i| source.control_points[i][j].clone())
                    .collect()
            })
            .collect(),
        weights: (0..3)
            .map(|j| (0..2).map(|i| source.weights[i][j]).collect())
            .collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let along_v = face_cut::generate_iso_intersection_curve(
        [&transposed, &plane],
        Axis::V,
        0,
        None,
        1e-7,
        100000,
    )
    .unwrap();
    assert!((along_v.fixed_parameter - 0.1).abs() < 1e-10);
    assert!(
        along_v
            .cut
            .agreements
            .iter()
            .all(|r| r.status == Status::WithinTolerance)
    );
    // At v=0 this chart has the same probe crossing, but the complete iso
    // section leaves the plane. The probe cannot authorize the spatial cut.
    let mut bent = source.clone();
    for row in &mut bent.control_points {
        row[2][0] += 0.2;
    }
    assert!(
        face_cut::generate_iso_intersection_curve([&bent, &plane], Axis::U, 0, None, 1e-7, 100000)
            .is_err()
    );
    assert!(
        face_cut::generate_iso_intersection_curve(
            [&source, &plane],
            Axis::U,
            0,
            Some([[0.1, 0., 0.01], [0.1, 1., 1.]]),
            1e-7,
            100000
        )
        .is_err()
    );
    assert!(
        face_cut::generate_iso_intersection_curve(
            [&source, &plane],
            Axis::U,
            1,
            None,
            1e-7,
            100000
        )
        .is_err()
    );
    let bounded =
        face_cut::generate_iso_intersection_curve([&source, &plane], Axis::U, 0, None, 1e-7, 8)
            .unwrap();
    assert!(bounded.cut.cells <= 8);
    assert!(
        face_cut::generate_iso_intersection_curve([&source, &plane], Axis::U, 0, None, 1e-7, 7)
            .is_err()
    );
    assert_eq!((source, plane), before);
}
