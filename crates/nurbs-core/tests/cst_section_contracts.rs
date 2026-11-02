use nurbs_core::construct::curves::airfoil::{CstProfile,cst_algebraic,cst_section_properties};
fn profile(upper:f64,lower:f64)->CstProfile {CstProfile{n1:0.5,n2:1.,upper:vec![upper;2],lower:vec![lower;2]}}
fn contains(bounds:[f64;2],value:f64) {assert!(bounds[0]<=value&&value<=bounds[1],"{bounds:?} excludes {value}");}
fn contains_ratio(bounds:[f64;2],numerator:i128,denominator:i128) {
    // Compare each endpoint as an exact binary rational, without rounding
    // the independent expected rational to a floating-point test oracle.
    let rational=|x:f64| {
        let bits=x.to_bits();let exponent=((bits>>52)&2047) as i32-1023-52;
        let mantissa=((bits&((1u64<<52)-1))|(1u64<<52)) as i128;
        let mantissa=if bits>>63==0 {mantissa}else{-mantissa};
        assert!(exponent> -120&&exponent<70);
        if exponent>=0 {(mantissa<<exponent,1)}else{(mantissa,1i128<<(-exponent))}
    };
    let (a,b)=rational(bounds[0]);assert!(a*denominator<=numerator*b);
    let (a,b)=rational(bounds[1]);assert!(a*denominator>=numerator*b);
}
#[test]
fn symmetric_round_nose_area_and_centroid_follow_independent_beta_integrals() {
    let m=cst_algebraic(&profile(0.125,-0.125),1.,1_000_000).unwrap();
    let p=cst_section_properties(&m,1_000_000).unwrap();
    // h=x^(1/2)(1-x)/4: area=1/15, x centroid=(3/2)/(7/2).
    contains_ratio(p.area,1,15);contains_ratio(p.centroid[0],3,7);contains(p.centroid[1],0.);
    assert!(p.area[1]-p.area[0]<1e-13);
}
#[test]
fn cambered_same_side_profile_has_nonzero_first_ordinate_moment() {
    let m=cst_algebraic(&profile(0.25,0.125),1.,1_000_000).unwrap();
    let p=cst_section_properties(&m,1_000_000).unwrap();
    // Integral (upper²-lower²)/2 = (3/128)*B(2,3)=1/512.
    contains_ratio(p.area,1,30);contains_ratio(p.centroid[0],3,7);contains_ratio(p.centroid[1],15,256);
    let mirror=cst_algebraic(&profile(-0.125,-0.25),1.,1_000_000).unwrap();
    let r=cst_section_properties(&mirror,1_000_000).unwrap();
    contains_ratio(r.area,1,30);contains_ratio(r.centroid[1],-15,256);
}
#[test]
fn integer_exponent_polynomial_and_world_scale_obey_exact_moment_laws() {
    for chord in [0.125,2.,1e6] {
        let mut source=profile(0.5,0.25);source.n1=1.;
        let m=cst_algebraic(&source,chord,1_000_000).unwrap();
        let p=cst_section_properties(&m,1_000_000).unwrap();
        // h=x(1-x)/4, ordinate moment=(3/32)*B(3,3)=1/320.
        contains(p.area,chord*chord/24.);contains(p.centroid[0],chord/2.);
        contains(p.centroid[1],chord*3./40.);
    }
}
#[test]
fn varying_coefficients_keep_source_geometry_and_full_shared_work() {
    let mut source=profile(0.25,-0.125);source.upper=vec![0.25,0.5];source.lower=vec![-0.125,0.125];
    let m=cst_algebraic(&source,1.,1_000_000).unwrap();
    let p=cst_section_properties(&m,1_000_000).unwrap();
    // Thickness shape is (3/8)*(1-x)+(3/8)*x, so area=1/10.
    contains(p.area,1./10.);contains(p.centroid[0],3./7.);
    contains_ratio(p.centroid[1],13,256);assert!(p.work_used>1);
    assert!(cst_section_properties(&m,p.work_used-1).is_err());
    assert_eq!(cst_section_properties(&m,p.work_used).unwrap().area,p.area);
    assert_eq!(m.source().upper,source.upper);
}
#[test]
fn collapsed_crossed_and_unrepresentable_sections_refuse() {
    for source in [profile(0.125,0.125),profile(-0.125,0.125),CstProfile{n1:0.5,n2:1.,upper:vec![0.25,0.],lower:vec![0.,0.125]}] {
        let m=cst_algebraic(&source,1.,1_000_000).unwrap();assert!(cst_section_properties(&m,1_000_000).is_err());
    }
    let m=cst_algebraic(&profile(0.125,-0.125),1e-200,1_000_000).unwrap();
    assert!(cst_section_properties(&m,1_000_000).is_err());
    let m=cst_algebraic(&profile(0.125,-0.125),1.,1_000_000).unwrap();
    for budget in [0,1,1_000_001] {assert!(cst_section_properties(&m,budget).is_err());}
}
#[test]
fn full_degree_nonconstant_section_keeps_finite_positive_area() {
    let source=CstProfile{n1:0.5,n2:1.,upper:(0..12).map(|i|0.125+i as f64/32.).collect(),lower:vec![-0.125;12]};
    let m=cst_algebraic(&source,1e9,1_000_000).unwrap();assert_eq!(m.curve().degree,25);
    let p=cst_section_properties(&m,1_000_000).unwrap();assert!(p.area[0]>0.&&p.area[1].is_finite());
    for axis in p.centroid {assert!(axis[0].is_finite()&&axis[1].is_finite()&&axis[0]<=axis[1]);}
    // Independent reduction: Bernstein coefficients a_i=1/8+i/32
    // represent 1/8+(11/32)*x, despite retaining shape order eleven.
    contains_ratio(p.area,89*1_000_000_000_000_000_000,840);
    contains_ratio(p.centroid[0],127*1_000_000_000,267);
    contains_ratio(p.centroid[1],2079*1_000_000_000,91136);
}
