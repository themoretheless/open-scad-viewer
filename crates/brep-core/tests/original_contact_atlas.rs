use brep_core::profile_certificate as p;
use nurbs_core::curve::Curve;
use std::cmp::Ordering;
fn curve(points: Vec<Vec<f64>>) -> Curve {
    let degree = points.len() - 1;
    Curve {
        degree,
        knots: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
        weights: vec![1.; points.len()],
        control_points: points,
        periodic: false,
    }
}
#[test]
fn complete_three_curve_census_merges_exact_triple_nodes_from_distinct_primitives() {
    let source = curve(vec![vec![-3., 0.], vec![0., 6.], vec![3., 0.]]);
    let horizontal = curve(vec![vec![-4., 1.5], vec![4., 1.5]]);
    let nonlinear = curve(vec![
        vec![-3., 3.],
        vec![-3., 3.],
        vec![-2., 1.],
        vec![0., -3.],
        vec![3., 3.],
    ]);
    let limits = p::ContactAtlasLimits::default();
    let sources = vec![source, horizontal, nonlinear];
    let atlas = p::assemble_original_contact_atlas(&sources, 1e-12, limits).unwrap();
    atlas.reverify_source().unwrap();
    assert_eq!(atlas.pairs().len(), 6);
    assert_eq!(atlas.occurrences().len(), 7);
    assert_eq!(atlas.groups().iter().filter(|g| g.len() == 3).count(), 2);
    assert_eq!(atlas.groups().len(), 3); // two triple nodes + original stationary endpoint
    assert_eq!(
        atlas
            .ordered_parameters()
            .iter()
            .map(Vec::len)
            .collect::<Vec<_>>(),
        [2, 2, 3]
    );
    assert!(atlas.comparisons_used() > 0);
    let insufficient = p::ContactAtlasLimits {
        max_comparisons: atlas.comparisons_used() - 1,
        ..limits
    };
    assert!(p::assemble_original_contact_atlas(&sources, 1e-12, insufficient).is_err());
}
#[test]
fn a_self_node_keeps_two_distinct_ordered_source_parameters() {
    let source = curve(vec![
        vec![3., -1.5],
        vec![-1., 3.5],
        vec![-1., -3.5],
        vec![3., 1.5],
    ]);
    let atlas =
        p::assemble_original_contact_atlas(&[source], 1e-12, p::ContactAtlasLimits::default())
            .unwrap();
    atlas.reverify_source().unwrap();
    assert_eq!(atlas.groups().len(), 1);
    assert_eq!(atlas.occurrences().len(), 1);
    assert_eq!(atlas.ordered_parameters()[0].len(), 2);
    let params = &atlas.ordered_parameters()[0];
    let a = params[0][0];
    let b = params[1][0];
    let oa = atlas.occurrences()[a.occurrence];
    let ob = atlas.occurrences()[b.occurrence];
    let proof = p::compare_event_parameters(
        atlas.owners()[oa.pair].clone(),
        p::EventParameterRef {
            node: oa.node,
            source: a.side,
        },
        atlas.owners()[ob.pair].clone(),
        p::EventParameterRef {
            node: ob.node,
            source: b.side,
        },
        p::ParameterOrderLimits::default(),
    )
    .unwrap();
    assert_eq!(proof.ordering(), Ordering::Less);
}
#[test]
fn overlap_branches_and_partial_event_budget_do_not_become_empty_isolated_atlases() {
    let a = curve(vec![vec![0., 0.], vec![2., 0.]]);
    let b = curve(vec![vec![1., 0.], vec![3., 0.]]);
    assert!(
        p::assemble_original_contact_atlas(&[a, b], 1e-12, p::ContactAtlasLimits::default())
            .is_err()
    );
    let a = curve(vec![vec![-2., 0.], vec![0., 4.], vec![2., 0.]]);
    let b = curve(vec![vec![-3., 1.], vec![3., 1.]]);
    assert!(
        p::assemble_original_contact_atlas(
            &[a, b],
            1e-12,
            p::ContactAtlasLimits {
                max_events: 1,
                ..p::ContactAtlasLimits::default()
            }
        )
        .is_err()
    );
}
