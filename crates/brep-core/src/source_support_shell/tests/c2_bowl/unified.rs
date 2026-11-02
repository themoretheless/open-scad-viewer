use super::*;
use crate::source_seam_tangency::{self as seams, material_orientation, vertices};
#[path="unified/model.rs"]
mod model;
fn run(hybrid: bool, jump: bool) {
    let (model, parents, target, changed) = if hybrid {
        model::hybrid(jump)
    } else {
        (super::model(), Vec::new(), usize::MAX, None)
    };
    let shell = prepare_projective(&model, limits()).unwrap().shell.unwrap();
    let g = crate::source_shell_geometry::qualify_projective(shell, geometry_limits()).unwrap();
    eprintln!(
        "unified hybrid={hybrid} jump={jump} geometry={} pair={:?} face={:?}",
        g.reason, g.next_pair, g.uncertain_face
    );
    let geometry = g.geometry.expect(g.reason);
    let volume = crate::source_volume::qualify(
        geometry,
        crate::source_volume::Limits {
            axis: 2,
            origin: 0.,
            absolute_error: if hybrid { 8. } else { 0.25 },
            tolerance_uv: 1e-8,
            cells: 100000,
            spans: 100000,
            domain_cells: 100000,
        },
    )
    .unwrap();
    eprintln!(
        "unified volume hybrid={hybrid} jump={jump}: {} cells={} spans={} domains={} face={:?} uv={:?}",
        volume.reason,
        volume.cells,
        volume.spans,
        volume.domain_cells,
        volume.uncertain_face,
        volume.uncertain_uv
    );
    let body = volume.body.expect(volume.reason);
    let expected = if hybrid {
        3456. + if jump { 9. / 128. } else { 0. }
    } else {
        16.
    };
    assert!(body.volume()[0] <= expected && body.volume()[1] >= expected);
    let before = body.definition().unwrap();
    let shell = body.geometry().shell();
    if hybrid {
        let pair = body
            .geometry()
            .contacts()
            .pairs
            .iter()
            .find(|p| p.paired_projection.is_some())
            .expect("hybrid needs a freshly proved curved-boundary projection")
            .faces;
        let r = crate::source_paired_projection_contact::certify(shell, pair, 100000000, 100000)
            .unwrap();
        assert!(r.certificate.is_some());
        let (work, driver) = (r.exact_work, r.driver_cells);
        assert!(
            crate::source_paired_projection_contact::certify(shell, pair, work, driver)
                .unwrap()
                .certificate
                .is_some()
        );
        let short = crate::source_paired_projection_contact::certify(shell, pair, work - 1, driver)
            .unwrap();
        assert!(short.certificate.is_none() && short.exact_work < work);
        let short = crate::source_paired_projection_contact::certify(shell, pair, work, driver - 1)
            .unwrap();
        assert!(short.certificate.is_none() && short.driver_cells < driver);
    }
    if hybrid {
        let pair = body
            .geometry()
            .contacts()
            .pairs
            .iter()
            .find(|p| p.curved_boundary.is_some())
            .expect("hybrid needs a fresh clipped source-boundary proof")
            .faces;
        let r =
            crate::source_curved_boundary_contact::certify(shell, pair, 100000000, 100000).unwrap();
        assert!(r.certificate.is_some());
        let (work, driver) = (r.exact_work, r.driver_cells);
        assert!(
            crate::source_curved_boundary_contact::certify(shell, pair, work, driver)
                .unwrap()
                .certificate
                .is_some()
        );
        let r =
            crate::source_curved_boundary_contact::certify(shell, pair, work - 1, driver).unwrap();
        assert!(r.certificate.is_none() && r.exact_work < work);
        let r =
            crate::source_curved_boundary_contact::certify(shell, pair, work, driver - 1).unwrap();
        assert!(r.certificate.is_none() && r.driver_cells < driver);
    }
    let polar_faces = shell
        .poles()
        .iter()
        .map(|(a, _)| a.face)
        .collect::<BTreeSet<_>>();
    let classes = shell
        .uses()
        .iter()
        .map(|pair| {
            if pair.iter().all(|a| polar_faces.contains(&a.face)) {
                Class::Curvature {
                    normal_distance: 0.,
                    curvature_distance: 0.,
                }
            } else if hybrid && pair.iter().all(|a| parents[a.face] == target) {
                if changed.is_some_and(|f| pair.iter().any(|a| a.face == f)) {
                    Class::TangentPlanes
                } else {
                    Class::Curvature {
                        normal_distance: 0.,
                        curvature_distance: 0.,
                    }
                }
            } else {
                Class::C0
            }
        })
        .collect::<Vec<_>>();
    let edge_axes = shell
        .uses()
        .iter()
        .map(|p| {
            if hybrid && p.iter().all(|a| parents[a.face] == target) {
                0
            } else {
                2
            }
        })
        .collect::<Vec<_>>();
    let qualify_boundary = |classes: &[Class], exact_budget| {
        seams::qualify_boundary_with_exact_curvature(
            &body,
            classes,
            &edge_axes,
            pole_limits(),
            seams::Limits {
                max_sine_squared: 1e-10,
                cells: 100000,
                curve_spans: 100000,
                normal_spans: 100000,
            },
            PolarBudgets {
                normal_coefficients: 100000000,
                shape_products: 100000000,
                shape_coefficients: 100000000,
            },
            exact_budget,
        )
    };
    let r = qualify_boundary(&classes, 100000000).unwrap();
    let exact_work = r.ordinary_exact_work + r.polar_exact_work;
    assert!(exact_work > 0);
    assert!(
        qualify_boundary(&classes, exact_work)
            .unwrap()
            .certificate
            .is_some()
    );
    let short = qualify_boundary(&classes, exact_work - 1).unwrap();
    assert!(short.certificate.is_none());
    assert!(short.ordinary_exact_work + short.polar_exact_work <= exact_work - 1);
    if jump {
        let upgraded = classes
            .iter()
            .map(|c| {
                if *c == Class::TangentPlanes {
                    Class::Curvature {
                        normal_distance: 0.,
                        curvature_distance: 0.,
                    }
                } else {
                    *c
                }
            })
            .collect::<Vec<_>>();
        assert!(
            qualify_boundary(&upgraded, 100000000)
                .unwrap()
                .certificate
                .is_none(),
            "a genuine G1 curvature jump must not gain G2"
        );
    }
    let boundary = r.certificate.expect(r.reason);
    let r = seams::qualify_c2_pole_neighborhoods(&boundary, 100000).unwrap();
    let poles = r.certificate.expect(r.reason);
    let r = crate::source_volume::regularity::stratified::qualify(&poles, 100000, 1000000).unwrap();
    let rank = r.certificate.expect(r.reason);
    let r =
        crate::source_volume::regularity::smooth::qualify_stratified(&rank, 2, 100000, 100000000)
            .unwrap();
    let face_limits = (r.cells, r.exact_work);
    let faces = r.certificate.expect("complete source C2 face jets");
    assert_eq!(faces.faces().len(), shell.faces().len());
    assert!(
        crate::source_volume::regularity::smooth::qualify_stratified(
            &rank,
            2,
            face_limits.0,
            face_limits.1
        )
        .unwrap()
        .certificate
        .is_some()
    );
    if face_limits.0 > 0 {
        assert!(
            crate::source_volume::regularity::smooth::qualify_stratified(
                &rank,
                2,
                face_limits.0 - 1,
                face_limits.1
            )
            .unwrap()
            .certificate
            .is_none()
        );
    }
    if face_limits.1 > 0 {
        assert!(
            crate::source_volume::regularity::smooth::qualify_stratified(
                &rank,
                2,
                face_limits.0,
                face_limits.1 - 1
            )
            .unwrap()
            .certificate
            .is_none()
        );
    }
    let r = seams::qualify_exact_smooth_collection(
        boundary.ordinary(),
        &edge_axes,
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
    let r = material_orientation::qualify(
        &exact,
        material_orientation::Limits {
            steps: 1000000,
            curve_spans: 100000,
            normal_spans: 100000,
        },
    )
    .unwrap();
    let oriented = r.certificate.expect(r.reason);
    let axes = body
        .geometry()
        .topology()
        .links
        .iter()
        .map(|l| {
            if poles.neighborhoods().iter().any(|n| n.vertex() == l.vertex) {
                2
            } else if l.cycle.iter().all(|n| classes[n.edge] != Class::C0) {
                0
            } else {
                2
            }
        })
        .collect::<Vec<_>>();
    let r = vertices::qualify_stratified(
        &faces,
        &oriented,
        &axes,
        1e-8,
        vertices::Limits {
            steps: 1000000,
            normal_spans: 100000,
            domain_cells: 100000,
            exact_work: 100000000,
        },
    )
    .unwrap();
    eprintln!(
        "unified hybrid={hybrid} jump={jump}: reason={} vertex={:?} work={}/{}/{}/{}",
        r.reason, r.uncertain_vertex, r.steps, r.normal_spans, r.domain_cells, r.exact_work
    );
    let tight = vertices::Limits {
        steps: r.steps,
        normal_spans: r.normal_spans,
        domain_cells: r.domain_cells,
        exact_work: r.exact_work,
    };
    let atlas = r.certificate.expect(r.reason);
    assert!(std::ptr::eq(atlas.body(), &body));
    assert!(std::ptr::eq(
        atlas.pole_neighborhoods(),
        poles.neighborhoods()
    ));
    assert_eq!(
        atlas.vertices().len(),
        body.geometry().topology().links.len()
    );
    assert_eq!(
        atlas
            .vertices()
            .iter()
            .filter(|v| matches!(v, vertices::Vertex::Pole { order: 2, .. }))
            .count(),
        2
    );
    let stars = atlas
        .vertices()
        .iter()
        .filter_map(|v| {
            if let vertices::Vertex::Smooth(n) = v {
                Some(n.order())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(
        stars,
        if hybrid {
            vec![if jump { 1 } else { 2 }]
        } else {
            Vec::new()
        }
    );
    assert!(
        vertices::qualify_stratified(&faces, &oriented, &axes, 1e-8, tight)
            .unwrap()
            .certificate
            .is_some()
    );
    for d in 0..4 {
        let mut short = tight;
        let positive = match d {
            0 => short.steps > 0,
            1 => short.normal_spans > 0,
            2 => short.domain_cells > 0,
            _ => short.exact_work > 0,
        };
        if !positive {
            continue;
        }
        match d {
            0 => short.steps -= 1,
            1 => short.normal_spans -= 1,
            2 => short.domain_cells -= 1,
            _ => short.exact_work -= 1,
        };
        assert!(
            vertices::qualify_stratified(&faces, &oriented, &axes, 1e-8, short)
                .unwrap()
                .certificate
                .is_none()
        );
    }
    let mut wrong = axes.clone();
    let vi = body
        .geometry()
        .topology()
        .links
        .iter()
        .position(|l| poles.neighborhoods().iter().any(|n| n.vertex() == l.vertex))
        .unwrap();
    wrong[vi] = 0;
    assert!(
        vertices::qualify_stratified(&faces, &oriented, &wrong, 1e-8, tight)
            .unwrap()
            .certificate
            .is_none()
    );
    if !hybrid {
        let sharp = vec![Class::C0; classes.len()];
        let c = seams::qualify_boundary(
            &body,
            &sharp,
            seams::PoleRequirement::C0,
            seams::Limits {
                max_sine_squared: 0.01,
                cells: 100000,
                curve_spans: 100000,
                normal_spans: 100000,
            },
        )
        .unwrap()
        .certificate
        .unwrap();
        let e = seams::qualify_exact_smooth_collection(
            c.ordinary(),
            &edge_axes,
            seams::ExactSmoothLimits {
                incidence_steps: 1000000,
                exact_work: 100000000,
                cells: 100000,
                curve_spans: 100000,
                source_spans: 100000,
            },
        )
        .unwrap()
        .certificate
        .unwrap();
        let o = material_orientation::qualify(
            &e,
            material_orientation::Limits {
                steps: 1000000,
                curve_spans: 100000,
                normal_spans: 100000,
            },
        )
        .unwrap()
        .certificate
        .unwrap();
        let sharp_atlas = vertices::qualify_stratified(&faces, &o, &axes, 1e-8, tight)
            .unwrap()
            .certificate
            .unwrap();
        assert!(
            sharp_atlas
                .vertices()
                .iter()
                .all(|v| matches!(v, vertices::Vertex::C0 { .. }))
        );
    }
    assert_eq!(body.definition().unwrap(), before);
}
#[test]
fn complete_common_vertex_atlas_retains_physical_poles_and_explicit_c0() {
    run(false, false);
}
#[test]
fn common_atlas_contains_poles_and_an_ordinary_g2_star() {
    run(true, false);
}
#[test]
fn common_atlas_keeps_an_ordinary_g1_curvature_jump_beside_c2_poles() {
    run(true, true);
}

#[path="unified/c1.rs"]

mod c1;
