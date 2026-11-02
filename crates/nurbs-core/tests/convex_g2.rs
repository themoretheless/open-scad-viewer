use nurbs_core::{curve::Curve, surface::Surface,
    original_graph_transition::{self as graph, Budget, CurvatureDecision, NormalRadiusDecision}};
#[test]
fn convex_rolloff_preserves_contacts_and_whole_field_curvature() {
    let sources: [Surface;2] = std::array::from_fn(|side|Surface {
        degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
        control_points:(0..2).map(|i|vec![vec![i as f64,0.,0.],
            vec![i as f64,if side==0{-1.}else{1.},-1./1024.]]).collect(),
        weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false,
    });
    let before=sources.clone();
    let width=Curve {degree:1,knots:vec![0.,0.,1.,1.],
        control_points:vec![vec![1./1024.,0.,0.];2],weights:vec![1.;2],periodic:false};
    let mut b=Budget::new(1_000_000).unwrap();
    let p=graph::build_convex_g2([&sources[0],&sources[1]],&width,&mut b).unwrap();
    assert_eq!(p.contact_jets(),[true;2]);
    for side in 0..2 {assert_eq!(p.source_contact(side,&mut b).unwrap().source(),&sources[side]);}
    // Exact derivative Bernstein hull proves nonnegative bending on ALL V,
    // unlike sampled curvature signs. End coefficients certify zero at seams.
    let q=p.profile_coefficients();
    let second=q.windows(3).map(|c|(&c[2]-&c[1]*num_rational::BigRational::from_integer(2.into())+&c[0])*num_rational::BigRational::from_integer(30.into())).collect::<Vec<_>>();
    let zero=num_rational::BigRational::from_integer(0.into());
    let two=num_rational::BigRational::from_integer(2.into());
    // Subdivide the exact derivative polynomial at 1/2; both Bernstein
    // hulls are nonnegative, despite a negative unsplit control coefficient.
    let mut row=second.clone();let mut left=vec![row[0].clone()];
    let mut right=vec![row.last().unwrap().clone()];
    while row.len()>1 {
        row=row.windows(2).map(|c|(&c[0]+&c[1])/&two).collect();
        left.push(row[0].clone());right.push(row.last().unwrap().clone());
    }
    assert!(left.iter().chain(&right).all(|x|x>=&zero));
    assert_eq!(row[0],num_rational::BigRational::from_integer(4.into()));
    assert_eq!(second[0],num_rational::BigRational::from_integer(0.into()));
    assert_eq!(second[4],num_rational::BigRational::from_integer(0.into()));
    let r=p.inspect_curvature(10.,10000,&mut b).unwrap();
    assert_eq!(r.decision,CurvatureDecision::Within);
    assert!(r.absolute_curvature_upper.unwrap()<=10.);
    let core=p.inspect_normal_radius(1.,0.001,[[0.,1.],[0.499,0.501]],1000,&mut b).unwrap();
    assert_eq!(core.decision,NormalRadiusDecision::Within);
    let rejected=p.inspect_curvature(1.,10000,&mut b).unwrap();
    assert_ne!(rejected.decision,CurvatureDecision::Within);
    assert_eq!(sources,before);
    assert!(graph::build_convex_g2([&sources[0],&sources[1]],&width,&mut Budget::new(0).unwrap()).is_err());
}
