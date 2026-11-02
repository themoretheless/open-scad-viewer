use brep_core::profile_certificate::original_restriction::{
    self as r,
    wire::{self, prism},
};
use num_rational::BigRational as Q;
use nurbs_core::curve::Curve;
fn q(n: i64, d: i64) -> [String; 2] {
    [n.to_string(), d.to_string()]
}
fn polygon(points: &[[f64; 2]]) -> wire::Wire {
    let c = Curve::from_polyline(points.iter().map(|p| p.to_vec()).collect()).unwrap();
    let source = r::restrict_binary64(&c, c.domain()).unwrap();
    wire::sew(vec![source], vec![false], q(0, 1)).unwrap()
}
#[test]
fn exact_shell_has_closed_incidence_and_all_continuous_boundary_agreements() {
    let profile = polygon(&[[0., 0.], [2., 0.], [2., 1.], [0., 1.], [0., 0.]]);
    let shell = prism::extrude_binary64(&[profile], [0., 3.], 1e-7).unwrap();
    let m = shell.model();
    assert_eq!((m.vertices.len(), m.edges.len(), m.faces.len()), (8, 12, 6));
    assert!(m.shells[0].closed);
    assert!(m.bodies.is_empty());
    assert_eq!(shell.agreement().edges, 12);
    assert_eq!(shell.agreement().coedges, 24);
    assert_eq!(shell.wall_sources().len(), 4);
    shell.reverify().unwrap();
    let mut forged = m.clone();
    forged.vertices[0].point[0] += Q::new(1.into(), 10.into());
    forged.validate_topology_with_vertices(|_| Ok(())).unwrap();
    assert!(prism::verify_geometry(&forged).is_err());
    let mut forged = m.clone();
    let n = forged.faces.len();
    forged.faces[n - 2].surface = forged.faces[n - 1].surface.clone();
    assert!(prism::verify_geometry(&forged).is_err());
    let mut forged = m.clone();
    forged.loops[0].coedges[0].reversed = true;
    assert!(prism::verify_geometry(&forged).is_err());
}
#[test]
fn nondyadic_source_contact_and_height_are_not_rounded_into_shell_vertices() {
    let c = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.], vec![1., 0.]],
        weights: vec![1., 3.],
        periodic: false,
    };
    let fixed = r::restrict(&c, [q(0, 1), q(1, 3)]).unwrap();
    let c = Curve::from_polyline(vec![
        vec![0.6, 0.],
        vec![0.6, 1.],
        vec![0., 1.],
        vec![0., 0.],
    ])
    .unwrap();
    let flexible = r::restrict_binary64(&c, c.domain()).unwrap();
    let profile = wire::sew(
        vec![fixed.clone(), flexible],
        vec![false, true],
        q(1, 1_000_000),
    )
    .unwrap();
    let shell = prism::extrude(&[profile], [q(1, 3), q(4, 3)], 1e-7).unwrap();
    assert!(
        shell
            .model()
            .vertices
            .iter()
            .any(|v| v.point[0] == Q::new(3.into(), 5.into()))
    );
    assert!(
        shell
            .model()
            .vertices
            .iter()
            .all(|v| v.point[2] == Q::new(1.into(), 3.into())
                || v.point[2] == Q::new(4.into(), 3.into()))
    );
    assert_eq!(shell.source_profiles()[0].pieces()[0], fixed.spans());
    shell.reverify().unwrap();
}
#[test]
fn hole_shell_keeps_cap_loops_and_refuses_nonpositive_height() {
    let outer = polygon(&[[0., 0.], [4., 0.], [4., 4.], [0., 4.], [0., 0.]]);
    let hole = polygon(&[[1., 1.], [1., 2.], [2., 2.], [2., 1.], [1., 1.]]);
    let profiles = [outer, hole];
    let shell = prism::extrude_binary64(&profiles, [-2., 2.], 1e-7).unwrap();
    assert_eq!(
        shell
            .model()
            .faces
            .iter()
            .filter(|f| f.holes.len() == 1)
            .count(),
        2
    );
    assert_eq!(
        brep_topology::explorer::euler_characteristic(shell.model()),
        0
    );
    shell.reverify().unwrap();
    assert!(prism::extrude_binary64(&profiles, [1., 1.], 1e-7).is_err());
    assert!(prism::extrude_binary64(&profiles, [2., 1.], 1e-7).is_err());
}
