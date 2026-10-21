//! Arbitrary rational profiles carried in the normal plane of a helix.
//! Cubic Hermite transport with a continuous real-arithmetic error estimate;
//! binary64 trig/control rounding, collision and solid topology are separate.
use crate::{Result,check,numeric,curve::Curve,surface::Surface};
#[derive(Clone,Copy,Debug)]
pub struct Request {
    pub center:[f64;3], pub radius:f64, pub height:f64, pub turns:f64,
    pub phase_degrees:f64, pub tolerance:f64, pub max_spans:usize,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Stop { EstimateReached, SpanBudget }
#[derive(Clone,Debug)]
pub struct Report {
    pub patches:Vec<Surface>, pub spans:usize,
    /// Continuous transport remainder assuming exact real arithmetic samples.
    /// Does not include libm, coefficient or Surface::evaluate rounding.
    pub real_arithmetic_error_estimate:f64, pub stop:Stop,
}
/// The local X axis is radial. Local Y is tangent×radial, hence both axes
/// span the helix normal plane. Height/turns may be signed. Local 3D profiles
/// are admitted only when every control has Z=0; 2D profiles are also accepted.
pub fn approximate(profile:&Curve,r:Request)->Result<Report> {
    profile.validate()?;
    check(profile.control_points.len()<=32 && (profile.control_points[0].len()==2
        || profile.control_points.iter().all(|p|p.len()==3&&p[2]==0.)),
        "Spring profile requires 2D or local XY controls and at most 32 controls")?;
    check(r.center.iter().all(|v|v.is_finite()) && r.radius.is_finite()&&r.radius>0.
        && r.height.is_finite()&&r.turns.is_finite()&&r.turns!=0.
        && r.phase_degrees.is_finite()&&r.tolerance.is_finite()&&r.tolerance>0.
        && (1..=1024).contains(&r.max_spans),"Invalid spring dimensions, tolerance or span budget")?;
    let omega=std::f64::consts::TAU*r.turns;
    let radial_speed=r.radius*omega;let speed=radial_speed.hypot(r.height);
    numeric(omega.is_finite()&&speed.is_finite()&&speed>0.,"Spring tangent exceeds numerical range")?;
    let alpha=radial_speed/speed;let beta=r.height/speed;let phase=r.phase_degrees.to_radians();
    let extent=profile.control_points.iter().map(|p|(r.radius+p[0]).hypot(beta*p[1])).fold(0_f64,f64::max);
    let estimate=|n:usize| {
        let width=(0..n).map(|i|(((i+1) as f64/n as f64)-(i as f64/n as f64)).next_up()).fold(0_f64,f64::max);
        extent*(omega.abs()*width).powi(4)/384.
    };
    let mut n=1;while estimate(n)>r.tolerance && n<r.max_spans {n=(n*2).min(r.max_spans);}
    let ts:Vec<_>=(0..=n).map(|i|i as f64/n as f64).collect();
    let error=estimate(n);
    numeric(error.is_finite(),"Spring interpolation estimate overflow")?;
    let trajectory=|p:&Vec<f64>,t:f64| {
        let (s,c)=(phase+omega*t).sin_cos();let a=r.radius+p[0];let b=beta*p[1];
        ([r.center[0]+a*c-b*s,r.center[1]+a*s+b*c,r.center[2]+r.height*t-alpha*p[1]],
         [omega*(-a*s-b*c),omega*(a*c-b*s),r.height])
    };
    let mut patches=Vec::new();
    for start in (0..n).step_by(10) {
        let end=(start+10).min(n);
        let controls=profile.control_points.iter().map(|p| {
            let mut row=Vec::new();
            for i in start..end {
                let (a,da)=trajectory(p,ts[i]);let (b,db)=trajectory(p,ts[i+1]);let h=(ts[i+1]-ts[i])/3.;
                if i==start {row.push(a.to_vec());}
                row.push((0..3).map(|k|a[k]+h*da[k]).collect());
                row.push((0..3).map(|k|b[k]-h*db[k]).collect());row.push(b.to_vec());
            } row
        }).collect();
        let mut knots=vec![ts[start];4];for i in start+1..end {knots.extend([ts[i];3]);}knots.extend([ts[end];4]);
        let surface=Surface {degree_u:profile.degree,degree_v:3,knots_u:profile.knots.clone(),knots_v:knots,
            control_points:controls,weights:profile.weights.iter().map(|w|vec![*w;3*(end-start)+1]).collect(),
            periodic_u:profile.periodic,periodic_v:false};
        surface.validate()?;patches.push(surface);
    }
    Ok(Report {patches,spans:n,real_arithmetic_error_estimate:error,
        stop:if error<=r.tolerance {Stop::EstimateReached} else {Stop::SpanBudget}})
}
