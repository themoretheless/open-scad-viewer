use brep_core::rational_section_loft;
use nurbs_core::{curve::Curve, surface::Axis};
fn circles(z: f64, scale: f64) -> Vec<Vec<Curve>> {
    vec![
        vec![nurbs_core::primitives::circle([0., 0., z], [0., 0., 1.], 3. * scale).unwrap()],
        vec![
            nurbs_core::primitives::circle([0., 0., z], [0., 0., 1.], scale)
                .unwrap()
                .reverse()
                .unwrap(),
        ],
    ]
}
#[test]
fn rational_hollow_loft_keeps_curved_walls_caps_and_shared_edges() {
    let model =
        rational_section_loft(&[circles(0., 1.), circles(5., 1.5), circles(10., 2.)]).unwrap();
    let report = model.validate().unwrap();
    assert_eq!(report.boundary_edge_count, 0);
    assert_eq!(model.faces.len(), 18);
    assert_eq!(model.bodies.len(), 1);
    assert!(model.shells[0].closed);
    assert_eq!(
        model.faces.iter().filter(|f| !f.holes.is_empty()).count(),
        2
    );
    for face in &model.faces[..16] {
        assert_eq!(face.surface.degree_u, 2);
        assert!(face.surface.weights.iter().flatten().any(|w| *w != 1.));
        for (u, v) in [(0.13, 0.27), (0.61, 0.83)] {
            let p = face.surface.evaluate(u, v).unwrap().point;
            let radius = p[0].hypot(p[1]);
            let factor = 1. + p[2] / 10.;
            assert!((radius / factor - 1.).abs() < 1e-12 || (radius / factor - 3.).abs() < 1e-12);
        }
    }
    // Authoritative edge/pcurve agreement is independently checked by validate;
    // every undirected edge must have two incident coedges after capping.
    for edge in 0..model.edges.len() {
        assert_eq!(
            model
                .loops
                .iter()
                .flat_map(|l| &l.coedges)
                .filter(|c| c.edge == edge)
                .count(),
            2
        );
    }
    let cap = &model.faces[16];
    assert_eq!(cap.holes.len(), 1);
    let outer = &model.loops[cap.outer];
    assert!(
        outer
            .coedges
            .iter()
            .any(|c| model.edges[c.edge].curve.degree == 2)
    );
    assert_eq!(
        model.faces[0].surface.iso(Axis::V, 0.).unwrap().weights,
        vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.]
    );
}
#[test]
fn invalid_holes_joins_correspondence_and_nonplanar_caps_refuse() {
    let start = circles(0., 1.);
    let end = circles(10., 1.);
    let mut wrong = start.clone();
    wrong[1][0] = wrong[1][0].reverse().unwrap();
    assert!(rational_section_loft(&[wrong, end.clone()]).is_err());
    let mut outside = start.clone();
    for p in &mut outside[1][0].control_points {
        p[0] += 10.;
    }
    assert!(rational_section_loft(&[outside, end.clone()]).is_err());
    let mut broken = start.clone();
    broken[0][0].control_points[0][0] += 0.01;
    assert!(rational_section_loft(&[broken, end.clone()]).is_err());
    let mut nonplanar = start.clone();
    nonplanar[0][0].control_points[1][2] = 0.1;
    assert!(rational_section_loft(&[nonplanar, end.clone()]).is_err());
    let mut weight = end.clone();
    weight[0][0].weights[1] = -1.;
    assert!(rational_section_loft(&[start.clone(), weight]).is_err());
    assert!(rational_section_loft(&[start]).is_err());
}
fn law(a: f64, b: f64) -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![a, 0., 0.], vec![b, 0., 0.]],
        weights: vec![1., 1.],
        periodic: false,
    }
}
#[test]
fn retained_hollow_rational_loft_step_roundtrip_preserves_topology_and_surfaces() {
    let model = rational_section_loft(&[circles(0., 1.), circles(10., 2.)]).unwrap();
    let (text, _, _) = cad_step::export_step_v5(&model).unwrap();
    assert!(text.contains("RATIONAL_B_SPLINE_SURFACE"));
    let (restored, _, _) = cad_step::import_step_v5(&text).unwrap();
    assert_eq!(restored.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(restored.faces.len(), model.faces.len());
    assert_eq!(restored.edges.len(), model.edges.len());
    assert_eq!(
        restored
            .faces
            .iter()
            .filter(|f| !f.holes.is_empty())
            .count(),
        2
    );
    for (i, (a, b)) in model.faces.iter().zip(&restored.faces).enumerate() {
        for (u, v) in [(0.13, 0.27), (0.61, 0.83)] {
            let original_u = if model.shells[0]
                .faces
                .iter()
                .find(|f| f.face == i)
                .unwrap()
                .reversed
            {
                a.surface.knots_u[a.surface.degree_u] + a.surface.knots_u[a.surface.control_points.len()] - u
            } else {
                u
            };
            let p = a.surface.evaluate(original_u, v).unwrap().point;
            let q = b.surface.evaluate(u, v).unwrap().point;
            assert!(p.iter().zip(q).all(|(x, y)| (x - y).abs() < 1e-10), "STEP changed face {i}");
        }
    }
}
#[test]
fn rational_topology_supports_more_than_sixteen_sections_without_refit() {
    let sections = (0..33)
        .map(|i| circles(i as f64, 1. + i as f64 / 32.))
        .collect::<Vec<_>>();
    let model = rational_section_loft(&sections).unwrap();
    assert_eq!(model.faces.len(), 258);
    assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
}

#[test]
fn reverse_travel_keeps_consistent_outward_shell_senses() {
    let up = rational_section_loft(&[circles(0., 1.), circles(10., 1.)]).unwrap();
    let down = rational_section_loft(&[circles(10., 1.), circles(0., 1.)]).unwrap();
    assert!(
        up.shells[0]
            .faces
            .iter()
            .zip(&down.shells[0].faces)
            .all(|(a, b)| a.reversed != b.reversed)
    );
    assert_eq!(down.validate().unwrap().boundary_edge_count, 0);
}

#[test]
fn natural_capped_loft_interpolates_middle_section_and_curved_shared_seams() {
    let sections = [circles(0., 1.), circles(5., 2.), circles(10., 1.)];
    let model = brep_core::natural_section_loft(&sections, &[0., 0.5, 1.]).unwrap();
    assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(model.bodies.len(), 1);
    assert!(model.shells[0].closed);
    assert_eq!(model.faces.len(), 10);
    for face in &model.faces[..8] {
        assert_eq!(face.surface.degree_v, 3);
        for u in [0., 0.17, 0.5, 0.83, 1.] {
            let p = face.surface.evaluate(u, 0.5).unwrap().point;
            assert!((p[2] - 5.).abs() < 1e-12);
            let r = p[0].hypot(p[1]);
            assert!((r - 6.).abs() < 1e-12 || (r - 2.).abs() < 1e-12);
        }
    }
    assert!(brep_core::natural_section_loft(&sections, &[0., 0., 1.]).is_err());
}

#[test]
fn capped_supplied_loft_patches_share_full_curved_seams() {
    let sections = [circles(0., 1.), circles(5., 2.), circles(10., 1.)];
    let prepared = sections
        .iter()
        .map(|s| {
            s.iter()
                .map(|ring| {
                    ring.iter()
                        .flat_map(|c| {
                            c.decompose()
                                .unwrap()
                                .into_iter()
                                .map(|p| p.definition().clone())
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let sides = prepared[0]
        .iter()
        .enumerate()
        .map(|(r, ring)| {
            ring.iter()
                .enumerate()
                .map(|(i, _)| {
                    let curves = prepared
                        .iter()
                        .map(|s| {
                            let mut c = s[r][i].clone();
                            let [a, b] = c.domain();
                            for k in &mut c.knots {
                                *k = (*k - a) / (b - a);
                            }
                            c
                        })
                        .collect::<Vec<_>>();
                    nurbs_core::natural_loft::interpolate(&curves, &[0., 0.5, 1.]).unwrap()
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let model = brep_core::capped_loft_surfaces(&sections[0], &sections[2], &sides).unwrap();
    assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(model.bodies.len(), 1);
    let mut broken = sides.clone();
    broken[0][0].control_points[0][0][2] += 1.;
    assert!(brep_core::capped_loft_surfaces(&sections[0], &sections[2], &broken).is_err());
}
#[test]
fn natural_capped_loft_step_roundtrip_and_independent_fixture() {
    let model = brep_core::natural_section_loft(
        &[circles(0., 1.), circles(5., 2.), circles(10., 1.)],
        &[0., 0.5, 1.],
    )
    .unwrap();
    let (text, _, _) = cad_step::export_step_v9(&model).unwrap();
    let restored = cad_step::import_step_v9(&text).unwrap().0;
    assert_eq!(restored.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(restored.bodies.len(), 1);
    assert_eq!(restored.faces.len(), model.faces.len());
    for (i, (a, b)) in model.faces.iter().zip(&restored.faces).enumerate() {
        let reverse = model.shells[0]
            .faces
            .iter()
            .find(|f| f.face == i)
            .unwrap()
            .reversed;
        for (u, v) in [(0.13, 0.27), (0.61, 0.83)] {
            let p = a
                .surface
                .evaluate(if reverse { a.surface.knots_u[a.surface.degree_u] + a.surface.knots_u[a.surface.control_points.len()] - u } else { u }, v)
                .unwrap()
                .point;
            let q = b.surface.evaluate(u, v).unwrap().point;
            assert!(p.iter().zip(q).all(|(x, y)| (x - y).abs() < 1e-9), "STEP changed face {i}");
        }
    }
    if let Ok(dir) = std::env::var("ADVANCED_LOFT_STEP_OUTPUT") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/natural-hollow-solid.step"), text).unwrap();
    }
}

#[test]
fn differing_section_weights_retain_rational_sections_and_closed_topology() {
    let mut sections = [circles(0., 1.), circles(5., 1.2), circles(10., 1.5)];
    for (j, section) in sections.iter_mut().enumerate() {
        for ring in section {
            // Change the conic shape, not just its homogeneous scale.
            ring[0].weights[1] *= 1. + j as f64 / 8.;
        }
    }
    let ruled = rational_section_loft(&sections).unwrap();
    assert_eq!(ruled.validate().unwrap().boundary_edge_count, 0);
    let natural = brep_core::natural_section_loft(&sections, &[0., 0.5, 1.]).unwrap();
    assert_eq!(natural.validate().unwrap().boundary_edge_count, 0);
    for (j, section) in sections.iter().enumerate() {
        let spans = section.iter().flat_map(|ring| ring[0].decompose().unwrap())
            .map(|s| s.definition().clone()).collect::<Vec<_>>();
        for (i, span) in spans.iter().enumerate() {
            for u in [0., 0.13, 0.5, 0.87, 1.] {
                let [a,b] = span.domain();
                let expected = span.evaluate(a + u * (b-a)).unwrap().point;
                let actual = natural.faces[i].surface.evaluate(u, j as f64 / 2.).unwrap().point;
                for k in 0..3 { assert!((actual[k]-expected[k]).abs() < 1e-11); }
            }
        }
    }
    let (step, _, _) = cad_step::export_step_v5(&natural).unwrap();
    assert!(step.contains("RATIONAL_B_SPLINE_SURFACE"));
}
