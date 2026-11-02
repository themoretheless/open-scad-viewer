use super::*;
use nurbs_core::progressive_sweep::{
    OriginalProfileBoundaryLimits, OriginalProfileClass as Class, OriginalSmoothVolumeLimits,
};
fn source(weights: [f64; 4]) -> (Vec<Curve>, LoftCap) {
    let xy = [
        [1., 0.],
        [1., 1.],
        [0., 1.],
        [-1., 1.],
        [-1., 0.],
        [-1., -1.],
        [0., -1.],
        [1., -1.],
        [1., 0.],
    ];
    let profiles = (0..4)
        .map(|i| Curve {
            degree: 2,
            knots: vec![7., 7., 7., 11., 11., 11.],
            control_points: xy[2 * i..=2 * i + 2]
                .iter()
                .map(|p| vec![p[0], p[1], 0.])
                .collect(),
            weights: vec![1., weights[i], 1.],
            periodic: false,
        })
        .collect::<Vec<_>>();
    let mut c = cap(vec![
        vec![-2., -2., 0.],
        vec![2., -2., 0.],
        vec![2., 2., 0.],
        vec![-2., 2., 0.],
    ]);
    c.trims = vec![
        profiles
            .iter()
            .map(|p| Curve {
                degree: p.degree,
                knots: p.knots.clone(),
                weights: p.weights.clone(),
                periodic: false,
                control_points: p
                    .control_points
                    .iter()
                    .map(|p| vec![(p[0] + 2.) / 4., (p[1] + 2.) / 4.])
                    .collect(),
            })
            .collect(),
    ];
    (profiles, c)
}
fn boundary_limits() -> OriginalProfileBoundaryLimits {
    OriginalProfileBoundaryLimits {
        steps: 100000,
        cells: 1000000,
        exact_work: 1000000,
        products: 1000000,
    }
}
#[test]
fn original_rational_wall_g2_and_mixed_sharp_g1_classes_belong_to_the_complete_material_body() {
    for (mixed, merged) in [(false, false), (true, false), (false, true), (true, true)] {
        let (mut profiles, c) = source(if mixed { [0.5, 0.5, 1., 1.] } else { [0.5; 4] });
        if merged {
            let control_points = std::iter::once(profiles[0].control_points[0].clone())
                .chain(
                    profiles
                        .iter()
                        .flat_map(|p| p.control_points[1..].iter().cloned()),
                )
                .collect();
            let weights = std::iter::once(1.)
                .chain(profiles.iter().flat_map(|p| p.weights[1..].iter().copied()))
                .collect();
            profiles = vec![Curve {
                degree: 2,
                knots: vec![0., 0., 0., 0.25, 0.25, 0.5, 0.5, 0.75, 0.75, 1., 1., 1.],
                control_points,
                weights,
                periodic: false,
            }];
        }
        let mut end = c.clone();
        for p in end.surface.control_points.iter_mut().flatten() {
            p[2] += 64.;
        }
        let path = nurbs_core::primitives::line([0.; 3], [0., 0., 64.]).unwrap();
        let scale = constant_vector_law([1., 0., 0.]).unwrap();
        let twist = constant_vector_law([0.; 3]).unwrap();
        let sweep = MultiSweep::new(
            &profiles,
            &path,
            &scale,
            &twist,
            Options {
                normal: [1., 0., 0.],
                orientation: Orientation::Fixed,
                spacing: Spacing::Parameter,
                initial_sections: 5,
                max_sections: 5,
                max_deviation: 0.01,
            },
        )
        .unwrap();
        let before = (profiles.clone(), c.surface.clone(), c.trims.clone());
        let make = |order| {
            construct_with_original_smooth_volume(
                &sweep,
                &[profiles.len()],
                5,
                [&c, &c],
                [&c, &end],
                1e-8,
                0.01,
                limits(),
                [[1., 0., 0.], [0., 1., 0.], [0., 0., 1. / 64.]],
                OriginalSmoothVolumeLimits {
                    order,
                    cells: 1000000,
                    exact_work: 1000000,
                    products: 1000000,
                },
            )
            .unwrap()
        };
        let body = match make(2) {
            OriginalSmoothBodyAttempt::Certified(b) => b,
            OriginalSmoothBodyAttempt::Refused(r) => panic!(
                "original conic material body: {}; original={:?}",
                r.reason, r.original
            ),
        };
        let classes = if mixed {
            [Class::G2, Class::G1, Class::C0, Class::G1]
        } else {
            [Class::G2; 4]
        };
        let run =
            |classes: &[Class], l| qualify_original_smooth_boundary(&body, classes, l).unwrap();
        let qualified = match run(&classes, boundary_limits()) {
            OriginalBoundaryBodyAttempt::Certified(b) => b,
            OriginalBoundaryBodyAttempt::Refused(r) => {
                panic!("original wall Gk: {}; profiles={:?}", r.reason, r.profiles)
            }
        };
        assert!(std::ptr::eq(qualified.body(), &*body));
        assert!(std::ptr::eq(qualified.profiles().volume(), body.original()));
        assert_eq!(qualified.profiles().classes(), &classes);
        assert_eq!(qualified.profiles().report().inspected_joins, 4);
        assert_eq!(
            qualified.profiles().report().smooth_joins,
            if mixed { 3 } else { 4 }
        );
        assert_eq!(qualified.cap_wall_continuity(), "C0");
        assert!(
            qualified
                .body()
                .retained_report()
                .volume
                .as_ref()
                .unwrap()
                .proven
        );
        assert!(
            qualified
                .body()
                .original()
                .report()
                .volume
                .parameter_topology_preserved
        );
        let work = qualified.work();
        let steps = qualified.profiles().report().steps;
        assert!(work.cells <= 1000000 && work.exact_work <= 1000000 && work.products <= 1000000);
        let exact = OriginalProfileBoundaryLimits {
            steps,
            cells: work.cells,
            exact_work: work.exact_work,
            products: work.products,
        };
        assert!(matches!(
            run(&classes, exact),
            OriginalBoundaryBodyAttempt::Certified(_)
        ));
        for l in [
            OriginalProfileBoundaryLimits { steps: 0, ..exact },
            OriginalProfileBoundaryLimits { cells: 0, ..exact },
            OriginalProfileBoundaryLimits {
                exact_work: 0,
                ..exact
            },
            OriginalProfileBoundaryLimits {
                products: 0,
                ..exact
            },
            OriginalProfileBoundaryLimits {
                steps: steps - 1,
                ..exact
            },
            OriginalProfileBoundaryLimits {
                cells: work.cells - 1,
                ..exact
            },
            OriginalProfileBoundaryLimits {
                exact_work: work.exact_work - 1,
                ..exact
            },
            OriginalProfileBoundaryLimits {
                products: work.products - 1,
                ..exact
            },
        ] {
            let OriginalBoundaryBodyAttempt::Refused(r) = run(&classes, l) else {
                panic!("exhausted full boundary work admitted")
            };
            assert!(
                !r.certified
                    && r.work.cells <= l.cells
                    && r.work.exact_work <= l.exact_work
                    && r.work.products <= l.products
            );
        }
        for classes in [&classes[..3], &[Class::G2; 5][..]] {
            assert!(matches!(
                run(classes, boundary_limits()),
                OriginalBoundaryBodyAttempt::Refused(_)
            ));
        }
        if mixed {
            let OriginalBoundaryBodyAttempt::Refused(r) = run(&[Class::G2; 4], boundary_limits())
            else {
                panic!("curvature jump promoted to G2")
            };
            assert_eq!(r.profiles.unwrap().uncertain_join, Some([0, 1]));
        }
        let lower = match make(1) {
            OriginalSmoothBodyAttempt::Certified(b) => b,
            _ => panic!("C1 original conic body"),
        };
        let OriginalBoundaryBodyAttempt::Refused(r) =
            qualify_original_smooth_boundary(&lower, &classes, boundary_limits()).unwrap()
        else {
            panic!("C1 volume alone manufactured G2")
        };
        assert_eq!(r.reason, "original-profile-boundary-volume-order-unproved");
        assert_eq!((profiles, c.surface, c.trims), before);
        eprintln!(
            "original rational wall mixed={mixed} merged={merged}: cells={} exact={} products={} steps={steps}",
            work.cells, work.exact_work, work.products
        );
    }
}

#[test]
fn longitudinal_g2_is_bound_to_the_actual_moving_tilted_frame_and_nonplanar_filled_caps() {
    let corners = vec![
        vec![-0.5, -1., 0.5],
        vec![0.5, -1., -0.5],
        vec![0.5, 1., 0.5],
        vec![-0.5, 1., -0.5],
    ];
    let mut c = cap(corners.clone());
    let points = [
        corners[0].clone(),
        vec![0., -1., 0.],
        corners[1].clone(),
        corners[2].clone(),
        corners[3].clone(),
    ];
    let profiles = (0..5)
        .map(|i| {
            Curve::from_polyline(vec![points[i].clone(), points[(i + 1) % 5].clone()]).unwrap()
        })
        .collect::<Vec<_>>();
    let mut first = c.trims[0][0].clone();
    first.control_points[1][0] = 0.5;
    let mut second = c.trims[0][0].clone();
    second.control_points[0][0] = 0.5;
    c.trims[0].splice(0..1, [first, second]);
    let path = nurbs_core::primitives::line([0.; 3], [0., 0., 1.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let axis = constant_vector_law([0.5, 0., 1.]).unwrap();
    let normal = nurbs_core::primitives::line([1., 0., 0.], [1., 0.125, 0.]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        Options {
            normal: [1., 0., 0.],
            orientation: Orientation::Fixed,
            spacing: Spacing::Parameter,
            initial_sections: 5,
            max_sections: 5,
            max_deviation: 0.05,
        },
    )
    .unwrap()
    .with_frame_laws(&axis, &normal)
    .unwrap();
    let rows = sweep.sections_at(5).unwrap();
    let retained = [0, 4].map(|station| {
        let mut result = cap([0, 2, 3, 4]
            .iter()
            .map(|&i| rows[i][station].control_points[0].clone())
            .collect());
        result.trims = c.trims.clone();
        result
    });
    let strict = construct_with_original_smooth_volume(
        &sweep,
        &[5],
        5,
        [&c, &c],
        [&retained[0], &retained[1]],
        1e-8,
        0.05,
        limits(),
        [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        OriginalSmoothVolumeLimits {
            order: 2,
            cells: 100000,
            exact_work: 1000000,
            products: 1000000,
        },
    )
    .err()
    .expect("independently rounded retained cap must not get exact admission");
    assert_eq!(strict.code, "BREP_RATIONAL_SWEEP_REFUSED");
    assert!(strict.message.contains("whole-boundary agreement"));
    let attempt = construct_affine_with_original_smooth_volume(
        &sweep,
        &[5],
        5,
        [&c, &c],
        1e-8,
        0.05,
        limits(),
        [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        OriginalSmoothVolumeLimits {
            order: 2,
            cells: 100000,
            exact_work: 1000000,
            products: 1000000,
        },
        crate::AffineStationLimits {
            quantum: 1. / 1048576.,
            tolerance: 0.001,
            work: 1000000,
            matrix_cells: 10000,
        },
    )
    .unwrap();
    let body = match attempt {
        OriginalSmoothBodyAttempt::Certified(b) => b,
        OriginalSmoothBodyAttempt::Refused(r) => panic!(
            "split moving saddle body: {}; original={:?}",
            r.reason, r.original
        ),
    };
    let classes = [Class::G2, Class::C0, Class::C0, Class::C0, Class::C0];
    let result = qualify_original_smooth_boundary(&body, &classes, boundary_limits()).unwrap();
    let boundary = match result {
        OriginalBoundaryBodyAttempt::Certified(b) => b,
        OriginalBoundaryBodyAttempt::Refused(r) => panic!(
            "moving saddle G2 wall: {}; profiles={:?}",
            r.reason, r.profiles
        ),
    };
    assert_eq!(boundary.profiles().classes(), &classes);
    assert_eq!(boundary.profiles().report().inspected_joins, 5);
    assert_eq!(boundary.profiles().report().smooth_joins, 1);
    assert_eq!(boundary.cap_wall_continuity(), "C0");
    let correction = boundary
        .body()
        .retained_report()
        .affine_candidate
        .as_ref()
        .unwrap();
    assert!(correction.coherent_controls && correction.wall_displacement_upper.unwrap() > 0.);
    assert!(correction.work <= 1000000);
    let retained = boundary.body().retained_report();
    let corrected = nurbs_core::numerics::error_upper::add(
        retained.wall.continuous_error_upper.unwrap(),
        correction.wall_displacement_upper.unwrap(),
    )
    .unwrap();
    assert!(retained.boundary_error_upper.unwrap() >= corrected);
    assert!(
        retained
            .cap_errors
            .unwrap()
            .iter()
            .all(|e| *e <= retained.boundary_error_upper.unwrap())
    );
    assert!(
        boundary
            .body()
            .original()
            .report()
            .volume
            .embedding_certified
    );
    assert!(boundary.body().retained_report().solid_geometry_proven());
    assert!(
        boundary
            .body()
            .retained_report()
            .boundary_error_upper
            .unwrap()
            <= 0.05
    );
    let OriginalBoundaryBodyAttempt::Refused(r) =
        qualify_original_smooth_boundary(&body, &[Class::G2; 5], boundary_limits()).unwrap()
    else {
        panic!("sharp corner promoted to G2")
    };
    assert_eq!(r.profiles.unwrap().uncertain_join, Some([0, 1]));
    eprintln!(
        "moving tilted nonplanar original G2 wall: cells={} exact={} products={}",
        boundary.work().cells,
        boundary.work().exact_work,
        boundary.work().products
    );
}

mod curved_affine;
