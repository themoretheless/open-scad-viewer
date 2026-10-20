//! Read-only contact diagnostics. Visiting all pairs is not a completeness
//! certificate: unresolved boundaries and unclassified contacts remain visible.
use super::{Result, Value, field};
use brep_core::face_contacts::Limits;
use value_codec::json;
fn group_output(report: &brep_core::face_contacts::Report) -> Value {
    json!(
        report
            .disjoint_groups
            .iter()
            .map(|c| json!({"face":c.face,"range":c.range,
        "axis":c.axis,"firstBeforeRange":c.first_before_range,
        "firstBounds":c.first_bounds,"rangeBounds":c.range_bounds,
        "projection":c.projection.as_ref().map(|p| json!({"direction":p.direction,"firstBounds":p.first_bounds,"rangeBounds":p.range_bounds}))}))
            .collect::<Vec<_>>()
    )
}
pub fn diagnose_sweep_volume(v: Value) -> Result<Value> {
    diagnose_sweep_volume_with_projections(v, &[])
}
pub(super) fn diagnose_sweep_volume_with_projections(
    v: Value,
    proposals: &[Option<[[i8; 3]; 2]>],
) -> Result<Value> {
    diagnose_sweep_volume_impl(v,proposals,false)
}
pub fn diagnose_sweep_volume_and_charts(v:Value)->Result<Value> {
    diagnose_sweep_volume_impl(v,&[],true)
}
fn diagnose_sweep_volume_impl(v:Value,proposals:&[Option<[[i8;3];2]>],capture_charts:bool)->Result<Value> {
    // Reuse only a native completed session, once, with bit-identical complete
    // request bytes. No transported positive flag is an admission premise.
    if proposals.is_empty() {
        let key=volume_request_key(&v)?;
        if let Some(mut report)=VOLUME_SESSIONS.with(|r|r.borrow_mut().completed.take())
            .filter(|(previous,_)|*previous==key).map(|(_,report)|report) {
            if !capture_charts {report.as_object_mut().unwrap().remove("retainedWallCharts");}
            return Ok(report);
        }
    }
    let model: brep_core::Model = field(&v, "model")?;
    let cap = &v["capBudgets"];
    let inspect=|model:&brep_core::Model,tolerance:f64,limits:brep_core::volume_validity::Limits,cells:usize,
        caps:&[usize],cap_limits:brep_core::sweep_cap_contacts::Budgets,proposals:&[Option<[[i8;3];2]>]| {
        if capture_charts {
            brep_core::volume_validity::inspect_sweep_with_projections_and_charts(model,tolerance,limits,cells,caps,cap_limits,proposals)
        } else {
            Ok((brep_core::volume_validity::inspect_sweep_with_projections(model,tolerance,limits,cells,caps,cap_limits,proposals)?,brep_core::sweep_retained_charts::Report {certified:false,cells:0,charts:Vec::new(),unresolved_faces:Vec::new()}))
        }
    };
    let (r,charts) = inspect(
        &model,
        field(&v, "toleranceUv")?,
        brep_core::volume_validity::Limits {
            boundary: brep_core::boundary_embedding::Limits {
                exact_work: field(&v, "maxExactWork")?,
                trim_pairs: field(&v, "maxTrimPairs")?,
                trim_cells: field(&v, "maxTrimCells")?,
                trim_domain_cells: field(&v, "maxTrimDomainCells")?,
                spans: field(&v, "maxSpans")?,
                contacts: Limits {
                    pairs: field(&v, "maxPairs")?,
                    cells: field(&v, "maxCells")?,
                    domain_cells: field(&v, "maxDomainCells")?,
                    cells_per_pair: field(&v, "cellsPerPair")?,
                    domain_cells_per_pair: field(&v, "domainCellsPerPair")?,
                },
            },
            nesting_pairs: field(&v, "nestingPairs")?,
            nesting_cells: field(&v, "nestingCells")?,
            nesting_domain_cells: field(&v, "nestingDomainCells")?,
            orientation_cells: field(&v, "orientationCells")?,
            orientation_domain_cells: field(&v, "orientationDomainCells")?,
            orientation_spans: field(&v, "orientationSpans")?,
        },
        field(&v, "maxLinearCells")?,
        &field::<Vec<usize>>(&v, "capFaces")?,
        brep_core::sweep_cap_contacts::Budgets {
            max_walls: field(cap, "maxWalls")?,
            max_exact_work: field(cap, "maxExactWork")?,
            max_chart_cells: field(cap, "maxChartCells")?,
            max_trim_pairs: field(cap, "maxTrimPairs")?,
            max_trim_cells: field(cap, "maxTrimCells")?,
            max_trim_domain_cells: field(cap, "maxTrimDomainCells")?,
        },
        proposals,
    )?;
    volume_value(&r,&charts,capture_charts)
}
fn volume_value(r:&brep_core::volume_validity::Report,charts:&brep_core::sweep_retained_charts::Report,capture_charts:bool)->Result<Value> {
    let mut output=
        json!({"boundary":boundary_value(&r.boundary),"solidGeometryCertified":r.proven,"boundaryEmbeddingCertified":r.boundary.proven,
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
            "outward":s.outward,"attempts":s.attempts.len()})).collect::<Vec<_>>() });
    if capture_charts {output["retainedWallCharts"]=super::cad_sweep_smoothness::charts_value(&charts);}
    Ok(output)
}
struct VolumeSession {
    work:brep_core::volume_validity::Inspector,
    request_key:Vec<u8>,
}
#[derive(Default)]
struct VolumeRegistry {next:u64,live:std::collections::BTreeMap<String,VolumeSession>,completed:Option<(Vec<u8>,Value)>}
fn volume_request_key(v:&Value)->Result<Vec<u8>> {
    let mut request=v.clone();
    request.as_object_mut().ok_or_else(||nurbs_core::Error::new("BREP_VOLUME_SESSION","Expected volume request object"))?.remove("op");
    value_codec::encode_binary(&request).map_err(|e|nurbs_core::Error::new("BREP_VOLUME_SESSION",e.to_string()))
}
thread_local! {static VOLUME_SESSIONS:std::cell::RefCell<VolumeRegistry>=std::cell::RefCell::new(VolumeRegistry::default());}
pub fn begin_sweep_volume(v:Value)->Result<Value> {
    VOLUME_SESSIONS.with(|store| {
        let mut store=store.borrow_mut();
        if store.live.len()>=4 {return Err(nurbs_core::Error::new("BREP_VOLUME_SESSION","Sweep volume session limit reached"));}
        let model=std::sync::Arc::new(field::<brep_core::Model>(&v,"model")?);
        let cap=&v["capBudgets"];
        let (limits,linear,caps,cap_limits)=(
        brep_core::volume_validity::Limits {
            boundary: brep_core::boundary_embedding::Limits {
                exact_work: field(&v, "maxExactWork")?,
                trim_pairs: field(&v, "maxTrimPairs")?,
                trim_cells: field(&v, "maxTrimCells")?,
                trim_domain_cells: field(&v, "maxTrimDomainCells")?,
                spans: field(&v, "maxSpans")?,
                contacts: Limits {
                    pairs: field(&v, "maxPairs")?,
                    cells: field(&v, "maxCells")?,
                    domain_cells: field(&v, "maxDomainCells")?,
                    cells_per_pair: field(&v, "cellsPerPair")?,
                    domain_cells_per_pair: field(&v, "domainCellsPerPair")?,
                },
            },
            nesting_pairs: field(&v, "nestingPairs")?,
            nesting_cells: field(&v, "nestingCells")?,
            nesting_domain_cells: field(&v, "nestingDomainCells")?,
            orientation_cells: field(&v, "orientationCells")?,
            orientation_domain_cells: field(&v, "orientationDomainCells")?,
            orientation_spans: field(&v, "orientationSpans")?,
        },
        field(&v, "maxLinearCells")?,
        field::<Vec<usize>>(&v, "capFaces")?,
        brep_core::sweep_cap_contacts::Budgets {
            max_walls: field(cap, "maxWalls")?,
            max_exact_work: field(cap, "maxExactWork")?,
            max_chart_cells: field(cap, "maxChartCells")?,
            max_trim_pairs: field(cap, "maxTrimPairs")?,
            max_trim_cells: field(cap, "maxTrimCells")?,
            max_trim_domain_cells: field(cap, "maxTrimDomainCells")?,
        },
        );
        let work=brep_core::volume_validity::Inspector::new(model.clone(),field(&v,"toleranceUv")?,limits,linear,
            Some((caps.clone(),cap_limits)),vec![])?;
        store.next=store.next.checked_add(1).ok_or_else(||nurbs_core::Error::new("BREP_VOLUME_SESSION","Sweep volume session IDs exhausted"))?;
        let id=store.next.to_string();
        store.completed=None;
        let request_key=volume_request_key(&v)?;
        store.live.insert(id.clone(),VolumeSession {work,request_key});
        Ok(json!({"session":id,"done":false,"completedSteps":0}))
    })
}
pub fn step_sweep_volume(v:Value)->Result<Value> {
    let id=field::<String>(&v,"session")?;
    let units=field::<usize>(&v,"maxUnits")?;
    // Invalid caller step sizes preserve the session; native errors discard it.
    if !(1..=64).contains(&units) {return Err(nurbs_core::Error::new("BREP_VOLUME_SESSION","Sweep volume step requires 1..64 units"));}
    VOLUME_SESSIONS.with(|store| {
        let mut store=store.borrow_mut();
        let mut session=store.live.remove(&id).ok_or_else(||nurbs_core::Error::new("BREP_VOLUME_SESSION","Unknown sweep volume session"))?;
        let advanced=session.work.advance(units)?;
        let completed=session.work.completed_steps();
        if !session.work.finished() {
            store.live.insert(id.clone(),session);
            return Ok(json!({"session":id,"done":false,"advancedUnits":advanced,"completedSteps":completed}));
        }
        let (report,charts)=session.work.finish_with_charts()?;
        let value=volume_value(&report,&charts,true)?;
        if report.proven {store.completed=Some((session.request_key,value.clone()));}
        Ok(json!({"session":id,"done":true,"advancedUnits":advanced,"completedSteps":completed,"report":value}))
    })
}
pub fn drop_sweep_volume(v:Value)->Result<Value> {
    let id=field::<String>(&v,"session")?;
    let removed=VOLUME_SESSIONS.with(|store|store.borrow_mut().live.remove(&id).is_some());
    Ok(json!({"dropped":removed}))
}

pub fn diagnose_sweep_embedding(v: Value) -> Result<Value> {
    let model: brep_core::Model = field(&v, "model")?;
    let cap = &v["capBudgets"];
    let r = brep_core::boundary_embedding::inspect_sweep(
        &model,
        field(&v, "toleranceUv")?,
        brep_core::boundary_embedding::Limits {
            exact_work: field(&v, "maxExactWork")?,
            trim_pairs: field(&v, "maxTrimPairs")?,
            trim_cells: field(&v, "maxTrimCells")?,
            trim_domain_cells: field(&v, "maxTrimDomainCells")?,
            spans: field(&v, "maxSpans")?,
            contacts: Limits {
                pairs: field(&v, "maxPairs")?,
                cells: field(&v, "maxCells")?,
                domain_cells: field(&v, "maxDomainCells")?,
                cells_per_pair: field(&v, "cellsPerPair")?,
                domain_cells_per_pair: field(&v, "domainCellsPerPair")?,
            },
        },
        field(&v, "maxLinearCells")?,
        &field::<Vec<usize>>(&v, "capFaces")?,
        brep_core::sweep_cap_contacts::Budgets {
            max_walls: field(cap, "maxWalls")?,
            max_exact_work: field(cap, "maxExactWork")?,
            max_chart_cells: field(cap, "maxChartCells")?,
            max_trim_pairs: field(cap, "maxTrimPairs")?,
            max_trim_cells: field(cap, "maxTrimCells")?,
            max_trim_domain_cells: field(cap, "maxTrimDomainCells")?,
        },
    )?;
    Ok(boundary_value(&r))
}
fn boundary_value(r: &brep_core::boundary_embedding::Report) -> Value {
    json!({"boundaryEmbeddingCertified":r.proven,"solidGeometryCertified":false,
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
            .collect::<Vec<_>>() })
}
pub fn diagnose(v: Value) -> Result<Value> {
    diagnose_inner(v, false)
}
pub fn diagnose_self_intersection(v: Value) -> Result<Value> {
    diagnose_inner(v, true)
}
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
        let result =
            brep_core::self_intersection::inspect(&model, tolerance_uv, max_spans, limits)?;
        (
            result.pairs,
            Some((result.faces, max_spans, result.absence_proven)),
        )
    } else {
        (
            brep_core::face_contacts::inspect(&model, tolerance_uv, limits)?,
            None,
        )
    };
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
            (witness,r.cells+p.hull_cells+p.boundary_cells,r.domain_cells,boxes,count)
        } else {if p.boundary.is_some() {shared_boundaries+=1;} else if p.hull_disjoint {disjoint+=1;} else {unresolved_pairs+=1;}(None,p.hull_cells+p.boundary_cells,0,Vec::new(),0)};
        json!({"faces":p.faces,"status":p.reason,"sharedBoundary":p.boundary.as_ref().map(|c|match c {
            brep_core::face_contacts::SharedBoundary::SweepCap{cap,wall,edge}=>json!({"kind":"sweep-cap","cap":cap,"wall":wall,"edge":edge}),
            brep_core::face_contacts::SharedBoundary::ExactHull(c)=>json!({"kind":"exact-hull","faces":c.faces,"edges":c.edges,"vertex":c.vertex,"hullIntersection":c.hull_intersection}),
            brep_core::face_contacts::SharedBoundary::PlanarFace(c)=>json!({"kind":"planar-face","edge":c.edge,"planarFace":c.planar_face,"sidedFace":c.sided_face}),
            brep_core::face_contacts::SharedBoundary::OppositeSides(c)=>json!({"kind":"opposite-sides","edge":c.edge,"faces":c.faces}),
            brep_core::face_contacts::SharedBoundary::OwnedVertex(c)=>json!({"kind":"owned-vertex","faces":c.faces,"vertex":c.vertex,
                "trimmedFace":c.trimmed_face,"coordinates":c.coordinates,"distanceUpper":c.distance_upper,
                "hullPenetrations":c.hull_penetrations,"exactVertexContact":c.exact_vertex_contact}),
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
    fn volume_request(model:&brep_core::Model)->Value {
        json!({"model":model,"toleranceUv":1e-8,"maxExactWork":1000000,"maxTrimPairs":10000,
            "maxTrimCells":100000,"maxTrimDomainCells":1000000,"maxSpans":1024,"maxLinearCells":100000,
            "maxPairs":20000,"maxCells":200000,"maxDomainCells":1000000,"cellsPerPair":1000,"domainCellsPerPair":10000,
            "nestingPairs":1000,"nestingCells":100000,"nestingDomainCells":1000000,
            "orientationCells":100000,"orientationDomainCells":1000000,"orientationSpans":100,"capFaces":[],
            "capBudgets":{"maxWalls":1024,"maxExactWork":1000000,"maxChartCells":1000,
                "maxTrimPairs":100000,"maxTrimCells":100000,"maxTrimDomainCells":1000000}})
    }
    fn complete_owned_volume(request:Value)->Value {
        let id=field::<String>(&begin_sweep_volume(request).unwrap(),"session").unwrap();
        loop {let step=step_sweep_volume(json!({"session":id,"maxUnits":64})).unwrap();
            if field::<bool>(&step,"done").unwrap() {return step["report"].clone();}}
    }
    #[test]
    fn native_completed_volume_reuse_requires_exact_request_and_is_single_use() {
        let model=brep_core::cuboid([0.;3],[1.;3]).unwrap();let request=volume_request(&model);
        let expected=complete_owned_volume(request.clone());
        assert!(VOLUME_SESSIONS.with(|r|r.borrow().completed.is_some()));
        let actual=diagnose_sweep_volume_and_charts(request.clone()).unwrap();assert_eq!(actual,expected);
        assert!(VOLUME_SESSIONS.with(|r|r.borrow().completed.is_none()));
        complete_owned_volume(request.clone());
        let mut restricted=request.clone();restricted["maxCells"]=json!(1);
        assert_eq!(diagnose_sweep_volume_and_charts(restricted).unwrap()["solidGeometryCertified"],json!(false));
        assert!(VOLUME_SESSIONS.with(|r|r.borrow().completed.is_none()));
        complete_owned_volume(request.clone());
        let mut gap=model.clone();gap.edges[0].curve.control_points[0][2]+=1e-12;
        assert_eq!(diagnose_sweep_volume_and_charts(volume_request(&gap)).unwrap()["solidGeometryCertified"],json!(false));
        assert!(VOLUME_SESSIONS.with(|r|r.borrow().completed.is_none()));
        complete_owned_volume(request.clone());
        let id=field::<String>(&begin_sweep_volume(request.clone()).unwrap(),"session").unwrap();
        assert!(VOLUME_SESSIONS.with(|r|r.borrow().completed.is_none()));
        drop_sweep_volume(json!({"session":id})).unwrap();
        let mut positive_zero=request.clone();positive_zero["model"]["vertices"][0]["point"][0]=json!(0.0);
        let mut negative_zero=positive_zero.clone();negative_zero["model"]["vertices"][0]["point"][0]=json!(-0.0);
        assert_ne!(volume_request_key(&positive_zero).unwrap(),volume_request_key(&negative_zero).unwrap());
        let mut changed_caps=request.clone();changed_caps["capFaces"]=json!([0,1]);
        assert_ne!(volume_request_key(&request).unwrap(),volume_request_key(&changed_caps).unwrap());
    }
    #[test]
    fn owned_volume_session_matches_complete_report_and_never_emits_partial_proof() {
        let model=brep_core::cuboid([0.;3],[1.;3]).unwrap();
        for max_cells in [1,200000] {
            let mut request=volume_request(&model);request["maxCells"]=json!(max_cells);
            let expected=diagnose_sweep_volume_and_charts(request.clone()).unwrap();
            for batch in [1,2,64] {
                let start=begin_sweep_volume(request.clone()).unwrap();
                let id=field::<String>(&start,"session").unwrap();
                assert!(start.get("report").is_none());
                assert!(step_sweep_volume(json!({"session":id,"maxUnits":0})).is_err());
                let mut previous=0;let mut calls=0;
                loop {
                    let step=step_sweep_volume(json!({"session":id,"maxUnits":batch})).unwrap();
                    let completed=field::<usize>(&step,"completedSteps").unwrap();
                    assert!(completed>=previous);previous=completed;
                    calls+=1;assert!(calls<1000);
                    if field::<bool>(&step,"done").unwrap() {
                        assert_eq!(step["report"],expected);break;
                    }
                    assert!(step.get("report").is_none());
                    assert!(step.get("solidGeometryCertified").is_none());
                }
                assert!(step_sweep_volume(json!({"session":id,"maxUnits":1})).is_err());
                assert_eq!(drop_sweep_volume(json!({"session":id})).unwrap()["dropped"],json!(false));
            }
        }
    }
    #[test]
    fn owned_volume_session_drop_limits_and_stale_handles_are_enforced() {
        let model=brep_core::cuboid([0.;3],[1.;3]).unwrap();let request=volume_request(&model);
        let mut ids=vec![];
        for _ in 0..4 {ids.push(field::<String>(&begin_sweep_volume(request.clone()).unwrap(),"session").unwrap());}
        assert!(begin_sweep_volume(request.clone()).is_err());
        for id in ids {
            assert_eq!(drop_sweep_volume(json!({"session":id})).unwrap()["dropped"],json!(true));
            assert!(step_sweep_volume(json!({"session":id,"maxUnits":1})).is_err());
        }
        let id=field::<String>(&begin_sweep_volume(request).unwrap(),"session").unwrap();
        assert_eq!(drop_sweep_volume(json!({"session":id})).unwrap()["dropped"],json!(true));
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
        assert_eq!(
            report["scope"],
            json!("within-face-and-distinct-face-pairs")
        );
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
