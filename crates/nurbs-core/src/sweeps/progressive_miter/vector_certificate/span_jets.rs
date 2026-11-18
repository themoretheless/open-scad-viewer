//! XYZ first/second jets on one unchanged original knot span, including limits.
use super::*;
pub(crate) fn certify(c:&Curve,span:usize,region:[f64;2],max:usize)->Result<Report> {
    c.validate()?;
    check(max<=100000 && (1..=32).contains(&c.degree) && c.control_points.iter().all(|p|p.len()==3)
        && span>=c.degree && span<c.control_points.len() && c.knots[span]<c.knots[span+1]
        && c.knots[span]<=region[0] && region[0]<=region[1] && region[1]<=c.knots[span+1],"Invalid original span jets")?;
    let mut out=Report{status:Status::Unresolved,cells:0,value:None,first:None,second:None,single_span:false,reason:Some("span-jets-unresolved")};
    let mut jets=[[[0.;2];3];3];
    for axis in 0..3 {
        if out.cells==max {return Ok(out);}out.cells+=1;
        let Ok(j)=scalar_certificate::span_jets_axis(c,axis,span,region[0],region[1]) else{return Ok(out);};
        for k in 0..3 {jets[k][axis]=[j[k].lo,j[k].hi];}
    }
    out.status=Status::Certified;out.value=Some(jets[0]);out.first=Some(jets[1]);out.second=Some(jets[2]);
    out.single_span=true;out.reason=None;Ok(out)
}
