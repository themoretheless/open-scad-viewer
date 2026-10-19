//! Exact original-column identities. Proposal division never supplies proof.
use super::RationalPiece;
use crate::numerics::compensated::two_sum;

#[derive(Clone, Debug)]
pub struct ColumnFactor {
    pub base: usize,
    pub dependent: usize,
    pub scale: f64,
    pub constant: [f64; 3],
}
fn multiply(a: f64, b: f64) -> Option<f64> {
    if a == 0. || b == 0. {
        return Some(0.);
    }
    if a == 1. {
        return Some(b);
    }
    if b == 1. {
        return Some(a);
    }
    let p = a * b;
    // FMA residual equality is conclusive away from gradual underflow only.
    (p.is_finite()
        && a.abs() >= 1e-100
        && b.abs() >= 1e-100
        && p.abs() >= 1e-100
        && a.mul_add(b, -p) == 0.)
        .then_some(p)
}
fn subtract(a: f64, b: f64) -> Option<f64> {
    let (s, e) = two_sum(a, -b);
    (s.is_finite() && e == 0.).then_some(s)
}
fn residual(a: [f64; 3], b: [f64; 3], scale: f64) -> Option<[f64; 3]> {
    Some([
        subtract(b[0], multiply(scale, a[0])?)?,
        subtract(b[1], multiply(scale, a[1])?)?,
        subtract(b[2], multiply(scale, a[2])?)?,
    ])
}
/// Every failed exact scalar comparison consumes the same source work ledger.
/// Exhaustion stops the owner, rather than silently falling back with less work.
pub(crate) fn prove_factor(
    pieces: &[RationalPiece],
    work: &mut u64,
    max: u64,
) -> std::result::Result<Option<ColumnFactor>, ()> {
    let first = pieces[0].controls[0];
    for base in 0..3 {
        for dependent in 0..3 {
            if base == dependent {
                continue;
            }
            let proposal = pieces.iter().flat_map(|p| &p.controls).find_map(|p| {
                (0..3).find_map(|k| {
                    let d = p[base][k] - first[base][k];
                    if d == 0. {
                        return None;
                    }
                    let scale = (p[dependent][k] - first[dependent][k]) / d;
                    (scale.is_finite() && scale != 0. && scale.abs() <= 1e9).then_some(scale)
                })
            });
            let Some(scale) = proposal else {
                continue;
            };
            let Some(constant) = residual(first[base], first[dependent], scale) else {
                continue;
            };
            let mut valid = true;
            'check: for p in pieces.iter().flat_map(|p| &p.controls) {
                for k in 0..3 {
                    if *work == max {
                        return Err(());
                    }
                    *work += 1;
                    let value =
                        multiply(scale, p[base][k]).and_then(|v| subtract(p[dependent][k], v));
                    if value != Some(constant[k]) {
                        valid = false;
                        break 'check;
                    }
                }
            }
            if valid {
                return Ok(Some(ColumnFactor {
                    base,
                    dependent,
                    scale,
                    constant,
                }));
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_identity_refuses_one_ulp_tail_mutation_and_short_proof_work() {
        let mut p = RationalPiece {
            controls: (0..3)
                .map(|i| {
                    let x = i as f64;
                    [
                        [x, x + 1., 2. * x],
                        [x / 8. + 2., (x + 1.) / 8. + 3., x / 4. + 4.],
                        [0.; 3],
                        [0.; 3],
                    ]
                })
                .collect(),
            weights: vec![1., 0.5, 1.],
        };
        let saved = p.controls.clone();
        let mut work = 0;
        let f = prove_factor(&[p.clone()], &mut work, 1000)
            .unwrap()
            .unwrap();
        assert_eq!(
            (f.base, f.dependent, f.scale, f.constant),
            (0, 1, 0.125, [2., 3., 4.])
        );
        for max in [0, work - 1] {
            let mut used = 0;
            assert!(prove_factor(&[p.clone()], &mut used, max).is_err());
            assert_eq!(used, max);
        }
        assert_eq!(p.controls, saved);
        p.controls[2][1][2] = p.controls[2][1][2].next_up();
        assert!(prove_factor(&[p], &mut 0, 1000).unwrap().is_none());
    }
    #[test]
    fn exact_factor_arithmetic_never_certifies_underflow_as_zero() {
        let tiny = f64::from_bits(1);
        assert_eq!(multiply(tiny, 1.), Some(tiny));
        assert_eq!(multiply(tiny, tiny), None);
        assert_eq!(multiply(1e-200, 1e100), None);
        assert_eq!(multiply(0.125, 4.), Some(0.5));
        assert_eq!(subtract(1., tiny), None);
        assert_eq!(subtract(tiny, 0.), Some(tiny));
    }
}
