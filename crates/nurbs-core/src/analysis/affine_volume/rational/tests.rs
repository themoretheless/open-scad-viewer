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
        uv_window: [[0., 1.]; 2],
        carrier: &q,
        origin: [0.; 3],
        pieces: Motion::Rational(&p),
        poses: BTreeMap::new(),
        coordinates: None,
        separable: None,
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
        uv_window: [[0., 1.]; 2],
        carrier: &q,
        origin: [0.; 3],
        pieces: Motion::Rational(&p),
        poses: BTreeMap::new(),
        coordinates: Some(c),
        separable: None,
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

#[test]
fn correlated_original_translation_owns_global_volume_without_reference_body() {
    let q = carrier();
    let mut p = sweep();
    for pose in p.iter_mut().flat_map(|p| &mut p.controls) {
        pose[3] = [-0.875 * pose[0][0], -0.875 * pose[0][1], 0.25];
    }
    let saved = p.clone();
    let full =
        certify_rational_preconditioned(&q, [0.; 3], &p, true, projection(), 100000, 1000000)
            .unwrap();
    assert!(
        full.volume.embedding_certified
            && full.volume.regularity_certified
            && full.volume.parameter_topology_preserved,
        "{full:?}"
    );
    assert_eq!(
        full.coordinates
            .as_ref()
            .unwrap()
            .column_factor
            .as_ref()
            .unwrap()
            .dependent,
        3
    );
    for (cells, work) in [
        (0, 1000000),
        (full.volume.cells - 1, 1000000),
        (100000, 0),
        (100000, full.volume.exact_work - 1),
    ] {
        let r = certify_rational_preconditioned(&q, [0.; 3], &p, true, projection(), cells, work)
            .unwrap();
        assert!(!r.volume.embedding_certified);
        assert!(r.volume.cells <= cells && r.volume.exact_work <= work);
    }
    let twice = [p.clone(), p.clone()].concat();
    assert!(
        !certify_rational_preconditioned(&q, [0.; 3], &twice, true, projection(), 100000, 1000000)
            .unwrap()
            .volume
            .embedding_certified
    );
    let mut folded = p.clone();
    for pose in folded.iter_mut().flat_map(|p| &mut p.controls) {
        pose[3] = [-2. * pose[0][0], -2. * pose[0][1], 0.25];
    }
    assert!(
        !certify_rational_preconditioned(&q, [0.; 3], &folded, true, projection(), 100000, 1000000)
            .unwrap()
            .volume
            .embedding_certified
    );
    let mut changed = p.clone();
    changed[1].controls[1][3][0] = changed[1].controls[1][3][0].next_up();
    assert!(prove_factor(&changed, &mut 0, 1000000).unwrap().is_none());
    for (a, b) in p.iter().zip(saved) {
        assert_eq!(a.controls, b.controls);
        assert_eq!(a.weights, b.weights);
    }
}

#[test]
fn original_material_window_excludes_only_unused_carrier_axis_and_preserves_refusals() {
    let mut q = carrier();
    q.knots_u = vec![0.75, 0.75, 5.25, 5.25];
    q.knots_v = vec![-2.5, -2.5, 2.5, 2.5];
    q.control_points = vec![
        vec![vec![0.75, -2.5, 0.], vec![0.75, 2.5, 0.]],
        vec![vec![5.25, -2.5, 0.], vec![5.25, 2.5, 0.]],
    ];
    let mut p = sweep();
    for pose in p.iter_mut().flat_map(|p| &mut p.controls) {
        pose[3] = [-0.875 * pose[0][0], -0.875 * pose[0][1], 0.25];
    }
    let window = normalize_uv_window(&q, [[1., 5.], [-2., 2.]])
        .unwrap()
        .unwrap();
    assert!(window[0][0] > 0. && window[0][1] < 1.);
    let full = certify_rational_preconditioned_window(
        &q,
        [0.; 3],
        &p,
        true,
        projection(),
        window,
        100000,
        1000000,
    )
    .unwrap();
    assert!(
        full.volume.embedding_certified
            && full.volume.regularity_certified
            && full.volume.parameter_topology_preserved,
        "{full:?}"
    );
    assert!(
        !certify_rational_preconditioned(&q, [0.; 3], &p, true, projection(), 10000, 1000000)
            .unwrap()
            .volume
            .embedding_certified
    );
    for (cells, exact) in [
        (0, 1000000),
        (full.volume.cells - 1, 1000000),
        (100000, full.volume.exact_work - 1),
    ] {
        let short = certify_rational_preconditioned_window(
            &q,
            [0.; 3],
            &p,
            true,
            projection(),
            window,
            cells,
            exact,
        )
        .unwrap();
        assert!(!short.volume.embedding_certified);
    }
    let twice = [p.clone(), p.clone()].concat();
    assert!(
        !certify_rational_preconditioned_window(
            &q,
            [0.; 3],
            &twice,
            true,
            projection(),
            window,
            100000,
            1000000
        )
        .unwrap()
        .volume
        .embedding_certified
    );
    let mut folded = p.clone();
    for pose in folded.iter_mut().flat_map(|p| &mut p.controls) {
        pose[3] = [-2. * pose[0][0], -2. * pose[0][1], 0.25];
    }
    assert!(
        !certify_rational_preconditioned_window(
            &q,
            [0.; 3],
            &folded,
            true,
            projection(),
            window,
            10000,
            1000000
        )
        .unwrap()
        .volume
        .embedding_certified
    );
    assert!(
        normalize_uv_window(&q, [[0., 5.], [-2., 2.]])
            .unwrap()
            .is_none()
    );
    assert!(
        certify_rational_preconditioned_window(
            &q,
            [0.; 3],
            &p,
            true,
            projection(),
            [[-0.01, 1.], [0., 1.]],
            0,
            0
        )
        .is_err()
    );
}

#[test]
fn original_nonplanar_graph_has_global_proof_without_height_smallness_or_atlas_budget_reset() {
    let mut q = carrier();
    q.degree_u = 2;
    q.degree_v = 2;
    q.knots_u = vec![2., 2., 2., 7., 7., 7.];
    q.knots_v = vec![-3., -3., -3., 5., 5., 5.];
    q.control_points = (0..3)
        .map(|i| {
            (0..3)
                .map(|j| {
                    vec![
                        1. + 2. * i as f64 + j as f64,
                        -2. + 2. * j as f64,
                        if i == 1 && j == 1 { 1024. } else { 0. },
                    ]
                })
                .collect()
        })
        .collect();
    q.weights = vec![vec![1.; 3]; 3];
    let p = vec![line(0., 1., vec![1., 4.]), line(1., 3., vec![2., 1.])];
    let saved = q.clone();
    let full = certify_rational(&q, [0.; 3], &p, false, projection(), 2, 1000000).unwrap();
    assert!(
        full.embedding_certified && full.regularity_certified && full.parameter_topology_preserved,
        "{full:?}"
    );
    assert_eq!(
        full.reason,
        Some("original-nonplanar-graph-translation-certified")
    );
    assert_eq!(full.cells, 2);
    for (cells, work) in [
        (0, 1000000),
        (full.cells - 1, 1000000),
        (2, 0),
        (2, full.exact_work - 1),
    ] {
        let r = certify_rational(&q, [0.; 3], &p, false, projection(), cells, work).unwrap();
        assert!(
            !r.embedding_certified && r.cells <= cells && r.exact_work <= work,
            "{r:?}"
        );
    }
    let mut oblique = p.clone();
    let basis = [[1., 0.25, 0.], [0., 1., 0.25], [0.5, 0.25, 1.]];
    for pose in oblique.iter_mut().flat_map(|p| &mut p.controls) {
        let t = pose[3][2];
        for j in 0..3 {
            pose[j] = basis[j];
        }
        pose[3] = [0.5 * t, 0.25 * t, t];
    }
    let proof =
        certify_rational_preconditioned(&q, [0.; 3], &oblique, false, projection(), 2, 1000000)
            .unwrap();
    assert!(
        proof.volume.embedding_certified && proof.volume.regularity_certified,
        "{proof:?}"
    );
    assert_eq!(proof.volume.reason, full.reason);
    let mut singular = oblique.clone();
    for pose in singular.iter_mut().flat_map(|p| &mut p.controls) {
        pose[2] = pose[1];
    }
    assert!(
        !certify_rational(&q, [0.; 3], &singular, false, projection(), 2, 1000000)
            .unwrap()
            .embedding_certified
    );
    let polynomial = [std::array::from_fn(|i| {
        let mut pose = identity();
        pose[3][2] = i as f64;
        pose
    })];
    let poly =
        super::super::certify(&q, [0.; 3], &polynomial, false, projection(), 1, 1000000).unwrap();
    assert!(
        poly.embedding_certified && poly.regularity_certified,
        "{poly:?}"
    );
    assert_eq!(poly.reason, full.reason);
    let mut reversal = p.clone();
    reversal[1] = line(1., 0., vec![1., 1.]);
    assert!(
        !certify_rational(&q, [0.; 3], &reversal, false, projection(), 2, 1000000)
            .unwrap()
            .embedding_certified
    );
    let mut changed = q.clone();
    changed.control_points[1][1][0] = changed.control_points[1][1][0].next_up();
    assert!(
        !certify_rational(&changed, [0.; 3], &p, false, projection(), 2, 1000000)
            .unwrap()
            .embedding_certified
    );
    let mut collapsed = q.clone();
    for row in &mut collapsed.control_points {
        for point in row {
            point[1] = 0.;
        }
    }
    assert!(
        !certify_rational(&collapsed, [0.; 3], &p, false, projection(), 2, 1000000)
            .unwrap()
            .embedding_certified
    );
    let mut stopped = p.clone();
    stopped[1] = line(1., 1., vec![1., 1.]);
    assert!(
        !certify_rational(&q, [0.; 3], &stopped, false, projection(), 2, 1000000)
            .unwrap()
            .regularity_certified
    );
    assert_eq!(q.control_points, saved.control_points);
}

#[path = "../../../../tests/support/moving_nonplanar_carrier.rs"]
mod moving_nonplanar_fixture;
#[test]
fn nonplanar_moving_matrix_closed_time_uses_exact_paired_source_shear() {
    let (q, p) = moving_nonplanar_fixture::fixture();
    let saved = (q.clone(), p.clone());
    let full =
        certify_rational_preconditioned(&q, [0.; 3], &p, true, projection(), 100000, 1000000)
            .unwrap();
    assert!(
        full.volume.embedding_certified
            && full.volume.regularity_certified
            && full.volume.parameter_topology_preserved,
        "{full:?}"
    );
    let shear = full.source_shear.as_ref().unwrap();
    assert_eq!((shear.base, shear.dependent, shear.scale), (0, 2, 8.));
    for (cells, work) in [
        (0, 1000000),
        (full.volume.cells - 1, 1000000),
        (100000, 0),
        (100000, full.volume.exact_work - 1),
    ] {
        let short =
            certify_rational_preconditioned(&q, [0.; 3], &p, true, projection(), cells, work)
                .unwrap();
        assert!(
            !short.volume.embedding_certified
                && short.volume.cells <= cells
                && short.volume.exact_work <= work,
            "{short:?}"
        );
    }
    let mut weighted = q.clone();
    weighted.weights = vec![vec![1., 2.], vec![2., 4.], vec![1., 2.]];
    let rational_carrier = certify_rational_preconditioned(
        &weighted,
        [0.; 3],
        &p,
        true,
        projection(),
        100000,
        1000000,
    )
    .unwrap();
    assert!(
        rational_carrier.volume.embedding_certified && rational_carrier.volume.regularity_certified,
        "{rational_carrier:?}"
    );
    assert!(rational_carrier.source_shear.is_some());
    let quadrant = [
        [1., 0.],
        [1., 0.25],
        [0.75, 0.5],
        [0.5, 0.75],
        [0.25, 1.],
        [0., 1.],
    ];
    let polynomial = (0..4)
        .map(|branch| {
            std::array::from_fn(|i| {
                let [mut x, mut y] = quadrant[i];
                for _ in 0..branch {
                    (x, y) = (-y, x);
                }
                [
                    [x, y, 0.],
                    [0., 0., 1.],
                    [8. * x - y / 65536., 8. * y + x / 65536., 0.],
                    [0.; 3],
                ]
            })
        })
        .collect::<Vec<[Pose; 6]>>();
    let quintic = super::super::certify_preconditioned(
        &q,
        [0.; 3],
        &polynomial,
        true,
        projection(),
        100000,
        1000000,
    )
    .unwrap();
    assert!(
        quintic.volume.embedding_certified
            && quintic.volume.regularity_certified
            && quintic.volume.parameter_topology_preserved,
        "{quintic:?}"
    );
    assert!(quintic.source_shear.is_some());
    let twice = [p.clone(), p.clone()].concat();
    assert!(
        !certify_rational_preconditioned(&q, [0.; 3], &twice, true, projection(), 10000, 1000000)
            .unwrap()
            .volume
            .embedding_certified
    );
    let mut gap = p.clone();
    gap[1].controls[0][3][0] = f64::from_bits(1);
    assert_eq!(
        certify_rational_preconditioned(&q, [0.; 3], &gap, true, projection(), 100000, 1000000)
            .unwrap()
            .volume
            .reason,
        Some("affine-volume-station-identity-unproved")
    );
    let mut fold = q.clone();
    for row in &mut fold.control_points {
        for v in row {
            v[0] -= 3.;
        }
    }
    assert!(
        !certify_rational_preconditioned(&fold, [0.; 3], &p, true, projection(), 10000, 1000000)
            .unwrap()
            .volume
            .embedding_certified
    );
    assert_eq!(q, saved.0);
    for (a, b) in p.iter().zip(saved.1) {
        assert_eq!(a.controls, b.controls);
        assert_eq!(a.weights, b.weights);
    }
}
