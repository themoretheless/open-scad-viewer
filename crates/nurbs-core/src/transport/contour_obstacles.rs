use super::*;
use super::rotating_tools::xy_curve;
use crate::analysis::curve_measure::*;
pub(super) const OPS:&[&str]=&["contour_box_distance","contour_disk_distance","contour_capsule_distance","contour_rectangle_distance","planar_contour_sets_distance"];

pub(super) fn dispatch(v:&Value,op:&str)->Result<Value>{
    let raw:Vec<crate::curve::Curve>=field(v,"curves")?;
    check((1..=32).contains(&raw.len()),"Expected one to 32 planar contours")?;
    let curves=raw.into_iter().map(xy_curve).collect::<Result<Vec<_>>>()?;
    let contours=curves.iter().collect::<Vec<_>>();
    let tol=field(v,"distance_tolerance_mm")?;
    let budget=field(v,"max_cells")?;
    if op=="planar_contour_sets_distance" {
        let n:usize=field(v,"first_count")?;
        check(n>0 && n<contours.len(),"Both contour sets require at least one curve")?;
        let (left,right)=contours.split_at(n);
        let r=measure_planar_contour_sets_distance(left,right,tol,budget)?;
        return Ok(json!({"distance":r.bounds,"witnessContours":r.witness_contours,"witnessParameters":r.witness_parameters,
            "workUsed":r.work_cells,"converged":r.converged}));
    }
    let r=match op{
        "contour_box_distance"=>measure_planar_contour_set_box_distance(&contours,field(v,"bounds")?,tol,budget)?,
        "contour_disk_distance"=>measure_planar_contour_set_disk_distance(&contours,field(v,"center")?,field(v,"radius_mm")?,tol,budget)?,
        "contour_capsule_distance"=>{
            let endpoints:[[f64;2];2]=field(v,"endpoints")?;
            let capsule=CapsuleObstacle{endpoints:[&endpoints[0],&endpoints[1]],radius:field(v,"radius_mm")?};
            measure_planar_contour_set_capsule_distance(&contours,capsule,tol,budget)?
        }
        "contour_rectangle_distance"=>{
            let axes:[[f64;2];2]=field(v,"axes")?;
            let center:[f64;2]=field(v,"center")?;
            let half_extents:[f64;2]=field(v,"half_extents")?;
            let rectangle=OrientedBoxObstacle{center:&center,axes:&[&axes[0],&axes[1]],half_extents:&half_extents};
            measure_planar_contour_set_oriented_box_distance(&contours,rectangle,tol,budget)?
        }
        _=>return Err(input("Unknown contour distance operation")),
    };
    Ok(json!({"distance":r.bounds,"witnessContour":r.witness_contour,"witnessParameter":r.witness_parameter,
        "workUsed":r.work_cells,"stopReason":format!("{:?}",r.stop_reason)}))
}
