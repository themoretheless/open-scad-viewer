use super::*;
use crate::analysis::curve_measure::{
    BoxObstacle, CapsuleObstacle, FirstToolpathContact, OrientedBoxObstacle, SphericalObstacle,
    partition_curve_box_clearance, partition_curve_capsule_clearance,
    partition_curve_oriented_box_clearance, partition_curve_spherical_clearance,
};

pub(super) const OPS:&[&str]=&["sphere_path_clearance","capsule_path_clearance","box_path_clearance",
    "oriented_box_path_clearance","first_sphere_contact","minimum_sphere_partition_gap"];

pub(super) fn dispatch(v:&Value,op:&str)->Result<Value>{
    let curve=field(v,"curve")?;
    let domain=field(v,"domain")?;
    let radius=field(v,"radius_mm")?;
    let tol=field(v,"parameter_tolerance")?;
    let budget=field(v,"max_cells")?;
    let r=match op {
        "capsule_path_clearance"=>{
            let data:Vec<[f64;7]>=field(v,"capsules")?;
            let obstacles=data.iter().map(|p|CapsuleObstacle{endpoints:[&p[..3],&p[3..6]],radius:p[6]}).collect::<Vec<_>>();
            partition_curve_capsule_clearance(&curve,domain,&obstacles,radius,tol,budget)?
        }
        "box_path_clearance"=>{
            let data:Vec<[[f64;3];2]>=field(v,"boxes")?;
            let obstacles=data.iter().map(|p|BoxObstacle{minimum:&p[0],maximum:&p[1]}).collect::<Vec<_>>();
            partition_curve_box_clearance(&curve,domain,&obstacles,radius,tol,budget)?
        }
        "oriented_box_path_clearance"=>{
            let data:Vec<[[f64;3];5]>=field(v,"oriented_boxes")?;
            let frames=data.iter().map(|p|[p[1].as_slice(),p[2].as_slice(),p[3].as_slice()]).collect::<Vec<_>>();
            let obstacles=data.iter().zip(&frames).map(|(p,axes)|OrientedBoxObstacle{center:&p[0],axes,half_extents:&p[4]}).collect::<Vec<_>>();
            partition_curve_oriented_box_clearance(&curve,domain,&obstacles,radius,tol,budget)?
        }
        _=>{
            let data:Vec<[f64;4]>=field(v,"spheres")?;
            let obstacles=data.iter().map(|p|SphericalObstacle{center:&p[..3],radius:p[3]}).collect::<Vec<_>>();
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
    let cells=r.cells.into_iter().map(|c|json!({"domain":c.parameter_interval,"span":c.span,"gaps":c.gap_bounds,"status":format!("{:?}",c.status)})).collect::<Vec<_>>();
    Ok(json!({"cells":cells,"workUsed":r.work_obstacle_cells}))
}
