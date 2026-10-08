//! Original Frenet value enclosure from rational path velocity/acceleration.
//! No sampled frame premise, corrected Frenet sign transport or body proof.
use super::*;

/// Whole original-path Frenet jet regularity, with one adaptive shared budget.
/// Proves nonzero tangent/curvature direction; no seam continuity, surface R or I.
pub fn certify_frenet_cover(path:&Curve,twist:&Curve,max_cells:usize)->Result<RegularityReport>{
    check(max_cells<=100000,"Frenet cover work exceeds100000 cells")?;
    path.validate()?;
    check(path.control_points.iter().all(|p|p.len()==3),"Frenet path requires XYZ controls")?;
    super::super::validate_law(twist,false)?;
    certify_frame_cover(max_cells,|interval,remaining|certify_frenet_path(path,twist,interval,remaining))
}


pub fn certify_frenet_path(path:&Curve,twist:&Curve,traversal:[f64;2],max_cells:usize)->Result<Report>{
    super::super::validate_law(twist,false)?;
    let p=vector_certificate::certify_fourth_traversal(path,traversal,max_cells)?;
    let mut out=Report {status:Status::Unresolved,cells:p.base.cells,longitudinal:None,transverse:None,binormal:None,
        single_span:false,reason:p.base.reason};
    if p.base.status!=Status::Certified {return Ok(out);}
    let position=jet(path,&p.base)?;let [a,b]=path.domain();let width=I::point(b).sub(I::point(a))?;
    let w2=width.mul(width)?;let w3=w2.mul(width)?;let w4=w3.mul(width)?;
    let jerk=mul(decode(p.third.unwrap())?,w3)?;let snap=mul(decode(p.fourth.unwrap())?,w4)?;
    let velocity=Jet {v:position.d,d:position.dd,dd:jerk};
    let acceleration=Jet {v:position.dd,d:jerk,dd:snap};
    out.reason=Some("frenet-path-tangent-unresolved");
    let Some(t)=normalize(velocity)? else {return Ok(out);};
    out.reason=Some("frenet-curvature-direction-unresolved");
    let Some(b)=normalize(cross_jet(velocity,acceleration)?)? else {return Ok(out);};
    let n=cross_jet(b,t)?;
    out.status=Status::Certified;out.reason=None;out.single_span=p.base.single_span;
    out.longitudinal=Some(encode(t));out.transverse=Some(encode(n));out.binormal=Some(encode(b));
    apply_twist(out,twist,traversal,max_cells)
}

pub fn certify_frenet_control_trajectories(
    path:&Curve,scale:&Curve,twist:&Curve,affine:Option<(&Curve,&Curve)>,
    qs:&[[[f64;2];3]],traversal:[f64;2],max_cells:usize,
)->Result<TrajectoriesReport>{
    path.validate()?;
    check(path.control_points.iter().all(|p|p.len()==3),"Frenet path requires XYZ controls")?;
    if (0..3).any(|k|path.control_points.iter().all(|p|p[k]==path.control_points[0][k])) {
        return certify_frenet_planar_control_trajectories(path,scale,twist,affine,qs,traversal,max_cells);
    }
    let frame=certify_frenet_path(path,twist,traversal,max_cells)?;
    super::trajectory::control_trajectories_with_fit(path,scale,affine,qs,traversal,max_cells,frame,None)
}

pub fn certify_frenet_path_values(
    path:&Curve,twist:&Curve,traversal:[f64;2],max_cells:usize,
)->Result<ValuesReport>{
    frenet_values_at(path,twist,traversal,traversal,max_cells)
}

fn frenet_values_at(
    path:&Curve,twist:&Curve,traversal:[f64;2],law_traversal:[f64;2],max_cells:usize,
)->Result<ValuesReport>{
    super::super::validate_law(twist,false)?;
    let p=vector_certificate::certify_traversal(path,traversal,max_cells,false)?;
    let mut out=ValuesReport {status:Status::Unresolved,cells:p.cells,longitudinal:None,transverse:None,binormal:None};
    if p.status!=Status::Certified {return Ok(out);}
    let constant=|v|Jet {v,d:[I::point(0.);3],dd:[I::point(0.);3]};
    let Some(t)=normalize(constant(decode(p.first.unwrap())?))? else {return Ok(out);};
    // Positive path-domain factors cancel under normalization. Cross with the
    // original acceleration removes its tangential component without samples.
    let curvature=cross(t.v,decode(p.second.unwrap())?)?;
    let plane=(0..3).find(|&k|path.control_points.iter().all(|q|q[k]==path.control_points[0][k]));
    let b=if let Some(axis)=plane.filter(|&k|curvature[k].lo>0. || curvature[k].hi<0.) {
        // Exact original plane and strict curvature sign make binormal an
        // exact coordinate axis, including a C1 knot with differing speeds.
        let mut v=[I::point(0.);3];v[axis]=I::point(if curvature[axis].lo>0. {1.} else {-1.});constant(v)
    } else {let Some(b)=normalize(constant(curvature))? else {return Ok(out);};b};
    let n=cross(b.v,t.v)?;
    let charge=(twist.degree..twist.control_points.len()).filter(|&i|twist.knots[i]<twist.knots[i+1]).count();
    if charge>max_cells-out.cells {return Ok(out);}
    let theta=super::super::scalar_certificate::value_traversal(twist,law_traversal,charge)?;
    out.cells+=charge;
    let Some(theta)=theta else {return Ok(out);};
    let trig=super::super::trigonometric_certificate::certify(theta)?;
    let n=add(mul(n,I::new(trig.cos[0],trig.cos[1])?)?,mul(b.v,I::new(trig.sin[0],trig.sin[1])?)?)?;
    out.status=Status::Certified;
    out.longitudinal=Some(t.v.map(|x|[x.lo,x.hi]));
    out.transverse=Some(n.map(|x|[x.lo,x.hi]));
    out.binormal=Some(cross(t.v,n)?.map(|x|[x.lo,x.hi]));
    Ok(out)
}

/// Relative Frenet pose: original path derivatives and station laws retain
/// distinct parameter intervals. Translation is covered separately by callers.
pub(crate) fn certify_frenet_relative_values(
    path:&Curve,scale:&Curve,twist:&Curve,affine:Option<(&Curve,&Curve)>,
    qs:&[[[f64;2];3]],path_traversal:[f64;2],law_traversal:[f64;2],max_cells:usize,
)->Result<ControlValuesReport>{
    let frame=frenet_values_at(path,twist,path_traversal,law_traversal,max_cells)?;
    let zero=crate::sweeps::progressive_sweep::constant_vector_law([0.;3])?;
    super::trajectory::control_values_with_fit(&zero,scale,affine,qs,law_traversal,max_cells,frame,None)
}

pub fn certify_frenet_control_values(
    path:&Curve,scale:&Curve,twist:&Curve,affine:Option<(&Curve,&Curve)>,
    qs:&[[[f64;2];3]],traversal:[f64;2],max_cells:usize,
)->Result<ControlValuesReport>{
    let frame=certify_frenet_path_values(path,twist,traversal,max_cells)?;
    super::trajectory::control_values_with_fit(path,scale,affine,qs,traversal,max_cells,frame,None)
}

/// On an original coordinate-plane path, Frenet binormal is constant wherever
/// the certified curvature cross component has a strict sign. This avoids
/// fourth derivatives without changing the original Frenet transport.
pub fn certify_frenet_planar_control_trajectories(
    path:&Curve,scale:&Curve,twist:&Curve,affine:Option<(&Curve,&Curve)>,
    qs:&[[[f64;2];3]],traversal:[f64;2],max_cells:usize,
)->Result<TrajectoriesReport>{
    super::super::validate_law(twist,false)?;
    let p=vector_certificate::certify_third_traversal(path,traversal,max_cells)?;
    let mut frame=Report {status:Status::Unresolved,cells:p.base.cells,longitudinal:None,transverse:None,
        binormal:None,single_span:false,reason:p.base.reason};
    if p.base.status==Status::Certified {
        frame.reason=Some("frenet-planar-binormal-unproved");
        let axis=(0..3).find(|&k|path.control_points.iter().all(|q|q[k]==path.control_points[0][k]));
        if let Some(axis)=axis {
            let curvature=cross(decode(p.base.first.unwrap())?,decode(p.base.second.unwrap())?)?[axis];
            let sign=if curvature.lo>0. {Some(1.)} else if curvature.hi<0. {Some(-1.)} else {None};
            if let Some(sign)=sign {
                let position=jet(path,&p.base)?;
                let [a,b]=path.domain();let width=I::point(b).sub(I::point(a))?;
                let jerk=mul(decode(p.third.unwrap())?,width.mul(width)?.mul(width)?)?;
                if let Some(t)=normalize(Jet {v:position.d,d:position.dd,dd:jerk})? {
                    let mut v=[I::point(0.);3];v[axis]=I::point(sign);
                    let b=Jet {v,d:[I::point(0.);3],dd:[I::point(0.);3]};
                    let n=cross_jet(b,t)?;
                    frame.status=Status::Certified;frame.reason=None;frame.single_span=p.base.single_span;
                    frame.longitudinal=Some(encode(t));frame.transverse=Some(encode(n));frame.binormal=Some(encode(b));
                }
            }
        }
    }
    let frame=if frame.status==Status::Certified {apply_twist(frame,twist,traversal,max_cells)?} else {frame};
    super::trajectory::control_trajectories_with_fit(path,scale,affine,qs,traversal,max_cells,frame,None)
}

#[test]
fn frenet_original_curvature_and_twist_values_include_authored_endpoints(){
    let path=Curve {degree:2,knots:vec![2.,2.,2.,5.,5.,5.],
        control_points:vec![vec![0.;3],vec![0.5,0.,0.],vec![1.,1.,0.]],weights:vec![1.;3],periodic:false};
    let twist=crate::primitives::line([0.;3],[0.25,0.,0.]).unwrap();
    for interval in [[0.,0.],[0.25,0.5],[1.,1.]] {
        let r=certify_frenet_path_values(&path,&twist,interval,1000).unwrap();
        assert_eq!(r.status,Status::Certified);
        for t in [interval[0],(interval[0]+interval[1])/2.,interval[1]] {
            let h=(1.+4.*t*t).sqrt();let angle=0.25*t;
            let n=[-2.*t/h*angle.cos(),angle.cos()/h,angle.sin()];
            for (bound,x) in r.transverse.unwrap().into_iter().zip(n){assert!(bound[0]<=x&&x<=bound[1]);}
        }
        let short=certify_frenet_path_values(&path,&twist,interval,r.cells-1).unwrap();
        assert_eq!(short.status,Status::Unresolved);assert!(short.transverse.is_none()&&short.longitudinal.is_none());
    }
    let straight=crate::primitives::line([0.;3],[0.,0.,10.]).unwrap();
    let r=certify_frenet_path_values(&straight,&twist,[0.,1.],1000).unwrap();
    assert_eq!(r.status,Status::Unresolved);assert!(r.transverse.is_none());
}

#[test]
fn frenet_interior_inflection_refuses_despite_regular_endpoint_curvature(){
    // C(t)=(t,(t-1/2)^3,0), curvature vanishes only at the interior inflection.
    let path=Curve {degree:3,knots:vec![0.,0.,0.,0.,1.,1.,1.,1.],
        control_points:vec![vec![0.,-0.125,0.],vec![1./3.,0.125,0.],vec![2./3.,-0.125,0.],vec![1.,0.125,0.]],
        weights:vec![1.;4],periodic:false};
    let zero=crate::sweeps::progressive_sweep::constant_vector_law([0.;3]).unwrap();
    for t in [0.,1.] {
        assert_eq!(certify_frenet_path_values(&path,&zero,[t,t],1000).unwrap().status,Status::Certified);
    }
    for interval in [[0.,1.],[0.5,0.5]] {
        let r=certify_frenet_path_values(&path,&zero,interval,1000).unwrap();
        assert_eq!(r.status,Status::Unresolved);
        assert!(r.longitudinal.is_none()&&r.transverse.is_none()&&r.binormal.is_none());
    }
}

#[test]
fn frenet_planar_control_jets_match_independent_product_derivatives(){
    let path=Curve {degree:2,knots:vec![2.,2.,2.,5.,5.,5.],
        control_points:vec![vec![0.;3],vec![0.5,0.,0.],vec![1.,1.,0.]],weights:vec![1.;3],periodic:false};
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let a=0.25;
    let twist=crate::primitives::line([0.;3],[a,0.,0.]).unwrap();
    let qs=[[[1.,1.],[1.,1.],[0.,0.]],[[2.,2.],[1.,1.],[0.,0.]]];
    let r=certify_frenet_planar_control_trajectories(&path,&scale,&twist,None,&qs,[0.25,0.5],1000).unwrap();
    assert_eq!(r.status,Status::Certified);assert!(r.single_span);
    for t in [0.25_f64,0.375,0.5] {
        let h=1.+4.*t*t;let c=(a*t).cos();let s=(a*t).sin();let l=1.+t;
        let n=[-2.*t/h.sqrt(),1./h.sqrt()];
        let nd=[-2./h.powf(1.5),-4.*t/h.powf(1.5)];
        let ndd=[24.*t/h.powf(2.5),(-4.+32.*t*t)/h.powf(2.5)];
        for (j,jet) in r.jets.as_ref().unwrap().iter().enumerate() {
            let q=(j+1) as f64;let k=l*(q*c-s);
            let kd=q*c-s+a*l*(-q*s-c);let kdd=2.*a*(-q*s-c)-a*a*l*(q*c-s);
            let z=l*(q*s+c);let zd=q*s+c+a*l*(q*c-s);let zdd=2.*a*(q*c-s)-a*a*l*(q*s+c);
            let expected=[[t+n[0]*k,t*t+n[1]*k,z],
                [1.+nd[0]*k+n[0]*kd,2.*t+nd[1]*k+n[1]*kd,zd],
                [ndd[0]*k+2.*nd[0]*kd+n[0]*kdd,2.+ndd[1]*k+2.*nd[1]*kd+n[1]*kdd,zdd]];
            for (bounds,v) in [jet.value,jet.first,jet.second].into_iter().zip(expected) {
                for i in 0..3 {assert!(bounds[i][0]<=v[i]&&v[i]<=bounds[i][1]);}
            }
        }
    }
    let short=certify_frenet_planar_control_trajectories(&path,&scale,&twist,None,&qs,[0.25,0.5],r.cells-1).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.jets.is_none());
    let mut nonplanar=path.clone();nonplanar.control_points[1][2]=f64::MIN_POSITIVE;
    let refused=certify_frenet_planar_control_trajectories(&nonplanar,&scale,&twist,None,&qs,[0.25,0.5],1000).unwrap();
    assert_eq!(refused.status,Status::Unresolved);assert!(refused.jets.is_none());
}

#[test]
fn spatial_frenet_axes_match_independent_polynomial_derivatives(){
    // C(t)=(t,t²,t³), with an independent source knot domain [2,5].
    let path=Curve {degree:3,knots:vec![2.,2.,2.,2.,5.,5.,5.,5.],
        control_points:vec![vec![0.;3],vec![1./3.,0.,0.],vec![2./3.,1./3.,0.],vec![1.,1.,1.]],weights:vec![1.;4],periodic:false};
    let zero=crate::sweeps::progressive_sweep::constant_vector_law([0.;3]).unwrap();
    let r=certify_frenet_path(&path,&zero,[0.25,0.5],1000).unwrap();
    assert_eq!(r.status,Status::Certified);assert!(r.single_span);
    for t in [0.25_f64,0.375,0.5] {
        for (jet,u,ud,udd,f,fd,fdd) in [
            (r.longitudinal.as_ref().unwrap(),[1.,2.*t,3.*t*t],[0.,2.,6.*t],[0.,0.,6.],1.+4.*t*t+9.*t.powi(4),8.*t+36.*t.powi(3),8.+108.*t*t),
            (r.binormal.as_ref().unwrap(),[3.*t*t,-3.*t,1.],[6.*t,-3.,0.],[6.,0.,0.],1.+9.*t*t+9.*t.powi(4),18.*t+36.*t.powi(3),18.+108.*t*t),
        ] {
            let value=std::array::from_fn::<_,3,_>(|k|u[k]/f.sqrt());
            let first=std::array::from_fn::<_,3,_>(|k|ud[k]/f.sqrt()-0.5*u[k]*fd/f.powf(1.5));
            let second=std::array::from_fn::<_,3,_>(|k|udd[k]/f.sqrt()-ud[k]*fd/f.powf(1.5)-0.5*u[k]*fdd/f.powf(1.5)+0.75*u[k]*fd*fd/f.powf(2.5));
            for (bound,x) in [jet.value,jet.first,jet.second].into_iter().zip([value,first,second]) {
                for k in 0..3 {assert!(bound[k][0]<=x[k]&&x[k]<=bound[k][1]);}
            }
        }
    }
    let short=certify_frenet_path(&path,&zero,[0.25,0.5],r.cells-1).unwrap();
    assert_eq!(short.status,Status::Unresolved);
    assert!(short.longitudinal.is_none()&&short.transverse.is_none()&&short.binormal.is_none());
}


#[cfg(test)]
mod whole_cover_tests {
    use super::*;
    #[test]
    fn whole_rational_closed_frenet_cover_includes_every_knot_side(){
        let path=crate::primitives::circle([0.;3],[0.,0.,1.],5.).unwrap();
        let zero=crate::sweeps::progressive_sweep::constant_vector_law([0.;3]).unwrap();
        let r=certify_frenet_cover(&path,&zero,10000).unwrap();
        assert_eq!(r.status,Status::Certified);
        let intervals=r.intervals.as_ref().unwrap();
        assert_eq!(intervals.first().unwrap().traversal[0],0.);
        assert_eq!(intervals.last().unwrap().traversal[1],1.);
        assert!(intervals.len()>1);
        for pair in intervals.windows(2){assert_eq!(pair[0].traversal[1],pair[1].traversal[0]);}
        for knot in [0.25,0.5,0.75]{
            assert!(intervals.iter().any(|i|i.traversal[0]<=knot&&i.traversal[1]>=knot));
        }
        let short=certify_frenet_cover(&path,&zero,r.cells-1).unwrap();
        assert_eq!(short.status,Status::Unresolved);assert!(short.intervals.is_none());
        // Frame regularity is not a claim that the retained closed seam is G1/G2.
    }
    #[test]
    fn whole_spatial_frenet_cover_uses_original_jets_and_discards_partial_work(){
        let path=Curve {degree:3,knots:vec![2.,2.,2.,2.,5.,5.,5.,5.],
            control_points:vec![vec![0.;3],vec![1./3.,0.,0.],vec![2./3.,1./3.,0.],vec![1.,1.,1.]],weights:vec![1.;4],periodic:false};
        let zero=Curve {degree:1,knots:vec![7.,7.,9.,9.],control_points:vec![vec![0.;3];2],weights:vec![1.;2],periodic:false};
        let whole=certify_frenet_cover(&path,&zero,10000).unwrap();
        assert_eq!(whole.status,Status::Certified);
        #[cfg(feature="transport")]
        {
            use value_codec::json;
            let r=crate::transport::dispatch(json!({"op":"sweep_frenet_frame_regularity","path":path,"twist":zero,"maxCells":10000})).unwrap();
            assert_eq!(r["regularityCertified"],true);assert_eq!(r["cells"],whole.cells);
            for flag in ["continuousBound","surfaceRegularityCertified","globalEmbeddingCertified"] {assert_eq!(r[flag],false);}
        }
        let intervals=whole.intervals.as_ref().unwrap();
        assert_eq!(intervals.first().unwrap().traversal[0],0.);
        assert_eq!(intervals.last().unwrap().traversal[1],1.);
        for pair in intervals.windows(2){assert_eq!(pair[0].traversal[1],pair[1].traversal[0]);}
        let short=certify_frenet_cover(&path,&zero,whole.cells-1).unwrap();
        assert_eq!(short.status,Status::Unresolved);assert!(short.intervals.is_none());assert!(short.cells<whole.cells);
        let empty=certify_frenet_cover(&path,&zero,0).unwrap();assert_eq!(empty.cells,0);assert!(empty.intervals.is_none());
        let inflection=Curve {degree:3,knots:vec![0.,0.,0.,0.,1.,1.,1.,1.],
            control_points:vec![vec![-1.,-1.,0.],vec![-1./3.,1.,0.],vec![1./3.,-1.,0.],vec![1.,1.,0.]],weights:vec![1.;4],periodic:false};
        let refused=certify_frenet_cover(&inflection,&zero,1000).unwrap();
        assert_eq!(refused.status,Status::Unresolved);assert!(refused.intervals.is_none());assert!(refused.cells<=1000);
    }
}

#[test]
fn frenet_relative_values_separate_original_path_and_station_laws(){
    let path=Curve {degree:2,knots:vec![2.,2.,2.,5.,5.,5.],
        control_points:vec![vec![0.;3],vec![0.5,0.,0.],vec![1.,1.,0.]],weights:vec![1.;3],periodic:false};
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let twist=crate::primitives::line([0.;3],[0.25,0.,0.]).unwrap();
    let qs=[[[1.,1.],[0.,0.],[0.,0.]]];
    let r=certify_frenet_relative_values(&path,&scale,&twist,None,&qs,[0.2,0.3],[0.6,0.7],1000).unwrap();
    assert_eq!(r.status,Status::Certified);let bounds=r.values.as_ref().unwrap()[0];
    for u in [0.2_f64,0.25,0.3]{for s in [0.6_f64,0.65,0.7]{
        let d=(1.+4.*u*u).sqrt();let a=0.25*s;
        let expected=[-(1.+s)*2.*u*a.cos()/d,(1.+s)*a.cos()/d,(1.+s)*a.sin()];
        for k in 0..3{assert!(bounds[k][0]<=expected[k]&&expected[k]<=bounds[k][1],"{bounds:?} vs {expected:?}");}
    }}
    let short=certify_frenet_relative_values(&path,&scale,&twist,None,&qs,[0.2,0.3],[0.6,0.7],r.cells-1).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.values.is_none());
}
