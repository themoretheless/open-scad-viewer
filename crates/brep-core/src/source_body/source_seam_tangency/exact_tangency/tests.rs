use super::*;
use crate::source_boundary_fragment::{Endpoint, Fragment};
use nurbs_core::{curve::Curve, surface::Surface};
fn patch(sign: f64, curvature: f64) -> Surface {
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
                            sign * j as f64 / 2.,
                            if j == 2 { curvature } else { 0. },
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
    let world = Curve {
        degree: a.degree_u,
        knots: a.knots_u.clone(),
        control_points: a.control_points.iter().map(|r| r[0].clone()).collect(),
        weights: a.weights.iter().map(|r| r[0]).collect(),
        periodic: false,
    };
    let fa = Fragment::new(a, &pc, Endpoint::Parameter(0.), Endpoint::Parameter(1.)).unwrap();
    let fb = Fragment::new(b, &pc, Endpoint::Parameter(1.), Endpoint::Parameter(0.)).unwrap();
    let r = crate::source_shared_edge::qualify(&world, [&fa, &fb], [false; 2], 100000000).unwrap();
    r.edge.expect(r.reason)
}
fn limits() -> Limits {
    Limits {
        max_sine_squared: 0.,
        cells: 1000,
        curve_spans: 4000,
        normal_spans: 4000,
    }
}
#[test]
fn exact_regular_g1_keeps_hessian_jumps_under_affine_frames_and_weight_scales() {
    let frames = [
        [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        [[2., 0., 0.], [0., 0.5, 0.], [0., 0., 4.]],
        [[1., 0.25, 0.5], [0., 2., 0.25], [0.5, 0., 1.]],
        [[-1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
    ];
    for frame in frames {
        for weights in [[1., 1.], [2., 0.5]] {
            let mut surfaces = [patch(1., 1.), patch(-1., 2.)];
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
            let r = qualify_tangency_exact(&edge, limits(), 2, 100000000).unwrap();
            let seam = r.seam.expect(r.reason);
            assert_eq!(seam.sine_squared_bounds(), [0.; 2]);
            assert_eq!(seam.edge().definition(), before);
            assert!(r.exact_work > 0);
            assert_eq!(r.cells, 1);
            assert_eq!(r.normal_spans, 2);
            let second = super::super::qualify_curvature_exact(
                &edge,
                super::super::CurvatureLimits {
                    normal_distance: 0.,
                    curvature_distance: 0.,
                    cells: 1000,
                    curve_spans: 4000,
                    shape_cells: 4000,
                },
                2,
                100000000,
            )
            .unwrap();
            assert!(second.seam.is_none());
            assert_eq!(
                second.reason,
                "exact-original-meridian-graph-jets-different"
            );
            let tight = || Limits {
                max_sine_squared: 0.,
                cells: r.cells,
                curve_spans: r.curve_spans,
                normal_spans: r.normal_spans,
            };
            assert!(
                qualify_tangency_exact(&edge, tight(), 2, r.exact_work)
                    .unwrap()
                    .seam
                    .is_some()
            );
            for dim in 0..4 {
                let mut l = tight();
                let mut work = r.exact_work;
                match dim {
                    0 => work -= 1,
                    1 => l.cells -= 1,
                    2 => l.curve_spans -= 1,
                    _ => l.normal_spans -= 1,
                }
                if let Ok(short) = qualify_tangency_exact(&edge, l, 2, work) {
                    assert!(short.seam.is_none());
                }
            }
            eprintln!(
                "exact regular G1 Hessian-jump: work={} cells={} normals={}",
                r.exact_work, r.cells, r.normal_spans
            );
        }
    }
}
#[test]
fn exact_regular_g1_projection_zero_does_not_authorize_a_singular_endpoint() {
    let mut surfaces = [patch(1., 1.), patch(-1., 2.)];
    for s in &mut surfaces {
        for i in 0..3 {
            for j in 0..3 {
                s.control_points[i][j][0] = [0., 0.5, 0.][i];
                s.control_points[i][j][2] += i as f64 / 2.;
            }
        }
    }
    let e = edge(&surfaces[0], &surfaces[1]);
    let r = qualify_tangency_exact(&e, limits(), 2, 100000000).unwrap();
    assert!(r.seam.is_some(), "{}", r.reason);
    assert_eq!(r.cells, 1);
    assert_eq!(r.normal_spans, 2);
    eprintln!(
        "projection-changing exact G1: work={} cells={} normals={}",
        r.exact_work, r.cells, r.normal_spans
    );
    let mut a = patch(1., 1.);
    let mut b = patch(-1., 2.);
    for s in [&mut a, &mut b] {
        for p in &mut s.control_points[1] {
            p[0] = 0.;
        }
    }
    let e = edge(&a, &b);
    let (equal, _, _) = identity(&e, 2, 100000000).unwrap();
    assert!(equal);
    let mut l = limits();
    l.cells = 32;
    l.normal_spans = 128;
    let r = qualify_tangency_exact(&e, l, 2, 100000000).unwrap();
    assert!(r.seam.is_none());
    assert!(r.cells <= 32 && r.normal_spans <= 128 && r.uncertain_canonical.is_some());
}
#[test]
fn exact_regular_g1_refuses_real_derivative_jumps_and_identically_zero_projections() {
    let a = patch(1., 1.);
    let mut b = patch(-1., 2.);
    b.control_points[1][1][2] = 0.125;
    let e = edge(&a, &b);
    let r = qualify_tangency_exact(&e, limits(), 2, 100000000).unwrap();
    assert!(r.seam.is_none());
    assert_eq!(r.reason, "exact-original-meridian-gradients-different");
    assert_eq!(r.cells, 0);
    let a = patch(1., 0.);
    let b = patch(-1., 0.);
    let e = edge(&a, &b);
    assert!(
        qualify_tangency_exact(&e, limits(), 0, 100000000)
            .unwrap()
            .seam
            .is_none()
    );
    assert!(qualify_tangency_exact(&e, limits(), 3, 100000000).is_err());
    assert!(qualify_tangency_exact(&e, limits(), 2, 0).is_err());
}
