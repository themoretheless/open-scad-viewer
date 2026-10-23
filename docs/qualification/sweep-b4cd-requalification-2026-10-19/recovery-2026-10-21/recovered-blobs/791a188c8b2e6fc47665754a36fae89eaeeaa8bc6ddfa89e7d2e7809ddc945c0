//! Outward binary64 composition of certified nonnegative error bounds.
//! These operations compose premises; they do not establish their provenance.
fn valid(x:f64)->bool{x.is_finite()&&x>=0.}
fn successor(x:f64)->Option<f64>{
    if !valid(x){return None;}
    let upper=f64::from_bits(x.to_bits()+1);
    upper.is_finite().then_some(upper)
}
/// A zero contribution is exact; otherwise one successor encloses nearest addition.
pub fn add(a:f64,b:f64)->Option<f64>{
    if !valid(a)||!valid(b){return None;}
    if a==0.{return Some(b);} if b==0.{return Some(a);}
    successor(a+b)
}
/// One successor encloses nearest multiplication, including subnormal products.
pub fn multiply(a:f64,b:f64)->Option<f64>{
    if !valid(a)||!valid(b){return None;}
    if a==0.||b==0.{return Some(0.);}
    successor(a*b)
}
/// 6369051672525773 / 2^52 has exact square greater than two.
pub fn sqrt_two(value:f64)->Option<f64>{multiply(value,1.4142135623730951)}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn error_upper_preserves_zero_and_refuses_invalid_or_overflow(){
        assert_eq!(add(1.,2f64.powi(-53)),Some(1.+2f64.powi(-52)));
        let tiny=f64::from_bits(1);
        assert_eq!(add(0.,tiny),Some(tiny));assert_eq!(add(tiny,tiny),Some(3.*tiny));
        assert_eq!(add(f64::MAX,0.),Some(f64::MAX));assert_eq!(add(f64::MAX,f64::MAX),None);
        assert_eq!(multiply(tiny,tiny),Some(tiny));assert_eq!(multiply(0.,f64::MAX),Some(0.));
        assert_eq!(multiply(f64::MAX,2.),None);
        for bad in [-1.,f64::NAN,f64::INFINITY]{assert_eq!(add(bad,1.),None);assert_eq!(multiply(1.,bad),None);assert_eq!(sqrt_two(bad),None);}
    }
    #[test]
    fn sqrt_two_upper_has_independent_exact_squared_oracle(){
        let coefficient=6369051672525773u128;let denominator=1u128<<52;
        assert!(coefficient*coefficient>2*denominator*denominator);
        fn dyadic(x:f64)->(u128,i32){let bits=x.to_bits();let exponent=((bits>>52)&2047) as i32;
            let fraction=(bits&((1u64<<52)-1)) as u128;
            if exponent==0{(fraction,-1074)}else{((1u128<<52)|fraction,exponent-1075)}}
        for x in [f64::from_bits(1),f64::from_bits(3),f64::MIN_POSITIVE,0.25,1.,1.+f64::EPSILON,2f64.powi(500),f64::MAX/2.]{
            let upper=sqrt_two(x).unwrap();let(a,e)=dyadic(x);let(b,f)=dyadic(upper);let shift=2*(f-e);
            if shift>=0{assert!((b*b)<<shift>=2*a*a);}else{assert!(b*b>=(2*a*a)<<(-shift));}
        }
        assert_eq!(sqrt_two(0.),Some(0.));assert_eq!(sqrt_two(f64::MAX),None);
    }
}
