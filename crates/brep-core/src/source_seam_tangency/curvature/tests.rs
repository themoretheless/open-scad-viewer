use super::*;
use crate::source_boundary_fragment::{Endpoint, Fragment};
use nurbs_core::{curve::Curve, surface::Surface};
fn patch(y_sign: f64, curvature: [f64; 3]) -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..3)
            .map(|u| {
                (0..3)
                    .map(|v| {
                        vec![
                            u as f64 / 2.,
                            y_sign * v as f64 / 2.,
                            if v == 2 { curvature[u] } else { 0. },
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}
fn edge(a: &Surface, b: &Surface) -> crate::source_shared_edge::SharedEdge {
    let pc = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
    let world = Curve::from_polyline(vec![
        a.control_points[0][0].clone(),
        a.control_points.last().unwrap()[0].clone(),
    ])
    .unwrap();
    let fa = Fragment::new(a, &pc, Endpoint::Parameter(0.), Endpoint::Parameter(1.)).unwrap();
    let fb = Fragment::new(b, &pc, Endpoint::Parameter(1.), Endpoint::Parameter(0.)).unwrap();
    crate::source_shared_edge::qualify(&world, [&fa, &fb], [false, false], 100000000)
        .unwrap()
        .edge
        .unwrap()
}
fn limits() -> CurvatureLimits {
    CurvatureLimits {
        normal_distance: 1e-7,
        curvature_distance: 1e-6,
        cells: 1000,
        curve_spans: 4000,
        shape_cells: 4000,
    }
}
#[test]
fn nonzero_second_jets_keep_one_coherent_opposite_orientation() {
    let a = patch(1., [1.; 3]);
    let b = patch(-1., [1.; 3]);
    let edge = edge(&a, &b);
    let before = edge.definition();
    let r = qualify_curvature(&edge, limits()).unwrap();
    let seam = r.seam.expect(r.reason);
    assert_eq!(seam.orientation(), Orientation::Opposite);
    assert!(seam.normal_upper() <= seam.normal_tolerance());
    assert!(seam.curvature_upper() <= seam.curvature_tolerance());
    assert_eq!(seam.edge().definition(), before);
    // The curvature tensor is nonzero; reversing only its sign, or only the
    // normal, cannot establish this physical G2 seam.
    let q = [[[0., 1.], [0., 0.]]; 2];
    let differences = surface_shape_operator::compare_rectangles([&a, &b], q, 2)
        .unwrap()
        .bounds
        .unwrap();
    assert!(differences[0].normal[0] > 1.);
    assert!(differences[0].tensor[0] > 3.);
    assert!(differences[1].tensor[1] < 1e-6);
}
#[test]
fn matching_tangent_planes_do_not_hide_a_curvature_jump() {
    let a = patch(1., [0.; 3]);
    let b = patch(-1., [1.; 3]);
    let edge = edge(&a, &b);
    assert!(
        crate::source_seam_tangency::qualify(
            &edge,
            crate::source_seam_tangency::Limits {
                max_sine_squared: 1e-9,
                cells: 1000,
                curve_spans: 4000,
                normal_spans: 4000
            }
        )
        .unwrap()
        .seam
        .is_some()
    );
    let r = qualify_curvature(&edge, limits()).unwrap();
    assert!(r.seam.is_none());
    assert_eq!(r.reason, "source-seam-curvature-envelope-separated");
}
#[test]
fn matching_endpoint_curvatures_cannot_admit_an_interior_jump() {
    let a = patch(1., [0.; 3]);
    let b = patch(-1., [0., 1., 0.]);
    let edge = edge(&a, &b);
    for x in [0., 1.] {
        let q = [[[x, x], [0., 0.]]; 2];
        let r = surface_shape_operator::compare_rectangles([&a, &b], q, 2).unwrap();
        assert!(r.bounds.unwrap()[1].tensor[1] < 1e-6);
    }
    let r = qualify_curvature(&edge, limits()).unwrap();
    assert!(r.seam.is_none());
    assert_eq!(r.reason, "source-seam-curvature-envelope-separated");
}
#[test]
fn each_budget_and_singular_endpoints_prevent_partial_curvature_certificates() {
    let a = patch(1., [1.; 3]);
    let b = patch(-1., [1.; 3]);
    let edge = edge(&a, &b);
    let baseline = qualify_curvature(&edge, limits()).unwrap();
    assert!(baseline.seam.is_some());
    for dimension in 0..3 {
        let mut short = limits();
        match dimension {
            0 => short.cells = baseline.cells - 1,
            1 => short.curve_spans = baseline.curve_spans - 1,
            _ => short.shape_cells = baseline.shape_cells - 1,
        };
        if let Ok(r) = qualify_curvature(&edge, short) {
            assert!(r.seam.is_none());
        }
    }
    let mut singular = a.clone();
    for row in &mut singular.control_points {
        row[1] = row[0].clone();
    }
    let singular = edge_for_singular(&singular, &b);
    assert!(
        qualify_curvature(&singular, limits())
            .unwrap()
            .seam
            .is_none()
    );
}
fn edge_for_singular(a: &Surface, b: &Surface) -> crate::source_shared_edge::SharedEdge {
    edge(a, b)
}

#[test]
fn authored_affine_frames_and_independent_positive_weight_scales_keep_g2() {
    let frames = [
        [[2., 0., 0.], [0., 0.5, 0.], [0., 0., 4.]],
        [[1., 0., 0.], [0., 1., 0.], [0., 0.25, 1.]],
        [[-1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        [[1., 0.25, 0.5], [0., 2., 0.25], [0.5, 0., 1.]],
    ];
    for frame in frames {
        for weights in [[1., 1.], [2., 0.5]] {
            let mut surfaces = [patch(1., [1.; 3]), patch(-1., [1.; 3])];
            for (side, surface) in surfaces.iter_mut().enumerate() {
                for point in surface.control_points.iter_mut().flatten() {
                    let old = point.clone();
                    *point = (0..3)
                        .map(|axis| {
                            [8., -4., 2.][axis]
                                + (0..3).map(|k| frame[axis][k] * old[k]).sum::<f64>()
                        })
                        .collect();
                }
                for row in &mut surface.weights {
                    row.fill(weights[side]);
                }
            }
            let source = edge(&surfaces[0], &surfaces[1]);
            let before = source.definition();
            let report = qualify_curvature(&source, limits()).unwrap();
            let seam = report.seam.expect(report.reason);
            assert_eq!(seam.orientation(), Orientation::Opposite);
            assert_eq!(seam.edge().definition(), before);
            assert!(seam.curvature_upper() <= 1e-6);
        }
    }
}

#[test]
fn normal_and_curvature_cannot_choose_different_orientation_branches() {
    let a = patch(1., [1.; 3]);
    let b = patch(-1., [-1.; 3]);
    let source = edge(&a, &b);
    let q = [[[0., 1.], [0., 0.]]; 2];
    let bounds = surface_shape_operator::compare_rectangles([&a, &b], q, 2)
        .unwrap()
        .bounds
        .unwrap();
    // Each quantity matches under a different sign, which is not geometric G2.
    assert!(bounds[0].tensor[1] < 1e-6);
    assert!(bounds[1].normal[1] < 1e-7);
    let report = qualify_curvature(&source, limits()).unwrap();
    assert!(report.seam.is_none());
    assert_eq!(report.reason, "source-seam-curvature-envelope-separated");
}

#[test]
fn exact_regular_nonzero_jets_use_complete_native_shapes_and_one_orientation() {
    let frames = [
        [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        [[2., 0., 0.], [0., 0.5, 0.], [0., 0., 4.]],
        [[1., 0.25, 0.5], [0., 2., 0.25], [0.5, 0., 1.]],
        [[-1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
    ];
    for frame in frames {
        for weights in [[1., 1.], [2., 0.5]] {
            let mut surfaces = [patch(1., [1.; 3]), patch(-1., [1.; 3])];
            for (side, s) in surfaces.iter_mut().enumerate() {
                for p in s.control_points.iter_mut().flatten() {
                    let old = p.clone();
                    *p = (0..3)
                        .map(|i| {
                            [8., -4., 2.][i] + (0..3).map(|j| frame[i][j] * old[j]).sum::<f64>()
                        })
                        .collect();
                }
                for row in &mut s.weights {
                    row.fill(weights[side]);
                }
            }
            let edge = edge(&surfaces[0], &surfaces[1]);
            let before = edge.definition();
            let mut l = limits();
            l.normal_distance = 0.;
            l.curvature_distance = 0.;
            let r = qualify_curvature_exact(&edge, l, 2, 100000000).unwrap();
            let seam = r.seam.expect(r.reason);
            assert!(seam.is_exact());
            assert_eq!(seam.orientation(), Orientation::Opposite);
            assert_eq!(seam.normal_upper(), 0.);
            assert_eq!(seam.curvature_upper(), 0.);
            assert_eq!(seam.normal_tolerance(), 0.);
            assert_eq!(seam.curvature_tolerance(), 0.);
            assert_eq!(seam.edge().definition(), before);
            assert!(r.exact_work > 0 && r.cells >= 2 && r.shape_cells >= 4);
            for dim in 0..4 {
                let mut l = limits();
                l.normal_distance = 0.;
                l.curvature_distance = 0.;
                let mut work = 100000000;
                match dim {
                    0 => work = r.exact_work - 1,
                    1 => l.cells = r.cells - 1,
                    2 => l.curve_spans = r.curve_spans - 1,
                    _ => l.shape_cells = r.shape_cells - 1,
                }
                assert!(
                    qualify_curvature_exact(&edge, l, 2, work)
                        .unwrap()
                        .seam
                        .is_none()
                );
            }
            eprintln!(
                "exact regular G2: weights={weights:?} work={} cells={} shape_cells={}",
                r.exact_work, r.cells, r.shape_cells
            );
        }
    }
}
#[test]
fn exact_regular_identity_never_hides_jumps_singular_ends_or_bad_projection() {
    let a = patch(1., [1.; 3]);
    for curvature in [[2.; 3], [1., 2., 1.], [-1.; 3]] {
        let b = patch(-1., curvature);
        let e = edge(&a, &b);
        let r = qualify_curvature_exact(&e, limits(), 2, 100000000).unwrap();
        assert!(r.seam.is_none());
        assert_eq!(r.reason, "exact-original-meridian-graph-jets-different");
    }
    let mut a = patch(1., [1.; 3]);
    let mut b = patch(-1., [1.; 3]);
    // Both represent z=y² and match exact graph jets on the dense regular
    // locus, but their x=u² parameterization collapses rank at one endpoint.
    for s in [&mut a, &mut b] {
        for p in &mut s.control_points[1] {
            p[0] = 0.;
        }
    }
    let pc = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
    let world = Curve {
        degree: 2,
        knots: a.knots_u.clone(),
        control_points: a.control_points.iter().map(|row| row[0].clone()).collect(),
        weights: vec![1.; 3],
        periodic: false,
    };
    let fa = Fragment::new(&a, &pc, Endpoint::Parameter(0.), Endpoint::Parameter(1.)).unwrap();
    let fb = Fragment::new(&b, &pc, Endpoint::Parameter(1.), Endpoint::Parameter(0.)).unwrap();
    let r = crate::source_shared_edge::qualify(&world, [&fa, &fb], [false; 2], 100000000).unwrap();
    let e = r.edge.expect(r.reason);
    let maps = [
        e.exact_parameter_curve(0).unwrap(),
        e.exact_parameter_curve(1).unwrap(),
    ];
    assert!(
        nurbs_core::factored_surface_normals::prepare_exact_meridian_jets(
            [&a, &b],
            [maps[0].0, maps[1].0],
            [maps[0].1, maps[1].1],
            [maps[0].2, maps[1].2],
            2,
            100000000
        )
        .unwrap()
        .certificate
        .is_some()
    );
    assert!(
        qualify_curvature_exact(&e, limits(), 2, 100000000)
            .unwrap()
            .seam
            .is_none()
    );
    let a = patch(1., [0.; 3]);
    let b = patch(-1., [0.; 3]);
    let e = edge(&a, &b);
    assert!(
        qualify_curvature_exact(&e, limits(), 0, 100000000)
            .unwrap()
            .seam
            .is_none()
    );
    assert!(qualify_curvature_exact(&e, limits(), 3, 100000000).is_err());
    assert!(qualify_curvature_exact(&e, limits(), 2, 0).is_err());
}
#[test]
fn exact_regular_g2_crosses_projection_zero_with_independent_native_regularity() {
    let make = |sign: f64| Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        vec![
                            [0., 0.5, 0.][i],
                            sign * j as f64 / 2.,
                            i as f64 / 2. + if j == 2 { 1. } else { 0. },
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let a = make(1.);
    let b = make(-1.);
    let pc = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
    let world = Curve {
        degree: 2,
        knots: a.knots_u.clone(),
        control_points: a.control_points.iter().map(|r| r[0].clone()).collect(),
        weights: vec![1.; 3],
        periodic: false,
    };
    let fa = Fragment::new(&a, &pc, Endpoint::Parameter(0.), Endpoint::Parameter(1.)).unwrap();
    let fb = Fragment::new(&b, &pc, Endpoint::Parameter(1.), Endpoint::Parameter(0.)).unwrap();
    let r = crate::source_shared_edge::qualify(&world, [&fa, &fb], [false; 2], 100000000).unwrap();
    let edge = r.edge.expect(r.reason);
    // XY projection folds at u=1/2. World S_u=(0,0,1), S_v=(0,±1,0)
    // remain independent there. Native geometry must independently cover it.
    let shape =
        surface_shape_operator::compare_rectangles([&a, &b], [[[0.5, 0.5], [0., 0.]]; 2], 2)
            .unwrap();
    assert!(shape.bounds.is_some());
    let mut l = limits();
    l.normal_distance = 0.;
    l.curvature_distance = 0.;
    let r = qualify_curvature_exact(&edge, l, 2, 100000000).unwrap();
    let seam = r.seam.expect(r.reason);
    assert!(seam.is_exact());
    assert_eq!(seam.orientation(), Orientation::Opposite);
    assert_eq!(seam.normal_upper(), 0.);
    assert_eq!(seam.curvature_upper(), 0.);
    assert!(r.shape_cells >= 4);
    eprintln!(
        "projection-changing exact regular G2: work={} cells={} shape_cells={}",
        r.exact_work, r.cells, r.shape_cells
    );
}
