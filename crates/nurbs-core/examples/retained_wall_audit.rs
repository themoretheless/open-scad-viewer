//! Run with: cargo run -p nurbs-core --example retained_wall_audit
use nurbs_core::{curve::Curve, surface::Surface, retained_wall_coefficients};
fn main() {
    let start=Curve{degree:2,knots:vec![7.,7.,7.,19.,19.,19.],
        control_points:vec![vec![0.,0.,0.],vec![1.,1.,0.],vec![2.,0.,0.]],
        weights:vec![1.,0.5,1.],periodic:false};
    let mut end=start.clone();
    for point in &mut end.control_points {point[2]=4.;}
    let surface=Surface{degree_u:2,degree_v:1,knots_u:vec![0.,0.,0.,1.,1.,1.],
        knots_v:vec![0.,0.,1.,1.],
        control_points:(0..3).map(|i|vec![start.control_points[i].clone(),end.control_points[i].clone()]).collect(),
        weights:(0..3).map(|i|vec![start.weights[i],end.weights[i]]).collect(),
        periodic_u:false,periodic_v:false};
    assert!(retained_wall_coefficients::matches(&surface,&start,&end,3));
    let mut damaged=surface.clone();damaged.control_points[1][0][0]+=0.125;
    assert!(!retained_wall_coefficients::matches(&damaged,&start,&end,3));
    assert!(!retained_wall_coefficients::matches(&surface,&start,&end,2));
    println!("Exact retained coefficients matched; damaged controls and insufficient budget refused.");
    // This premise alone does not prove filled UV coverage or shell embedding.
}
