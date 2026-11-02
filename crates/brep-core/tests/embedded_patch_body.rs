#[path = "../examples/support/retained_patch_fixture.rs"]
mod fixture;
use brep_core::{boundary_embedding as e, face_cut, region_replacement as r};
fn limits() -> r::EmbeddedPatchLimits {
    r::EmbeddedPatchLimits {
        construction: Default::default(),
        linear_cells: 100_000,
        global: e::Limits {
            exact_work: 1_000_000,
            trim_pairs: 10_000,
            trim_cells: 100_000,
            trim_domain_cells: 1_000_000,
            spans: 10_000,
            contacts: brep_core::face_contacts::Limits {
                pairs: 10_000,
                cells: 1_000_000,
                domain_cells: 1_000_000,
                cells_per_pair: 1000,
                domain_cells_per_pair: 10_000,
            },
        },
    }
}
#[test]
fn admitted_three_and_five_patch_bodies_own_fresh_complete_embedding_evidence() {
    for n in [3, 5] {
        let (mut m, face, cut, seeds) = fixture::source();
        if n == 5 {
            let edge = m.loops[m.faces[face].outer].coedges[2].edge;
            let d = m.edges[edge].curve.domain();
            m = brep_core::feature_ops::edge_partition::split(
                &m,
                edge,
                &[d[0] + (d[1] - d[0]) * 0.25, d[0] + (d[1] - d[0]) * 0.75],
                "embedded-source-rim",
                Default::default(),
            )
            .unwrap()
            .model()
            .clone();
        }
        let before = m.clone();
        let cuts = [cut];
        let req = [face_cut::FaceRetention {
            face,
            cuts: &cuts,
            keep_uv: &seeds,
        }];
        let out = r::trim_and_sew_embedded_g2_patch_set(
            &m,
            &req,
            "embedded-whole-body",
            1e-8,
            limits(),
            fixture::fan,
        )
        .unwrap();
        assert_eq!(m, before);
        assert_eq!(out.retained().patch_faces().len(), n);
        assert!(out.global().proven && out.global().intersections.absence_proven);
        assert_eq!(
            out.global().intersections.pairs.total_pairs,
            if n == 3 { 528 } else { 595 }
        );
        assert!(out.global().intersections.pairs.next_pair.is_none());
        assert!(out.retained().seams().complete && out.retained().seams().within_tolerance);
        assert!(
            out.retained()
                .fields()
                .iter()
                .all(|p| p.absolute_curvature_upper.is_some_and(|x| x <= 10.))
        );
        assert_eq!(out.tolerance_uv(), 1e-8);
        assert_eq!(out.limits().global.contacts.cells, 1_000_000);
        assert_eq!(out.model(), out.retained().model());
        eprintln!(
            "embedded_body patches={n} proven={} pairs={}",
            out.global().proven,
            out.global().intersections.pairs.total_pairs
        );
    }
}
#[test]
fn unvisited_global_pairs_return_diagnostics_but_no_admitted_body() {
    let (m, face, cut, seeds) = fixture::source();
    let before = m.clone();
    let cuts = [cut];
    let req = [face_cut::FaceRetention {
        face,
        cuts: &cuts,
        keep_uv: &seeds,
    }];
    let mut l = limits();
    l.global.contacts.pairs = 1;
    match r::trim_and_sew_embedded_g2_patch_set(
        &m,
        &req,
        "partial-global-refusal",
        1e-8,
        l,
        fixture::fan,
    ) {
        Err(r::EmbeddedPatchFailure::Unproven(d)) => {
            assert!(!d.report().proven);
            assert!(d.report().intersections.pairs.next_pair.is_some());
            assert_eq!(d.report().intersections.pairs.pairs.len(), 1);
            assert_eq!(d.source_faces().len(), m.faces.len());
            assert_eq!(d.patch_faces().len(), 3);
            assert_eq!(d.limits().global.contacts.pairs, 1);
            assert_eq!(d.tolerance_uv(), 1e-8);
        }
        Err(e) => panic!("Wrong refusal stage: {e:?}"),
        Ok(_) => panic!("A pending suffix cannot admit a body"),
    }
    assert_eq!(m, before);
}
#[test]
fn construction_and_invalid_audit_work_remain_distinct_atomic_refusals() {
    fn standard_error<E: std::error::Error>() {}
    standard_error::<r::EmbeddedPatchFailure>();
    let (m, face, cut, seeds) = fixture::source();
    let before = m.clone();
    let cuts = [cut];
    let req = [face_cut::FaceRetention {
        face,
        cuts: &cuts,
        keep_uv: &seeds,
    }];
    assert!(matches!(
        r::trim_and_sew_embedded_g2_patch_set(&m, &req, "bad-construction", 1e-8, limits(), |_| {
            Err(nurbs_core::Error::new(
                "TEST_CONSTRUCTION_REFUSAL",
                "No candidate",
            ))
        }),
        Err(r::EmbeddedPatchFailure::Construction(_))
    ));
    let mut l = limits();
    l.global.contacts.pairs = 0;
    assert!(matches!(
        r::trim_and_sew_embedded_g2_patch_set(&m, &req, "invalid-audit", 1e-8, l, fixture::fan),
        Err(r::EmbeddedPatchFailure::GlobalAudit(_))
    ));
    assert_eq!(m, before);
}

#[test]
fn admitted_patch_body_step_roundtrip_retains_closed_curvature_bounded_boundary() {
    let (m, face, cut, seeds) = fixture::source();
    let cuts = [cut];
    let requests = [face_cut::FaceRetention { face, cuts: &cuts, keep_uv: &seeds }];
    let admitted = r::trim_and_sew_embedded_g2_patch_set(
        &m, &requests, "embedded-step", 1e-8, limits(), fixture::fan,
    ).unwrap();
    let (step, _, _) = brep_core::export_step_v9(admitted.model()).unwrap();
    let (back, _, _) = brep_core::import_step_v9(&step).unwrap();
    back.validate().unwrap();
    if let Ok(path) = std::env::var("EMBEDDED_PATCH_STEP_OUTPUT") {
        std::fs::write(path, &step).unwrap();
    }
    // Exact authored curve equality only identifies the seam's scope; the
    // following inspection proves G2 anew on the actual imported supports.
    let imported_seams = admitted.retained().seams().seams.iter().map(|seam| {
        let curve = &admitted.model().edges[seam.edge].curve;
        let matches = back.edges.iter().enumerate()
            .filter(|(_, edge)| &edge.curve == curve)
            .map(|(i, _)| i).collect::<Vec<_>>();
        assert_eq!(matches.len(), 1, "STEP lost unique authored seam curve");
        matches[0]
    }).collect::<Vec<_>>();
    assert!(!imported_seams.is_empty());
    let seams = brep_core::sweep_smoothness::geometric::inspect_edges_with_exact(
        &back, &imported_seams,
        brep_core::sweep_smoothness::geometric::Tolerances {
            position: admitted.model().tolerance_mm, normal: 1e-5, curvature: 1e-4,
        }, 100_000, 1_000_000,
    ).unwrap();
    assert!(seams.complete && seams.within_tolerance, "STEP roundtrip lost G2");
    assert_eq!(back.bodies.len(), admitted.model().bodies.len());
    assert_eq!(back.faces.len(), admitted.model().faces.len());
    assert!(back.shells.iter().all(|s| s.closed));
    let closure = brep_core::watertight::inspect_with_exact(&back, 100_000, 1_000_000).unwrap();
    assert_eq!(closure.closure.status, brep_core::watertight::Closure::WithinTolerance);
    for face in 0..back.faces.len() {
        let report = brep_core::face_curvature::inspect(
            &back, face, 10., 1e-8,
            brep_core::face_curvature::Limits { cells: 50_000, domain_cells: 1_000_000, shape_cells: 1_000_000 },
            &[],
        ).unwrap();
        assert_eq!(report.decision, brep_core::face_curvature::Decision::Within, "face {face}: {}", report.reason);
        assert!(report.absolute_curvature_upper.is_some_and(|k| k.is_finite() && k <= 10.));
    }
    let global = e::inspect_with_linear(&back, 1e-8, limits().global, 100_000).unwrap();
    assert!(global.proven, "STEP roundtrip lost global embedding proof");
}
