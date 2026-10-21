//! Closed radial cam contours with knife-edge/roller follower geometry.
//! Adaptive cubic Hermite approximation; diagnostics are sampled, not
//! continuous manufacturing, undercut-free or global topology certificates.
use crate::{Result,check,numeric,curve::Curve};
#[derive(Clone,Copy,Debug)]
pub enum Law { Dwell, Harmonic, Cycloidal, Polynomial345, Polynomial4567 }
#[derive(Clone,Copy,Debug)]
pub struct Motion { pub end_degrees:f64, pub end_lift:f64, pub law:Law }
#[derive(Clone,Copy,Debug)]
pub struct Request {
    pub center:[f64;3], pub base_radius:f64, pub roller_radius:f64,
    pub phase_degrees:f64, pub tolerance:f64, pub max_spans:usize,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Stop { SampleEstimateReached, SpanBudget, ParameterPrecision }
#[derive(Clone,Debug)]
pub struct Cell { pub angle:[f64;2], pub sampled_error:f64 }
#[derive(Clone,Debug)]
pub struct Report {
    pub curves:Vec<Curve>, pub cells:Vec<Cell>, pub stop:Stop,
    pub max_sampled_error:f64, pub max_sampled_pressure_angle:f64,
    pub sampled_undercut:bool,
}
#[derive(Clone,Copy)]
struct Phase {a:f64,b:f64,from:f64,to:f64,law:Law}
#[derive(Clone,Copy)]
struct Sample {p:[f64;3],d:[f64;3],pressure:f64,undercut:bool}
struct Piece {cell:Cell,p:[[f64;3];4]}
fn law(l:Law,t:f64)->(f64,f64,f64) {
    if matches!(l,Law::Dwell) {return (0.,0.,0.);}
    let pi=std::f64::consts::PI;
    if t==0.||t==1. {return (t,0.,if matches!(l,Law::Harmonic) {pi*pi*0.5*(1.-2.*t)} else {0.});}
    match l {
        Law::Dwell=>unreachable!(),
        Law::Harmonic=>((1.-(pi*t).cos())*0.5,pi*0.5*(pi*t).sin(),pi*pi*0.5*(pi*t).cos()),
        Law::Cycloidal=>{let a=std::f64::consts::TAU*t;(t-a.sin()/std::f64::consts::TAU,1.-a.cos(),std::f64::consts::TAU*a.sin())},
        Law::Polynomial345=>(t.powi(3)*(10.-15.*t+6.*t*t),30.*t*t*(1.-t).powi(2),60.*t*(1.-t)*(1.-2.*t)),
        Law::Polynomial4567=>(t.powi(4)*(35.-84.*t+70.*t*t-20.*t.powi(3)),140.*t.powi(3)*(1.-t).powi(3),420.*t*t*(1.-t).powi(2)*(1.-2.*t)),
    }
}
fn at(r:Request,s:Phase,t:f64)->Result<Sample> {
    let w=s.b-s.a;let theta=if t==0. {s.a} else if t==1. {s.b} else {s.a+w*t};
    let (h,dh,ddh)=law(s.law,t);let delta=s.to-s.from;
    let lift=if t==0. {s.from} else if t==1. {s.to} else {s.from+delta*h};
    let radius=r.base_radius+r.roller_radius+lift;
    let rp=delta*dh/w;let rpp=(delta*ddh/w)/w;let speed=radius.hypot(rp);
    numeric(radius.is_finite()&&rp.is_finite()&&rpp.is_finite()&&speed.is_finite()&&speed>0.,"Cam motion exceeded numeric range")?;
    let a=radius/speed;let b=rp/speed;
    let curvature=(a*a+2.*b*b-a*(rpp/speed))/speed;
    let factor=1.-r.roller_radius*curvature;
    let (sin,cos)=(theta+r.phase_degrees.rem_euclid(360.).to_radians()).sin_cos();
    let n=[a*cos+b*sin,a*sin-b*cos];
    let p=[r.center[0]+radius*cos-r.roller_radius*n[0],r.center[1]+radius*sin-r.roller_radius*n[1],r.center[2]];
    let d=[factor*(rp*cos-radius*sin),factor*(rp*sin+radius*cos),0.];
    numeric(p.iter().chain(&d).all(|v|v.is_finite())&&curvature.is_finite(),"Cam offset exceeded numeric range")?;
    Ok(Sample {p,d,pressure:rp.atan2(radius).abs(),undercut:factor<=0.})
}
fn bezier(p:[[f64;3];4],t:f64)->[f64;3] {
    let mut p=p;for n in (1..4).rev() {for i in 0..n {for k in 0..3 {p[i][k]=p[i][k]*(1.-t)+p[i+1][k]*t;}}}p[0]
}
/// Motions cover a full 360-degree cycle, starting/ending at zero lift.
/// Dwell cannot change lift. max_spans must cover all initial motions.
/// Returned clamped curve chunks have shared endpoints and exact data closure;
/// radius/pressure/undercut and error claims remain sampled.
pub fn profile(r:Request,motions:&[Motion])->Result<Report> {
    check(r.center.iter().all(|v|v.is_finite())&&r.base_radius.is_finite()&&r.base_radius>0.
        &&r.roller_radius.is_finite()&&r.roller_radius>=0.&&r.phase_degrees.is_finite()
        &&r.tolerance.is_finite()&&r.tolerance>0.&&(1..=4096).contains(&r.max_spans)
        &&(1..=64).contains(&motions.len())&&r.max_spans>=motions.len(),"Invalid cam dimensions, tolerance, motions or budget")?;
    let mut phases=Vec::new();let (mut angle,mut lift)=(0_f64,0_f64);
    for m in motions {
        check(m.end_degrees.is_finite()&&m.end_degrees>angle&&m.end_degrees<=360.
            &&m.end_lift.is_finite()&&m.end_lift>=0.&&(!matches!(m.law,Law::Dwell)||m.end_lift==lift),"Invalid cam motion interval or dwell lift")?;
        let a=angle.to_radians();let b=if m.end_degrees==360. {std::f64::consts::TAU} else {m.end_degrees.to_radians()};
        numeric(b>a,"Cam angle interval collapsed")?;
        phases.push(Phase {a,b,from:lift,to:m.end_lift,law:m.law});angle=m.end_degrees;lift=m.end_lift;
    }
    check(angle==360.&&lift==0.,"Cam program must close at 360 degrees with zero lift")?;
    let first=at(r,phases[0],0.)?.p;
    let mut pending:Vec<_>=(0..phases.len()).rev().map(|i|(i,0.,1.)).collect();
    let mut leaves=phases.len();let mut pieces=Vec::new();let mut stop=Stop::SampleEstimateReached;
    let mut pressure=0_f64;let mut undercut=false;
    while let Some((i,lo,hi))=pending.pop() {
        let s=phases[i];let a=at(r,s,lo)?;let mut b=at(r,s,hi)?;
        if i==phases.len()-1&&hi==1. {b.p=first;}
        pressure=pressure.max(a.pressure).max(b.pressure);undercut|=a.undercut||b.undercut;
        let theta=|t:f64|if t==0. {s.a} else if t==1. {s.b} else {s.a+(s.b-s.a)*t};
        let bounds=[theta(lo),theta(hi)];let h=(bounds[1]-bounds[0])/3.;
        let p=[a.p,std::array::from_fn(|k|a.p[k]+h*a.d[k]),std::array::from_fn(|k|b.p[k]-h*b.d[k]),b.p];
        let mut error=0_f64;
        for q in [0.25,0.5,0.75] {
            let ideal=at(r,s,lo+(hi-lo)*q)?;pressure=pressure.max(ideal.pressure);undercut|=ideal.undercut;
            let actual=bezier(p,q);error=error.max((actual[0]-ideal.p[0]).hypot(actual[1]-ideal.p[1]).hypot(actual[2]-ideal.p[2]));
        }
        let mid=lo*0.5+hi*0.5;
        let splittable=lo<mid&&mid<hi&&bounds[0]<theta(mid)&&theta(mid)<bounds[1];
        if error>r.tolerance &&leaves<r.max_spans &&splittable {leaves+=1;pending.push((i,mid,hi));pending.push((i,lo,mid));continue;}
        if error>r.tolerance {
            if !splittable {stop=Stop::ParameterPrecision;} else if stop!=Stop::ParameterPrecision {stop=Stop::SpanBudget;}
        }
        pieces.push(Piece {cell:Cell {angle:bounds,sampled_error:error},p});
    }
    let mut curves=Vec::new();
    for chunk in pieces.chunks(85) {
        let mut controls=vec![chunk[0].p[0].to_vec()];let mut knots=vec![chunk[0].cell.angle[0];4];
        for (i,p) in chunk.iter().enumerate() {
            controls.extend(p.p[1..].iter().map(|v|v.to_vec()));
            if i+1<chunk.len() {knots.extend([p.cell.angle[1];3]);}
        }
        knots.extend([chunk.last().unwrap().cell.angle[1];4]);
        let c=Curve {degree:3,weights:vec![1.;controls.len()],knots,control_points:controls,periodic:false};c.validate()?;curves.push(c);
    }
    let error=pieces.iter().map(|p|p.cell.sampled_error).fold(0_f64,f64::max);
    Ok(Report {curves,cells:pieces.into_iter().map(|p|p.cell).collect(),stop,max_sampled_error:error,max_sampled_pressure_angle:pressure,sampled_undercut:undercut})
}
