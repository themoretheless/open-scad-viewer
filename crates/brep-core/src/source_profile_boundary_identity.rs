//! Exact identity along a clamped natural boundary of a positive weight profile.
//! This proves a curve identity, not retained contact ownership or Body admission.
use nurbs_core::{Error, Result, normal_alignment::WeightProfile};

pub struct Certificate<'p, 's> {
    profile: &'p WeightProfile<'s>,
    boundary: (usize, bool),
}
impl<'p, 's> Certificate<'p, 's> {
    pub fn profile(&self) -> &'p WeightProfile<'s> {
        self.profile
    }
    pub fn boundary(&self) -> (usize, bool) {
        self.boundary
    }
}
pub struct Report<'p, 's> {
    pub certificate: Option<Certificate<'p, 's>>,
    pub exact_work: u64,
    pub reason: &'static str,
}
pub fn certify<'p, 's>(
    profile: &'p WeightProfile<'s>,
    axis: usize,
    upper: bool,
    max_work: u64,
) -> Result<Report<'p, 's>> {
    if axis > 1 || max_work > 100_000_000 {
        return Err(Error::new(
            "BREP_PROFILE_BOUNDARY",
            "Choose a natural boundary and bounded exact work",
        ));
    }
    let mut out = Report {
        certificate: None,
        exact_work: 0,
        reason: "profile-boundary-layout-unproven",
    };
    let s = profile.source();
    s.validate()?;
    let counts = [s.control_points.len(), s.control_points[0].len()];
    let degrees = [s.degree_u, s.degree_v];
    let knots = [&s.knots_u, &s.knots_v];
    if s.periodic_u || s.periodic_v || degrees[axis] == 0 {
        return Ok(out);
    }
    let endpoint = if upper {
        knots[axis][counts[axis]]
    } else {
        knots[axis][degrees[axis]]
    };
    let clamped = if upper {
        &knots[axis][counts[axis]..]
    } else {
        &knots[axis][..=degrees[axis]]
    };
    if clamped.len() != degrees[axis] + 1 || clamped.iter().any(|&k| k != endpoint) {
        return Ok(out);
    }
    let (rows, columns, pivot) = profile.factors();
    if !pivot.is_finite()
        || pivot <= 0.
        || rows
            .iter()
            .chain(columns)
            .chain(s.weights.iter().flatten())
            .any(|&w| !w.is_finite() || w <= 0.)
    {
        out.reason = "profile-boundary-positive-weights-unproven";
        return Ok(out);
    }
    let fixed = if upper { counts[axis] - 1 } else { 0 };
    for free in 0..counts[1 - axis] {
        let [i, j] = if axis == 0 {
            [fixed, free]
        } else {
            [free, fixed]
        };
        // Exact dyadic determinant: row[i]*column[j] - pivot*source_weight.
        // No rounded target product or ratio participates in this identity.
        let sign = crate::source_allowed_contact::orient(
            &[
                [0., 0., 0.],
                [rows[i], s.weights[i][j], 0.],
                [pivot, columns[j], 0.],
            ],
            Some([0, 1]),
            &mut out.exact_work,
            max_work,
        )?;
        if sign != Some(cad_predicates::Sign::Zero) {
            out.reason = if sign.is_none() {
                "profile-boundary-work-limit"
            } else {
                "profile-boundary-weight-identity-unproven"
            };
            return Ok(out);
        }
    }
    // Clamping selects one control row/column. All controls and knots are owned
    // by the same source; exact boundary weights agree, so the entire rational
    // boundary curve agrees pointwise, including every trimmed subinterval.
    out.certificate = Some(Certificate {
        profile,
        boundary: (axis, upper),
    });
    out.reason = "profile-natural-boundary-identity-qualified";
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn boundary_identity_refuses_one_ulp_residual_and_exhausted_work() {
        let s = nurbs_core::surface::Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1., 1.], vec![1., f64::from_bits(1f64.to_bits() + 1)]],
            periodic_u: false,
            periodic_v: false,
        };
        let (p, _) = nurbs_core::normal_alignment::qualify_weight_profile(&s, [0, 0], 4).unwrap();
        let p = p.unwrap();
        for axis in 0..2 {
            let report = certify(&p, axis, false, 10000).unwrap();
            let proof = report.certificate.unwrap();
            assert!(std::ptr::eq(proof.profile(), &p));
            assert_eq!(proof.boundary(), (axis, false));
            assert!(report.exact_work > 0);
            let failed = certify(&p, axis, true, 10000).unwrap();
            assert!(failed.certificate.is_none());
            assert_eq!(failed.reason, "profile-boundary-weight-identity-unproven");
            let stopped = certify(&p, axis, false, 0).unwrap();
            assert!(stopped.certificate.is_none());
            assert_eq!(stopped.exact_work, 0);
        }
        assert!(certify(&p, 2, false, 100).is_err());
    }
}
