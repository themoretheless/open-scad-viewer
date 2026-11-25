use super::*;
use crate::shared_tessellation::{self, Seam, Side};

fn boundary(index: usize) -> Result<Side> {
    match index {
        0 => Ok(Side::U0), 1 => Ok(Side::U1), 2 => Ok(Side::V0), 3 => Ok(Side::V1),
        _ => Err(input("Patch side must be 0 (U0), 1 (U1), 2 (V0) or 3 (V1)")),
    }
}
pub(super) fn build(v: &Value) -> Result<Value> {
    let surfaces:Vec<crate::surface::Surface>=field(v,"surfaces")?;
    let addresses:Vec<[usize;5]>=field(v,"seams")?;
    let seams=addresses.into_iter().map(|[a,side_a,b,side_b,reverse]|{
        check(reverse<=1,"Seam reversal must be 0 or 1")?;
        Ok(Seam{patches:[a,b],sides:[boundary(side_a)?,boundary(side_b)?],reversed:reverse==1})
    }).collect::<Result<Vec<_>>>()?;
    let r=shared_tessellation::tessellate(&surfaces,&seams,field(v,"segments")?,field(v,"tolerance_mm")?,
        field(v,"max_cells")?,field(v,"max_agreement_cells")?)?;
    check(r.within_tolerance,"Shared mesh exceeds tolerance or seam agreement is unresolved")?;
    let mesh=r.mesh.ok_or_else(||input("Shared mesh has unresolved seam agreement"))?;
    let vertices=mesh.points.len();
    let positions:Vec<_>=mesh.points.into_iter().flatten().collect();
    let indices:Vec<_>=mesh.triangles.iter().flat_map(|t|t.vertices).collect();
    let faces:Vec<_>=mesh.triangles.iter().map(|t|t.patch).collect();
    Ok(json!({"positions":positions,"indices":indices,"faceIds":faces,"vertices":vertices,"triangles":mesh.triangles.len(),
        "agreementChecks":r.agreements.len(),"agreementCells":r.agreement_cells,"cells":r.cells,"errorUpperMm":r.error_upper}))
}
