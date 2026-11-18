#[test]
fn original_derivative_budget_refuses_zero_and_partial_axes() {
    let surface=nurbs_core::surface::Surface {degree_u:1,degree_v:1,
        knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
        control_points:vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]],vec![vec![1.,0.,0.],vec![1.,1.,0.]]],
        weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false};
    assert!(super::original_support_derivatives(&surface,[[-1.,1.],[0.,1.]],0).is_err());
    for budget in [0,1] {
        let (bound,spent)=super::original_support_derivatives(&surface,[[0.,1.];2],budget).unwrap();
        assert!(bound.is_none());assert_eq!(spent,budget);
    }
    let (bound,spent)=super::original_support_derivatives(&surface,[[0.,1.];2],2).unwrap();
    assert!(bound.is_some());assert_eq!(spent,2);
}

use super::qualify_vertex_link;
#[test]
fn vertex_links_refuse_pinches_and_branching() {
    // Two otherwise closed fans touching at a vertex are not one manifold fan.
    assert_eq!(
        qualify_vertex_link(&[(0, 1), (1, 2), (2, 0), (3, 4), (4, 5), (5, 3)]),
        Err("profile-shell-disconnected-vertex-link")
    );
    // Three faces along a physical edge create branching in the vertex link.
    assert_eq!(
        qualify_vertex_link(&[(0, 1), (1, 2), (2, 0), (0, 3)]),
        Err("profile-shell-nonmanifold-vertex")
    );
    assert!(qualify_vertex_link(&[(0, 1), (1, 2), (2, 0)]).is_ok());
    // A valid two-edge lens has parallel link arcs, each counted once.
    assert!(qualify_vertex_link(&[(0, 1), (1, 0)]).is_ok());
    assert!(qualify_vertex_link(&[]).is_err());
}
