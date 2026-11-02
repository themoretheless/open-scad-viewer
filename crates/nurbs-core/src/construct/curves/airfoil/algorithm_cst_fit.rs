//! Dimensionless inverse CST model with explicit world placement.
use super::*;

fn validate(points:&[[f64;3]],order:usize,n1:f64,n2:f64,lambda:f64)->Result<()> {
    check((16..=4096).contains(&points.len()),"CST fitting needs 16..4096 data sites")?;
    check((1..=11).contains(&order),"CST fit order must be 1..11")?;
    require_finite_slice(&points.iter().flatten().copied().collect::<Vec<_>>(),"cst_fit_data")?;
    check(n1.is_finite()&&n2.is_finite()&&n1>0.&&n2>0.,"CST class exponents must be positive and finite")?;
    check(lambda.is_finite()&&(0. ..=1.).contains(&lambda),"Tikhonov lambda must be in [0, 1]")
}
/// Infer chord endpoints from average extreme-X sites. Both closed tips must
/// be present; this is a heuristic frame, not a general chord detector.
/// Use cst_fit_in_frame for authored orientation or ambiguous point clouds.
pub fn cst_fit(points:&[[f64;3]],order:usize,n1:f64,n2:f64,lambda:f64)->Result<CstFitResult> {
    validate(points,order,n1,n2,lambda)?;
    cst_fit_in_frame(points,CstFrame::inferred(points)?,order,n1,n2,lambda)
}
/// Fit dimensionless CST coefficients in an authored orthogonal plane.
/// Residuals are in physical coordinates; out-of-plane deviation is reported
/// independently, never silently treated as exact planar geometry.
pub fn cst_fit_in_frame(points:&[[f64;3]],frame:CstFrame,order:usize,n1:f64,n2:f64,lambda:f64)->Result<CstFitResult> {
    fit(points,frame,order,n1,n2,lambda,None)
}
/// Fit authored upper/lower samples without inferring sides from signs.
/// This also supports cambered profiles whose two sides share an ordinate sign.
pub fn cst_fit_sides(upper:&[[f64;3]],lower:&[[f64;3]],frame:CstFrame,order:usize,n1:f64,n2:f64,lambda:f64)->Result<CstFitResult> {
    let count=upper.len().checked_add(lower.len()).ok_or_else(||crate::resource("CST site count overflow"))?;
    check((16..=4096).contains(&count),"CST fitting needs 16..4096 total data sites")?;
    let points:Vec<_>=upper.iter().chain(lower).copied().collect();
    let sides:Vec<_>=(0..count).map(|i|i<upper.len()).collect();
    fit(&points,frame,order,n1,n2,lambda,Some(&sides))
}
fn fit(points:&[[f64;3]],frame:CstFrame,order:usize,n1:f64,n2:f64,lambda:f64,sides:Option<&[bool]>)->Result<CstFitResult> {
    validate(points,order,n1,n2,lambda)?;
    let (xaxis,yaxis,normal,chord)=frame.axes()?;
    let dot=|a:[f64;3],b:[f64;3]|->f64 {(0..3).map(|i|a[i]*b[i]).sum()};
    // (normalized station, normalized ordinate, physical ordinate, source).
    let mut sites=Vec::with_capacity(points.len());let mut max_out_of_plane=0_f64;
    for &p in points {
        let d=std::array::from_fn(|i|p[i]-frame.origin[i]);
        crate::numeric(d.iter().all(|x|x.is_finite()),"CST coordinate subtraction overflowed")?;
        let u=dot(d,xaxis)/chord;let physical_v=dot(d,yaxis);let v=physical_v/chord;let z=dot(d,normal).abs();
        crate::numeric([u,v,physical_v,z].iter().all(|x|x.is_finite()),"CST projection overflowed")?;
        check((-128.*f64::EPSILON..=1.+128.*f64::EPSILON).contains(&u),"CST site lies outside the authored chord interval")?;
        sites.push((u.clamp(0.,1.),v,physical_v,p));max_out_of_plane=max_out_of_plane.max(z);
    }
    let columns=order+1;
    let row=|u:f64|->Vec<f64> {let class=u.powf(n1)*(1.-u).powf(n2);(0..columns).map(|i|class*bernstein(order,i,u)).collect()};
    let fit_side=|upper:bool|->Result<(Vec<f64>,f64,Vec<[f64;3]>)> {
        let side:Vec<_>=sites.iter().enumerate().filter(|(i,s)|match sides {Some(labels)=>labels[*i]==upper,None=>if upper {s.1>=0.} else {s.1<=0.}}).map(|(_,s)|s).collect();
        check(side.len()>columns,"CST fit needs more data sites per surface than coefficients")?;
        let design:Vec<_>=side.iter().map(|s|row(s.0)).collect();
        let targets:Vec<_>=side.iter().map(|s|vec![s.1]).collect();
        let solved=lsq_tikhonov(&design,&targets,lambda)?;
        let coefficients:Vec<_>=solved.iter().map(|r|r[0]).collect();
        crate::numeric(coefficients.iter().all(|x|x.is_finite()),"CST coefficients overflowed")?;
        let mut residual=0_f64;
        for (r,s) in design.iter().zip(&side) {
            let predicted=chord*r.iter().zip(&coefficients).map(|(a,b)|a*b).sum::<f64>();
            let error=(predicted-s.2).abs();
            crate::numeric(error.is_finite(),"CST residual overflowed")?;residual=residual.max(error);
        }
        Ok((coefficients,residual,side.iter().map(|s|s.3).collect()))
    };
    let (upper,ru,pu)=fit_side(true)?;let (lower,rl,pl)=fit_side(false)?;
    let profile=CstProfile{n1,n2,upper,lower};
    let dense=|coefficients:&[f64]|->Result<Vec<[f64;3]>> {(0..=512).map(|i| {
        let s=i as f64/512.;let u=s*s;frame.point(u,cst_shape(u,n1,n2,coefficients))
    }).collect()};
    let a=dense(&profile.upper)?;let b=dense(&profile.lower)?;
    let distance=|p:&[f64;3],q:&[f64;3]|(p[0]-q[0]).hypot(p[1]-q[1]).hypot(p[2]-q[2]);
    let discrete=|source:&[[f64;3]],target:&[[f64;3]]|source.iter().map(|p|target.iter().map(|q|distance(p,q)).fold(f64::INFINITY,f64::min)).fold(0_f64,f64::max);
    let hausdorff_residual=discrete(&pu,&a).max(discrete(&pl,&b));
    crate::numeric(hausdorff_residual.is_finite(),"CST sampled distance overflowed")?;
    Ok(CstFitResult{profile,max_residual:next_up(ru.max(rl)),hausdorff_residual:next_up(hausdorff_residual),lambda,frame,max_out_of_plane:next_up(max_out_of_plane)})
}
