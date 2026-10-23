//! Exact plane premise from represented original rational source coefficients.
//! No path regularity, Bishop transport, retained error, or closed seam claim.
use crate::{Result, check, curve::Curve};

#[derive(Clone, Debug)]
pub(super) struct Report {
    pub proved: bool,
    pub exact_work: u64,
    pub reason: Option<&'static str>,
}

/// Positive source weights extend each pole-plane identity to the entire
/// rational curve. Directions are formed by exact predicates from input bits;
/// rounded subtraction, fitted planes, and sampled points are not premises.
pub(super) fn certify(path: &Curve, normal: [f64; 3], max_work: u64) -> Result<Report> {
    path.validate()?;
    check(
        path.control_points.iter().all(|p| p.len() == 3) && normal.iter().all(|x| x.is_finite()),
        "Source plane requires finite XYZ coefficients and normal",
    )?;
    check(
        max_work <= 1000000,
        "Source plane exact budget exceeds1000000",
    )?;
    let mut out = Report {
        proved: false,
        exact_work: 0,
        reason: Some("source-plane-exact-work-unproved"),
    };
    if normal.iter().all(|x| *x == 0.) {
        out.reason = Some("source-plane-normal-degenerate");
        return Ok(out);
    }
    if max_work == 0 {
        return Ok(out);
    }
    // Preserve the existing finite original-coefficient admission capacity.
    // The source plane uses an exact algebraic zero, not a fitted point or
    // a rounded sum of the original normal and path origin.
    if path.control_points.len() > (65536 - 4) / 3 {
        out.reason = Some("source-plane-snapshot-unproved");
        return Ok(out);
    }
    // Binary64 coefficients are exact dyadic leaves. Expand the original
    // normal dot (pole-origin) into six products before any subtraction;
    // the fixed-size dyadic accumulator preserves cancellation/underflow.
    // Charge each original product, including failed/nonzero comparisons.
    let origin = &path.control_points[0];
    for pole in path.control_points.iter().skip(1) {
        if max_work - out.exact_work < 6 {
            return Ok(out);
        }
        out.exact_work += 6;
        let terms = std::array::from_fn::<_, 6, _>(|k| {
            if k < 3 {
                (normal[k], pole[k])
            } else {
                (-normal[k - 3], origin[k - 3])
            }
        });
        if crate::exact_products::sum_sign(&terms) != std::cmp::Ordering::Equal {
            out.reason = Some("source-plane-coefficients-nonzero");
            return Ok(out);
        }
    }
    out.proved = true;
    out.reason = None;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source() -> Curve {
        Curve {
            degree: 2,
            knots: vec![2., 2., 2., 5., 5., 5.],
            control_points: vec![vec![0.; 3], vec![0.5, -0.5, 0.], vec![1., -2., 1.]],
            weights: vec![1., 0.75, 1.25],
            periodic: false,
        }
    }
    #[test]
    fn nonaxial_original_rational_plane_and_exact_budget_are_separate() {
        let path = source();
        let proof = certify(&path, [1.; 3], 1000000).unwrap();
        assert!(proof.proved, "{proof:?}");
        assert!(proof.exact_work > 0);
        // Independent rational Bernstein equation on the nonunit domain.
        for t in [0., 0.125, 0.5, 0.875, 1.] {
            let b = [(1. - t) * (1. - t), 2. * t * (1. - t), t * t];
            let w = b[0] + 0.75 * b[1] + 1.25 * b[2];
            let point = [
                (0.375 * b[1] + 1.25 * b[2]) / w,
                (-0.375 * b[1] - 2.5 * b[2]) / w,
                1.25 * b[2] / w,
            ];
            assert!((point.iter().sum::<f64>()).abs() < 1e-14);
        }
        let short = certify(&path, [1.; 3], proof.exact_work - 1).unwrap();
        assert!(!short.proved);
        assert!(short.exact_work <= proof.exact_work - 1);
        let zero = certify(&path, [1.; 3], 0).unwrap();
        assert!(!zero.proved);
        assert_eq!(zero.exact_work, 0);
    }
    #[test]
    fn a_single_binary64_step_off_plane_is_not_a_tolerance_premise() {
        let mut path = source();
        path.control_points[2][2] = 1_f64.next_up();
        let proof = certify(&path, [1.; 3], 1000000).unwrap();
        assert!(!proof.proved);
        assert_eq!(proof.reason, Some("source-plane-coefficients-nonzero"));
        let zero = certify(&source(), [0.; 3], 1000000).unwrap();
        assert!(!zero.proved);
    }
    #[test]
    fn plane_does_not_claim_regular_transport_and_invalid_inputs_stay_errors() {
        let mut constant = source();
        constant.control_points = vec![vec![3., 7., 11.]; 3];
        assert!(certify(&constant, [1.; 3], 1000000).unwrap().proved);
        assert!(certify(&source(), [f64::NAN, 1., 1.], 0).is_err());
        assert!(certify(&source(), [1.; 3], 1000001).is_err());
    }
}
