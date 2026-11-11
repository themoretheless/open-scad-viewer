#[path = "../examples/support/compatible_contact_fixture.rs"]
mod fixture;
use brep_core::{face_cut, region_replacement as r};
#[test]
fn saved_patch_graph_proofs_bind_actual_closed_model_and_preserve_source_supports() {
    let (source, request, skin, contacts) = fixture::fixture();
    let before = source.clone();
    let retention = face_cut::plan_contact_retention(
        &source,
        0,
        request,
        &skin,
        [&contacts[0], &contacts[1]],
        "saved-graph-regression",
        1e-8,
        Default::default(),
        |roots| Ok(fixture::paths(&source, request, &contacts, roots)),
    )
    .unwrap();
    let partition = retention.plan().model.clone();
    let authored = fixture::patch(&retention, &skin);
    let selected = retention
        .plan()
        .regions
        .iter()
        .flat_map(|r| r.discard_faces.iter().copied())
        .collect::<Vec<_>>();
    let patches = authored
        .patches
        .iter()
        .map(|p| r::SewPatch {
            surface: &p.surface,
            loops: &p.loops,
            reversed: p.reversed,
        })
        .collect::<Vec<_>>();
    let body = r::sew_g2_patch_set_with_graph_work(
        &partition,
        r::PatchReplacement {
            selected: &selected,
            new_vertices: &authored.vertices,
            new_edges: &authored.edges,
            patches: &patches,
        },
        "saved-graph-qualified",
        1e-8,
        Default::default(),
        100_000_000,
    )
    .unwrap();
    body.model().validate().unwrap();
    assert!(body.seams().complete && body.seams().within_tolerance);
    assert!(body.seam_coefficient_work() > 0 && body.seam_coefficient_work() <= 100_000_000);
    assert!(body.work().0 <= 100000 && body.work().1 <= 1000000);
    assert!(
        body.seams()
            .seams
            .iter()
            .all(|s| s.positions.as_ref().is_some_and(|p| p.proven))
    );
    assert_eq!(body.fields().len(), body.model().faces.len());
    for f in body.fields() {
        assert_eq!(f.decision, brep_core::face_curvature::Decision::Within);
        assert!(f.absolute_curvature_upper.is_some_and(|k| k <= 10.));
    }
    for (old, new) in body.source_faces().iter().enumerate() {
        if let Some(new) = new {
            assert_eq!(
                body.model().faces[*new].surface,
                partition.faces[old].surface
            );
        }
    }
    // A full-curve G2 result must not be inherited by independently trimmed
    // coedge clocks with different intervals.
    let mut trimmed = body.model().clone();
    let edge = body.seams().seams[0].edge;
    let cd = trimmed.edges[edge].curve.domain();
    let coedge = trimmed
        .loops
        .iter_mut()
        .flat_map(|w| &mut w.coedges)
        .find(|c| c.edge == edge)
        .unwrap();
    let pd = coedge.pcurve.domain();
    *coedge = coedge
        .clone()
        .with_trim(brep_topology::CoedgeTrim {
            curve_parameter: [cd[0], cd[0] * 0.5 + cd[1] * 0.5],
            pcurve_parameter: pd,
            periodic_lift: [[0; 2]; 2],
        })
        .unwrap();
    let trimmed_before = trimmed.clone();
    let result = brep_core::sweep_smoothness::geometric::inspect_edges_with_graph_work(
        &trimmed,
        &[edge],
        brep_core::sweep_smoothness::geometric::Tolerances {
            position: 1e-7,
            normal: 1e-5,
            curvature: 1e-4,
        },
        1000,
        100000,
        1000000,
    );
    assert!(
        result.is_err(),
        "partial clocks must never inherit full-edge admission"
    );
    assert_eq!(trimmed, trimmed_before);
    assert_eq!(source, before);
    assert_eq!(partition, retention.plan().model);
}
