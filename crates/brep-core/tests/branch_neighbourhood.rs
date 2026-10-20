use brep_core::profile_certificate as p;
use num_rational::BigRational as Q;
use nurbs_core::curve::Curve;
use std::sync::Arc;
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
fn q(r: &p::ExactRatio) -> Q {
    Q::new(r[0].parse().unwrap(), r[1].parse().unwrap())
}
#[test]
fn original_neighbourhood_stops_before_the_next_contact_and_coordinate_turn() {
    let a = curve(vec![vec![-3., 0.], vec![0., 6.], vec![3., 0.]]);
    let b = curve(vec![vec![-4., 1.5], vec![4., 1.5]]);
    let atlas = Arc::new(
        p::assemble_original_contact_atlas(&[a, b], 1e-12, p::ContactAtlasLimits::default())
            .unwrap(),
    );
    let graph = p::own_branch_neighbourhood(
        atlas.clone(),
        0,
        0,
        1,
        1,
        p::BranchNeighbourhoodLimits::default(),
    )
    .unwrap();
    graph.reverify_source().unwrap();
    assert!(q(&graph.record().natural_cut) < Q::new(1.into(), 2.into()));
    assert!(!graph.record().contact_coordinate_critical);
    assert!(graph.record().steps > 1);
    assert!(
        p::own_branch_neighbourhood(
            atlas,
            0,
            0,
            1,
            1,
            p::BranchNeighbourhoodLimits {
                max_steps: 1,
                ..p::BranchNeighbourhoodLimits::default()
            }
        )
        .is_err()
    );
}
#[test]
fn stationary_cusp_sides_have_actual_monotone_neighbourhoods_on_both_sides() {
    let c = curve(vec![
        vec![3., -1.],
        vec![-1., 1.],
        vec![-1., -1.],
        vec![3., 1.],
    ]);
    let atlas = Arc::new(
        p::assemble_original_contact_atlas(&[c], 1e-12, p::ContactAtlasLimits::default()).unwrap(),
    );
    let a = p::own_branch_neighbourhood(
        atlas.clone(),
        0,
        0,
        -1,
        0,
        p::BranchNeighbourhoodLimits::default(),
    )
    .unwrap();
    let b = p::own_branch_neighbourhood(atlas, 0, 0, 1, 0, p::BranchNeighbourhoodLimits::default())
        .unwrap();
    a.reverify_source().unwrap();
    b.reverify_source().unwrap();
    assert!(a.record().contact_coordinate_critical && b.record().contact_coordinate_critical);
    assert!(q(&a.record().natural_cut) < Q::new(1.into(), 2.into()));
    assert!(q(&b.record().natural_cut) > Q::new(1.into(), 2.into()));
    assert!(q(&a.record().cut_position[1]) < Q::from_integer(0.into()));
    assert!(q(&b.record().cut_position[1]) > Q::from_integer(0.into()));
}
