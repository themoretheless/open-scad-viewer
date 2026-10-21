use brep_core::profile_certificate as p;
use nurbs_core::curve::Curve;
use std::sync::Arc;
fn line(a: [f64; 2], b: [f64; 2]) -> Curve {
    Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap()
}
fn graph(sources: &[Curve]) -> Arc<p::OwnedOriginalCarrierGraph> {
    let a =
        Arc::new(p::assemble_original_contact_atlas(sources, 1e-12, Default::default()).unwrap());
    Arc::new(p::own_original_carrier_graph(Arc::new(p::own_original_carriers(a).unwrap())).unwrap())
}
#[test]
fn cylinder_and_torus_corner_quotients_keep_actual_shore_provenance() {
    let g = graph(&[
        line([0., 0.], [1., 0.]),
        line([1., 0.], [1., 1.]),
        line([1., 1.], [0., 1.]),
        line([0., 1.], [0., 0.]),
    ]);
    let flat = p::own_original_chart_vertices(
        g.clone(),
        [[0., 1.], [0., 1.]],
        [false, false],
        Default::default(),
    )
    .unwrap();
    assert_eq!(flat.groups().len(), 4);
    let cylinder = p::own_original_chart_vertices(
        g.clone(),
        [[0., 1.], [0., 1.]],
        [true, false],
        Default::default(),
    )
    .unwrap();
    cylinder.reverify_source().unwrap();
    assert_eq!(cylinder.groups().len(), 2);
    assert_eq!(cylinder.upper_shifts().iter().filter(|s| s[0]).count(), 2);
    let torus =
        p::own_original_chart_vertices(g, [[0., 1.], [0., 1.]], [true, true], Default::default())
            .unwrap();
    torus.reverify_source().unwrap();
    assert_eq!(torus.groups().len(), 1);
    assert_eq!(torus.groups()[0].len(), 4);
}
#[test]
fn independently_owned_irrational_shore_contacts_are_exactly_identified() {
    // x=3(2t^2-1)^2 touches x=0 at t=sqrt(1/2), y=t.
    let a = Curve {
        degree: 4,
        knots: vec![0., 0., 0., 0., 0., 1., 1., 1., 1., 1.],
        weights: vec![1.; 5],
        control_points: vec![
            vec![3., 0.],
            vec![3., 0.25],
            vec![1., 0.5],
            vec![-3., 0.75],
            vec![3., 1.],
        ],
        periodic: false,
    };
    let mut b = a.clone();
    for p in &mut b.control_points {
        p[0] = 6. - p[0];
    }
    let g = graph(&[a, b, line([0., 0.], [0., 1.]), line([6., 0.], [6., 1.])]);
    assert_eq!(g.vertices().len(), 8);
    let q =
        p::own_original_chart_vertices(g, [[0., 6.], [0., 1.]], [true, false], Default::default())
            .unwrap();
    q.reverify_source().unwrap();
    assert_eq!(q.groups().len(), 5);
    assert!(q.groups().iter().any(|group| group.len() == 2
        && group.iter().all(|&i| matches!(
            q.graph().vertices()[i],
            p::OriginalCarrierVertex::Contact(_)
        ))));
}
#[test]
fn one_ulp_difference_and_missing_budget_never_create_a_shore_match() {
    let next = f64::from_bits(0.5f64.to_bits() + 1);
    let g = graph(&[line([0., 0.5], [0.25, 0.5]), line([1., next], [0.75, next])]);
    let q = p::own_original_chart_vertices(
        g.clone(),
        [[0., 1.], [0., 1.]],
        [true, false],
        Default::default(),
    )
    .unwrap();
    q.reverify_source().unwrap();
    assert_eq!(q.groups().len(), 4);
    assert!(
        p::own_original_chart_vertices(
            g.clone(),
            [[0., 1.], [0., 1.]],
            [true, false],
            p::OriginalChartVertexLimits {
                comparisons: 0,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        p::own_original_chart_vertices(g, [[0., 1.], [0.6, 1.]], [true, false], Default::default())
            .is_err()
    );
}
