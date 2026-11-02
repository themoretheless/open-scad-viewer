//! Independent literal row identities plus a shared rational denominator ratio.
use super::{Axis, PResult, Refusal, Surface, Work, polynomial, row};
use crate::core::curve_identity::compare_definitions;
use crate::editing::exact_surface_knot_insertion::denominator::polynomial as denominator;
use cad_predicates::{BezierIdentity, Reason};
pub(super) struct Verification {
    pub verified: usize,
    pub refusal: Option<Refusal>,
}
pub(super) fn verify(
    source: &Surface,
    candidate: &Surface,
    axis: Axis,
    work: &mut Work,
) -> Verification {
    let count = match axis {
        Axis::U => source.control_points[0].len(),
        Axis::V => source.control_points.len(),
    };
    let mut verified = 0;
    let result = (|| -> PResult<()> {
        let source0 = row(source, axis, 0);
        let candidate0 = row(candidate, axis, 0);
        let [a, b] = source0.domain();
        // Cox-de Boor local-power extraction is independent of the proposal's
        // Bernstein/binomial conversion and Euclidean factor construction.
        let sa = denominator(&source0, a, b, work)?;
        let ca = denominator(&candidate0, a, b, work)?;
        for index in 0..count {
            if work.used == work.limit {
                return Err(Refusal::ResourceLimit);
            }
            let s = row(source, axis, index);
            let c = row(candidate, axis, index);
            let proof = compare_definitions(&s, &c, 0., work.limit - work.used, false)
                .map_err(|_| Refusal::InvalidOutput)?;
            work.used += proof.work_used;
            match proof.outcome {
                BezierIdentity::Equal => {}
                BezierIdentity::Different => return Err(Refusal::DifferentDefinition),
                BezierIdentity::Indeterminate(Reason::ResourceLimit) => {
                    return Err(Refusal::ResourceLimit);
                }
                BezierIdentity::Indeterminate(reason) => {
                    return Err(Refusal::PredicateIndeterminate(reason));
                }
            }
            let sw = if index == 0 {
                sa.clone()
            } else {
                denominator(&s, a, b, work)?
            };
            let cw = if index == 0 {
                ca.clone()
            } else {
                denominator(&c, a, b, work)?
            };
            if polynomial::multiply(&sw, &ca, work)? != polynomial::multiply(&cw, &sa, work)? {
                return Err(Refusal::DifferentDefinition);
            }
            // Positive original/output weights make the two anchor denominator
            // functions strictly positive on the ENTIRE selected domain. Thus
            // every row has the same rational scale, and summing in the
            // unchanged other-axis basis preserves the full tensor law.
            verified += 1;
        }
        Ok(())
    })();
    Verification {
        verified,
        refusal: result.err(),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn equal_constant_rows_cannot_hide_inconsistent_denominator_scales() {
        let source = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![vec![vec![0., 0., 0.], vec![0., 3., 0.]]; 2],
            weights: vec![vec![1., 3.], vec![2., 6.]],
            periodic_u: false,
            periodic_v: false,
        };
        let mut candidate = source.clone();
        for weights in &mut candidate.weights {
            weights[1] *= 2.;
        }
        source.validate().unwrap();
        candidate.validate().unwrap();
        let mut work = Work {
            used: 0,
            limit: 1_000_000,
        };
        let p = verify(&source, &candidate, Axis::U, &mut work);
        assert_eq!(p.verified, 1);
        assert_eq!(p.refusal, Some(Refusal::DifferentDefinition));
        let mut low = Work { used: 0, limit: 1 };
        let p = verify(&source, &source, Axis::U, &mut low);
        assert_eq!(p.verified, 0);
        assert_eq!(p.refusal, Some(Refusal::ResourceLimit));
        assert!(low.used <= 1);
    }
}
