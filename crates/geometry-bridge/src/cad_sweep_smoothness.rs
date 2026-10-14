//! Serialization only; topology ownership and exact admission live in Rust kernels.
use super::{Result, Value, field};
use brep_core::sweep_smoothness::{self, Axis, AxisReport};
use nurbs_core::continuity::ProjectiveSeamCollectionReport;
use value_codec::json;
fn collection(r: &ProjectiveSeamCollectionReport, order: usize) -> Value {
    json!({"method":"constant-projective-strip-jets","exactG1G2Certified":r.certified,
        "certifiedOrder":r.certified_order,"exactWork":r.exact_work,"unresolvedSeams":r.unresolved_seams,
        "seams":r.seams.iter().map(|p|json!({"method":"constant-projective-strip-jets",
            "certified":p.certified,"exactIdentity":p.exact_identity,"regularityCertified":p.regularity_certified,
            "work":p.work,"reason":p.reason,"certifiedOrder":if p.certified {Some(order)}else{None}})).collect::<Vec<_>>()})
}
fn station(r: &AxisReport) -> Value {
    json!({"method":"retained-miter-station-joins","scope":"wall-station-seams",
        "maxWork":r.max_work,"exactWork":r.exact_work,"extractionComplete":r.extraction_complete,
        "unclassifiedFaces":r.unclassified_faces,"unpairedEdges":r.unpaired_edges,"capEdges":r.cap_edges,
        "edgeIds":r.edge_ids,"g2":collection(&r.g2,2),"g1Audit":r.g1.as_ref().map(|p|collection(p,1)),
        "stationG1Certified":r.g1_certified,"stationG2Certified":r.g2_certified})
}
pub fn diagnose(v: Value, profile: bool) -> Result<Value> {
    let model: brep_core::Model = field(&v, "model")?;
    let caps: Vec<usize> = field(&v, "capFaces")?;
    let max_work = field(&v, "maxWork")?;
    if !profile {
        return Ok(station(&sweep_smoothness::inspect_axis(
            &model,
            &caps,
            Axis::Station,
            max_work,
        )?));
    }
    let r = sweep_smoothness::inspect_profile(&model, &caps, max_work)?;
    let p = &r.profile;
    Ok(
        json!({"method":"retained-miter-profile-joins","scope":"wall-profile-seams",
        "maxWork":p.max_work,"exactWork":p.exact_work,"extractionComplete":p.extraction_complete,
        "unclassifiedFaces":p.unclassified_faces,"unpairedEdges":p.unpaired_edges,"edgeIds":p.edge_ids,
        "profile":collection(&p.g2,2),"g1Audit":p.g1.as_ref().map(|a|collection(a,1)),
        "profileG1Certified":p.g1_certified,"g1Method":if p.g2_certified {"implied-by-G2"}
            else if p.g1_certified {"exact-projective-audit"}else{"unproved"},
        "station":station(&r.station),"totalExactWork":r.total_exact_work,
        "stationContinuity":r.station_continuity,"capContinuity":r.cap_continuity,
        "fullBoundarySmoothnessCertified":false}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn protocol_preserves_owned_sets_shared_work_and_refusal() {
        let sections = [0., 3., 10.]
            .iter()
            .map(|&z| {
                vec![vec![
                    nurbs_core::primitives::circle([0., 0., z], [0., 0., 1.], 0.5).unwrap(),
                ]]
            })
            .collect::<Vec<_>>();
        let model = brep_core::rational_section_loft(&sections).unwrap();
        let caps = [model.faces.len() - 2, model.faces.len() - 1];
        let v = json!({"model":model,"capFaces":caps,"maxWork":2000000});
        let cap_request = json!({"model":model,"endpoints":[sections[0],sections[2]],
            "maxProducts":100000,"maxEdges":1024,"maxWalls":1024,"maxExactWork":1000000,
            "maxChartCells":100000,"maxTrimPairs":100000,"maxTrimCells":100000,"maxTrimDomainCells":100000});
        let cap_decomposition = diagnose_cap_decomposition(cap_request.clone()).unwrap();
        assert_eq!(cap_decomposition["certified"], true);
        assert_eq!(cap_decomposition["regions"]["exact"], true);
        assert_eq!(cap_decomposition["regions"]["continuousBound"], false);
        assert_eq!(
            cap_decomposition["regions"]["globalEmbeddingCertified"],
            false
        );
        let mut zero_caps = cap_request;
        zero_caps["maxProducts"] = json!(0);
        let zero_caps = diagnose_cap_decomposition(zero_caps).unwrap();
        assert_eq!(zero_caps["certified"], false);
        assert!(zero_caps["capErrorUpper"].is_null());
        let decomposition = diagnose_decomposition(json!({"model":model,"sections":sections,
            "closed":false,"maxFaces":1024,"maxExactWork":1000000,"maxProducts":100000}))
        .unwrap();
        assert_eq!(decomposition["certified"], true);
        assert_eq!(decomposition["inspectedFaces"], 8);
        assert!(decomposition["wallErrorUpper"].as_f64().unwrap() < 1e-10);
        let zero_decomposition = diagnose_decomposition(json!({"model":model,"sections":sections,
            "closed":false,"maxFaces":1024,"maxExactWork":1000000,"maxProducts":0}))
        .unwrap();
        assert_eq!(zero_decomposition["certified"], false);
        assert!(zero_decomposition["wallErrorUpper"].is_null());
        let correspondence = diagnose_correspondence(json!({"model":model,"sections":sections,
            "closed":false,"maxFaces":1024,"maxExactWork":1000000}))
        .unwrap();
        assert_eq!(correspondence["exact"], true);
        assert_eq!(correspondence["wallErrorUpper"], 0);
        assert_eq!(correspondence["inspectedFaces"], 8);
        let short = diagnose_correspondence(json!({"model":model,"sections":sections,
            "closed":false,"maxFaces":1024,"maxExactWork":1}))
        .unwrap();
        assert_eq!(short["exact"], false);
        assert!(short["wallErrorUpper"].is_null());
        let charts =
            diagnose_charts(json!({"model":model,"capFaces":caps,"maxCells":10000})).unwrap();
        assert_eq!(charts["allChartsCertified"], true);
        assert_eq!(charts["globalEmbeddingCertified"], false);
        assert_eq!(charts["charts"].as_array().unwrap().len(), 8);
        let zero_charts =
            diagnose_charts(json!({"model":model,"capFaces":caps,"maxCells":0})).unwrap();
        assert_eq!(zero_charts["allChartsCertified"], false);
        assert_eq!(zero_charts["cells"], 0);
        assert_eq!(zero_charts["unresolvedFaces"].as_array().unwrap().len(), 8);
        let r = diagnose(v.clone(), true).unwrap();
        assert_eq!(r["profileG1Certified"], true);
        assert_eq!(r["profile"]["exactG1G2Certified"], true);
        assert_eq!(r["station"]["stationG2Certified"], true);
        assert_eq!(r["stationContinuity"], "G2");
        assert_eq!(r["capContinuity"], "C0");
        assert_eq!(r["fullBoundarySmoothnessCertified"], false);
        let total: u64 = field(&r, "totalExactWork").unwrap();
        let profile: u64 = field(&r, "exactWork").unwrap();
        let station: u64 = field(&r["station"], "exactWork").unwrap();
        assert_eq!(total, profile + station);
        assert!(total <= 2000000);
        let mut zero = v.clone();
        zero["maxWork"] = json!(0);
        let r = diagnose(zero, true).unwrap();
        assert_eq!(r["profileG1Certified"], false);
        assert_eq!(r["totalExactWork"], 0);
        let mut duplicate = v;
        duplicate["capFaces"] = json!([caps[0], caps[0]]);
        assert!(diagnose(duplicate, false).is_err());
    }
}

pub(crate) fn charts_value(r: &brep_core::sweep_retained_charts::Report) -> Value {
    json!({"allChartsCertified":r.certified,"cells":r.cells,"unresolvedFaces":r.unresolved_faces,
        "globalEmbeddingCertified":false,"charts":r.charts.iter().map(|(face,p)|json!({"face":face,
            "audit":{"certified":p.certified,"cells":p.cells,"projection":p.projection,
                "reason":p.reason,"globalEmbeddingCertified":false}})).collect::<Vec<_>>() })
}

pub fn diagnose_charts(v: Value) -> Result<Value> {
    let model: brep_core::Model = field(&v, "model")?;
    let r = brep_core::sweep_retained_charts::inspect(
        &model,
        &field::<Vec<usize>>(&v, "capFaces")?,
        field(&v, "maxCells")?,
    )?;
    Ok(charts_value(&r))
}

pub fn diagnose_correspondence(v: Value) -> Result<Value> {
    let model: brep_core::Model = field(&v, "model")?;
    let sections: Vec<Vec<Vec<nurbs_core::curve::Curve>>> = field(&v, "sections")?;
    let r = brep_core::sweep_retained_walls::inspect_correspondence(
        &model,
        &sections,
        field(&v, "closed")?,
        field(&v, "maxFaces")?,
        field(&v, "maxExactWork")?,
    )?;
    Ok(
        json!({"exact":r.certified,"wallErrorUpper":if r.certified {Some(0)}else{None},
        "inspectedFaces":r.inspected_faces,"reason":r.reason}),
    )
}

pub fn diagnose_decomposition(v: Value) -> Result<Value> {
    let model: brep_core::Model = field(&v, "model")?;
    let sections: Vec<Vec<Vec<nurbs_core::curve::Curve>>> = field(&v, "sections")?;
    let r = brep_core::sweep_retained_decomposition::inspect(
        &model,
        &sections,
        field(&v, "closed")?,
        field(&v, "maxProducts")?,
        field(&v, "maxFaces")?,
        field(&v, "maxExactWork")?,
    )?;
    Ok(
        json!({"certified":r.certified,"wallErrorUpper":r.wall_error_upper,
        "inspectedFaces":r.inspected_faces,"products":r.products,"reason":r.reason}),
    )
}

pub fn diagnose_cap_decomposition(v: Value) -> Result<Value> {
    let model: brep_core::Model = field(&v, "model")?;
    let endpoints = field::<[Vec<Vec<nurbs_core::curve::Curve>>; 2]>(&v, "endpoints")?;
    let budgets = brep_core::sweep_cap_contacts::Budgets {
        max_walls: field(&v, "maxWalls")?,
        max_exact_work: field(&v, "maxExactWork")?,
        max_chart_cells: field(&v, "maxChartCells")?,
        max_trim_pairs: field(&v, "maxTrimPairs")?,
        max_trim_cells: field(&v, "maxTrimCells")?,
        max_trim_domain_cells: field(&v, "maxTrimDomainCells")?,
    };
    let r = brep_core::sweep_retained_decomposition::inspect_caps(
        &model,
        &endpoints,
        budgets,
        field(&v, "maxProducts")?,
        field(&v, "maxEdges")?,
    )?;
    Ok(
        json!({"certified":r.certified,"capErrorUpper":r.cap_error_upper,"products":r.products,
        "reason":r.reason,"regions":r.regions.as_ref().map(|p|json!({"exact":p.exact,
            "capErrorUpper":if p.exact {Some(0.)}else{None},"exactWork":p.exact_work,
            "faces":[model.faces.len().saturating_sub(2),model.faces.len().saturating_sub(1)],
            "inspectedEdges":p.inspected_edges,"reason":p.reason,"continuousBound":false,
            "globalEmbeddingCertified":false,"solidCertified":false}))}),
    )
}
