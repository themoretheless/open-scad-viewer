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
