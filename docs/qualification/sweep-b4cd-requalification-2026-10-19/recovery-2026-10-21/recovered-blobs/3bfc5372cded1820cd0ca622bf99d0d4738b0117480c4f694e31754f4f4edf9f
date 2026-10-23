//! Affine rational sweep between two authored guide curves.
use crate::{Result,check,curve::Curve,surface::Surface};
use crate::foundation::guards::{Budget, require_finite_f64, require_finite_point};
use crate::scaled_sweep::{normalized,binomial};

/// Profile coordinates are local [x,y,z]; width is the local x rail separation.
/// S=A(v)+(x(u)/width)*(B(v)-A(v))+y(u)*axis_y+z(u)*axis_z.
/// The transverse axes are authored dimensionless vectors, not an RMF frame.
pub fn sweep(profile:&Curve,guide_a:&Curve,guide_b:&Curve,width:f64,axis_y:[f64;3],axis_z:[f64;3])->Result<Surface>{
    profile.validate()?;
    require_finite_f64(width,"width")?;
    require_finite_point(&axis_y,"axis_y")?;
    require_finite_point(&axis_z,"axis_z")?;
    check(profile.control_points[0].len()==3 && profile.control_points.len()<=32
        && width>0.,
        "Two-guide sweep requires a 3D profile and positive width")?;
    let cross=[axis_y[1]*axis_z[2]-axis_y[2]*axis_z[1],axis_y[2]*axis_z[0]-axis_y[0]*axis_z[2],axis_y[0]*axis_z[1]-axis_y[1]*axis_z[0]];
    check(cross.iter().all(|x|x.is_finite()) && cross.iter().any(|x|*x!=0.),"Two-guide transverse axes must be independent")?;
    let a=normalized(guide_a)?;
    let b=normalized(guide_b)?;
    check(a.control_points[0].len()==3 && b.control_points[0].len()==3,"Two-guide sweep needs 3D guides")?;
    let degree=a.degree+b.degree;
    check(degree<=25,"Two-guide sweep product degree exceeds 25")?;
    let mut cuts:Vec<_>=a.knots.iter().chain(&b.knots).copied().filter(|x|*x>=0. && *x<=1.).collect();
    cuts.sort_by(f64::total_cmp);cuts.dedup();
    check(cuts.len()>=2 && (cuts.len()-1)*degree+1<=32,"Two-guide sweep exceeds 32 path controls")?;
    let max=profile.weights.iter().copied().fold(0.,f64::max);
    let mut points=vec![Vec::<Vec<f64>>::new();profile.control_points.len()];
    let mut weights=vec![Vec::<f64>::new();profile.control_points.len()];
    let mut knots=vec![0.;degree+1];
    // Unified guard as a backstop over the span-decomposition budget.
    let mut guard=Budget::with_iterations(cuts.len())?.guard("two_guide_sweep");
    for (span,cut) in cuts.windows(2).enumerate(){
        guard.tick()?;
        let a=a.trim(cut[0],cut[1])?;let b=b.trim(cut[0],cut[1])?;
        check(a.control_points.len()==a.degree+1 && b.control_points.len()==b.degree+1,"Two-guide span decomposition is not Bezier")?;
        for i in 0..profile.control_points.len(){
            let p=&profile.control_points[i];let alpha=p[0]/width;
            check(alpha.is_finite() && (p[0]==0. || alpha!=0.),"Two-guide profile width ratio is unrepresentable")?;
            for k in 0..=degree{
                let mut denominator=0.;let mut numerator=[0.;3];
                for j in 0..=a.degree{
                    if k<j || k-j>b.degree{continue;}
                    let l=k-j;
                    let w=binomial(a.degree,j)*binomial(b.degree,l)/binomial(degree,k)*a.weights[j]*b.weights[l];
                    check(w.is_finite() && w>0.,"Two-guide product weight is unrepresentable")?;
                    denominator+=w;
                    for d in 0..3{
                        let q=(1.-alpha)*a.control_points[j][d]+alpha*b.control_points[l][d]+p[1]*axis_y[d]+p[2]*axis_z[d];
                        let term=w*q;
                        check(q.is_finite() && term.is_finite() && (q==0. || term!=0.),"Two-guide homogeneous control is unrepresentable")?;
                        numerator[d]+=term;
                    }
                }
                let p:Vec<_>=numerator.into_iter().map(|x|x/denominator).collect();
                let w=profile.weights[i]/max*denominator;
                check(p.iter().all(|x|x.is_finite()) && denominator.is_finite() && denominator>0. && w.is_finite() && w>0.,"Two-guide result is unrepresentable")?;
                if span>0 && k==0{
                    check(points[i].last()==Some(&p) && weights[i].last()==Some(&w),"Two-guide span endpoint mismatch")?;
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
    fn preserves_rational_guides_and_affine_profile_coordinates(){
        let p=line([0.,0.,0.],[2.,1.,0.]);
        let a=line([0.,0.,0.],[0.,0.,4.]);
        let mut b=line([2.,0.,0.],[4.,0.,6.]);b.weights=vec![1.,2.];
        let s=sweep(&p,&a,&b,2.,[0.,1.,0.],[0.,0.,1.]).unwrap();
        for u in [0.,0.13,0.5,0.87,1.]{for v in [0.,0.17,0.5,0.83,1.]{
            let q=s.evaluate(u,v).unwrap().point;
            let bx=(2.+6.*v)/(1.+v);let bz=12.*v/(1.+v);
            assert!((q[0]-u*bx).abs()<1e-10);
            assert!((q[1]-u).abs()<1e-10);
            assert!((q[2]-((1.-u)*4.*v+u*bz)).abs()<1e-10);
        }}
    }
    #[test]
    fn rejects_degenerate_authored_frame_and_invalid_width(){
        let p=line([0.;3],[1.,0.,0.]);
        assert!(sweep(&p,&p,&p,0.,[0.,1.,0.],[0.,0.,1.]).is_err());
        assert!(sweep(&p,&p,&p,1.,[0.,1.,0.],[0.,2.,0.]).is_err());
    }
    #[test]
    fn non_finite_width_and_axes_are_typed_rejections(){
        let p=line([0.;3],[1.,0.,0.]);
        let err=sweep(&p,&p,&p,f64::NAN,[0.,1.,0.],[0.,0.,1.]).unwrap_err();
        assert_eq!(err.code,crate::INVALID_INPUT,"{err:?}");
        assert!(err.contains("width"),"{err}");
        let err=sweep(&p,&p,&p,1.,[0.,f64::INFINITY,0.],[0.,0.,1.]).unwrap_err();
        assert!(err.contains("axis_y"),"{err}");
        let err=sweep(&p,&p,&p,1.,[0.,1.,0.],[f64::NAN,0.,1.]).unwrap_err();
        assert!(err.contains("axis_z"),"{err}");
    }
    #[test]
    fn retains_rational_profile_and_independent_guide_domains(){
        let mut p=line([0.,0.,0.],[2.,1.,1.]);p.weights=vec![1.,3.];
        let a=line([0.,0.,0.],[0.,0.,4.]);
        let mut b=line([2.,0.,0.],[4.,0.,6.]);b.knots=vec![2.,2.,6.,6.];b.weights=vec![1.,2.];
        let s=sweep(&p,&a,&b,2.,[0.,2.,0.],[0.,0.,3.]).unwrap();
        for u in [0.,0.13,0.5,0.87,1.]{for v in [0.,0.17,0.5,0.83,1.]{
            let q=s.evaluate(u,v).unwrap().point;let alpha=3.*u/(1.+2.*u);
            assert!((q[0]-alpha*(2.+6.*v)/(1.+v)).abs()<1e-10);
            assert!((q[1]-2.*alpha).abs()<1e-10);
            assert!((q[2]-((1.-alpha)*4.*v+alpha*12.*v/(1.+v)+3.*alpha)).abs()<1e-10);
        }}
    }
    #[test]

#[cfg(feature = "transport")]
    fn preserves_multispan_guide_and_json_roles(){
        let p=line([0.;3],[2.,0.,0.]);
        let a=crate::primitives::polyline(&[[0.,0.,0.],[0.,0.,1.],[0.,0.,4.]],false).unwrap();
        let b=line([2.,0.,0.],[4.,0.,6.]);
        let value=crate::transport::dispatch(value_codec::json!({"op":"surface_two_guide_sweep","profile":p,"guide_a":a,"guide_b":b,"width":2.,"axis_y":[0.,1.,0.],"axis_z":[0.,0.,1.]})).unwrap();
        let s:Surface=value_codec::from_value(value).unwrap();
        assert_eq!(s.degree_v,2);
        assert_eq!(s.control_points[0].len(),5);
        for u in [0.,0.13,0.5,0.87,1.]{for v in [0.,0.17,0.5,0.83,1.]{
            let q=s.evaluate(u,v).unwrap().point;
            let az=if v<=0.5 {2.*v} else {6.*v-2.};
            assert!((q[0]-u*(2.+2.*v)).abs()<1e-10);
            assert!((q[2]-((1.-u)*az+u*6.*v)).abs()<1e-10);
        }}
    }
    #[test]
    fn closed_circular_profile_and_guides_match_independent_torus_equation(){
        let profile=crate::primitives::circle([1.,0.,0.],[0.,0.,1.],1.).unwrap();
        let a=crate::primitives::circle([0.;3],[0.,0.,1.],10.).unwrap();
        let b=crate::primitives::circle([0.;3],[0.,0.,1.],12.).unwrap();
        let s=sweep(&profile,&a,&b,2.,[0.,0.,1.],[0.,1.,0.]).unwrap();
        for u in [0.,0.13,0.37,0.63,0.87,1.]{for v in [0.,0.17,0.43,0.57,0.83,1.]{
            let q=s.evaluate(u,v).unwrap().point;
            let rho=q[0].hypot(q[1]);
            assert!(((rho-11.).powi(2)+q[2]*q[2]-1.).abs()<1e-10);
        }}
        for t in [0.,0.13,0.37,0.63,0.87,1.]{
            let a=s.evaluate(t,0.).unwrap().point;let b=s.evaluate(t,1.).unwrap().point;
            for d in 0..3{assert!((a[d]-b[d]).abs()<1e-10);}
            let a=s.evaluate(0.,t).unwrap().point;let b=s.evaluate(1.,t).unwrap().point;
            for d in 0..3{assert!((a[d]-b[d]).abs()<1e-10);}
        }
        assert!(!s.periodic_v);
    }
}
