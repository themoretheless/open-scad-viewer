use nurbs_core::{
    curve::Curve,
    progressive_sweep::{MultiSweep, Options, Orientation, Spacing, constant_vector_law},
    surface::Surface,
};
fn cap() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 1. / 64.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn options() -> Options {
    Options {
        normal: [1., 0., 0.],
        orientation: Orientation::Fixed,
        spacing: Spacing::Parameter,
        initial_sections: 5,
        max_sections: 5,
        max_deviation: 0.01,
    }
}
fn projection() -> [[f64; 3]; 3] {
    [[1., 0., 0.], [0., 1., 0.], [0., 0., 1. / 64.]]
}
#[test]
fn moving_original_frame_filled_nonplanar_carriers_have_global_embedding_and_shared_limit() {
    let c = cap();
    let original = c.clone();
    let path = nurbs_core::primitives::line([0.; 3], [0., 0., 64.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let axis = constant_vector_law([0., 0., 1.]).unwrap();
    let normal = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![1., 0., 0.], vec![1., 0.125, 0.], vec![1., 0., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let profiles = [nurbs_core::primitives::line([0.; 3], [1., 0., 0.]).unwrap()];
    let sweep = MultiSweep::new(&profiles, &path, &scale, &twist, options())
        .unwrap()
        .with_frame_laws(&axis, &normal)
        .unwrap();
    let audit = |cells| {
        sweep
            .certify_ideal_volume([&c, &c], projection(), cells, 1000000)
            .unwrap()
    };
    let full = audit(100000);
    assert!(full.embedding_certified, "{full:?}");
    assert!(full.regularity_certified && full.parameter_topology_preserved);
    assert!(full.monotonicity_lower.unwrap() > 0.);
    // Independent analytic oracle for this authored rational-frame law. This
    // grid checks the certificate's units and initial-pose mapping; samples
    // never supply admission or replace the whole-domain native proof.
    let image = |p: [f64; 3]| {
        let [u, v, t] = p;
        let b = 0.25 * t * (1. - t);
        let n = (1. + b * b).sqrt();
        [(u - b * v) / n, (b * u + v) / n, 64. * t + u * v / 64.]
    };
    let mut points = Vec::new();
    for u in [0., 0.25, 0.5, 0.75, 1.] {
        for v in [0., 0.25, 0.5, 0.75, 1.] {
            for t in [0., 0.25, 0.5, 0.75, 1.] {
                points.push([u, v, t]);
            }
        }
    }
    for (i, a) in points.iter().enumerate() {
        for b in &points[..i] {
            let fa = image(*a);
            let fb = image(*b);
            let mut left = 0.;
            let mut right = 0.;
            for k in 0..3 {
                let d = a[k] - b[k];
                left += (fa[k] - fb[k]) * if k == 2 { 1. / 64. } else { 1. } * d;
                right += d * d;
            }
            assert!(left + 1e-12 >= full.monotonicity_lower.unwrap() * right);
        }
    }

    assert!(full.cells <= 100000);
    assert!(!audit(0).embedding_certified);
    assert!(!audit(full.cells - 1).embedding_certified);
    assert!(audit(full.cells).embedding_certified);
    assert_eq!(c, original);
    // A singular projection cannot be upgraded by choosing another one per cell.
    let mut p = projection();
    p[2] = [0.; 3];
    assert!(
        !sweep
            .certify_ideal_volume([&c, &c], p, 200, 1000000)
            .unwrap()
            .embedding_certified
    );
}
#[test]
fn original_rational_weights_independent_domains_affine_laws_and_distinct_caps() {
    let a = cap();
    let mut b = a.clone();
    b.weights = vec![vec![1., 1.125], vec![1.125, 1.25]];
    for row in &mut b.control_points {
        for p in row {
            p[2] += 1. / 128.;
        }
    }
    // Same normalized chart, independently authored knot domains.
    b.knots_u = vec![-3., -3., 5., 5.];
    b.knots_v = vec![2., 2., 4., 4.];
    let path = nurbs_core::primitives::line([0.; 3], [0., 0., 64.]).unwrap();
    let profiles = [nurbs_core::primitives::line([0.; 3], [1., 0., 0.]).unwrap()];
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let axes = constant_vector_law([1., 1., 1.]).unwrap();
    let center = nurbs_core::primitives::line([0.; 3], [0.125, 0.125, 0.]).unwrap();
    let sweep = MultiSweep::new(&profiles, &path, &scale, &twist, options())
        .unwrap()
        .with_affine_laws(&axes, &center)
        .unwrap();
    let r = sweep
        .certify_ideal_volume([&a, &b], projection(), 100000, 1000000)
        .unwrap();
    assert!(r.embedding_certified, "{r:?}");
}
#[test]
fn a_folded_original_cap_or_discontinuous_law_cannot_receive_a_global_claim() {
    let mut c = cap();
    c.control_points[1] = c.control_points[0].clone();
    let path = nurbs_core::primitives::line([0.; 3], [0., 0., 64.]).unwrap();
    let profiles = [nurbs_core::primitives::line([0.; 3], [1., 0., 0.]).unwrap()];
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let s = MultiSweep::new(&profiles, &path, &scale, &twist, options()).unwrap();
    let r = s
        .certify_ideal_volume([&c, &c], projection(), 400, 1000000)
        .unwrap();
    assert!(
        !r.embedding_certified
            && !r.regularity_certified
            && !r.parameter_topology_preserved
            && r.monotonicity_lower.is_none()
    );
    let jump = Curve {
        degree: 1,
        knots: vec![0., 0., 0.5, 0.5, 1., 1.],
        control_points: vec![vec![0.; 3], vec![0.; 3], vec![1., 0., 0.], vec![1., 0., 0.]],
        weights: vec![1.; 4],
        periodic: false,
    };
    // The source constructor itself already rejects disconnected laws.
    assert!(MultiSweep::new(&profiles, &path, &scale, &jump, options()).is_err());
}
#[test]
fn guided_and_contact_source_maps_use_owned_original_anchor_and_fit() {
    let c = cap();
    let path = nurbs_core::primitives::line([0.; 3], [0., 0., 64.]).unwrap();
    let guide = nurbs_core::primitives::line([1., 0., 0.], [2., 0., 64.]).unwrap();
    let profiles = [nurbs_core::primitives::line([0.; 3], [1., 0., 0.]).unwrap()];
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let base = || {
        MultiSweep::new(
            &profiles,
            &path,
            &scale,
            &twist,
            Options {
                orientation: Orientation::RotationMinimizing,
                ..options()
            },
        )
        .unwrap()
    };
    let guided = base().with_orientation_guide(&guide).unwrap();
    let r = guided
        .certify_ideal_volume([&c, &c], projection(), 100000, 1000000)
        .unwrap();
    assert!(r.embedding_certified, "{r:?}");
    let contact = base().with_contact_guide(&guide, 0, 1.).unwrap();
    let r = contact
        .certify_ideal_volume([&c, &c], projection(), 100000, 1000000)
        .unwrap();
    assert!(r.embedding_certified, "{r:?}");
    assert!(
        !contact
            .certify_ideal_volume([&c, &c], projection(), r.cells - 1, 1000000)
            .unwrap()
            .embedding_certified
    );
}

#[test]
fn open_spatial_bishop_source_is_c1_c2_without_a_planar_or_curvature_premise() {
    use nurbs_core::progressive_sweep::Sweep;
    let profile = nurbs_core::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    let path = nurbs_core::paths::bezier(
        vec![
            vec![0., 0., 0.],
            vec![0.25, 0., 0.5],
            vec![0., 0.25, 1.],
            vec![0.125, 0.125, 2.],
        ],
        Some(vec![1., 1.125, 1.25, 1.]),
    )
    .unwrap();
    let before = path.clone();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = nurbs_core::primitives::line([0.; 3], [0.125, 0., 0.]).unwrap();
    for spacing in [
        Spacing::Parameter,
        Spacing::ArcLength {
            tolerance: 0.001,
            max_cells: 100000,
        },
    ] {
        let options = Options {
            orientation: Orientation::RotationMinimizing,
            spacing,
            ..options()
        };
        let sweep = Sweep::new(&profile, &path, &scale, &twist, options)
            .unwrap()
            .with_spatial_rmf_error_limits(64, 100000, 1000000)
            .unwrap();
        for order in [1, 2] {
            let p = sweep
                .certify_original_frame_smoothness(order, 100000, 1000000)
                .unwrap();
            assert!(
                p.source_frame_smoothness_certified,
                "{spacing:?}/{order}: {p:?}"
            );
            assert!(
                !sweep
                    .certify_original_frame_smoothness(order, p.cells - 1, 1000000)
                    .unwrap()
                    .source_frame_smoothness_certified
            );
            assert!(
                !sweep
                    .certify_original_frame_smoothness(order, 0, 1000000)
                    .unwrap()
                    .source_frame_smoothness_certified
            );
        }
    }
    assert_eq!(path, before);
}
