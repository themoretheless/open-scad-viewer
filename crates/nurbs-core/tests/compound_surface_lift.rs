use nurbs_core::{
    curve::Curve,
    curve_surface_agreement::{self as lift, Status},
    surface::Surface,
};
#[test]
fn literal_compound_clock_and_exact_reverse_are_covered_without_crossing_knot_halos() {
    let surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let p = Curve {
        degree: 3,
        knots: vec![2., 2., 2., 2., 3., 3., 3., 6., 6., 6., 6.],
        control_points: (0..7).map(|i| vec![i as f64 / 6., 0.]).collect(),
        weights: vec![1.; 7],
        periodic: false,
    };
    let c = Curve {
        control_points: p
            .control_points
            .iter()
            .map(|v| vec![v[0], v[1], 0.])
            .collect(),
        ..p.clone()
    };
    for (pc, reversed) in [(p.clone(), false), (p.reverse().unwrap(), true)] {
        let good = lift::verify(&c, &pc, &surface, reversed, 1e-7, 2).unwrap();
        assert_eq!(good.status, Status::WithinTolerance);
        assert_eq!(good.cells, 2);
        let short = lift::verify(&c, &pc, &surface, reversed, 1e-7, 1).unwrap();
        assert_eq!(short.status, Status::Unresolved);
        assert_eq!(short.cells, 1);
        let mut damaged = c.clone();
        damaged.control_points[1][2] = 0.01;
        assert_eq!(
            lift::verify(&damaged, &pc, &surface, reversed, 1e-7, 1000)
                .unwrap()
                .status,
            Status::Mismatch
        );
    }
    let mut wrong_clock = p.clone();
    for t in &mut wrong_clock.knots[4..7] {
        *t = 3.25;
    }
    assert_ne!(
        lift::verify(&c, &wrong_clock, &surface, false, 1e-7, 1000)
            .unwrap()
            .status,
        Status::WithinTolerance
    );
}
