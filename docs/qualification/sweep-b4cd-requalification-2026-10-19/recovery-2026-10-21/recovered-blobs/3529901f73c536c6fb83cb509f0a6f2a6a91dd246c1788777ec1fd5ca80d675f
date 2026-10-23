//! Cubic corrugated annular skins with shared-data end rims.
//! Radial wall thickness, not constant normal thickness. No B-rep incidence
//! or rounding-inclusive geometry certificate is created.
use crate::{Result,check,numeric,surface::Surface};
#[derive(Clone,Copy,Debug)]
pub struct Request {
    pub center:[f64;3], pub mean_radius:f64, pub amplitude:f64,
    pub height:f64, pub radial_thickness:f64, pub periods:usize,
}
#[derive(Clone,Debug)]
pub struct Shell {
    pub outer:Vec<Surface>, pub inner:Vec<Surface>,
    pub bottom:Surface, pub top:Surface,
}
fn circle_knots()->Vec<f64> {vec![0.,0.,0.,1.,1.,2.,2.,3.,3.,4.,4.,4.]}
fn circles()->([[f64;2];9],Vec<f64>) {
    ([[1.,0.],[1.,1.],[0.,1.],[-1.,1.],[-1.,0.],[-1.,-1.],[0.,-1.],[1.,-1.],[1.,0.]],vec![1.,0.5_f64.sqrt(),1.,0.5_f64.sqrt(),1.,0.5_f64.sqrt(),1.,0.5_f64.sqrt(),1.])
}
fn rim(inner:Vec<Vec<f64>>,outer:Vec<Vec<f64>>,weights:Vec<f64>)->Result<Surface> {
    let s=Surface {degree_u:1,degree_v:2,knots_u:vec![0.,0.,1.,1.],knots_v:circle_knots(),control_points:vec![inner,outer],weights:vec![weights;2],periodic_u:false,periodic_v:false};s.validate()?;Ok(s)
}
/// Radius is an alternating cubic Hermite wave: each half-period goes from
/// mean+amplitude to mean-amplitude with zero endpoint slope. The axial radius
/// law is C2 in real arithmetic. Signed amplitude selects the initial extremum.
/// Output normals face outward on the outer skin, inward on the cavity, -Z/+Z
/// on bottom/top. Shared circle data is reversed when edge directions differ.
pub fn cubic(r:Request)->Result<Shell> {
    check(r.center.iter().all(|v|v.is_finite())&&r.mean_radius.is_finite()&&r.amplitude.is_finite()
        &&r.radial_thickness.is_finite()&&r.radial_thickness>0.&&r.mean_radius>r.amplitude.abs()+r.radial_thickness
        &&r.height.is_finite()&&r.height>0.&&(1..=256).contains(&r.periods),"Bellows requires positive inner radius, height, radial thickness and 1..256 periods")?;
    let pieces=2*r.periods;
    let z:Vec<_>=(0..=3*pieces).map(|i|r.center[2]+r.height*(i as f64/(3*pieces) as f64)).collect();
    numeric(z.iter().all(|v|v.is_finite())&&z.windows(2).all(|v|v[0]<v[1]),"Bellows axial controls overflow or collapse")?;
    let radii:Vec<_>=(0..=3*pieces).map(|i| {
        let segment=(i/3).min(pieces-1);let local=i-3*segment;
        let sign=if segment%2==0 {1.} else {-1.};
        r.mean_radius+sign*r.amplitude*if local<=1 {1.} else {-1.}
    }).collect();
    for &outer in &radii {
        let inner=outer-r.radial_thickness;
        numeric(inner>0.&&inner<outer,"Bellows radial thickness collapses")?;
        for axis in 0..2 {
            let c=r.center[axis];
            numeric(c+inner>c&&c-inner<c&&c+outer>c+inner&&c-outer<c-inner,
                "Bellows transverse coordinates or wall gap collapse")?;
        }
    }
    let (circle,weights)=circles();let mut outer=Vec::new();let mut inner=Vec::new();
    for start in (0..pieces).step_by(10) {
        let end=(start+10).min(pieces);let mut knots=vec![start as f64/pieces as f64;4];
        for i in start+1..end {knots.extend([i as f64/pieces as f64;3]);}knots.extend([end as f64/pieces as f64;4]);
        for cavity in [false,true] {
            let controls=(3*start..=3*end).map(|i| {
                let radius=radii[i]-if cavity {r.radial_thickness} else {0.};
                (0..9).map(|j| {let q=circle[if cavity {j} else {8-j}];vec![r.center[0]+radius*q[0],r.center[1]+radius*q[1],z[i]]}).collect()
            }).collect();
            let s=Surface {degree_u:3,degree_v:2,knots_u:knots.clone(),knots_v:circle_knots(),control_points:controls,
                weights:vec![weights.clone();3*(end-start)+1],periodic_u:false,periodic_v:false};s.validate()?;
            if cavity {inner.push(s);} else {outer.push(s);}
        }
    }
    let reversed=|row:&Vec<Vec<f64>>|row.iter().rev().cloned().collect();
    let bottom=rim(reversed(&inner[0].control_points[0]),outer[0].control_points[0].clone(),weights.clone())?;
    let top=rim(inner.last().unwrap().control_points.last().unwrap().clone(),reversed(outer.last().unwrap().control_points.last().unwrap()),weights)?;
    Ok(Shell {outer,inner,bottom,top})
}
