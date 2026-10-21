//! Sagitta-driven rational caps and convex lens surface assemblies.
//! Exact conic constructions in real arithmetic; no B-rep or rounding certificate.
use crate::{Result,check,curve::Curve,surface::{Surface,revolve}};
fn dimensions(center:[f64;3],axes:[f64;2],height:f64)->Result<()> {
    check(center.iter().all(|v|v.is_finite()) && axes.iter().all(|v|v.is_finite()&&*v>0.)
        && height.is_finite() && height>=0.,"Invalid cap center, axes or height")?;
    check((0..2).all(|i|(center[i]+axes[i]).is_finite() && center[i]+axes[i]>center[i]
        && (center[i]-axes[i]).is_finite() && center[i]-axes[i]<center[i]),"Cap axes overflow or collapse")?;
    Ok(())
}
fn cap(center:[f64;3],axes:[f64;2],height:f64,sign:f64)->Result<Surface> {
    dimensions(center,axes,height)?;
    let a=axes[0];
    // Scale the sagitta ratio before forming a²+h². For h>a this remains
    // the major cap, rather than silently replacing it with a hemisphere.
    let (degree,points,weights,knots)=if height==0. {
        (1,vec![vec![0.,0.,0.],vec![a,0.,0.]],vec![1.,1.],vec![0.,0.,1.,1.])
    } else {
        let scale=a.max(height);let ar=a/scale;let hr=height/scale;
        let tangent=scale*((ar*ar+hr*hr)/(2.*ar));
        let weight=ar/ar.hypot(hr);
        check(tangent.is_finite()&&tangent>0.&&weight.is_finite()&&weight>0.,"Cap conic exceeds numeric range")?;
        (2,vec![vec![0.,0.,height],vec![tangent,0.,height],vec![a,0.,0.]],vec![1.,weight,1.],vec![0.,0.,0.,1.,1.,1.])
    };
    let profile=Curve {degree,control_points:points,weights,knots,periodic:false};
    let mut s=revolve(&profile,[0.;3],[0.,0.,1.],360.)?;
    // Materialize the pole and transform only once, preserving the rim row.
    for (i,row) in s.control_points.iter_mut().enumerate() {for p in row {
        if i==0 {p[0]=0.;p[1]=0.;}
        p[0]+=center[0];p[1]=center[1]+(p[1]/a)*axes[1];p[2]=center[2]+sign*p[2];
    }}
    check(height==0. || (center[2]+sign*height).is_finite() && center[2]+sign*height!=center[2],"Cap height overflows or collapses")?;
    s.validate()?;Ok(s)
}
/// Upper elliptic-base spherical-profile cap, including major caps h>a.
/// The base lies at center.z, the pole at center.z+height. No bottom disk.
pub fn dome(center:[f64;3],base_axes:[f64;2],height:f64)->Result<Surface> {
    check(height>0.,"Dome requires positive height")?;cap(center,base_axes,height,1.)
}
#[derive(Clone,Debug)]
pub struct Lens {
    pub upper:Surface,
    pub lower:Surface,
    /// Present only for positive rim thickness; its two boundary rows reuse
    /// the exact cap rim controls and weights, avoiding independently fit seams.
    pub barrel:Option<Surface>,
    pub rim_thickness:f64,
}
/// Biconvex or plano-convex lens. Sags are nonnegative minor caps (<= X radius).
/// Vertex thickness must be at least their sum. Zero sag is a planar disk;
/// both zero yields a cylinder. Concave/meniscus lenses are not this contract.
pub fn lens(center:[f64;3],base_axes:[f64;2],sags:[f64;2],vertex_thickness:f64)->Result<Lens> {
    dimensions(center,base_axes,vertex_thickness)?;
    check(vertex_thickness>0. && sags.iter().all(|h|h.is_finite()&&*h>=0.&&*h<=base_axes[0])
        && sags[0]+sags[1]<=vertex_thickness,"Lens requires minor nonnegative sags and thickness >= sag sum")?;
    let rim=vertex_thickness-(sags[0]+sags[1]);
    let top=center[2]+vertex_thickness*0.5-sags[0];
    let bottom=top-rim;
    check(top.is_finite()&&bottom.is_finite()&&(rim==0.||top>bottom),"Lens rim thickness overflows or collapses")?;
    let upper=cap([center[0],center[1],top],base_axes,sags[0],1.)?;
    let mut lower=cap([center[0],center[1],bottom],base_axes,sags[1],-1.)?;
    // Canonical horizontal cap rims have the same original angular basis.
    let upper_rim=upper.control_points.last().unwrap();
    let lower_rim=lower.control_points.last_mut().unwrap();
    for (a,b) in upper_rim.iter().zip(lower_rim.iter_mut()) {b[0]=a[0];b[1]=a[1];b[2]=bottom;}
    let barrel=if rim==0. {None} else {
        let s=Surface {degree_u:1,degree_v:upper.degree_v,knots_u:vec![0.,0.,1.,1.],knots_v:upper.knots_v.clone(),
            control_points:vec![lower_rim.clone(),upper_rim.clone()],weights:vec![upper.weights.last().unwrap().clone();2],periodic_u:false,periodic_v:false};
        s.validate()?;Some(s)
    };
    lower.validate()?;
    Ok(Lens {upper,lower,barrel,rim_thickness:rim})
}
