//! Numerical proposals for station-strip reparameterization. A proposal never
//! certifies a seam: exact strip identities and regularity remain mandatory.
use crate::{Result, check, surface::Surface};

fn derivative(s: &Surface, boundary: &str) -> Result<Option<[f64; 3]>> {
    s.validate()?;
    check(
        matches!(boundary, "vMin" | "vMax"),
        "Invalid station boundary",
    )?;
    check(
        s.control_points.iter().flatten().all(|p| p.len() == 3),
        "Station scale requires XYZ surfaces",
    )?;
    let n = s.control_points[0].len();
    let v0 = s.knots_v[s.degree_v];
    let v1 = s.knots_v[n];
    if !s.periodic_v
        && n == s.degree_v + 1
        && s.knots_v.len() == 2 * n
        && s.knots_v[..n].iter().all(|&v| v == v0)
        && s.knots_v[n..].iter().all(|&v| v == v1)
        && s.weights
            .iter()
            .all(|row| row[0] > 0. && row.iter().all(|&w| w == row[0]))
    {
        let i = if boundary == "vMin" { 0 } else { n - 2 };
        return Ok(Some(std::array::from_fn(|k| {
            (s.control_points[0][i + 1][k] - s.control_points[0][i][k]) * s.degree_v as f64
        })));
    }
    let u = (s.knots_u[s.degree_u] + s.knots_u[s.control_points.len()]) / 2.;
    let v = if boundary == "vMin" { v0 } else { v1 };
    Ok(s.evaluate(u, v)?
        .first_derivatives()
        .map(|(_, dv)| dv.map(|x| x * (v1 - v0))))
}

pub fn propose_station_normal_scale(a: &Surface, b: &Surface, ab: &str, bb: &str) -> Result<f64> {
    let (Some(da), Some(db)) = (derivative(a, ab)?, derivative(b, bb)?) else {
        return Ok(1.);
    };
    let mut k = 0;
    for i in 1..3 {
        if da[i].abs() > da[k].abs() {
            k = i;
        }
    }
    let ratio = (db[k] / da[k]).abs();
    Ok(if ratio.is_finite() && ratio > 0. {
        ratio
    } else {
        1.
    })
}

fn cross_derivative(s: &Surface, boundary: &str) -> Result<Option<[f64; 3]>> {
    if matches!(boundary, "vMin" | "vMax") {
        return derivative(s, boundary);
    }
    check(
        matches!(boundary, "uMin" | "uMax"),
        "Invalid boundary scale proposal",
    )?;
    s.validate()?;
    // Exact index transposition, with no extraction, fitting or rounding.
    let transposed = Surface {
        degree_u: s.degree_v,
        degree_v: s.degree_u,
        knots_u: s.knots_v.clone(),
        knots_v: s.knots_u.clone(),
        control_points: (0..s.control_points[0].len())
            .map(|v| s.control_points.iter().map(|r| r[v].clone()).collect())
            .collect(),
        weights: (0..s.weights[0].len())
            .map(|v| s.weights.iter().map(|r| r[v]).collect())
            .collect(),
        periodic_u: s.periodic_v,
        periodic_v: s.periodic_u,
    };
    derivative(
        &transposed,
        if boundary == "uMin" { "vMin" } else { "vMax" },
    )
}
pub fn propose_boundary_normal_scale(a: &Surface, b: &Surface, ab: &str, bb: &str) -> Result<f64> {
    let (Some(da), Some(db)) = (cross_derivative(a, ab)?, cross_derivative(b, bb)?) else {
        return Ok(1.);
    };
    let mut k = 0;
    for i in 1..3 {
        if da[i].abs() > da[k].abs() {
            k = i;
        }
    }
    let ratio = (db[k] / da[k]).abs();
    Ok(if ratio.is_finite() && ratio > 0. {
        ratio
    } else {
        1.
    })
}

/// Exact quotient of represented derivative components as a second proposal.
/// It need not equal the exact surface derivative ratio: full coefficient and
/// regularity proofs still decide admission, with no coefficient snapping.
pub(crate) fn propose_boundary_binary_ratio_scalar(
    a: &Surface,
    b: &Surface,
    ab: &str,
    bb: &str,
) -> Result<Option<cad_predicates::AuthoredScalar>> {
    let (Some(da), Some(db)) = (cross_derivative(a, ab)?, cross_derivative(b, bb)?) else {
        return Ok(None);
    };
    let k = (0..3)
        .max_by(|&i, &j| da[i].abs().total_cmp(&da[j].abs()))
        .unwrap();
    fn dyadic(x: f64) -> Option<(u128, i32)> {
        if !x.is_finite() || x == 0. {
            return None;
        }
        let bits = x.abs().to_bits();
        let e = ((bits >> 52) & 2047) as i32;
        let mut m = (bits & ((1u64 << 52) - 1)) as u128;
        let mut exponent = if e == 0 {
            -1074
        } else {
            m |= 1u128 << 52;
            e - 1023 - 52
        };
        let zeros = m.trailing_zeros();
        m >>= zeros;
        exponent += zeros as i32;
        Some((m, exponent))
    }
    let (Some((mut denominator, de)), Some((mut numerator, ne))) = (dyadic(da[k]), dyadic(db[k]))
    else {
        return Ok(None);
    };
    fn gcd(mut a: u128, mut b: u128) -> u128 {
        while b != 0 {
            let r = a % b;
            a = b;
            b = r
        }
        a
    }
    let g = gcd(numerator, denominator);
    numerator /= g;
    denominator /= g;
    let shift = ne - de;
    if shift >= 0 {
        let Some(v) = numerator
            .checked_mul(1u128.checked_shl(shift as u32).unwrap_or(0))
            .filter(|&v| v > 0)
        else {
            return Ok(None);
        };
        numerator = v;
    } else {
        let Some(v) = denominator
            .checked_mul(1u128.checked_shl((-shift) as u32).unwrap_or(0))
            .filter(|&v| v > 0)
        else {
            return Ok(None);
        };
        denominator = v;
    }
    if numerator > i64::MAX as u128 || denominator > u64::MAX as u128 {
        return Ok(None);
    }
    Ok(Some(cad_predicates::AuthoredScalar::RationalConstant {
        numerator: numerator as i64,
        denominator: denominator as u64,
    }))
}

/// Bounded continued-fraction proposal, never a certificate. In particular,
/// rounding near a rational cannot admit a seam: exact identities check the
/// candidate against every original retained coefficient under max_work.
pub(crate) fn propose_station_scale_scalar(ratio: f64) -> cad_predicates::AuthoredScalar {
    use cad_predicates::AuthoredScalar;
    let fallback = AuthoredScalar::Binary64Bits(ratio.to_bits());
    if !ratio.is_finite() || ratio <= 0. {
        return fallback;
    }
    let (mut p0, mut p1, mut q0, mut q1) = (0i128, 1i128, 1i128, 0i128);
    let mut value = ratio;
    for _ in 0..32 {
        let whole = value.floor();
        if !whole.is_finite() || whole > 1_000_000_000_000. {
            break;
        }
        let a = whole as i128;
        let p = a * p1 + p0;
        let q = a * q1 + q0;
        if p > 1_000_000_000_000 || q > 1_000_000 || q <= 0 {
            break;
        }
        if p > 0 && ((p as f64 / q as f64) - ratio).abs() <= 8. * f64::EPSILON * ratio {
            // Keep existing binary proposals when they represent this ratio.
            if (q as u64).is_power_of_two() && p as f64 / q as f64 == ratio {
                return fallback;
            }
            return AuthoredScalar::RationalConstant {
                numerator: p as i64,
                denominator: q as u64,
            };
        }
        (p0, p1, q0, q1) = (p1, p, q1, q);
        let remainder = value - whole;
        if remainder == 0. {
            break;
        }
        value = 1. / remainder;
    }
    fallback
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn binary_ratio_proposal_preserves_finite_dyadics_and_refuses_overflow() {
        let a = strip(1, 1.);
        for (height, numerator, denominator) in [(7., 7, 1), (0.125, 1, 8), (1.5, 3, 2)] {
            assert_eq!(
                propose_boundary_binary_ratio_scalar(&a, &strip(1, height), "vMax", "vMin")
                    .unwrap(),
                Some(cad_predicates::AuthoredScalar::RationalConstant {
                    numerator,
                    denominator
                })
            );
        }
        for height in [0., f64::MIN_POSITIVE, f64::from_bits(1)] {
            assert!(
                propose_boundary_binary_ratio_scalar(&a, &strip(1, height), "vMax", "vMin")
                    .unwrap()
                    .is_none()
            );
        }
        assert!(
            propose_boundary_binary_ratio_scalar(&a, &strip(1, f64::MAX), "vMax", "vMin").is_err()
        );
    }
    #[test]
    fn rational_scale_proposals_are_reduced_bounded_and_not_certificates() {
        for (n, d) in [(7, 3), (11, 7), (13, 11), (1, 3)] {
            assert_eq!(
                propose_station_scale_scalar(n as f64 / d as f64),
                cad_predicates::AuthoredScalar::RationalConstant {
                    numerator: n,
                    denominator: d
                }
            );
        }
        for ratio in [2., 0.5, f64::MAX, f64::MIN_POSITIVE] {
            assert_eq!(
                propose_station_scale_scalar(ratio),
                cad_predicates::AuthoredScalar::Binary64Bits(ratio.to_bits())
            );
        }
        assert_eq!(
            propose_station_scale_scalar(f64::from_bits(1f64.to_bits() + 1)),
            cad_predicates::AuthoredScalar::RationalConstant {
                numerator: 1,
                denominator: 1
            }
        );
    }
    fn strip(degree: usize, height: f64) -> Surface {
        Surface {
            degree_u: 1,
            degree_v: degree,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: [vec![7.; degree + 1], vec![19.; degree + 1]].concat(),
            control_points: (0..2)
                .map(|u| {
                    (0..=degree)
                        .map(|v| vec![u as f64, 0., height * v as f64])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; degree + 1]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn normalized_bezier_scale_uses_endpoint_poles_and_retains_source() {
        let a = strip(5, 0.125);
        let b = strip(5, 0.25);
        let saved = a.control_points.clone();
        assert_eq!(
            propose_station_normal_scale(&a, &b, "vMax", "vMin").unwrap(),
            2.
        );
        assert_eq!(
            propose_station_normal_scale(&a, &a, "vMax", "vMin").unwrap(),
            1.
        );
        assert_eq!(a.control_points, saved);
        assert!(propose_station_normal_scale(&a, &b, "uMin", "vMin").is_err());
        let flat = strip(1, 0.);
        assert_eq!(
            propose_station_normal_scale(&flat, &flat, "vMax", "vMin").unwrap(),
            1.
        );
    }
    #[test]
    fn varying_weights_use_the_native_rational_derivative() {
        let mut a = strip(1, 0.125);
        let mut b = strip(1, 0.25);
        a.weights = vec![vec![1., 2.]; 2];
        b.weights = a.weights.clone();
        assert_eq!(
            propose_station_normal_scale(&a, &b, "vMin", "vMin").unwrap(),
            2.
        );
    }
    #[cfg(feature = "codec")]
    #[test]
    fn proposal_is_available_through_the_json_boundary() {
        let result = crate::transport::dispatch(value_codec::json!({
            "op":"surface_station_normal_scale", "reference":strip(5,0.125), "edited":strip(5,0.25),
            "referenceBoundary":"vMax", "editedBoundary":"vMin"
        }))
        .unwrap();
        assert_eq!(result.as_f64(), Some(2.));
    }
}
