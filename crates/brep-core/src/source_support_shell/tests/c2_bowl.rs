use super::*;
use crate::source_seam_tangency::{self as smooth, Class, PolarBudgets, PoleStrata};
use nurbs_core::curve::Curve;
use std::collections::BTreeSet;
fn cardinal(p: &[f64]) -> [f64; 2] {
    if p[0].abs() > p[1].abs() {
        [p[0].signum(), 0.]
    } else {
        [0., p[1].signum()]
    }
}
fn model() -> crate::Model {
    let spans = crate::linear_canal::construct(
        [[0., 0., 0.], [0., 0., 8.]],
        [1., 1.],
        [1., 0., 0.],
        std::f64::consts::TAU,
    )
    .unwrap();
    let mut model = crate::linear_canal::to_capped_region(&spans, 1e-7).unwrap();
    for face in &mut model.faces {
        let old = face.surface.clone();
        let n = old.control_points.len();
        assert_eq!(old.degree_v, 2);
        assert_eq!(old.control_points[0].len(), 3);
        let pole = (0..2).find(|&e| {
            let r = &old.control_points[if e == 0 { 0 } else { n - 1 }];
            r.iter().all(|p| p == &r[0])
        });
        let ring = if pole == Some(0) { n - 1 } else { 0 };
        let a = cardinal(&old.control_points[ring][0]);
        let b = cardinal(&old.control_points[ring][2]);
        assert_eq!(a[0] * b[0] + a[1] * b[1], 0.);
        let angular = (0..3)
            .map(|j| {
                let v = j as f64 / 2.;
                [(1. - v) * a[0] + v * b[0], (1. - v) * a[1] + v * b[1]]
            })
            .collect::<Vec<_>>();
        let mut s = old.clone();
        s.weights = vec![vec![1.; 3]; n];
        if let Some(end) = pole {
            assert_eq!(old.degree_u, 2);
            let base = old.control_points[ring][0][2];
            for i in 0..n {
                let r = if end == 0 {
                    i as f64 / 2.
                } else {
                    1. - i as f64 / 2.
                };
                for j in 0..3 {
                    s.control_points[i][j] = vec![
                        r * angular[j][0],
                        r * angular[j][1],
                        base + if r == 1. { [1., 0., 1.][j] } else { 0. },
                    ];
                }
            }
        } else {
            for i in 0..n {
                let base = old.control_points[i][0][2];
                assert!(old.control_points[i].iter().all(|p| p[2] == base));
                for j in 0..3 {
                    s.control_points[i][j] =
                        vec![angular[j][0], angular[j][1], base + [1., 0., 1.][j]];
                }
            }
        }
        for p in s.control_points.iter_mut().flatten() {
            for x in p {
                if *x == 0. {
                    *x = 0.;
                }
            }
        }
        face.surface = s;
    }
    for vertex in &mut model.vertices {
        let old = vertex.point;
        let base = if old[2] < 4. { 0. } else { 8. };
        vertex.point = if old[0].hypot(old[1]) < 1e-8 {
            [0., 0., base]
        } else {
            let p = cardinal(&old);
            [p[0], p[1], base + 1.]
        };
    }
    let mut done = BTreeSet::new();
    for face in 0..model.faces.len() {
        let s = model.faces[face].surface.clone();
        let wire = model.faces[face].outer;
        for coedge in model.loops[wire].coedges.clone() {
            if !done.insert(coedge.edge) {
                continue;
            }
            let p = &coedge.pcurve;
            let first = &p.control_points[0];
            let last = p.control_points.last().unwrap();
            let fixed = if first[0] == last[0] { 0 } else { 1 };
            let at = first[fixed];
            let (degree, knots, points) = if fixed == 0 {
                let row = if at == s.knots_u[s.degree_u] {
                    0
                } else {
                    s.control_points.len() - 1
                };
                (s.degree_v, s.knots_v.clone(), s.control_points[row].clone())
            } else {
                let col = if at == s.knots_v[s.degree_v] {
                    0
                } else {
                    s.control_points[0].len() - 1
                };
                (
                    s.degree_u,
                    s.knots_u.clone(),
                    s.control_points.iter().map(|r| r[col].clone()).collect(),
                )
            };
            let start = model.vertices[model.edges[coedge.edge].vertices[0]]
                .point
                .to_vec();
            let mut points: Vec<Vec<f64>> = points;
            if points[0] != start {
                assert_eq!(points.last().unwrap(), &start);
                points.reverse();
            }
            model.edges[coedge.edge].curve = Curve {
                degree,
                knots,
                weights: vec![1.; points.len()],
                control_points: points,
                periodic: false,
            };
        }
    }
    model.validate().unwrap();
    model
}
fn pole_limits() -> crate::source_pole_tangency::PoleCurvatureLimits {
    use crate::source_pole_tangency::*;
    PoleCurvatureLimits {
        graphs: GraphLimits {
            charts: ProjectedChartLimits {
                transverse: TransverseLimits {
                    atlas: FanLimits {
                        inward_extent: 0.125,
                        subdivisions: 8,
                        exact_work: 100000000,
                        cells: 100000,
                    },
                    columns: 100000,
                },
                subdivisions: 8,
                cells: 100000,
            },
            boundary_steps: 100000,
        },
        jet_work: 100000000,
    }
}
#[test]
fn parabolic_closed_body_has_owned_nonzero_c2_pole_neighborhoods() {
    let model = model();
    let r = prepare_projective(&model, limits()).unwrap();
    let shell = r.shell.expect(r.reason);
    let r = crate::source_shell_geometry::qualify_projective(shell, geometry_limits()).unwrap();
    eprintln!(
        "parabolic C2 geometry: {} face={:?} pair={:?}",
        r.reason, r.uncertain_face, r.next_pair
    );
    let geometry = r.geometry.expect(r.reason);
    let r = crate::source_volume::qualify(
        geometry,
        crate::source_volume::Limits {
            axis: 2,
            origin: 0.,
            absolute_error: 0.25,
            tolerance_uv: 1e-8,
            cells: 50000,
            spans: 100000,
            domain_cells: 100000,
        },
    )
    .unwrap();
    let body = r.body.expect(r.reason);
    assert!(body.volume()[0] <= 16. && body.volume()[1] >= 16.);
    let faces = body
        .geometry()
        .shell()
        .poles()
        .iter()
        .map(|(a, _)| a.face)
        .collect::<BTreeSet<_>>();
    let classes = body
        .geometry()
        .shell()
        .uses()
        .iter()
        .map(|pair| {
            if pair.iter().all(|a| faces.contains(&a.face)) {
                Class::Curvature {
                    normal_distance: 0.,
                    curvature_distance: 0.,
                }
            } else {
                Class::C0
            }
        })
        .collect::<Vec<_>>();
    let r = smooth::qualify_boundary_with_exact_polar_curvature(
        &body,
        &classes,
        pole_limits(),
        smooth::Limits {
            max_sine_squared: 1e-10,
            cells: 10000,
            curve_spans: 10000,
            normal_spans: 10000,
        },
        PolarBudgets {
            normal_coefficients: 100000000,
            shape_products: 100000000,
            shape_coefficients: 100000000,
        },
        100000000,
    )
    .unwrap();
    let boundary = r.certificate.expect(r.reason);
    assert!(matches!(boundary.poles(), PoleStrata::FacewiseCurvature(_)));
    let r = smooth::qualify_c2_pole_neighborhoods(&boundary, 100000).unwrap();
    let atlas = r.certificate.expect(r.reason);
    assert_eq!(atlas.neighborhoods().len(), 2);
    for n in atlas.neighborhoods() {
        assert_eq!(n.dropped_axis(), 2);
        assert!(n.projected_radius_lower() > 0.);
        for (b, v) in n.coefficient_bounds().into_iter().zip([0., 0., 2., 0., 2.]) {
            assert!(b[0] <= v && v <= b[1], "{b:?} versus {v}");
        }
    }
    eprintln!(
        "nonzero-pole-C2 metrics: neighborhoods={} steps={} volume={:?} radii={:?}",
        atlas.neighborhoods().len(),
        r.steps,
        body.volume(),
        atlas
            .neighborhoods()
            .iter()
            .map(|n| (n.vertex(), n.projected_radius_lower()))
            .collect::<Vec<_>>()
    );
}
