use nurbs_core::{
    curve::Curve,
    periodic_trim::{self as p, ChartBoundaryGraph, ChartEdge},
};
fn graph() -> ChartBoundaryGraph {
    let c = Curve::from_polyline(vec![
        vec![0., 0.],
        vec![2., 0.],
        vec![2., 2.],
        vec![0., 2.],
        vec![0., 0.],
    ])
    .unwrap();
    ChartBoundaryGraph {
        edges: vec![
            ChartEdge {
                curve: c.clone(),
                source: Some([0, 0]),
            },
            ChartEdge {
                curve: c,
                source: Some([1, 0]),
            },
        ],
        loops: vec![vec![0], vec![1]],
        connector_pairs: vec![],
    }
}
#[test]
fn candidate_compaction_preserves_actual_source_roles_without_claiming_a_certificate() {
    let g = graph();
    let candidate = p::compact_image_alias_candidates(&g, &[]).unwrap().unwrap();
    assert_eq!(candidate.edges.len(), 1);
    assert_eq!(candidate.edges[0].coefficient, 2);
    assert_eq!(candidate.edges[0].contributions[0].source, Some([0, 0]));
    assert_eq!(candidate.edges[0].contributions[1].source, Some([1, 0]));
    assert_eq!(candidate.loops.len(), 2);
    assert_eq!(candidate.expanded_edges, 2);
}
#[test]
fn candidate_alias_input_rejects_partial_census_bad_indices_and_invalid_orientation() {
    let g = graph();
    for alias in [
        p::CandidateImageAlias {
            edges: [0, 0],
            orientation: 1,
        },
        p::CandidateImageAlias {
            edges: [0, 2],
            orientation: 1,
        },
        p::CandidateImageAlias {
            edges: [0, 1],
            orientation: 0,
        },
    ] {
        assert!(p::compact_image_alias_candidates(&g, &[alias]).is_err());
    }
    let mut bad = g.clone();
    bad.loops.pop();
    assert!(p::compact_image_alias_candidates(&bad, &[]).is_err());
    let mut bad = g.clone();
    bad.loops.push(vec![0]);
    assert!(p::compact_image_alias_candidates(&bad, &[]).is_err());
    let mut bad = g.clone();
    bad.connector_pairs.push([0, 2]);
    assert!(p::compact_image_alias_candidates(&bad, &[]).is_err());
    let mut bad = g;
    bad.edges[0]
        .curve
        .control_points
        .iter_mut()
        .for_each(|p| p.push(0.));
    assert!(p::compact_image_alias_candidates(&bad, &[]).is_err());
}
