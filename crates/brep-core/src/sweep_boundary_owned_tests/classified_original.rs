use super::*;
use nurbs_core::progressive_sweep::{
    OriginalClassifiedVolumeLimits, OriginalProfileClass, OriginalSmoothVolumeLimits,
};
fn original_limits() -> OriginalClassifiedCappedMaterialLimits {
    OriginalClassifiedCappedMaterialLimits {
        volume: OriginalClassifiedVolumeLimits {
            charts: OriginalSmoothVolumeLimits {
                order: 2,
                cells: 1000000,
                exact_work: 1000000,
                products: 1000000,
            },
            steps: 1000000,
        },
        material: MaterialLimits {
            pairs: 100000,
            cells: 100000,
            domain_cells: 1000000,
        },
    }
}
fn source(g1: bool) -> (Vec<Curve>, LoftCap) {
    let mut c = cap(vec![
        vec![0., 0., 0.],
        vec![1., 0., 0.],
        vec![1., 1., 0.],
        vec![0., 1., 0.],
    ]);
    // Split the lower outer edge to require a genuine declared G2 join. Sharp
    // outer/hole corners remain C0. The material topology has one through hole.
    c.trims[0].splice(
        0..1,
        [[[0., 0.], [0.5, 0.]], [[0.5, 0.], [1., 0.]]].map(|p| Curve {
            degree: 1,
            knots: vec![-3., -3., 5., 5.],
            control_points: p.iter().map(|p| p.to_vec()).collect(),
            weights: vec![1.; 2],
            periodic: false,
        }),
    );
    let hole = [
        [0.25, 0.25],
        [0.25, 0.75],
        [0.75, 0.75],
        [0.75, 0.25],
        [0.25, 0.25],
    ];
    c.trims.push(
        hole.windows(2)
            .map(|p| Curve {
                degree: 1,
                knots: vec![7., 7., 11., 11.],
                control_points: p.iter().map(|p| p.to_vec()).collect(),
                weights: vec![1.; 2],
                periodic: false,
            })
            .collect(),
    );
    if g1 {
        // The first line meets a quadratic parabola with the same tangent but
        // a nonzero normal second derivative. Plane UV lifts are exact.
        c.trims[0][1] = Curve {
            degree: 2,
            knots: vec![-3., -3., -3., 5., 5., 5.],
            control_points: vec![vec![0.5, 0.], vec![0.75, 0.], vec![1., 0.25]],
            weights: vec![1.; 3],
            periodic: false,
        };
        c.trims[0][2].control_points[0][1] = 0.25;
    }
    let profiles = c
        .trims
        .iter()
        .flatten()
        .map(|c| Curve {
            degree: c.degree,
            knots: c.knots.clone(),
            control_points: c
                .control_points
                .iter()
                .map(|p| vec![p[0], p[1], 0.])
                .collect(),
            weights: c.weights.clone(),
            periodic: false,
        })
        .collect();
    (profiles, c)
}
fn run(mode: usize, affine: bool, unequal: bool, g1: bool) {
    let (profiles, source) = source(g1);
    let path = nurbs_core::primitives::line([0.; 3], [0., 0., 2.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let axis = constant_vector_law([0., 0., 1.]).unwrap();
    let normal = if mode == 0 {
        constant_vector_law([1., 0., 0.]).unwrap()
    } else {
        Curve {
            degree: 2,
            knots: vec![17., 17., 17., 29., 29., 29.],
            control_points: vec![vec![1., 0., 0.], vec![1., 1. / 16., 0.], vec![1., 0., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        }
    };
    let guide = Curve {
        degree: 2,
        knots: vec![-9., -9., -9., 7., 7., 7.],
        control_points: vec![vec![1., 0., 0.], vec![1., 1. / 16., 1.], vec![1., 0., 2.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        Options {
            normal: [1., 0., 0.],
            orientation: if mode == 2 {
                Orientation::RotationMinimizing
            } else {
                Orientation::Fixed
            },
            spacing: Spacing::Parameter,
            initial_sections: 5,
            max_sections: 5,
            max_deviation: 0.05,
        },
    )
    .unwrap();
    let sweep = if mode == 2 {
        sweep.with_orientation_guide(&guide)
    } else {
        sweep.with_frame_laws(&axis, &normal)
    }
    .unwrap();
    let source_end = if unequal {
        unequal_cap(&source)
    } else {
        source.clone()
    };
    let before = (
        profiles.clone(),
        source.surface.clone(),
        source.trims.clone(),
        source_end.surface.clone(),
        source_end.trims.clone(),
    );
    let mut end = source_end.clone();
    for p in end.surface.control_points.iter_mut().flatten() {
        p[2] += 2.;
    }
    let classes = [
        if g1 {
            OriginalProfileClass::G1
        } else {
            OriginalProfileClass::G2
        },
        OriginalProfileClass::C0,
        OriginalProfileClass::C0,
        OriginalProfileClass::C0,
        OriginalProfileClass::C0,
        OriginalProfileClass::C0,
        OriginalProfileClass::C0,
        OriginalProfileClass::C0,
        OriginalProfileClass::C0,
    ];
    let projection = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    let affine_limits = crate::AffineStationLimits {
        quantum: 1. / 1048576.,
        tolerance: 0.001,
        work: 1000000,
        matrix_cells: 10000,
    };
    let run = |l| {
        if affine {
            construct_affine_with_original_classified_material(
                &sweep,
                &[5, 4],
                5,
                [&source, &source_end],
                &classes,
                1e-8,
                0.05,
                limits(),
                projection,
                l,
                affine_limits,
            )
            .unwrap()
        } else {
            construct_with_original_classified_material(
                &sweep,
                &[5, 4],
                5,
                [&source, &source_end],
                [&source, &end],
                &classes,
                1e-8,
                0.05,
                limits(),
                projection,
                l,
            )
            .unwrap()
        }
    };
    let body = match run(original_limits()) {
        OriginalClassifiedBodyAttempt::Certified(b) => b,
        OriginalClassifiedBodyAttempt::Refused(r) => {
            if let Some(v) = r.retained.as_ref().and_then(|r| r.volume.as_ref()) {
                eprintln!(
                    "retained failure: boundary={} agreement={}/{} trim={} injective={} unresolved={:?}",
                    v.boundary.proven,
                    v.boundary.agreement.all_equal,
                    v.boundary.agreement.all_joins_exact,
                    v.boundary.trim.all_valid,
                    v.boundary.intersections.faces.all_faces_injective,
                    v.boundary
                        .intersections
                        .pairs
                        .pairs
                        .iter()
                        .filter(|p| p.reason == "pair-unresolved" || p.reason == "interior-contact")
                        .map(|p| (
                            p.faces,
                            p.reason,
                            p.result.as_ref().map(|r| (r.cells, r.domain_cells))
                        ))
                        .collect::<Vec<_>>()
                );
            }
            panic!(
                "original material to retained body mode={mode} affine={affine} unequal={unequal} g1={g1}: {}",
                r.reason
            );
        }
    };
    assert_eq!(body.boundary_classes(), classes);
    assert_eq!(body.cap_wall_continuity(), "C0");
    assert!(std::ptr::eq(body.original().volume().sweep(), &sweep));
    assert!(std::ptr::eq(body.original().caps()[0], &source));
    assert!(std::ptr::eq(body.original().caps()[1], &source_end));
    assert!(
        body.retained_report().certified
            && body.retained_report().continuous_bound
            && body.retained_report().solid_geometry_proven()
    );
    if unequal {
        let actual = body
            .retained_report()
            .affine_volume
            .as_ref()
            .expect("fresh paired-carrier filled proof");
        assert!(
            actual.certified
                && actual.paired_carriers
                && actual.packet_bound
                && actual.material_roles_certified
        );
        assert!(actual.volume.as_ref().unwrap().embedding_certified);
        assert!(
            !body.retained_report().volume.as_ref().unwrap().proven,
            "the unresolved pair census must remain unresolved in its diagnostics"
        );
    }
    assert_eq!(body.model().bodies.len(), 1);
    assert_eq!(
        body.model()
            .faces
            .iter()
            .filter(|f| !f.holes.is_empty())
            .count(),
        2
    );
    let error = body.retained_report().boundary_error_upper.unwrap();
    assert!(error > 0. && error <= 0.05);
    if mode == 0 && !affine {
        assert!(error < 1e-10);
    }
    let work = body.original().work();
    let mut tight = original_limits();
    tight.volume.charts.cells = work.cells;
    tight.volume.charts.exact_work = work.exact_work;
    tight.volume.charts.products = work.products;
    tight.material.pairs = work.material_pairs;
    tight.material.cells = work.material_cells;
    tight.material.domain_cells = work.material_domain_cells;
    if let OriginalClassifiedBodyAttempt::Refused(r) = run(tight) {
        panic!(
            "measured original replay: reason={} work={:?} expected={work:?} retained={:?}",
            r.reason,
            r.original_work,
            r.retained.as_ref().map(|r| r.reason)
        );
    }
    for d in 0..6 {
        let mut short = tight;
        let positive = match d {
            0 => work.cells > 0,
            1 => work.exact_work > 0,
            2 => work.products > 0,
            3 => work.material_pairs > 0,
            4 => work.material_cells > 0,
            _ => work.material_domain_cells > 0,
        };
        if !positive {
            continue;
        }
        match d {
            0 => short.volume.charts.cells -= 1,
            1 => short.volume.charts.exact_work -= 1,
            2 => short.volume.charts.products -= 1,
            3 => short.material.pairs -= 1,
            4 => short.material.cells -= 1,
            _ => short.material.domain_cells -= 1,
        }
        let OriginalClassifiedBodyAttempt::Refused(r) = run(short) else {
            panic!("incomplete original work admitted")
        };
        assert!(r.original.is_some() && r.retained.is_none());
        assert!(
            r.original_work.cells <= short.volume.charts.cells
                && r.original_work.exact_work <= short.volume.charts.exact_work
                && r.original_work.products <= short.volume.charts.products
                && r.original_work.material_pairs <= short.material.pairs
                && r.original_work.material_cells <= short.material.cells
                && r.original_work.material_domain_cells <= short.material.domain_cells
        );
    }
    if g1 {
        let mut upgraded = classes;
        upgraded[0] = OriginalProfileClass::G2;
        let r = construct_with_original_classified_material(
            &sweep,
            &[5, 4],
            5,
            [&source, &source_end],
            [&source, &end],
            &upgraded,
            1e-8,
            0.05,
            limits(),
            projection,
            original_limits(),
        )
        .unwrap();
        let OriginalClassifiedBodyAttempt::Refused(r) = r else {
            panic!("true G1 source curvature jump acquired G2")
        };
        assert!(r.original.is_some() && r.retained.is_none());
    }
    let mut short = limits();
    short.source_cells = 0;
    let r = construct_with_original_classified_material(
        &sweep,
        &[5, 4],
        5,
        [&source, &source_end],
        [&source, &end],
        &classes,
        1e-8,
        0.05,
        short,
        projection,
        original_limits(),
    )
    .unwrap();
    let OriginalClassifiedBodyAttempt::Refused(r) = r else {
        panic!("retained source work is mandatory")
    };
    assert!(r.original.is_none() && r.retained.is_some());
    assert!(r.original_work.exact_work > 0);
    let r = construct_with_original_classified_material(
        &sweep,
        &[5, 4],
        5,
        [&source, &source_end],
        [&source, &end],
        &vec![OriginalProfileClass::G2; 9],
        1e-8,
        0.05,
        limits(),
        projection,
        original_limits(),
    )
    .unwrap();
    let OriginalClassifiedBodyAttempt::Refused(r) = r else {
        panic!("sharp original corners acquired G2")
    };
    assert!(r.original.is_some() && r.retained.is_none());
    assert_eq!(
        before,
        (
            profiles.clone(),
            source.surface.clone(),
            source.trims.clone(),
            source_end.surface.clone(),
            source_end.trims.clone()
        )
    );
    eprintln!(
        "classified retained hollow mode={mode} affine={affine} unequal={unequal} g1={g1}: original={:?}, retained error={:?}",
        body.original().work(),
        body.retained_report().boundary_error_upper
    );
}

fn unequal_cap(source: &LoftCap) -> LoftCap {
    let q = [0., -9. / 16., 5. / 4., -9. / 16., 0.];
    let surface = Surface {
        degree_u: 4,
        degree_v: 4,
        knots_u: vec![17., 17., 17., 17., 17., 25., 25., 25., 25., 25.],
        knots_v: vec![-7., -7., -7., -7., -7., 1., 1., 1., 1., 1.],
        control_points: (0..5)
            .map(|i| {
                (0..5)
                    .map(|j| vec![i as f64 / 4., j as f64 / 4., q[i] * q[j] / 64.])
                    .collect()
            })
            .collect(),
        weights: vec![vec![2.; 5]; 5],
        periodic_u: false,
        periodic_v: false,
    };
    let mut trims = source.trims.clone();
    for c in trims.iter_mut().flatten() {
        for p in &mut c.control_points {
            p[0] = 17. + 8. * p[0];
            p[1] = -7. + 8. * p[1];
        }
        for w in &mut c.weights {
            *w *= 2.;
        }
    }
    LoftCap { surface, trims }
}
#[test]
fn classified_ideal_hollow_body_and_retained_solid_share_one_original_packet() {
    run(0, false, false, false)
}
#[test]
fn classified_hollow_authored_frame_owns_actual_solid_and_ideal_material() {
    run(1, false, false, false)
}
#[test]
fn classified_hollow_guide_owns_actual_solid_and_ideal_material() {
    run(2, false, false, false)
}
#[test]
fn classified_hollow_affine_packet_owns_actual_solid_and_ideal_material() {
    run(1, true, false, false)
}
#[test]
fn unequal_filled_caps_and_a_hole_bind_ideal_material_to_actual_solid() {
    run(0, true, true, false)
}

#[test]
fn genuine_g1_original_curvature_jump_and_sharp_joins_own_a_retained_solid() {
    run(0, false, false, true)
}
