//! Complete transverse contact isolation by exact physical-X slabs.
use super::*;
impl Patch {
    /// True proves that the complete polynomial images meet ONLY on their
    /// identical end/start boundary. It does not establish B-rep edge coverage,
    /// G2, field curvature, or within-face embedding away from this boundary.
    pub fn transverse_contact_only(&self, next: &Patch, b: &mut Budget) -> Result<bool> {
        if !self.joins_position_exact(next, b)? {
            return Ok(false);
        }
        for p in [self, next] {
            let domain = p.width.domain();
            let low = b.authored(domain[0])?;
            let high = b.authored(domain[1])?;
            let span = b.keep(&high - &low)?;
            if !span.is_positive() {
                return Ok(false);
            }
            let n = p.coefficients.len() - 1;
            for (i, row) in p.coefficients.iter().enumerate() {
                let expected = b.keep(&low + &span * ratio(i as u64, n as u64))?;
                for point in row {
                    if !b.keep(&point[0] - &expected)?.is_zero() {
                        return Ok(false);
                    }
                }
            }
        }
        // An injective transverse clock ensures the admitted common boundary
        // has no fold or repeated points. Do not infer this from endpoints.
        let row = self.coefficients.last().unwrap();
        let m = row.len() - 1;
        let span = b.keep(&row[m][1] - &row[0][1])?;
        if !span.is_positive() {
            return Ok(false);
        }
        for (j, point) in row.iter().enumerate() {
            let expected = b.keep(&row[0][1] + &span * ratio(j as u64, m as u64))?;
            if !b.keep(&point[1] - expected)?.is_zero() {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigInt;
    fn pair() -> [Patch; 2] {
        let source: [Surface; 2] = std::array::from_fn(|side| Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 2., 2., 2.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: [0., 0., 1.]
                .into_iter()
                .enumerate()
                .map(|(i, z)| {
                    vec![
                        vec![i as f64, 0., z],
                        vec![i as f64, if side == 0 { -1. } else { 1. }, z - 0.25],
                    ]
                })
                .collect(),
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        });
        let mut b = Budget::new(1_000_000).unwrap();
        std::array::from_fn(|i| {
            build(
                [&source[0], &source[1]],
                &Curve {
                    degree: 1,
                    knots: vec![i as f64, i as f64, i as f64 + 1., i as f64 + 1.],
                    control_points: vec![vec![0.25, 0., 0.]; 2],
                    weights: vec![1.; 2],
                    periodic: false,
                },
                &mut b,
            )
            .unwrap()
        })
    }
    #[test]
    fn physical_slabs_isolate_the_whole_common_boundary() {
        let [a, z] = pair();
        let mut b = Budget::new(1_000_000).unwrap();
        assert!(a.transverse_contact_only(&z, &mut b).unwrap());
        assert!(!z.transverse_contact_only(&a, &mut b).unwrap());
        assert!(!a.transverse_contact_only(&a, &mut b).unwrap());
        assert!(
            a.transverse_contact_only(&z, &mut Budget::new(0).unwrap())
                .is_err()
        );
    }
    #[test]
    fn actual_interior_x_and_whole_boundary_coefficients_are_rechecked() {
        let [a, mut z] = pair();
        let mut b = Budget::new(1_000_000).unwrap();
        z.coefficients[1][2][0] += Q::new(1.into(), BigInt::one() << 100usize);
        assert!(!a.transverse_contact_only(&z, &mut b).unwrap());
        let [a, mut z] = pair();
        z.coefficients[0][2][2] += ratio(1, 1000);
        assert!(!a.transverse_contact_only(&z, &mut b).unwrap());
    }
    #[test]
    fn matching_but_folded_transverse_boundaries_are_not_admitted() {
        let [mut a, mut z] = pair();
        let mut b = Budget::new(1_000_000).unwrap();
        a.coefficients.last_mut().unwrap()[1][1] = ratio(1, 2);
        z.coefficients[0][1][1] = ratio(1, 2);
        assert!(a.joins_position_exact(&z, &mut b).unwrap());
        assert!(!a.transverse_contact_only(&z, &mut b).unwrap());
    }
    #[test]
    fn retained_projection_checks_full_net_not_only_source_contact_rims() {
        let [mut a, _] = pair();
        let mut b = Budget::new(1_000_000).unwrap();
        assert!(a.inspect_retained_projection(0, &mut b).unwrap());
        assert!(a.inspect_retained_projection(1, &mut b).unwrap());
        a.coefficients[1][2][1] += Q::new(1.into(), BigInt::one() << 100usize);
        assert!(!a.inspect_retained_projection(0, &mut b).unwrap());
        assert!(a.inspect_retained_projection(2, &mut b).is_err());
        let [a, _] = pair();
        assert!(
            a.inspect_retained_projection(0, &mut Budget::new(0).unwrap())
                .is_err()
        );
    }
}
