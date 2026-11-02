//! Two genuinely different filled caps under nonconstant spatial source frames.
use super::*;
fn curve(points: &[[f64; 3]], weights: &[f64], raw: [f64; 2]) -> Curve {
    let degree = points.len() - 1;
    Curve {
        degree,
        knots: [vec![raw[0]; degree + 1], vec![raw[1]; degree + 1]].concat(),
        control_points: points.iter().map(|p| p.to_vec()).collect(),
        weights: weights.to_vec(),
        periodic: false,
    }
}
#[test]
fn authored_spatial_two_cap_hole_clock_and_affine_matrix() {
    for arc in [false, true] {
        for affine in [false, true] {
            run(true, arc, affine, false);
        }
    }
}
#[test]
fn guided_spatial_two_cap_hole_clock_and_affine_matrix() {
    for arc in [false, true] {
        for affine in [false, true] {
            run(false, arc, affine, false);
        }
    }
}
#[test]
fn authored_spatial_nonuniform_rational_cap_with_affine_laws() {
    run(true, false, true, true);
}
#[test]
fn guided_spatial_nonuniform_rational_cap_with_affine_laws() {
    run(false, false, true, true);
}
fn run(authored: bool, arc: bool, affine: bool, rational: bool) {
    let (mut profiles, mut caps) = source();
    if rational {
        // Retain the complete natural outer material. Nonuniform interior
        // weights change the actual rational XY map and the nonplanar cap.
        // Hole coverage belongs to the eight separate paired-cap cases.
        profiles.truncate(5);
        for c in &mut caps {
            c.trims.truncate(1);
        }
        caps[1].surface.weights[2][2] *= 65. / 64.;
        assert_ne!(caps[1].surface.weights[2][2], caps[1].surface.weights[0][0]);
    }
    // The original control simplex has determinant -1/512, so this is not a
    // planar-path case. Its Z derivative is strictly positive throughout.
    let path = curve(
        &[
            [0., 0., 0.],
            [1. / 16., 0., 0.5],
            [0., 1. / 16., 1.5],
            [1. / 8., 1. / 16., 2.],
        ],
        &[1.; 4],
        [-5., 7.],
    );
    let axis = curve(
        &[[0., 0., 1.], [1. / 32., 1. / 64., 1.]],
        &[1.; 2],
        [11., 19.],
    );
    let normal = curve(&[[1., 0., 0.], [1., 1. / 32., 0.]], &[1.; 2], [-3., 13.]);
    let mut guide = path.clone();
    guide.knots = [vec![23.; 4], vec![35.; 4]].concat();
    for (i, p) in guide.control_points.iter_mut().enumerate() {
        p[0] += 1.;
        p[1] += i as f64 / 128.;
        p[2] += i as f64 / 256.;
    }
    let twist = curve(
        &[[0., 0., 0.], [1. / 128., 0., 0.], [1. / 64., 0., 0.]],
        &[1., 2., 1.],
        [17., 29.],
    );
    let axes = curve(
        &[
            [1., 1., 1.],
            [65. / 64., 63. / 64., 129. / 128.],
            [33. / 32., 65. / 64., 127. / 128.],
        ],
        &[1., 2., 1.],
        [3., 11.],
    );
    let center = curve(
        &[
            [0.; 3],
            [1. / 128., -1. / 128., 0.],
            [1. / 64., 0., 1. / 128.],
        ],
        &[1., 0.5, 2.],
        [-9., 7.],
    );
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let before_profiles = profiles.clone();
    let before_caps = caps
        .each_ref()
        .map(|c| (c.surface.clone(), c.trims.clone()));
    let before_laws = [
        path.clone(),
        axis.clone(),
        normal.clone(),
        guide.clone(),
        twist.clone(),
        axes.clone(),
        center.clone(),
    ];
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        Options {
            normal: [1., 0., 0.],
            orientation: if authored {
                Orientation::Fixed
            } else {
                Orientation::RotationMinimizing
            },
            spacing: if arc {
                Spacing::ArcLength {
                    tolerance: 0.001,
                    max_cells: 100000,
                }
            } else {
                Spacing::Parameter
            },
            initial_sections: 9,
            max_sections: 65,
            max_deviation: 0.05,
        },
    )
    .unwrap();
    let sweep = if authored {
        sweep.with_frame_laws(&axis, &normal)
    } else {
        sweep.with_orientation_guide(&guide)
    }
    .unwrap();
    let sweep = if affine {
        sweep.with_affine_laws(&axes, &center).unwrap()
    } else {
        sweep
    };
    let loops = if rational { vec![5] } else { vec![5, 5] };
    let classes = [
        OriginalProfileClass::G2,
        OriginalProfileClass::C0,
        OriginalProfileClass::C0,
        OriginalProfileClass::C0,
        OriginalProfileClass::C0,
    ]
    .repeat(loops.len());
    let projection = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    let certify = |l| {
        certify_original_classified_capped_material(
            &sweep,
            &loops,
            [&caps[0], &caps[1]],
            &classes,
            projection,
            1e-8,
            l,
        )
        .unwrap()
    };
    let body = match certify(limits()) {
        OriginalClassifiedCappedMaterialAttempt::Certified(body) => body,
        OriginalClassifiedCappedMaterialAttempt::Refused(r) => panic!(
            "spatial capped authored={authored} arc={arc} affine={affine} rational={rational}: {:?}, volume={:?}, work={:?}",
            r.reason, r.volume, r.work
        ),
    };
    assert_eq!(body.boundary_classes(), classes);
    assert_eq!(body.cap_wall_continuity(), "C0");
    for (i, bindings) in body.bindings().into_iter().enumerate() {
        assert!(
            std::ptr::eq(body.caps()[i], &caps[i])
                && std::ptr::eq(body.volume().caps()[i], &caps[i].surface)
        );
        assert_eq!(bindings.len(), profiles.len());
        assert!(
            bindings
                .iter()
                .all(|b| b.kind() == MaterialSpanBindingKind::NormalizedMap)
        );
        assert_eq!(
            body.material_reports()[i].trim.as_ref().unwrap().winding,
            if rational {
                vec![Some(1)]
            } else {
                vec![Some(1), Some(-1)]
            }
        );
    }
    let work = body.work();
    let exact = OriginalClassifiedCappedMaterialLimits {
        volume: OriginalClassifiedVolumeLimits {
            charts: OriginalSmoothVolumeLimits {
                cells: work.cells,
                exact_work: work.exact_work,
                products: work.products,
                ..limits().volume.charts
            },
            steps: body.volume().report().profiles.as_ref().unwrap().steps,
        },
        material: MaterialLimits {
            pairs: work.material_pairs,
            cells: work.material_cells,
            domain_cells: work.material_domain_cells,
        },
    };
    if let OriginalClassifiedCappedMaterialAttempt::Refused(r) = certify(exact) {
        panic!(
            "exact replay authored={authored} arc={arc} affine={affine} rational={rational}: baseline={work:?}; reason={:?} work={:?} material={:?} volume={:?}",
            r.reason,
            r.work,
            r.materials
                .iter()
                .map(|m| m
                    .trim
                    .as_ref()
                    .map(|t| (t.reason, t.cells, t.pairs, t.domain_cells)))
                .collect::<Vec<_>>(),
            r.volume
        );
    }
    let mut short = exact;
    short.volume.charts.exact_work -= 1;
    let OriginalClassifiedCappedMaterialAttempt::Refused(r) = certify(short) else {
        panic!("short spatial exact work admitted")
    };
    assert!(
        r.work.exact_work <= short.volume.charts.exact_work
            && r.work.cells <= short.volume.charts.cells
    );
    assert_eq!(profiles, before_profiles);
    for (i, c) in caps.iter().enumerate() {
        assert_eq!(
            (&c.surface, &c.trims),
            (&before_caps[i].0, &before_caps[i].1)
        );
    }
    assert_eq!(
        [&path, &axis, &normal, &guide, &twist, &axes, &center],
        before_laws.each_ref()
    );
    eprintln!(
        "spatial capped authored={authored} arc={arc} affine={affine} rational={rational}: {work:?}"
    );
}
