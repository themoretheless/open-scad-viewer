use brep_core::profile_certificate::{self as pc, ExactRatio};
fn q(n: i64, d: i64) -> ExactRatio {
    [n.to_string(), d.to_string()]
}
#[test]
fn tacnode_and_triple_crossing_keep_all_singular_ports_and_refuse_invalid_inputs() {
    // (v-1/2)^2-(u-1/2)^4: two branches tangent at one node.
    let tacnode = vec![
        vec![q(3, 16), q(1, 2), q(-3, 2), q(2, 1), q(-1, 1)],
        vec![q(-1, 1)],
        vec![q(1, 1)],
    ];
    // (v-1/2)*((v-1/2)^2-(u-1/2)^2): three concurrent lines.
    let triple = vec![
        vec![q(0, 1), q(-1, 2), q(1, 2)],
        vec![q(1, 2), q(1, 1), q(-1, 1)],
        vec![q(-3, 2)],
        vec![q(1, 1)],
    ];
    for (polynomial, ports) in [(tacnode, 4), (triple, 6)] {
        let verified = pc::certify_algebraic_branches(&polynomial).unwrap();
        verified.reverify().unwrap();
        let singular = verified
            .certificate()
            .vertices
            .iter()
            .filter(|v| v.singular)
            .collect::<Vec<_>>();
        assert_eq!(singular.len(), 1);
        assert_eq!(singular[0].ports.len(), ports);
        assert!(
            verified
                .certificate()
                .edges
                .iter()
                .all(|e| e.multiplicity == 1)
        );
    }
    assert!(pc::certify_algebraic_branches(&[vec![q(1, 1); 18]]).is_err());
    assert!(pc::certify_algebraic_branches(&[vec![q(1, 0)]]).is_err());
}
#[test]
fn cusp_and_isolated_real_node_have_exact_incidence() {
    // (v-1/2)^2-(u-1/2)^3.
    let p = vec![
        vec![q(3, 8), q(-3, 4), q(3, 2), q(-1, 1)],
        vec![q(-1, 1)],
        vec![q(1, 1)],
    ];
    let c = pc::certify_algebraic_branches(&p).unwrap();
    c.reverify().unwrap();
    assert_eq!(c.certificate().dimension, pc::AlgebraicSetDimension::Curves);
    let nodes = c
        .certificate()
        .vertices
        .iter()
        .filter(|v| v.singular)
        .collect::<Vec<_>>();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].ports.len(), 2);
    assert!(c.certificate().edges.iter().all(|e| e.multiplicity == 1));
    let point = vec![
        vec![q(1, 2), q(-1, 1), q(1, 1)],
        vec![q(-1, 1)],
        vec![q(1, 1)],
    ];
    let p = pc::certify_algebraic_branches(&point).unwrap();
    assert_eq!(p.certificate().dimension, pc::AlgebraicSetDimension::Points);
    assert_eq!(p.certificate().vertices.len(), 1);
    assert!(p.certificate().vertices[0].singular);
    assert!(p.certificate().vertices[0].ports.is_empty());
}
#[test]
fn crossing_ports_and_repeated_factors_preserve_geometry_and_multiplicity() {
    // (v-u)^2.
    let p = vec![
        vec![q(0, 1), q(0, 1), q(1, 1)],
        vec![q(0, 1), q(-2, 1)],
        vec![q(1, 1)],
    ];
    let c = pc::certify_algebraic_branches(&p).unwrap();
    assert!(c.certificate().edges.iter().all(|e| e.multiplicity == 2));
    c.reverify().unwrap();
    // (v-1/2)^2-(u-1/2)^2.
    let cross = vec![
        vec![q(0, 1), q(1, 1), q(-1, 1)],
        vec![q(-1, 1)],
        vec![q(1, 1)],
    ];
    let c = pc::certify_algebraic_branches(&cross).unwrap();
    let center = c
        .certificate()
        .vertices
        .iter()
        .find(|v| v.singular)
        .unwrap();
    assert_eq!(center.ports.len(), 4);
    let mut bad = c.certificate().clone();
    bad.vertices
        .iter_mut()
        .find(|v| v.singular)
        .unwrap()
        .ports
        .swap(0, 1);
    assert!(pc::verify_algebraic_branches(&cross, &bad).is_err());
    let mut bad = c.certificate().clone();
    bad.edges[0].multiplicity += 1;
    assert!(pc::verify_algebraic_branches(&cross, &bad).is_err());
}
#[test]
fn vertical_repeated_component_and_whole_patch_are_explicit() {
    // (u-1/3)^3, with no nonvertical primitive factor.
    let p = vec![vec![q(-1, 27), q(1, 3), q(-1, 1), q(1, 1)]];
    let c = pc::certify_algebraic_branches(&p).unwrap();
    assert_eq!(c.certificate().edges.len(), 1);
    assert_eq!(c.certificate().edges[0].multiplicity, 3);
    assert!(!c.certificate().vertices.iter().any(|v| v.singular));
    let c = pc::certify_algebraic_branches(&[]).unwrap();
    assert_eq!(c.certificate().dimension, pc::AlgebraicSetDimension::Area);
    assert!(c.certificate().edges.is_empty());
    let c = pc::certify_algebraic_branches(&[vec![q(1, 1)]]).unwrap();
    assert_eq!(c.certificate().dimension, pc::AlgebraicSetDimension::Empty);
}

#[test]
fn irrational_singular_parameter_is_selected_algebraically() {
    // (v-1/2)^2-(u^2-1/2)^3: an interior cusp at u=sqrt(1/2).
    let p = vec![
        vec![
            q(3, 8),
            q(0, 1),
            q(-3, 4),
            q(0, 1),
            q(3, 2),
            q(0, 1),
            q(-1, 1),
        ],
        vec![q(-1, 1)],
        vec![q(1, 1)],
    ];
    let c = pc::certify_algebraic_branches(&p).unwrap();
    c.reverify().unwrap();
    let (id, v) = c
        .certificate()
        .vertices
        .iter()
        .enumerate()
        .find(|(_, v)| v.singular)
        .unwrap();
    assert_eq!(v.ports.len(), 2);
    let b = c.vertex_enclosure(id).unwrap();
    assert_ne!(b[0][0], b[0][1]);
    let mut bad = c.certificate().clone();
    bad.vertices[id].singular = false;
    assert!(pc::verify_algebraic_branches(&p, &bad).is_err());
}
