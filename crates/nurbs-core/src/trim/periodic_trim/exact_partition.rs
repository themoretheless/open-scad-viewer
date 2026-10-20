//! Whole original-function preservation, separate from isotopy and contact claims.
use super::Report;
use crate::{Result, check, curve::Curve};
use cad_predicates::{
    AuthoredScalar, BezierIdentity, PredicateContext, SourceArena, ToleranceContext,
};
#[derive(Clone, Debug, PartialEq)]
pub struct ExactPartitionIdentity {
    source: Curve,
    partition: Report,
    work: u64,
}
impl ExactPartitionIdentity {
    pub fn source(&self) -> &Curve {
        &self.source
    }
    pub fn partition(&self) -> &Report {
        &self.partition
    }
    pub fn work_used(&self) -> u64 {
        self.work
    }
    pub fn reverify(&self, max_work: u64) -> Result<()> {
        let fresh = prove(&self.source, &self.partition, max_work)?;
        check(
            fresh.0 && fresh.1 == self.work,
            "Original exact partition identity remains unproven",
        )
    }
}
fn prove(source: &Curve, partition: &Report, max_work: u64) -> Result<(bool, u64)> {
    source.validate()?;
    check(
        !source.periodic
            && source.control_points[0].len() == 2
            && max_work <= cad_predicates::MAX_WORK
            && (1..=256).contains(&partition.pieces.len()),
        "Exact partition needs bounded nonperiodic UV source and pieces",
    )?;
    let mut end = source.domain()[0];
    for p in &partition.pieces {
        p.curve.validate()?;
        check(
            !p.curve.periodic
                && p.curve.control_points[0].len() == 2
                && p.source_domain == p.curve.domain()
                && p.source_domain[0] == end
                && p.source_domain[0] < p.source_domain[1]
                && p.translation.iter().all(|x| x.is_finite()),
            "Exact partition omits or changes an original interval",
        )?;
        end = p.source_domain[1];
    }
    check(
        end == source.domain()[1],
        "Exact partition omits the original upper endpoint",
    )?;
    let mut work = 0;
    for p in &partition.pieces {
        let mut values = Vec::new();
        for c in [source, &p.curve] {
            for (point, w) in c.control_points.iter().zip(&c.weights) {
                values.extend([point[0], point[1], 0., *w]);
            }
        }
        let offset = values.len();
        values.extend(&source.knots);
        values.extend(&p.curve.knots);
        values.extend([p.translation[0], p.translation[1], 0.]);
        let arena = SourceArena::authored(
            "original-exact-periodic-partition",
            1,
            values
                .iter()
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .map_err(|_| crate::input("Exact partition source admission failed"))?;
        let rows = |start: usize, n: usize| {
            (start..start + n)
                .map(|i| std::array::from_fn(|k| arena.leaf(4 * i + k).unwrap()))
                .collect::<Vec<_>>()
        };
        let refs = |start: usize, n: usize| {
            (start..start + n)
                .map(|i| arena.leaf(i).unwrap())
                .collect::<Vec<_>>()
        };
        let shift = offset + source.knots.len() + p.curve.knots.len();
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(
            &arena,
            &tolerance,
            cad_predicates::Limits {
                max_work: max_work - work,
                ..Default::default()
            },
            None,
        );
        let receipt = cad_predicates::rational_bspline_restriction_identity(
            &mut ctx,
            source.degree,
            &rows(0, source.control_points.len()),
            &refs(offset, source.knots.len()),
            p.curve.degree,
            &rows(source.control_points.len(), p.curve.control_points.len()),
            &refs(offset + source.knots.len(), p.curve.knots.len()),
            std::array::from_fn(|i| arena.leaf(shift + i).unwrap()),
        )
        .map_err(|_| crate::input("Exact partition source predicate failed"))?;
        work += receipt.work_used;
        if receipt.outcome != BezierIdentity::Equal {
            return Ok((false, work));
        }
    }
    Ok((true, work))
}
/// Own exact source(u)=candidate_piece(u)+translation on every original
/// parameter. No approximate deviation, sample, preview flag, tile or endpoint
/// proximity is identity authority. Chart/contact/face admission stays separate.
pub fn verify_exact_partition_identity(
    source: &Curve,
    partition: &Report,
    max_work: u64,
) -> Result<Option<ExactPartitionIdentity>> {
    Ok(verify_with_work(source, partition, max_work)?.0)
}
pub(super) fn verify_with_work(
    source: &Curve,
    partition: &Report,
    max_work: u64,
) -> Result<(Option<ExactPartitionIdentity>, u64)> {
    let (qualified, work) = prove(source, partition, max_work)?;
    Ok((
        qualified.then(|| ExactPartitionIdentity {
            source: source.clone(),
            partition: partition.clone(),
            work,
        }),
        work,
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_checks_actual_source_candidate_translation_and_charged_work() {
        let source = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap();
        let report = Report {
            pieces: vec![super::super::Piece {
                source_domain: source.domain(),
                curve: source.clone(),
                translation: [0.; 2],
                tile: [0; 2],
            }],
            candidate_hulls_contained: false,
        };
        let proof = verify_exact_partition_identity(&source, &report, 1_000_000)
            .unwrap()
            .unwrap();
        proof.reverify(1_000_000).unwrap();
        let mut bad = proof.clone();
        bad.source.control_points[1][0] = 1.0_f64.next_up();
        assert!(bad.reverify(1_000_000).is_err());
        let mut bad = proof.clone();
        bad.partition.pieces[0].curve.weights[1] = 2.;
        assert!(bad.reverify(1_000_000).is_err());
        let mut bad = proof.clone();
        bad.partition.pieces[0].translation[0] = f64::EPSILON;
        assert!(bad.reverify(1_000_000).is_err());
        let mut bad = proof.clone();
        bad.work -= 1;
        assert!(bad.reverify(1_000_000).is_err());
    }
}
