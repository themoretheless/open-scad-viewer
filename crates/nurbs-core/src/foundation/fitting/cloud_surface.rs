//! Conditioned PCA chart and outward bounds for the polynomial cloud fit.
use crate::{Result,numeric,distance_bounds::{Interval as I,box_distance}};
fn dot(a:[f64;3],b:[f64;3])->f64 {(0..3).map(|i|a[i]*b[i]).sum()}
fn norm(a:[f64;3])->f64 {a[0].hypot(a[1]).hypot(a[2])}
/// Orthogonal power iteration on normalized source coordinates. Reject a
/// numerically rank-one cloud instead of generating two coincident axes.
pub(super) fn parameters(points:&[[f64;3]])->Result<Vec<[f64;2]>> {
    let scale=points.iter().flatten().map(|v|v.abs()).fold(0_f64,f64::max);
    numeric(scale>0.,"Cloud surface needs two-dimensional spread")?;
    let center:[f64;3]=std::array::from_fn(|k|points.iter().map(|p|(p[k]/scale)/points.len() as f64).sum());
    let data:Vec<[f64;3]>=points.iter().map(|p|std::array::from_fn(|k|p[k]/scale-center[k])).collect();
    let mut cov=[[0.;3];3];for p in &data {for i in 0..3 {for j in 0..3 {cov[i][j]+=p[i]*p[j];}}}
    let magnitude=cov.iter().flatten().map(|v|v.abs()).fold(0_f64,f64::max);
    numeric(magnitude.is_finite()&&magnitude>0.,"Cloud surface covariance collapsed")?;
    for row in &mut cov {for v in row {*v/=magnitude;}}
    let mut axes: [[f64;3];2]=[[0.;3];2];
    let first=(0..3).max_by(|&i,&j|cov[i][i].total_cmp(&cov[j][j])).unwrap();axes[0][first]=1.;
    for k in 0..2 {
        if k==1 {
            // A merely perpendicular coordinate axis may be the plane normal.
            // Choose the projected covariance image with greatest spread.
            let mut best=[0.;3];
            for axis in 0..3 {
                let mut seed=[0.;3];seed[axis]=1.;
                let projection=dot(seed,axes[0]);for i in 0..3 {seed[i]-=projection*axes[0][i];}
                let mut next=std::array::from_fn(|i|dot(cov[i],seed));
                for _ in 0..2 {let p=dot(next,axes[0]);for i in 0..3 {next[i]-=p*axes[0][i];}}
                if norm(next)>norm(best) {best=next;}
            }
            let length=norm(best);
            numeric(length>1e-14,"Cloud surface needs a separated second covariance direction")?;
            axes[1]=best.map(|v|v/length);
        }
        for _ in 0..24 {
            let mut next=std::array::from_fn(|i|dot(cov[i],axes[k]));
            if k==1 {for _ in 0..2 {let projection=dot(next,axes[0]);for i in 0..3 {next[i]-=projection*axes[0][i];}}}
            let length=norm(next);
            numeric(length.is_finite()&&length>1e-14,"Cloud surface has no numerically separated second PCA axis")?;
            axes[k]=next.map(|v|v/length);
        }
    }
    Ok(data.iter().map(|p|[dot(*p,axes[0]),dot(*p,axes[1])]).collect())
}
fn casteljau(mut p:Vec<[I;3]>,t:f64)->Result<[I;3]> {
    let a=I::point(1.).sub(I::point(t))?;let b=I::point(t);
    for n in (1..p.len()).rev() {for i in 0..n {for k in 0..3 {p[i][k]=p[i][k].mul(a)?.add(p[i+1][k].mul(b)?)?;}}}
    Ok(p[0])
}
/// Exact binary64-authored polynomial net, evaluated with outward operations.
pub(super) fn site_error(controls:&[Vec<Vec<f64>>],uv:&[[f64;2]],points:&[[f64;3]])->Result<f64> {
    let mut worst=0_f64;
    for (q,p) in uv.iter().zip(points) {
        let rows=controls.iter().map(|r|casteljau(r.iter().map(|c|std::array::from_fn(|k|I::point(c[k]))).collect(),q[1])).collect::<Result<Vec<_>>>()?;
        let evaluated=casteljau(rows,q[0])?;
        worst=worst.max(box_distance(&evaluated,&p.map(I::point))?.1);
    }
    Ok(worst)
}
/// Positive Bernstein basis keeps the entire fitted surface inside this box.
/// The maximum pair distance between control/data boxes bounds both directed
/// distances, without assuming a sample density or an unseen source surface.
pub(super) fn hull_bound(controls:&[Vec<Vec<f64>>],points:&[[f64;3]])->Result<f64> {
    let bounds=|k:usize,source:bool|->Result<I> {
        let values:Vec<f64>=if source {points.iter().map(|p|p[k]).collect()} else {controls.iter().flatten().map(|p|p[k]).collect()};
        I::new(values.iter().copied().fold(f64::INFINITY,f64::min),values.iter().copied().fold(f64::NEG_INFINITY,f64::max))
    };
    let a=(0..3).map(|k|bounds(k,false)).collect::<Result<Vec<_>>>()?;
    let b=(0..3).map(|k|bounds(k,true)).collect::<Result<Vec<_>>>()?;
    Ok(box_distance(&a,&b)?.1)
}
