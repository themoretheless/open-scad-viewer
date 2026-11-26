use super::*;
use crate::analysis::curve_measure::{
    ToolpathObstacle, measure_minimum_toolpath_clearance, partition_toolpath_clearance_with_margin,
    partition_toolpath_clearance, measure_toolpath_clearance_violation_length,
    measure_toolpath_violation_lengths_by_obstacle, measure_toolpath_clearance_violation_duration, TimedMotionSegment,
};
use crate::analysis::curve_measure::{
    measure_minimum_spherical_clearance, measure_minimum_box_clearance, measure_minimum_oriented_box_clearance,
    measure_minimum_capsule_clearance, SphericalMinimumClearance,
    BoxObstacle, CapsuleObstacle, FirstToolpathContact, OrientedBoxObstacle, SphericalObstacle,
    partition_curve_box_clearance, partition_curve_capsule_clearance,
    partition_curve_oriented_box_clearance, partition_curve_spherical_clearance,
};

pub(super) const OPS:&[&str]=&["angular_speed_limit","angular_acceleration_limit","angular_jerk_limit","angular_motion_limits","angular_slowdown_bounds","angular_duration_bounds","angular_motion_deadline","sphere_path_clearance","capsule_path_clearance","box_path_clearance",
    "oriented_box_path_clearance","first_sphere_contact","minimum_sphere_partition_gap","minimum_mixed_gap","mixed_path_clearance","violation_path_length","violation_obstacle_lengths","violation_path_duration","chain_violation_duration","last_chain_violation","chain_clearance_timeline","longest_chain_violation","minimum_synchronized_gap","synchronized_contact_intervals","minimum_tool_pair_gap","tool_pair_intervals","first_tool_pair_contact","last_tool_pair_contact","tool_contact_duration","longest_tool_contact","peak_tool_pair_count","simultaneous_tool_duration","simultaneous_tool_windows","peak_tool_contacts","tool_pairs_at_time","minimum_scheduled_gap","scheduled_contact_intervals","held_contact_intervals","minimum_held_gap","minimum_affine_gap","affine_contact_intervals","first_chain_violation","minimum_chain_gap","mixed_margin_clearance","variable_sphere_clearance","minimum_variable_sphere_gap","variable_scene_clearance","minimum_variable_scene_gap","profiled_scene_clearance","minimum_profiled_scene_gap","minimum_sphere_gap","minimum_box_gap","minimum_oriented_box_gap","minimum_capsule_gap"];

pub(super) fn minimum(r:SphericalMinimumClearance)->Value{
    json!({"gap":r.gap_bounds,"witnessParameter":r.witness_parameter,"witnessObstacle":r.witness_obstacle,
        "workUsed":r.work_obstacle_cells,"stopReason":format!("{:?}",r.stop_reason)})
}

pub(super) fn dispatch(v:&Value,op:&str)->Result<Value>{
    if super::angular_limits::OPS.contains(&op) { return super::angular_limits::dispatch(v,op); }
    if super::radius_clearance::OPS.contains(&op) { return super::radius_clearance::dispatch(v,op); }
    if super::tool_pair_clearance::OPS.contains(&op) { return super::tool_pair_clearance::dispatch(v,op); }
    if matches!(op, "minimum_synchronized_gap" | "synchronized_contact_intervals" | "minimum_scheduled_gap" | "scheduled_contact_intervals" | "held_contact_intervals" | "minimum_held_gap" | "minimum_affine_gap" | "affine_contact_intervals") {
        return super::synchronized_clearance::dispatch(v, op);
    }
    if matches!(op, "chain_violation_duration" | "last_chain_violation" | "chain_clearance_timeline" | "longest_chain_violation" | "first_chain_violation" | "minimum_chain_gap") {
        return with_obstacles(v, |obstacles| super::chain_clearance::dispatch(v, op, obstacles));
    }
    let curve=field(v,"curve")?;
    let domain=field(v,"domain")?;
    let radius=field(v,"radius_mm")?;
    let minimize=matches!(op,"minimum_sphere_gap"|"minimum_capsule_gap"|"minimum_box_gap"|"minimum_oriented_box_gap");
    let tol=field(v,if minimize || op=="minimum_mixed_gap" {"gap_tolerance_mm"} else {"parameter_tolerance"})?;
    let budget=field(v,"max_cells")?;
    let r=match op {
        "minimum_mixed_gap" | "mixed_path_clearance" | "mixed_margin_clearance" | "violation_path_length" | "violation_obstacle_lengths" | "violation_path_duration" => {
            return with_obstacles(v, |obstacles| {
            if op == "violation_path_duration" {
                let segment = TimedMotionSegment { curve: &curve, interval: domain, duration: field(v,"duration_s")? };
                let report = measure_toolpath_clearance_violation_duration(segment, &obstacles, radius, field(v,"margin_mm")?, tol, budget)?;
                return Ok(json!({"duration":report.bounds,"byObstacle":report.by_obstacle,"workUsed":report.work_obstacle_cells}));
            }
            if matches!(op,"violation_path_length" | "violation_obstacle_lengths") {
                let margin = field(v,"margin_mm")?;
                let length_tol = field(v,"length_tolerance_mm")?;
                let length_budget = field(v,"max_length_cells")?;
                if op == "violation_obstacle_lengths" {
                    let r = measure_toolpath_violation_lengths_by_obstacle(&curve, domain, &obstacles, radius, margin, tol, length_tol, budget, length_budget)?;
                    return Ok(json!({"length":r.union.bounds,"byObstacle":r.by_obstacle,"workUsed":r.union.work_obstacle_cells,"lengthCells":r.union.work_length_cells}));
                }
                let r = measure_toolpath_clearance_violation_length(&curve, domain, &obstacles, radius, margin, tol, length_tol, budget, length_budget)?;
                return Ok(json!({"length":r.bounds,"workUsed":r.work_obstacle_cells,"lengthCells":r.work_length_cells}));
            }
            if op == "minimum_mixed_gap" {
                return Ok(minimum(measure_minimum_toolpath_clearance(&curve, domain, &obstacles, radius, tol, budget)?));
            }
            let partition = if op == "mixed_margin_clearance" {
                partition_toolpath_clearance_with_margin(&curve, domain, &obstacles, radius, field(v, "margin_mm")?, tol, budget)?
            } else {
                partition_toolpath_clearance(&curve, domain, &obstacles, radius, tol, budget)?
            };
            Ok(partition_report(partition))
            })
        }
        "capsule_path_clearance" | "minimum_capsule_gap"=>{
            let data:Vec<[f64;7]>=field(v,"capsules")?;
            let obstacles=data.iter().map(|p|CapsuleObstacle{endpoints:[&p[..3],&p[3..6]],radius:p[6]}).collect::<Vec<_>>();
            if minimize { return Ok(minimum(measure_minimum_capsule_clearance(&curve,domain,&obstacles,radius,tol,budget)?)); }
            partition_curve_capsule_clearance(&curve,domain,&obstacles,radius,tol,budget)?
        }
        "box_path_clearance" | "minimum_box_gap"=>{
            let data:Vec<[[f64;3];2]>=field(v,"boxes")?;
            let obstacles=data.iter().map(|p|BoxObstacle{minimum:&p[0],maximum:&p[1]}).collect::<Vec<_>>();
            if minimize { return Ok(minimum(measure_minimum_box_clearance(&curve,domain,&obstacles,radius,tol,budget)?)); }
            partition_curve_box_clearance(&curve,domain,&obstacles,radius,tol,budget)?
        }
        "oriented_box_path_clearance" | "minimum_oriented_box_gap"=>{
            let data:Vec<[[f64;3];5]>=field(v,"oriented_boxes")?;
            let frames=data.iter().map(|p|[p[1].as_slice(),p[2].as_slice(),p[3].as_slice()]).collect::<Vec<_>>();
            let obstacles=data.iter().zip(&frames).map(|(p,axes)|OrientedBoxObstacle{center:&p[0],axes,half_extents:&p[4]}).collect::<Vec<_>>();
            if minimize { return Ok(minimum(measure_minimum_oriented_box_clearance(&curve,domain,&obstacles,radius,tol,budget)?)); }
            partition_curve_oriented_box_clearance(&curve,domain,&obstacles,radius,tol,budget)?
        }
        _=>{
            let data:Vec<[f64;4]>=field(v,"spheres")?;
            let obstacles=data.iter().map(|p|SphericalObstacle{center:&p[..3],radius:p[3]}).collect::<Vec<_>>();
            if minimize { return Ok(minimum(measure_minimum_spherical_clearance(&curve,domain,&obstacles,radius,tol,budget)?)); }
            partition_curve_spherical_clearance(&curve,domain,&obstacles,radius,tol,budget)?
        }
    };
    if op=="first_sphere_contact"{
        let contact=match r.first_contact(){
            FirstToolpathContact::Clear=>json!({"kind":"Clear"}),
            FirstToolpathContact::Possible{earliest_parameter}=>json!({"kind":"Possible","earliest":earliest_parameter}),
            FirstToolpathContact::Confirmed{parameter_bounds,obstacle_index}=>json!({"kind":"Confirmed","domain":parameter_bounds,"obstacle":obstacle_index}),
        };
        return Ok(json!({"contact":contact,"workUsed":r.work_obstacle_cells}));
    }
    if op=="minimum_sphere_partition_gap"{
        let minimum=r.minimum_clearance().map(|m|json!({"gap":m.gap_bounds,"domains":m.parameter_intervals,"witness":m.upper_witness}));
        return Ok(json!({"minimum":minimum,"workUsed":r.work_obstacle_cells}));
    }
    Ok(partition_report(r))
}

pub(super) fn partition_report(r:crate::analysis::curve_measure::SphericalClearancePartition)->Value{
    let cells=r.cells.into_iter().map(|c|json!({"domain":c.parameter_interval,"span":c.span,"gaps":c.gap_bounds,"status":format!("{:?}",c.status)})).collect::<Vec<_>>();
    json!({"cells":cells,"workUsed":r.work_obstacle_cells})
}

pub(super) fn with_obstacles(v: &Value, run: impl FnOnce(&[ToolpathObstacle]) -> Result<Value>) -> Result<Value> {
    let rows: Vec<Vec<f64>> = field(v, "obstacles")?;
    let frames: Vec<_> = rows.iter().map(|p| {
        (p.len() == 15).then(|| [&p[3..6], &p[6..9], &p[9..12]])
    }).collect();
    let obstacles = rows.iter().zip(&frames).map(|(p, axes)| {
        Ok(match p.len() {
            4 => ToolpathObstacle::Sphere(SphericalObstacle { center: &p[..3], radius: p[3] }),
            6 => ToolpathObstacle::Box(BoxObstacle { minimum: &p[..3], maximum: &p[3..] }),
            7 => ToolpathObstacle::Capsule(CapsuleObstacle { endpoints: [&p[..3], &p[3..6]], radius: p[6] }),
            15 => ToolpathObstacle::OrientedBox(OrientedBoxObstacle { center: &p[..3], axes: axes.as_ref().unwrap(), half_extents: &p[12..] }),
            _ => return Err(input("Mixed obstacles require 4, 6, 7 or 15 coordinates")),
        })
    }).collect::<Result<Vec<_>>>()?;
    run(&obstacles)
}
