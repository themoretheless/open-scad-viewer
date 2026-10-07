//! Read-only contact diagnostics. Visiting all pairs is not a completeness
//! certificate: unresolved boundaries and unclassified contacts remain visible.
use super::{Result, Value, field};
use brep_core::face_contacts::Limits;
use value_codec::json;
fn group_output(report:&brep_core::face_contacts::Report)->Value {
    json!(report.disjoint_groups.iter().map(|c|json!({"face":c.face,"range":c.range,
        "axis":c.axis,"firstBeforeRange":c.first_before_range,
        "firstBounds":c.first_bounds,"rangeBounds":c.range_bounds})).collect::<Vec<_>>())
}
pub fn diagnose_sweep_volume(v:Value)->Result<Value> {
    diagnose_sweep_volume_with_projections(v,&[])
}
pub(super) fn diagnose_sweep_volume_with_projections(v:Value,proposals:&[Option<[[i8;3];2]>])->Result<Value> {
    let model:brep_core::Model=field(&v,"model")?;
    let cap=field::<Value>(&v,"capBudgets")?;
    let r=brep_core::volume_validity::inspect_sweep_with_projections(&model,field(&v,"toleranceUv")?,
        brep_core::volume_validity::Limits {
            boundary:brep_core::boundary_embedding::Limits {
                exact_work:field(&v,"maxExactWork")?,trim_pairs:field(&v,"maxTrimPairs")?,
                trim_cells:field(&v,"maxTrimCells")?,trim_domain_cells:field(&v,"maxTrimDomainCells")?,
                spans:field(&v,"maxSpans")?,contacts:Limits {
                    pairs:field(&v,"maxPairs")?,cells:field(&v,"maxCells")?,
                    domain_cells:field(&v,"maxDomainCells")?,cells_per_pair:field(&v,"cellsPerPair")?,
                    domain_cells_per_pair:field(&v,"domainCellsPerPair")?,
                },
            },
            nesting_pairs:field(&v,"nestingPairs")?,nesting_cells:field(&v,"nestingCells")?,
            nesting_domain_cells:field(&v,"nestingDomainCells")?,orientation_cells:field(&v,"orientationCells")?,
            orientation_domain_cells:field(&v,"orientationDomainCells")?,orientation_spans:field(&v,"orientationSpans")?,
        },field(&v,"maxLinearCells")?,&field::<Vec<usize>>(&v,"capFaces")?,
        brep_core::sweep_cap_contacts::Budgets {
            max_walls:field(&cap,"maxWalls")?,max_exact_work:field(&cap,"maxExactWork")?,
            max_chart_cells:field(&cap,"maxChartCells")?,max_trim_pairs:field(&cap,"maxTrimPairs")?,
            max_trim_cells:field(&cap,"maxTrimCells")?,max_trim_domain_cells:field(&cap,"maxTrimDomainCells")?,
        },proposals)?;
    Ok(json!({"solidGeometryCertified":r.proven,"boundaryEmbeddingCertified":r.boundary.proven,
        "allFacesInjective":r.boundary.intersections.faces.all_faces_injective,
        "allPairsClassified":r.boundary.intersections.pairs.all_pairs_classified,
        "nextPair":r.boundary.intersections.pairs.next_pair,
        "individualPairs":r.boundary.intersections.pairs.pairs.len(),
        "groupedPairs":r.boundary.intersections.pairs.grouped_pairs,
        "groupCells":r.boundary.intersections.pairs.group_cells,
        "contactCells":r.boundary.intersections.pairs.cells,"contactDomainCells":r.boundary.intersections.pairs.domain_cells,
        "disjointGroups":group_output(&r.boundary.intersections.pairs),
        "nesting":r.nesting.as_ref().map(|n|json!({"rolesConsistent":n.roles_consistent,
            "parents":n.parents,"totalPairs":n.total_pairs,"visitedPairs":n.pairs.len(),
            "cells":n.cells,"domainCells":n.domain_cells,
            "pairs":n.pairs.iter().map(|p|json!({"shells":p.shells,"reason":p.result.reason,
                "boundarySeparationCertified":p.result.boundary_separation_certified,
                "separationLower":p.result.boundary.lower_bound_mm})).collect::<Vec<_>>() })),
        "orientationCells":r.orientation_cells,"orientationDomainCells":r.orientation_domain_cells,
        "orientations":r.orientations.iter().map(|s|json!({"shell":s.shell,"expectedOutward":s.expected_outward,
            "outward":s.outward,"attempts":s.attempts.len()})).collect::<Vec<_>>() }))
}
pub fn diagnose_sweep_embedding(v:Value)->Result<Value> {
    let model:brep_core::Model=field(&v,"model")?;
    let cap=field::<Value>(&v,"capBudgets")?;
    let r=brep_core::boundary_embedding::inspect_sweep(&model,field(&v,"toleranceUv")?,
        brep_core::boundary_embedding::Limits {
            exact_work:field(&v,"maxExactWork")?,trim_pairs:field(&v,"maxTrimPairs")?,
            trim_cells:field(&v,"maxTrimCells")?,trim_domain_cells:field(&v,"maxTrimDomainCells")?,
            spans:field(&v,"maxSpans")?,contacts:Limits {
                pairs:field(&v,"maxPairs")?,cells:field(&v,"maxCells")?,
                domain_cells:field(&v,"maxDomainCells")?,cells_per_pair:field(&v,"cellsPerPair")?,
                domain_cells_per_pair:field(&v,"domainCellsPerPair")?,
            },
        },field(&v,"maxLinearCells")?,&field::<Vec<usize>>(&v,"capFaces")?,
        brep_core::sweep_cap_contacts::Budgets {
            max_walls:field(&cap,"maxWalls")?,max_exact_work:field(&cap,"maxExactWork")?,
            max_chart_cells:field(&cap,"maxChartCells")?,max_trim_pairs:field(&cap,"maxTrimPairs")?,
            max_trim_cells:field(&cap,"maxTrimCells")?,max_trim_domain_cells:field(&cap,"maxTrimDomainCells")?,
        })?;
    Ok(json!({"boundaryEmbeddingCertified":r.proven,"solidGeometryCertified":false,
        "exactBoundaryCertified":r.agreement.all_equal && r.agreement.all_joins_exact,
        "trimCertified":r.trim.all_valid,"allFacesInjective":r.intersections.faces.all_faces_injective,
        "linearCells":r.intersections.faces.linear_cells,"spans":r.intersections.faces.spans,
        "allPairsClassified":r.intersections.pairs.all_pairs_classified,
        "totalPairs":r.intersections.pairs.total_pairs,"nextPair":r.intersections.pairs.next_pair,
        "individualPairs":r.intersections.pairs.pairs.len(),"groupedPairs":r.intersections.pairs.grouped_pairs,
        "groupCells":r.intersections.pairs.group_cells,"disjointGroups":group_output(&r.intersections.pairs),
        "contactCells":r.intersections.pairs.cells,"contactDomainCells":r.intersections.pairs.domain_cells,
        "pairs":r.intersections.pairs.pairs.iter().map(|p|json!({"faces":p.faces,"reason":p.reason,
            "allowedBoundary":p.boundary.is_some()})).collect::<Vec<_>>(),
        "unresolvedFaces":r.intersections.faces.faces.iter().filter(|f|
            !f.result.as_ref().is_some_and(|r|r.proven) && !f.linear.as_ref().is_some_and(|r|r.certified))
            .map(|f|f.face).collect::<Vec<_>>(),
        "caps":r.cap_contacts.iter().map(|(cap,r)|json!({"face":cap,"capCertified":r.cap_certified,
            "planarControlHullCertified":r.planar_control_hull_certified,
            "allowedBoundaries":r.allowed_boundaries,"unresolvedWalls":r.unresolved_walls,"reason":r.reason}))
            .collect::<Vec<_>>() }))
}
pub fn diagnose(v: Value) -> Result<Value> { diagnose_inner(v, false) }
pub fn diagnose_self_intersection(v: Value) -> Result<Value> { diagnose_inner(v, true) }
fn diagnose_inner(v: Value, combined: bool) -> Result<Value> {
    let model: brep_core::Model = field(&v, "model")?;
    let tolerance_uv: f64 = field(&v, "toleranceUv")?;
    let limits = Limits {
        pairs: field(&v, "maxPairs")?,
        cells: field(&v, "maxCells")?,
        domain_cells: field(&v, "maxDomainCells")?,
        cells_per_pair: field(&v, "cellsPerPair")?,
        domain_cells_per_pair: field(&v, "domainCellsPerPair")?,
    };
    let max_boxes: usize = field(&v, "maxBoxes")?;
    if max_boxes > 4096 {
        return Err(nurbs_core::Error::new(
            "BREP_CONTACT_OUTPUT",
            "Return at most 4096 unresolved boxes",
        ));
    }
    let (report, face_report) = if combined {
        let max_spans: usize = field(&v, "maxSpans")?;
        let result = brep_core::self_intersection::inspect(&model, tolerance_uv, max_spans, limits)?;
        (result.pairs, Some((result.faces, max_spans, result.absence_proven)))
    } else { (brep_core::face_contacts::inspect(&model, tolerance_uv, limits)?, None) };
    let mut exported = 0;
    let mut unresolved_boxes = 0;
    let mut contacts = 0;
    let mut shared_boundaries = 0;
    let mut disjoint = 0;
    let mut unresolved_pairs = 0;
    let pairs=report.pairs.iter().map(|p| {
        let (witness,cells,domain_cells,boxes,count)=if let Some(r)=&p.result {
            let witness=r.contact.as_ref().map(|w|json!({"firstUv":w.first_uv,"secondUv":w.second_uv,"pointIntervalMm":w.point,"contractionUpper":w.contraction_upper}));
            let count=r.unresolved.len();
            let boxes=r.unresolved.iter().take(max_boxes-exported).copied().collect::<Vec<_>>();
            exported+=boxes.len(); unresolved_boxes+=count;
            if r.contact.is_some() {contacts+=1;}
            if r.absence_proven {disjoint+=1;} else {unresolved_pairs+=1;}
            (witness,r.cells+p.hull_cells,r.domain_cells,boxes,count)
        } else {if p.boundary.is_some() {shared_boundaries+=1;} else if p.hull_disjoint {disjoint+=1;} else {unresolved_pairs+=1;}(None,p.hull_cells,0,Vec::new(),0)};
        json!({"faces":p.faces,"status":p.reason,"sharedBoundary":p.boundary.as_ref().map(|c|match c {
            brep_core::face_contacts::SharedBoundary::SweepCap{cap,wall,edge}=>json!({"kind":"sweep-cap","cap":cap,"wall":wall,"edge":edge}),
            brep_core::face_contacts::SharedBoundary::ExactHull(c)=>json!({"kind":"exact-hull","faces":c.faces,"edges":c.edges,"vertex":c.vertex,"hullIntersection":c.hull_intersection}),
            brep_core::face_contacts::SharedBoundary::PlanarFace(c)=>json!({"kind":"planar-face","edge":c.edge,"planarFace":c.planar_face,"sidedFace":c.sided_face}),
            brep_core::face_contacts::SharedBoundary::OppositeSides(c)=>json!({"kind":"opposite-sides","edge":c.edge,"faces":c.faces}),
        }),"witness":witness,"cells":cells,"domainCells":domain_cells,"unresolvedBoxes":boxes,"unresolvedBoxCount":count})
    }).collect::<Vec<_>>();
    let mut output = json!({"method":"interval-trimmed-face-contacts","scope":"distinct-face-pairs","solidGeometryStatus":"not-certified",
        "totalPairs":report.total_pairs,"visitedPairs":pairs.len(),"unvisitedPairs":report.total_pairs-pairs.len()-report.grouped_pairs,"nextPair":report.next_pair,
        "groupedPairs":report.grouped_pairs,"groupCells":report.group_cells,"disjointGroups":group_output(&report),
        "allPairsVisited":report.next_pair.is_none(),"allPairsDisjoint":report.all_pairs_disjoint,"allPairsClassified":report.all_pairs_classified,"sharedBoundaryPairCount":shared_boundaries,
        "contactPairCount":contacts,"disjointPairCount":disjoint,"unresolvedPairCount":unresolved_pairs,
        "unresolvedBoxCount":unresolved_boxes,"exportedBoxCount":exported,"boxesTruncated":exported<unresolved_boxes,
        "cells":report.cells,"domainCells":report.domain_cells,"toleranceUv":tolerance_uv,
        "limits":{"maxPairs":limits.pairs,"maxCells":limits.cells,"maxDomainCells":limits.domain_cells,"cellsPerPair":limits.cells_per_pair,"domainCellsPerPair":limits.domain_cells_per_pair,"maxBoxes":max_boxes},"pairs":pairs});
    if let Some((faces, max_spans, absence_proven)) = face_report {
        output["scope"] = json!("within-face-and-distinct-face-pairs");
        output["absenceProven"] = json!(absence_proven);
        output["allFacesInjective"] = json!(faces.all_faces_injective);
        output["spans"] = json!(faces.spans);
        output["maxSpans"] = json!(max_spans);
        output["faces"] = json!(faces.faces.iter().map(|face| json!({
            "face": face.face,
            "result": face.result.as_ref().map(|r| json!({"proven":r.proven,"projection":r.projection,
                "contractionUpper":r.contraction_upper,"spans":r.spans,"reason":r.reason}))
        })).collect::<Vec<_>>());
    }
    Ok(output)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn request(model: &brep_core::Model) -> Value {
        json!({"op":"cad_face_contacts","model":model,"toleranceUv":1e-8,"maxPairs":100,"maxCells":10000,"maxDomainCells":100000,"cellsPerPair":16,"domainCellsPerPair":1000,"maxBoxes":2})
    }
    #[test]
    fn combined_diagnostics_keep_face_limits_and_volume_scope_explicit() {
        let model = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
        let mut q = request(&model);
        q["op"] = json!("cad_self_intersection");
        q["maxSpans"] = json!(6);
        let report = crate::dispatch(q.clone()).unwrap();
        assert_eq!(report["absenceProven"], json!(true));
        assert_eq!(report["solidGeometryStatus"], json!("not-certified"));
        assert_eq!(report["scope"], json!("within-face-and-distinct-face-pairs"));
        assert_eq!(field::<Vec<Value>>(&report, "faces").unwrap().len(), 6);
        q["maxSpans"] = json!(1);
        let report = crate::dispatch(q.clone()).unwrap();
        assert_eq!(report["absenceProven"], json!(false));
        assert_eq!(report["allPairsClassified"], json!(true));
        let faces: Vec<Value> = field(&report, "faces").unwrap();
        assert_eq!(faces[1]["result"], Value::Null);
        q["maxSpans"] = json!(6);
        q["maxPairs"] = json!(1);
        let report = crate::dispatch(q).unwrap();
        assert_eq!(report["absenceProven"], json!(false));
        assert_eq!(report["allFacesInjective"], json!(true));
        assert_eq!(report["unvisitedPairs"], json!(14));
    }
    #[test]
    fn visited_is_not_complete_and_output_truncation_is_explicit() {
        let m = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
        let before = value_codec::to_value(&m).unwrap();
        let r = crate::dispatch(request(&m)).unwrap();
        assert_eq!(r["visitedPairs"], json!(15));
        assert_eq!(r["allPairsVisited"], json!(true));
        assert_eq!(r["allPairsDisjoint"], json!(false));
        assert_eq!(r["solidGeometryStatus"], json!("not-certified"));
        assert_eq!(r["exportedBoxCount"], json!(0));
        assert_eq!(r["boxesTruncated"], json!(false));
        assert_eq!(r["contactPairCount"], json!(0));
        assert_eq!(value_codec::to_value(&m).unwrap(), before);
        let pairs: Vec<Value> = field(&r, "pairs").unwrap();
        let exported: usize = pairs
            .iter()
            .map(|p| field::<Vec<Value>>(p, "unresolvedBoxes").unwrap().len())
            .sum();
        assert_eq!(exported, 0);
        assert_eq!(r["sharedBoundaryPairCount"], json!(12));
        assert_eq!(r["allPairsClassified"], json!(true));
        let mut q = request(&m);
        q["maxPairs"] = json!(1);
        q["maxBoxes"] = json!(0);
        let r = crate::dispatch(q).unwrap();
        assert_eq!(r["visitedPairs"], json!(1));
        assert_eq!(r["unvisitedPairs"], json!(14));
        assert_eq!(r["nextPair"], json!([0, 2]));
        assert_eq!(r["allPairsVisited"], json!(false));
        assert_eq!(r["exportedBoxCount"], json!(0));
    }
    #[test]
    fn contact_witness_survives_without_exporting_unresolved_boxes() {
        let mut m = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
        for (f, face) in m.faces.iter_mut().enumerate() {
            face.surface.control_points = (0..2)
                .map(|u| {
                    (0..2)
                        .map(|v| {
                            if f == 1 {
                                vec![u as f64, 0.5, v as f64 - 0.5]
                            } else {
                                vec![u as f64, v as f64, f as f64 * 10.]
                            }
                        })
                        .collect()
                })
                .collect();
        }
        let mut q = request(&m);
        q["maxBoxes"] = json!(0);
        let r = crate::dispatch(q).unwrap();
        assert_eq!(r["contactPairCount"], json!(1));
        assert_eq!(r["disjointPairCount"], json!(14));
        let pairs: Vec<Value> = field(&r, "pairs").unwrap();
        assert_eq!(pairs[0]["status"], json!("interior-contact"));
        let w: Value = field(&pairs[0], "witness").unwrap();
        let point: [[f64; 2]; 3] = field(&w, "pointIntervalMm").unwrap();
        for (d, x) in point.iter().zip([0.5, 0.5, 0.]) {
            assert!(d[0] <= x && d[1] >= x);
        }
        assert_eq!(r["boxesTruncated"], json!(true));
        let mut q = request(&m);
        q["maxBoxes"] = json!(4097);
        assert!(crate::dispatch(q).is_err());
        let mut q = request(&m);
        q["maxCells"] = json!(0);
        assert!(crate::dispatch(q).is_err());
    }
}
