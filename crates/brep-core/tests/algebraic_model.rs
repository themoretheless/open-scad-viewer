use brep_core::{AlgebraicModel, ExactBooleanOptions, ExactModel, minkowski3d::curved, operations};
use num_rational::BigRational as Q;
use num_traits::{One, ToPrimitive, Zero};
use nurbs_core::surface::Surface;
#[cfg(feature = "codec")]
use value_codec::{Deserialize, Serialize};
fn q(n: i64, d: i64) -> Q {
    Q::new(n.into(), d.into())
}
fn choose(n: usize, k: usize) -> Q {
    (0..k).fold(Q::one(), |v, j| {
        v * Q::from_integer((n - j).into()) / Q::from_integer((j + 1).into())
    })
}
fn surface(p: &[Vec<Q>]) -> Surface {
    surface_degree(p, 2)
}
fn surface_degree(p: &[Vec<Q>], d: usize) -> Surface {
    let mut net = vec![vec![vec![0.; 3]; d + 1]; d + 1];
    for i in 0..=d {
        for j in 0..=d {
            let mut z = Q::zero();
            for (y, row) in p.iter().enumerate() {
                for (x, c) in row.iter().enumerate() {
                    if x <= i && y <= j {
                        z += c * choose(i, x) / choose(d, x) * choose(j, y) / choose(d, y);
                    }
                }
            }
            let f = z.to_f64().unwrap();
            assert_eq!(Q::from_float(f).unwrap(), z);
            net[i][j] = vec![i as f64 / d as f64, j as f64 / d as f64, f];
        }
    }
    Surface {
        degree_u: d,
        degree_v: d,
        knots_u: [vec![0.; d + 1], vec![1.; d + 1]].concat(),
        knots_v: [vec![0.; d + 1], vec![1.; d + 1]].concat(),
        control_points: net,
        weights: vec![vec![1.; d + 1]; d + 1],
        periodic_u: false,
        periodic_v: false,
    }
}
fn caps() -> (Surface, Surface) {
    let lower = vec![vec![Q::zero(), Q::zero(), q(1, 8)], vec![], vec![q(1, 4)]];
    // Difference 1/64 - (u-1/4)^2 - (v-1/4)^2.
    let upper = vec![
        vec![q(-7, 64), q(1, 2), q(-7, 8)],
        vec![q(1, 2)],
        vec![q(-3, 4)],
    ];
    (surface(&upper), surface(&lower))
}
fn witness() -> Vec<Vec<[String; 2]>> {
    vec![vec![["4".into(), "1".into()]]]
}
#[test]
fn exact_model_owns_algebraic_edges_without_rational_curve_proxies() {
    let (upper, lower) = caps();
    let region = curved::certify_graph_region_brep(&upper, &lower, &witness(), None).unwrap();
    let model = AlgebraicModel::from_region(region, 1e-6).unwrap();
    model.reverify().unwrap();
    let t = model.topology();
    assert_eq!(t.faces.len(), 2);
    assert_eq!(t.bodies.len(), 1);
    assert_eq!(t.genus(), 0);
    assert!(!t.edges.is_empty());
    for i in 0..t.edges.len() {
        let b = model
            .edge_parameter_enclosure(i, &["1".into(), "4".into()])
            .unwrap();
        assert_eq!(b[0][0], ["1".to_string(), "4".to_string()]);
    }
    #[cfg(feature = "codec")]
    {
        let value = model.to_value();
        let decoded = AlgebraicModel::from_value(value.clone()).unwrap();
        assert_eq!(decoded.topology(), model.topology());
        let mut forged = value;
        forged["topology"]["edges"][0]["curve"]["domainEdge"] = value_codec::json!(999999);
        assert!(AlgebraicModel::from_value(forged).is_err());
    }
}
#[test]
fn typed_boolean_of_independent_graph_slabs_returns_new_algebraic_model() {
    let (upper, lower) = caps();
    let a = curved::GraphSlab::new(surface(&[vec![q(-2, 1)]]), upper).unwrap();
    let b = curved::GraphSlab::new(lower, surface(&[vec![q(2, 1)]])).unwrap();
    let result = operations::boolean_exact(
        &ExactModel::GraphSlab(a.clone()),
        &ExactModel::GraphSlab(b.clone()),
        "intersection",
        &ExactBooleanOptions::default(),
    )
    .unwrap();
    let ExactModel::Algebraic(model) = result else {
        panic!("An exact algebraic result must not be downgraded to rational curves");
    };
    model.reverify().unwrap();
    assert_eq!(model.topology().bodies.len(), 1);
    assert!(
        operations::boolean_exact(
            &ExactModel::GraphSlab(a),
            &ExactModel::GraphSlab(b),
            "union",
            &ExactBooleanOptions::default()
        )
        .is_err()
    );
}

#[test]
fn surface_source_branch_graph_and_boolean_receipts_are_not_interchangeable() {
    let (upper, lower) = caps();
    let g = curved::certify_surface_algebraic_branches(&upper, &lower).unwrap();
    g.reverify().unwrap();
    assert!(g.branches().certificate().edges.len() >= 2);
    let a = curved::GraphSlab::new(surface(&[vec![q(-2, 1)]]), upper.clone()).unwrap();
    let b = curved::GraphSlab::new(lower, surface(&[vec![q(2, 1)]])).unwrap();
    let op =
        brep_core::algebraic_model::certify_graph_slab_boolean(&a, &b, &witness(), 1e-6).unwrap();
    brep_core::algebraic_model::verify_graph_slab_boolean(&a, &b, op.model(), op.certificate())
        .unwrap();
    let mut bad = op.certificate().clone();
    bad.input_sources[0][0] = String::new();
    assert!(
        brep_core::algebraic_model::verify_graph_slab_boolean(&a, &b, op.model(), &bad).is_err()
    );
    let mut changed = upper;
    changed.control_points[1][1][2] += 0.125;
    assert!(
        curved::verify_surface_algebraic_branches(
            &changed,
            g.carriers().last().unwrap(),
            g.certificate()
        )
        .is_err()
    );
}

#[test]
fn nonrational_quartic_boundary_is_stored_as_an_algebraic_edge() {
    let lower = surface_degree(&[vec![q(0, 1), q(0, 1), q(3, 8)], vec![], vec![q(3, 8)]], 4);
    let upper = surface_degree(
        &[
            vec![q(-93, 4096), q(3, 16), q(-3, 4), q(3, 1), q(-3, 1)],
            vec![q(3, 16)],
            vec![q(-3, 4)],
            vec![q(3, 1)],
            vec![q(-3, 1)],
        ],
        4,
    );
    let a = curved::GraphSlab::new(surface(&[vec![q(-2, 1)]]), upper).unwrap();
    let b = curved::GraphSlab::new(lower, surface(&[vec![q(2, 1)]])).unwrap();
    let m = vec![vec![["1".into(), "64".into()]]];
    let result = brep_core::algebraic_model::certify_graph_slab_boolean(&a, &b, &m, 1e-6).unwrap();
    result.model().reverify().unwrap();
    let point = result
        .model()
        .edge_parameter_enclosure(0, &["3".into(), "16".into()])
        .unwrap();
    assert_ne!(point[1][0], point[1][1]);
    assert!(result.model().carriers().iter().all(|s| s.degree_u == 4));
}

#[test]
fn exact_intersection_is_commutative_and_algebraic_identity_remains_edit_safe() {
    let (upper, lower) = caps();
    let a = curved::GraphSlab::new(surface(&[vec![q(-2, 1)]]), upper).unwrap();
    let b = curved::GraphSlab::new(lower, surface(&[vec![q(2, 1)]])).unwrap();
    let first =
        brep_core::algebraic_model::certify_graph_slab_boolean(&a, &b, &witness(), 1e-6).unwrap();
    let reverse =
        brep_core::algebraic_model::certify_graph_slab_boolean(&b, &a, &witness(), 1e-6).unwrap();
    assert!(reverse.certificate().envelope_reversed);
    assert_eq!(first.model().topology(), reverse.model().topology());
    brep_core::algebraic_model::verify_graph_slab_boolean(
        &b,
        &a,
        reverse.model(),
        reverse.certificate(),
    )
    .unwrap();
    let model = ExactModel::Algebraic(first.into_model());
    let result = operations::boolean_exact(
        &model,
        &model,
        "difference",
        &ExactBooleanOptions::default(),
    )
    .unwrap();
    let ExactModel::Nurbs(empty) = result else {
        panic!("Expected empty exact identity result");
    };
    assert!(empty.is_empty());
}

#[test]
fn rational_nurbs_carriers_keep_source_weights_in_the_branch_certificate() {
    // W=1+12u-12u² has degree-four Bernstein controls [1,4,5,4,1].
    // The authored poles below represent affine X=u exactly, despite weights.
    let height = vec![
        vec![q(-105, 64), q(15, 2), q(-105, 8)],
        vec![q(15, 2)],
        vec![q(-59, 4)],
    ];
    let w = [q(1, 1), q(12, 1), q(-12, 1)];
    let weights = [1., 4., 5., 4., 1.];
    let x = [0., 0.0625, 0.5, 0.9375, 1.];
    let mut numerator = vec![vec![Q::zero(); 5]; 3];
    for (j, row) in height.iter().enumerate() {
        for (i, c) in row.iter().enumerate() {
            for (k, w) in w.iter().enumerate() {
                numerator[j][i + k] += c * w;
            }
        }
    }
    let mut s = surface_degree(&[vec![q(0, 1)]], 4);
    s.degree_v = 2;
    s.knots_v = vec![0., 0., 0., 1., 1., 1.];
    s.control_points = vec![vec![vec![0.; 3]; 3]; 5];
    s.weights = weights.iter().map(|&v| vec![v; 3]).collect();
    for i in 0..5 {
        for j in 0..3 {
            let mut z = Q::zero();
            for b in 0..=j {
                for a in 0..=i {
                    z += &numerator[b][a] * choose(i, a) / choose(4, a) * choose(j, b)
                        / choose(2, b);
                }
            }
            z /= Q::from_float(weights[i]).unwrap();
            let f = z.to_f64().unwrap();
            assert_eq!(Q::from_float(f).unwrap(), z);
            s.control_points[i][j] = vec![x[i], j as f64 / 2., f];
        }
    }
    let lower = surface(&[vec![q(0, 1), q(0, 1), q(15, 8)], vec![], vec![q(1, 4)]]);
    let proof = curved::certify_surface_algebraic_branches(&s, &lower).unwrap();
    proof.reverify().unwrap();
    assert_eq!(proof.carriers()[0].weights, s.weights);
    assert!(!proof.branches().certificate().edges.is_empty());
}

#[cfg(feature = "codec")]
#[test]
fn tagged_exact_geometry_roundtrips_and_unknown_geometry_does_not_fall_back() {
    let (upper, lower) = caps();
    let a =
        ExactModel::GraphSlab(curved::GraphSlab::new(surface(&[vec![q(-2, 1)]]), upper).unwrap());
    let b =
        ExactModel::GraphSlab(curved::GraphSlab::new(lower, surface(&[vec![q(2, 1)]])).unwrap());
    let a = ExactModel::from_value(a.to_value()).unwrap();
    let b = ExactModel::from_value(b.to_value()).unwrap();
    let result =
        operations::boolean_exact(&a, &b, "intersection", &ExactBooleanOptions::default()).unwrap();
    let value = result.to_value();
    let restored = ExactModel::from_value(value.clone()).unwrap();
    let ExactModel::Algebraic(model) = restored else {
        panic!("Algebraic geometry was lost on persistence");
    };
    model.reverify().unwrap();
    let mut bad = value;
    bad["kind"] = value_codec::json!("nurbs");
    assert!(ExactModel::from_value(bad).is_err());
}
