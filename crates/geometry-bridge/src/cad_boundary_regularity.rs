//! Independent whole-boundary regularity, with its own finite cell ledger.
use crate::{Result,Value,field};
use value_codec::json;
pub(super) fn inspect(model:&brep_core::Model,request:&Value)->Result<Value> {
    let max_cells=if request.get("maxRegularityCells").is_some(){field(request,"maxRegularityCells")?}else{10000};
    let report=brep_core::boundary_embedding::regularity::inspect(model,max_cells)?;
    Ok(json!({"allFacesRegular":report.all_faces_regular,
        "allFacesRegularModuloPoles":report.all_faces_regular_modulo_poles,
        "cells":report.cells,"maxCells":max_cells,"unresolvedFaces":report.unresolved_faces,
        "faces":report.faces.iter().map(|r|r.as_ref().map(|r|json!({"spanwiseRegular":r.spanwise_regular,
            "interiorBasisC1":r.interior_basis_c1,"cells":r.cells,"provenCells":r.proven_cells,
            "unresolved":r.unresolved.iter().map(|u|u.domain).collect::<Vec<_>>()}))).collect::<Vec<_>>(),
        "poleQuotients":report.pole_quotients.iter().enumerate().filter_map(|(face,r)|r.as_ref().map(|r|json!({
            "face":face,"edge":r.edge,"vertex":r.vertex,"collapsedEnd":r.collapsed_end,
            "proven":r.proof.proven,"cells":r.proof.cells,"reason":r.proof.reason}))).collect::<Vec<_>>() }))
}
pub(super) fn attach(output:&mut Value,regularity:Value) {
    output["regularBoundarySolidCertified"]=json!(output["solidGeometryCertified"]==json!(true)
        &&regularity["allFacesRegularModuloPoles"]==json!(true));
    output["boundaryRegularity"]=regularity;
}
