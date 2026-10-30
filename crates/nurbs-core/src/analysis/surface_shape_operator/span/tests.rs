use super::*;
fn surface(height: f64) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 0.5, 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![0.5, 0., 0.], vec![0.5, 1., 0.]],
            vec![vec![1., 0., height], vec![1., 1., height]],
        ],
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn original_span_bounds_cover_one_sided_c0_endpoints_without_claiming_a_join() {
    let s = surface(0.);
    let before = s.clone();
    assert_eq!(
        rectangle_bounds(&s, [[0.5, 0.5], [0., 1.]], 10)
            .unwrap()
            .shape
            .status,
        Status::ContinuityNotProven
    );
    for span in [1, 2] {
        let report = span_rectangle_bounds(&s, [span, 1], [[0.5, 0.5], [0.25, 0.75]]).unwrap();
        assert_eq!(report.cells, 1);
        assert_eq!(report.shape.status, Status::Available);
        let n = report.shape.normal.unwrap();
        assert!(n[2][0] > 0.999 && n[2][0] <= 1. && n[2][1] >= 1.);
        assert!(
            report
                .shape
                .tensor
                .unwrap()
                .iter()
                .flatten()
                .flatten()
                .all(|x| x.abs() < 1e-10)
        );
    }
    assert_eq!(s, before);
}
#[test]
fn one_sided_bounds_expose_a_real_kink_and_refuse_cross_span_queries() {
    let s = surface(0.25);
    let a = span_rectangle_bounds(&s, [1, 1], [[0.5, 0.5], [0.25, 0.75]])
        .unwrap()
        .shape
        .normal
        .unwrap();
    let b = span_rectangle_bounds(&s, [2, 1], [[0.5, 0.5], [0.25, 0.75]])
        .unwrap()
        .shape
        .normal
        .unwrap();
    assert!(a[0][0] - b[0][1] > 0.4);
    assert!(span_rectangle_bounds(&s, [1, 1], [[0.49, 0.51], [0., 1.]]).is_err());
    assert!(span_rectangle_bounds(&s, [0, 1], [[0., 0.], [0., 1.]]).is_err());
    assert!(span_rectangle_bounds(&s, [usize::MAX, 1], [[0., 0.], [0., 1.]]).is_err());
    assert!(span_rectangle_bounds(&s, [1, 1], [[f64::NAN, 0.5], [0., 1.]]).is_err());
}
#[test]
fn singular_span_never_acquires_shape_from_a_proposal_frame() {
    let mut s = surface(0.);
    for row in &mut s.control_points {
        row[1] = row[0].clone();
    }
    let r = span_rectangle_bounds(&s, [1, 1], [[0., 0.5], [0., 1.]]).unwrap();
    assert_eq!(r.shape.status, Status::RegularityNotProven);
    assert!(r.shape.normal.is_none() && r.shape.tensor.is_none());
}

#[test]
fn regular_midpoint_frame_cannot_certify_a_rectangle_crossing_a_fold() {
    let mut s = surface(0.);
    s.control_points[0][1][1] = -1.;
    let point = span_rectangle_bounds(&s, [1, 1], [[0.3125, 0.3125], [0.5, 0.5]]).unwrap();
    assert_eq!(point.shape.status, Status::Available);
    let region = span_rectangle_bounds(&s, [1, 1], [[0.125, 0.5], [0.25, 0.75]]).unwrap();
    assert_eq!(region.shape.status, Status::RegularityNotProven);
    assert!(region.shape.normal.is_none() && region.shape.tensor.is_none());
}
