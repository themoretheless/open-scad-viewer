use super::*;
use crate::curve::Curve;
use crate::editing::{exact_curve_merge as merge,exact_knot_removal as removal};
pub(super) const OPS: &[&str] = &["compress_profile_curve", "compress_profile_segments", "compress_profile_chain", "orient_profile_chain", "restore_profile_partition", "assemble_open_profile", "assemble_closed_profile", "assemble_planar_profile", "orient_planar_profile", "assemble_projected_profile", "assemble_oblique_profile", "assemble_spatial_profile"];

pub(super) fn apply(v:&Value,op:&str)->Result<Value> {
    let mut sources:Vec<Curve>=field(v,"curves")?;
    check((1..=256).contains(&sources.len()),"Expected one to 256 curve segments")?;
    let work=field(v,"max_work")?;
    let input_count=sources.len();
    let flatten=matches!(op,"assemble_planar_profile"|"orient_planar_profile");
    if flatten {
        sources=sources.into_iter().map(super::super::rotating_tools::xy_curve).collect::<Result<Vec<_>>>()?;
    }
    let (mut curve,mut report)=if op=="compress_profile_curve" {
        check(input_count==1,"Expected one profile curve")?;
        let r=removal::compress_curve(&sources[0],work)?;
        (r.curve,json!({"completed":r.completed,"removedKnots":r.removed_knots,"reducedDegrees":r.reduced_degrees,"workUsed":r.work_used}))
    }else if op=="compress_profile_segments" {
        let i=field::<usize>(v,"piece")?;
        check(i<input_count,"Selected segment index is out of range")?;
        let r=removal::compress_curve_batch(&sources,work)?;
        let mut segments=r.segments.ok_or_else(||input("Exact batch compression exhausted its work budget"))?;
        let counts:Vec<_>=segments.iter().map(|s|s.curve.control_points.len()).collect();
        (segments.swap_remove(i).curve,json!({"completed":true,"controlCounts":counts,"selected":i,"workUsed":r.work_used}))
    }else{
        let index=||field::<usize>(v,"first_segment");
        let reverse=||field::<bool>(v,"reverse");
        let pairs=||field::<usize>(v,"max_pairs");
        let cells=||field::<usize>(v,"max_cells");
        let r=match op {
            "compress_profile_chain"=>merge::compress_mixed_degree_chain(&sources,work)?,
            "orient_profile_chain"=>merge::orient_and_compress_chain(&sources,work)?,
            "restore_profile_partition"=>merge::assemble_parameter_partition(&sources,work)?,
            "assemble_open_profile"=>merge::assemble_open_endpoint_chain(&sources,&field::<Vec<f64>>(v,"start")?,work)?,
            "assemble_closed_profile"=>merge::assemble_closed_endpoint_chain(&sources,index()?,reverse()?,work)?,
            "assemble_planar_profile"=>merge::assemble_simple_planar_loop(&sources,index()?,reverse()?,work,field(v,"tolerance_uv")?,pairs()?,cells()?)?,
            "orient_planar_profile"=>merge::assemble_wound_planar_loop(&sources,index()?,field(v,"clockwise")?,work,field(v,"tolerance_uv")?,field(v,"tolerance_mm2")?,pairs()?,cells()?)?,
            "assemble_projected_profile"=>merge::assemble_simple_spatial_loop(&sources,index()?,reverse()?,field(v,"dropped_axis")?,work,field(v,"tolerance_mm")?,pairs()?,cells()?)?,
            "assemble_oblique_profile"=>merge::assemble_oblique_simple_loop(&sources,index()?,reverse()?,field(v,"projection_forms")?,work,field(v,"tolerance_mm")?,pairs()?,cells()?)?,
            "assemble_spatial_profile"=>merge::assemble_pairwise_simple_spatial_loop(&sources,index()?,reverse()?,work,field(v,"tolerance_mm")?,pairs()?,cells()?)?,
            _=>return Err(input("Unknown profile compression operation")),
        };
        let c=r.curve.ok_or_else(||input(format!("Exact profile assembly refused: {:?}",r.refusal)))?;
        (c,json!({"completed":true,"workUsed":r.work_used}))
    };
    if flatten {for point in &mut curve.control_points {point.push(0.);}}
    let [lo,hi]=curve.domain();
    report["sourceSegments"]=json!(input_count);
    report["degree"]=json!(curve.degree);
    report["controlCount"]=json!(curve.control_points.len());
    report["domain"]=json!([lo,hi]);
    report["start"]=json!(curve.evaluate(lo)?.point);
    report["end"]=json!(curve.evaluate(hi)?.point);
    report["curve"]=json!(curve);
    Ok(report)
}
