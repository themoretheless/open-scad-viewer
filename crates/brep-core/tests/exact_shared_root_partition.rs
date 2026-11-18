use brep_core::{Model, feature_ops::edge_partition::{self, Limits}};
fn source() -> Model {
    let mut m = brep_core::cuboid([0.; 3], [2., 3., 4.]).unwrap();
    m.edges[0].curve.weights = vec![1., 3.];
    for wire in &mut m.loops {
        for c in &mut wire.coedges {
            if c.edge == 0 {
                c.pcurve.weights = if c.reversed {
                    vec![3., 1.]
                } else {
                    vec![1., 3.]
                };
            }
        }
    }
    m.validate().unwrap();
    m
}
#[test]
fn actual_shared_cad_edge_children_reprove_the_complete_rational_world_root() {
    let source = source();
    let before = source.clone();
    let split = edge_partition::split(&source, 0, &[0.5], "exact-rational-root", Limits::default())
        .unwrap();
    assert_eq!(split.edges().len(), 2);
    let proof = split.exact_world_root_partition(1000000).unwrap().unwrap();
    assert_eq!(proof.source(), &source.edges[0].curve);
    assert_eq!(proof.range(), source.edges[0].curve.domain());
    proof.reverify(1000000).unwrap();
    assert!(split.exact_world_root_partition(1).unwrap().is_none());
    for &edge in split.edges() {
        assert_eq!(
            split
                .model()
                .loops
                .iter()
                .flat_map(|l| &l.coedges)
                .filter(|c| c.edge == edge)
                .count(),
            2
        );
    }
    assert!(split.reverify().unwrap().boundary_complete);
    assert_eq!(source, before);
}
