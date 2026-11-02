use super::*;
use crate as nurbs;
#[path = "../../../../tests/support/general_frame_source.rs"]
mod source;
fn limits() -> OriginalClassifiedVolumeLimits {
    OriginalClassifiedVolumeLimits {
        charts: OriginalSmoothVolumeLimits {
            order: 2,
            cells: 1000000,
            exact_work: 1000000,
            products: 1000000,
        },
        steps: 1000000,
    }
}
fn projection() -> [[f64; 3]; 3] {
    [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]
}
fn polygon(scale: f64) -> Vec<Curve> {
    let r = scale / 1024.;
    let xyz = |p: [f64; 2]| [-2. * r * p[0], -r * p[1], 9. * r * p[0]];
    let corners = [
        [-1., -1.],
        [0., -1.],
        [1., -1.],
        [1., 1.],
        [-1., 1.],
        [-1., -1.],
    ];
    corners
        .windows(2)
        .map(|p| crate::primitives::line(xyz(p[0]), xyz(p[1])).unwrap())
        .collect()
}
fn run(inner: bool) {
    let path = source::path();
    let (a, n, twist) = source::laws();
    let (_, cap, _) = source::material();
    let mut profiles = polygon(1.);
    let mut loops = vec![5];
    let mut classes = vec![
        OriginalProfileClass::G2,
        OriginalProfileClass::C0,
        OriginalProfileClass::C0,
        OriginalProfileClass::C0,
        OriginalProfileClass::C0,
    ];
    if inner {
        // The native owner checks trace jets; it deliberately does not claim
        // material containment, nesting or hole orientation from this partition.
        profiles.extend(polygon(0.25));
        loops.push(5);
        classes.extend(classes.clone());
    }
    let before = (profiles.clone(), cap.clone(), path.clone());
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        Options {
            normal: [0., 0., 1.],
            orientation: Orientation::Fixed,
            spacing: Spacing::ArcLength {
                tolerance: 0.001,
                max_cells: 100000,
            },
            initial_sections: 33,
            max_sections: 129,
            max_deviation: 0.05,
        },
    )
    .unwrap()
    .with_frame_laws(&a, &n)
    .unwrap();
    let certify = |c: &[OriginalProfileClass], l| {
        sweep
            .certify_original_classified_volume_atlas([&cap, &cap], &loops, c, projection(), l)
            .unwrap()
    };
    let body = match certify(&classes, limits()) {
        OriginalClassifiedVolumeAttempt::Certified(b) => b,
        OriginalClassifiedVolumeAttempt::Refused(r) => {
            panic!("mixed closed original volume refused: {r:?}")
        }
    };
    let r = body.report();
    assert!(
        r.certified
            && r.charts.certified
            && r.charts.motion_continuity_certified
            && r.charts.cap_continuity_certified
    );
    assert!(
        r.charts.volume.embedding_certified
            && r.charts.volume.regularity_certified
            && r.charts.volume.parameter_topology_preserved
    );
    let motion = r.charts.closed_boundary.as_ref().unwrap();
    assert!(motion.frame_certified && motion.laws_certified);
    assert!(!motion.profiles_certified && !motion.differential_compatibility_certified);
    assert_eq!(motion.inspected_profile_joins, 0);
    let jets = r.profiles.as_ref().unwrap();
    assert_eq!(jets.inspected_joins, 5 * loops.len());
    assert_eq!(jets.smooth_joins, loops.len());
    assert_eq!(body.classes(), classes);
    assert_eq!(body.loop_sizes(), loops);
    assert!(std::ptr::eq(body.sweep(), &sweep) && std::ptr::eq(body.caps()[0], &cap));
    let exact = OriginalClassifiedVolumeLimits {
        charts: OriginalSmoothVolumeLimits {
            cells: r.cells,
            exact_work: r.exact_work,
            products: r.products,
            ..limits().charts
        },
        steps: jets.steps,
    };
    assert!(matches!(
        certify(&classes, exact),
        OriginalClassifiedVolumeAttempt::Certified(_)
    ));
    for short in [
        OriginalClassifiedVolumeLimits {
            charts: OriginalSmoothVolumeLimits {
                cells: 0,
                ..exact.charts
            },
            ..exact
        },
        OriginalClassifiedVolumeLimits {
            charts: OriginalSmoothVolumeLimits {
                exact_work: 0,
                ..exact.charts
            },
            ..exact
        },
        OriginalClassifiedVolumeLimits {
            charts: OriginalSmoothVolumeLimits {
                products: r.products - 1,
                ..exact.charts
            },
            ..exact
        },
        OriginalClassifiedVolumeLimits {
            charts: OriginalSmoothVolumeLimits {
                products: 0,
                ..exact.charts
            },
            ..exact
        },
        OriginalClassifiedVolumeLimits {
            steps: jets.steps - 1,
            ..exact
        },
        OriginalClassifiedVolumeLimits { steps: 0, ..exact },
    ] {
        let OriginalClassifiedVolumeAttempt::Refused(q) = certify(&classes, short) else {
            panic!("exhausted aggregate work admitted")
        };
        assert!(
            !q.certified
                && q.cells <= short.charts.cells
                && q.exact_work <= short.charts.exact_work
                && q.products <= short.charts.products
        );
        if let Some(p) = q.profiles {
            assert!(p.steps <= short.steps);
        }
    }
    let mut promoted = classes.clone();
    promoted[1] = OriginalProfileClass::G2;
    let OriginalClassifiedVolumeAttempt::Refused(q) = certify(&promoted, limits()) else {
        panic!("sharp corner acquired G2")
    };
    assert!(q.charts.certified && !q.certified);
    assert_eq!(q.profiles.unwrap().uncertain_join, Some([0, 1]));
    for bad in [
        classes[..classes.len() - 1].to_vec(),
        [classes.clone(), vec![OriginalProfileClass::C0]].concat(),
    ] {
        let OriginalClassifiedVolumeAttempt::Refused(q) = certify(&bad, limits()) else {
            panic!("class partition mismatch admitted")
        };
        assert_eq!(
            q.reason,
            Some("original-profile-boundary-class-count-mismatch")
        );
    }
    // Old uniform API must still reject the same sharp original polygon.
    assert!(matches!(
        sweep
            .certify_original_smooth_volume_atlas(
                [&cap, &cap],
                &loops,
                projection(),
                limits().charts
            )
            .unwrap(),
        OriginalSmoothVolumeAttempt::Refused(_)
    ));
    assert_eq!((&profiles, &cap, &path), (&before.0, &before.1, &before.2));
    eprintln!(
        "mixed closed C2 original volume: rings={} cells={} exact={} products={} profile_steps={}",
        loops.len(),
        r.cells,
        r.exact_work,
        r.products,
        jets.steps
    );
}
#[test]
fn spatial_closed_volume_retains_declared_sharp_corners_and_g2_split() {
    run(false);
}
#[test]
fn spatial_closed_volume_covers_all_profile_rings_without_material_credit() {
    run(true);
}
#[test]
fn closed_rational_g1_joins_do_not_acquire_g2_or_ignore_broken_motion() {
    let path = source::path();
    let (a, n, twist) = source::laws();
    let (mut profiles, cap, _) = source::material();
    profiles[0].weights[1] = 0.25;
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let options = Options {
        normal: [0., 0., 1.],
        orientation: Orientation::Fixed,
        spacing: Spacing::ArcLength {
            tolerance: 0.001,
            max_cells: 100000,
        },
        initial_sections: 33,
        max_sections: 129,
        max_deviation: 0.05,
    };
    let sweep = MultiSweep::new(&profiles, &path, &scale, &twist, options.clone())
        .unwrap()
        .with_frame_laws(&a, &n)
        .unwrap();
    let classes = [
        OriginalProfileClass::G1,
        OriginalProfileClass::G2,
        OriginalProfileClass::G2,
        OriginalProfileClass::G1,
    ];
    let body = match sweep
        .certify_original_classified_volume_atlas(
            [&cap, &cap],
            &[4],
            &classes,
            projection(),
            limits(),
        )
        .unwrap()
    {
        OriginalClassifiedVolumeAttempt::Certified(b) => b,
        OriginalClassifiedVolumeAttempt::Refused(r) => panic!("closed mixed rational joins: {r:?}"),
    };
    assert_eq!(body.report().profiles.as_ref().unwrap().smooth_joins, 4);
    let OriginalClassifiedVolumeAttempt::Refused(r) = sweep
        .certify_original_classified_volume_atlas(
            [&cap, &cap],
            &[4],
            &[OriginalProfileClass::G2; 4],
            projection(),
            limits(),
        )
        .unwrap()
    else {
        panic!("unequal rational curvature acquired G2")
    };
    assert!(r.charts.certified && !r.certified);
    assert_eq!(r.profiles.unwrap().uncertain_join, Some([0, 0]));
    let mut broken = twist.clone();
    broken.control_points[4][0] = 0.125;
    let bad = MultiSweep::new(&profiles, &path, &scale, &broken, options)
        .unwrap()
        .with_frame_laws(&a, &n)
        .unwrap();
    let OriginalClassifiedVolumeAttempt::Refused(r) = bad
        .certify_original_classified_volume_atlas(
            [&cap, &cap],
            &[4],
            &[OriginalProfileClass::C0; 4],
            projection(),
            limits(),
        )
        .unwrap()
    else {
        panic!("C0 profile declarations bypassed periodic motion")
    };
    assert!(!r.certified && !r.charts.motion_continuity_certified);
}

mod guided;

mod adversarial;
