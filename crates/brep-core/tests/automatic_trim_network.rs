#[path = "../examples/support/curved_trim_fixture.rs"]
mod fixture;
use brep_core::{face_cut, watertight};
#[test]
fn selected_upper_sectors_are_replaced_by_a_closed_owned_plane_cap() {
    let source = fixture::build();
    let cutter = brep_core::cuboid([-1., -1., 0.5], [2., 2., 0.75]).unwrap();
    let out = face_cut::partition_ruled_network(
        &source,
        &[1, 3, 4, 5],
        &cutter,
        0,
        1e-8,
        100000,
        1000000,
    )
    .unwrap();
    let mut selected = Vec::new();
    let mut cells = out.cells;
    for section in &out.sections {
        let uv = match section.source_face {
            1 => [0.5, 0.8],
            3 => [0.75, 0.95],
            4 => [0.5, 0.9],
            5 => [0.25, 0.95],
            _ => unreachable!(),
        };
        let mut owners = Vec::new();
        for face in section.faces {
            let r = brep_core::face_domain::FaceDomain::new(&out.model, face, 1e-8)
                .unwrap()
                .classify(uv.map(|v| [v, v]), 100000 - cells)
                .unwrap();
            cells += r.cells;
            if r.location == nurbs_core::trim_domain::Location::Inside && r.winding == Some(1) {
                owners.push(face);
            }
        }
        assert_eq!(owners.len(), 1);
        selected.push(owners[0]);
    }
    let before = out.model.clone();
    let capped = brep_core::region_replacement::cap_region(
        &out.model,
        &selected,
        &cutter.faces[0].surface,
        1e-8,
        100000 - cells,
        1000000 - out.exact_work,
    )
    .unwrap();
    cells += capped.cells;
    assert!(cells <= 100000 && out.exact_work + capped.replacement.exact_work <= 1000000);
    assert!(capped.interior.certified && capped.replacement.trims.all_valid);
    assert_eq!(
        capped.replacement.closure.closure.status,
        watertight::Closure::WithinTolerance
    );
    assert_eq!(capped.replacement.model.faces.len(), 7);
    assert_eq!(capped.replacement.model.edges.len(), 15);
    assert_eq!(capped.replacement.model.vertices.len(), 10);
    let mut misplaced = cutter.faces[0].surface.clone();
    for row in &mut misplaced.control_points {
        for p in row {
            p[2] += 0.01;
        }
    }
    assert!(
        brep_core::region_replacement::cap_region(
            &out.model, &selected, &misplaced, 1e-8, 100000, 1000000
        )
        .is_err()
    );
    assert!(
        brep_core::region_replacement::cap_region(
            &out.model,
            &selected[..1],
            &cutter.faces[0].surface,
            1e-8,
            100000,
            1000000
        )
        .is_err()
    );
    assert!(
        brep_core::region_replacement::cap_region(
            &out.model,
            &selected,
            &cutter.faces[0].surface,
            1e-8,
            16,
            1000000
        )
        .is_err()
    );
    assert_eq!(out.model, before);
}
#[test]
fn neighboring_sections_share_each_original_root_once_and_roll_back_together() {
    let source = fixture::build();
    let before = source.clone();
    let cutter = brep_core::cuboid([-1., -1., 0.5], [2., 2., 0.75]).unwrap();
    let out = face_cut::partition_ruled_network(
        &source,
        &[1, 3, 4, 5],
        &cutter,
        0,
        1e-8,
        100000,
        1000000,
    )
    .unwrap();
    assert_eq!(out.sections.len(), 4);
    assert_eq!(out.root_vertices.len(), 4);
    assert_eq!(out.model.faces.len(), 10);
    assert_eq!(out.model.edges.len(), 20);
    assert_eq!(out.model.vertices.len(), 12);
    assert!(out.cells <= 100000 && out.exact_work <= 1000000);
    let queried = out
        .boundary_coverage
        .iter()
        .map(|(e, _)| *e)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(queried.len(), out.boundary_coverage.len());
    assert!(out.boundary_coverage.iter().all(|(_, r)| r.complete));
    for &(_, _, v) in &out.root_vertices {
        assert_eq!(
            out.sections
                .iter()
                .filter(|s| s.endpoints.contains(&v))
                .count(),
            2
        );
        assert_eq!(
            out.model
                .edges
                .iter()
                .filter(|e| e.vertices.contains(&v))
                .count(),
            4
        );
    }
    assert!(out.model.persistent_naming_complete());
    assert_eq!(
        watertight::inspect(&out.model, 100000).unwrap().status,
        watertight::Closure::WithinTolerance
    );
    // The last requested face has no section. None of the four earlier cuts
    // may escape as a partially committed model.
    assert!(
        face_cut::partition_ruled_network(
            &source,
            &[1, 3, 4, 5, 2],
            &cutter,
            0,
            1e-8,
            100000,
            1000000
        )
        .is_err()
    );
    assert!(
        face_cut::partition_ruled_network(
            &source,
            &[1, 3, 4, 5],
            &cutter,
            0,
            1e-8,
            out.cells - 1,
            1000000
        )
        .is_err()
    );
    assert!(
        face_cut::partition_ruled_network(&source, &[1, 1], &cutter, 0, 1e-8, 100000, 1000000)
            .is_err()
    );
    #[cfg(feature = "codec")]
    {
        let restored: brep_core::Model =
            value_codec::from_str(&value_codec::to_string(&out.model).unwrap()).unwrap();
        assert_eq!(restored, out.model);
        assert_eq!(
            watertight::inspect(&restored, 100000).unwrap().status,
            watertight::Closure::WithinTolerance
        );
    }
    assert_eq!(source, before);
}

#[test]
fn automatic_cap_is_atomic_and_seeds_cannot_authorize_ambiguous_removal() {
    use face_cut::{CapSelection, trim_ruled_and_cap};
    let source = fixture::build();
    let before = source.clone();
    let cutter = brep_core::cuboid([-1., -1., 0.5], [2., 2., 0.75]).unwrap();
    let requests = [
        CapSelection { face: 1, remove_uv: [0.5, 0.8] },
        CapSelection { face: 3, remove_uv: [0.75, 0.95] },
        CapSelection { face: 4, remove_uv: [0.5, 0.9] },
        CapSelection { face: 5, remove_uv: [0.25, 0.95] },
    ];
    let run = |seeds: &[CapSelection], uncut: &[usize], budget| {
        trim_ruled_and_cap(&source, seeds, uncut, &cutter, 0, 1e-8, budget, 1000000)
    };
    let out = run(&requests, &[], 100000).unwrap();
    assert_eq!(out.cap.replacement.model.faces.len(), 7);
    assert!(out.cap.replacement.trims.all_valid);
    assert_eq!(out.cap.replacement.closure.closure.status, watertight::Closure::WithinTolerance);
    assert!(out.cells <= 100000 && out.exact_work <= 1000000);
    assert!(run(&requests, &[], out.cells - 1).is_err());
    assert!(run(&requests, &[1], 100000).is_err());
    assert!(run(&requests[..1], &[], 100000).is_err());
    let mut invalid = requests;
    invalid[0].remove_uv = [f64::NAN, 0.8];
    assert!(run(&invalid, &[], 100000).is_err());
    invalid[0].remove_uv = [0., 0.];
    assert!(run(&invalid, &[], 100000).is_err());
    assert_eq!(source, before);
    #[cfg(feature = "codec")]
    {
        let restored: brep_core::Model = value_codec::from_str(
            &value_codec::to_string(&out.cap.replacement.model).unwrap()).unwrap();
        assert_eq!(restored, out.cap.replacement.model);
        assert_eq!(watertight::inspect(&restored, 100000).unwrap().status,
            watertight::Closure::WithinTolerance);
    }
}

#[test]
fn transposed_original_charts_share_roots_and_close_the_same_material_cut() {
    use face_cut::{CapSelection, trim_ruled_and_cap};
    let mut source = fixture::build();
    fixture::transpose_charts(&mut source);
    let before = source.clone();
    let cutter = brep_core::cuboid([-1., -1., 0.5], [2., 2., 0.75]).unwrap();
    let requests = [
        CapSelection { face: 1, remove_uv: [0.8, 0.5] },
        CapSelection { face: 3, remove_uv: [0.95, 0.75] },
        CapSelection { face: 4, remove_uv: [0.9, 0.5] },
        CapSelection { face: 5, remove_uv: [0.95, 0.25] },
    ];
    let out = trim_ruled_and_cap(&source, &requests, &[], &cutter, 0,
        1e-8, 100000, 1000000).unwrap();
    assert_eq!(out.cap.replacement.model.faces.len(), 7);
    assert!(out.cap.replacement.trims.all_valid);
    assert_eq!(out.cap.replacement.closure.closure.status, watertight::Closure::WithinTolerance);
    assert!(out.cells <= 100000 && out.exact_work <= 1000000);
    assert_eq!(source, before);
}
