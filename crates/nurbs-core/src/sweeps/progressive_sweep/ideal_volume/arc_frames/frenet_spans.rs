//! Normalize and compose EACH original span before taking a field hull.
//! Mixing raw speeds/accelerations across a knot first can create a spurious
//! zero curvature denominator. No rounded extracted curve becomes a source.
use super::*;
fn unknown(cells:usize,reason:Option<&'static str>)->frame::Report {
    frame::Report{status:Status::Unresolved,cells,longitudinal:None,transverse:None,binormal:None,single_span:false,reason}
}
pub(in crate::sweeps::progressive_sweep::ideal_volume) fn certify(s:&Sweep<'_>,station:[f64;2],p:frame::PathPhase,length:I,max:usize)->Result<frame::Report> {
    let [a,b]=s.path.domain();let width=I::point(b).sub(I::point(a))?;
    let raw=I::point(a).add(width.mul(I::new(p.traversal[0],p.traversal[1])?)?)?.intersect(a,b)?;
    let mut spent=0;let mut answer:Option<frame::Report>=None;let mut count=0;
    for span in s.path.degree..s.path.control_points.len() {
        let lo=raw.lo.max(s.path.knots[span]);let hi=raw.hi.min(s.path.knots[span+1]);
        if s.path.knots[span]>=s.path.knots[span+1] || lo>hi {continue;}
        if spent==max {return Ok(unknown(spent,Some("arc-frenet-span-work-limit")));}spent+=1;
        let (r,source)=frame::certify_frenet_span(s.path,span,[lo,hi],max-spent)?;
        if r.status!=Status::Certified {return Ok(unknown(spent+r.cells,r.reason));}
        let [v,acc]=source.unwrap();let speed=iv::norm(v)?;
        if speed.lo<=0. {return Ok(unknown(spent+r.cells,Some("arc-frenet-span-speed-unproved")));}
        let q=speed.mul(speed)?;
        let phase=frame::PathPhase{traversal:p.traversal,first:length.div(speed)?,
            second:I::point(0.).sub(length.mul(length)?.mul(iv::dot_tight(v,acc)?)?.div(q.mul(q)?)?)?,position_jets:None};
        let r=frame::compose_and_twist(r,phase,s.twist,station,max-spent)?;spent+=r.cells;
        if r.status!=Status::Certified {return Ok(unknown(spent,r.reason));}
        count+=1;
        if let Some(old)=&mut answer {
            for (a,b) in [(&mut old.longitudinal,&r.longitudinal),(&mut old.transverse,&r.transverse),(&mut old.binormal,&r.binormal)] {
                let a=a.as_mut().unwrap();let b=b.as_ref().unwrap();
                for (a,b) in [(&mut a.value,&b.value),(&mut a.first,&b.first),(&mut a.second,&b.second)] {
                    for k in 0..3 {a[k]=[a[k][0].min(b[k][0]),a[k][1].max(b[k][1])];}
                }
            }
        }else{answer=Some(r);}
    }
    let Some(mut out)=answer else{return Ok(unknown(spent,Some("arc-frenet-span-domain-unproved")));};
    out.cells=spent;out.single_span=count==1;Ok(out)
}
