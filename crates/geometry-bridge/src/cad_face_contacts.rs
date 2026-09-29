//! Read-only contact diagnostics. Visiting all pairs is not a completeness
//! certificate: unresolved boundaries and unclassified contacts remain visible.
use super::{Result, Value, field};
use brep_core::face_contacts::Limits;
use value_codec::json;
pub fn diagnose(v: Value) -> Result<Value> {
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
    let report = brep_core::face_contacts::inspect(&model, tolerance_uv, limits)?;
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
            (witness,r.cells,r.domain_cells,boxes,count)
        } else {if p.boundary.is_some() {shared_boundaries+=1;} else {unresolved_pairs+=1;}(None,0,0,Vec::new(),0)};
        json!({"faces":p.faces,"status":p.reason,"sharedBoundary":p.boundary.as_ref().map(|c|match c {
            brep_core::face_contacts::SharedBoundary::PlanarFace(c)=>json!({"kind":"planar-face","edge":c.edge,"planarFace":c.planar_face,"sidedFace":c.sided_face}),
            brep_core::face_contacts::SharedBoundary::OppositeSides(c)=>json!({"kind":"opposite-sides","edge":c.edge,"faces":c.faces}),
        }),"witness":witness,"cells":cells,"domainCells":domain_cells,"unresolvedBoxes":boxes,"unresolvedBoxCount":count})
    }).collect::<Vec<_>>();
    Ok(
        json!({"method":"interval-trimmed-face-contacts","scope":"distinct-face-pairs","solidGeometryStatus":"not-certified",
        "totalPairs":report.total_pairs,"visitedPairs":pairs.len(),"unvisitedPairs":report.total_pairs-pairs.len(),"nextPair":report.next_pair,
        "allPairsVisited":report.next_pair.is_none(),"allPairsDisjoint":report.all_pairs_disjoint,"allPairsClassified":report.all_pairs_classified,"sharedBoundaryPairCount":shared_boundaries,
        "contactPairCount":contacts,"disjointPairCount":disjoint,"unresolvedPairCount":unresolved_pairs,
        "unresolvedBoxCount":unresolved_boxes,"exportedBoxCount":exported,"boxesTruncated":exported<unresolved_boxes,
        "cells":report.cells,"domainCells":report.domain_cells,"toleranceUv":tolerance_uv,
        "limits":{"maxPairs":limits.pairs,"maxCells":limits.cells,"maxDomainCells":limits.domain_cells,"cellsPerPair":limits.cells_per_pair,"domainCellsPerPair":limits.domain_cells_per_pair,"maxBoxes":max_boxes},"pairs":pairs}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    fn request(model: &brep_core::Model) -> Value {
        json!({"op":"cad_face_contacts","model":model,"toleranceUv":1e-8,"maxPairs":100,"maxCells":10000,"maxDomainCells":100000,"cellsPerPair":16,"domainCellsPerPair":1000,"maxBoxes":2})
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
