use nurbs_core::{
    surface::Surface,
    surface_offset::{bounds, evaluate, jacobian_bounds},
};
fn sheet() -> Surface {
    Surface {
        degree_u: 3,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [0., 2., -1., 1.]
            .map(|x| vec![vec![x, 0.5, 0.], vec![x, 0.5, 1.]])
            .to_vec(),
        weights: vec![vec![1.; 2]; 4],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn nonlinear_coordinate_plane_has_exact_oriented_normal_and_original_derivative_bounds() {
    let s = sheet();
    for (u, sign) in [([0.149, 0.154], -1.), ([0.49, 0.51], 1.)] {
        let domain = [u, [0.1, 0.3]];
        let b = bounds(&s, domain, 0.2, 128).unwrap();
        assert_eq!(b.unit_normals.unwrap(), [[0.; 2], [sign; 2], [0.; 2]]);
        let original = jacobian_bounds(&s, domain, 0., 128).unwrap();
        let offset = jacobian_bounds(&s, domain, 0.2, 128).unwrap();
        assert_eq!(offset.derivatives, original.derivatives);
        let p = evaluate(&s, [(u[0] + u[1]) * 0.5, 0.2], 0.2).unwrap();
        let image = b.image.unwrap();
        assert!(
            p.point
                .iter()
                .enumerate()
                .all(|(k, x)| *x >= image[k][0] && *x <= image[k][1])
        );
    }
    assert!(
        bounds(&s, [[0., 1.], [0.1, 0.3]], 0.2, 128)
            .unwrap()
            .image
            .is_none()
    );
    assert!(
        jacobian_bounds(&s, [[0., 1.], [0.1, 0.3]], 0.2, 128)
            .unwrap()
            .derivatives
            .is_none()
    );
}
#[test]
fn nearly_planar_control_net_is_never_snapped_to_an_exact_normal() {
    let mut s = sheet();
    s.control_points[1][1][1] += 0.01;
    let b = bounds(&s, [[0.149, 0.154], [0.1, 0.3]], 0.2, 128).unwrap();
    let n = b.unit_normals.unwrap();
    assert!(n[2][0] > 0.);
    assert_ne!(n, [[0.; 2], [-1.; 2], [0.; 2]]);
}
