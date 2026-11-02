//! Fresh actual-model field/seam checks and exact retained-domain embedding.
use brep_core::{Model, boundary_embedding as b, face_curvature as c};
pub fn qualify(
    model: &Model,
    smooth: &[usize],
    dir: &std::path::Path,
) -> Result<bool, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(
        dir.join("body.json"),
        value_codec::to_string(
            &value_codec::json!({"model":model,"smoothEdges":smooth,"accepted":false}),
        )?,
    )?;
    let closure = brep_core::watertight::inspect_with_exact(model, 1_000_000, 1_000_000)?;
    let mut admitted = closure.closure.status == brep_core::watertight::Closure::WithinTolerance;
    let mut fields = Vec::new();
    let mut seams = Vec::new();
    for face in 0..model.faces.len() {
        eprintln!("phase=curvature-face-{face}");
        let r = c::inspect(
            model,
            face,
            10.,
            1e-8,
            c::Limits {
                cells: 50_000,
                domain_cells: 1_000_000,
                shape_cells: 1_000_000,
            },
            &[],
        )?;
        admitted &= r.decision == c::Decision::Within;
        fields.push(value_codec::json!({"face":face,"curvature":format!("{:?}",r.decision),"upper":r.absolute_curvature_upper,"cells":r.cells}));
        std::fs::write(
            dir.join("fields.json"),
            value_codec::to_string_pretty(&fields)?,
        )?;
    }
    for &edge in smooth {
        eprintln!("phase=whole-seam-{edge}");
        let mut uses = Vec::new();
        for (face, f) in model.faces.iter().enumerate() {
            for w in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
                for co in &model.loops[w].coedges {
                    if co.edge == edge {
                        uses.push((
                            face,
                            if co.reversed {
                                co.pcurve.reverse()?
                            } else {
                                co.pcurve.clone()
                            },
                        ));
                    }
                }
            }
        }
        assert_eq!(uses.len(), 2);
        let r = nurbs_core::surface_g2_pcurves::inspect_owned(
            &model.edges[edge].curve,
            [&uses[0].1, &uses[1].1],
            [
                &model.faces[uses[0].0].surface,
                &model.faces[uses[1].0].surface,
            ],
            [false; 2],
            1e-7,
            1e-5,
            1e-4,
            100_000,
            1_000_000,
        )?;
        admitted &= r.report.positions.proven
            && r.report.geometric.decision == nurbs_core::surface_join::Decision::WithinTolerance;
        seams.push(value_codec::json!({"edge":edge,"faces":[uses[0].0,uses[1].0],"positions":r.report.positions.proven,"g2":format!("{:?}",r.report.geometric.decision),"cells":r.report.geometric.cells,"branches":r.report.geometric.branches.iter().map(|r|value_codec::json!({"normalUpper":r.normal_upper,"tensorUpper":r.tensor_upper})).collect::<Vec<_>>()}));
        std::fs::write(
            dir.join("seams.json"),
            value_codec::to_string_pretty(&seams)?,
        )?;
    }
    eprintln!("phase=retained-global-embedding");
    let global = b::inspect_retained_rectangles(
        model,
        1e-8,
        b::Limits {
            exact_work: 1_000_000,
            trim_pairs: 10_000,
            trim_cells: 100_000,
            trim_domain_cells: 1_000_000,
            spans: 1024,
            contacts: brep_core::face_contacts::Limits {
                pairs: 10_000,
                cells: 100_000,
                domain_cells: 1_000_000,
                cells_per_pair: 1000,
                domain_cells_per_pair: 10_000,
            },
        },
        100_000,
        100_000,
    )?;
    admitted &= global.proven;
    let r = &global.proof;
    let report = value_codec::json!({"qualifiedForDeclaredGates":admitted,"closure":format!("{:?}",closure.closure.status),"faces":model.faces.len(),"edges":model.edges.len(),"vertices":model.vertices.len(),"fields":fields,"seams":seams,"globalEmbedding":global.proven,"totalPairs":r.intersections.pairs.total_pairs,"nextPair":r.intersections.pairs.next_pair,"exactEdgeLifts":r.agreement.all_equal,"exactWireJoins":r.agreement.all_joins_exact,"trimsValid":r.trim.all_valid,"allFacesInjective":r.intersections.faces.all_faces_injective,"pairs":r.intersections.pairs.pairs.iter().map(|p|value_codec::json!({"faces":p.faces,"reason":p.reason})).collect::<Vec<_>>(),"proofOnlyRestrictions":global.restrictions.iter().map(|p|value_codec::json!({"face":p.face,"axis":p.axis,"original":p.original,"retained":p.retained,"checkedCoordinates":p.checked_coordinates})).collect::<Vec<_>>(),"scope":"C0 closure, whole actual trimmed-face curvature <=10/mm, selected complete G2 seams, exact equivalent retained-domain global boundary embedding. Sharp source/end edges remain outside G2 scope; material/nesting is a separate gate."});
    std::fs::write(
        dir.join("report.json"),
        value_codec::to_string_pretty(&report)?,
    )?;
    if admitted {
        let (text, _, _) = brep_core::export_step_v5(model)?;
        std::fs::write(dir.join("candidate.step"), text)?;
    }
    eprintln!("qualifiedForDeclaredGates={admitted}");
    Ok(admitted)
}
