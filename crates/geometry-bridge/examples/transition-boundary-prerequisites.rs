use value_codec::{Value, json};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let input: Value = value_codec::from_str(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();
    let model: brep_core::Model = if args.iter().any(|a| a == "--regenerate") {
        brep_core::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap()
    } else {
        value_codec::from_value(input["request"]["model"].clone()).unwrap()
    };
    model.validate().unwrap();
    let before = format!("{model:?}");
    let agreement = brep_core::boundary_agreement::verify_exact(&model, 1_000_000).unwrap();
    let trim =
        brep_core::face_domain::audit_trim_regions(&model, 1e-8, 10000, 100000, 1000000).unwrap();
    let approximate = if args.iter().any(|a| a == "--exact-only") {
        None
    } else {
        Some(brep_core::boundary_agreement::verify(&model, 100000).unwrap())
    };
    let pair_cells = args
        .iter()
        .find_map(|a| a.strip_prefix("--pair-cells="))
        .map(|s| s.parse::<usize>().unwrap())
        .unwrap_or(256);
    let pair_domain_cells = args
        .iter()
        .find_map(|a| a.strip_prefix("--pair-domain-cells="))
        .map(|s| s.parse::<usize>().unwrap())
        .unwrap_or(4000);
    let embedding = if args.iter().any(|a| a == "--embedding") {
        let r = brep_core::boundary_embedding::inspect(
            &model,
            1e-8,
            brep_core::boundary_embedding::Limits {
                exact_work: 1_000_000,
                trim_pairs: 10000,
                trim_cells: 100000,
                trim_domain_cells: 1000000,
                spans: 4096,
                contacts: brep_core::face_contacts::Limits {
                    pairs: 400,
                    cells: 150000,
                    domain_cells: 1500000,
                    cells_per_pair: pair_cells,
                    domain_cells_per_pair: pair_domain_cells,
                },
            },
        )
        .unwrap();
        Some(
            json!({"proven":r.proven,"allFacesInjective":r.intersections.faces.all_faces_injective,"absenceProven":r.intersections.absence_proven,"allPairsClassified":r.intersections.pairs.all_pairs_classified,
      "hullContacts":r.hull_contacts.iter().map(|c|json!({"faces":c.faces,"edges":c.edges,"vertex":c.vertex,"contactEnclosure":c.contact_enclosure,"joinedProof":c.joined_proof.as_ref().map(|j|json!({"blendFace":j.blend_face,"wallFace":j.wall_face,"collapsedEnd":j.collapsed_end,"projection":j.projection,"proven":j.report.proven,"reason":j.report.reason,"cells":j.report.cells,"weightedBounds":j.report.weighted_bounds,"dominanceMarginLower":j.report.dominance_margin_lower}))})).collect::<Vec<_>>(),
      "pairs":r.intersections.pairs.pairs.iter().map(|p|json!({"faces":p.faces,"status":p.reason})).collect::<Vec<_>>(),"cells":r.intersections.pairs.cells,"domainCells":r.intersections.pairs.domain_cells}),
        )
    } else {
        None
    };
    let output = json!({"sourceModel":model,"pairCellsBudget":pair_cells,"pairDomainCellsBudget":pair_domain_cells,"embedding":embedding,"allEqual":agreement.all_equal,"allJoinsExact":agreement.all_joins_exact,"exactWork":agreement.work,
 "uses":agreement.uses.iter().map(|u|json!({"face":u.face,"wire":u.wire,"coedge":u.coedge,"edge":u.edge,"decision":u.decision.as_ref().map(|d|format!("{:?}",d.outcome))})).collect::<Vec<_>>(),
 "toleranceAgreementComplete":approximate.as_ref().map(|r|r.complete),"toleranceAgreementCells":approximate.as_ref().map(|r|r.cells),"toleranceUses":approximate.iter().flat_map(|r|r.uses.iter()).map(|u|json!({"face":u.face,"edge":u.edge,"status":format!("{:?}",u.status),"witness":u.witness,"distance":u.witness_distance})).collect::<Vec<_>>(),
 "allTrimsValid":trim.all_valid,"trimPairs":trim.pairs,"trimCells":trim.cells,"trimDomainCells":trim.domain_cells,
 "trims":trim.faces.iter().enumerate().map(|(face,r)|json!({"face":face,"report":r.as_ref().map(|r|json!({"valid":r.valid,"reason":r.reason,"winding":r.winding,"pairs":r.pairs,"cells":r.cells,"domainCells":r.domain_cells,"problemLoops":r.problem_loops}))})).collect::<Vec<_>>()});
    assert_eq!(format!("{model:?}"), before);
    println!(
        "allEqual={}, allJoinsExact={}, allTrimsValid={}, work={}",
        agreement.all_equal, agreement.all_joins_exact, trim.all_valid, agreement.work
    );
    std::fs::write(&args[2], value_codec::to_string(&output).unwrap()).unwrap();
}
