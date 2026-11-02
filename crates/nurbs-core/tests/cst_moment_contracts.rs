use nurbs_core::construct::curves::airfoil::{CstProfile,cst_algebraic,cst_section_moments};
fn profile(n1:f64,upper:f64,lower:f64)->CstProfile {CstProfile{n1,n2:1.,upper:vec![upper;2],lower:vec![lower;2]}}
fn ratio(bounds:[f64;2],n:i128,d:i128) {
    if n==0 {assert!(bounds[0]<=0.&&bounds[1]>=0.);return;}
    let rational=|x:f64| {
        let bits=x.to_bits();let e=((bits>>52)&2047) as i32-1023-52;
        let m=((bits&((1u64<<52)-1))|(1u64<<52)) as i128;
        let m=if bits>>63==0 {m}else{-m};assert!((-120..70).contains(&e));
        if e>=0 {(m<<e,1)}else{(m,1i128<<(-e))}
    };
    let (a,b)=rational(bounds[0]);assert!(a*d<=n*b,"{bounds:?} excludes {n}/{d}");
    let (a,b)=rational(bounds[1]);assert!(a*d>=n*b,"{bounds:?} excludes {n}/{d}");
}
#[test]
fn symmetric_round_nose_has_analytic_origin_and_central_moments() {
    let m=cst_algebraic(&profile(0.5,0.125,-0.125),1.,1_000_000).unwrap();
    let r=cst_section_moments(&m,1_000_000).unwrap();
    // Direct beta integrals of y², x², xy over the original section.
    for (bounds,(n,d)) in r.about_origin.into_iter().zip([(1,27720),(1,63),(0,1)]) {ratio(bounds,n,d);}
    for (bounds,(n,d)) in r.about_centroid.into_iter().zip([(1,27720),(8,2205),(0,1)]) {ratio(bounds,n,d);}
    assert!(r.about_origin[0][1]-r.about_origin[0][0]<1e-15);
}
#[test]
fn cambered_section_uses_signed_product_and_parallel_axis_theorem() {
    let m=cst_algebraic(&profile(0.5,0.25,0.125),1.,1_000_000).unwrap();
    let r=cst_section_moments(&m,1_000_000).unwrap();
    for (bounds,(n,d)) in r.about_origin.into_iter().zip([(1,7920),(1,126),(1,1280)]) {ratio(bounds,n,d);}
    for (bounds,(n,d)) in r.about_centroid.into_iter().zip([(767,64880640),(4,2205),(-1,17920)]) {ratio(bounds,n,d);}
    let m=cst_algebraic(&profile(0.5,-0.125,-0.25),1.,1_000_000).unwrap();
    let mirror=cst_section_moments(&m,1_000_000).unwrap();
    ratio(mirror.about_origin[2],-1,1280);ratio(mirror.about_centroid[2],1,17920);
}
#[test]
fn integer_exponents_and_chord_scaling_follow_fourth_power_law() {
    for (chord,scale) in [(0.5,1),(1.,16),(2.,256)] {
        let m=cst_algebraic(&profile(1.,0.5,0.25),chord,1_000_000).unwrap();
        let r=cst_section_moments(&m,1_000_000).unwrap();
        for (bounds,(n,d)) in r.about_origin.into_iter().zip([(1,3840),(1,80),(1,640)]) {ratio(bounds,n*scale,d*16);}
        for (bounds,(n,d)) in r.about_centroid.into_iter().zip([(1,38400),(1,480),(0,1)]) {ratio(bounds,n*scale,d*16);}
    }
}
#[test]
fn work_boundaries_and_nonphysical_sources_refuse() {
    let m=cst_algebraic(&profile(0.5,0.25,0.125),1.,1_000_000).unwrap();
    let r=cst_section_moments(&m,1_000_000).unwrap();
    assert!(cst_section_moments(&m,r.work_used-1).is_err());
    assert_eq!(cst_section_moments(&m,r.work_used).unwrap().about_origin,r.about_origin);
    for budget in [0,1,1_000_001] {assert!(cst_section_moments(&m,budget).is_err());}
    for p in [profile(0.5,0.125,0.125),profile(0.5,-0.125,0.125)] {
        let m=cst_algebraic(&p,1.,1_000_000).unwrap();assert!(cst_section_moments(&m,1_000_000).is_err());
    }
    let m=cst_algebraic(&profile(0.5,0.125,-0.125),1e-200,1_000_000).unwrap();assert!(cst_section_moments(&m,1_000_000).is_err());
}
#[test]
fn degree_twenty_five_source_does_not_overflow_cubic_binomial_products() {
    let p=CstProfile{n1:0.5,n2:1.,upper:(0..12).map(|i|0.125+i as f64/32.).collect(),lower:vec![-0.125;12]};
    let m=cst_algebraic(&p,1.,1_000_000).unwrap();assert_eq!(m.curve().degree,25);
    let r=cst_section_moments(&m,1_000_000).unwrap();assert!(r.work_used<=1_000_000);
    // Shape coefficients reduce analytically to 1/8+(11/32)x.
    // These exact beta integrals do not use the constructed NURBS poles.
    for (bounds,(n,d)) in r.about_origin.into_iter().zip([(3527,17425408),(5,168),(55,43008)]) {ratio(bounds,n,d);}
    for (bounds,(n,d)) in r.about_centroid.into_iter().zip([(9355010923,63523279339520),(487,84105),(9889,76554240)]) {ratio(bounds,n,d);}
    for bounds in r.about_centroid {assert!(bounds[1]-bounds[0]<1e-9);}
}

#[test]
fn representable_small_and_large_chords_keep_dimensional_bounds() {
    for exponent in [-20,20] {
        let chord=2_f64.powi(exponent);let scale=2_f64.powi(4*exponent);
        let m=cst_algebraic(&profile(0.5,0.125,-0.125),chord,1_000_000).unwrap();
        let r=cst_section_moments(&m,1_000_000).unwrap();
        ratio(r.about_origin[0].map(|x|x/scale),1,27720);
        ratio(r.about_centroid[1].map(|x|x/scale),8,2205);
    }
    let crossed=CstProfile{n1:0.5,n2:1.,upper:vec![0.25,0.],lower:vec![0.,0.125]};
    let m=cst_algebraic(&crossed,1.,1_000_000).unwrap();assert!(cst_section_moments(&m,1_000_000).is_err());
}
