use super::*;

#[test]
fn curved_original_g2_survives_moving_frames_and_nonplanar_filled_caps_together() {
    run(false);
}
#[test]
fn curved_g2_moving_frame_and_independent_weighted_cap_representations_are_jointly_owned() {
    run(true);
}
fn run(alternate: bool) {
    // A quadratic XY loop lifted by z=x*y/128. Its degree-four XYZ controls
    // below are exact integers/dyadics; no rounded composition is authority.
    let x = [6., 6., 5., 3., 0.];
    let y = [0., 3., 5., 6., 6.];
    let z = [0., 18., 30., 18., 0.];
    let profiles = (0..4)
        .map(|quarter| Curve {
            degree: 4,
            knots: vec![0., 0., 0., 0., 0., 1., 1., 1., 1., 1.],
            weights: vec![1.; 5],
            periodic: false,
            control_points: (0..5)
                .map(|i| {
                    let (a, b) = match quarter {
                        0 => (x[i], y[i]),
                        1 => (-y[i], x[i]),
                        2 => (-x[i], -y[i]),
                        _ => (y[i], -x[i]),
                    };
                    vec![
                        a,
                        b,
                        if quarter % 2 == 0 {
                            z[i] / 128.
                        } else {
                            -z[i] / 128.
                        },
                    ]
                })
                .collect(),
        })
        .collect::<Vec<_>>();
    let corners = vec![
        vec![-8., -8., 0.5],
        vec![8., -8., -0.5],
        vec![8., 8., 0.5],
        vec![-8., 8., -0.5],
    ];
    let mut c = cap(corners.clone());
    let xy = [
        [6., 0.],
        [6., 6.],
        [0., 6.],
        [-6., 6.],
        [-6., 0.],
        [-6., -6.],
        [0., -6.],
        [6., -6.],
        [6., 0.],
    ];
    c.trims = vec![
        (0..4)
            .map(|i| Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                weights: vec![1.; 3],
                periodic: false,
                control_points: xy[2 * i..=2 * i + 2]
                    .iter()
                    .map(|p| vec![(p[0] + 8.) / 16., (p[1] + 8.) / 16.])
                    .collect(),
            })
            .collect(),
    ];
    let path = nurbs_core::primitives::line([0.; 3], [0., 0., 64.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let axis = constant_vector_law([0.5, 0., 1.]).unwrap();
    let normal = nurbs_core::primitives::line([1., 0., 0.], [1., 0.125, 0.]).unwrap();
    let options = Options {
        normal: [1., 0., 0.],
        orientation: Orientation::Fixed,
        spacing: Spacing::Parameter,
        initial_sections: 5,
        max_sections: 5,
        max_deviation: 0.05,
    };
    let sweep = MultiSweep::new(&profiles, &path, &scale, &twist, options.clone())
        .unwrap()
        .with_frame_laws(&axis, &normal)
        .unwrap();
    // Candidate cap controls only. The actual owner freshly binds their errors,
    // material geometry and the original curved source packet independently.
    let corner_profiles = (0..4)
        .map(|i| {
            Curve::from_polyline(vec![corners[i].clone(), corners[(i + 1) % 4].clone()]).unwrap()
        })
        .collect::<Vec<_>>();
    let ghost = MultiSweep::new(&corner_profiles, &path, &scale, &twist, options)
        .unwrap()
        .with_frame_laws(&axis, &normal)
        .unwrap();
    let rows = ghost.sections_at(5).unwrap();
    let retained = [0, 4].map(|station| {
        let mut result = cap(rows
            .iter()
            .map(|r| r[station].control_points[0].clone())
            .collect());
        result.trims = c.trims.clone();
        result
    });
    let strict = construct_with_original_smooth_volume(SweepSource { sweep: &sweep, loop_sizes: &[4], sections: 5, source_caps: [&c, &c] }, [&retained[0], &retained[1]], SweepAccuracy { tolerance_uv: 1e-8, error_budget: 0.05 }, limits(), [[1. / 16., 0., 0.], [0., 1. / 16., 0.], [0., 0., 1. / 64.]], OriginalSmoothVolumeLimits {
            order: 2,
            cells: 1000000,
            exact_work: 1000000,
            products: 1000000,
        })
    .err()
    .expect("independently rounded retained cap must not get exact admission");
    assert_eq!(strict.code, "BREP_RATIONAL_SWEEP_REFUSED");
    assert!(strict.message.contains("whole-boundary agreement"));
    let mut other = c.clone();
    if alternate {
        other.surface.degree_u = 2;
        other.surface.degree_v = 2;
        other.surface.knots_u = vec![17., 17., 17., 29., 29., 29.];
        other.surface.knots_v = vec![-7., -7., -7., 13., 13., 13.];
        other.surface.control_points = [-8., -4., 8.]
            .iter()
            .map(|&x| {
                [-8., -4., 8.]
                    .iter()
                    .map(|&y| vec![x, y, x * y / 128.])
                    .collect()
            })
            .collect();
        other.surface.weights = [1., 2., 3.]
            .iter()
            .map(|a| [1., 2., 3.].iter().map(|b| 8. * a * b).collect())
            .collect();
        for p in other
            .trims
            .iter_mut()
            .flatten()
            .flat_map(|c| &mut c.control_points)
        {
            p[0] = 17. + 12. * p[0];
            p[1] = -7. + 20. * p[1];
        }
    }
    let result = construct_affine_with_original_smooth_volume(SweepSource { sweep: &sweep, loop_sizes: &[4], sections: 5, source_caps: [&c, &other] }, SweepAccuracy { tolerance_uv: 1e-8, error_budget: 0.05 }, limits(), [[1. / 16., 0., 0.], [0., 1. / 16., 0.], [0., 0., 1. / 64.]], OriginalSmoothVolumeLimits {
            order: 2,
            cells: 1000000,
            exact_work: 1000000,
            products: 1000000,
        }, crate::AffineStationLimits {
            quantum: 1. / 1048576.,
            tolerance: 0.001,
            work: 1000000,
            matrix_cells: 10000,
        })
    .unwrap();
    let body = match result {
        OriginalSmoothBodyAttempt::Certified(b) => b,
        OriginalSmoothBodyAttempt::Refused(r) => {
            if let Some(v) = &r.retained.affine_volume {
                eprintln!(
                    "actual affine volume refusal: {} cells={} work={} packet={} roles={} volume={:?} orientation={:?}",
                    v.reason,
                    v.cells,
                    v.work,
                    v.packet_bound,
                    v.material_roles_certified,
                    v.volume,
                    v.orientation
                );
            }
            if let Some(v) = &r.retained.volume {
                eprintln!(
                    "curved retained diagnostics: boundary={} agreement={} trim={} self={} faces={} pairs={} nesting={} orientations={:?}",
                    v.boundary.proven,
                    v.boundary.agreement.all_equal,
                    v.boundary.trim.all_valid,
                    v.boundary.intersections.absence_proven,
                    v.boundary.intersections.faces.all_faces_injective,
                    v.boundary.intersections.pairs.all_pairs_classified,
                    v.nesting.is_some(),
                    v.orientations
                        .iter()
                        .map(|o| (o.shell, o.expected_outward, o.outward))
                        .collect::<Vec<_>>()
                );
                eprintln!(
                    "unproved faces={:?} pairs={:?}",
                    v.boundary
                        .intersections
                        .faces
                        .faces
                        .iter()
                        .filter(|f| !f.absence_proven())
                        .map(|f| f.face)
                        .collect::<Vec<_>>(),
                    v.boundary
                        .intersections
                        .pairs
                        .pairs
                        .iter()
                        .map(|p| (p.faces, p.reason))
                        .collect::<Vec<_>>()
                );
            }
            panic!(
                "curved moving saddle body: {}; original={:?}; retained={:?}",
                r.reason, r.original, r.retained.reason
            )
        }
    };
    let result =
        qualify_original_smooth_boundary(&body, &[Class::G2; 4], boundary_limits()).unwrap();
    let boundary = match result {
        OriginalBoundaryBodyAttempt::Certified(b) => b,
        OriginalBoundaryBodyAttempt::Refused(r) => panic!(
            "curved moving saddle G2: {}; profiles={:?}",
            r.reason, r.profiles
        ),
    };
    assert_eq!(boundary.profiles().report().smooth_joins, 4);
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
    let actual = boundary
        .body()
        .retained_report()
        .affine_volume
        .as_ref()
        .unwrap();
    assert!(std::ptr::eq(
        boundary.body().original().caps()[1],
        &other.surface
    ));
    let identity = actual.carrier_identity.as_ref().unwrap();
    assert!(identity.certified);
    if alternate {
        assert!(identity.cells > 0 && identity.exact_work > 0);
    }
    assert!(actual.certified && actual.packet_bound && actual.material_roles_certified);
    let mapped = actual.volume.as_ref().unwrap();
    assert!(
        mapped.embedding_certified
            && mapped.regularity_certified
            && mapped.parameter_topology_preserved
    );
    assert!(actual.orientation.as_ref().unwrap().sign.is_some());
    let agreement = actual.agreement.as_ref().unwrap();
    assert!(agreement.all_equal && agreement.all_joins_exact);
    assert!(actual.trim.as_ref().unwrap().all_valid);
    assert!(actual.cells <= 10000 && actual.work <= 1000000);
    assert!(
        !boundary
            .body()
            .retained_report()
            .volume
            .as_ref()
            .unwrap()
            .proven
    );
    for (cells, work) in [
        (0, 1000000),
        (actual.cells.saturating_sub(1), 1000000),
        (10000, actual.work - 1),
    ] {
        let short = construct_affine_with_original_smooth_volume(SweepSource { sweep: &sweep, loop_sizes: &[4], sections: 5, source_caps: [&c, &other] }, SweepAccuracy { tolerance_uv: 1e-8, error_budget: 0.05 }, limits(), [[1. / 16., 0., 0.], [0., 1. / 16., 0.], [0., 0., 1. / 64.]], OriginalSmoothVolumeLimits {
                order: 2,
                cells: 1000000,
                exact_work: 1000000,
                products: 1000000,
            }, crate::AffineStationLimits {
                quantum: 1. / 1048576.,
                tolerance: 0.001,
                work,
                matrix_cells: cells,
            })
        .unwrap();
        match short {
            OriginalSmoothBodyAttempt::Refused(short) => {
                assert!(!short.certified && !short.retained.solid_geometry_proven());
                let proof = short.retained.affine_volume.unwrap();
                assert!(!proof.certified && proof.cells <= cells && proof.work <= work);
            }
            OriginalSmoothBodyAttempt::Certified(short) => {
                // A previous adaptive visit count is not a lower bound: a
                // smaller allowance can choose another complete proof cover.
                assert!(cells > 0 && short.original().report().certified);
                let proof = short.retained_report().affine_volume.as_ref().unwrap();
                assert!(proof.certified && proof.cells <= cells && proof.work <= work);
                assert!(proof.packet_bound && proof.material_roles_certified);
                assert!(proof.volume.as_ref().unwrap().embedding_certified);
                assert!(
                    proof.agreement.as_ref().unwrap().all_equal
                        && proof.trim.as_ref().unwrap().all_valid
                );
                eprintln!(
                    "adaptive actual volume budget cells={cells} work={work}: complete cells={} work={}",
                    proof.cells, proof.work
                );
            }
        }
    }
    let zero = construct_affine_with_original_smooth_volume(SweepSource { sweep: &sweep, loop_sizes: &[4], sections: 5, source_caps: [&c, &other] }, SweepAccuracy { tolerance_uv: 1e-8, error_budget: 0.05 }, limits(), [[1. / 16., 0., 0.], [0., 1. / 16., 0.], [0., 0., 1. / 64.]], OriginalSmoothVolumeLimits {
            order: 2,
            cells: 1000000,
            exact_work: 1000000,
            products: 1000000,
        }, crate::AffineStationLimits {
            quantum: 1. / 1048576.,
            tolerance: 0.001,
            work: 0,
            matrix_cells: 10000,
        })
    .unwrap();
    let OriginalSmoothBodyAttempt::Refused(zero) = zero else {
        panic!("zero exact affine work admitted a body")
    };
    assert!(!zero.certified && !zero.retained.certified && zero.original.is_none());
    assert_eq!(zero.retained.affine_candidate.unwrap().work, 0);
    eprintln!(
        "curved G2 + moving tilted frame + nonplanar caps alternate={alternate}: cells={} exact={} products={} error={:?}",
        boundary.work().cells,
        boundary.work().exact_work,
        boundary.work().products,
        boundary.body().retained_report().boundary_error_upper
    );
}
