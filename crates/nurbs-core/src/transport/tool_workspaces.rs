use super::*;
use crate::analysis::curve_measure::{
    measure_minimum_profiled_radius_workspace_clearance, measure_minimum_profiled_rounded_hull_workspace_clearance,
    partition_profiled_radius_convex_workspace, partition_profiled_rounded_hull_workspace, WorkspacePartition,
    WorkspaceCapsule, WorkspaceHalfSpace, partition_curve_ball_convex_workspace,
    partition_curve_ball_obstacle_clearance, partition_curve_capsule_convex_workspace,
    partition_curve_convex_workspace, partition_curve_obstacle_clearance,
    partition_curve_rounded_hull_obstacle_clearance, partition_curve_rounded_hull_workspace,
};

pub(super) const OPS:&[&str]=&["point_workspace","point_obstacles","ball_obstacles","profiled_ball_workspace","minimum_profiled_ball_workspace","profiled_hull_workspace","minimum_profiled_hull_workspace","ball_workspace",
    "capsule_workspace","hull_workspace","hull_obstacles"];

fn planes(data:&[[f64;4]]) -> Vec<WorkspaceHalfSpace<'_>> {
    data.iter().map(|p|WorkspaceHalfSpace{normal:&p[..3],offset:p[3]}).collect()
}

pub(super) fn dispatch(v:&Value,op:&str)->Result<Value>{
    let curve=field(v,"curve")?;
    if matches!(op, "profiled_ball_workspace" | "minimum_profiled_ball_workspace" | "profiled_hull_workspace" | "minimum_profiled_hull_workspace") {
        let data:Vec<[f64;4]>=field(v,"planes")?;
        let walls=planes(&data);
        let rows:Vec<[f64;2]>=field(v,"radius_profile")?;
            let profile=rows.into_iter().map(|[u,r]|(u,r)).collect::<Vec<_>>();
        let budget=field(v,"max_cells")?;
        let hull=matches!(op,"profiled_hull_workspace" | "minimum_profiled_hull_workspace");
        let offsets:Vec<[f64;3]>=if hull {field(v,"vertex_offsets")?} else {Vec::new()};
        let vertices=offsets.iter().map(|p|p.as_slice()).collect::<Vec<_>>();
        if op.starts_with("minimum_") {
            let tol=field(v,"gap_tolerance_mm")?;
            let r=if hull {measure_minimum_profiled_rounded_hull_workspace_clearance(&curve,&profile,&walls,&vertices,tol,budget)?}
                else {measure_minimum_profiled_radius_workspace_clearance(&curve,&profile,&walls,tol,budget)?};
            return Ok(json!({"gap":r.gap_bounds,"witnessParameter":r.witness_parameter,"witnessPlane":r.witness_plane,
                "workUsed":r.work_plane_cells,"stopReason":format!("{:?}",r.stop_reason)}));
        }
        let tol=field(v,"parameter_tolerance")?;
        let r=if hull {partition_profiled_rounded_hull_workspace(&curve,&profile,&walls,&vertices,tol,budget)?}
            else {partition_profiled_radius_convex_workspace(&curve,&profile,&walls,tol,budget)?};
        return Ok(workspace_report(r));
    }
    let domain=field(v,"domain")?;
    let tolerance=field(v,"parameter_tolerance")?;
    let budget=field(v,"max_cells")?;
    let radius=if matches!(op,"point_workspace"|"point_obstacles"){0.}else{field(v,"radius_mm")?};
    let offsets:Vec<[f64;3]>=if matches!(op,"capsule_workspace"|"hull_workspace"|"hull_obstacles"){field(v,"vertex_offsets")?}else{Vec::new()};
    let vertices=offsets.iter().map(|p|p.as_slice()).collect::<Vec<_>>();
    if matches!(op,"point_obstacles"|"ball_obstacles"|"hull_obstacles"){
        let data:Vec<Vec<[f64;4]>>=field(v,"obstacles")?;
        let sets=data.iter().map(|p|planes(p)).collect::<Vec<_>>();
        let obstacles=sets.iter().map(Vec::as_slice).collect::<Vec<_>>();
        let r=match op{
            "point_obstacles"=>partition_curve_obstacle_clearance(&curve,domain,&obstacles,tolerance,budget)?,
            "ball_obstacles"=>partition_curve_ball_obstacle_clearance(&curve,domain,&obstacles,radius,tolerance,budget)?,
            _=>partition_curve_rounded_hull_obstacle_clearance(&curve,domain,&obstacles,&vertices,radius,tolerance,budget)?,
        };
        let cells=r.cells.into_iter().map(|c|json!({"domain":c.parameter_interval,"span":c.span,"status":format!("{:?}",c.status),
            "obstacles":c.obstacles.into_iter().map(|s|format!("{s:?}")).collect::<Vec<_>>()})).collect::<Vec<_>>();
        return Ok(json!({"cells":cells,"workUsed":r.work_plane_cells}));
    }
    let data:Vec<[f64;4]>=field(v,"planes")?;
    let walls=planes(&data);
    let r=match op{
        "point_workspace"=>partition_curve_convex_workspace(&curve,domain,&walls,tolerance,budget)?,
        "ball_workspace"=>partition_curve_ball_convex_workspace(&curve,domain,&walls,radius,tolerance,budget)?,
        "capsule_workspace"=>{
            let endpoint_offsets=vertices.as_slice().try_into().map_err(|_|input("Capsule requires two endpoint offsets"))?;
            partition_curve_capsule_convex_workspace(&curve,domain,&walls,WorkspaceCapsule{endpoint_offsets,radius},tolerance,budget)?
        }
        _=>partition_curve_rounded_hull_workspace(&curve,domain,&walls,&vertices,radius,tolerance,budget)?,
    };
    Ok(workspace_report(r))
}

fn workspace_report(r:WorkspacePartition)->Value{
    let cells=r.cells.into_iter().map(|c|json!({"domain":c.parameter_interval,"span":c.span,"status":format!("{:?}",c.status),
        "planeResiduals":c.plane_residuals})).collect::<Vec<_>>();
    json!({"cells":cells,"workUsed":r.work_plane_cells})
}
