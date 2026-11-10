//! Original nonplanar grid body used to qualify ordinary material vertex stars.
use super::*;
use crate::source_seam_tangency::{self as seams, Class};

fn source_model() -> crate::Model {
    source_model_with_curvature_jump(false)
}
fn source_model_with_curvature_jump(jump: bool) -> crate::Model {
    let cube = crate::cuboid([0.; 3], [if jump { 6. } else { 1. }; 3]).unwrap();
    let mut polygons = Vec::new();
    for face in &cube.faces {
        for i in 0..2 {
            for j in 0..2 {
                polygons.push(
                    [[0, 0], [1, 0], [1, 1], [0, 1]]
                        .into_iter()
                        .map(|[a, b]| {
                            face.surface
                                .evaluate((i + a) as f64 / 2., (j + b) as f64 / 2.)
                                .unwrap()
                                .point
                                .as_slice()
                                .try_into()
                                .unwrap()
                        })
                        .collect(),
                );
            }
        }
    }
    let mut model = crate::operations::model_from_polygons(polygons, 1e-8).unwrap();
    // New authored dyadic source data. The original axis-aligned boundaries
    // remain linear under z += xy/8; the two filled cap graphs are nonplanar.
    // All source/trace/contact identities are freshly checked after this edit.
    let shear = |p: &mut [f64]| {
        p[2] += p[0] * p[1] / 8.;
    };
    for vertex in &mut model.vertices {
        shear(&mut vertex.point);
    }
    for edge in &mut model.edges {
        for p in &mut edge.curve.control_points {
            shear(p);
        }
    }
    for face in &mut model.faces {
        for p in face.surface.control_points.iter_mut().flatten() {
            shear(p);
        }
    }
    if jump {
        let old = &model.faces[4].surface;
        assert_eq!(&old.control_points[0][0][..2], &[0., 0.]);
        assert_eq!(&old.control_points[1][1][..2], &[3., 3.]);
        // On this top quadrant, the authored cubic has the same four boundary
        // traces. Its internal u=3/v=3 joins have zero value/gradient residual
        // but a NONZERO normal Hessian residual away from their endpoints.
        // B_1^3(u/3) B_1^3(v/3)/64 uses only dyadic authored controls.
        let mut s = old.clone();
        s.degree_u = 3;
        s.degree_v = 3;
        s.knots_u = vec![0., 0., 0., 0., 3., 3., 3., 3.];
        s.knots_v = s.knots_u.clone();
        s.control_points = (0..4)
            .map(|i| {
                (0..4)
                    .map(|j| {
                        let (x, y) = (i as f64, j as f64);
                        vec![
                            x,
                            y,
                            6. + x * y / 8. + if i == 1 && j == 1 { 1. / 64. } else { 0. },
                        ]
                    })
                    .collect()
            })
            .collect();
        s.weights = vec![vec![1.; 4]; 4];
        model.faces[4].surface = s;
    }
    model.validate().unwrap();
    model
}

#[test]
fn nonplanar_grid_body_owns_face_jets_and_oriented_exact_edge_classes() {
    let model = source_model();
    assert_eq!(model.faces.len(), 24);
    let r = prepare_projective(&model, limits()).unwrap();
    let shell = r.shell.expect(r.reason);
    let r = crate::source_shell_geometry::qualify_projective(shell, geometry_limits()).unwrap();
    eprintln!(
        "ordinary grid geometry: {} face={:?} pair={:?}",
        r.reason, r.uncertain_face, r.next_pair
    );
    let geometry = r.geometry.expect(r.reason);
    let r = crate::source_volume::qualify(
        geometry,
        crate::source_volume::Limits {
            axis: 2,
            origin: 0.,
            absolute_error: 0.01,
            tolerance_uv: 1e-8,
            cells: 100000,
            spans: 100000,
            domain_cells: 1000000,
        },
    )
    .unwrap();
    let body = r.body.expect(r.reason);
    assert!(body.volume()[0] <= 1. && body.volume()[1] >= 1.);
    let before = body.definition().unwrap();
    let r = crate::source_volume::regularity::qualify(&body, 100000).unwrap();
    let regular = r.certificate.expect(r.reason);
    let r =
        crate::source_volume::regularity::smooth::qualify(&regular, 2, 100000, 10000000).unwrap();
    let smooth = r
        .certificate
        .expect("All original nonplanar grid faces are C2");
    assert_eq!(smooth.faces().len(), 24);
    let shell = body.geometry().shell();
    let mut axes = Vec::new();
    let classes = shell
        .uses()
        .iter()
        .map(|pair| {
            let parent = pair[0].face / 4;
            axes.push([2, 2, 1, 0, 1, 0][parent]);
            if parent == pair[1].face / 4 {
                // This staging collection supplies regular coverage. The following
                // exact factory must prove ZERO gradient/Hessian differences.
                Class::Curvature {
                    normal_distance: 0.01,
                    curvature_distance: 0.1,
                }
            } else {
                Class::C0
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(classes.iter().filter(|c| **c != Class::C0).count(), 24);
    let r = seams::qualify_collection(
        &body,
        &classes,
        seams::Limits {
            max_sine_squared: 0.01,
            cells: 100000,
            curve_spans: 100000,
            normal_spans: 100000,
        },
    )
    .unwrap();
    let collection = r.certificate.expect(r.reason);
    let r = seams::qualify_exact_smooth_collection(
        &collection,
        &axes,
        seams::ExactSmoothLimits {
            incidence_steps: 1000000,
            exact_work: 100000000,
            cells: 100000,
            curve_spans: 100000,
            source_spans: 100000,
        },
    )
    .unwrap();
    let exact = r.certificate.expect(r.reason);
    assert_eq!(exact.curvature_seams().len(), 24);
    assert!(
        exact
            .curvature_seams()
            .iter()
            .all(|(_, s)| s.is_exact() && s.normal_upper() == 0. && s.curvature_upper() == 0.)
    );
    let r = seams::material_orientation::qualify(
        &exact,
        seams::material_orientation::Limits {
            steps: 1000000,
            curve_spans: 100000,
            normal_spans: 100000,
        },
    )
    .unwrap();
    let oriented = r.certificate.expect(r.reason);
    assert_eq!(oriented.edges().len(), 24);
    assert_eq!(body.geometry().topology().links.len(), 26);
    let smooth_vertices = body
        .geometry()
        .topology()
        .links
        .iter()
        .filter(|link| link.cycle.iter().all(|n| classes[n.edge] != Class::C0))
        .count();
    assert_eq!(smooth_vertices, 6);
    let vertex_axes = body
        .geometry()
        .topology()
        .links
        .iter()
        .map(|link| {
            let parent = shell.uses()[link.cycle[0].edge][0].face / 4;
            [2, 2, 1, 0, 1, 0][parent]
        })
        .collect::<Vec<_>>();
    let r = seams::vertices::qualify(
        &smooth,
        &oriented,
        &vertex_axes,
        1e-8,
        seams::vertices::Limits {
            steps: 1000000,
            normal_spans: 100000,
            domain_cells: 100000,
            exact_work: 100000000,
        },
    )
    .unwrap();
    eprintln!(
        "ordinary grid graph stars: {} vertex={:?} steps={} normal={} domain={} exact={}",
        r.reason, r.uncertain_vertex, r.steps, r.normal_spans, r.domain_cells, r.exact_work
    );
    let tight = seams::vertices::Limits {
        steps: r.steps,
        normal_spans: r.normal_spans,
        domain_cells: r.domain_cells,
        exact_work: r.exact_work,
    };
    let atlas = r.certificate.expect(r.reason);
    assert!(std::ptr::eq(atlas.body(), &body));
    assert_eq!(atlas.face_order(), 2);
    assert_eq!(atlas.edge_classes(), classes);
    assert_eq!(atlas.vertices().len(), 26);
    assert_eq!(
        atlas
            .vertices()
            .iter()
            .filter(|v| matches!(v, seams::vertices::Vertex::C0 { .. }))
            .count(),
        20
    );
    let stars = atlas
        .vertices()
        .iter()
        .filter_map(|v| match v {
            seams::vertices::Vertex::Smooth(n) => Some(n),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(stars.len(), 6);
    assert!(
        stars
            .iter()
            .all(|n| n.order() == 2 && n.collars().len() == 4)
    );
    assert!(
        seams::vertices::qualify(&smooth, &oriented, &vertex_axes, 1e-8, tight)
            .unwrap()
            .certificate
            .is_some()
    );
    for dim in 0..4 {
        let mut l = tight;
        match dim {
            0 => l.steps -= 1,
            1 => l.normal_spans -= 1,
            2 => l.domain_cells -= 1,
            _ => l.exact_work -= 1,
        }
        assert!(
            seams::vertices::qualify(&smooth, &oriented, &vertex_axes, 1e-8, l)
                .unwrap()
                .certificate
                .is_none()
        );
    }
    assert!(
        seams::vertices::qualify(
            &smooth,
            &oriented,
            &vertex_axes[..vertex_axes.len() - 1],
            1e-8,
            tight
        )
        .is_err()
    );
    let mut invalid = vertex_axes.clone();
    invalid[0] = 3;
    assert!(seams::vertices::qualify(&smooth, &oriented, &invalid, 1e-8, tight).is_err());
    let other_model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    let other_shell = prepare_projective(&other_model, limits())
        .unwrap()
        .shell
        .unwrap();
    let other_geometry =
        crate::source_shell_geometry::qualify_projective(other_shell, geometry_limits())
            .unwrap()
            .geometry
            .unwrap();
    let other = crate::source_volume::qualify(
        other_geometry,
        crate::source_volume::Limits {
            axis: 2,
            origin: 0.,
            absolute_error: 0.01,
            tolerance_uv: 1e-8,
            cells: 100000,
            spans: 100000,
            domain_cells: 1000000,
        },
    )
    .unwrap()
    .body
    .unwrap();
    let other_rank = crate::source_volume::regularity::qualify(&other, 100000)
        .unwrap()
        .certificate
        .unwrap();
    let other_faces =
        crate::source_volume::regularity::smooth::qualify(&other_rank, 2, 100000, 1000000)
            .unwrap()
            .certificate
            .unwrap();
    let other_axes = vec![2; other.geometry().topology().links.len()];
    assert!(seams::vertices::qualify(&other_faces, &oriented, &other_axes, 1e-8, tight).is_err());
    assert_eq!(body.definition().unwrap(), before);
}

#[test]
fn ordinary_vertex_has_g1_graph_with_a_proved_interior_curvature_jump() {
    let model = source_model_with_curvature_jump(true);
    let shell = prepare_projective(&model, limits()).unwrap().shell.unwrap();
    let r = crate::source_shell_geometry::qualify_projective(shell, geometry_limits()).unwrap();
    eprintln!(
        "G1 grid geometry: {} face={:?} pair={:?}",
        r.reason, r.uncertain_face, r.next_pair
    );
    let geometry = r.geometry.expect(r.reason);
    let r = crate::source_volume::qualify(
        geometry,
        crate::source_volume::Limits {
            axis: 2,
            origin: 0.,
            absolute_error: 0.01,
            tolerance_uv: 1e-8,
            cells: 100000,
            spans: 100000,
            domain_cells: 1000000,
        },
    )
    .unwrap();
    let body = r.body.expect(r.reason);
    let expected = 216. + 9. / 1024.;
    assert!(body.volume()[0] <= expected && body.volume()[1] >= expected);
    let before = body.definition().unwrap();
    let r = crate::source_volume::regularity::qualify(&body, 100000).unwrap();
    let rank = r.certificate.expect(r.reason);
    let smooth = crate::source_volume::regularity::smooth::qualify(&rank, 2, 100000, 10000000)
        .unwrap()
        .certificate
        .expect("C2 original mixed-degree faces");
    let shell = body.geometry().shell();
    let mut axes = Vec::new();
    let mut jumps = Vec::new();
    let classes = shell
        .uses()
        .iter()
        .enumerate()
        .map(|(index, pair)| {
            let parent = pair[0].face / 4;
            axes.push([2, 2, 1, 0, 1, 0][parent]);
            if parent != pair[1].face / 4 {
                Class::C0
            } else if parent == 1 && pair.iter().any(|a| a.face == 4) {
                jumps.push(index);
                Class::TangentPlanes
            } else {
                Class::Curvature {
                    normal_distance: 0.01,
                    curvature_distance: 0.1,
                }
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(jumps.len(), 2);
    let r = seams::qualify_collection(
        &body,
        &classes,
        seams::Limits {
            max_sine_squared: 0.01,
            cells: 100000,
            curve_spans: 100000,
            normal_spans: 100000,
        },
    )
    .unwrap();
    let collection = r.certificate.expect(r.reason);
    let r = seams::qualify_exact_smooth_collection(
        &collection,
        &axes,
        seams::ExactSmoothLimits {
            incidence_steps: 1000000,
            exact_work: 100000000,
            cells: 100000,
            curve_spans: 100000,
            source_spans: 100000,
        },
    )
    .unwrap();
    let exact = r.certificate.expect(r.reason);
    assert_eq!(exact.tangent_seams().len(), 2);
    assert_eq!(exact.curvature_seams().len(), 22);
    let r = seams::material_orientation::qualify(
        &exact,
        seams::material_orientation::Limits {
            steps: 1000000,
            curve_spans: 100000,
            normal_spans: 100000,
        },
    )
    .unwrap();
    let oriented = r.certificate.expect(r.reason);
    let proposals = body
        .geometry()
        .topology()
        .links
        .iter()
        .map(|l| [2, 2, 1, 0, 1, 0][shell.uses()[l.cycle[0].edge][0].face / 4])
        .collect::<Vec<_>>();
    let r = seams::vertices::qualify(
        &smooth,
        &oriented,
        &proposals,
        1e-8,
        seams::vertices::Limits {
            steps: 1000000,
            normal_spans: 100000,
            domain_cells: 100000,
            exact_work: 100000000,
        },
    )
    .unwrap();
    eprintln!(
        "G1 grid graph stars: {} vertex={:?} steps={} normal={} domain={} exact={}",
        r.reason, r.uncertain_vertex, r.steps, r.normal_spans, r.domain_cells, r.exact_work
    );
    let tight = seams::vertices::Limits {
        steps: r.steps,
        normal_spans: r.normal_spans,
        domain_cells: r.domain_cells,
        exact_work: r.exact_work,
    };
    let atlas = r.certificate.expect(r.reason);
    let orders = atlas
        .vertices()
        .iter()
        .filter_map(|v| match v {
            seams::vertices::Vertex::Smooth(n) => Some(n.order()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(orders.iter().filter(|&&k| k == 1).count(), 1);
    assert_eq!(orders.iter().filter(|&&k| k == 2).count(), 5);
    assert_eq!(
        atlas
            .vertices()
            .iter()
            .filter(|v| matches!(v, seams::vertices::Vertex::C0 { .. }))
            .count(),
        20
    );
    assert!(
        seams::vertices::qualify(&smooth, &oriented, &proposals, 1e-8, tight)
            .unwrap()
            .certificate
            .is_some()
    );
    for axis in 0..4 {
        let mut short = tight;
        match axis {
            0 => short.steps -= 1,
            1 => short.normal_spans -= 1,
            2 => short.domain_cells -= 1,
            _ => short.exact_work -= 1,
        }
        let r = seams::vertices::qualify(&smooth, &oriented, &proposals, 1e-8, short).unwrap();
        assert!(
            r.certificate.is_none(),
            "G1 star admitted short budget axis={axis}"
        );
        assert!(
            r.steps <= short.steps
                && r.normal_spans <= short.normal_spans
                && r.domain_cells <= short.domain_cells
                && r.exact_work <= short.exact_work
        );
    }
    // Full-carrier Hessian identity must reject an upgrade even though the
    // second-order residual vanishes at the central endpoint itself.
    for index in jumps {
        let r = seams::qualify_curvature_exact(
            &shell.edges()[index],
            seams::CurvatureLimits {
                normal_distance: 0.,
                curvature_distance: 0.,
                cells: 100000,
                curve_spans: 100000,
                shape_cells: 100000,
            },
            axes[index],
            100000000,
        )
        .unwrap();
        assert!(r.seam.is_none());
        assert_eq!(r.reason, "exact-original-meridian-graph-jets-different");
    }
    assert_eq!(body.definition().unwrap(), before);
}

#[test]
fn original_interior_contact_refusal_names_the_actual_unproved_face_pair() {
    let mut model = source_model_with_curvature_jump(true);
    // Same complete boundary and injective XY chart, but the interior cubic
    // depression crosses the bottom face. A valid incidence graph and shared
    // edges must not authorize this self-intersecting original body.
    model.faces[4].surface.control_points[1][1][2] -= 32.;
    model.validate().unwrap();
    let before = model
        .faces
        .iter()
        .map(|f| f.surface.clone())
        .collect::<Vec<_>>();
    let shell = prepare_projective(&model, limits()).unwrap().shell.unwrap();
    let r = crate::source_shell_geometry::qualify_projective(shell, geometry_limits()).unwrap();
    assert!(r.geometry.is_none());
    assert_eq!(r.reason, "source-shell-extra-contact-unproven");
    assert_eq!(r.next_pair, Some([0, 4]));
    assert_eq!(
        model
            .faces
            .iter()
            .map(|f| f.surface.clone())
            .collect::<Vec<_>>(),
        before
    );
}
