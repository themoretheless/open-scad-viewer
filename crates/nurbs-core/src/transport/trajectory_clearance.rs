use super::*;
use crate::analysis::curve_measure::{
    ToolpathObstacle, measure_minimum_toolpath_clearance, partition_toolpath_clearance_with_margin,
    partition_toolpath_clearance,
};
use crate::analysis::curve_measure::{
    measure_minimum_spherical_clearance, measure_minimum_box_clearance, measure_minimum_oriented_box_clearance,
    measure_minimum_capsule_clearance, SphericalMinimumClearance,
    BoxObstacle, CapsuleObstacle, FirstToolpathContact, OrientedBoxObstacle, SphericalObstacle,
    partition_curve_box_clearance, partition_curve_capsule_clearance,
    partition_curve_oriented_box_clearance, partition_curve_spherical_clearance,
};

pub(super) const OPS:&[&str]=&["sphere_path_clearance","capsule_path_clearance","box_path_clearance",
    "oriented_box_path_clearance","first_sphere_contact","minimum_sphere_partition_gap","minimum_mixed_gap","mixed_path_clearance","mixed_margin_clearance","minimum_sphere_gap","minimum_box_gap","minimum_oriented_box_gap","minimum_capsule_gap"];

fn minimum(r:SphericalMinimumClearance)->Value{
    json!({"gap":r.gap_bounds,"witnessParameter":r.witness_parameter,"witnessObstacle":r.witness_obstacle,
        "workUsed":r.work_obstacle_cells,"stopReason":format!("{:?}",r.stop_reason)})
}

pub(super) fn dispatch(v:&Value,op:&str)->Result<Value>{
    let curve=field(v,"curve")?;
    let domain=field(v,"domain")?;
    let radius=field(v,"radius_mm")?;
    let minimize=matches!(op,"minimum_sphere_gap"|"minimum_capsule_gap"|"minimum_box_gap"|"minimum_oriented_box_gap");
    let tol=field(v,if minimize || op=="minimum_mixed_gap" {"gap_tolerance_mm"} else {"parameter_tolerance"})?;
    let budget=field(v,"max_cells")?;
    let r=match op {
        "minimum_mixed_gap" | "mixed_path_clearance" | "mixed_margin_clearance" => {
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
            if op == "minimum_mixed_gap" {
                return Ok(minimum(measure_minimum_toolpath_clearance(&curve, domain, &obstacles, radius, tol, budget)?));
            }
            if op == "mixed_margin_clearance" {
                partition_toolpath_clearance_with_margin(&curve, domain, &obstacles, radius, field(v, "margin_mm")?, tol, budget)?
            } else {
                partition_toolpath_clearance(&curve, domain, &obstacles, radius, tol, budget)?
            }
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
    let cells=r.cells.into_iter().map(|c|json!({"domain":c.parameter_interval,"span":c.span,"gaps":c.gap_bounds,"status":format!("{:?}",c.status)})).collect::<Vec<_>>();
    Ok(json!({"cells":cells,"workUsed":r.work_obstacle_cells}))
}
