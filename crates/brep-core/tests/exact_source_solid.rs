use brep_core::profile_certificate::original_restriction::{
    self as r,
    wire::{self, jordan, nesting, prism},
};
use nurbs_core::curve::Curve;
fn q(n: i64, d: i64) -> [String; 2] {
    [n.to_string(), d.to_string()]
}
fn polygon(points: &[[f64; 2]]) -> wire::Wire {
    let c = Curve::from_polyline(points.iter().map(|p| p.to_vec()).collect()).unwrap();
    wire::sew(
        vec![r::restrict_binary64(&c, c.domain()).unwrap()],
        vec![false],
        q(0, 1),
    )
    .unwrap()
}
fn outer() -> wire::Wire {
    polygon(&[[0., 0.], [5., 0.], [5., 5.], [0., 5.], [0., 0.]])
}
fn admit(profiles: &[wire::Wire]) -> Result<prism::Solid, nurbs_core::Error> {
    prism::extrude(profiles, [q(1, 3), q(4, 3)], 1e-7)
        .unwrap()
        .qualify_solid(jordan::Limits::default(), nesting::Limits::default())
}
#[test]
fn exact_embedded_solid_has_outward_outer_and_two_proved_through_holes() {
    let a = polygon(&[[1., 1.], [1., 2.], [2., 2.], [2., 1.], [1., 1.]]);
    let b = polygon(&[[3., 3.], [3., 4.], [4., 4.], [4., 3.], [3., 3.]]);
    let solid = admit(&[outer(), a, b]).unwrap();
    assert_eq!(solid.model().bodies.len(), 1);
    assert!(solid.model().bodies[0].inner_shells.is_empty());
    assert_eq!(
        brep_topology::explorer::euler_characteristic(solid.model()),
        -2
    );
    assert_eq!(
        solid
            .nesting()
            .orientations()
            .iter()
            .map(|o| o.windings)
            .collect::<Vec<_>>(),
        vec![[1, 0], [0, -1], [0, -1]]
    );
    assert_eq!(
        solid.nesting().containment(),
        &[
            vec![None, Some(0), Some(0)],
            vec![Some(1), None, Some(0)],
            vec![Some(1), Some(0), None]
        ]
    );
    assert!(solid.shell_geometry().model().bodies.is_empty());
    solid.reverify().unwrap();
}
#[test]
fn wrong_outer_and_hole_orientation_never_assign_body_roles() {
    let clockwise = polygon(&[[0., 0.], [0., 5.], [5., 5.], [5., 0.], [0., 0.]]);
    assert!(admit(&[clockwise]).is_err());
    let wrong_hole = polygon(&[[1., 1.], [2., 1.], [2., 2.], [1., 2.], [1., 1.]]);
    assert!(admit(&[outer(), wrong_hole]).is_err());
}
#[test]
fn outside_and_nested_holes_refuse_even_with_disjoint_jordan_boundaries() {
    let outside = polygon(&[[6., 1.], [6., 2.], [7., 2.], [7., 1.], [6., 1.]]);
    assert!(admit(&[outer(), outside]).is_err());
    let a = polygon(&[[1., 1.], [1., 4.], [4., 4.], [4., 1.], [1., 1.]]);
    let b = polygon(&[[2., 2.], [2., 3.], [3., 3.], [3., 2.], [2., 2.]]);
    assert!(admit(&[outer(), a, b]).is_err());
}
#[test]
fn exhausted_nesting_budget_and_bowtie_do_not_promote_shells() {
    let shell = prism::extrude_binary64(&[outer()], [0., 2.], 1e-7).unwrap();
    let error = shell
        .qualify_solid(
            jordan::Limits::default(),
            nesting::Limits { max_ray_cells: 0 },
        )
        .unwrap_err();
    assert!(error.to_string().contains("budget exhausted"));
    assert!(shell.model().bodies.is_empty());
    let bowtie = polygon(&[[0., 0.], [2., 2.], [0., 2.], [2., 0.], [0., 0.]]);
    assert!(admit(&[bowtie]).is_err());
}

#[test]
fn thin_exact_hole_keeps_nonzero_geometry_without_numeric_area_sampling() {
    let delta = 2f64.powi(-45);
    let hole = polygon(&[
        [1., 1.],
        [1., 1. + delta],
        [1. + delta, 1. + delta],
        [1. + delta, 1.],
        [1., 1.],
    ]);
    let solid = admit(&[outer(), hole]).unwrap();
    assert_eq!(solid.nesting().orientations()[1].windings, [0, -1]);
    assert_eq!(solid.nesting().containment()[1][0], Some(1));
    assert_eq!(solid.model().bodies.len(), 1);
    solid.reverify().unwrap();
}
