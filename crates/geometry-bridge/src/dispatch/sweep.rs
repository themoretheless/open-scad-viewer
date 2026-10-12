//! Bridge operations of the `sweep` family.
use crate::*;

const OPS: &[&str] = &[
    "brep_sweep_viewport_evidence",
    "brep_progressive_profile_body",
    "brep_sweep_law_payload",
    "brep_sweep_constructor",
    "brep_sweep_stream_start",
    "brep_sweep_stream_next",
    "brep_sweep_stream_release",
    "brep_miter_body",
    "brep_transform_certified_miter",
    "brep_smooth_certified_miter",
    "brep_reconstruct_certified_miter",
    "brep_progressive_miter_body",
    "brep_sweep_release_owner",
    "brep_sweep_solid_admission",
    "brep_miter_correct_sections",
    "brep_sweep_retained_wall_charts_audit",
    "brep_sweep_cap_pairs_audit",
    "brep_sweep_cap_evidence_audit",
    "brep_miter_station_smoothness_audit",
    "brep_miter_profile_smoothness_audit",
    "brep_sweep_retained_decomposition_audit",
    "brep_sweep_retained_correspondence_audit",
    "brep_sweep_retained_cap_decomposition_audit",
    "brep_sweep_retained_caps_audit",
    "brep_sweep_cap_contacts_audit",
    "brep_sweep_embedding_audit",
    "brep_sweep_volume_audit",
];

pub(crate) fn route(v: Value) -> Routed {
    if !OPS.contains(&v["op"].as_str().unwrap_or("")) {
        return Routed::Unhandled(v);
    }
    Routed::Handled(handle(v))
}

fn handle(v: Value) -> Result<Value> {
    match v["op"].as_str().unwrap_or("") {
        "brep_sweep_viewport_evidence" => sweep_viewport::read(v),
        "brep_progressive_profile_body" => sweep_pipeline::profile_body(v),
        "brep_sweep_law_payload" => sweep_pipeline::law_payload(v),
        "brep_sweep_constructor" => sweep_pipeline::constructor(v),
        "brep_sweep_stream_start" => sweep_pipeline::stream_start(v),
        "brep_sweep_stream_next" => sweep_pipeline::stream_next(v),
        "brep_sweep_stream_release" => sweep_pipeline::stream_release(v),
        "brep_miter_body" => sweep_pipeline::miter(v),
        "brep_transform_certified_miter" => sweep_pipeline::transform(v),
        "brep_smooth_certified_miter" => sweep_pipeline::smooth(v,false),
        "brep_reconstruct_certified_miter" => sweep_pipeline::smooth(v,true),
        "brep_progressive_miter_body" => sweep_pipeline::progressive_miter(v),
        "brep_sweep_release_owner" => sweep_pipeline::release(v),
        "brep_sweep_solid_admission" => sweep_pipeline::admission(v),
        "brep_miter_correct_sections" => sweep_pipeline::correct(v),
        "brep_sweep_retained_wall_charts_audit" => {
            let report = brep_core::miter_seams::inspect_charts(&field(&v,"model")?,&field::<Vec<usize>>(&v,"capFaces")?,field(&v,"maxCells")?)?;
            let charts: Vec<_> = report.charts.iter().map(|(face,a)| json!({"face":face,"audit":{
                "certified":a.certified,"cells":a.cells,"projection":a.projection,"reason":a.reason,"globalEmbeddingCertified":false}})).collect();
            Ok(json!({"allChartsCertified":report.certified,"cells":report.cells,"charts":charts,
                "unresolvedFaces":report.unresolved,"globalEmbeddingCertified":false}))
        }
        "brep_sweep_cap_pairs_audit" => {
            sweep_cap_evidence::inspect_pairs(&field(&v,"model")?,&field::<Vec<usize>>(&v,"capFaces")?,&v["budgets"])
        }
        "brep_sweep_cap_evidence_audit" => {
            let options=&v["boundaryOptions"];
            sweep_cap_evidence::inspect(&field(&v,"model")?,&field::<Vec<usize>>(&v,"capFaces")?,field(&v,"maxWalls")?,
                field(options,"tolerance")?,field(options,"maxProducts")?,field(options,"maxCells")?,field(options,"maxWork")?)
        }
        "brep_miter_station_smoothness_audit" => {
            let model = field(&v,"model")?; let caps: Vec<usize> = field(&v,"capFaces")?; let max_work = field(&v,"maxWork")?;
            let report = brep_core::miter_seams::inspect(&model,&caps,max_work,true)?;
            Ok(miter_smoothness::station(&report,max_work))
        }
        "brep_miter_profile_smoothness_audit" => {
            let model = field(&v,"model")?; let caps: Vec<usize> = field(&v,"capFaces")?; let max_work = field(&v,"maxWork")?;
            let report = brep_core::miter_seams::inspect_profile(&model,&caps,max_work)?;
            Ok(miter_smoothness::profile(&report,max_work,!caps.is_empty()))
        }
        "brep_sweep_retained_decomposition_audit" => {
            let r=brep_core::sweep_retained::decomposition(&field(&v,"model")?,&field::<Vec<Vec<Vec<Curve>>>>(&v,"sections")?,field(&v,"closed")?,field(&v,"maxProducts")?,field(&v,"maxFaces")?,field(&v,"maxExactWork")?)?;
            Ok(json!({"certified":r.error_upper.is_some(),"wallErrorUpper":r.error_upper,"inspectedFaces":r.inspected_faces,"products":r.products,"reason":r.reason}))
        }
        "brep_sweep_retained_correspondence_audit" => {
            let r=brep_core::sweep_retained::inspect(&field(&v,"model")?,&field::<Vec<Vec<Vec<Curve>>>>(&v,"sections")?,field(&v,"closed")?,field(&v,"maxFaces")?,field(&v,"maxExactWork")?)?;
            Ok(json!({"exact":r.exact,"wallErrorUpper":if r.exact {Some(0)} else {None},"inspectedFaces":r.inspected_faces,"exactWork":r.work,"reason":r.reason}))
        }
        "brep_sweep_retained_cap_decomposition_audit" => {
            let r=brep_core::sweep_retained::cap_decomposition(&field(&v,"model")?,&field::<Vec<Vec<Vec<Curve>>>>(&v,"endpoints")?,brep_core::sweep_cap_contacts::Budgets {
                max_walls:field(&v,"maxWalls")?,max_exact_work:field(&v,"maxExactWork")?,max_chart_cells:field(&v,"maxChartCells")?,max_trim_pairs:field(&v,"maxTrimPairs")?,max_trim_cells:field(&v,"maxTrimCells")?,max_trim_domain_cells:field(&v,"maxTrimDomainCells")?,
            },field(&v,"maxProducts")?,field(&v,"maxEdges")?)?;
            let regions=r.regions.map(|r|json!({"exact":r.exact,"capErrorUpper":if r.exact {Some(0)} else {None},"exactWork":r.exact_work,"faces":r.faces,"reason":r.reason}));
            Ok(json!({"certified":r.error_upper.is_some(),"capErrorUpper":r.error_upper,"products":r.products,"regions":regions,"reason":r.reason}))
        }
        "brep_sweep_retained_caps_audit" => {
            let r=brep_core::sweep_cap_contacts::inspect_retained_caps(&field(&v,"model")?,&field::<Vec<Vec<Vec<Curve>>>>(&v,"endpoints")?,brep_core::sweep_cap_contacts::Budgets {
                max_walls:field(&v,"maxWalls")?,max_exact_work:field(&v,"maxExactWork")?,max_chart_cells:field(&v,"maxChartCells")?,max_trim_pairs:field(&v,"maxTrimPairs")?,max_trim_cells:field(&v,"maxTrimCells")?,max_trim_domain_cells:field(&v,"maxTrimDomainCells")?,
            },field(&v,"maxEdges")?)?;
            Ok(json!({"exact":r.exact,"capErrorUpper":if r.exact {Some(0)} else {None},"exactWork":r.exact_work,"faces":r.faces,"reason":r.reason}))
        }
        "brep_sweep_cap_contacts_audit" => {
            let report=brep_core::sweep_cap_contacts::inspect(&field::<brep_core::Model>(&v,"model")?,field(&v,"capFace")?,
                &field::<Vec<usize>>(&v,"capFaces")?,brep_core::sweep_cap_contacts::Budgets {
                    max_walls:field(&v,"maxWalls")?,max_exact_work:field(&v,"maxExactWork")?,
                    max_chart_cells:field(&v,"maxChartCells")?,max_trim_pairs:field(&v,"maxTrimPairs")?,
                    max_trim_cells:field(&v,"maxTrimCells")?,max_trim_domain_cells:field(&v,"maxTrimDomainCells")?,
                })?;
            Ok(json!({"capCertified":report.cap_certified,"planarControlHullCertified":report.planar_control_hull_certified,"allCapWallContactsCertified":report.all_cap_wall_contacts_certified,
                "separatedWalls":report.separated_walls,"allowedBoundaries":report.allowed_boundaries,
                "unresolvedWalls":report.unresolved_walls,"exactWork":report.exact_work,"reason":report.reason,
                "globalEmbeddingCertified":false}))
        }
        "brep_sweep_embedding_audit" => cad_face_contacts::diagnose_sweep_embedding(v),
        "brep_sweep_volume_audit" => cad_face_contacts::diagnose_sweep_volume(v),
        _ => unreachable!("route admits only OPS"),
    }
}
