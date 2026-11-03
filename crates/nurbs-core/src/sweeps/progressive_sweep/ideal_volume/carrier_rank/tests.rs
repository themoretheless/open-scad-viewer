use super::*;
fn plane(x:f64,domain:[f64;2])->Surface {
    Surface{degree_u:1,degree_v:1,knots_u:vec![domain[0],domain[0],domain[1],domain[1]],knots_v:vec![7.,7.,11.,11.],
        control_points:vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]],vec![vec![x,0.,0.],vec![x,1.,0.]]],
        weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false}
}
fn work()->Work{Work{used:0,limit:100000}}
#[test]
fn exact_carrier_rank_normalizes_each_original_cap_before_homotopy() {
    let a=plane(1.,[0.,1.]);let b=plane(-1.,[101.,109.]);let before=(a.clone(),b.clone());
    let mut w=work();let q=prove([&a,&b],0.5,&mut w).unwrap().unwrap();
    assert!(q.partials[0].iter().all(Zero::is_zero));assert_eq!(q.partials[1][1],Q::from_integer(1.into()));
    assert_eq!(q.normalized_parameters,std::array::from_fn(|_|Q::new(1.into(),2.into())));
    let cost=w.used;let mut exact=Work{used:0,limit:cost};assert!(prove([&a,&b],0.5,&mut exact).unwrap().is_some());
    let mut short=Work{used:0,limit:cost-1};assert!(prove([&a,&b],0.5,&mut short).is_err());assert!(short.used<cost);
    let c=plane(-2.,[0.,2.]);assert!(prove([&a,&c],0.5,&mut work()).unwrap().is_none());
    assert_eq!(before,(a,b));
}
#[test]
fn exact_carrier_rank_collinear_columns_and_one_ulp_near_miss() {
    let mut s=plane(1.,[-3.,5.]);for i in 0..2 {for j in 0..2 {let x=(i+j) as f64;s.control_points[i][j]=vec![x,2.*x,3.*x];}}
    let q=prove([&s,&s],0.5,&mut work()).unwrap().unwrap();assert!(!q.partials[0][0].is_zero() && !q.partials[1][0].is_zero());
    s.control_points[1][0][2]=3f64.next_up();
    assert!(prove([&s,&s],0.5,&mut work()).unwrap().is_none());
}
#[test]
fn exact_carrier_rank_rational_multispan_cartesian_jets_and_ambiguous_knot() {
    let mut s=plane(1.,[-3.,5.]);s.knots_u=vec![-3.,-3.,1.,5.,5.];
    s.control_points=vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]];3];
    s.weights=vec![vec![1.,0.5],vec![2.,1.],vec![4.,2.]];
    let before=s.clone();let q=prove([&s,&s],0.5,&mut work()).unwrap().unwrap();
    assert!(q.partials[0].iter().all(Zero::is_zero));assert!(!q.partials[1][1].is_zero());assert_eq!(s,before);
    s.control_points[2][0][0]=1.;s.control_points[2][1][0]=1.;
    assert!(prove([&s,&s],0.5,&mut work()).is_err());
}

#[test]
fn exact_carrier_rank_shared_ledger_never_grants_positive_credit() {
    let s=plane(0.,[0.,1.]);
    let report=||IdealVolumeReport{embedding_certified:false,regularity_certified:false,parameter_topology_preserved:false,
        cells:0,exact_work:0,boxes:0,monotonicity_lower:None,projection:[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]],reason:None};
    let mut r=report();assert!(inspect([&s,&s],&mut r,1,100000).is_some());assert_eq!(r.cells,1);
    let mut replay=report();assert!(inspect([&s,&s],&mut replay,r.cells,r.exact_work).is_some());
    for (cells,exact) in [(0,r.exact_work),(1,0),(1,r.exact_work-1)] {
        let mut short=report();assert!(inspect([&s,&s],&mut short,cells,exact).is_none());
        assert!(short.cells<=cells && short.exact_work<=exact);
        assert!(!short.embedding_certified && !short.regularity_certified && !short.parameter_topology_preserved);
    }
}

#[test]
fn exact_carrier_rank_quadratic_rational_original_jet_has_analytic_derivatives() {
    let mut s=plane(1.,[-3.,5.]);s.degree_u=2;s.knots_u=vec![-3.,-3.,-3.,5.,5.,5.];
    s.control_points=[0.,0.,1.].map(|x|vec![vec![x,0.,0.],vec![x,1.,0.]]).to_vec();
    s.weights=vec![vec![1.,0.5],vec![2.,1.],vec![4.,2.]];
    let mut w=work();let uv=std::array::from_fn(|_|Q::new(1.into(),2.into()));
    let jet=tensor::Tensor::load(&s,&mut w).unwrap().first(&uv,&mut w).unwrap();
    assert_eq!(jet[0],[Q::new(32.into(),27.into()),Q::zero(),Q::zero()]);
    assert_eq!(jet[1],[Q::zero(),Q::new(8.into(),9.into()),Q::zero()]);
    assert!(prove([&s,&s],0.5,&mut work()).unwrap().is_none());
}
