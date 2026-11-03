//! Exact rational spatial C4 closed path; no sampled closure premise.
use super::nurbs;
use nurbs::curve::Curve;
pub fn path()->Curve {
    let points=[[0.,0.,0.],[1.,0.,0.],[2.,1.,0.],[3.,3.,1.],[4.,6.,4.],
        [2.,7.,2.],[-2.,7.,-2.],[-4.,6.,-4.],[-3.,3.,-1.],[-2.,1.,0.],[-1.,0.,0.],[0.,0.,0.]];
    Curve{degree:11,knots:[vec![-3.;12],vec![5.;12]].concat(),
        control_points:points.map(|p|vec![p[0],p[1],p[2]/64.]).to_vec(),
        weights:vec![1.,1.,1.,1.,1.,1.125,1.125,1.,1.,1.,1.,1.],periodic:false}
}
pub fn twice()->Curve {
    let mut p=path();let points=p.control_points.clone();let weights=p.weights.clone();
    p.control_points.extend(points.into_iter().skip(1));p.weights.extend(weights.into_iter().skip(1));
    p.knots=[vec![0.;12],vec![1.;11],vec![2.;12]].concat();p
}
