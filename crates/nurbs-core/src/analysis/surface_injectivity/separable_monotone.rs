//! Source-owned product-coordinate proof, including isolated critical points.
//! If S_a(u,v)=f(u) and S_b(u,v)=g(v), with strictly monotone f,g on the
//! entire original intervals, equality of two world points forces both
//! parameters equal. The third coordinate is unrestricted. Regularity, trims,
//! other faces, shell orientation and smoothness are separate obligations.
use super::*;
use crate::curve::Curve;
fn charge(work:&mut usize,max:usize)->bool {
    if *work==max {false} else {*work+=1;true}
}
/// Validated literal polynomial single-span source; no degree elevation,
/// rounded extraction, sampled Jacobian, or transformed replacement net.
pub(super) fn certify(s:&Surface,work:&mut usize,max:usize)->Result<Option<[usize;2]>> {
    let size=[s.control_points.len(),s.control_points[0].len()];
    let degree=[s.degree_u,s.degree_v];let knots=[&s.knots_u,&s.knots_v];
    if s.periodic_u || s.periodic_v || !(1..=32).contains(&degree[0])
        || !(1..=32).contains(&degree[1]) || size!=[degree[0]+1,degree[1]+1]
        || s.control_points.iter().flatten().any(|p|p.len()!=3) {return Ok(None);}
    for axis in 0..2 {for (i,&k) in knots[axis].iter().enumerate() {
        if !charge(work,max) {return Ok(None);}
        if k!=knots[axis][if i<=degree[axis] {degree[axis]} else {size[axis]}] {return Ok(None);}
    }}
    let weight=s.weights[0][0];
    for &w in s.weights.iter().flatten() {
        if !charge(work,max) || w!=weight {return Ok(None);}
    }
    let mut axes=[None;2];
    for axis in 0..2 {for coord in 0..3 {
        if !charge(work,max) {return Ok(None);}
        let last=if axis==0 {&s.control_points[size[0]-1][0]} else {&s.control_points[0][size[1]-1]};
        if s.control_points[0][0][coord]==last[coord] {continue;}
        let mut independent=true;
        'poles:for i in 0..size[0] {for j in 0..size[1] {
            if !charge(work,max) {return Ok(None);}
            let base=if axis==0 {s.control_points[i][0][coord]} else {s.control_points[0][j][coord]};
            if s.control_points[i][j][coord]!=base {independent=false;break 'poles;}
        }}
        if !independent {continue;}
        // Common positive weights cancel EXACTLY. This scalar proof view
        // copies stored source coordinates; the second component is zero.
        let scalar=Curve {degree:degree[axis],knots:knots[axis].clone(),
            control_points:(0..size[axis]).map(|i|vec![if axis==0 {
                s.control_points[i][0][coord]
            } else {s.control_points[0][i][coord]},0.]).collect(),
            weights:vec![1.;size[axis]],periodic:false};
        if crate::trim_simplicity::polynomial_coordinate_monotone(&scalar,work,max)? {
            axes[axis]=Some(coord);break;
        }
    }}
    Ok(match axes { [Some(a),Some(b)] if a!=b=>Some([a,b]),_=>None })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn source()->Surface {
        Surface {degree_u:5,degree_v:1,knots_u:[vec![0.;6],vec![1.;6]].concat(),
            knots_v:vec![-2.,-2.,6.,6.],
            control_points:(0..6).map(|i|(0..2).map(|j|vec![
                if i%2==0 {-10.}else{10.},(2*i*i) as f64-9.*i as f64+7.5,j as f64*2.
            ]).collect()).collect(),weights:vec![vec![2.;2];6],periodic_u:false,periodic_v:false}
    }
    #[test]
    fn full_product_chart_owns_critical_projection_and_every_budget() {
        let s=source();let before=s.clone();let mut work=0;
        assert_eq!(certify(&s,&mut work,1000).unwrap(),Some([0,2]));
        let mut replay=0;assert_eq!(certify(&s,&mut replay,work).unwrap(),Some([0,2]));
        assert_eq!(replay,work);
        let mut short=0;assert_eq!(certify(&s,&mut short,work-1).unwrap(),None);
        assert_eq!(short,work-1);
        let r=certify_chart(&s,1000).unwrap();assert!(r.proven,"{r:?}");
        assert_eq!(r.reason,"global-separable-coordinate-monotonicity");
        assert!(r.contraction_upper.is_none());assert_eq!(s,before);
        assert!(!certify_contraction(&s,1000).unwrap().proven,
            "the strict rank/contraction contract must remain unchanged");
        let mut changed=s.clone();changed.control_points[2][1][0]=changed.control_points[2][1][0].next_up();
        assert_eq!(certify(&changed,&mut 0,1000).unwrap(),None);
        let mut rational=s.clone();rational.weights[2][0]=2f64.next_up();
        assert_eq!(certify(&rational,&mut 0,1000).unwrap(),None);
    }
    #[test]
    fn two_distinct_source_parameters_with_the_same_world_point_are_not_admitted() {
        let mut s=source();
        for (i,row) in s.control_points.iter_mut().enumerate() {for p in row {
            p[0]=if i==0 || i==5 {0.}else{1.};p[1]=0.;
        }}
        assert_eq!(certify(&s,&mut 0,1000).unwrap(),None);
        assert!(!certify_chart(&s,1000).unwrap().proven);
        assert_eq!(s.evaluate(0.,0.).unwrap().point,s.evaluate(1.,0.).unwrap().point);
    }
}

#[cfg(test)]
mod rank_contract {
    use super::*;
    #[test]
    fn weak_coordinate_injection_cannot_grant_fibered_interior_rank() {
        let s=Surface {degree_u:5,degree_v:1,
            knots_u:[vec![0.;6],vec![1.;6]].concat(),knots_v:vec![0.,0.,1.,1.],
            control_points:(0..6).map(|i|(0..2).map(|j|
                vec![if i%2==0 {-10.}else{10.},j as f64,0.]).collect()).collect(),
            weights:vec![vec![1.;2];6],periodic_u:false,periodic_v:false};
        // Q=(10(2u-1)^5,v,0) is injective, yet Qu=0 at u=1/2.
        let before=s.clone();
        assert!(certify_chart(&s,1000).unwrap().proven);
        assert!(!certify_contraction(&s,1000).unwrap().proven);
        assert!(!super::super::certify(&s,1000).unwrap().proven,
            "legacy callers may require rank as well as injection");
        let pose=|z|[[1.,0.,0.],[0.,1.,0.],[0.;3],[0.,0.,z]];
        let motion=[crate::affine_volume::RationalPiece {controls:vec![pose(0.),pose(1.)],weights:vec![1.;2]}];
        let r=crate::fibered_volume::certify(&s,[0.;3],&motion,
            crate::fibered_volume::Proposal {source_normal:[0.,0.,1.],
                world_rows:[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]]},
            crate::fibered_volume::Limits {cells:1000,exact_work:1000000}).unwrap();
        assert!(!r.embedding_certified && !r.parameter_topology_preserved && !r.spanwise_interior_regular,"{r:?}");
        assert_eq!(r.reason,Some("fiber-carrier-global-injectivity-unproved"));
        assert!(!r.carrier.as_ref().unwrap().proven);
        assert_eq!(s,before);
    }
}
