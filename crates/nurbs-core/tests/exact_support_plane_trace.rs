use num_rational::BigRational as Q;
use nurbs_core::original_graph_transition::{Budget, polynomial_trim_trace_covered};
#[test]
fn a_shared_edge_does_not_hide_an_isolated_touch_elsewhere_on_the_plane() {
    // Simple CCW polygon above V=0: covered edge U=-2..-1, but another
    // non-edge boundary vertex touches V=0 at U=1. That contact is NOT covered.
    let points = [
        [-2, 0],
        [-1, 0],
        [-1, 1],
        [0, 1],
        [1, 0],
        [2, 1],
        [2, 2],
        [-2, 2],
    ];
    let curves = (0..points.len())
        .map(|i| {
            [points[i], points[(i + 1) % points.len()]]
                .map(|p| p.map(|x| Q::from_integer(x.into())))
                .to_vec()
        })
        .collect::<Vec<_>>();
    let mut covered = vec![false; points.len()];
    covered[0] = true;
    let mut b = Budget::new(100_000).unwrap();
    assert!(
        !polynomial_trim_trace_covered(
            &curves,
            &covered,
            1,
            false,
            &Q::from_integer(0.into()),
            &mut b
        )
        .unwrap()
    );
}
