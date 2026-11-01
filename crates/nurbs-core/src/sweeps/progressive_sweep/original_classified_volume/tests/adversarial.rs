//! Whole-source counterexamples for the spatial interval-frame proof.
use super::*;
fn saddle(figure: bool) -> Surface {
    let r = 1. / 1024.;
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [-1., 1.]
            .into_iter()
            .map(|a| {
                [-1., 1.]
                    .into_iter()
                    .map(|b| {
                        if figure {
                            vec![
                                2. * r * a + r * a * b / 32.,
                                -r * a + 2. * r * a * b / 32.,
                                r * b,
                            ]
                        } else {
                            vec![r * a, r * b, r * a * b / 32.]
                        }
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn options(normal: [f64; 3]) -> Options {
    Options {
        normal,
        orientation: Orientation::FixedNormal,
        spacing: Spacing::ArcLength {
            tolerance: 0.001,
            max_cells: 100000,
        },
        initial_sections: 33,
        max_sections: 129,
        max_deviation: 0.05,
    }
}
#[test]
fn spatial_carrier_cannot_hide_a_center_law_that_cancels_all_traversal() {
    let cap = saddle(false);
    let before = cap.clone();
    let profiles = [crate::primitives::line([0.; 3], [1. / 1024., 0., 0.]).unwrap()];
    let path = crate::primitives::line([0.; 3], [0., 0., 2.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let zero = constant_vector_law([0.; 3]).unwrap();
    let axes = constant_vector_law([1.; 3]).unwrap();
    let mut cancel = Curve::from_polyline(vec![vec![0.; 3], vec![0., 0., -2.]]).unwrap();
    cancel.knots = vec![7., 7., 11., 11.];
    let make = |center| {
        MultiSweep::new(&profiles, &path, &scale, &zero, options([1., 0., 0.]))
            .unwrap()
            .with_affine_laws(&axes, center)
            .unwrap()
    };
    let good = make(&zero)
        .certify_ideal_volume_atlas([&cap, &cap], projection(), 100000, 1000000)
        .unwrap();
    assert!(
        good.embedding_certified && good.regularity_certified && good.parameter_topology_preserved,
        "{good:?}"
    );
    // P(s)=2s*e_z and c(s)=-2s*e_z; the actual source F is independent
    // of s on this complete nonplanar carrier, although all frames are smooth.
    let bad = make(&cancel)
        .certify_ideal_volume_atlas([&cap, &cap], projection(), 100000, 1000000)
        .unwrap();
    assert!(
        !bad.embedding_certified && !bad.regularity_certified && !bad.parameter_topology_preserved,
        "{bad:?}"
    );
    assert!(bad.cells <= 100000 && bad.exact_work <= 1000000);
    assert_eq!(cap, before);
}
fn figure_eight() -> Curve {
    let p = [
        [0., 0., 0.],
        [10., 0., 0.],
        [0., 0., 0.],
        [-10., 0., 0.],
        [0., 0., 0.],
    ];
    let v = [
        [5., 10., 0.],
        [0., -10., 0.],
        [-5., 10., 0.],
        [0., -10., 0.],
        [5., 10., 0.],
    ];
    let mut cp = Vec::new();
    for i in 0..4 {
        // Quintic Hermite data with shared first derivatives and zero second
        // derivatives. Every stored coordinate is an exact dyadic integer.
        let control = [
            p[i],
            std::array::from_fn(|k| p[i][k] + v[i][k] / 5.),
            std::array::from_fn(|k| p[i][k] + 2. * v[i][k] / 5.),
            std::array::from_fn(|k| p[i + 1][k] - 2. * v[i + 1][k] / 5.),
            std::array::from_fn(|k| p[i + 1][k] - v[i + 1][k] / 5.),
            p[i + 1],
        ];
        cp.extend(
            control
                .into_iter()
                .skip(usize::from(i != 0))
                .map(|q| q.to_vec()),
        );
    }
    Curve {
        degree: 5,
        knots: [
            vec![0.; 6],
            vec![1.; 5],
            vec![2.; 5],
            vec![3.; 5],
            vec![4.; 6],
        ]
        .concat(),
        weights: vec![1.; cp.len()],
        control_points: cp,
        periodic: false,
    }
}
#[test]
fn smooth_closed_figure_eight_refuses_a_genuine_original_self_intersection() {
    let path = figure_eight();
    let cap = saddle(true);
    let before = (path.clone(), cap.clone());
    assert_eq!(
        path.evaluate(0.).unwrap().point,
        path.evaluate(2.).unwrap().point
    );
    assert_eq!(cap.evaluate(0.5, 0.5).unwrap().point, [0.; 3]);
    let profiles = [crate::primitives::line([0.; 3], [1. / 1024., 0., 0.]).unwrap()];
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let zero = constant_vector_law([0.; 3]).unwrap();
    let sweep = MultiSweep::new(&profiles, &path, &scale, &zero, options([0., 0., 1.])).unwrap();
    let frame = sweep.sweeps[0]
        .certify_closed_path_frame_smoothness(1, 100000, 1000000)
        .unwrap();
    assert!(frame.closed_source_frame_smoothness_certified, "{frame:?}");
    // q(1/2,1/2)=0, so F at raw path parameters 0 and 2 is the same
    // original point. Positive arc speed makes these distinct S1 parameters.
    let r = sweep
        .certify_ideal_volume_atlas([&cap, &cap], projection(), 300000, 1000000)
        .unwrap();
    assert!(
        !r.embedding_certified && !r.regularity_certified && !r.parameter_topology_preserved,
        "{r:?}"
    );
    assert!(r.cells <= 300000 && r.exact_work <= 1000000);
    assert_eq!((path, cap), before);
}
