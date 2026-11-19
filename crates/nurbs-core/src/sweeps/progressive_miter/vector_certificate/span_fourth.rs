//! Complete XYZ jets of ONE original B-spline span, before nonlinear operations.
use super::*;
use crate::distance_bounds::Interval as I;
pub(crate) fn certify(c:&Curve,span:usize,region:[f64;2],max:usize)->Result<FourthReport> {
    c.validate()?;
    check(max<=100000 && (1..=32).contains(&c.degree) && c.control_points.iter().all(|p|p.len()==3)
        && span>=c.degree && span<c.control_points.len() && c.knots[span]<c.knots[span+1]
        && c.knots[span]<=region[0] && region[0]<=region[1] && region[1]<=c.knots[span+1],"Invalid original fourth span")?;
    let mut out=FourthReport{base:Report{status:Status::Unresolved,cells:0,value:None,first:None,second:None,
        single_span:false,reason:Some("fourth-span-unresolved")},third:None,fourth:None};
    let mut jets=[[[0.;2];3];5];
    for axis in 0..3 {
        // Same conservative three derivative stages per component as the
        // complete traversal owner. Failed source arithmetic stays charged.
        if 3>max-out.base.cells {return Ok(out);}out.base.cells+=3;
        let Ok(j)=scalar_certificate::fourth_span_jets_axis(c,axis,span,I::new(region[0],region[1])?) else{return Ok(out);};
        for k in 0..5 {jets[k][axis]=[j[k].lo,j[k].hi];}
    }
    out.base.status=Status::Certified;out.base.value=Some(jets[0]);out.base.first=Some(jets[1]);out.base.second=Some(jets[2]);
    out.third=Some(jets[3]);out.fourth=Some(jets[4]);out.base.single_span=true;out.base.reason=None;Ok(out)
}

#[test]
fn original_span_fourth_jets_keep_rational_derivatives_raw_units_and_complete_budget() {
    let c=Curve{degree:1,knots:vec![2.,2.,5.,5.],control_points:vec![vec![0.;3],vec![1.,2.,10.]],weights:vec![1.,2.],periodic:false};
    let q=certify(&c,1,[3.5,3.5],9).unwrap();assert_eq!(q.base.status,Status::Certified);
    let jets=[q.base.value.unwrap(),q.base.first.unwrap(),q.base.second.unwrap(),q.third.unwrap(),q.fourth.unwrap()];
    for (k,value) in [2./3.,8./27.,-32./243.,64./729.,-512./6561.].into_iter().enumerate(){
        for (axis,scale) in [1.,2.,10.].into_iter().enumerate(){assert!(jets[k][axis][0]<=value*scale && value*scale<=jets[k][axis][1]);}
    }
    for budget in [0,8] {let r=certify(&c,1,[3.5,3.5],budget).unwrap();assert_eq!(r.base.status,Status::Unresolved);
        assert!(r.base.cells<=budget && r.base.value.is_none() && r.base.first.is_none() && r.base.second.is_none() && r.third.is_none() && r.fourth.is_none());}
}
