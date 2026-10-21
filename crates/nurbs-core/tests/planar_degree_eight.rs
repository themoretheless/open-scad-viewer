use nurbs_core::{curve::Curve,intersection::planar_scalar};
#[test]
fn degree_eight_scalar_census_keeps_a_sixfold_stationary_zero_and_seventh_order_contact() {
    // Exact binary64 degree elevation of x=(2t-1)^7, y=8t.
    let x=[-1.,0.75,-0.5,0.25,0.,-0.25,0.5,-0.75,1.];
    let curve=Curve{degree:8,knots:[vec![0.;9],vec![1.;9]].concat(),
        control_points:x.into_iter().enumerate().map(|(i,x)|vec![x,i as f64]).collect(),weights:vec![1.;9],periodic:false};
    assert!(planar_scalar::monotone(&curve,0));
    let stationary=planar_scalar::stationary(&curve,0).unwrap().unwrap();
    assert_eq!(stationary.len(),1);assert!((stationary[0]-0.5).abs()<1e-12);
    let line=Curve::from_polyline(vec![vec![0.,-10.],vec![0.,10.]]).unwrap();
    let Some(planar_scalar::Relation::Points(points))=planar_scalar::intersect(&curve,&line,1e-7).unwrap() else {panic!("No complete scalar census")};
    assert_eq!(points.len(),1);assert_eq!(points[0].multiplicity,7);assert!((points[0].first-0.5).abs()<1e-12);
}
