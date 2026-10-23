use nurbs_core::{
    ss_intersection::{self as ss, CoverageProof, SurfaceSurfaceComponent},
    surface::Surface,
};
fn graph() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 0.5, 0.], vec![0.1, 1., 0.]],
            vec![vec![1., 0., 1.], vec![1., 0.5, 1.], vec![1.1, 1., 1.]],
        ],
        weights: vec![vec![1.; 3]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn plane(y: [f64; 2]) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 5., 5.],
        knots_v: vec![-3., -3., 7., 7.],
        control_points: vec![
            vec![vec![0.5, y[0], -1.], vec![0.5, y[0], 2.]],
            vec![vec![0.5, y[1], -1.], vec![0.5, y[1], 2.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn original_graph_completeness_has_independent_nonconstant_root_oracle() {
    let a = graph();
    let b = plane([-1., 2.]);
    let r = ss::intersect_surface_surface_report(&a, &b, None).unwrap();
    assert!(r.complete());
    assert_eq!(r.coverage_proof, CoverageProof::MonotoneGraphInRectangle);
    assert!(!r.topology_proven && !r.summary().permits_topology_authorship);
    let SurfaceSurfaceComponent::Continued(c) = &r.components[0] else {
        panic!("candidate graph expected")
    };
    // x=u+.1v^2, y=v, z=u: exactly one transverse root u=.5-.1v^2.
    for sample in &c.samples {
        let [u, v] = sample.uv_first;
        assert!((u - (0.5 - 0.1 * v * v)).abs() < 1e-13);
        let p = a.evaluate(u, v).unwrap().point;
        let q = b
            .evaluate(sample.uv_second[0], sample.uv_second[1])
            .unwrap()
            .point;
        for k in 0..3 {
            assert!((p[k] - q[k]).abs() < 1e-13)
        }
    }
    assert!((c.first_uv[0] - c.last_uv[0]).abs() > 0.09);
    assert_eq!(c.geometry_enclosure, [[0., 1.1], [0., 1.], [0., 1.]]);
}
#[test]
fn rational_original_source_keeps_coverage_without_owning_trim_topology() {
    let mut a = graph();
    a.weights = vec![vec![1., 1.25, 1.]; 2];
    let r = ss::intersect_surface_surface_report(&a, &plane([-1., 2.]), None).unwrap();
    assert!(r.complete());
    assert_eq!(r.coverage_proof, CoverageProof::MonotoneGraphInRectangle);
    assert!(!r.topology_proven);
}
#[test]
fn a_plane_that_clips_the_graph_must_not_receive_the_whole_graph_receipt() {
    let r = ss::intersect_surface_surface_report(&graph(), &plane([0.25, 0.75]), None).unwrap();
    assert!(!r.complete());
    assert_ne!(r.coverage_proof, CoverageProof::MonotoneGraphInRectangle);
    assert!(!r.unresolved.is_empty());
}
