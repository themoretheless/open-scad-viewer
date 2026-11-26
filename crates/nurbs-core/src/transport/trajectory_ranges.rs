use super::*;
use crate::analysis::curve_measure::{
    DirectionalMotionLimits, PositionAxis, TrajectoryAxis, partition_directional_position_range,
    partition_multi_axis_position_ranges, partition_trajectory_limits,
};

pub(super) const OPS: &[&str] = &["partition_axis_position","partition_axis_positions","partition_trajectory_bounds"];

pub(super) fn dispatch(v: &Value, op: &str) -> Result<Value> {
    let curve = field(v,"curve")?;
    let domain = field(v,"domain")?;
    let tolerance = field(v,"parameter_tolerance")?;
    let budget = field(v,"max_cells")?;
    if op=="partition_axis_position" {
        let axis: [f64;3] = field(v,"axis_vector")?;
        let r=partition_directional_position_range(&curve,domain,&axis,field(v,"allowed")?,tolerance,budget)?;
        let cells=r.cells.into_iter().map(|c|json!({"domain":c.parameter_interval,"span":c.span,
            "position":c.position_bounds,"status":format!("{:?}",c.status)})).collect::<Vec<_>>();
        return Ok(json!({"cells":cells,"workUsed":r.work_cells}));
    }
    let directions:Vec<[f64;3]>=field(v,"axis_vectors")?;
    let allowed:Vec<[f64;2]>=field(v,"allowed_ranges")?;
    check(!directions.is_empty() && directions.len()==allowed.len(),"Expected one allowed range per axis")?;
    let positions=directions.iter().zip(allowed).map(|(d,allowed)|PositionAxis{direction:d,allowed}).collect::<Vec<_>>();
    if op=="partition_axis_positions" {
        let r=partition_multi_axis_position_ranges(&curve,domain,&positions,tolerance,budget)?;
        let cells=r.cells.into_iter().map(|c|json!({"domain":c.parameter_interval,"span":c.span,"axes":
            c.axes.into_iter().map(|a|json!({"position":a.bounds,"status":format!("{:?}",a.status)})).collect::<Vec<_>>() })).collect::<Vec<_>>();
        return Ok(json!({"cells":cells,"workUsed":r.work_axis_cells}));
    }
    let limits:Vec<[f64;3]>=field(v,"motion_limits")?;
    check(limits.len()==positions.len(),"Expected one set of motion limits per axis")?;
    let axes=positions.into_iter().zip(limits).map(|(position,[speed,acceleration,jerk])|
        TrajectoryAxis{position,motion:DirectionalMotionLimits{speed,acceleration,jerk}}).collect::<Vec<_>>();
    let r=partition_trajectory_limits(&curve,domain,&axes,tolerance,budget)?;
    let cells=r.cells.into_iter().map(|c|json!({"domain":c.parameter_interval,"span":c.span,"axes":
        c.axes.into_iter().map(|a|json!({"position":a.position.bounds,"status":format!("{:?}",a.position.status),
        "motion":a.motion.map(|s|format!("{s:?}"))})).collect::<Vec<_>>() })).collect::<Vec<_>>();
    Ok(json!({"cells":cells,"workUsed":r.work_axis_cells}))
}
