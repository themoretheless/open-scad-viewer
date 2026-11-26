use super::*;
use crate::analysis::curve_measure::{
    DirectionalMotionLimits, partition_planar_angular_speed_limit, partition_planar_angular_acceleration_limit,
    partition_planar_angular_jerk_limit, partition_planar_angular_motion_limits, planar_angular_motion_slowdown_bounds,
    planar_angular_motion_duration_bounds, check_planar_angular_motion_deadline,
};
pub(super) const OPS:&[&str]=&["angular_speed_limit","angular_acceleration_limit","angular_jerk_limit","angular_motion_limits",
    "angular_slowdown_bounds","angular_duration_bounds","angular_motion_deadline"];

pub(super) fn dispatch(v:&Value,op:&str)->Result<Value>{
    let d=super::rotating_tools::xy_curve(field(v,"curve")?)?;
    let domain=field(v,"domain")?;
    let tol=field(v,"parameter_tolerance")?;
    let budget=field(v,"max_cells")?;
    if matches!(op,"angular_speed_limit" | "angular_acceleration_limit" | "angular_jerk_limit"){
        let classify=match op {
            "angular_speed_limit"=>partition_planar_angular_speed_limit,
            "angular_acceleration_limit"=>partition_planar_angular_acceleration_limit,
            _=>partition_planar_angular_jerk_limit,
        };
        let r=classify(&d,domain,field(v,"limit")?,tol,budget)?;
        return Ok(json!({"workUsed":r.work_cells,"cells":r.cells.into_iter().map(|c|json!({"domain":c.parameter_interval,
            "span":c.span,"status":format!("{:?}",c.status)})).collect::<Vec<_>>()}));
    }
    let [speed,acceleration,jerk]=field(v,"limits")?;
    let limits=DirectionalMotionLimits{speed,acceleration,jerk};
    Ok(match op {
        "angular_motion_limits"=>{
            let r=partition_planar_angular_motion_limits(&d,domain,limits,tol,budget)?;
            json!({"workUsed":r.work_cells,"cells":r.cells.into_iter().map(|c|json!({"domain":c.parameter_interval,
                "span":c.span,"statuses":c.statuses.map(|s|format!("{s:?}"))})).collect::<Vec<_>>()})
        }
        "angular_slowdown_bounds"=>json!({"slowdown":planar_angular_motion_slowdown_bounds(&d,domain,limits,tol,budget)?}),
        "angular_duration_bounds"=>{
            let r=planar_angular_motion_duration_bounds(&d,domain,limits,tol,budget)?;
            json!({"duration":r.duration_bounds,"slowdown":r.slowdown_bounds})
        }
        "angular_motion_deadline"=>{
            let r=check_planar_angular_motion_deadline(&d,domain,limits,field(v,"deadline")?,tol,budget)?;
            json!({"status":format!("{:?}",r.status),"statuses":r.limit_statuses.map(|s|format!("{s:?}"))})
        }
        _=>return Err(input("Unknown angular limit operation")),
    })
}
