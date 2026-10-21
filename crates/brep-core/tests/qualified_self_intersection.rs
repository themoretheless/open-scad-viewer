use brep_core::{face_contacts, self_intersection};
fn limits() -> face_contacts::Limits {
    face_contacts::Limits {
        pairs: 100,
        cells: 10000,
        domain_cells: 100000,
        cells_per_pair: 128,
        domain_cells_per_pair: 1000,
    }
}
#[test]
fn qualified_coverage_preserves_absence_and_pending_pair_distinctions() {
    let model = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    let saved = model.clone();
    let good = self_intersection::inspect_qualified(&model, 1e-8, 64, limits(), 1000000).unwrap();
    assert!(good.absence_proven && good.tolerance_contact_locality_proven);
    assert!(good.interior_contacts.is_empty() && good.unresolved.is_empty());
    let partial = self_intersection::inspect_qualified(
        &model,
        1e-8,
        64,
        face_contacts::Limits {
            pairs: 1,
            ..limits()
        },
        1000000,
    )
    .unwrap();
    assert!(!partial.absence_proven && !partial.tolerance_contact_locality_proven);
    assert!(partial.pairs.next_pair.is_some());
    assert_eq!(model, saved);
    let exhausted = self_intersection::inspect_qualified(
        &model,
        1e-8,
        64,
        face_contacts::Limits {
            cells: 2,
            ..limits()
        },
        0,
    )
    .unwrap();
    assert!(!exhausted.absence_proven && !exhausted.tolerance_contact_locality_proven);
    assert_eq!(exhausted.pairs.next_pair, Some([0, 1]));
    assert!(exhausted.pairs.pairs.is_empty() && exhausted.cells <= 2);
}
#[test]
fn detected_interior_contact_cannot_be_promoted_by_fallback_coverage() {
    let mut model = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    for (face, f) in model.faces.iter_mut().enumerate() {
        f.surface.control_points = (0..2)
            .map(|u| {
                (0..2)
                    .map(|v| {
                        if face == 1 {
                            vec![u as f64, 0.5, v as f64 - 0.5]
                        } else {
                            vec![u as f64, v as f64, face as f64 * 10.]
                        }
                    })
                    .collect()
            })
            .collect();
    }
    let report = self_intersection::inspect_qualified(&model, 1e-8, 64, limits(), 1000000).unwrap();
    assert!(!report.absence_proven && !report.tolerance_contact_locality_proven);
    assert!(report.interior_contacts.contains(&[0, 1]));
    assert!(
        report.cells <= limits().cells
            && report.domain_cells <= limits().domain_cells
            && report.fallback_exact_work <= 1000000
    );
}

#[path = "../examples/support/contact_graft_fixture.rs"]
mod fixture;
#[test]
fn all_variable_body_pairs_are_covered_without_promoting_metric_locality() {
    let f = fixture::build_variable_rolling_automatic();
    let field = f.radius_field;
    let q = brep_core::contact_graft::attach_radius_tangent_qualified(
        &f.source,
        &f.patch,
        &f.mapping,
        &f.targets,
        &[1, 3],
        [field, [0., 1., 0., 0.], field],
        field,
        1e-8,
        1e-7,
        1e-5,
        100000,
        1000000,
        2000000,
    )
    .unwrap();
    let r = self_intersection::inspect_qualified(
        &q.qualified.graft.model,
        1e-8,
        1024,
        face_contacts::Limits {
            pairs: 10000,
            cells: 100000 - q.cells,
            domain_cells: 1000000,
            cells_per_pair: 1000,
            domain_cells_per_pair: 10000,
        },
        1000000 - q.exact_work,
    )
    .unwrap();
    assert!(r.faces.all_faces_injective && r.tolerance_contact_locality_proven);
    assert_eq!(r.pairs.total_pairs, 21);
    assert!(r.unresolved.is_empty() && r.interior_contacts.is_empty());
    assert!(!r.absence_proven && !r.localized.is_empty());
    assert!(
        r.localized
            .iter()
            .all(|p| p.distance_upper_mm <= q.qualified.graft.model.tolerance_mm)
    );
    assert!(
        r.cells + q.cells <= 100000
            && r.domain_cells <= 1000000
            && r.fallback_exact_work + q.exact_work <= 1000000
    );
    // A zero row at an interior knot cannot use the single-Bernstein endpoint
    // argument to claim that contacts occur only at the authored rim.
    let mut changed = q.qualified.graft.model.clone();
    let skin = q.qualified.graft.patch_face;
    for point in &mut changed.faces[skin].surface.control_points[2] {
        point[2] = 0.;
    }
    let plane = changed
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == 0.)
        })
        .unwrap();
    let denied =
        brep_core::boundary_contact_tube::inspect_pair(&changed, [plane, skin], 10000, 1000000)
            .unwrap();
    assert!(denied.certificate.is_none());
}
