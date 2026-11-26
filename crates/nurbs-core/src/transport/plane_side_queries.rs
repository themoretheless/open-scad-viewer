use super::*;
use crate::editing::exact_surface_derivative as plane;
fn cell(c: plane::SurfacePlaneCell) -> Value {
    json!({"domain":c.domain,"side":format!("{:?}",c.side),"residual":c.residual.map(|q|q.to_string()),"corners":c.corners.map(|q|q.to_string())})
}
pub(super) fn inspect(v: &Value, op: &str) -> Result<Value> {
    let surface = field(v, "surface")?;
    let normal = field(v, "plane_normal")?;
    let offset = field(v, "plane_offset_mm")?;
    let work = field(v, "max_work")?;
    let (items, used) = match op {
        "audit_plane_side" => {
            let r = plane::audit_surface_plane_side(&surface, normal, offset, work)?;
            (
                r.cells
                    .map(|cells| cells.into_iter().map(cell).collect::<Vec<_>>()),
                r.work_used,
            )
        }
        "refine_plane_side" => {
            let r = plane::refine_surface_plane_side(
                &surface,
                normal,
                offset,
                field(v, "max_depth")?,
                work,
            )?;
            (
                r.regions.map(|regions| {
                    regions
                        .into_iter()
                        .map(|r| json!({"originalDomain":r.original_domain,"cell":cell(r.cell)}))
                        .collect()
                }),
                r.work_used,
            )
        }
        _ => {
            let r = plane::audit_surface_plane_graphs(&surface, normal, offset, work)?;
            (r.cells.map(|cells|cells.into_iter().map(|c|{use plane::SurfacePlaneGraphStatus as Status;
 let status=match c.status{Status::Graphs(g)=>json!({"kind":"Graphs","axes":(*g).map(|g|g.map(|v|json!({"solveAxis":format!("{:?}",v.solve_axis),"position":format!("{:?}",v.position),"derivative":v.derivative.map(|q|q.to_string())})))}),other=>json!({"kind":format!("{other:?}")})}; json!({"domain":c.domain,"status":status})}).collect()),r.work_used)
        }
    };
    Ok(match items {
        Ok(cells) => json!({"status":"complete","cells":cells,"workUsed":used}),
        Err(reason) => json!({"status":"refused","reason":format!("{reason:?}"),"workUsed":used}),
    })
}
