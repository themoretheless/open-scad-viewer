use super::*;
use crate::analysis::curve_measure::*;
use crate::curve::Curve;

pub(super) const OPS:&[&str]=&["rotating_hull_workspace","minimum_rotating_hull_gap","rotating_hull_obstacles","rounded_rotating_obstacles",
    "rotating_face_obstacles","rotating_corner_obstacles","rotating_asymmetric_obstacles","rotating_skew_obstacles","rotating_chord_obstacles","rotating_clipped_chord_obstacles","rotating_exact_chord_obstacles","profiled_rotating_obstacles","profiled_rotating_workspace","minimum_profiled_rotating_gap"];

fn xy_curve(mut curve:Curve)->Result<Curve>{
    curve.validate()?;
    for point in &mut curve.control_points {
        check(point.len()==2 || (point.len()==3 && point[2]==0.),"Rotating tools require curves in the XY plane")?;
        point.truncate(2);
    }
    Ok(curve)
}

fn planes(rows:&[[f64;3]])->Vec<WorkspaceHalfSpace<'_>>{
    rows.iter().map(|p|WorkspaceHalfSpace{normal:&p[..2],offset:p[2]}).collect()
}

pub(super) fn dispatch(v:&Value,op:&str)->Result<Value>{
    let [position,direction]:[Curve;2]=field(v,"curves")?;
    let p=xy_curve(position)?;
    let d=xy_curve(direction)?;
    let rows:Vec<[f64;2]>=field(v,"vertex_offsets")?;
    let vertices=rows.iter().map(|p|p.as_slice()).collect::<Vec<_>>();
    let budget=field(v,"max_cells")?;
    let tol=field(v,if op.starts_with("minimum_") {"gap_tolerance_mm"} else {"parameter_tolerance"})?;
    let profile=||->Result<Vec<(f64,f64)>>{
        let data:Vec<[f64;2]>=field(v,"radius_profile")?;
        Ok(data.into_iter().map(|[u,r]|(u,r)).collect())
    };
    if op.ends_with("_obstacles") {
        let data:Vec<Vec<[f64;3]>>=field(v,"obstacles")?;
        let sets=data.iter().map(|rows|planes(rows)).collect::<Vec<_>>();
        let obs=sets.iter().map(Vec::as_slice).collect::<Vec<_>>();
        let r=match op {
            "rotating_hull_obstacles"=>partition_planar_rotating_hull_obstacle_clearance(&p,&d,field(v,"domain")?,&obs,&vertices,tol,budget)?,
            "rounded_rotating_obstacles"=>partition_planar_rotating_rounded_hull_obstacle_clearance(&p,&d,field(v,"domain")?,&obs,&vertices,field(v,"radius_mm")?,tol,budget)?,
            _=>{
                let certify=match op {
                    "rotating_face_obstacles"=>partition_planar_rotating_profiled_hull_obstacle_contact,
                    "rotating_corner_obstacles"=>partition_planar_rotating_profiled_hull_obstacle_corner_contact,
                    "rotating_asymmetric_obstacles"=>partition_planar_rotating_profiled_hull_obstacle_asymmetric_contact,
                    "rotating_skew_obstacles"=>partition_planar_rotating_profiled_hull_obstacle_skew_contact,
                    "rotating_chord_obstacles"=>partition_planar_rotating_profiled_hull_obstacle_chord_contact,
                    "rotating_clipped_chord_obstacles"=>partition_planar_rotating_profiled_hull_obstacle_clipped_chord_contact,
                    "rotating_exact_chord_obstacles"=>partition_planar_rotating_profiled_hull_obstacle_exact_chord_contact,
                    _=>partition_planar_rotating_profiled_hull_obstacle_clearance,
                };
                certify(&p,&d,&profile()?,&obs,&vertices,tol,budget)?
            }
        };
        return Ok(json!({"workUsed":r.work_plane_cells,"cells":r.cells.into_iter().map(|c|json!({
            "domain":c.parameter_interval,"span":c.span,"status":format!("{:?}",c.status),"obstacles":c.obstacles.into_iter().map(|s|format!("{s:?}")).collect::<Vec<_>>()
        })).collect::<Vec<_>>()}));
    }
    let data:Vec<[f64;3]>=field(v,"planes")?;
    let walls=planes(&data);
    if op.starts_with("minimum_"){
        let r=if op=="minimum_rotating_hull_gap" {measure_minimum_planar_rotating_hull_workspace_clearance(&p,&d,field(v,"domain")?,&walls,&vertices,tol,budget)?}
            else {measure_minimum_planar_rotating_profiled_hull_workspace_clearance(&p,&d,&profile()?,&walls,&vertices,tol,budget)?};
        return Ok(json!({"gap":r.gap_bounds,"witnessParameter":r.witness_parameter,"witnessPlane":r.witness_plane,"workUsed":r.work_plane_cells,"stopReason":format!("{:?}",r.stop_reason)}));
    }
    let r=if op=="rotating_hull_workspace"{partition_planar_rotating_hull_workspace(&p,&d,field(v,"domain")?,&walls,&vertices,tol,budget)?}
        else {partition_planar_rotating_profiled_hull_workspace(&p,&d,&profile()?,&walls,&vertices,tol,budget)?};
    Ok(super::tool_workspaces::workspace_report(r))
}
