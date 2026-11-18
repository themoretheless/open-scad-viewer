use super::*;
    #[test]
    fn nonnegative_factor_matches_original_interval_bits() {
        let bits=|x:Interval|[x.lo.to_bits(),x.hi.to_bits()];
        let values=[-f64::MAX,-1e200,-1.,-f64::MIN_POSITIVE,-f64::from_bits(1),-0.,0.,f64::from_bits(1),f64::MIN_POSITIVE,1.,1e200,f64::MAX];
        for &lo in &values {for &hi in &values {if lo>hi {continue}
            for factor in [[0.,0.],[0.,1.],[0.125,0.875],[1.,1.],[f64::from_bits(1),f64::MIN_POSITIVE]] {
                let a=Interval{lo,hi};let b=Interval{lo:factor[0],hi:factor[1]};
                assert_eq!(a.mul(b).map(bits),mul_blend_factor(a,b).map(bits));
            }
        }}
        let mut seed=0x784abc3712d051f3_u64;
        for _ in 0..1000000 {
            let mut next=||{seed^=seed<<13;seed^=seed>>7;seed^=seed<<17;f64::from_bits(seed)};
            let x=next();let y=next();let z=next().abs();let w=next().abs();
            if ![x,y,z,w].iter().all(|v|v.is_finite()){continue}
            let a=Interval{lo:x.min(y),hi:x.max(y)};let b=Interval{lo:z.min(w),hi:z.max(w)};
            assert_eq!(a.mul(b).map(bits),mul_blend_factor(a,b).map(bits));
        }
    }
