//! Outward Bernstein extraction from one ORIGINAL B-spline chart.
use super::*;
pub(super) fn select(source:&Surface,p:&Curve,direction:[f64;2])->Result<([usize;2],[[f64;2];2])>{
    let mut spans=[0;2];let mut domains=[[0.;2];2];
    for axis in 0..2 {
        let (knots,degree,n)=if axis==0 {(&source.knots_u,source.degree_u,source.control_points.len())}
            else {(&source.knots_v,source.degree_v,source.control_points[0].len())};
        let mut low=f64::INFINITY;let mut high=f64::NEG_INFINITY;
        for point in &p.control_points {
            let end=if direction[axis]==0. {I::point(point[axis])}
                else if let Some(value)=crate::exact_curve_segments::exact_add(point[axis],direction[axis]) {I::point(value)}
                else {I::point(point[axis]).add(I::point(direction[axis]))?};
            low=low.min(point[axis]).min(end.lo);high=high.max(point[axis]).max(end.hi);
        }
        let span=(degree..n).find(|&i|knots[i]<knots[i+1]&&knots[i]<=low&&high<=knots[i+1])
            .ok_or_else(||refused("Original collar map crosses a knot chart"))?;
        // A touched interior knot must have a unique original two-jet. Do not
        // grant a whole-support G2 claim from an arbitrary one-sided branch.
        for value in [knots[span],knots[span+1]] {
            if low<=value&&value<=high&&knots[degree]<value&&value<knots[n]
                &&knots.iter().filter(|&&v|v==value).count()+2>degree {
                return Err(refused("Original collar touches a non-C2 knot"));
            }
        }
        spans[axis]=span;domains[axis]=[knots[span],knots[span+1]];
    }
    Ok((spans,domains))
}
fn line(controls:&[[I;4]],knots:&[f64],degree:usize,span:usize,ends:[f64;2],work:&mut Work)->Result<Vec<[I;4]>>{
    let mut out=Vec::with_capacity(degree+1);
    for k in 0..=degree {
        let mut values=controls.to_vec();
        for r in 1..=degree {
            let at=ends[usize::from(r>degree-k)];
            for j in (r..=degree).rev() {
                let i=span-degree+j;
                work.charge(4)?;
                let alpha=if at==knots[i] {I::point(0.)}
                    else if at==knots[i+degree-r+1] {I::point(1.)}
                    else {I::point(at).sub(I::point(knots[i]))?.div(I::point(knots[i+degree-r+1]).sub(I::point(knots[i]))?)?.intersect(0.,1.)?};
                let beta=I::point(1.).sub(alpha)?.intersect(0.,1.)?;
                for axis in 0..4 {
                    work.charge(3)?;let (a,b)=(values[j-1][axis],values[j][axis]);
                    values[j][axis]=a.mul(beta)?.add(b.mul(alpha)?)?.intersect(a.lo.min(b.lo),a.hi.max(b.hi))?;
                }
            }
        }
        out.push(values[degree]);
    }
    Ok(out)
}
pub(super) fn net(source:&Surface,spans:[usize;2],domains:[[f64;2];2],origin:[f64;3],work:&mut Work)->Result<Vec<Vec<[I;4]>>>{
    let (p,q)=(source.degree_u,source.degree_v);
    let mut controls=vec![vec![[I::point(0.);4];q+1];p+1];
    for (i,row) in controls.iter_mut().enumerate(){for (j,h) in row.iter_mut().enumerate(){
        let (u,v)=(spans[0]-p+i,spans[1]-q+j);let weight=I::point(source.weights[u][v]);h[3]=weight;
        for k in 0..3 {work.charge(2)?;h[k]=I::point(source.control_points[u][v][k]).sub(I::point(origin[k]))?.mul(weight)?;}
    }}
    for row in &mut controls {*row=line(row,&source.knots_v,q,spans[1],domains[1],work)?;}
    let mut out=vec![vec![[I::point(0.);4];q+1];p+1];
    for j in 0..=q {let column=controls.iter().map(|r|r[j]).collect::<Vec<_>>();
        for (i,value) in line(&column,&source.knots_u,p,spans[0],domains[0],work)?.into_iter().enumerate(){out[i][j]=value;}
    }
    Ok(out)
}
