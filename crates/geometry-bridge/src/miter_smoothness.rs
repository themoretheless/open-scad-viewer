//! Serialization of native retained-seam qualification.
use brep_core::miter_seams::{ProfileReport, Smoothness};
use value_codec::{Value, json};
fn seam_set(r: &nurbs_core::sweep_seam_set::Report, order: usize) -> Value {
    let seams: Vec<_> = r.seams.iter().map(|s| json!({"certified":s.certified,"exactIdentity":s.exact_identity,
        "regularityCertified":s.regularity_certified,"work":s.work,"reason":s.reason,
        "method":"constant-projective-strip-jets","certifiedOrder":if s.certified {Some(order)} else {None}})).collect();
    json!({"method":"constant-projective-strip-jets","exactG1G2Certified":r.certified,
        "certifiedOrder":r.order,"exactWork":r.work,"unresolvedSeams":r.unresolved,"seams":seams})
}
pub fn station(r: &Smoothness, max_work: u64) -> Value {
    json!({"method":"retained-miter-station-joins","scope":"wall-station-seams","maxWork":max_work,
        "exactWork":r.work,"extractionComplete":r.extraction.complete(),
        "unclassifiedFaces":r.extraction.unclassified_faces,"unpairedEdges":r.extraction.unpaired_edges,
        "capEdges":r.extraction.cap_edges,"edgeIds":r.extraction.edge_ids,"g2":seam_set(&r.g2,2),
        "g1Audit":r.g1.as_ref().map(|r| seam_set(r,1)),"stationG1Certified":r.g1_certified,"stationG2Certified":r.g2.certified})
}
pub fn profile(r: &ProfileReport, max_work: u64, has_caps: bool) -> Value {
    let p = &r.profile;
    json!({"method":"retained-miter-profile-joins","scope":"wall-profile-seams","maxWork":max_work,
        "exactWork":p.work,"extractionComplete":p.extraction.complete(),
        "unclassifiedFaces":p.extraction.unclassified_faces,"unpairedEdges":p.extraction.unpaired_edges,
        "edgeIds":p.extraction.edge_ids,"profile":seam_set(&p.g2,2),"g1Audit":p.g1.as_ref().map(|r| seam_set(r,1)),
        "profileG1Certified":p.g1_certified,"g1Method":if !p.g1_certified {"unproved"} else if p.g2.certified {"implied-by-G2"} else {"exact-projective-audit"},
        "station":station(&r.station,max_work-p.work),"totalExactWork":r.total_work,
        "stationContinuity":if r.station.g2.certified {"G2"} else if r.station.g1_certified {"G1"} else {"C0"},
        "capContinuity":if has_caps {"C0"} else {"absent"},"fullBoundarySmoothnessCertified":false})
}
