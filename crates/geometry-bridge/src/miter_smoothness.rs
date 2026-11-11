//! Serialization of native retained-seam qualification.
use brep_core::miter_seams::Smoothness;
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
