//! Rational conic-parameter twist around a fixed authored axis.
use crate::{Result,check,curve::Curve,surface::Surface};
use crate::scaled_sweep::{normalized,binomial};

/// Rotate P(u)-origin around axis with rational conic angle parameterization.
/// Add the path translation C(v)-C(0); this is not an RMF or arc-length twist.
pub fn sweep(profile:&Curve,path:&Curve,origin:[f64;3],axis:[f64;3],start_degrees:f64,sweep_degrees:f64)->Result<Surface>{
    profile.validate()?;
    check(profile.control_points[0].len()==3 && profile.control_points.len()<=32
        && origin.iter().chain(&axis).all(|x|x.is_finite()) && start_degrees.is_finite() && sweep_degrees.is_finite(),
        "Twist sweep needs a 3D profile, finite origin, axis and angles")?;
    let max_axis=axis.iter().fold(0_f64,|a,x|a.max(x.abs()));
    check(max_axis>0.,"Twist sweep axis must be nonzero")?;
    let n=axis.map(|x|x/max_axis);
    let length=n.iter().map(|x|x*x).sum::<f64>().sqrt();
    let n=n.map(|x|x/length);
    let a=normalized(path)?;
    check(a.control_points[0].len()==3,"Twist sweep needs a 3D path")?;
    let start=a.control_points[0].clone();
    let b=if sweep_degrees==0. {
        let angle=start_degrees.to_radians();
        check(angle.is_finite(),"Twist start angle is unrepresentable")?;
        let point=[angle.cos(),angle.sin(),0.];
        crate::paths::bezier(vec![point.to_vec(),point.to_vec()],None)?
    }else{crate::primitives::circle_arc([0.;3],[0.,0.,1.],1.,start_degrees,sweep_degrees)?};
    let degree=a.degree+b.degree;
    check(degree<=25,"Twist sweep product degree exceeds 25")?;
    let mut cuts:Vec<_>=a.knots.iter().chain(&b.knots).copied().filter(|x|*x>=0. && *x<=1.).collect();
    cuts.sort_by(f64::total_cmp);cuts.dedup();
    check(cuts.len()>=2 && (cuts.len()-1)*degree+1<=32,"Twist sweep exceeds 32 path controls")?;
    let max=profile.weights.iter().copied().fold(0.,f64::max);
    let mut points=vec![Vec::<Vec<f64>>::new();profile.control_points.len()];
    let mut weights=vec![Vec::<f64>::new();profile.control_points.len()];
    let mut knots=vec![0.;degree+1];
    for (span,cut) in cuts.windows(2).enumerate(){
        let a=a.trim(cut[0],cut[1])?;let b=b.trim(cut[0],cut[1])?;
        check(a.control_points.len()==a.degree+1 && b.control_points.len()==b.degree+1,"Twist span decomposition is not Bezier")?;
        for i in 0..profile.control_points.len(){
            let relative=std::array::from_fn::<_,3,_>(|d|profile.control_points[i][d]-origin[d]);
            let dot=relative.iter().zip(n).map(|(x,y)|x*y).sum::<f64>();
            let parallel=n.map(|x|x*dot);
            let tangent=[n[1]*relative[2]-n[2]*relative[1],n[2]*relative[0]-n[0]*relative[2],n[0]*relative[1]-n[1]*relative[0]];
            for k in 0..=degree{
                let mut denominator=0.;let mut numerator=[0.;3];
                for j in 0..=a.degree{
                    if k<j || k-j>b.degree{continue;}
                    let l=k-j;
                    let w=binomial(a.degree,j)*binomial(b.degree,l)/binomial(degree,k)*a.weights[j]*b.weights[l];
                    check(w.is_finite() && w>0.,"Twist product weight is unrepresentable")?;
                    denominator+=w;
                    for d in 0..3{
                        let q=origin[d]+(a.control_points[j][d]-start[d])+parallel[d]
                            +b.control_points[l][0]*(relative[d]-parallel[d])+b.control_points[l][1]*tangent[d];
                        let term=w*q;
                        check(q.is_finite() && term.is_finite() && (q==0. || term!=0.),"Twist homogeneous control is unrepresentable")?;
                        numerator[d]+=term;
                    }
                }
                let p:Vec<_>=numerator.into_iter().map(|x|x/denominator).collect();
                let w=profile.weights[i]/max*denominator;
                check(p.iter().all(|x|x.is_finite()) && denominator.is_finite() && denominator>0. && w.is_finite() && w>0.,"Twist result is unrepresentable")?;
                if span>0 && k==0{
                    check(points[i].last()==Some(&p) && weights[i].last()==Some(&w),"Twist span endpoint mismatch")?;
                }else{points[i].push(p);weights[i].push(w);}
            }
        }
        if span+2<cuts.len(){knots.extend(std::iter::repeat_n(cut[1],degree));}
    }
    knots.extend(std::iter::repeat_n(1.,degree+1));
    let result=Surface{degree_u:profile.degree,degree_v:degree,knots_u:profile.knots.clone(),knots_v:knots,control_points:points,weights,periodic_u:profile.periodic,periodic_v:false};
    result.validate()?;Ok(result)
}

#[cfg(test)]
mod tests{
    use super::*;
    fn line(a:[f64;3],b:[f64;3])->Curve{crate::primitives::line(a,b).unwrap()}
    #[test]
    fn matches_independent_quarter_turn_conic_and_translation(){
        let p=line([1.,0.,0.],[2.,0.,0.]);let path=line([0.;3],[0.,0.,4.]);
        let s=sweep(&p,&path,[0.;3],[0.,0.,1.],0.,90.).unwrap();
        for u in [0.,0.13,0.5,0.87,1.]{for v in [0_f64,0.17,0.5,0.83,1.]{
            let w=(1.-v).powi(2)+2.*std::f64::consts::FRAC_1_SQRT_2*v*(1.-v)+v*v;
            let c=((1.-v).powi(2)+std::f64::consts::SQRT_2*v*(1.-v))/w;
            let t=(std::f64::consts::SQRT_2*v*(1.-v)+v*v)/w;
            let q=s.evaluate(u,v).unwrap().point;
            assert!((q[0]-(1.+u)*c).abs()<1e-10);
            assert!((q[1]-(1.+u)*t).abs()<1e-10);
            assert!((q[2]-4.*v).abs()<1e-10);
        }}
    }
    #[test]
    fn respects_arbitrary_axis_center_and_distant_path_anchor(){
        let p=line([10.,21.,30.],[10.,22.,30.]);let path=line([1e9,0.,10.],[1e9,0.,14.]);
        let s=sweep(&p,&path,[10.,20.,30.],[1e300,0.,0.],0.,90.).unwrap();
        let q=s.evaluate(0.5,1.).unwrap().point;
        assert!((q[0]-10.).abs()<1e-10);assert!((q[1]-20.).abs()<1e-10);assert!((q[2]-35.5).abs()<1e-10);
        assert!(sweep(&p,&path,[10.,20.,30.],[0.;3],0.,90.).is_err());
    }
    #[test]
    fn preserves_rational_profile_for_a_full_turn_and_constant_rotation(){
        let mut p=line([1.,0.,0.],[2.,0.,0.]);p.weights=vec![1.,2.];
        let path=line([0.;3],[0.,0.,4.]);
        for angle in [0.,360.]{
            let s=sweep(&p,&path,[0.;3],[0.,0.,1.],0.,angle).unwrap();
            for u in [0.,0.13,0.5,0.87,1.]{for v in [0_f64,0.17,0.5,0.83,1.]{
                let q=s.evaluate(u,v).unwrap().point;let radius=(1.+3.*u)/(1.+u);
                assert!((q[0]*q[0]+q[1]*q[1]-radius*radius).abs()<1e-10);
                assert!((q[2]-4.*v).abs()<1e-10);
            }}
        }
    }
}
