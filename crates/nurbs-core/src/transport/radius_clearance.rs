use super::*;
use super::trajectory_clearance::{with_obstacles, minimum, partition_report};
use crate::analysis::curve_measure::{
    SphericalObstacle, measure_minimum_variable_radius_spherical_clearance, partition_variable_radius_spherical_clearance,
    measure_minimum_variable_radius_toolpath_clearance, partition_variable_radius_toolpath_clearance,
    measure_minimum_profiled_radius_toolpath_clearance, partition_profiled_radius_toolpath_clearance,
};
pub(super) const OPS:&[&str]=&["variable_sphere_clearance","minimum_variable_sphere_gap","variable_scene_clearance",
    "minimum_variable_scene_gap","profiled_scene_clearance","minimum_profiled_scene_gap"];
pub(super) fn dispatch(v:&Value,op:&str)->Result<Value>{
    let curve=field(v,"curve")?;
    let minimize=op.starts_with("minimum_");
    let tol=field(v,if minimize {"gap_tolerance_mm"} else {"parameter_tolerance"})?;
    let budget=field(v,"max_cells")?;
    if matches!(op,"variable_sphere_clearance"|"minimum_variable_sphere_gap") {
        let rows:Vec<[f64;4]>=field(v,"spheres")?;
        let spheres=rows.iter().map(|p|SphericalObstacle{center:&p[..3],radius:p[3]}).collect::<Vec<_>>();
        let domain=field(v,"domain")?;
        let radii=field(v,"tool_radii")?;
        return if minimize {
            measure_minimum_variable_radius_spherical_clearance(&curve,domain,&spheres,radii,tol,budget).map(minimum)
        } else {
            partition_variable_radius_spherical_clearance(&curve,domain,&spheres,radii,tol,budget).map(partition_report)
        };
    }
    with_obstacles(v,|obstacles| {
        if matches!(op,"profiled_scene_clearance"|"minimum_profiled_scene_gap") {
            let rows:Vec<[f64;2]>=field(v,"radius_profile")?;
            let profile=rows.into_iter().map(|[u,r]|(u,r)).collect::<Vec<_>>();
            return if minimize {
                measure_minimum_profiled_radius_toolpath_clearance(&curve,&profile,obstacles,tol,budget).map(minimum)
            } else {
                partition_profiled_radius_toolpath_clearance(&curve,&profile,obstacles,tol,budget).map(partition_report)
            };
        }
        let domain=field(v,"domain")?;
        let radii=field(v,"tool_radii")?;
        if minimize {
            measure_minimum_variable_radius_toolpath_clearance(&curve,domain,obstacles,radii,tol,budget).map(minimum)
        } else {
            partition_variable_radius_toolpath_clearance(&curve,domain,obstacles,radii,tol,budget).map(partition_report)
        }
    })
}
