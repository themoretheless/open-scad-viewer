//! Fresh global boundary audit of the saved corner body; no inferred admission.
use brep_core::{boundary_embedding as b, face_contacts};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::PathBuf::from(std::env::args().nth(1).ok_or("body directory required")?);
    let model: brep_core::Model = if std::env::args().nth(2).as_deref() == Some("--binary") {
        std::fs::create_dir_all(&root)?;
        let source = brep_core::cuboid([0.; 3], [10.; 3])?;
        let net = brep_core::region_replacement::propose_binary_corner_patch_network(
            0.75,
            [0.13, 0.2, 0.35],
            5.,
            0.214,
        )?;
        let body = brep_core::region_replacement::propose_corner_patch_body(&source, &net)?;
        std::fs::write(
            root.join("body.json"),
            value_codec::to_string(
                &value_codec::json!({"model":body.model,"accepted":false,"binaryBoundaries":true}),
            )?,
        )?;
        body.model
    } else {
        let value: value_codec::Value =
            value_codec::from_str(&std::fs::read_to_string(root.join("body.json"))?)?;
        value_codec::from_value(value["model"].clone())?
    };
    let exact = brep_core::boundary_agreement::verify_exact(&model, 1_000_000)?;
    std::fs::write(
        root.join("exact-boundaries.json"),
        value_codec::to_string(
            &value_codec::json!({"allEqual":exact.all_equal,"allJoinsExact":exact.all_joins_exact,"work":exact.work,"uses":exact.uses.iter().map(|u|value_codec::json!({"face":u.face,"edge":u.edge,"decision":format!("{:?}",u.decision.as_ref().map(|x|&x.outcome))})).collect::<Vec<_>>()}),
        )?,
    )?;
    eprintln!("phase=exact-boundary-and-global-embedding");
    let r = b::inspect_with_linear(
        &model,
        1e-8,
        b::Limits {
            exact_work: 1_000_000,
            trim_pairs: 10_000,
            trim_cells: 100_000,
            trim_domain_cells: 1_000_000,
            spans: 1024,
            contacts: face_contacts::Limits {
                pairs: 10_000,
                cells: 100_000,
                domain_cells: 1_000_000,
                cells_per_pair: 1000,
                domain_cells_per_pair: 10_000,
            },
        },
        100_000,
    )?;
    let faces = r
        .intersections
        .faces
        .faces
        .iter()
        .map(|f| {
            value_codec::json!({
        "face":f.face,"injectivityProven":f.absence_proven()})
        })
        .collect::<Vec<_>>();
    let pairs = r
        .intersections
        .pairs
        .pairs
        .iter()
        .map(|p| {
            value_codec::json!({
        "faces":p.faces,"reason":p.reason,"hullDisjoint":p.hull_disjoint,
        "sharedBoundaryProven":p.boundary.is_some(),
        "absenceProven":p.result.as_ref().is_some_and(|s|s.absence_proven)})
        })
        .collect::<Vec<_>>();
    let uses=r.agreement.uses.iter().map(|u|value_codec::json!({"face":u.face,"edge":u.edge,"decision":format!("{:?}",u.decision.as_ref().map(|x|&x.outcome))})).collect::<Vec<_>>();
    let report = value_codec::json!({"boundaryEmbeddingProven":r.proven,
        "exactEdgeLifts":r.agreement.all_equal,"exactWireJoins":r.agreement.all_joins_exact,
        "exactWork":r.agreement.work,"allTrimRegionsValid":r.trim.all_valid,
        "allFacesInjective":r.intersections.faces.all_faces_injective,
        "totalPairs":r.intersections.pairs.total_pairs,"nextPair":r.intersections.pairs.next_pair,
        "allPairsClassified":r.intersections.pairs.all_pairs_classified,
        "selfIntersectionAbsenceProven":r.intersections.absence_proven,
        "faces":faces,"pairs":pairs,"edgeLifts":uses,
        "scope":"Immutable original saved body. Exact boundary embedding; no automatic upgrade of tolerance contacts or G2 to global absence. No material/orientation claim."});
    std::fs::write(
        root.join("global-embedding.json"),
        value_codec::to_string_pretty(&report)?,
    )?;
    println!("boundaryEmbeddingProven={}", r.proven);
    Ok(())
}
