//! FixedNormal jets from original path derivatives and a constant seed normal.
//! This frame certificate alone does not certify retained surfaces or bodies.
use super::*;

/// Complete original-path frame jet cover, with one adaptive work budget.
/// Proves nonzero tangent and seed projection only, not surface regularity.
pub fn certify_fixed_normal_cover(
    path:&Curve,normal:[f64;3],twist:&Curve,max_cells:usize,
)->Result<RegularityReport>{
    check(max_cells<=100000,"FixedNormal cover work exceeds100000 cells")?;
    path.validate()?;
    check(path.control_points.iter().all(|p|p.len()==3),"FixedNormal path requires XYZ controls")?;
    check(normal.iter().all(|x|x.is_finite()),"FixedNormal seed must be finite")?;
    super::super::validate_law(twist,false)?;
    certify_frame_cover(max_cells,|interval,remaining|certify_fixed_normal_path(path,normal,twist,interval,remaining))
}

pub fn certify_fixed_normal_path_values(
    path:&Curve, normal:[f64;3], twist:&Curve, traversal:[f64;2], max_cells:usize,
) -> Result<ValuesReport> {
    fixed_normal_values_at(path,normal,twist,traversal,traversal,max_cells,false)
}
pub(super) fn fixed_normal_values_at(
    path:&Curve,normal:[f64;3],twist:&Curve,traversal:[f64;2],law_traversal:[f64;2],max_cells:usize,original_plane_proved:bool,
)->Result<ValuesReport>{
    // Normalize on a complete adaptive source cover: a regular tangent can
    // have a wide component box containing zero. Failed attempts consume the
    // same budget as successful leaves; no partial cover is returned.
    let mut out=ValuesReport {status:Status::Unresolved,cells:0,longitudinal:None,transverse:None,binormal:None};
    let incomplete=|mut report:ValuesReport| {
        report.longitudinal=None;report.transverse=None;report.binormal=None;
        report
    };
    let mut pending=vec![traversal];
    while let Some(cell)=pending.pop() {
        if out.cells==max_cells {return Ok(incomplete(out));}
        let leaf=fixed_normal_values_cell(path,normal,twist,cell,law_traversal,max_cells-out.cells,original_plane_proved)?;
        out.cells+=leaf.cells;
        if leaf.status!=Status::Certified {
            let mid=cell[0]+(cell[1]-cell[0])*0.5;
            if leaf.cells==0 || mid<=cell[0] || mid>=cell[1] {return Ok(incomplete(out));}
            pending.push([mid,cell[1]]);pending.push([cell[0],mid]);
            continue;
        }
        let merge=|target:&mut Option<[[f64;2];3]>,value:Option<[[f64;2];3]>| {
            let value=value.expect("Certified frame has all value fields");
            if let Some(bounds)=target {for axis in 0..3 {bounds[axis][0]=bounds[axis][0].min(value[axis][0]);bounds[axis][1]=bounds[axis][1].max(value[axis][1]);}}
            else {*target=Some(value);}
        };
        merge(&mut out.longitudinal,leaf.longitudinal);
        merge(&mut out.transverse,leaf.transverse);
        merge(&mut out.binormal,leaf.binormal);
    }
    out.status=Status::Certified;
    Ok(out)
}
fn fixed_normal_values_cell(
    path:&Curve,normal:[f64;3],twist:&Curve,traversal:[f64;2],law_traversal:[f64;2],max_cells:usize,original_plane_proved:bool,
)->Result<ValuesReport>{
    check(normal.iter().all(|x|x.is_finite()),"FixedNormal seed must be finite")?;
    super::super::validate_law(twist,false)?;
    let (first,cells)=if traversal[0]==traversal[1] {
        let r=if traversal[0]==0. || traversal[0]==1. {
            vector_certificate::certify_endpoint_first(path,traversal[0]==1.,max_cells)?
        } else {vector_certificate::certify_first_point(path,traversal[0],max_cells)?};
        (r.first,r.cells)
    } else {
        let r=vector_certificate::certify_traversal(path,traversal,max_cells,false)?;
        (if r.status==Status::Certified {r.first} else {None},r.cells)
    };
    let mut out=ValuesReport {status:Status::Unresolved,cells,longitudinal:None,transverse:None,binormal:None};
    let Some(first)=first else {return Ok(out);};
    let constant=|v|Jet {v,d:[I::point(0.);3],dd:[I::point(0.);3]};
    let Some(mut t)=normalize(constant(decode(first)?))? else {return Ok(out);};
    // A constant original rational coordinate has exactly zero derivative,
    // independently of its positive weights. An axial perpendicular seed is
    // then the exact transverse unit vector; B=T×N is already unit length.
    let axial=(0..3).find(|&axis|normal[axis]!=0.&&
        (0..3).all(|k|k==axis||normal[k]==0.)&&
        path.control_points.iter().all(|p|p[axis]==path.control_points[0][axis]));
    let (b,n)=if let Some(axis)=axial {
        t.v[axis]=I::point(0.);
        let mut n=[I::point(0.);3];n[axis]=I::point(normal[axis].signum());
        (constant(cross(t.v,n)?),n)
    }else if original_plane_proved {
        // The caller owns an exact original-coefficient plane premise.
        // Its constant unit seed is perpendicular to every source tangent;
        // B=T×N therefore needs no second interval normalization.
        let Some(n)=normalize(constant(normal.map(I::point)))? else{return Ok(out);};
        (constant(cross(t.v,n.v)?),n.v)
    }else{
        let Some(b)=normalize(constant(cross(t.v,normal.map(I::point))?))? else {return Ok(out);};
        let n=cross(b.v,t.v)?;(b,n)
    };
    let charge=(twist.degree..twist.control_points.len()).filter(|&i|twist.knots[i]<twist.knots[i+1]).count();
    if charge>max_cells-out.cells {return Ok(out);}
    let theta=super::super::scalar_certificate::value_traversal(twist,law_traversal,charge)?;
    out.cells+=charge;
    let Some(theta)=theta else {return Ok(out);};
    let trig=super::super::trigonometric_certificate::certify(theta)?;
    let cosine=I::new(trig.cos[0],trig.cos[1])?;let sine=I::new(trig.sin[0],trig.sin[1])?;
    // Rotate the original orthonormal N/B pair directly, preserving its
    // algebraic correlation rather than crossing T with an interval box.
    let binormal=sub(mul(b.v,cosine)?,mul(n,sine)?)?;
    let n=add(mul(n,cosine)?,mul(b.v,sine)?)?;
    out.longitudinal=Some(t.v.map(|x|[x.lo,x.hi]));
    out.transverse=Some(n.map(|x|[x.lo,x.hi]));
    out.binormal=Some(binormal.map(|x|[x.lo,x.hi]));
    out.status=Status::Certified;
    Ok(out)
}

#[test]
fn adaptive_values_cover_regular_turning_tangent_and_reject_partial_work() {
    let twist=crate::sweeps::progressive_sweep::constant_vector_law([0.;3]).unwrap();
    for (points,weights) in [
        (vec![vec![0.,0.,0.],vec![1.,1.,0.],vec![2.,-1.,0.],vec![3.,0.,0.]],vec![1.,2.,2.,1.]),
        (vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![1.,1.,0.],vec![2.,0.,0.],vec![1.,0.,0.]],vec![1.;5]),
    ] {
        let degree=points.len()-1;
        let path=Curve {degree,knots:[vec![0.;degree+1],vec![1.;degree+1]].concat(),control_points:points,weights,periodic:false};
        let report=certify_fixed_normal_path_values(&path,[0.,0.,1.],&twist,[0.,1.],10000).unwrap();
        assert_eq!(report.status,Status::Certified);
        assert!(report.cells>0);
        assert_eq!(certify_fixed_normal_path_values(&path,[0.,0.,1.],&twist,[0.,1.],0).unwrap().status,Status::Unresolved);
        assert_eq!(certify_fixed_normal_path_values(&path,[0.,0.,1.],&twist,[0.,1.],report.cells-1).unwrap().status,Status::Unresolved);
    }
}

/// Original path frame and authored laws have independent traversal domains
/// after length inversion. Only relative pose is returned; translation belongs
/// to the original source length certificate.
pub(crate) fn certify_fixed_normal_relative_values(
    path:&Curve,normal:[f64;3],scale:&Curve,twist:&Curve,affine:Option<(&Curve,&Curve)>,
    qs:&[[[f64;2];3]],path_traversal:[f64;2],law_traversal:[f64;2],max_cells:usize,
)->Result<ControlValuesReport>{
    let frame=fixed_normal_values_at(path,normal,twist,path_traversal,law_traversal,max_cells,false)?;
    let zero=crate::sweeps::progressive_sweep::constant_vector_law([0.;3])?;
    super::trajectory::control_values_with_fit(&zero,scale,affine,qs,law_traversal,max_cells,frame,None)
}

/// Requires a charged exact original-coefficient plane proof in the caller.
/// Only the planar RMF source theorem may use this correlated value field.
pub(crate) fn certify_proved_planar_relative_values(
    path:&Curve,normal:[f64;3],scale:&Curve,twist:&Curve,affine:Option<(&Curve,&Curve)>,
    qs:&[[[f64;2];3]],path_traversal:[f64;2],law_traversal:[f64;2],max_cells:usize,
)->Result<ControlValuesReport>{
    let frame=fixed_normal_values_at(path,normal,twist,path_traversal,law_traversal,max_cells,true)?;
    let zero=crate::sweeps::progressive_sweep::constant_vector_law([0.;3])?;
    super::trajectory::control_values_with_fit(&zero,scale,affine,qs,law_traversal,max_cells,frame,None)
}

/// Full original position/pose field; the caller must own the exact plane
/// premise and its work, just as for the independent-length relative field.
pub(crate) fn certify_proved_planar_control_values(
    path:&Curve,normal:[f64;3],scale:&Curve,twist:&Curve,affine:Option<(&Curve,&Curve)>,
    qs:&[[[f64;2];3]],traversal:[f64;2],max_cells:usize,
)->Result<ControlValuesReport>{
    let frame=fixed_normal_values_at(path,normal,twist,traversal,traversal,max_cells,true)?;
    super::trajectory::control_values_with_fit(path,scale,affine,qs,traversal,max_cells,frame,None)
}

pub fn certify_fixed_normal_control_values(
    path:&Curve,normal:[f64;3],scale:&Curve,twist:&Curve,affine:Option<(&Curve,&Curve)>,
    qs:&[[[f64;2];3]],traversal:[f64;2],max_cells:usize,
)->Result<ControlValuesReport>{
    let frame=certify_fixed_normal_path_values(path,normal,twist,traversal,max_cells)?;
    super::trajectory::control_values_with_fit(path,scale,affine,qs,traversal,max_cells,frame,None)
}

pub fn certify_fixed_normal_control_trajectories(
    path:&Curve,normal:[f64;3],scale:&Curve,twist:&Curve,affine:Option<(&Curve,&Curve)>,
    qs:&[[[f64;2];3]],traversal:[f64;2],max_cells:usize,
)->Result<TrajectoriesReport>{
    let frame=certify_fixed_normal_path(path,normal,twist,traversal,max_cells)?;
    super::trajectory::control_trajectories_with_fit(path,scale,affine,qs,traversal,max_cells,frame,None)
}

pub fn certify_fixed_normal_path(
    path: &Curve, normal: [f64; 3], twist: &Curve,
    traversal: [f64; 2], max_cells: usize,
) -> Result<Report> {
    check(normal.iter().all(|x| x.is_finite()), "FixedNormal seed must be finite")?;
    super::super::validate_law(twist, false)?;
    let p = vector_certificate::certify_third_traversal(path, traversal, max_cells)?;
    let mut out = Report {
        status: Status::Unresolved, cells: p.base.cells,
        longitudinal: None, transverse: None, binormal: None,
        single_span: false, reason: p.base.reason,
    };
    if p.base.status != Status::Certified { return Ok(out); }
    let position = jet(path, &p.base)?;
    let [a,b] = path.domain();
    let width = I::point(b).sub(I::point(a))?;
    let jerk = mul(decode(p.third.unwrap())?, width.mul(width)?.mul(width)?)?;
    out.reason = Some("fixed-normal-path-tangent-unresolved");
    let Some(mut t) = normalize(Jet { v: position.d, d: position.dd, dd: jerk })?
        else { return Ok(out); };
    let planar_axis=(0..3).find(|&axis|normal[axis]!=0.
        && (0..3).all(|k|k==axis||normal[k]==0.)
        && path.control_points.iter().all(|p|p[axis]==path.control_points[0][axis]));
    let (b,n)=if let Some(axis)=planar_axis {
        // Positive rational weights preserve this exact source plane.
        // Every one-sided tangent lies in it, so the projected unit seed
        // is constant, with zero first/second derivatives. This source
        // identity removes interval dependency, not actual frame motion.
        t.v[axis]=I::point(0.);t.d[axis]=I::point(0.);t.dd[axis]=I::point(0.);
        let mut unit=[I::point(0.);3];
        unit[axis]=I::point(if normal[axis]>0. {1.} else {-1.});
        let n=Jet {v:unit,d:[I::point(0.);3],dd:[I::point(0.);3]};
        (cross_jet(t,n)?,n)
    }else {
        let seed = Jet { v: normal.map(I::point), d: [I::point(0.);3], dd: [I::point(0.);3] };
        out.reason = Some("fixed-normal-projection-unresolved");
        let Some(b) = normalize(cross_jet(t, seed)?)? else { return Ok(out); };
        (b,cross_jet(b,t)?)
    };
    out.longitudinal = Some(encode(t));
    out.transverse = Some(encode(n));
    out.binormal = Some(encode(b));
    out.single_span = p.base.single_span;
    out.reason = None;
    out.status = Status::Certified;
    apply_twist(out, twist, traversal, max_cells)
}

#[test]
fn curved_fixed_normal_jets_match_analytic_original_path_and_refuse_partial_work() {
    // C(t)=(t,t²,0), on an independent authored domain [2,5].
    let path = Curve { degree:2, knots:vec![2.,2.,2.,5.,5.,5.],
        control_points:vec![vec![0.,0.,0.],vec![0.5,0.,0.],vec![1.,1.,0.]], weights:vec![1.;3], periodic:false };
    let twist = crate::sweeps::progressive_sweep::constant_vector_law([0.;3]).unwrap();
    let r = certify_fixed_normal_path(&path,[0.,0.,1.],&twist,[0.25,0.5],1000).unwrap();
    assert_eq!(r.status,Status::Certified); assert!(r.single_span);
    // Value endpoints use original derivative restrictions, including both
    // authored endpoints, rather than zero-width derivative division.
    for t in [0.,0.375,1.] {
        let v=certify_fixed_normal_path_values(&path,[0.,0.,1.],&twist,[t,t],1000).unwrap();
        assert_eq!(v.status,Status::Certified);
        let h=(1.+4.*t*t).sqrt();
        for (bounds,x) in v.longitudinal.unwrap().into_iter().zip([1./h,2.*t/h,0.]) {
            assert!(bounds[0]<=x && x<=bounds[1]);
        }
        let short=certify_fixed_normal_path_values(&path,[0.,0.,1.],&twist,[t,t],v.cells-1).unwrap();
        assert_eq!(short.status,Status::Unresolved);
        assert!(short.longitudinal.is_none() && short.transverse.is_none());
    }
    let scale=crate::sweeps::progressive_sweep::constant_vector_law([1.,0.,0.]).unwrap();
    let qs=[[[1.,1.],[0.,0.],[0.,0.]],[[2.,2.],[0.,0.],[0.,0.]]];
    let batch=certify_fixed_normal_control_values(&path,[0.,0.,1.],&scale,&twist,None,&qs,[0.25,0.5],1000).unwrap();
    assert_eq!(batch.status,Status::Certified);
    let short=certify_fixed_normal_control_values(&path,[0.,0.,1.],&scale,&twist,None,&qs,[0.25,0.5],batch.cells-1).unwrap();
    assert_eq!(short.status,Status::Unresolved);
    for t in [0.25_f64,0.375,0.5] {
        let h=1.+4.*t*t;
        let values=[1./h.sqrt(),2.*t/h.sqrt(),0.];
        let first=[-4.*t/h.powf(1.5),2./h.powf(1.5),0.];
        let second=[(-4.+32.*t*t)/h.powf(2.5),-24.*t/h.powf(2.5),0.];
        let axis=r.longitudinal.as_ref().unwrap();
        for (bounds,expected) in [axis.value,axis.first,axis.second].into_iter().zip([values,first,second]) {
            for k in 0..3 { assert!(bounds[k][0]<=expected[k] && expected[k]<=bounds[k][1]); }
        }
        let n=r.transverse.as_ref().unwrap();
        assert!(n.value[2][0]<=1. && n.value[2][1]>=1.);
        for bounds in [n.first,n.second] { for k in 0..3 {assert!(bounds[k][0]<=0. && bounds[k][1]>=0.);} }
    }
    let short=certify_fixed_normal_path(&path,[0.,0.,1.],&twist,[0.25,0.5],r.cells-1).unwrap();
    assert_eq!(short.status,Status::Unresolved);
    assert!(short.longitudinal.is_none() && short.transverse.is_none());
    let line=crate::primitives::line([0.;3],[0.,0.,10.]).unwrap();
    let singular=certify_fixed_normal_path(&line,[0.,0.,1.],&twist,[0.,1.],1000).unwrap();
    assert_eq!(singular.status,Status::Unresolved);
    assert!(singular.longitudinal.is_none() && singular.transverse.is_none());
}

#[test]
fn fixed_normal_interior_projection_singularity_is_not_hidden_by_regular_endpoints(){
    // C(t)=(t²-t,0,t): tangent is parallel to seed only at t=1/2.
    let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
        control_points:vec![vec![0.;3],vec![-0.5,0.,0.5],vec![0.,0.,1.]],
        weights:vec![1.;3],periodic:false};
    let zero=crate::sweeps::progressive_sweep::constant_vector_law([0.;3]).unwrap();
    for t in [0.,1.] {
        let r=certify_fixed_normal_path_values(&path,[0.,0.,1.],&zero,[t,t],1000).unwrap();
        assert_eq!(r.status,Status::Certified);
    }
    for interval in [[0.,1.],[0.5,0.5]] {
        let r=certify_fixed_normal_path_values(&path,[0.,0.,1.],&zero,interval,1000).unwrap();
        assert_eq!(r.status,Status::Unresolved);
        assert!(r.longitudinal.is_none()&&r.transverse.is_none()&&r.binormal.is_none());
    }
    let r=certify_fixed_normal_path(&path,[0.,0.,1.],&zero,[0.,1.],1000).unwrap();
    assert_eq!(r.status,Status::Unresolved);
    assert_eq!(r.reason,Some("fixed-normal-projection-unresolved"));
    assert!(r.longitudinal.is_none()&&r.transverse.is_none()&&r.binormal.is_none());
}

#[test]
fn fixed_normal_complete_cover_subdivides_original_circle_and_refuses_partial_cover(){
    let path=crate::primitives::circle([0.;3],[0.,0.,1.],5.).unwrap();
    let zero=crate::sweeps::progressive_sweep::constant_vector_law([0.;3]).unwrap();
    let whole=certify_fixed_normal_cover(&path,[0.,0.,1.],&zero,10000).unwrap();
    assert_eq!(whole.status,Status::Certified);
    let intervals=whole.intervals.as_ref().unwrap();
    assert!(intervals.len()>1);
    assert_eq!(intervals.first().unwrap().traversal[0],0.);
    assert_eq!(intervals.last().unwrap().traversal[1],1.);
    for pair in intervals.windows(2){assert_eq!(pair[0].traversal[1],pair[1].traversal[0]);}
    for interval in intervals {
        let t=(interval.traversal[0]+interval.traversal[1])/2.;
        let p=path.evaluate(t).unwrap().point;
        for k in 0..2 {let expected=p[k]/5.;let bound=interval.binormal.value[k];assert!(bound[0]<=expected&&expected<=bound[1]);}
    }
    let short=certify_fixed_normal_cover(&path,[0.,0.,1.],&zero,whole.cells-1).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.intervals.is_none());
    let empty=certify_fixed_normal_cover(&path,[0.,0.,1.],&zero,0).unwrap();
    assert_eq!(empty.status,Status::Unresolved);assert_eq!(empty.cells,0);assert!(empty.intervals.is_none());
    let singular=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
        control_points:vec![vec![0.;3],vec![-0.5,0.,0.5],vec![0.,0.,1.]],weights:vec![1.;3],periodic:false};
    let refused=certify_fixed_normal_cover(&singular,[0.,0.,1.],&zero,1000).unwrap();
    assert_eq!(refused.status,Status::Unresolved);assert!(refused.intervals.is_none());
    assert!(refused.cells<=1000);
    #[cfg(feature="transport")]
    {
        use value_codec::json;
        let r=crate::transport::dispatch(json!({"op":"sweep_fixed_normal_frame_regularity",
            "path":path,"normal":[0.,0.,1.],"twist":zero,"maxCells":10000})).unwrap();
        assert_eq!(r["regularityCertified"],true);
        assert_eq!(r["cells"],whole.cells);
        assert_eq!(r["certifiedIntervals"],whole.certified_intervals);
        for flag in ["continuousBound","surfaceRegularityCertified","globalEmbeddingCertified"] {assert_eq!(r[flag],false);}
    }
}

#[test]
fn closed_circle_knot_contact_blocks_single_span_remainder_without_endpoint_jet_proof(){
    let path=crate::primitives::circle([0.;3],[0.,0.,1.],5.).unwrap();
    let mut twist=crate::sweeps::progressive_sweep::constant_vector_law([0.;3]).unwrap();
    twist.control_points[1][0]=std::f64::consts::TAU;
    let aligned=certify_fixed_normal_path(&path,[0.,0.,1.],&twist,[0.25,0.25+1./256.],10000).unwrap();
    let interior=certify_fixed_normal_path(&path,[0.,0.,1.],&twist,[0.25+1./1024.,0.25+1./256.],10000).unwrap();
    eprintln!("aligned status={:?} single_span={} reason={:?} cells={}; interior status={:?} single_span={} cells={}",aligned.status,aligned.single_span,aligned.reason,aligned.cells,interior.status,interior.single_span,interior.cells);
    assert!(aligned.status!=Status::Certified||!aligned.single_span);
    assert_eq!(interior.status,Status::Certified);assert!(interior.single_span);
}

#[test]
fn original_coordinate_plane_has_constant_projected_normal_jets() {
    let twist=crate::sweeps::progressive_sweep::constant_vector_law([0.;3]).unwrap();
    for axis in 0..3 {for sign in [-1.,1.] {
        let mut points=vec![vec![0.;3];3];
        for (i,p) in points.iter_mut().enumerate() {
            p[axis]=17.;p[(axis+1)%3]=i as f64*0.5;
            p[(axis+2)%3]=if i==2 {1.}else {0.};
        }
        let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:points,weights:vec![1.,2.,1.],periodic:false};
        let mut normal=[0.;3];normal[axis]=sign*3.;
        let report=certify_fixed_normal_path(&path,normal,&twist,[0.25,0.5],1000).unwrap();
        assert_eq!(report.status,Status::Certified);
        let n=report.transverse.unwrap();
        for k in 0..3 {
            let expected=if k==axis {sign}else {0.};
            assert!(n.value[k][0]<=expected&&expected<=n.value[k][1]);
            assert!(n.first[k][0]<=0.&&n.first[k][1]>=0.);
            assert!(n.second[k][0]<=0.&&n.second[k][1]>=0.);
            assert!(n.first[k][1]-n.first[k][0]<1e-10);
            assert!(n.second[k][1]-n.second[k][0]<1e-10);
        }
    }}
}

#[test]
fn relative_values_separate_original_path_parameter_from_length_station_laws(){
    let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![vec![0.;3],vec![0.,0.,0.5],vec![0.,1.,1.]],weights:vec![1.;3],periodic:false};
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let twist=crate::primitives::line([0.;3],[0.25,0.,0.]).unwrap();
    let qs=[[[1.,1.],[0.,0.],[0.,0.]]];
    let r=certify_fixed_normal_relative_values(&path,[1.,0.,0.],&scale,&twist,None,&qs,[0.2,0.3],[0.6,0.7],1000).unwrap();
    assert_eq!(r.status,Status::Certified);let bounds=r.values.as_ref().unwrap()[0];
    for u in [0.2_f64,0.25,0.3]{for s in [0.6_f64,0.65,0.7]{
        let a=0.25*s;let d=(1.+4.*u*u).sqrt();
        let expected=[(1.+s)*a.cos(),(1.+s)*a.sin()/d,-2.*u*(1.+s)*a.sin()/d];
        for k in 0..3{assert!(bounds[k][0]<=expected[k]&&expected[k]<=bounds[k][1],"{bounds:?} vs {expected:?}");}
    }}
    let short=certify_fixed_normal_relative_values(&path,[1.,0.,0.],&scale,&twist,None,&qs,[0.2,0.3],[0.6,0.7],r.cells-1).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.values.is_none());
}

#[test]
fn axial_value_premise_preserves_seed_sign_and_falls_back_for_nonplanar_source(){
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let twist=crate::primitives::line([0.;3],[0.25,0.,0.]).unwrap();
    let qs=[[[1.,1.],[0.,0.],[0.,0.]]];
    for x in [0_f64,0.25]{for sign in [-1_f64,1.]{
        let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![vec![0.;3],vec![0.,0.,0.5],vec![x,1.,1.]],weights:vec![1.;3],periodic:false};
        let r=certify_fixed_normal_relative_values(&path,[sign,0.,0.],&scale,&twist,None,&qs,[0.2,0.3],[0.6,0.7],1000).unwrap();
        assert_eq!(r.status,Status::Certified);let bounds=r.values.unwrap()[0];
        for u in [0.2_f64,0.25,0.3]{for s in [0.6_f64,0.65,0.7]{
            let d=(4.*x*x*u*u+4.*u*u+1.).sqrt();let t=[2.*x*u/d,2.*u/d,1./d];
            let raw=[1.-t[0]*t[0],-t[0]*t[1],-t[0]*t[2]];
            let len=raw.iter().map(|v|v*v).sum::<f64>().sqrt();let n=raw.map(|v|sign*v/len);
            let b=[t[1]*n[2]-t[2]*n[1],t[2]*n[0]-t[0]*n[2],t[0]*n[1]-t[1]*n[0]];
            for k in 0..3{let expected=(1.+s)*(n[k]*(0.25*s).cos()+b[k]*(0.25*s).sin());
                assert!(bounds[k][0]<=expected&&expected<=bounds[k][1],"{bounds:?} vs {expected}");}
        }}
    }}
}
