use num_bigint::BigInt;
use num_rational::BigRational as Q;
use num_traits::{One,Zero,ToPrimitive};
use nurbs_core::ss_intersection as ss;
fn q(n:i64,d:i64)->Q {Q::new(n.into(),d.into())}
fn product(a:&[Q],b:&[Q])->Vec<Q> {
    let mut out=vec![Q::zero();a.len()+b.len()-1];
    for (i,x) in a.iter().enumerate() {for (j,y) in b.iter().enumerate() {out[i+j]+=x*y;}}
    out
}
fn isolate(p:Vec<Q>,d:[Q;2])->ss::QualifiedSsAlgebraicRootCensus {
    ss::QualifiedSsAlgebraicRootCensus::isolate(p,d,Default::default()).unwrap()
}
#[test]
fn closed_census_includes_repeated_endpoint_and_interior_roots_once() {
    let mut p=vec![Q::one()];
    for root in [q(0,1),q(0,1),q(1,4),q(1,2),q(1,2),q(1,2),q(1,1)] {
        p=product(&p,&[-root,Q::one()]);
    }
    let census=isolate(p.clone(),[q(0,1),q(1,1)]);
    assert_eq!(census.roots().len(),4);
    for (clock,value) in census.roots().iter().zip([q(0,1),q(1,4),q(1,2),q(1,1)]) {
        assert_eq!(clock.polynomial(),p);
        assert!(clock.equals_rational(&value,Default::default()).unwrap().0);
        clock.reverify().unwrap();
    }
    assert!(census.work()>0);census.reverify().unwrap();
    // Redundant input zeros do not change the normalized replay definition.
    p.extend([Q::zero(),Q::zero()]);
    isolate(p,[q(0,1),q(1,1)]).reverify().unwrap();
    let mut highest=vec![Q::zero();65];highest[64]=Q::one();
    let endpoint=isolate(highest,[q(0,1),q(1,1)]);
    assert_eq!(endpoint.roots().len(),1);endpoint.reverify().unwrap();
}
#[test]
fn irrational_multiple_root_and_exact_midpoint_are_both_retained() {
    let p=product(&[q(1,1),q(0,1),q(-16,1),q(0,1),q(64,1)],&[q(-1,2),q(1,1)]);
    let census=isolate(p,[q(0,1),q(1,1)]);
    assert_eq!(census.roots().len(),2);
    let expected=ss::QualifiedSsAlgebraicClock::from_binary(&[-1.,0.,8.],[0.34,0.38],Default::default()).unwrap();
    assert!(census.roots()[0].equals_clock(&expected,Default::default()).unwrap().0);
    assert!(census.roots()[1].equals_rational(&q(1,2),Default::default()).unwrap().0);
    census.reverify().unwrap();
}
#[test]
fn clocks_closer_than_one_binary_ulp_are_exhaustively_separated() {
    let a=q(1,2);let b=&a+Q::new(1.into(),BigInt::one()<<80);
    assert_eq!(a.to_f64(),b.to_f64());
    let p=product(&[-a.clone(),Q::one()],&[-b.clone(),Q::one()]);
    let census=isolate(p,[q(49,100),q(51,100)]);
    assert_eq!(census.roots().len(),2);
    assert!(census.roots()[0].equals_rational(&a,Default::default()).unwrap().0);
    assert!(census.roots()[1].equals_rational(&b,Default::default()).unwrap().0);
    assert!(census.roots()[0].isolating_interval()[1]<census.roots()[1].isolating_interval()[0]);
    census.reverify().unwrap();
}
#[test]
fn empty_and_point_domains_are_proved_without_an_invented_root() {
    assert!(isolate(vec![q(1,1)],[q(0,1),q(1,1)]).roots().is_empty());
    assert!(isolate(vec![q(1,1),q(0,1),q(1,1)],[q(-10,1),q(10,1)]).roots().is_empty());
    assert_eq!(isolate(vec![q(-1,2),q(1,1)],[q(1,2),q(1,2)]).roots().len(),1);
    assert!(isolate(vec![q(-1,2),q(1,1)],[q(0,1),q(0,1)]).roots().is_empty());
}
#[test]
fn exhausted_or_invalid_census_never_returns_a_partial_list() {
    let p=product(&[q(-1,4),q(1,1)],&[q(-3,4),q(1,1)]);
    let d=[q(0,1),q(1,1)];
    for limits in [ss::SsAlgebraicRootCensusLimits {max_cells:1,..Default::default()},
        ss::SsAlgebraicRootCensusLimits {max_roots:1,..Default::default()},
        ss::SsAlgebraicRootCensusLimits {clock:ss::SsAlgebraicClockLimits {max_operations:1,..Default::default()},..Default::default()}] {
        assert!(ss::QualifiedSsAlgebraicRootCensus::isolate(p.clone(),d.clone(),limits).is_err());
    }
    assert!(ss::QualifiedSsAlgebraicRootCensus::isolate(vec![Q::zero()],d.clone(),Default::default()).is_err());
    assert!(ss::QualifiedSsAlgebraicRootCensus::isolate(p,[q(1,1),q(0,1)],Default::default()).is_err());
    assert!(ss::QualifiedSsAlgebraicRootCensus::isolate(vec![Q::new_raw(1.into(),0.into()),Q::one()],d,Default::default()).is_err());
}
