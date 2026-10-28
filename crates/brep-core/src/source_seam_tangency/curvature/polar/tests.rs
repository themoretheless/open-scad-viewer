use super::*;
use crate::source_boundary_fragment::{Endpoint, Fragment};
use nurbs_core::{
    curve::Curve, factored_surface_normals as normals, surface::Surface, surface_join::Boundary,
};
fn bowl(sign: f64, weighted: bool, angular: f64) -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        vec![
                            i as f64 / 2.,
                            sign * (i * j) as f64 / 4.,
                            if i == 2 {
                                [1., 1. + sign * 0.25, 1. + sign * 0.5 + angular][j]
                            } else {
                                0.
                            },
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        if weighted {
                            2f64.powi(i as i32 - j as i32)
                        } else {
                            1.
                        }
                    })
                    .collect()
            })
            .collect(),
        periodic_u: false,
        periodic_v: false,
    }
}
fn shared(a: &Surface, b: &Surface) -> crate::source_shared_edge::SharedEdge {
    let pc = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
    let world = Curve {
        degree: a.degree_u,
        knots: a.knots_u.clone(),
        control_points: a.control_points.iter().map(|r| r[0].clone()).collect(),
        weights: a.weights.iter().map(|r| r[0]).collect(),
        periodic: false,
    };
    let fa = Fragment::new(a, &pc, Endpoint::Parameter(0.), Endpoint::Parameter(1.)).unwrap();
    let fb = Fragment::new(b, &pc, Endpoint::Parameter(1.), Endpoint::Parameter(0.)).unwrap();
    let r =
        crate::source_shared_edge::qualify(&world, [&fa, &fb], [false, false], 100000000).unwrap();
    r.edge.expect(r.reason)
}
fn limits() -> CurvatureLimits {
    CurvatureLimits {
        normal_distance: 0.01,
        curvature_distance: 0.05,
        cells: 100000,
        curve_spans: 100000,
        shape_cells: 100000,
    }
}
fn plane(s: &Surface) -> normals::PlaneLimit<'_> {
    let r = normals::prepare_plane_limit(s, Boundary::UMin, 100000000).unwrap();
    r.certificate.expect(r.reason)
}
#[test]
fn nonzero_polar_curvature_preserves_coherent_opposite_orientation_and_all_budgets() {
    for weighted in [false, true] {
        let a = bowl(1., weighted, 2.);
        let b = bowl(-1., weighted, 2.);
        let edge = shared(&a, &b);
        let before = edge.definition();
        let pa = plane(&a);
        let pb = plane(&b);
        let r = qualify_polar_curvature(&edge, [&pa, &pb], limits(), 100000000, 100000000).unwrap();
        let seam = r.seam.expect(r.reason);
        assert_eq!(seam.orientation(), Orientation::Opposite);
        assert!(seam.normal_upper() <= 0.01 && seam.curvature_upper() <= 0.05);
        assert_eq!(seam.edge().definition(), before);
        assert!(
            qualify_polar_curvature(&edge, [&pa, &pb], limits(), r.products - 1, 100000000)
                .unwrap()
                .seam
                .is_none()
        );
        assert!(
            qualify_polar_curvature(&edge, [&pa, &pb], limits(), 100000000, r.coefficients - 1)
                .unwrap()
                .seam
                .is_none()
        );
        let mut low = limits();
        low.cells = r.cells - 1;
        assert!(
            qualify_polar_curvature(&edge, [&pa, &pb], low, 100000000, 100000000)
                .unwrap()
                .seam
                .is_none()
        );
        let mut low = limits();
        low.curve_spans = r.curve_spans - 1;
        assert!(
            qualify_polar_curvature(&edge, [&pa, &pb], low, 100000000, 100000000)
                .unwrap()
                .seam
                .is_none()
        );
        let mut changed = a.clone();
        for p in changed.control_points.iter_mut().flatten() {
            p[0] += 1.;
        }
        let wrong = plane(&changed);
        assert!(
            qualify_polar_curvature(&edge, [&wrong, &pb], limits(), 100000000, 100000000).is_err()
        );
        eprintln!(
            "polar-nonzero G2 metrics: weighted={} cells={} products={} coefficients={}",
            weighted, r.cells, r.products, r.coefficients
        );
    }
}
#[test]
fn identical_polar_tangent_fields_do_not_hide_a_curvature_jump() {
    let a = bowl(1., false, 2.);
    let b = bowl(-1., false, 3.);
    let edge = shared(&a, &b);
    let pa = plane(&a);
    let pb = plane(&b);
    let g1 = crate::source_seam_tangency::qualify_polar_tangency(
        &edge,
        [&pa, &pb],
        crate::source_seam_tangency::Limits {
            max_sine_squared: 1e-4,
            cells: 100000,
            curve_spans: 100000,
            normal_spans: 100000,
        },
        100000000,
    )
    .unwrap();
    assert!(g1.seam.is_some());
    let g2 = qualify_polar_curvature(&edge, [&pa, &pb], limits(), 100000000, 100000000).unwrap();
    assert!(g2.seam.is_none());
    assert_eq!(g2.reason, "source-seam-curvature-envelope-separated");
}
#[test]
fn matching_polar_endpoint_shapes_do_not_admit_an_interior_curvature_jump() {
    let source = |sign: f64, perturb: bool| Surface {
        degree_u: 4,
        degree_v: 2,
        knots_u: [vec![0.; 5], vec![1.; 5]].concat(),
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..5)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        let height = if i < 2 {
                            0.
                        } else {
                            (i * (i - 1)) as f64 / 12.
                        };
                        vec![
                            i as f64 / 4.,
                            sign * (i * j) as f64 / 8.,
                            height * if j == 2 { 3. } else { 1. }
                                + if perturb && i == 3 && j == 2 {
                                    0.25
                                } else {
                                    0.
                                },
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 5],
        periodic_u: false,
        periodic_v: false,
    };
    let a = source(1., false);
    let b = source(-1., true);
    let edge = shared(&a, &b);
    let pa = plane(&a);
    let pb = plane(&b);
    let fa = pa
        .prepare_graph_shape(2, 100000000)
        .unwrap()
        .certificate
        .unwrap();
    let fb = pb
        .prepare_graph_shape(2, 100000000)
        .unwrap()
        .certificate
        .unwrap();
    for u in [0., 1.] {
        let r = normals::compare_polar_shape_rectangles(
            [&fa, &fb],
            [[[u, u], [0., 0.]]; 2],
            100000000,
            2,
        )
        .unwrap();
        assert!(r.bounds.unwrap()[1].tensor[1] < 1e-6);
    }
    let r = qualify_polar_curvature(&edge, [&pa, &pb], limits(), 100000000, 100000000).unwrap();
    assert!(r.seam.is_none());
    assert_eq!(r.reason, "source-seam-curvature-envelope-separated");
}
#[test]
fn changing_projection_uses_original_regular_jets_without_cropping_the_far_end() {
    let source = |sign: f64| Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..2)
                    .map(|j| {
                        vec![
                            [0., 0.5, 0.5][i],
                            sign * (i * j) as f64 / 2.,
                            f64::from(i == 2),
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let a = source(1.);
    let b = source(-1.);
    let edge = shared(&a, &b);
    let pa = plane(&a);
    let pb = plane(&b);
    let r = qualify_polar_curvature(&edge, [&pa, &pb], limits(), 100000000, 100000000).unwrap();
    assert!(
        r.seam.is_some(),
        "{} at {:?}",
        r.reason,
        r.uncertain_canonical
    );
    assert!(r.regular_queries > 0);
    let mut low = limits();
    low.shape_cells = r.regular_queries - 1;
    assert!(
        qualify_polar_curvature(&edge, [&pa, &pb], low, 100000000, 100000000)
            .unwrap()
            .seam
            .is_none()
    );
    eprintln!(
        "polar-changing-projection G2 metrics: cells={} regular_queries={} coefficients={}",
        r.cells, r.regular_queries, r.coefficients
    );
}
