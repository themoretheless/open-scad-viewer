use super::*;
fn ledger() -> IdealVolumeReport {
    IdealVolumeReport {
        embedding_certified: false,
        regularity_certified: false,
        parameter_topology_preserved: false,
        cells: 0,
        exact_work: 0,
        boxes: 0,
        monotonicity_lower: None,
        projection: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        reason: None,
    }
}
fn caps(r: f64, weighted: bool) -> [Surface; 2] {
    let make = |u: [f64; 2], v: [f64; 2]| Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: [vec![u[0]; 3], vec![u[1]; 3]].concat(),
        knots_v: [vec![v[0]; 3], vec![v[1]; 3]].concat(),
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| vec![(i as f64 - 1.) * r, (j as f64 - 1.) * r, 0.])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let a = make([-3., 5.], [7., 11.]);
    let mut b = make([17., 29.], [-7., 13.]);
    b.control_points[1][1][0] += r / 8.;
    if weighted {
        let u = [1., 1.25, 2.];
        let v = [1., 1.5, 1.25];
        b.weights = (0..3)
            .map(|i| (0..3).map(|j| u[i] * v[j]).collect())
            .collect();
    }
    [a, b]
}
fn options() -> Options {
    Options {
        normal: [1., 0., 0.],
        orientation: Orientation::Fixed,
        spacing: Spacing::ArcLength {
            tolerance: 0.001,
            max_cells: 100000,
        },
        initial_sections: 3,
        max_sections: 9,
        max_deviation: 0.05,
    }
}
#[test]
fn common_carrier_metric_owns_rational_weights_normalized_domains_and_work() {
    let caps = caps(16., true);
    let before = caps.clone();
    let mut out = ledger();
    assert!(carrier::certify_pair([&caps[0], &caps[1]], &mut out, 10000).unwrap());
    assert!(out.cells > 0 && out.cells < 10000);
    let mut exact = ledger();
    assert!(carrier::certify_pair([&caps[0], &caps[1]], &mut exact, out.cells).unwrap());
    assert_eq!(exact.cells, out.cells);
    for max in [0, out.cells - 1] {
        let mut short = ledger();
        assert!(!carrier::certify_pair([&caps[0], &caps[1]], &mut short, max).unwrap());
        assert!(short.cells <= max);
    }
    assert_eq!(caps, before);
    eprintln!(
        "one common original rational carrier family metric: {} cells",
        out.cells
    );
}
#[test]
fn interpolated_rational_carriers_and_large_affine_transverse_motion_keep_global_topology() {
    let caps = caps(16., true);
    let before = caps.clone();
    let path = crate::primitives::line([0.; 3], [0., 0., 2.]).unwrap();
    let profiles = [crate::primitives::line([-16., -16., 0.], [16., -16., 0.]).unwrap()];
    let scale = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    let axes = constant_vector_law([3., 4., 5.]).unwrap();
    let mut center = crate::primitives::line([6., 8., 10.], [6., 8., 11.]).unwrap();
    center.knots = vec![17., 17., 29., 29.];
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = MultiSweep::new(&profiles, &path, &scale, &twist, options())
        .unwrap()
        .with_affine_laws(&axes, &center)
        .unwrap();
    let audit = |max| {
        sweep
            .certify_ideal_volume_atlas(
                [&caps[0], &caps[1]],
                [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                max,
                1000000,
            )
            .unwrap()
    };
    let full = audit(10000);
    assert!(
        full.embedding_certified && full.regularity_certified && full.parameter_topology_preserved,
        "{full:?}"
    );
    assert!(full.cells <= 10000);
    assert!(audit(full.cells).embedding_certified);
    for max in [0, full.cells - 1] {
        let short = audit(max);
        assert!(
            !short.embedding_certified
                && !short.regularity_certified
                && !short.parameter_topology_preserved,
            "{short:?}"
        );
        assert!(short.cells <= max);
    }
    let fixed_metric = sweep
        .certify_ideal_volume(
            [&caps[0], &caps[1]],
            [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            1000,
            1000000,
        )
        .unwrap();
    assert!(!fixed_metric.embedding_certified);
    assert_eq!(caps, before);
    eprintln!("interpolated rational source with affine scales/center: {full:?}");
}
#[test]
fn separately_regular_endpoint_carriers_cannot_hide_an_interior_rank_loss() {
    let mut caps = caps(1., false);
    caps[1].control_points = caps[0]
        .control_points
        .iter()
        .map(|row| row.iter().map(|p| vec![-p[0], p[1], p[2]]).collect())
        .collect();
    for cap in &caps {
        assert!(
            crate::surface_injectivity::certify_contraction(cap, 1000)
                .unwrap()
                .proven
        );
    }
    let mut work = ledger();
    assert!(!carrier::certify_pair([&caps[0], &caps[1]], &mut work, 10000).unwrap());
    // Two distinct normalized UV points coincide in the middle carrier.
    // This independent source evaluation witnesses the lost U direction.
    let point = |u: f64| {
        let mut p = [0.; 3];
        for cap in &caps {
            let du = [cap.knots_u[2], cap.knots_u[3]];
            let dv = [cap.knots_v[2], cap.knots_v[3]];
            let q = cap
                .evaluate(du[0] + u * (du[1] - du[0]), (dv[0] + dv[1]) / 2.)
                .unwrap()
                .point;
            for k in 0..3 {
                p[k] += 0.5 * q[k];
            }
        }
        p
    };
    assert_eq!(point(0.25), point(0.75));
    let path = crate::primitives::line([0.; 3], [0., 0., 2.]).unwrap();
    let profiles = [crate::primitives::line([-1., -1., 0.], [1., -1., 0.]).unwrap()];
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let sweep = MultiSweep::new(&profiles, &path, &scale, &twist, options()).unwrap();
    let result = sweep
        .certify_ideal_volume_atlas(
            [&caps[0], &caps[1]],
            [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            3000,
            1000000,
        )
        .unwrap();
    assert!(
        !result.embedding_certified
            && !result.regularity_certified
            && !result.parameter_topology_preserved,
        "{result:?}"
    );
    assert!(result.cells <= 3000);
}
