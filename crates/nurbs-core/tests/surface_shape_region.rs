use nurbs_core::{surface::Surface, surface_differential::Status, surface_shape_operator};
fn paraboloid() -> Surface {
    let squares = [0., 0., 1.];
    Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| vec![i as f64 / 2., j as f64 / 2., squares[i] + squares[j]])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn original_interior_point_box_encloses_analytic_world_normal() {
    let s = paraboloid();
    let before = s.clone();
    let r = surface_shape_operator::rectangle_bounds(&s, [[0.25, 0.25], [0.5, 0.5]], 100).unwrap();
    assert_eq!(r.shape.status, Status::Available);
    assert_eq!(r.cells, 1);
    for (range, expected) in r
        .shape
        .normal
        .unwrap()
        .iter()
        .zip([-1. / 3., -2. / 3., 2. / 3.])
    {
        assert!(range[0] <= expected && expected <= range[1]);
        assert!(range[1] - range[0] < 1e-10);
    }
    assert!(r.shape.tensor.is_some());
    assert_eq!(s, before);
}
#[test]
fn non_c2_knots_zero_work_and_singular_regions_remain_unqualified() {
    let mut s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 0.5, 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![0.5, 0., 0.], vec![0.5, 1., 0.]],
            vec![vec![1., 0., 1.], vec![1., 1., 1.]],
        ],
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let knot =
        surface_shape_operator::rectangle_bounds(&s, [[0.25, 0.75], [0.25, 0.75]], 100).unwrap();
    assert_eq!(knot.shape.status, Status::ContinuityNotProven);
    assert!(knot.shape.tensor.is_none());
    let empty =
        surface_shape_operator::rectangle_bounds(&paraboloid(), [[0.25, 0.25], [0.5, 0.5]], 0)
            .unwrap();
    assert_eq!(empty.cells, 0);
    assert!(empty.shape.normal.is_none());
    s = paraboloid();
    for row in &mut s.control_points {
        for p in row {
            *p = vec![0.; 3];
        }
    }
    let singular =
        surface_shape_operator::rectangle_bounds(&s, [[0.2, 0.4], [0.2, 0.4]], 100).unwrap();
    assert_eq!(singular.shape.status, Status::RegularityNotProven);
    assert!(singular.shape.normal.is_none());
}
