use super::*;
fn carrier() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 7., 7.],
        knots_v: vec![-3., -3., 5., 5.],
        control_points: vec![
            vec![vec![1., -2., 0.], vec![1., 2., 0.]],
            vec![vec![5., -2., 0.], vec![5., 2., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn identity() -> Pose {
    [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.], [0.; 3]]
}
fn projection() -> [[f64; 3]; 3] {
    [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]
}
fn line(a: f64, b: f64, weights: Vec<f64>) -> RationalPiece {
    let mut p = identity();
    p[3][2] = a;
    let mut q = identity();
    q[3][2] = b;
    RationalPiece {
        controls: vec![p, q],
        weights,
    }
}
fn sweep() -> Vec<RationalPiece> {
    let p = [
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
    (0..4)
        .map(|i| RationalPiece {
            weights: vec![1., 0.5, 1.],
            controls: p[2 * i..=2 * i + 2]
                .iter()
                .map(|p| [[p[0], p[1], 0.], [0., 0., 1.], [0.; 3], [0.; 3]])
                .collect(),
        })
        .collect()
}
#[test]
fn positive_rational_source_volume_and_original_derivatives_survive_all_short_budgets() {
    let q = carrier();
    let p = vec![line(0., 2., vec![1., 4.])];
    let saved = p[0].clone();
    let full = certify_rational(&q, [0.; 3], &p, false, projection(), 100000, 1000000).unwrap();
    assert!(
        full.embedding_certified && full.regularity_certified && full.parameter_topology_preserved,
        "{full:?}"
    );
    for cells in [0, full.cells - 1] {
        let short = certify_rational(&q, [0.; 3], &p, false, projection(), cells, 1000000).unwrap();
        assert!(!short.embedding_certified && short.cells <= cells);
    }
    let j = pose_on(&p[0], I::point(0.), 1).unwrap();
    assert!(j.origin[1][2].lo <= 8. && j.origin[1][2].hi >= 8.);
    let j = pose_on(&p[0], I::point(1.), 1).unwrap();
    assert!(j.origin[1][2].lo <= 0.5 && j.origin[1][2].hi >= 0.5);
    assert_eq!(p[0].controls, saved.controls);
    assert_eq!(p[0].weights, saved.weights);
}
#[test]
fn exact_rational_azimuth_owns_an_annular_body_volume_and_rejects_double_cover() {
    let q = carrier();
    let p = sweep();
    let full = certify_rational(&q, [0.; 3], &p, true, projection(), 1000000, 1000000).unwrap();
    eprintln!("exact rational annular volume: {full:?}");
    assert!(
        full.embedding_certified && full.regularity_certified && full.parameter_topology_preserved
    );
    let sign = jacobian_orientation_rational(&q, [0.; 3], &p, [0.5; 3], 100000).unwrap();
    assert_eq!(sign.sign, Some(-1));
    for (cells, exact) in [
        (0, 1000000),
        (full.cells - 1, 1000000),
        (1000000, 0),
        (1000000, full.exact_work - 1),
    ] {
        let short = certify_rational(&q, [0.; 3], &p, true, projection(), cells, exact).unwrap();
        assert!(!short.embedding_certified && !short.parameter_topology_preserved);
        assert!(short.cells <= cells && short.exact_work <= exact);
    }
    let repeated = [p.clone(), p.clone()].concat();
    let bad =
        certify_rational(&q, [0.; 3], &repeated, true, projection(), 100000, 1000000).unwrap();
    assert!(!bad.embedding_certified && !bad.parameter_topology_preserved);
    let mut gap = p.clone();
    gap[1].controls[0][3][0] = 1e-12;
    let bad = certify_rational(&q, [0.; 3], &gap, true, projection(), 1000000, 1000000).unwrap();
    assert_eq!(bad.reason, Some("affine-volume-station-identity-unproved"));
}
#[test]
fn nonbinary_time_knots_enclose_both_original_rational_one_sided_jets() {
    let p = vec![
        line(0., 1., vec![1., 1.]),
        line(1., 2., vec![1., 4.]),
        line(2., 3., vec![1., 1.]),
    ];
    let q = carrier();
    let mut c = Context {
        carrier: &q,
        origin: [0.; 3],
        pieces: Motion::Rational(&p),
        poses: BTreeMap::new(),
        coordinates: None,
    };
    let mut out = certify_rational(&q, [0.; 3], &p, false, projection(), 0, 0).unwrap();
    let t = 1. / 3.;
    let j = c.temporal([t, t], &mut out, 100000).unwrap().unwrap();
    assert!(j.origin[1][2].lo <= 3. && j.origin[1][2].hi >= 12.);
}
#[test]
fn rational_geometry_validation_precedes_zero_budget_refusals() {
    let q = carrier();
    let mut p = sweep();
    p[3].weights[1] = 0.;
    assert!(certify_rational(&q, [0.; 3], &p, true, projection(), 0, 0).is_err());
    let mut flat = q.clone();
    for p in flat.control_points.iter_mut().flatten() {
        p.pop();
    }
    assert!(certify_rational(&flat, [0.; 3], &sweep(), true, projection(), 0, 0).is_err());
    p = sweep();
    p[2].controls[1][0][0] = f64::NAN;
    assert!(certify_rational(&q, [0.; 3], &p, true, projection(), 0, 0).is_err());
    p = sweep();
    p[2].weights.pop();
    assert!(certify_rational(&q, [0.; 3], &p, true, projection(), 0, 0).is_err());
}

#[test]
fn rational_degree_eight_preserves_original_endpoint_derivatives_and_refuses_degree_nine() {
    let mut piece = RationalPiece {
        controls: (0..=8)
            .map(|i| {
                let mut p = identity();
                p[3][2] = i as f64;
                p
            })
            .collect(),
        weights: (1..=9).map(|i| i as f64).collect(),
    };
    let a = pose_on(&piece, I::point(0.), 1).unwrap().origin[1][2];
    let b = pose_on(&piece, I::point(1.), 1).unwrap().origin[1][2];
    assert!(a.lo <= 16. && a.hi >= 16.);
    let end = 64. / 9.;
    assert!(b.lo <= end && b.hi >= end);
    let report = certify_rational(
        &carrier(),
        [0.; 3],
        &[piece.clone()],
        false,
        projection(),
        100000,
        1000000,
    )
    .unwrap();
    assert!(
        report.embedding_certified && report.regularity_certified,
        "{report:?}"
    );
    piece.controls.push(identity());
    piece.weights.push(1.);
    assert!(certify_rational(&carrier(), [0.; 3], &[piece], false, projection(), 0, 0).is_err());
}

#[test]
fn fixed_original_coordinate_change_has_no_point_or_budget_admission_shortcut() {
    let q = carrier();
    let p = sweep();
    let saved = p.clone();
    let full =
        certify_rational_preconditioned(&q, [0.; 3], &p, true, projection(), 1000000, 1000000)
            .unwrap();
    assert!(
        full.volume.embedding_certified && full.volume.parameter_topology_preserved,
        "{full:?}"
    );
    let c = full.coordinates.as_ref().unwrap();
    assert!(c.determinant[0] > 0. || c.determinant[1] < 0.);
    for (cells, exact) in [
        (0, 1000000),
        (1, 1000000),
        (full.volume.cells - 1, 1000000),
        (1000000, full.volume.exact_work - 1),
    ] {
        let short =
            certify_rational_preconditioned(&q, [0.; 3], &p, true, projection(), cells, exact)
                .unwrap();
        assert!(
            !short.volume.embedding_certified
                && !short.volume.regularity_certified
                && !short.volume.parameter_topology_preserved
        );
        assert!(short.volume.cells <= cells && short.volume.exact_work <= exact);
    }
    let twice = [p.clone(), p.clone()].concat();
    let bad =
        certify_rational_preconditioned(&q, [0.; 3], &twice, true, projection(), 50000, 1000000)
            .unwrap();
    assert!(!bad.volume.embedding_certified && !bad.volume.parameter_topology_preserved);
    for (a, b) in p.iter().zip(saved) {
        assert_eq!(a.controls, b.controls);
        assert_eq!(a.weights, b.weights);
    }
}

#[test]
fn projected_original_coefficient_jets_enclose_translation_and_rational_derivatives() {
    let p = line(0., 2., vec![1., 4.]);
    let c = ProofCoordinates {
        rows: [[1., 0.125, 0.25], [-0.25, 1., 0.125], [0.125, -0.25, 1.]],
        offset: [1., 2., 3.],
        determinant: [1., 2.],
        column_factor: None,
    };
    for t in [0., 0.25, 0.5, 1.] {
        let e = pose_in_coordinates(&p, I::point(t), 1, &c).unwrap();
        // Literal source Tz=8t/(1+3t), derivative 8/(1+3t)^2.
        let z = 8. * t / (1. + 3. * t);
        let dz = 8. / (1. + 3. * t).powi(2);
        for k in 0..3 {
            let value = -c.rows[k][0] - 2. * c.rows[k][1] + c.rows[k][2] * (z - 3.);
            let derivative = c.rows[k][2] * dz;
            assert!(e.origin[0][k].lo <= value && e.origin[0][k].hi >= value);
            assert!(e.origin[1][k].lo <= derivative && e.origin[1][k].hi >= derivative);
        }
    }
}

#[test]
fn moving_affine_original_source_preconditioning_preserves_regular_thin_time_boxes() {
    let mut q = carrier();
    q.control_points = vec![
        vec![vec![0.75, -2.5, 0.], vec![0.75, 2.5, 0.]],
        vec![vec![5.25, -2.5, 0.], vec![5.25, 2.5, 0.]],
    ];
    let mut p = sweep();
    let basis = [[1., 0.125, 0.25], [0., 1., 0.125], [0.125, 0., 1.]];
    for piece in &mut p {
        for pose in &mut piece.controls {
            pose[1] = [pose[0][0] / 8., pose[0][1] / 8., 1.];
            for col in 0..3 {
                let a = pose[col];
                pose[col] = std::array::from_fn(|k| {
                    basis[0][k] * a[0] + basis[1][k] * a[1] + basis[2][k] * a[2]
                });
            }
            pose[3] = [1., 2., 3.];
        }
    }
    for w in &mut p[1].weights {
        *w *= 2.;
    }
    let report =
        certify_rational_preconditioned(&q, [0.; 3], &p, true, projection(), 10, 1000000).unwrap();
    let c = report.coordinates.unwrap();
    eprintln!("original affine coordinate proposal {c:?}");
    let global =
        certify_rational_preconditioned(&q, [0.; 3], &p, true, projection(), 50000, 1000000)
            .unwrap();
    assert!(
        global.volume.embedding_certified
            && global.volume.regularity_certified
            && global.volume.parameter_topology_preserved,
        "{global:?}"
    );
    let factor = global
        .coordinates
        .as_ref()
        .unwrap()
        .column_factor
        .as_ref()
        .unwrap();
    assert_eq!((factor.base, factor.dependent, factor.scale), (0, 1, 0.125));
    eprintln!("original moving affine global proof {global:?}");
    let mut context = Context {
        carrier: &q,
        origin: [0.; 3],
        pieces: Motion::Rational(&p),
        poses: BTreeMap::new(),
        coordinates: Some(c),
    };
    let mut work = certify_rational(&q, [0.; 3], &p, true, projection(), 0, 0).unwrap();
    for i in 0..128 {
        let Enclosed::Value(e) = context
            .enclose(
                [[0., 1.], [0., 1.], [i as f64 / 128., (i + 1) as f64 / 128.]],
                &mut work,
                100000,
            )
            .unwrap()
        else {
            panic!("no source jets {i}");
        };
        let det =
            iv::dot_tight(e.columns[0], iv::cross(e.columns[1], e.columns[2]).unwrap()).unwrap();
        assert!(
            det.lo > 0. || det.hi < 0.,
            "original thin time box {i} has ambiguous determinant {det:?}, columns {:?}",
            e.columns
        );
    }
}
