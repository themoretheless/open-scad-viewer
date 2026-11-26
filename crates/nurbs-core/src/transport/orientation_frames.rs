use super::*;
use super::rotating_tools::xy_curve;
use crate::analysis::curve_measure::{
    DirectionalMotionLimits, PlanarAngularTravelReport, measure_planar_angular_travel, measure_relative_planar_angular_travel,
    partition_planar_orientation_sector, partition_relative_planar_orientation_sector, partition_relative_planar_angular_motion_limits,
};
pub(super) const OPS:&[&str]=&["orientation_sector","relative_orientation_sector","relative_angular_limits","angular_travel","relative_angular_travel"];

fn travel(r:PlanarAngularTravelReport)->Value{
    json!({"net":r.net_bounds,"total":r.total_bounds,"workUsed":r.work_cells})
}

pub(super) fn dispatch(v:&Value,op:&str)->Result<Value>{
    let domain=field(v,"domain")?;
    let tol=field(v,"parameter_tolerance")?;
    let budget=field(v,"max_cells")?;
    if matches!(op,"orientation_sector" | "angular_travel"){
        let d=xy_curve(field(v,"curve")?)?;
        if op=="angular_travel" {return measure_planar_angular_travel(&d,domain,tol,budget).map(travel);}
        let r=partition_planar_orientation_sector(&d,domain,field(v,"rays")?,tol,budget)?;
        return Ok(json!({"workUsed":r.work_cells,"cells":r.cells.into_iter().map(|c|json!({"domain":c.parameter_interval,
            "span":c.span,"status":format!("{:?}",c.status)})).collect::<Vec<_>>()}));
    }
    let [direction,reference]=field(v,"curves")?;
    let d=xy_curve(direction)?;
    let reference=xy_curve(reference)?;
    Ok(match op{
        "relative_orientation_sector"=>{
            let r=partition_relative_planar_orientation_sector(&d,&reference,domain,field(v,"rays")?,tol,budget)?;
            json!({"workUsed":r.work_cells,"cells":r.cells.into_iter().map(|c|json!({"domain":c.parameter_interval,
                "spans":c.spans,"status":format!("{:?}",c.status)})).collect::<Vec<_>>()})
        }
        "relative_angular_limits"=>{
            let [speed,acceleration,jerk]=field(v,"limits")?;
            let r=partition_relative_planar_angular_motion_limits(&d,&reference,domain,DirectionalMotionLimits{speed,acceleration,jerk},tol,budget)?;
            json!({"workUsed":r.work_cells,"cells":r.cells.into_iter().map(|c|json!({"domain":c.parameter_interval,
                "spans":c.spans,"statuses":c.statuses.map(|s|format!("{s:?}"))})).collect::<Vec<_>>()})
        }
        "relative_angular_travel"=>travel(measure_relative_planar_angular_travel(&d,&reference,domain,tol,budget)?),
        _=>return Err(input("Unknown orientation frame operation")),
    })
}
