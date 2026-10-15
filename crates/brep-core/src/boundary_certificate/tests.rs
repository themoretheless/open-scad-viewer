use super::*;
fn volume_limits() -> volume_validity::Limits {
    volume_validity::Limits {
        boundary: crate::boundary_embedding::Limits {
            exact_work: 1000000,
            trim_pairs: 10000,
            trim_cells: 100000,
            trim_domain_cells: 1000000,
            spans: 1000,
            contacts: crate::face_contacts::Limits {
                pairs: 10000,
                cells: 100000,
                domain_cells: 1000000,
                cells_per_pair: 1000,
                domain_cells_per_pair: 10000,
            },
        },
        nesting_pairs: 100,
        nesting_cells: 100000,
        nesting_domain_cells: 1000000,
        orientation_cells: 100000,
        orientation_domain_cells: 1000000,
        orientation_spans: 100,
    }
}
fn limits() -> Limits {
    Limits {
        correspondence: surface_correspondence::Limits {
            cells: 10000,
            products: 1000000,
        },
        regularity_cells: 10000,
        linear_cells_per_boundary: 10000,
        reference_volume: volume_limits(),
        retained_volume: volume_limits(),
    }
}
fn warped() -> Model {
    let mut model = crate::cuboid([0.; 3], [4.; 3]).unwrap();
    // Nonconstant cross-section frame: transverse shear changes with height.
    // Both caps are nonplanar saddles. The full trilinear map is represented
    // exactly on each original bilinear face and each original linear edge.
    let warp = |p: &mut [f64]| {
        let [x, y, z] = [p[0], p[1], p[2]];
        p[0] = x + z * y / 64.;
        p[1] = y + z / 8.;
        p[2] = z + x * y / 64.;
    };
    for v in &mut model.vertices {
        warp(&mut v.point);
    }
    for edge in &mut model.edges {
        for p in &mut edge.curve.control_points {
            warp(p);
        }
    }
    for face in &mut model.faces {
        for p in face.surface.control_points.iter_mut().flatten() {
            warp(p);
        }
    }
    model.validate().unwrap();
    model
}
#[test]
fn moving_cross_sections_and_filled_saddle_caps_need_the_whole_certificate() {
    let source = warped();
    let original = source.clone();
    let report = inspect(&source, &source, 1e-8, 0., limits()).unwrap();
    assert!(
        report.certified,
        "{:?}; reference volume={:?} regularity={:?}",
        report.reason,
        report.reference_volume.as_ref().map(|r| (
            r.boundary.agreement.all_equal,
            r.boundary.intersections.faces.all_faces_injective,
            r.boundary
                .intersections
                .pairs
                .pairs
                .iter()
                .filter(|p| p.reason == "pair-unresolved")
                .map(|p| p.faces)
                .collect::<Vec<_>>()
        )),
        report
            .reference_regularity
            .as_ref()
            .map(|r| &r.unresolved_faces)
    );
    assert!(report.continuous_bound && report.within_budget && report.error_upper == Some(0.));
    assert_eq!(report.face_bounds.len(), source.faces.len());
    assert_eq!(source, original);
}
#[test]
fn every_face_and_both_boundaries_share_their_actual_work_limits() {
    let source = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    let full = inspect(&source, &source, 1e-8, 0., limits()).unwrap();
    assert!(full.certified);
    let short = inspect(
        &source,
        &source,
        1e-8,
        0.,
        Limits {
            correspondence: surface_correspondence::Limits {
                cells: source.faces.len() - 1,
                ..limits().correspondence
            },
            ..limits()
        },
    )
    .unwrap();
    assert!(!short.certified && !short.continuous_bound && short.error_upper.is_none());
    assert_eq!(short.correspondence_cells, source.faces.len() - 1);
    let used = full.reference_regularity.as_ref().unwrap().cells
        + full.retained_regularity.as_ref().unwrap().cells;
    let short = inspect(
        &source,
        &source,
        1e-8,
        0.,
        Limits {
            regularity_cells: used - 1,
            ..limits()
        },
    )
    .unwrap();
    assert!(short.continuous_bound && !short.certified);
    assert_eq!(short.reason, Some("whole-boundary-regularity-unproved"));
    let mut low = limits();
    low.retained_volume.boundary.contacts.pairs = 1;
    let short = inspect(&source, &source, 1e-8, 0., low).unwrap();
    assert!(short.reference_volume.as_ref().unwrap().proven);
    assert!(!short.retained_volume.as_ref().unwrap().proven && !short.certified);
}
#[test]
fn changed_holes_roles_or_uv_correspondence_cannot_reuse_a_certificate() {
    let source = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    let mut changed = source.clone();
    // Reversing a complete shell preserves combinatorial topology; its material
    // orientation and authored correspondence are still different.
    for u in &mut changed.shells[0].faces {
        u.reversed = !u.reversed;
    }
    changed.validate().unwrap();
    let report = inspect(&source, &changed, 1e-8, 1., limits()).unwrap();
    assert!(!report.certified && !report.continuous_bound);
    assert_eq!(report.reason, Some("boundary-correspondence-mismatch"));
}
#[test]
fn positional_identity_never_proves_a_folded_cap_regular() {
    let mut folded = crate::freeform_cuboid_solid([0.; 3], [3.; 3]).unwrap();
    for i in 1..3 {
        for j in 1..3 {
            folded.faces[1].surface.control_points[i][j][0] = if i == 1 { 6. } else { -3. };
        }
    }
    folded.validate().unwrap();
    let r = inspect(&folded, &folded, 1e-8, 0., limits()).unwrap();
    assert!(r.continuous_bound && r.error_upper == Some(0.) && !r.certified);
    assert!(!r.reference_regularity.unwrap().all_faces_regular);
}

#[test]
fn filled_cap_interior_displacement_is_included_in_the_complete_bound() {
    let mut source = crate::cuboid([0.; 3], [4.; 3]).unwrap();
    for face in &mut source.faces {
        face.surface =
            nurbs_core::surface_edit::elevate(&face.surface, nurbs_core::surface::Axis::U, 2)
                .unwrap();
        face.surface =
            nurbs_core::surface_edit::elevate(&face.surface, nurbs_core::surface::Axis::V, 2)
                .unwrap();
    }
    let mut retained = source.clone();
    retained.faces[1].surface.control_points[1][1][2] += 0.25;
    let result = inspect(&source, &retained, 1e-8, 0.3, limits()).unwrap();
    assert!(result.certified, "{:?}", result.reason);
    assert!(result.error_upper.unwrap() >= 0.25 && result.error_upper.unwrap() < 0.3);
    let denied = inspect(&source, &retained, 1e-8, 0.01, limits()).unwrap();
    assert!(denied.continuous_bound && !denied.within_budget && !denied.certified);
    assert_eq!(denied.reason, Some("boundary-error-budget-exceeded"));
}

#[test]
fn hole_regions_and_material_orientation_remain_independent_obligations() {
    let wire = |points: [[f64; 2]; 4]| {
        (0..4)
            .map(|i| nurbs_core::curve::Curve::from_polyline(vec![points[i].to_vec(), points[(i + 1) % 4].to_vec()]).unwrap())
            .collect()
    };
    let source = crate::prism::extrude(
        &[
            wire([[0., 0.], [4., 0.], [4., 4.], [0., 4.]]),
            wire([[1., 1.], [1., 2.], [2., 2.], [2., 1.]]),
        ],
        0.,
        8.,
    )
    .unwrap();
    assert!(source.faces.iter().any(|f| !f.holes.is_empty()));
    let r = inspect(&source, &source, 1e-8, 0., limits()).unwrap();
    assert!(r.certified, "{:?}", r.reason);
    let mut inverted = source.clone();
    for u in &mut inverted.shells[0].faces {
        u.reversed = !u.reversed;
    }
    inverted.validate().unwrap();
    let r = inspect(&inverted, &inverted, 1e-8, 0., limits()).unwrap();
    assert!(
        r.continuous_bound
            && r.reference_regularity.as_ref().unwrap().all_faces_regular
            && !r.certified
    );
    assert!(!r.reference_volume.as_ref().unwrap().proven);
}
