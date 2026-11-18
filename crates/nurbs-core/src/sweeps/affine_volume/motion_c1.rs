//! Exact C1 premise for the ORIGINAL equal-duration affine motion pieces.
//! This proves all twelve Cartesian coefficient derivatives, including T.
//! Positive rational weights make each piece smooth; exact endpoint values
//! and first jets glue them to a C1 map of the complete closed time interval.
use super::*;
use num_rational::BigRational as Q;
fn q(x: f64) -> Q { Q::from_float(x).expect("validated finite source coefficient") }
fn weight(motion: Motion<'_>, piece: usize, control: usize) -> f64 {
    match motion {
        Motion::Quintic(_) => 1.,
        Motion::Rational(p) => p[piece].weights[control],
    }
}
/// `None` means the exact-work ledger was exhausted, not a failed C1 join.
/// A comparison reserves 24 units: 8 scalar conversions, 2 degree conversions,
/// 2 differences, 6 products, and the endpoint/jet comparisons (with slack).
/// Degrees and source bit sizes are independently bounded by Motion::valid.
/// Equal station widths cancel the common global-time factor `pieces.len()`.
pub(super) fn inspect(motion: Motion<'_>, work: &mut u64, max: u64) -> Option<bool> {
    for i in 1..motion.len() {
        let a = motion.controls(i-1); let b = motion.controls(i);
        let n = a.len()-1; let m = b.len()-1;
        for col in 0..4 { for axis in 0..3 {
            if max.saturating_sub(*work)<24 { *work=max; return None; }
            *work+=24;
            if a[n][col][axis] != b[0][col][axis] {return Some(false);}
            // n*wa[n-1]/wa[n]*(a[n]-a[n-1]) equals
            // m*wb[1]/wb[0]*(b[1]-b[0]), without rounded derivatives
            // or division. All weights are strictly positive and source-owned.
            let left=Q::from_integer(n.into()) * q(weight(motion,i-1,n-1))
                * q(weight(motion,i,0)) * (q(a[n][col][axis])-q(a[n-1][col][axis]));
            let right=Q::from_integer(m.into()) * q(weight(motion,i,1))
                * q(weight(motion,i-1,n)) * (q(b[1][col][axis])-q(b[0][col][axis]));
            if left!=right {return Some(false);}
        }}
    }
    Some(true)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_weights_require_exact_cartesian_time_jets_and_budget() {
        let pose=|z|[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.],[0.,0.,z]];
        let mut p=vec![RationalPiece{controls:vec![pose(0.),pose(1.)],weights:vec![1.,2.]},
            RationalPiece{controls:vec![pose(1.),pose(2.)],weights:vec![2.,1.]}];
        let mut work=0;assert_eq!(inspect(Motion::Rational(&p),&mut work,1000),Some(true));
        assert_eq!(work,288);
        let mut spent=0;assert_eq!(inspect(Motion::Rational(&p),&mut spent,work),Some(true));
        let mut spent=0;assert_eq!(inspect(Motion::Rational(&p),&mut spent,work-1),None);
        assert_eq!(spent,work-1);
        p[1].weights[1]=1f64.next_up();
        assert_eq!(inspect(Motion::Rational(&p),&mut 0,1000),Some(false));
        p[1].weights[1]=1.;p[1].controls[0][3][2]=1f64.next_up();
        assert_eq!(inspect(Motion::Rational(&p),&mut 0,1000),Some(false));
    }
}
