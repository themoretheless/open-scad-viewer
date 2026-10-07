//! Exact positive homothety of original guide/path coefficients. This proves
//! identical normalized arc phase, not regularity, contact or retained error.
use crate::{Result, check, curve::Curve, distance_bounds::Interval};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Dyadic {
    n: i128,
    e: i32,
}
impl Dyadic {
    fn normalized(n: i128, e: i32) -> Self {
        if n == 0 {
            return Self { n: 0, e: 0 };
        }
        let shift = n.unsigned_abs().trailing_zeros();
        Self {
            n: n >> shift,
            e: e + shift as i32,
        }
    }
    fn from_f64(x: f64) -> Option<Self> {
        if !x.is_finite() {
            return None;
        }
        let bits = x.to_bits();
        let exponent = ((bits >> 52) & 2047) as i32;
        let mantissa = bits & ((1u64 << 52) - 1);
        let (n, e) = if exponent == 0 {
            (mantissa, -1074)
        } else {
            (mantissa | (1u64 << 52), exponent - 1075)
        };
        Some(Self::normalized(
            if bits >> 63 == 0 {
                n as i128
            } else {
                -(n as i128)
            },
            e,
        ))
    }
    fn sub(self, other: Self) -> Option<Self> {
        if other.n == 0 {
            return Some(self);
        }
        if self.n == 0 {
            return Some(Self {
                n: other.n.checked_neg()?,
                e: other.e,
            });
        }
        let e = self.e.min(other.e);
        let shifted = |v: Self| {
            let shift = u32::try_from(v.e - e).ok()?;
            if shift >= 127 {
                return None;
            }
            v.n.checked_mul(1i128.checked_shl(shift)?)
        };
        Some(Self::normalized(
            shifted(self)?.checked_sub(shifted(other)?)?,
            e,
        ))
    }
    fn mul(self, other: Self) -> Option<Self> {
        Some(Self::normalized(
            self.n.checked_mul(other.n)?,
            self.e.checked_add(other.e)?,
        ))
    }
}

#[derive(Clone, Debug)]
pub(super) struct Report {
    /// Upper enclosure of the positive exact rational homothety factor.
    /// Phase identity is proved by coefficient cross-products, not this float.
    pub scale_upper: Option<f64>,
    pub cells: usize,
}

/// Literal common positive rational basis plus exact G_i-G_0=s(P_i-P_0)
/// establishes G(t)=sP(t)+c for all original parameters. Checked integer
/// cross-products admit rational s, including nonbinary13/12. Arithmetic
/// refuses unsupported exponent spreads instead of rounding the identity.
pub(super) fn certify(path: &Curve, guide: &Curve, max_cells: usize) -> Result<Report> {
    path.validate()?;
    guide.validate()?;
    check(
        max_cells <= 100000,
        "Guide phase identity budget exceeds100000",
    )?;
    check(
        [path, guide]
            .iter()
            .all(|c| c.control_points.iter().all(|p| p.len() == 3)),
        "Guide phase identity requires XYZ coefficients",
    )?;
    let mut out = Report {
        scale_upper: None,
        cells: 0,
    };
    if path.degree != guide.degree
        || path.knots != guide.knots
        || path.weights != guide.weights
        || path.periodic != guide.periodic
        || path.control_points.len() != guide.control_points.len()
    {
        return Ok(out);
    }
    let p0 = &path.control_points[0];
    let g0 = &guide.control_points[0];
    let candidate = path
        .control_points
        .iter()
        .enumerate()
        .flat_map(|(i, p)| (0..3).map(move |k| (i, k, (p[k] - p0[k]).abs())))
        .max_by(|a, b| a.2.total_cmp(&b.2));
    let Some((i, k, width)) = candidate.filter(|c| c.2 > 0.) else {
        return Ok(out);
    };
    if !width.is_finite() {
        return Ok(out);
    }
    let scale = (guide.control_points[i][k] - g0[k]) / (path.control_points[i][k] - p0[k]);
    if !scale.is_finite() || scale <= 0. {
        return Ok(out);
    }
    let Some(s) = Dyadic::from_f64(scale) else {
        return Ok(out);
    };
    let differences = (|| {
        Some((
            Dyadic::from_f64(path.control_points[i][k])?.sub(Dyadic::from_f64(p0[k])?)?,
            Dyadic::from_f64(guide.control_points[i][k])?.sub(Dyadic::from_f64(g0[k])?)?,
        ))
    })();
    let Some((path_difference, guide_difference)) = differences else {
        return Ok(out);
    };
    if path_difference.n == 0
        || guide_difference.n == 0
        || path_difference.n.signum() != guide_difference.n.signum()
    {
        return Ok(out);
    }
    let binary_scale_exact = path_difference.mul(s) == Some(guide_difference);
    for (p, g) in path.control_points.iter().zip(&guide.control_points) {
        for k in 0..3 {
            if out.cells == max_cells {
                return Ok(out);
            }
            out.cells += 1;
            let exact = (|| {
                Some((
                    Dyadic::from_f64(g[k])?.sub(Dyadic::from_f64(g0[k])?)?,
                    Dyadic::from_f64(p[k])?.sub(Dyadic::from_f64(p0[k])?)?,
                ))
            })();
            let Some((lhs, rhs)) = exact else {
                return Ok(out);
            };
            if binary_scale_exact {
                if rhs.mul(s) != Some(lhs) {
                    return Ok(out);
                }
            } else {
                let Some(left_product) = lhs.mul(path_difference) else {
                    return Ok(out);
                };
                let Some(right_product) = rhs.mul(guide_difference) else {
                    return Ok(out);
                };
                if left_product != right_product {
                    return Ok(out);
                }
            }
        }
    }
    // Preserve exact binary factors without adding rounding slack. Otherwise
    // enclose the true quotient from original coordinates, never from rounded
    // differences. Consumers use only its upper bound for residual budgets.
    out.scale_upper = if binary_scale_exact {
        Some(scale)
    } else {
        (|| {
            let numerator = Interval::point(guide.control_points[i][k])
                .sub(Interval::point(g0[k]))
                .ok()?;
            let denominator = Interval::point(path.control_points[i][k])
                .sub(Interval::point(p0[k]))
                .ok()?;
            let enclosure = numerator.div_signed(denominator).ok()?;
            (enclosure.lo > 0. && enclosure.hi.is_finite()).then_some(enclosure.hi)
        })()
    };
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nonbinary_positive_ratio_preserves_original_normalized_arc_phase() {
        let path = crate::primitives::circle([0.; 3], [0., 0., 1.], 3.).unwrap();
        let guide = crate::primitives::circle([0.; 3], [0., 0., 1.], 3.25).unwrap();
        let proof = certify(&path, &guide, 1000).unwrap();
        assert!(
            proof.scale_upper.is_some(),
            "Exact13/12 homothety must not require a binary64 scale"
        );
        let upper = proof.scale_upper.unwrap();
        assert!(upper >= (13_f64 / 12.).next_up() && upper < 1.084);
        assert!(
            certify(&path, &guide, proof.cells - 1)
                .unwrap()
                .scale_upper
                .is_none()
        );
        assert_eq!(certify(&path, &guide, 0).unwrap().cells, 0);
        let mut changed = guide.clone();
        changed.control_points[2][0] = changed.control_points[2][0].next_up();
        assert!(
            certify(&path, &changed, 1000)
                .unwrap()
                .scale_upper
                .is_none()
        );
        let mut changed_basis = guide.clone();
        changed_basis.weights[1] = changed_basis.weights[1].next_up();
        assert!(
            certify(&path, &changed_basis, 1000)
                .unwrap()
                .scale_upper
                .is_none()
        );
        let reverse = certify(&guide, &path, 1000).unwrap();
        assert!(
            reverse
                .scale_upper
                .is_some_and(|s| s >= 12. / 13. && s < 1.)
        );
        let path = crate::primitives::line([0.; 3], [0., 0., 3.]).unwrap();
        let guide = crate::primitives::line([1., 2., 3.], [1., 2., 6.25]).unwrap();
        assert!(certify(&path, &guide, 1000).unwrap().scale_upper.is_some());
    }
    #[test]
    fn positive_original_homothety_requires_every_exact_coefficient_and_budget() {
        let path = crate::primitives::circle([0.; 3], [0., 0., 1.], 4.).unwrap();
        let guide = crate::primitives::circle([0.; 3], [0., 0., 1.], 4.25).unwrap();
        let saved = guide.control_points.clone();
        let proof = certify(&path, &guide, 1000).unwrap();
        assert_eq!(proof.scale_upper, Some(17. / 16.));
        assert!(
            certify(&path, &guide, proof.cells - 1)
                .unwrap()
                .scale_upper
                .is_none()
        );
        assert_eq!(certify(&path, &guide, 0).unwrap().cells, 0);
        let mut changed = guide.clone();
        changed.control_points[2][0] = changed.control_points[2][0].next_up();
        assert!(
            certify(&path, &changed, 1000)
                .unwrap()
                .scale_upper
                .is_none()
        );
        changed = guide.clone();
        changed.control_points[2][1] = changed.control_points[2][1].next_up();
        assert!(
            certify(&path, &changed, 1000)
                .unwrap()
                .scale_upper
                .is_none()
        );
        changed = guide.clone();
        changed.weights[1] = changed.weights[1].next_up();
        assert!(
            certify(&path, &changed, 1000)
                .unwrap()
                .scale_upper
                .is_none()
        );
        assert_eq!(guide.control_points, saved);
    }
    #[test]
    fn translation_is_exact_but_reversed_speed_and_constant_paths_are_not_admitted() {
        let path = crate::primitives::line([0.; 3], [0., 0., 4.]).unwrap();
        let guide = crate::primitives::line([1., 2., 3.], [1., 2., 7.]).unwrap();
        assert_eq!(certify(&path, &guide, 100).unwrap().scale_upper, Some(1.));
        let reversed = crate::primitives::line([1., 2., 3.], [1., 2., -1.]).unwrap();
        assert!(
            certify(&path, &reversed, 100)
                .unwrap()
                .scale_upper
                .is_none()
        );
        let mut constant = path.clone();
        constant.control_points[1] = constant.control_points[0].clone();
        assert!(
            certify(&constant, &guide, 100)
                .unwrap()
                .scale_upper
                .is_none()
        );
        assert!(certify(&path, &guide, 100001).is_err());
    }
    #[test]
    fn dyadic_arithmetic_preserves_subnormals_and_refuses_integer_overflow() {
        let tiny = f64::from_bits(1);
        assert_eq!(
            Dyadic::from_f64(tiny)
                .unwrap()
                .sub(Dyadic::from_f64(0.).unwrap())
                .unwrap(),
            Dyadic::from_f64(tiny).unwrap()
        );
        assert!(
            Dyadic::from_f64(1.)
                .unwrap()
                .sub(Dyadic::from_f64(tiny).unwrap())
                .is_none()
        );
        assert!(
            Dyadic { n: i128::MAX, e: 0 }
                .mul(Dyadic { n: 3, e: 0 })
                .is_none()
        );
    }
}
