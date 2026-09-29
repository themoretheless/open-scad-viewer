use cad_predicates::{AuthoredScalar, BezierIdentity, Limits, PredicateContext, SourceArena, ToleranceContext, rational_bezier_identity};
use value_codec::json;
fn main() {
    let mut cases = Vec::new();
    let mut check = |a: Vec<[f64;4]>, b: Vec<[f64;4]>| {
        let source = SourceArena::authored("identity-corpus", 1, a.iter().chain(&b).flatten().map(|v| AuthoredScalar::Binary64Bits(v.to_bits())).collect()).unwrap();
        let refs = |start: usize, len: usize| (start..start+len).map(|i| std::array::from_fn(|k| source.leaf(i*4+k).unwrap())).collect::<Vec<_>>();
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&source, &tolerance, Limits::default(), None);
        let result = rational_bezier_identity(&mut ctx, &refs(0,a.len()), &refs(a.len(),b.len())).unwrap();
        let outcome = match result.outcome { BezierIdentity::Equal => "equal", BezierIdentity::Different => "different", BezierIdentity::Indeterminate(_) => "indeterminate" };
        cases.push(json!({"a":a,"b":b,"outcome":outcome}));
    };
    for axis in 0..3 {
        for scale in [0.125, 1., 8.] {
            for shift in [0., 16.] {
                let transform = |mut p:[f64;4]| { let old=p; for k in 0..3 { p[(axis+k)%3]=old[k]*scale+shift; } p };
                let a = [[0.,0.,0.,3.],[5.,10.,2.5,6.],[10.,0.,5.,3.]].map(transform).to_vec();
                let b = [[0.,0.,0.,3.],[4.,8.,2.,5.],[6.,8.,3.,5.],[10.,0.,5.,3.]].map(transform).to_vec();
                check(a.clone(),b.clone());
                let mut scaled=b.clone(); for p in &mut scaled {p[3]*=4.;} check(a.clone(),scaled);
                let mut perturbed=b.clone(); perturbed[1][axis]=perturbed[1][axis].next_up(); check(a.clone(),perturbed);
                let mut reversed=b; reversed.reverse(); check(a,reversed);
            }
        }
    }
    // Every admitted degree, including large exact binomial products.
    for degree in 1..=32 {
        let a=vec![[1.25,-2.5,4.,1.];2];
        let b=(0..=degree).map(|i| [1.25,-2.5,4.,1.+(i%3) as f64*0.25]).collect::<Vec<_>>();
        check(a.clone(),b.clone());
        let mut bad=b; bad[degree/2][2]=4.000000000000001; check(a,bad);
    }
    // Nonconstant curves at high degrees with exactly representable controls.
    for degree in [2,4,8,16,32] {
        let a=vec![[0.,-2.,1.,1.],[4.,6.,-3.,1.]];
        let b=(0..=degree).map(|i| {let t=i as f64/degree as f64; [4.*t,-2.+8.*t,1.-4.*t,2.]}).collect::<Vec<_>>();
        check(a.clone(),b.clone());
        let mut bad=b; bad[degree/2][0]=bad[degree/2][0].next_up(); check(a,bad);
    }
    println!("{}",json!({"cases":cases}));
}
