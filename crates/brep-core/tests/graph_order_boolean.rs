use brep_core::{Model, minkowski3d::curved, operations};
use nurbs_core::ss_intersection as ss;
fn solid(upper: bool, shift: f64) -> Model {
    let mut m = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    let point = |p: &[f64]| {
        let (x, y, z) = (p[0], p[1], p[2]);
        let h = x * x / 8.;
        let gap = (x - 0.5).powi(2) * (y - 0.5).powi(2) + shift;
        vec![
            x,
            y,
            if upper {
                h + gap + z * (2. - h - gap)
            } else {
                -1. + z * (1. + h)
            },
        ]
    };
    for v in &mut m.vertices {
        v.point = point(&v.point).try_into().unwrap();
    }
    for e in &mut m.edges {
        let p = e.curve.control_points[0].clone();
        let q = e.curve.control_points.last().unwrap().clone();
        let a = point(&p);
        let b = point(&q);
        let mid = point(&(0..3).map(|k| (p[k] + q[k]) * 0.5).collect::<Vec<_>>());
        e.curve.degree = 2;
        e.curve.knots = vec![0., 0., 0., 1., 1., 1.];
        e.curve.weights = vec![1.; 3];
        e.curve.control_points = vec![
            a.clone(),
            (0..3).map(|k| 2. * mid[k] - (a[k] + b[k]) * 0.5).collect(),
            b,
        ];
    }
    for f in &mut m.faces {
        let old = f.surface.clone();
        let o = &old.control_points[0][0];
        let u = (0..3)
            .map(|k| old.control_points[1][0][k] - o[k])
            .collect::<Vec<_>>();
        let v = (0..3)
            .map(|k| old.control_points[0][1][k] - o[k])
            .collect::<Vec<_>>();
        let mut net = (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        point(
                            &(0..3)
                                .map(|k| o[k] + u[k] * (i as f64 / 2.) + v[k] * (j as f64 / 2.))
                                .collect::<Vec<_>>(),
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        for j in 0..3 {
            for k in 0..3 {
                net[1][j][k] = 2. * net[1][j][k] - (net[0][j][k] + net[2][j][k]) * 0.5;
            }
        }
        for i in 0..3 {
            for k in 0..3 {
                net[i][1][k] = 2. * net[i][1][k] - (net[i][0][k] + net[i][2][k]) * 0.5;
            }
        }
        f.surface.degree_u = 2;
        f.surface.degree_v = 2;
        f.surface.knots_u = vec![0., 0., 0., 1., 1., 1.];
        f.surface.knots_v = f.surface.knots_u.clone();
        f.surface.control_points = net;
        f.surface.weights = vec![vec![1.; 3]; 3];
    }
    m.rebuild_topology_ids();
    m.validate().unwrap();
    m
}
fn faces(a: &Model, b: &Model) -> [usize; 2] {
    for i in 0..a.faces.len() {
        for j in 0..b.faces.len() {
            if curved::certify_graph_order_boolean(
                a,
                b,
                "intersection",
                [i, j],
                0,
                ss::GraphOrderLimits::default(),
            )
            .is_ok()
            {
                return [i, j];
            }
        }
    }
    panic!("No proved material graph barriers")
}
#[test]
fn singular_crossing_contact_has_exact_regularized_material_booleans() {
    let a = solid(false, 0.);
    let b = solid(true, 0.);
    let f = faces(&a, &b);
    for op in ["intersection", "difference", "union", "xor"] {
        let r =
            curved::certify_graph_order_boolean(&a, &b, op, f, 0, ss::GraphOrderLimits::default())
                .unwrap();
        assert!(r.certificate().square_identity);
        curved::verify_graph_order_boolean(&a, &b, r.model(), r.certificate()).unwrap();
        match op {
            "intersection" => assert!(r.model().is_empty()),
            "difference" => assert_eq!(r.model(), &a),
            _ => assert_eq!(r.model().bodies.len(), 2),
        }
        let live = operations::boolean(&a, &b, op).unwrap();
        assert_eq!(live.0, r.model().0);
    }
}
#[test]
fn positive_material_overlap_and_receipt_damage_refuse_the_separator() {
    let a = solid(false, 0.);
    let b = solid(true, 0.);
    let f = faces(&a, &b);
    let r = curved::certify_graph_order_boolean(
        &a,
        &b,
        "intersection",
        f,
        0,
        ss::GraphOrderLimits::default(),
    )
    .unwrap();
    let mut bad = r.certificate().clone();
    bad.square_identity = false;
    assert!(curved::verify_graph_order_boolean(&a, &b, r.model(), &bad).is_err());
    assert!(
        curved::certify_graph_order_boolean(
            &a,
            &solid(true, -1. / 1024.),
            "intersection",
            f,
            0,
            ss::GraphOrderLimits::default()
        )
        .is_err()
    );
    let mut shifted = a.clone();
    shifted.vertices[0].point[2] += 1. / 1024.;
    shifted.rebuild_topology_ids();
    assert!(curved::verify_graph_order_boolean(&shifted, &b, r.model(), r.certificate()).is_err());
}

#[test]
fn reversed_material_order_preserves_difference_and_intersection() {
    let lower = solid(false, 0.);
    let upper = solid(true, 0.);
    assert_eq!(
        operations::boolean(&upper, &lower, "difference").unwrap(),
        upper
    );
    assert!(
        operations::boolean(&upper, &lower, "intersection")
            .unwrap()
            .is_empty()
    );
}
