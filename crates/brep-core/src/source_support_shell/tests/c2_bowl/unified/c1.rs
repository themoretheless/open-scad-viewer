//! A genuine C1/non-C2 pole beside an independently classified ordinary star.
use super::*;
use crate::source_volume::{
    self as volume,
    regularity::{smooth, stratified},
};
fn admit(model: &crate::Model) -> volume::Body<crate::source_projective_edge::Edge> {
    let r = prepare_projective(model, limits()).unwrap();
    let r = crate::source_shell_geometry::qualify_projective(
        r.shell.expect(r.reason),
        geometry_limits(),
    )
    .unwrap();
    let r = volume::qualify(
        r.geometry.expect(r.reason),
        volume::Limits {
            axis: 2,
            origin: 0.,
            absolute_error: 8.,
            tolerance_uv: 1e-8,
            cells: 100000,
            spans: 100000,
            domain_cells: 100000,
        },
    )
    .unwrap();
    r.body.expect(r.reason)
}
fn run(jump: bool) {
    let (model, parents, target, changed) = model::hybrid_folded(jump);
    let body = admit(&model);
    let definition = body.definition().unwrap();
    let expected = 3456. + if jump { 9. / 128. } else { 0. };
    assert!(body.volume()[0] <= expected && body.volume()[1] >= expected);
    let shell = body.geometry().shell();
    let polar_faces = shell
        .poles()
        .iter()
        .map(|(a, _)| a.face)
        .collect::<BTreeSet<_>>();
    let classes = shell
        .uses()
        .iter()
        .map(|p| {
            if p.iter().all(|a| polar_faces.contains(&a.face)) {
                Class::TangentPlanes
            } else if p.iter().all(|a| parents[a.face] == target) {
                if changed.is_some_and(|f| p.iter().any(|a| a.face == f)) {
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
    let axes = shell
        .uses()
        .iter()
        .map(|p| {
            if p.iter().all(|a| parents[a.face] == target) {
                0
            } else {
                2
            }
        })
        .collect::<Vec<_>>();
    let qualify = |budget| {
        seams::qualify_boundary_with_exact_regular_curvature(
            &body,
            &classes,
            &axes,
            pole_limits().graphs,
            seams::Limits {
                max_sine_squared: 1e-4,
                cells: 100000,
                curve_spans: 100000,
                normal_spans: 100000,
            },
            100000000,
            budget,
        )
        .unwrap()
    };
    let r = qualify(100000000);
    assert!(r.ordinary_exact_work > 0);
    assert!(qualify(r.ordinary_exact_work).certificate.is_some());
    assert!(qualify(r.ordinary_exact_work - 1).certificate.is_none());
    let boundary = r.certificate.expect(r.reason);
    let r = seams::qualify_c1_pole_neighborhoods(
        &boundary,
        seams::C1PoleLimits {
            incidence_steps: 100000,
            exact_work: 100000000,
        },
    )
    .unwrap();
    let poles = r.certificate.expect(r.reason);
    assert!(
        seams::qualify_c2_pole_neighborhoods(&boundary, 100000)
            .unwrap()
            .certificate
            .is_none()
    );
    let g2 =
        crate::source_pole_tangency::qualify_body_pole_curvature(&body, pole_limits()).unwrap();
    assert!(
        g2.certificate.is_none(),
        "folded source has no common quadratic jet"
    );
    let r = stratified::qualify(&poles, 100000, 1000000).unwrap();
    let rank_work = (r.cells, r.steps);
    let rank = r.certificate.expect(r.reason);
    assert!(
        stratified::qualify(&poles, rank_work.0, rank_work.1)
            .unwrap()
            .certificate
            .is_some()
    );
    for (cells, steps) in [
        (rank_work.0 - 1, rank_work.1),
        (rank_work.0, rank_work.1 - 1),
    ] {
        let r = stratified::qualify(&poles, cells, steps).unwrap();
        assert!(r.certificate.is_none() && r.cells <= cells && r.steps <= steps);
    }
    // Parametric face C2 is valid; physical pole order must still remain C1.
    let r = smooth::qualify_stratified(&rank, 2, 100000, 100000000).unwrap();
    let faces = r.certificate.expect("whole original face jets");
    let r = seams::qualify_exact_smooth_collection(
        boundary.ordinary(),
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
        "C1 hybrid jump={jump}: {} work={}/{}/{}/{} rank={rank_work:?}",
        r.reason, r.steps, r.normal_spans, r.domain_cells, r.exact_work
    );
    let tight = vertices::Limits {
        steps: r.steps,
        normal_spans: r.normal_spans,
        domain_cells: r.domain_cells,
        exact_work: r.exact_work,
    };
    let atlas = r.certificate.expect(r.reason);
    assert!(std::ptr::eq(atlas.body(), &body));
    assert!(atlas.pole_neighborhoods().is_empty());
    assert!(std::ptr::eq(
        atlas.c1_pole_neighborhoods(),
        poles.neighborhoods()
    ));
    assert_eq!(atlas.vertices().len(), 15);
    assert_eq!(
        atlas
            .vertices()
            .iter()
            .filter(|v| matches!(v, vertices::Vertex::Pole { order: 1, .. }))
            .count(),
        2
    );
    assert_eq!(
        atlas
            .vertices()
            .iter()
            .filter(|v| matches!(v, vertices::Vertex::C0 { .. }))
            .count(),
        12
    );
    assert_eq!(
        atlas
            .vertices()
            .iter()
            .filter_map(|v| if let vertices::Vertex::Smooth(n) = v {
                Some(n.order())
            } else {
                None
            })
            .collect::<Vec<_>>(),
        vec![if jump { 1 } else { 2 }]
    );
    assert!(
        vertices::qualify_stratified(&faces, &oriented, &axes, 1e-8, tight)
            .unwrap()
            .certificate
            .is_some()
    );
    for d in 0..4 {
        let mut short = tight;
        match d {
            0 => short.steps -= 1,
            1 => short.normal_spans -= 1,
            2 => short.domain_cells -= 1,
            _ => short.exact_work -= 1,
        }
        let r = vertices::qualify_stratified(&faces, &oriented, &axes, 1e-8, short).unwrap();
        assert!(
            r.certificate.is_none()
                && r.steps <= short.steps
                && r.normal_spans <= short.normal_spans
                && r.domain_cells <= short.domain_cells
                && r.exact_work <= short.exact_work
        );
    }
    let mut wrong = axes.clone();
    let pole = body
        .geometry()
        .topology()
        .links
        .iter()
        .position(|l| poles.neighborhoods().iter().any(|n| n.vertex() == l.vertex))
        .unwrap();
    wrong[pole] = 0;
    assert!(
        vertices::qualify_stratified(&faces, &oriented, &wrong, 1e-8, tight)
            .unwrap()
            .certificate
            .is_none()
    );
    // Even byte-identical geometry freshly owned by another Body cannot supply
    // edge evidence for these original face/rank/pole owners.
    let foreign = admit(&model);
    let all_c0 = vec![Class::C0; foreign.geometry().shell().edges().len()];
    let r = seams::qualify_boundary(
        &foreign,
        &all_c0,
        seams::PoleRequirement::C0,
        seams::Limits {
            max_sine_squared: 1e-4,
            cells: 100000,
            curve_spans: 100000,
            normal_spans: 100000,
        },
    )
    .unwrap();
    let foreign_boundary = r.certificate.expect(r.reason);
    let r = seams::qualify_exact_smooth_collection(
        foreign_boundary.ordinary(),
        &vec![2; all_c0.len()],
        seams::ExactSmoothLimits {
            incidence_steps: 1000000,
            exact_work: 100000000,
            cells: 100000,
            curve_spans: 100000,
            source_spans: 100000,
        },
    )
    .unwrap();
    let foreign_exact = r.certificate.expect(r.reason);
    let r = material_orientation::qualify(
        &foreign_exact,
        material_orientation::Limits {
            steps: 1000000,
            curve_spans: 100000,
            normal_spans: 100000,
        },
    )
    .unwrap();
    let foreign_oriented = r.certificate.expect(r.reason);
    assert!(vertices::qualify_stratified(&faces, &foreign_oriented, &axes, 1e-8, tight).is_err());
    assert_eq!(body.definition().unwrap(), definition);
}
#[test]
fn genuine_c1_poles_and_an_ordinary_g2_star_share_one_material_body() {
    run(false)
}
#[test]
fn genuine_c1_poles_and_an_ordinary_g1_jump_share_one_material_body() {
    run(true)
}
