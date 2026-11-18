//! Exact native source packet: rational G2 rings on a bilinear graph carrier.
//! Source construction is algebraic; sampled lifts do not define these curves.
#![allow(dead_code)] // shared by several test targets; each one uses a subset
use super::nurbs;
use nurbs::{curve::Curve,surface::Surface};
#[path="arc_bishop_source.rs"] mod arc;
pub const R:f64=3./128.;
pub fn path(reclock:bool)->Curve {
    let mut p=arc::path(reclock);for q in &mut p.control_points {q[2]=0.;}p
}
pub fn guide(path:&Curve)->Curve {
    let mut g=path.clone();for q in &mut g.control_points {q[2]+=R;}g
}
pub fn periodic(base:f64,peak:f64)->Curve {
    Curve{degree:6,knots:[vec![7.;7],vec![11.;7]].concat(),
        control_points:[base,base,base,peak,base,base,base].map(|x|vec![x,0.,0.]).to_vec(),
        weights:vec![1.;7],periodic:false}
}
// Rational quadratic quarter homogeneous products yield degree-four weights
// [1,1/2,1/2,1/2,1]. R=3/128 makes every Euclidean pole exactly dyadic.
// The nonplanar carrier is X=oldY*oldZ/16, Y=-oldZ, Z=oldY.
pub fn ring(scale:f64,offset:[f64;2],reverse:bool,warped:bool)->(Vec<Curve>,Vec<Curve>) {
    let r=R*scale;let a=[r,r,2.*r/3.,r/2.,0.];let b=[0.,r/2.,2.*r/3.,r,r];
    let h=[0.,r*r/32.,r*r/24.,r*r/32.,0.];let mut profiles=Vec::new();
    for q in 0..4 {
        let poles=(0..5).map(|i|{
            let (y,z)=match q {0=>(a[i],b[i]),1=>(-b[i],a[i]),2=>(-a[i],-b[i]),_=>(b[i],-a[i])};
            let x=if warped {(if q%2==0 {h[i]} else {-h[i]})+(offset[0]*z+offset[1]*y+offset[0]*offset[1])/16.} else {0.};
            vec![x,-(z+offset[1]),y+offset[0]]
        }).collect();
        profiles.push(Curve{degree:4,knots:[vec![0.;5],vec![1.;5]].concat(),control_points:poles,
            weights:vec![1.,0.5,0.5,0.5,1.],periodic:false});
    }
    let signs=[[1.,0.],[1.,1.],[0.,1.],[-1.,1.],[-1.,0.],[-1.,-1.],[0.,-1.],[1.,-1.],[1.,0.]];
    let mut trims=(0..4).map(|q|Curve{degree:2,knots:[vec![7.;3],vec![11.;3]].concat(),
        control_points:signs[2*q..=2*q+2].iter().map(|s|vec![1.+2.*(scale*s[0]+offset[0]/R),4.+2.*(scale*s[1]+offset[1]/R)]).collect(),
        weights:vec![1.,0.5,1.],periodic:false}).collect::<Vec<_>>();
    if reverse {
        for ring in [&mut profiles,&mut trims] {
            ring.reverse();for c in ring {c.control_points.reverse();c.weights.reverse();}
        }
    }
    (profiles,trims)
}
pub fn material(hole:bool,warped:bool)->(Vec<Curve>,Surface,Vec<Vec<Curve>>) {
    let surface=Surface{degree_u:1,degree_v:1,knots_u:vec![-1.,-1.,3.,3.],knots_v:vec![2.,2.,6.,6.],
        control_points:[-R,R].into_iter().map(|y|[-R,R].into_iter().map(|z|vec![if warped {y*z/16.} else {0.},-z,y]).collect()).collect(),
        weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false};
    let (mut profiles,outer)=ring(1.,[0.;2],false,warped);let mut trims=vec![outer];
    if hole {let (p,t)=ring(0.25,[0.;2],true,warped);profiles.extend(p);trims.push(t);}
    (profiles,surface,trims)
}

/// Actual closed centerline coincidence at raw parameters 0 and 2.
/// Quintic Hermite spans share position, first derivative and zero second jets.
pub fn figure_eight()->Curve {
    let p=[[0.,0.,0.],[10.,0.,0.],[0.,0.,0.],[-10.,0.,0.],[0.,0.,0.]];
    let v=[[5.,10.,0.],[0.,-10.,0.],[-5.,10.,0.],[0.,-10.,0.],[5.,10.,0.]];
    let mut poles=Vec::new();
    for i in 0..4 {
        let q=[p[i],std::array::from_fn(|k|p[i][k]+v[i][k]/5.),std::array::from_fn(|k|p[i][k]+2.*v[i][k]/5.),
            std::array::from_fn(|k|p[i+1][k]-2.*v[i+1][k]/5.),std::array::from_fn(|k|p[i+1][k]-v[i+1][k]/5.),p[i+1]];
        poles.extend(q.into_iter().skip(usize::from(i!=0)).map(|x|x.to_vec()));
    }
    Curve{degree:5,knots:[vec![0.;6],vec![1.;5],vec![2.;5],vec![3.;5],vec![4.;6]].concat(),
        weights:vec![1.;poles.len()],control_points:poles,periodic:false}
}
