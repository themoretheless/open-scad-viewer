use super::*;
#[cfg(test)]
pub(super) fn check_nonaxial_planar_rmf_original_image(spacing:Spacing){
    // C(u)=(u,-u,u²), N=(1,1,0)/sqrt(2). The independent Bishop
    // binormal is (-sqrt(2)u,sqrt(2)u,sqrt(2))/sqrt(2+4u²).
    let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
        control_points:vec![vec![0.;3],vec![0.5,-0.5,0.],vec![1.,-1.,1.]],weights:vec![1.;3],periodic:false};
    let profile=crate::primitives::line([1.,1.,0.],[2.,2.,0.]).unwrap();
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let twist=crate::primitives::line([0.;3],[0.125,0.,0.]).unwrap();
    let axes=crate::primitives::line([1.,1.,1.],[2.,1.,1.]).unwrap();
    let center=crate::primitives::line([0.;3],[0.125,0.25,0.]).unwrap();
    let opts=Options {normal:[1.,1.,0.],orientation:Orientation::RotationMinimizing,
        spacing,
        initial_sections:3,max_sections:17,max_deviation:2.};
    assert!(!original_planar_rmf(&path,opts.normal));
    let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap().with_affine_laws(&axes,&center).unwrap();
    let proof=sweep.rmf_planar_patch_error_bound(17,10000,1000000).unwrap();
    assert_eq!(proof.status,Status::Certified,"{proof:?}");
    let upper=proof.error_upper.unwrap();assert!(upper.is_finite());
    eprintln!("nonaxial original-plane RMF upper={upper}, cells={}",proof.cells);
    assert!(proof.within_budget&&upper<2.);
    let level=sweep.preview_at(17).unwrap();assert!(level.report.continuous_bound&&level.report.accepted);
    let root=2_f64.sqrt();
    let length=|u:f64|u*(2.+4.*u*u).sqrt()/2.+(root*u).asinh()/2.;
    for patch in proof.patches.as_ref().unwrap(){for i in 0..=8{for j in 0..=8{
        let a=patch.knots_v[patch.degree_v];let b=patch.knots_v[patch.control_points[0].len()];
        let t=a+(b-a)*i as f64/8.;let q=j as f64/8.;let mut lo=0.;let mut hi=1.;
        for _ in 0..64{let mid=(lo+hi)/2.;if length(mid)<t*length(1.){lo=mid;}else{hi=mid;}}
        let u=if spacing==Spacing::Parameter{t}else{(lo+hi)/2.};let angle=0.125*t;
        let x=(1.+t).powi(2)*root*(1.+q)+t/8.;let y=t/4.;
        let n=x*angle.cos()-y*angle.sin();let side=x*angle.sin()+y*angle.cos();
        let d=(2.+4.*u*u).sqrt();
        let expected=[u+n/root-root*u*side/d,-u+n/root+root*u*side/d,u*u+root*side/d];
        let actual=patch.evaluate(q,t).unwrap().point;
        assert!(norm(std::array::from_fn(|k|actual[k]-expected[k]))<=upper);
    }}}
    let short=sweep.rmf_planar_patch_error_bound(17,proof.cells-1,1000000).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
    let mut off_plane=path.clone();off_plane.control_points[1][1]=(-0.5_f64).next_up();
    let wrong=Sweep::new(&profile,&off_plane,&scale,&twist,opts).unwrap().rmf_planar_patch_error_bound(17,10000,1000000).unwrap();
    assert_eq!(wrong.status,Status::Unresolved);assert!(wrong.error_upper.is_none());
    let mut closed=crate::primitives::circle([0.;3],[0.,0.,1.],1.).unwrap();
    for p in &mut closed.control_points{*p=vec![p[0],-p[0],p[1]];}
    let closed=Sweep::new(&profile,&closed,&scale,&twist,Options{normal:[1.,0.,1.],..opts}).unwrap().rmf_planar_patch_error_bound(17,10000,1000000).unwrap();
    assert_eq!(closed.status,Status::Unresolved);assert!(closed.error_upper.is_none());
}
#[cfg(test)]
pub(super) fn check_original_planar_frame_arc_length(orientation:Orientation){
    let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    // Exact original polynomial C(u)=(0,u²,u). Its length primitive is
    // F(u)=u*sqrt(1+4u²)/2+asinh(2u)/4; inversion below never calls kernel.
    let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
        control_points:vec![vec![0.;3],vec![0.,0.,0.5],vec![0.,1.,1.]],weights:vec![1.;3],periodic:false};
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let twist=crate::primitives::line([0.;3],[0.125,0.,0.]).unwrap();
    let axes=crate::primitives::line([1.,1.,1.],[2.,1.,1.]).unwrap();
    let center=crate::primitives::line([0.;3],[0.125,0.25,0.]).unwrap();
    let opts=Options {normal:[1.,0.,0.],orientation,
        spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
        initial_sections:3,max_sections:17,max_deviation:2.};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap().with_affine_laws(&axes,&center).unwrap();
    let certify=|cells|if orientation==Orientation::FixedNormal {sweep.fixed_normal_patch_error_bound(17,cells,1000000)}else{sweep.rmf_planar_patch_error_bound(17,cells,1000000)};
    let proof=certify(10000).unwrap();
    assert_eq!(proof.status,Status::Certified);let upper=proof.error_upper.unwrap();assert!(upper<2.,"{proof:?}");
    let level=sweep.preview_at(17).unwrap();assert!(level.report.continuous_bound&&level.report.accepted);
    let length=|u:f64|u*(1.+4.*u*u).sqrt()/2.+(2.*u).asinh()/4.;
    for patch in proof.patches.as_ref().unwrap(){for i in 0..=8{for j in 0..=8{
        let a=patch.knots_v[patch.degree_v];let b=patch.knots_v[patch.control_points[0].len()];
        let t=a+(b-a)*i as f64/8.;let q=j as f64/8.;let mut lo=0.;let mut hi=1.;
        for _ in 0..64{let mid=(lo+hi)/2.;if length(mid)<t*length(1.){lo=mid;}else{hi=mid;}}
        let u=(lo+hi)/2.;let angle=0.125*t;
        let x=(1.+t).powi(2)*(1.+q)+t/8.;let y=t/4.;
        let d=(1.+4.*u*u).sqrt();let side=x*angle.sin()+y*angle.cos();
        let expected=[x*angle.cos()-y*angle.sin(),u*u+side/d,u-2.*u*side/d];
        let actual=patch.evaluate(q,t).unwrap().point;
        assert!(norm(std::array::from_fn(|k|actual[k]-expected[k]))<=upper);
    }}}
    let short=certify(proof.cells-1).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
    if orientation==Orientation::RotationMinimizing{
        let mut off_plane=path.clone();off_plane.control_points[1][0]=f64::from_bits(1);
        let wrong=Sweep::new(&profile,&off_plane,&scale,&twist,opts).unwrap().rmf_planar_patch_error_bound(17,10000,1000000).unwrap();
        assert_eq!(wrong.status,Status::Unresolved);assert!(wrong.error_upper.is_none());
        assert!(matches!(wrong.reason,Some("source-plane-coefficients-nonzero"|"source-plane-exact-work-unproved")));
        let closed=crate::primitives::circle([0.;3],[0.,0.,1.],1.).unwrap();
        let one=constant_vector_law([1.,0.,0.]).unwrap();let zero=constant_vector_law([0.;3]).unwrap();
        let unresolved=Sweep::new(&profile,&closed,&one,&zero,Options {normal:[1.,0.,1.],initial_sections:5,..opts}).unwrap().rmf_planar_patch_error_bound(17,10000,1000000).unwrap();
        assert_eq!(unresolved.status,Status::Unresolved);assert!(unresolved.error_upper.is_none());
    }
}
