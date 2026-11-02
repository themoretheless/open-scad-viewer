use brep_core::{face_curvature as c, region_replacement as r};
fn limits() -> r::StripLimits {
    r::StripLimits {
        tolerances: [1e-7, 1e-5, 1e-4],
        tolerance_uv: 1e-8,
        max_geometry_work: 10000,
        max_cells: 300000,
        max_exact: 1000000,
        max_coefficients: 2000000,
    }
}
#[test]
fn rebuilt_three_strips_keep_original_contacts_g2_and_pass_ten_per_mm() {
    let source = brep_core::cuboid([0.; 3], [10.; 3]).unwrap();
    let before = source.clone();
    let requests = source
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| e.vertices.contains(&0))
        .enumerate()
        .map(|(i, (edge, _))| r::EdgeRadius {
            edge,
            radius_mm: [0.13, 0.2, 0.35][i],
        })
        .collect::<Vec<_>>();
    let strips =
        r::build_trihedral_strips_with_profile(&source, 0, 0, &requests, 0.5, 2., limits(), 0.214)
            .unwrap();
    for s in &strips.strips {
        let shape = &s.patch.model.faces[0].surface;
        assert_eq!(shape.degree_v, 1);
        assert_eq!(shape.control_points[0].len(), 2);
        assert!(shape.weights.iter().flatten().all(|w| *w == 1.));
        assert_eq!(
            s.radius_mm,
            requests
                .iter()
                .find(|r| r.edge == s.source_edge)
                .unwrap()
                .radius_mm
        );
        assert!(s.g2.iter().all(|r| r.report.positions.proven
            && r.report.geometric.decision == nurbs_core::surface_join::Decision::WithinTolerance));
        let receipt = c::inspect(
            &s.patch.model,
            0,
            10.,
            1e-8,
            c::Limits {
                cells: 50000,
                domain_cells: 1000000,
                shape_cells: 1000000,
            },
            &[],
        )
        .unwrap();
        assert_eq!(
            receipt.decision,
            c::Decision::Within,
            "radius {}: {:?} {:?}",
            s.radius_mm,
            receipt.witness_uv,
            receipt.witness_curvature
        );
        let material = brep_core::face_domain::inspect_box_containment(
            &s.patch.model,
            0,
            [[-1e-7, 10.0000001]; 3],
            1e-8,
            50000,
            1000000,
        )
        .unwrap();
        assert_eq!(
            material.decision,
            brep_core::face_domain::BoxDecision::Within
        );
    }
    assert_eq!(source, before);
}
#[test]
fn invalid_profile_is_refused_without_modifying_source() {
    let source = brep_core::cuboid([0.; 3], [10.; 3]).unwrap();
    let before = source.clone();
    let requests = source
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| e.vertices.contains(&0))
        .map(|(edge, _)| r::EdgeRadius {
            edge,
            radius_mm: 0.2,
        })
        .collect::<Vec<_>>();
    for h in [f64::NAN, 0., 0.5] {
        assert!(
            r::build_trihedral_strips_with_profile(&source, 0, 0, &requests, 0.5, 2., limits(), h)
                .is_err()
        );
        assert_eq!(source, before);
    }
}
