fn main() {
    let output = std::env::args().nth(1).expect("output JSON path");
    let joint = std::env::args().nth(2).as_deref() == Some("embedding");
    let mut cases = Vec::new();
    for sign in [1., -1.] {
        for radii in [[0.5, 1.25], [1.25, 0.5]] {
            let model = brep_core::circular_blend::partial_annular_variable_arc(
                20.,
                5.,
                6.,
                radii,
                [0.3 * sign, 0.7 * sign, 0.4 * sign],
                1e-7,
            )
            .unwrap();
            let faces = brep_core::face_injectivity::inspect(&model, 10000).unwrap();
            let mut middle = Vec::new();
            for z in [1., -1.] {
                let frame = [[1., 0., 0., 0.], [0., 1., 0., 0.], [0., 0., z, 0.]];
                let proof = nurbs_core::surface_injectivity::certify_polar_projection(
                    &model.faces[5].surface,
                    frame,
                    16,
                    256,
                )
                .unwrap();
                middle.push(value_codec::json!({"frame":frame,"contractionUpper":proof}));
            }
            let contact_limits = brep_core::face_contacts::Limits {
                pairs: 1000,
                cells: 1000000,
                domain_cells: 8000000,
                cells_per_pair: if joint { 4096 } else { 128 },
                domain_cells_per_pair: if joint { 100000 } else { 2000 },
            };
            let (pairs, embedding) = if joint {
                let before = value_codec::to_string(&model).unwrap();
                let audit = brep_core::boundary_embedding::inspect(
                    &model,
                    1e-8,
                    brep_core::boundary_embedding::Limits {
                        exact_work: 1000000,
                        trim_pairs: 10000,
                        trim_cells: 10000,
                        trim_domain_cells: 1000000,
                        spans: 10000,
                        contacts: contact_limits,
                    },
                )
                .unwrap();
                assert_eq!(value_codec::to_string(&model).unwrap(), before);
                let summary = value_codec::json!({"proven":audit.proven,"absenceProven":audit.intersections.absence_proven,
                "exactBoundaryAgreement":audit.agreement.all_equal,"exactJoins":audit.agreement.all_joins_exact,
                "simpleTrims":audit.trim.all_valid,"hullCertificates":audit.hull_contacts.len(),
                "joinedProofs":audit.hull_contacts.iter().filter(|c|c.source_join.is_some()||c.polar_join.is_some()).map(|c|
                    value_codec::json!({"faces":c.faces,"edges":c.edges,
                        "polar":c.polar_join.as_ref().map(|p|value_codec::json!({"jacobian":p.jacobian,"determinantLower":p.determinant_lower,"diagonalLower":p.diagonal_lower,"coordinates":p.coordinates})),
                        "source":c.source_join.as_ref().map(|proof|{let p=&proof.report; value_codec::json!({"proven":p.proven,"reason":p.reason,"sourceFrame":p.source_frame,"cells":p.cells,"weightedBounds":p.weighted_bounds,"dominanceMarginLower":p.dominance_margin_lower})})})).collect::<Vec<_>>()});
                (audit.intersections.pairs, Some(summary))
            } else {
                (
                    brep_core::face_contacts::inspect(&model, 1e-8, contact_limits).unwrap(),
                    None,
                )
            };
            let polar=faces.faces[5].result.as_ref().and_then(|r|r.polar_p_matrix.as_ref()).map(|p|
            value_codec::json!({"coordinates":p.coordinates,"jacobian":p.jacobian,"rowSigns":p.row_signs,
                "diagonalLower":p.diagonal_lower,"determinantLower":p.determinant_lower}));
            cases.push(value_codec::json!({"radii":radii,"direction":sign,"allFacesInjective":faces.all_faces_injective,
            "faces":faces.faces.iter().map(|f|value_codec::json!({"face":f.face,"proven":f.absence_proven(),
                "reason":f.result.as_ref().map(|r|r.reason),"quotientProven":f.quotient.as_ref().is_some_and(|q|q.result.proven),
                "quotientReason":f.quotient.as_ref().map(|q|q.result.reason)})).collect::<Vec<_>>(),
            "middleProposals":middle,"middlePMatrix":polar,"embedding":embedding,
            "allPairsClassified":pairs.all_pairs_classified,"totalPairs":pairs.total_pairs,
            "visitedPairs":pairs.pairs.len(),"nextPair":pairs.next_pair,
            "pairs":pairs.pairs.iter().map(|p|value_codec::json!({"faces":p.faces,"reason":p.reason})).collect::<Vec<_>>()
        }));
        }
    }
    std::fs::write(output,value_codec::to_string_pretty(&value_codec::json!({"schema":"cad-variable-annular-self-qualification/1","limits":{"faceSpans":10000,"pairs":1000,"cells":1000000,"domainCells":8000000,"cellsPerPair":if joint {4096}else{128},"domainCellsPerPair":if joint {100000}else{2000},"exactWork":1000000,"trimPairs":10000,"trimCells":10000,"trimDomainCells":1000000,"spans":10000,"toleranceMm":1e-8},"cases":cases})).unwrap()).unwrap();
}
