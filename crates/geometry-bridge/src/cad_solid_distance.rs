use super::{Result,Value,field};
use value_codec::json;
fn validity_limits(v:&Value)->Result<brep_core::volume_validity::Limits>{
    Ok(brep_core::volume_validity::Limits{
        boundary:brep_core::boundary_embedding::Limits{
            exact_work:field(v,"exactWork")?,trim_pairs:field(v,"trimPairs")?,trim_cells:field(v,"trimCells")?,
            trim_domain_cells:field(v,"trimDomainCells")?,spans:field(v,"spans")?,
            contacts:brep_core::face_contacts::Limits{pairs:field(v,"facePairs")?,cells:field(v,"faceCells")?,
                domain_cells:field(v,"faceDomainCells")?,cells_per_pair:field(v,"faceCellsPerPair")?,domain_cells_per_pair:field(v,"faceDomainCellsPerPair")?}},
        nesting_pairs:field(v,"nestingPairs")?,nesting_cells:field(v,"nestingCells")?,nesting_domain_cells:field(v,"nestingDomainCells")?,
        orientation_cells:field(v,"orientationCells")?,orientation_domain_cells:field(v,"orientationDomainCells")?,orientation_spans:field(v,"orientationSpans")?})
}
pub fn measure(v:Value)->Result<Value>{
    let a:brep_core::Model=field(&v,"a")?;let b:brep_core::Model=field(&v,"b")?;
    let tolerance_mm:f64=field(&v,"toleranceMm")?;let tolerance_uv:f64=field(&v,"toleranceUv")?;
    let config:Value=field(&v,"validityLimits")?;
    let limits=brep_core::solid_distance::Limits{validity:validity_limits(&config)?,pairs:field(&v,"maxPairs")?,
        contact_pairs:field(&v,"maxContactPairs")?,cells:field(&v,"maxCells")?,domain_cells:field(&v,"maxDomainCells")?};
    let r=brep_core::solid_distance::distance(&a,&b,tolerance_mm,tolerance_uv,limits)?;
    let validity:Vec<_>=r.validity.iter().map(|v|json!({"proven":v.proven,
        "boundaryProven":v.boundary.proven,"exactAgreement":v.boundary.agreement.all_equal,
        "exactJoins":v.boundary.agreement.all_joins_exact,"trimValid":v.boundary.trim.all_valid,
        "selfIntersectionAbsent":v.boundary.intersections.absence_proven,
        "nestingRolesConsistent":v.nesting.as_ref().and_then(|n|n.roles_consistent),
        "orientations":v.orientations.iter().map(|o|json!({"shell":o.shell,"expectedOutward":o.expected_outward,"outward":o.outward})).collect::<Vec<_>>()
    })).collect();
    let contact=r.contact.as_ref().map(|(faces,w)|json!({"faces":faces,"firstUv":w.first_uv,"secondUv":w.second_uv,
        "pointIntervalMm":w.point,"contractionUpper":w.contraction_upper}));
    let separation=r.separation_witness(&a,&b).map(|w|json!({"faces":w.faces,
        "parameters":w.geometry.parameters,"points":w.geometry.points,"pointEnclosures":w.geometry.point_enclosures}));
    Ok(json!({"method":"certified-volume-distance","scope":"closed-material-sets",
        "validity":validity,"distanceIntervalMm":r.distance_interval_mm,"materialOverlap":r.material_overlap,
        "converged":r.converged,"reason":r.reason,"contact":contact,"separationWitness":separation,
        "totalShellPairs":r.total_pairs,"visitedShellPairs":r.pairs.len(),"contactPairsVisited":r.contact_pairs_visited,
        "cells":r.cells,"domainCells":r.domain_cells,"toleranceMm":tolerance_mm,"toleranceUv":tolerance_uv,
        "limits":{"maxPairs":limits.pairs,"maxContactPairs":limits.contact_pairs,"maxCells":limits.cells,"maxDomainCells":limits.domain_cells,"validity":config}}))
}
#[cfg(test)]
mod tests{
    use super::*;
    fn request(a:brep_core::Model,b:brep_core::Model)->Value{
        json!({"op":"cad_solid_distance","a":a,"b":b,"toleranceMm":1e-5,"toleranceUv":1e-8,
            "maxPairs":100,"maxContactPairs":1000,"maxCells":100000,"maxDomainCells":1000000,
            "validityLimits":{"exactWork":1000000,"trimPairs":10000,"trimCells":100000,"trimDomainCells":1000000,"spans":1000,
                "facePairs":10000,"faceCells":100000,"faceDomainCells":1000000,"faceCellsPerPair":1000,"faceDomainCellsPerPair":10000,
                "nestingPairs":100,"nestingCells":100000,"nestingDomainCells":1000000,"orientationCells":100000,"orientationDomainCells":1000000,"orientationSpans":100}})
    }
    #[test]
    fn export_native_contract_fixtures_when_requested(){
        let Ok(path)=std::env::var("CAD_SOLID_DISTANCE_FIXTURE") else{return};
        let a=brep_core::cuboid([0.;3],[2.;3]).unwrap();
        let mut reversed=a.clone();for f in &mut reversed.shells[0].faces{f.reversed=!f.reversed;}
        let cavity=brep_core::operations::boolean(&brep_core::cuboid([0.;3],[10.;3]).unwrap(),&brep_core::cuboid([2.;3],[8.;3]).unwrap(),"difference").unwrap();
        let cylinder=brep_core::analytic::cylinder(2.,4.).unwrap();let mut raised=cylinder.clone();
        for v in &mut raised.vertices{v.point[2]+=7.;}
        for e in &mut raised.edges{for p in &mut e.curve.control_points{p[2]+=7.;}}
        for f in &mut raised.faces{for row in &mut f.surface.control_points{for p in row{p[2]+=7.;}}}
        let meshes=[&cylinder,&raised].map(|m|crate::dispatch(json!({"op":"brep_nurbs_tessellate","model":m,"segments":16})).unwrap());
        let inputs=[request(a.clone(),brep_core::cuboid([0.5;3],[1.5;3]).unwrap()),
            request(a.clone(),reversed),request(a.clone(),brep_core::cuboid([1.,0.5,0.5],[3.,1.5,1.5]).unwrap()),
            request(a,brep_core::cuboid([5.,0.5,0.5],[6.,1.5,1.5]).unwrap()),
            request(cavity,brep_core::cuboid([3.;3],[4.;3]).unwrap()),request(cylinder,raised)];
        let cases=inputs.into_iter().enumerate().map(|(i,input)|{
            let result=crate::dispatch(input.clone()).unwrap();let mut value=json!({"request":input,"result":result});
            if i==5{value["displayMeshes"]=json!(meshes);value["expectedDistanceMm"]=json!(3.);}
            value
        }).collect::<Vec<_>>();
        std::fs::write(path,value_codec::to_string(&json!({"cases":cases})).unwrap()).unwrap();
    }
    #[test]
    fn dispatch_separates_material_containment_from_unproven_geometry(){
        let a=brep_core::cuboid([0.;3],[10.;3]).unwrap();let mut b=brep_core::cuboid([2.;3],[8.;3]).unwrap();
        let r=crate::dispatch(request(a.clone(),b.clone())).unwrap();
        assert_eq!(field::<String>(&r,"scope").unwrap(),"closed-material-sets");
        assert_eq!(field::<Option<[f64;2]>>(&r,"distanceIntervalMm").unwrap(),Some([0.,0.]));
        assert!(field::<bool>(&r,"converged").unwrap());
        for f in &mut b.shells[0].faces{f.reversed=!f.reversed;}
        let r=crate::dispatch(request(a,b)).unwrap();
        assert_eq!(field::<Option<[f64;2]>>(&r,"distanceIntervalMm").unwrap(),None);
        assert!(!field::<bool>(&r,"converged").unwrap());
        assert_eq!(field::<String>(&r,"reason").unwrap(),"volume-validity-unproven");
    }
}
