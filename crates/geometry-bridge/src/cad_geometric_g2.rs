//! Native geometric G2 ownership inspection and serialization. Reports never
//! authorize a solid or replace exact strip-identity results.
use super::{Value,Result,field};
use brep_core::{Model,sweep_smoothness::geometric::{self,Tolerances}};
use value_codec::json;
pub(crate) fn inspect(model:&Model,edges:&[usize],t:Tolerances,max_cells:usize)->Result<Value> {
    let report=geometric::inspect_edges(model,edges,t,max_cells)?;
    Ok(json!({"method":"owned-boundary-geometric-G2-intervals","complete":report.complete,"withinTolerance":report.within_tolerance,
        "positionTolerance":t.position,"normalTolerance":t.normal,"curvatureTolerance":t.curvature,
        "maxCells":max_cells,"cells":report.cells,"edgeIds":edges,"exactIdentityClaim":false,"solidGeometryCertified":false,
        "corners":report.corners.iter().map(|c|json!({"vertex":c.vertex,"edges":c.edges,"faces":c.faces,"omittedEdges":c.omitted_edges,
            "closedLink":c.closed_link,"allIncidentSeamsWithinTolerance":c.all_incident_seams_within_tolerance,"solidGeometryCertified":false})).collect::<Vec<_>>(),
        "seams":report.seams.iter().map(|s|json!({"edge":s.edge,"faces":s.faces,
            "inspection":s.inspection.as_ref().map(|r|json!({"positionBounds":r.position_bounds,"regularityProven":r.regularity_proven,"cells":r.cells,
                "decision":match r.decision {nurbs_core::surface_join::Decision::WithinTolerance=>"within-tolerance",nurbs_core::surface_join::Decision::CorrespondenceExceedsTolerance=>"exceeds-tolerance",nurbs_core::surface_join::Decision::Unresolved=>"unresolved"},
                "orientation":r.orientation.map(|o|match o {nurbs_core::surface_g2::Orientation::Same=>"same",nurbs_core::surface_g2::Orientation::Opposite=>"opposite"}),
                "branches":r.branches.iter().map(|b|json!({"normalLower":b.normal_lower,"normalUpper":b.normal_upper,"curvatureLower":b.tensor_lower,"curvatureUpper":b.tensor_upper})).collect::<Vec<_>>()
            }))})).collect::<Vec<_>>()}))
}
pub fn diagnose(v:Value)->Result<Value> {
    inspect(&field::<Model>(&v,"model")?,&field::<Vec<usize>>(&v,"edges")?,Tolerances {position:field(&v,"positionTolerance")?,normal:field(&v,"normalTolerance")?,curvature:field(&v,"curvatureTolerance")?},field(&v,"maxCells")?)
}
pub(crate) fn cap_value(model:&Model,r:&brep_core::sweep_smoothness::AxisReport,remaining:&mut usize)->Result<Value> {
    let geometric=inspect(model,&r.edge_ids,Tolerances {position:0.,normal:1e-7,curvature:1e-5},*remaining)?;
    *remaining-=geometric["cells"].as_u64().unwrap() as usize;
    Ok(json!({"method":"retained-annular-cap-internal-jets","edgeIds":r.edge_ids,
        "g1Certified":r.g1_certified,"g2Certified":r.g2_certified,"exactWork":r.exact_work,"maxWork":r.max_work,
        "unclassifiedFaces":r.unclassified_faces,"unpairedEdges":r.unpaired_edges,"extractionComplete":r.extraction_complete,
        "geometricG2":geometric}))
}

/// Rebuild from dimensions, then qualify the actual body. Caller-supplied
/// geometry or certificates are not used to construct or authorize this result.
pub fn create_annular_cylinder(v:Value)->Result<Value> {
    use brep_core::{analytic::g2_annular,boundary_embedding,face_contacts,volume_validity,sweep_cap_contacts,sweep_smoothness};
    let outer=field(&v,"outerRadius")?;let inner=field(&v,"innerRadius")?;let height=field(&v,"height")?;
    let extents=if v["transitionSizes"].is_null(){[field(&v,"transitionSize")?;4]}else{field::<[f64;4]>(&v,"transitionSizes")?};
    let mut built=g2_annular::cylinder_with_transitions(outer,inner,height,extents,1e-8)?;
    let patches=built.model.faces.iter().map(|f|f.surface.clone()).collect::<Vec<_>>();
    let mut reversed=vec![false;patches.len()];for u in &built.model.shells[0].faces{reversed[u.face]=u.reversed;}
    let assembly=brep_core::rounded_patch_network::assemble(&patches,&reversed,1e-8,1000000,&[&built.model])?;
    let assembly_work=assembly.correspondence_work;let owned_uses=assembly.directly_owned_uses;let proved_uses=assembly.proved_uses;
    built.model=assembly.model;
    let jets=sweep_smoothness::inspect_profile(&built.model,&[],2_000_000)?;
    let edges=jets.profile.edge_ids.iter().chain(&jets.station.edge_ids).copied().collect::<std::collections::BTreeSet<_>>();
    if !jets.profile.g2_certified||!jets.station.g2_certified||!jets.profile.extraction_complete||!jets.station.extraction_complete||edges.len()!=built.model.edges.len()||(0..built.model.edges.len()).any(|e|!edges.contains(&e)) {
        return Err(super::input("BREP_G2_UNPROVEN: saved annular transition jets are not certified; change the dimensions or transition size"));
    }
    let contacts=face_contacts::Limits{pairs:10000,cells:100000,domain_cells:1000000,cells_per_pair:1000,domain_cells_per_pair:10000};
    let limits=volume_validity::Limits{boundary:boundary_embedding::Limits{exact_work:1000000,trim_pairs:10000,trim_cells:100000,trim_domain_cells:1000000,spans:1024,contacts},nesting_pairs:100,nesting_cells:100000,nesting_domain_cells:1000000,orientation_cells:100000,orientation_domain_cells:1000000,orientation_spans:100};
    let volume=volume_validity::inspect_sweep_with_projections(&built.model,1e-8,limits,100000,&[],sweep_cap_contacts::Budgets{max_walls:1024,max_exact_work:1000000,max_chart_cells:1000,max_trim_pairs:10000,max_trim_cells:100000,max_trim_domain_cells:1000000},&built.projection_hints())?;
    if !volume.proven {return Err(super::input("BREP_G2_GLOBAL_UNPROVEN: annular body embedding or material orientation remains unproved"));}
    let pairs=&volume.boundary.intersections.pairs;
    Ok(json!({"model":built.model,"report":{"method":"native-g2-annular-cylinder/1","fullBoundaryG2Certified":true,
        "solidGeometryCertified":true,"profileSeams":jets.profile.edge_ids.len(),"stationSeams":jets.station.edge_ids.len(),
        "exactWork":jets.total_exact_work,"maxWork":2000000,"transitionSizes":built.transition_sizes,
        "assembly":{"method":"exact-owned-patch-network/1","correspondenceWork":assembly_work,"maxWork":1000000,"directlyOwnedUses":owned_uses,"provedUses":proved_uses},"radialWallThicknessLower":built.radial_wall_thickness_lower,"angularRadiusBounds":built.angular_radius_bounds,
        "allFacesInjective":volume.boundary.intersections.faces.all_faces_injective,"allPairsClassified":pairs.all_pairs_classified,
        "totalPairs":pairs.total_pairs,"coveredPairs":pairs.pairs.len()+pairs.grouped_pairs,"nextPair":pairs.next_pair}}))
}

/// General rational support-boundary transition proposals with independent
/// full-interval saved-geometry receipts. Open patches never authorize a solid.
pub fn create_surface_bridge(v:Value)->Result<Value> {
    use nurbs_core::{surface::Surface,surface_blend::{self,Boundary,Options},surface_g2::Report};
    let boundary=|name:&str|->Result<Boundary>{Ok(match name {
        "uMin"=>Boundary::UStart,"uMax"=>Boundary::UEnd,"vMin"=>Boundary::VStart,"vMax"=>Boundary::VEnd,
        _=>return Err(super::input("G2 bridge boundary must be uMin/uMax/vMin/vMax")),
    })};
    let names=field::<[String;2]>(&v,"boundaries")?;
    let options=Options{boundaries:[boundary(&names[0])?,boundary(&names[1])?],gains:field(&v,"gains")?,reverse_u:field(&v,"reverseU")?,max_patches:field(&v,"maxPatches")?};
    let tolerances=field::<[f64;3]>(&v,"tolerances")?;let max_cells=field(&v,"maxCells")?;
    let result=surface_blend::g2_qualified(&field::<Surface>(&v,"a")?,&field::<Surface>(&v,"b")?,options,tolerances,max_cells)?;
    let receipt=|r:&Report|json!({"positionBounds":r.position_bounds,"regularityProven":r.regularity_proven,"cells":r.cells,
        "orientation":r.orientation.map(|o|match o {nurbs_core::surface_g2::Orientation::Same=>"same",nurbs_core::surface_g2::Orientation::Opposite=>"opposite"}),
        "branches":r.branches.iter().map(|b|json!({"normalLower":b.normal_lower,"normalUpper":b.normal_upper,"curvatureLower":b.tensor_lower,"curvatureUpper":b.tensor_upper})).collect::<Vec<_>>(),
        "decision":match r.decision{nurbs_core::surface_join::Decision::WithinTolerance=>"within-tolerance",nurbs_core::surface_join::Decision::CorrespondenceExceedsTolerance=>"exceeds-tolerance",nurbs_core::surface_join::Decision::Unresolved=>"unresolved"}});
    Ok(json!({"patches":result.construction.patches,"report":{"method":"native-rational-surface-G2-bridge/1",
        "complete":result.complete,"withinTolerance":result.within_tolerance,"cells":result.cells,"maxCells":max_cells,
        "tolerances":tolerances,"exactIdentityClaim":false,"solidGeometryCertified":false,
        "supportSeams":result.support_seams.iter().map(|(patch,source,range,r)|json!({"patch":patch,"source":source,"sourceRange":range,"inspection":receipt(r)})).collect::<Vec<_>>(),
        "internalSeams":result.internal_seams.iter().map(|(a,b,r)|json!({"patches":[a,b],"inspection":receipt(r)})).collect::<Vec<_>>()}}))
}

/// Sew retained support, transition and end patches, then independently admit
/// the closed body. Caller proposals cannot supply a geometric certificate.
pub fn create_closed_patch_network(v:Value)->Result<Value> {
    use nurbs_core::surface::Surface;
    let surfaces=field::<Vec<Surface>>(&v,"surfaces")?;let reversed=field::<Vec<bool>>(&v,"reversed")?;
    let sources=if v["sources"].is_null(){Vec::<Model>::new()}else{field::<Vec<Model>>(&v,"sources")?};
    if sources.len()>8 {return Err(super::input("Closed patch network admits at most eight lineage sources"));}
    let source_refs=sources.iter().collect::<Vec<_>>();
    let max_work=if v["maxCorrespondenceWork"].is_null(){1000000}else{field::<u64>(&v,"maxCorrespondenceWork")?};
    let assembly=brep_core::rounded_patch_network::assemble(&surfaces,&reversed,field(&v,"tolerance")?,max_work,&source_refs)?;
    let require_g2=if v["requireFullG2"].is_null(){true}else{field::<bool>(&v,"requireFullG2")?};
    let smooth_work=if v["maxSmoothnessWork"].is_null(){2000000}else{field::<u64>(&v,"maxSmoothnessWork")?};
    let jets=brep_core::sweep_smoothness::inspect_profile(&assembly.model,&[],smooth_work)?;
    let coverage=jets.profile.edge_ids.iter().chain(&jets.station.edge_ids).copied().collect::<std::collections::BTreeSet<_>>();
    let g2=jets.profile.g2_certified&&jets.station.g2_certified&&jets.profile.extraction_complete&&jets.station.extraction_complete&&coverage.len()==assembly.model.edges.len();
    if require_g2&&!g2 {return Err(super::input("BREP_G2_UNPROVEN: saved closed patch network seams are not certified"));}
    let proposals=if v["projectionHints"].is_null(){Vec::new()}else{field::<Vec<Option<[[i8;3];2]>>>(&v,"projectionHints")?};
    let volume=assembly.qualify(&proposals)?;
    if !volume.proven {return Err(super::input("BREP_G2_GLOBAL_UNPROVEN: closed patch network embedding or material orientation is unproved"));}
    let pairs=&volume.boundary.intersections.pairs;
    Ok(json!({"model":assembly.model,"report":{"method":"native-closed-rational-patch-network/1",
        "solidGeometryCertified":true,"fullBoundaryG2Certified":g2,"requireFullG2":require_g2,
        "faces":surfaces.len(),"edges":coverage.len(),"correspondenceWork":assembly.correspondence_work,"maxCorrespondenceWork":max_work,
        "directlyOwnedUses":assembly.directly_owned_uses,"provedUses":assembly.proved_uses,"exactWork":jets.total_exact_work,"maxSmoothnessWork":smooth_work,
        "allFacesInjective":volume.boundary.intersections.faces.all_faces_injective,"allPairsClassified":pairs.all_pairs_classified,
        "totalPairs":pairs.total_pairs,"coveredPairs":pairs.pairs.len()+pairs.grouped_pairs,"nextPair":pairs.next_pair}}))
}
