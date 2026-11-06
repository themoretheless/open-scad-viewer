//! Original rational product coordinates with arbitrary positive separable
//! weights. Only injectivity transfers from the exact scalar sign owners.
//! Isolated critical points never establish immersion or a Jacobian metric.
use super::*;
use crate::curve::Curve;
use num_rational::BigRational as Q;
fn charge(work:&mut usize,max:usize)->bool {
    if *work>=max {false}else{*work+=1;true}
}
pub(super) fn certify(s:&Surface,work:&mut usize,max:usize)->Result<Option<[usize;2]>> {
    let n=[s.control_points.len(),s.control_points[0].len()];
    let d=[s.degree_u,s.degree_v];let knots=[&s.knots_u,&s.knots_v];
    if s.periodic_u || s.periodic_v || n!=[d[0]+1,d[1]+1]
        || d.iter().any(|d|!(1..=32).contains(d))
        || s.weights.iter().flatten().all(|w|*w==s.weights[0][0]) {return Ok(None);}
    for axis in 0..2 {for (i,&k) in knots[axis].iter().enumerate() {
        if !charge(work,max) || k!=knots[axis][if i<n[axis] {d[axis]}else{n[axis]}] {return Ok(None);}
    }}
    if s.weights.iter().flatten().any(|v|!v.is_finite() || *v<=0.) {return Ok(None);}
    for i in 1..n[0] {for j in 1..n[1] {
        // Cancel a literal common POSITIVE factor before doing arithmetic.
        // This preserves exactness for constant factors and avoids making
        // an independent proof pay for a full rational matrix conversion.
        if s.weights[i][0]==s.weights[0][0] || s.weights[0][j]==s.weights[0][0] {
            if !charge(work,max) {return Ok(None);}
            let expected=if s.weights[i][0]==s.weights[0][0] {s.weights[0][j]}else{s.weights[i][0]};
            if s.weights[i][j]!=expected {return Ok(None);}
            continue;
        }
        if !charge(work,max) {return Ok(None);}
        let a=crate::exact_curve_segments::exact_mul(s.weights[i][j],s.weights[0][0]);
        if !charge(work,max) {return Ok(None);}
        let b=crate::exact_curve_segments::exact_mul(s.weights[i][0],s.weights[0][j]);
        if let (Some(a),Some(b))=(a,b) {
            if !charge(work,max) || a!=b {return Ok(None);}
        }else{
            let mut q=Vec::new();
            for value in [s.weights[i][j],s.weights[0][0],s.weights[i][0],s.weights[0][j]] {
                if !charge(work,max) {return Ok(None);}
                q.push(Q::from_float(value).expect("finite original weight"));
            }
            if !charge(work,max) {return Ok(None);}
            let a=&q[0]*&q[1];
            if !charge(work,max) {return Ok(None);}
            let b=&q[2]*&q[3];
            if !charge(work,max) || a!=b {return Ok(None);}
        }
    }}
    let mut axes=[None;2];
    for axis in 0..2 {for coord in 0..3 {
        if !charge(work,max) {return Ok(None);}
        let last=if axis==0 {&s.control_points[n[0]-1][0]}else{&s.control_points[0][n[1]-1]};
        if s.control_points[0][0][coord]==last[coord] {continue;}
        let mut independent=true;
        'poles:for i in 0..n[0] {for j in 0..n[1] {
            if !charge(work,max) {return Ok(None);}
            let base=if axis==0 {s.control_points[i][0][coord]}else{s.control_points[0][j][coord]};
            if s.control_points[i][j][coord]!=base {independent=false;break 'poles;}
        }}
        if !independent {continue;}
        // Exact tensor factorization cancels the OTHER axis denominator.
        // This is the original scalar N/W, not a changed source-clock view.
        let scalar=Curve {degree:d[axis],knots:knots[axis].clone(),
            control_points:(0..n[axis]).map(|i|vec![if axis==0 {s.control_points[i][0][coord]}else{s.control_points[0][i][coord]},0.]).collect(),
            weights:(0..n[axis]).map(|i|if axis==0 {s.weights[i][0]}else{s.weights[0][i]}).collect(),periodic:false};
        if crate::trim_simplicity::rational_coordinate_monotone(&scalar,work,max)? {
            axes[axis]=Some(coord);break;
        }
    }}
    Ok(match axes {[Some(a),Some(b)]if a!=b=>Some([a,b]),_=>None})
}
#[cfg(test)]
mod tests {
    use super::*;
    fn surface()->Surface {
        let weights=[1.,2.,2.,2.,2.,1.];
        Surface {degree_u:5,degree_v:1,knots_u:[vec![0.;6],vec![1.;6]].concat(),knots_v:vec![0.,0.,1.,1.],
            control_points:weights.iter().enumerate().map(|(i,&w)|(0..2).map(|j|vec![if i%2==0 {-10./w}else{10./w},(2.*(i*i)as f64-9.*i as f64+7.5)/w,j as f64]).collect()).collect(),
            weights:weights.iter().map(|&w|vec![w,w]).collect(),periodic_u:false,periodic_v:false}
    }
    #[test]
    fn non_mobius_product_owns_original_coordinate_signs_without_rank_credit() {
        let s=surface();let before=s.clone();let mut used=0;
        assert_eq!(certify(&s,&mut used,1000).unwrap(),Some([0,2]),"work={used}");
        let mut replay=0;assert_eq!(certify(&s,&mut replay,used).unwrap(),Some([0,2]));assert_eq!(used,replay);
        assert!(certify(&s,&mut 0,used-1).unwrap().is_none());
        let r=certify_chart(&s,1000).unwrap();assert!(r.proven,"{r:?}");
        assert_eq!(r.reason,"global-exact-rational-separable-monotonicity");
        assert!(r.contraction_upper.is_none() && r.polar_p_matrix.is_none());
        assert!(!certify_contraction(&s,1000).unwrap().proven);
        let mut drift=s.clone();drift.weights[2][1]=drift.weights[2][1].next_up();
        assert!(certify(&drift,&mut 0,1000).unwrap().is_none());
        let mut folded=s.clone();folded.control_points[5]=folded.control_points[0].clone();
        assert!(certify(&folded,&mut 0,1000).unwrap().is_none());assert_eq!(s,before);
    }
}
