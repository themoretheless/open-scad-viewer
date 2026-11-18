//! Exact C1/C2 across every ORIGINAL internal knot line, in raw parameters.
//! Regularity, periodic closure and material gluing are separate obligations.
use super::surface_tensor_identity::{self, Work};
use crate::{Result, check, surface::Surface};

pub struct Certificate<'s> {
    source: &'s Surface,
    order: usize,
}
impl<'s> Certificate<'s> {
    pub fn source(&self) -> &'s Surface {
        self.source
    }
    pub fn order(&self) -> usize {
        self.order
    }
}
pub struct Report<'s> {
    pub certificate: Option<Certificate<'s>>,
    pub cells: usize,
    pub exact_work: u64,
    /// Original raw-parameter Ck only; Different does not refute geometric Gk.
    pub outcome: cad_predicates::BezierIdentity,
}
/// Fresh original homogeneous coefficients and raw knot widths prove complete
/// one-sided tensor jets, including mixed derivatives. Basis multiplicity is
/// not used as a substitute for actual jets. No numerical sampling, rounded
/// knot insertion or caller continuity flag authorizes the certificate.
pub fn qualify(
    source: &Surface,
    order: usize,
    cells: usize,
    exact_work: u64,
) -> Result<Report<'_>> {
    source.validate()?;
    check(
        source.control_points[0][0].len() == 3
            && matches!(order, 1 | 2)
            && cells <= 1000000
            && exact_work <= 100000000,
        "Choose original XYZ surface C1/C2 and bounded work",
    )?;
    let mut work = Work {
        cells: 0,
        exact_work: 0,
    };
    let outcome = surface_tensor_identity::query_outcome(
        &[source],
        Some(order),
        &mut work,
        cells,
        exact_work,
    );
    let proven = outcome == cad_predicates::BezierIdentity::Equal;
    Ok(Report {
        certificate: proven.then_some(Certificate { source, order }),
        cells: work.cells,
        exact_work: work.exact_work,
        outcome,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn saddle() -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![2., 2., 4., 6., 6.],
            knots_v: vec![-3., -3., 5., 5.],
            control_points: [-1., 0., 1.]
                .into_iter()
                .map(|x| {
                    [-1., 1.]
                        .into_iter()
                        .map(|y| vec![x, y, x * y / 8.])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1., 2.]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn actual_rational_raw_knot_jets_prove_c2_beyond_basis_multiplicity() {
        let source = saddle();
        let before = source.clone();
        assert!(
            !crate::surface_regularity::inspect(&source, 10000)
                .unwrap()
                .interior_basis_c1
        );
        let r = qualify(&source, 2, 100000, 1000000).unwrap();
        assert_eq!(r.outcome, cad_predicates::BezierIdentity::Equal);
        let proof = r.certificate.expect("Exact unchanged rational saddle jets");
        assert!(std::ptr::eq(proof.source(), &source));
        assert_eq!(proof.order(), 2);
        assert!(r.cells > 0 && r.exact_work > 0);
        // An aggregate allowance may exceed a single native query's hard cap.
        // The adapter must honor BOTH, not create an invalid native context.
        let aggregate = qualify(&source, 2, 100000, 10 * cad_predicates::MAX_WORK).unwrap();
        assert!(aggregate.certificate.is_some());
        assert_eq!(
            (aggregate.cells, aggregate.exact_work),
            (r.cells, r.exact_work)
        );
        assert!(
            qualify(&source, 2, r.cells, r.exact_work)
                .unwrap()
                .certificate
                .is_some()
        );
        for (cells, work) in [
            (r.cells - 1, r.exact_work),
            (r.cells, r.exact_work - 1),
            (0, r.exact_work),
            (r.cells, 0),
        ] {
            let short = qualify(&source, 2, cells, work).unwrap();
            assert!(short.certificate.is_none());
            assert_eq!(
                short.outcome,
                cad_predicates::BezierIdentity::Indeterminate(
                    cad_predicates::Reason::ResourceLimit
                )
            );
            assert!(short.cells <= cells && short.exact_work <= work);
        }
        assert_eq!(source, before);
        let mut crease = source.clone();
        crease.control_points[1][0][2] += 0.125;
        for order in [1, 2] {
            let mismatch = qualify(&crease, order, 100000, 1000000).unwrap();
            assert!(mismatch.certificate.is_none());
            assert_eq!(mismatch.outcome, cad_predicates::BezierIdentity::Different);
        }
        crease.weights[0][0] = -1.;
        assert!(qualify(&crease, 2, 0, 0).is_err());
    }
}
