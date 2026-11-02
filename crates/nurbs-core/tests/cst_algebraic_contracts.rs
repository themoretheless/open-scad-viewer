use nurbs_core::construct::curves::airfoil::{CstProfile,cst_algebraic,cst_shape};
fn profile()->CstProfile {CstProfile{n1:0.5,n2:1.,upper:vec![0.125,0.125],lower:vec![-0.125,-0.125]}}
fn rational(x:f64)->(i128,u128) {
    if x==0. {return (0,1);}
    let b=x.to_bits();let mut m=(b&((1u64<<52)-1))|(1u64<<52);let mut e=((b>>52)&2047) as i32-1023-52;
    while m&1==0 {m>>=1;e+=1;}
    let sign=if b>>63==0 {1i128}else{-1};
    if e>=0 {(sign*((m as i128)<<e),1)}else{assert!(-e<128);(sign*m as i128,1u128<<(-e))}
}
#[test]
fn enclosure_covers_independent_exact_rational_controls() {
    let mut p=profile();p.upper[1]=0.25;p.lower[1]=-0.25;
    let r=cst_algebraic(&p,1.,1_000_000).unwrap();
    // Independent degree-five control table for x=t², y=(t-t^5)/8.
    let x=[(0,1),(0,1),(1,10),(3,10),(3,5),(1,1)];
    let y=[(0,1),(1,40),(1,20),(3,40),(1,10),(0,1)];
    let (bn,bd)=rational(r.max_error());assert!(bn>0);
    for i in 0..11 {
        let (a,sgn)=if i<=5 {(5-i,1)}else{(i-5,-1)};
        for (axis,(n,d)) in [(0,x[a]),(1,(sgn*y[a].0,y[a].1))] {
            let (pn,pd)=rational(r.curve().control_points[i][axis]);
            let diff=(pn*d as i128-n as i128*pd as i128).unsigned_abs();
            assert!(diff*bd<=(bn as u128)*pd*(d as u128));
        }
    }
    assert_eq!(r.curve().degree,5);assert_eq!(r.curve().control_points.len(),11);
}
#[test]
fn direct_spans_follow_the_declared_source_parameterization() {
    for (n1,n2,upper) in [(0.5,1.,vec![0.2,0.3,0.1]),(1.5,2.,vec![0.3,-0.1,0.2]),(0.5,1.,vec![0.1;12]),(1.,1.,vec![0.125;2])] {
        let p=CstProfile{n1,n2,lower:upper.iter().map(|a|-a).collect(),upper};let r=cst_algebraic(&p,2.5,1_000_000).unwrap();
        assert!(r.max_error()<1e-10);
        for i in 0..=256 {
            let u=2.*i as f64/256.;let (t,c)=if u<=1. {(1.-u,&p.upper)}else{(u-1.,&p.lower)};
            let x=t.powi(r.parameter_power() as i32);let expected=[2.5*x,2.5*cst_shape(x,p.n1,p.n2,c),0.];
            let actual=r.curve().evaluate(u).unwrap().point;
            let delta:f64=actual.iter().zip(expected).map(|(a,b)|(a-b).abs()).sum();
            // Evaluation roundoff is separate from the nominal control bound.
            assert!(delta<=r.max_error()+64.*f64::EPSILON*2.5);
        }
        assert_eq!(r.curve().control_points.first(),r.curve().control_points.last());
        assert_eq!(r.chord(),2.5);assert_eq!(r.source().upper,p.upper);
    }
}
#[test]
fn one_shared_budget_is_enforced_and_source_is_unchanged() {
    let p=profile();let original=p.upper.clone();
    assert!(cst_algebraic(&p,1.,1).is_err());
    let full=cst_algebraic(&p,1.,1_000_000).unwrap();
    assert!(full.work_used()>1);
    assert!(cst_algebraic(&p,1.,full.work_used()-1).is_err());
    assert!(cst_algebraic(&p,1.,full.work_used()).is_ok());
    assert_eq!(p.upper,original);
}
#[test]
fn unsupported_exponents_degrees_and_invalid_options_are_refused() {
    for (n1,n2) in [(0.3,1.),(0.5,0.7),(12.5,1.),(f64::NAN,1.)] {
        let mut p=profile();p.n1=n1;p.n2=n2;assert!(cst_algebraic(&p,1.,1_000_000).is_err());
    }
    for chord in [0.,-1.,f64::NAN,f64::INFINITY,1e10] {assert!(cst_algebraic(&profile(),chord,1_000_000).is_err());}
    for budget in [0,1_000_001] {assert!(cst_algebraic(&profile(),1.,budget).is_err());}
    let mut p=profile();p.upper[0]=f64::NAN;assert!(cst_algebraic(&p,1.,1_000_000).is_err());
}

#[test]
fn redundant_constant_shape_rows_collapse_before_control_construction() {
    let mut p=profile();p.upper=vec![0.125;12];p.lower=vec![-0.125;12];
    let r=cst_algebraic(&p,1.,1_000_000).unwrap();
    assert_eq!(r.curve().degree,3);assert_eq!(r.curve().control_points.len(),7);
    assert_eq!(r.source().upper.len(),12);
    let same=cst_algebraic(&profile(),1.,1_000_000).unwrap();
    assert_eq!(r.curve().control_points,same.curve().control_points);
}

#[test]
fn integer_exponents_use_the_lower_degree_linear_chord_chart() {
    let mut p=profile();p.n1=1.;
    let r=cst_algebraic(&p,1.,1_000_000).unwrap();
    assert_eq!(r.parameter_power(),1);assert_eq!(r.curve().degree,2);
    assert_eq!(r.curve().control_points.len(),5);
    let expected=[[1.,0.,0.],[0.5,0.0625,0.],[0.,0.,0.],[0.5,-0.0625,0.],[1.,0.,0.]];
    for (a,b) in r.curve().control_points.iter().zip(expected) {for (a,b) in a.iter().zip(b) {assert!((a-b).abs()<=r.max_error());}}
}

#[test]
fn maximum_degree_and_coordinate_scale_keep_the_nominal_enclosure_finite() {
    let upper:Vec<f64>=(0..12).map(|i|if i%2==0 {0.75}else{-0.25}).collect();
    let p=CstProfile{n1:0.5,n2:1.,lower:upper.iter().map(|a|-a).collect(),upper};
    for chord in [1e-200,1.,1e9] {
        let r=cst_algebraic(&p,chord,1_000_000).unwrap();
        assert_eq!(r.curve().degree,25);assert_eq!(r.curve().control_points.len(),51);
        assert!(r.max_error().is_finite());assert!(r.max_error()<chord*1e-12);
        for i in 0..=64 {
            let t=i as f64/64.;let x=t*t;
            let expected=[chord*x,chord*cst_shape(x,p.n1,p.n2,&p.lower),0.];
            let actual=r.curve().evaluate(1.+t).unwrap().point;
            let error:f64=actual.iter().zip(expected).map(|(a,b)|(a-b).abs()).sum();
            assert!(error<=r.max_error()+256.*f64::EPSILON*chord);
        }
    }
}
