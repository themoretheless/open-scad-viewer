//! Original rational fourth derivative with a shared lower-jet work budget.
use super::*;
#[derive(Clone,Debug)]
pub struct FourthReport {pub base:Report,pub third:Option<[f64;2]>,pub fourth:Option<[f64;2]>}
fn refuse(out:&mut FourthReport,reason:&'static str){
    out.base.status=Status::Unresolved;out.base.value=None;out.base.first=None;out.base.second=None;
    out.base.single_span=false;out.base.reason=Some(reason);out.third=None;out.fourth=None;
}
pub fn certify_fourth_traversal(c:&Curve,traversal:[f64;2],max_cells:usize)->Result<FourthReport>{
    let lower=certify_third_traversal(c,traversal,max_cells)?;
    let mut out=FourthReport {base:lower.base,third:lower.third,fourth:None};
    if out.base.status!=Status::Certified {return Ok(out);}
    let [a,b]=c.domain();
    let mapped=Interval::point(a).add(Interval::point(b).sub(Interval::point(a))?.mul(Interval::new(traversal[0],traversal[1])?)?)?.intersect(a,b)?;
    let lo=mapped.lo.next_down().max(a);let hi=mapped.hi.next_up().min(b);
    let mut minimum=f64::INFINITY;let mut maximum=f64::NEG_INFINITY;
    for span in c.degree..c.control_points.len(){
        let l=lo.max(c.knots[span]);let h=hi.min(c.knots[span+1]);if l>=h {continue;}
        if out.base.cells==max_cells {refuse(&mut out,"fourth-derivative-work-limit");return Ok(out);}
        out.base.cells+=1;
        let result=(||->Result<Interval>{
            // Differentiate the original full-span homogeneous polynomial;
            // narrow restriction must not divide tiny rounded pole differences.
            let a=c.knots[span];let b=c.knots[span+1];
            let width=Interval::point(b).sub(Interval::point(a))?;
            let controls=crate::curve_distance::restricted_controls(c,span,Interval::new(a,b)?)?;
            let local=Interval::new(l,h)?.sub(Interval::point(a))?.div(width)?.intersect(0.,1.)?;
            let mut n=controls.iter().map(|p|p[0]).collect::<Vec<_>>();
            let mut w=controls.iter().map(|p|p[3]).collect::<Vec<_>>();
            let evaluate=|v:Vec<Interval>|restricted_hull(&v,local);
            let weight=evaluate(w.clone())?;
            let mut ns=[Interval::point(0.);5];let mut ws=ns;
            ns[0]=evaluate(n.clone())?;ws[0]=weight;
            for k in 1..=4 {
                n=derivative(&n,c.degree.saturating_sub(k-1),width)?;
                w=derivative(&w,c.degree.saturating_sub(k-1),width)?;
                ns[k]=evaluate(n.clone())?;ws[k]=evaluate(w.clone())?;
            }
            // Leibniz recurrence N=R*W, retaining all rational terms.
            let choose=[[1.,0.,0.,0.,0.],[1.,1.,0.,0.,0.],[1.,2.,1.,0.,0.],[1.,3.,3.,1.,0.],[1.,4.,6.,4.,1.]];
            let mut r=[Interval::point(0.);5];r[0]=ns[0].div(weight)?;
            for k in 1..=4 {
                let mut numerator=ns[k];
                for j in 1..=k {numerator=numerator.sub(r[k-j].mul(ws[j])?.mul(Interval::point(choose[k][j]))?)?;}
                r[k]=numerator.div(weight)?;
            }
            Ok(r[4])
        })();
        let Ok(v)=result else {refuse(&mut out,"fourth-derivative-numeric-unresolved");return Ok(out);};
        minimum=minimum.min(v.lo);maximum=maximum.max(v.hi);
    }
    if !minimum.is_finite()||!maximum.is_finite(){refuse(&mut out,"fourth-derivative-domain-unresolved");return Ok(out);}
    out.fourth=Some([minimum,maximum]);Ok(out)
}

#[test]
fn original_rational_fourth_derivative_keeps_domain_and_shared_budget(){
    let c=Curve {degree:1,knots:vec![2.,2.,5.,5.],control_points:vec![vec![0.;3],vec![1.,0.,0.]],weights:vec![1.,2.],periodic:false};
    let r=certify_fourth_traversal(&c,[0.25,0.5],100).unwrap();
    assert_eq!(r.base.status,Status::Certified);assert_eq!(r.base.cells,3);
    for t in [0.25_f64,0.375,0.5] {
        let expected=-48./(3_f64.powi(4)*(1.+t).powi(5));let bound=r.fourth.unwrap();
        assert!(bound[0]<=expected&&expected<=bound[1]);
    }
    let short=certify_fourth_traversal(&c,[0.25,0.5],r.base.cells-1).unwrap();
    assert_eq!(short.base.status,Status::Unresolved);
    assert!(short.base.value.is_none()&&short.third.is_none()&&short.fourth.is_none());
}
