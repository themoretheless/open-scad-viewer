use super::*;
use crate::{affine,analysis::primitive_fit as fit,primitives,surface::Surface};

fn orient(surface: &Surface, origin:[f64;3], z:[f64;3])->Result<Surface>{
    let cross=|a:[f64;3],b:[f64;3]|[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]];
    let helper=if z[0].abs()<0.8 {[1.,0.,0.]} else {[0.,1.,0.]};
    let x=cross(helper,z);
    let norm=x[0].hypot(x[1]).hypot(x[2]);
    check(norm.is_finite()&&norm>0.,"Fitted axis has no transverse frame")?;
    let x=x.map(|v|v/norm);
    let y=cross(z,x);
    affine::surface(surface,&[[x[0],y[0],z[0],origin[0]],[x[1],y[1],z[1],origin[1]],[x[2],y[2],z[2],origin[2]],[0.,0.,0.,1.]])
}
pub(super) fn build(v:&Value,op:&str)->Result<Value>{
    let points:Vec<[f64;3]>=field(v,"points")?;
    let (surface,metrics,parameters)=match op{
        "fit_sphere_cloud"=>{
            let f=fit::fit_sphere(&points)?;
            (primitives::sphere(f.center,f.radius)?,f.report,json!({"center":f.center,"radiusMm":f.radius}))
        }
        "fit_torus_cloud"=>{
            let f=fit::fit_torus(&points)?;
            let local=primitives::torus([0.;3],f.major_radius,f.minor_radius,f.minor_radius)?;
            (orient(&local,f.center,f.axis)?,f.report,json!({"center":f.center,"axis":f.axis,"majorRadiusMm":f.major_radius,"minorRadiusMm":f.minor_radius}))
        }
        "fit_cylinder_cloud"=>{
            let f=fit::fit_cylinder(&points)?;
            let h:f64=field(v,"height")?;
            let local=primitives::cylinder([0.,0.,-h/2.],f.radius,h)?;
            (orient(&local,f.point_on_axis,f.direction)?,f.report,json!({"axisPoint":f.point_on_axis,"axis":f.direction,"radiusMm":f.radius}))
        }
        "fit_cone_cloud"=>{
            let f=fit::fit_cone(&points)?;
            let h:f64=field(v,"height")?;
            let local=primitives::cone_frustum([0.;3],0.,h*f.half_angle.tan(),h)?;
            (orient(&local,f.vertex,f.axis)?,f.report,json!({"vertex":f.vertex,"axis":f.axis,"halfAngleRad":f.half_angle}))
        }
        _=>{
            let f=fit::fit_plane(&points)?;
            let w:f64=field(v,"width")?; let h:f64=field(v,"height")?;
            check(w>0. && h>0.,"Fitted plane width and height must be positive")?;
            let local=crate::patches::plane([-w/2.,-h/2.,0.],[w,0.,0.],[0.,h,0.])?;
            // Anchor the displayed rectangle at the projected cloud centroid.
            let n=points.len() as f64;
            let center:[f64;3]=std::array::from_fn(|k|points.iter().map(|p|p[k]/n).sum());
            let distance=f.offset+(0..3).map(|k|center[k]*f.normal[k]).sum::<f64>();
            let origin=std::array::from_fn(|k|center[k]-distance*f.normal[k]);
            (orient(&local,origin,f.normal)?,f.report,json!({"normal":f.normal,"offsetMm":f.offset,"center":origin}))
        }
    };
    Ok(json!({"surface":surface,"parameters":parameters,"sampleCount":points.len(),"rmsMm":metrics.rms,
        "maxDeviationMm":metrics.max_dev,"iterations":metrics.iterations,"converged":metrics.converged}))
}
