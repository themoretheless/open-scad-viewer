use super::*;
use crate::{curve_extension::{self,Continuity,End},surface::Axis};

pub(super) fn apply(v:&Value,op:&str)->Result<Value>{
    if op=="simplify_linear_curve" {
        let r=crate::linear_simplification::simplify(&field(v,"curve")?,field(v,"max_work")?)?;
        return Ok(json!({"curve":r.curve,"retainedControls":r.retained_source_controls,"removedControls":r.removed_source_controls,
            "complete":r.complete,"reason":r.reason.map(|s|format!("{s:?}")),"work":r.work_used}));
    }
    let class=match field::<String>(v,"continuity_class")?.as_str(){
        "G1"=>Continuity::G1,"G2"=>Continuity::G2,"C2"=>Continuity::C2,_=>return Err(input("Unknown extension continuity class")),
    };
    let length=field(v,"distance_mm")?;
    if op=="extend_terminal_curve" {
        let end=match field::<String>(v,"source_end")?.as_str(){"start"=>End::Start,"end"=>End::Stop,_=>return Err(input("Unknown curve end"))};
        let r=curve_extension::extend_curve(&field(v,"curve")?,end,length,class)?;
        return Ok(json!({"curve":r.curve,"domain":r.curve.domain(),"stable":r.stable,"extrapolationGrowth":r.extrapolation_growth,
            "requestedContinuity":format!("{class:?}"),"continuityCertified":false}));
    }
    let (axis,end)=match field::<String>(v,"source_edge")?.as_str(){
        "u_min"=>(Axis::U,End::Start),"u_max"=>(Axis::U,End::Stop),"v_min"=>(Axis::V,End::Start),"v_max"=>(Axis::V,End::Stop),
        _=>return Err(input("Unknown surface boundary")),
    };
    let r=crate::editing::surface_extension::extend_surface(&field(v,"surface")?,axis,end,length,class)?;
    Ok(json!({"surface":r.surface,"stable":r.stable,"extrapolationGrowth":r.max_extrapolation_growth,
        "requestedContinuity":format!("{class:?}"),"continuityCertified":false}))
}
