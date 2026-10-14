//! Fixed frame derived from the original path's initial tangent, before twist.
//! This supplies original value enclosures, not a retained sweep certificate.
use super::*;

pub fn certify_fixed_path_values(
    path:&Curve, normal:[f64;3], twist:&Curve, traversal:[f64;2], max_cells:usize,
)->Result<ValuesReport>{
    check(max_cells<=100000,"Fixed frame budget exceeds100000 cells")?;
    check(normal.iter().all(|x|x.is_finite()),"Fixed frame normal must be finite")?;
    super::super::validate_law(twist,false)?;
    let source=vector_certificate::certify_endpoint_first(path,false,max_cells)?;
    let mut out=ValuesReport {status:Status::Unresolved,cells:source.cells,
        longitudinal:None,transverse:None,binormal:None};
    let Some(first)=source.first else {return Ok(out);};
    let constant=|v|Jet {v,d:[I::point(0.);3],dd:[I::point(0.);3]};
    let Some(t)=normalize(constant(decode(first)?))? else {return Ok(out);};
    let Some(b)=normalize(constant(cross(t.v,normal.map(I::point))?))? else {return Ok(out);};
    let n=cross(b.v,t.v)?;
    let charge=(twist.degree..twist.control_points.len())
        .filter(|&i|twist.knots[i]<twist.knots[i+1]).count();
    if charge>max_cells-out.cells {return Ok(out);}
    let theta=super::super::scalar_certificate::value_traversal(twist,traversal,charge)?;
    out.cells+=charge;
    let Some(theta)=theta else {return Ok(out);};
    let trig=super::super::trigonometric_certificate::certify(theta)?;
    let rotated=add(mul(n,I::new(trig.cos[0],trig.cos[1])?)?,
        mul(b.v,I::new(trig.sin[0],trig.sin[1])?)?)?;
    out.status=Status::Certified;
    out.longitudinal=Some(t.v.map(|x|[x.lo,x.hi]));
    out.transverse=Some(rotated.map(|x|[x.lo,x.hi]));
    out.binormal=Some(cross(t.v,rotated)?.map(|x|[x.lo,x.hi]));
    Ok(out)
}

pub fn certify_fixed_path_control_values(
    path:&Curve,normal:[f64;3],scale:&Curve,twist:&Curve,affine:Option<(&Curve,&Curve)>,
    qs:&[[[f64;2];3]],traversal:[f64;2],max_cells:usize,
)->Result<ControlValuesReport>{
    certify_fixed_path_control_values_with_frame_path(path,path,normal,scale,twist,affine,qs,traversal,max_cells)
}

/// Caller must prove that frame_path has the original initial tangent direction.
/// Translation remains evaluated on path, including its original rational basis.
pub(crate) fn certify_fixed_path_control_values_with_frame_path(
    path:&Curve,frame_path:&Curve,normal:[f64;3],scale:&Curve,twist:&Curve,affine:Option<(&Curve,&Curve)>,
    qs:&[[[f64;2];3]],traversal:[f64;2],max_cells:usize,
)->Result<ControlValuesReport>{
    let frame=certify_fixed_path_values(frame_path,normal,twist,traversal,max_cells)?;
    super::trajectory::control_values_with_fit(path,scale,affine,qs,traversal,max_cells,frame,None)
}

/// Relative pose in the original initial-tangent frame. Translation is owned
/// separately by the source arc-length certificate, not by a proxy path.
pub(crate) fn certify_fixed_relative_value(
    path:&Curve,normal:[f64;3],scale:&Curve,twist:&Curve,affine:Option<(&Curve,&Curve)>,
    q:[[f64;2];3],traversal:[f64;2],max_cells:usize,
)->Result<ControlValueReport>{
    let frame=certify_fixed_path_values(path,normal,twist,traversal,max_cells)?;
    let zero=crate::sweeps::progressive_sweep::constant_vector_law([0.;3])?;
    let batch=super::trajectory::control_values_with_fit(&zero,scale,affine,&[q],traversal,max_cells,frame,None)?;
    Ok(ControlValueReport {status:batch.status,cells:batch.cells,value:batch.values.map(|mut v|v.remove(0)),reason:batch.reason})
}
pub(crate) fn certify_fixed_relative_trajectory(
    path:&Curve,normal:[f64;3],scale:&Curve,twist:&Curve,affine:Option<(&Curve,&Curve)>,
    q:[[f64;2];3],traversal:[f64;2],max_cells:usize,
)->Result<TrajectoryReport>{
    let frame=certify_fixed_path(path,normal,twist,traversal,max_cells)?;
    let zero=crate::sweeps::progressive_sweep::constant_vector_law([0.;3])?;
    super::trajectory::control_trajectory_with_fit(&zero,scale,affine,q,traversal,max_cells,frame,None)
}

/// Original fixed basis is constant in traversal. Its uncertainty comes from
/// the initial source tangent; only original twist contributes frame derivatives.
pub fn certify_fixed_path(
    path:&Curve,normal:[f64;3],twist:&Curve,traversal:[f64;2],max_cells:usize,
)->Result<Report>{
    super::super::validate_law(twist,false)?;
    let zero=crate::sweeps::progressive_sweep::constant_vector_law([0.;3])?;
    let basis=certify_fixed_path_values(path,normal,&zero,[0.,0.],max_cells)?;
    let constant=|v:[[f64;2];3]|FrameJet {value:v,first:[[0.,0.];3],second:[[0.,0.];3]};
    let report=Report {status:basis.status,cells:basis.cells,
        longitudinal:basis.longitudinal.map(constant),
        transverse:basis.transverse.map(constant),binormal:basis.binormal.map(constant),
        single_span:basis.status==Status::Certified,
        reason:if basis.status==Status::Certified {None} else {Some("fixed-initial-frame-unresolved")}};
    apply_twist(report,twist,traversal,max_cells)
}

pub fn certify_fixed_path_control_trajectories(
    path:&Curve,normal:[f64;3],scale:&Curve,twist:&Curve,affine:Option<(&Curve,&Curve)>,
    qs:&[[[f64;2];3]],traversal:[f64;2],max_cells:usize,
)->Result<TrajectoriesReport>{
    certify_fixed_path_control_trajectories_with_frame_path(path,path,normal,scale,twist,affine,qs,traversal,max_cells)
}

pub(crate) fn certify_fixed_path_control_trajectories_with_frame_path(
    path:&Curve,frame_path:&Curve,normal:[f64;3],scale:&Curve,twist:&Curve,affine:Option<(&Curve,&Curve)>,
    qs:&[[[f64;2];3]],traversal:[f64;2],max_cells:usize,
)->Result<TrajectoriesReport>{
    let frame=certify_fixed_path(frame_path,normal,twist,traversal,max_cells)?;
    super::trajectory::control_trajectories_with_fit(path,scale,affine,qs,traversal,max_cells,frame,None)
}

#[test]
fn fixed_jets_enclose_independent_scale_twist_product_derivatives(){
    let path=crate::primitives::line([0.;3],[0.,0.,10.]).unwrap();
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let a=std::f64::consts::FRAC_PI_2;
    let twist=crate::primitives::line([0.;3],[a,0.,0.]).unwrap();
    let qs=[[[1.,1.],[0.,0.],[0.,0.]],[[2.,2.],[0.,0.],[0.,0.]]];
    for interval in [[0.25,0.5],[0.5,0.75]] {
        let r=certify_fixed_path_control_trajectories(&path,[1.,0.,0.],&scale,&twist,None,&qs,interval,1000).unwrap();
        assert_eq!(r.status,Status::Certified);assert!(r.single_span);
        for (i,jet) in r.jets.as_ref().unwrap().iter().enumerate(){
            let q=(i+1) as f64;
            for t in [interval[0],(interval[0]+interval[1])/2.,interval[1]] {
                let c=(a*t).cos();let s=(a*t).sin();let k=1.+t;
                let expected=[[q*k*c,q*k*s,10.*t],
                    [q*(c-a*k*s),q*(s+a*k*c),10.],
                    [q*(-2.*a*s-a*a*k*c),q*(2.*a*c-a*a*k*s),0.]];
                for (bound,value) in [jet.value,jet.first,jet.second].into_iter().zip(expected){
                    for k in 0..3 {assert!(bound[k][0]<=value[k]&&value[k]<=bound[k][1]);}
                }
            }
        }
        let short=certify_fixed_path_control_trajectories(&path,[1.,0.,0.],&scale,&twist,None,&qs,interval,r.cells-1).unwrap();
        assert_eq!(short.status,Status::Unresolved);assert!(short.jets.is_none());
    }
}

#[test]
fn fixed_original_frame_encloses_twisted_transport_and_refuses_missing_work(){
    let path=crate::primitives::line([0.;3],[0.,0.,10.]).unwrap();
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let twist=crate::primitives::line([0.;3],[std::f64::consts::FRAC_PI_2,0.,0.]).unwrap();
    let q=[[[1.,1.],[0.,0.],[0.,0.]]];
    for interval in [[0.,0.],[0.25,0.5],[1.,1.]] {
        let r=certify_fixed_path_control_values(&path,[1.,0.,0.],&scale,&twist,None,&q,interval,1000).unwrap();
        assert_eq!(r.status,Status::Certified);
        let value=r.values.unwrap()[0];
        for t in [interval[0],(interval[0]+interval[1])/2.,interval[1]] {
            let angle=t*std::f64::consts::FRAC_PI_2;
            let expected=[(1.+t)*angle.cos(),(1.+t)*angle.sin(),10.*t];
            for k in 0..3 {assert!(value[k][0]<=expected[k]&&expected[k]<=value[k][1]);}
        }
        let short=certify_fixed_path_control_values(&path,[1.,0.,0.],&scale,&twist,None,&q,interval,r.cells-1).unwrap();
        assert_eq!(short.status,Status::Unresolved);assert!(short.values.is_none());
    }
    let singular=certify_fixed_path_values(&path,[0.,0.,1.],&twist,[0.,1.],1000).unwrap();
    assert_eq!(singular.status,Status::Unresolved);assert!(singular.transverse.is_none());
}
