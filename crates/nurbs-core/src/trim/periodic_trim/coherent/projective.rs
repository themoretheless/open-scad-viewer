//! Source-owned nonlinear correspondence, separate from normalized-function identity.
use crate::{Result, check, curve::Curve};
use cad_predicates::{
    AuthoredScalar, BezierIdentity, PredicateContext, SourceArena, ToleranceContext,
};

#[derive(Clone, Debug)]
pub struct ProjectiveCurveCorrespondence {
    sources: [Curve; 2],
    alpha_beta: [f64; 2],
    reverse: bool,
    work: u64,
}
impl ProjectiveCurveCorrespondence {
    pub fn sources(&self) -> [&Curve; 2] {
        [&self.sources[0], &self.sources[1]]
    }
    pub fn alpha_beta(&self) -> [f64; 2] {
        self.alpha_beta
    }
    pub fn orientation(&self) -> i8 {
        if self.reverse { -1 } else { 1 }
    }
    pub fn work_used(&self) -> u64 {
        self.work
    }
    pub fn reverify(&self, max_work: u64) -> Result<()> {
        let fresh = prove(
            &self.sources[0],
            &self.sources[1],
            self.alpha_beta,
            self.reverse,
            max_work,
        )?;
        check(
            fresh.0 && fresh.1 == self.work,
            "Original projective curve correspondence remains unproven",
        )
    }
}
fn clamped(c: &Curve) -> bool {
    let d = c.domain();
    !c.periodic
        && (1..=32).contains(&c.degree)
        && c.knots[..=c.degree].iter().all(|&k| k == d[0])
        && c.knots[c.control_points.len()..].iter().all(|&k| k == d[1])
}
fn bezier(c: &Curve) -> bool {
    clamped(c) && c.control_points.len() == c.degree + 1
}
fn prove(
    a: &Curve,
    b: &Curve,
    alpha_beta: [f64; 2],
    reverse: bool,
    max_work: u64,
) -> Result<(bool, u64)> {
    a.validate()?;
    b.validate()?;
    check(
        alpha_beta.iter().all(|x| x.is_finite() && *x > 0.) && max_work <= cad_predicates::MAX_WORK,
        "Projective correspondence needs positive finite factors and bounded work",
    )?;
    let bezier_pair = bezier(a) && bezier(b);
    let basis_pair = clamped(a)
        && clamped(b)
        && a.degree == b.degree
        && a.control_points.len() == b.control_points.len()
        && a.control_points.len() <= 4096;
    if !(bezier_pair || basis_pair)
        || a.control_points[0].len() != b.control_points[0].len()
        || !(2..=3).contains(&a.control_points[0].len())
    {
        return Ok((false, 0));
    }
    let mut values = Vec::new();
    for c in [a, b] {
        for (p, w) in c.control_points.iter().zip(&c.weights) {
            values.extend([p[0], p[1], *p.get(2).unwrap_or(&0.), *w]);
        }
    }
    values.extend(alpha_beta);
    if !bezier_pair {
        values.extend(&a.knots);
        values.extend(&b.knots);
    }
    let arena = SourceArena::authored(
        "original-projective-curve-correspondence",
        1,
        values
            .iter()
            .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect(),
    )
    .map_err(|_| crate::input("Projective correspondence input admission failed"))?;
    let rows = |start: usize, n: usize| {
        (start..start + n)
            .map(|i| std::array::from_fn(|k| arena.leaf(4 * i + k).unwrap()))
            .collect::<Vec<_>>()
    };
    let n = a.control_points.len() + b.control_points.len();
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tolerance,
        cad_predicates::Limits {
            max_work,
            ..Default::default()
        },
        None,
    );
    let proof = if bezier_pair {
        cad_predicates::rational_bezier_projective_correspondence(
            &mut ctx,
            &rows(0, a.control_points.len()),
            &rows(a.control_points.len(), b.control_points.len()),
            [arena.leaf(4 * n).unwrap(), arena.leaf(4 * n + 1).unwrap()],
            reverse,
        )
    } else {
        let start = 4 * n + 2;
        let knots = |offset: usize, count: usize| {
            (offset..offset + count)
                .map(|i| arena.leaf(i).unwrap())
                .collect::<Vec<_>>()
        };
        cad_predicates::rational_bspline_projective_correspondence(
            &mut ctx,
            a.degree,
            &rows(0, a.control_points.len()),
            &knots(start, a.knots.len()),
            &rows(a.control_points.len(), b.control_points.len()),
            &knots(start + a.knots.len(), b.knots.len()),
            [arena.leaf(4 * n).unwrap(), arena.leaf(4 * n + 1).unwrap()],
            reverse,
        )
    }
    .map_err(|_| crate::input("Projective correspondence exact predicate failed"))?;
    Ok((proof.outcome == BezierIdentity::Equal, proof.work_used))
}
/// Qualify a complete normalized-interval fractional-linear correspondence.
/// Distinctness/embedding, seam partitions and synchronized homotopy are separate.
pub fn verify_projective_curve_correspondence(
    a: &Curve,
    b: &Curve,
    alpha_beta: [f64; 2],
    reverse: bool,
    max_work: u64,
) -> Result<Option<ProjectiveCurveCorrespondence>> {
    let (qualified, work) = prove(a, b, alpha_beta, reverse, max_work)?;
    Ok(qualified.then(|| ProjectiveCurveCorrespondence {
        sources: [a.clone(), b.clone()],
        alpha_beta,
        reverse,
        work,
    }))
}

/// Proof that BOTH actual source and saved candidate families correspond under
/// one global monotone nonlinear map, including all original seam intervals.
#[derive(Clone, Debug)]
pub struct ProjectivePartitionHomotopy {
    sources: [Curve; 2],
    partitions: [super::Report; 2],
    alpha_beta: [f64; 2],
    reverse: bool,
    work: u64,
}
impl ProjectivePartitionHomotopy {
    pub fn sources(&self) -> [&Curve; 2] {
        [&self.sources[0], &self.sources[1]]
    }
    pub fn partitions(&self) -> [&super::Report; 2] {
        [&self.partitions[0], &self.partitions[1]]
    }
    pub fn orientation(&self) -> i8 {
        if self.reverse { -1 } else { 1 }
    }
    pub fn work_used(&self) -> u64 {
        self.work
    }
    pub fn reverify(&self, max_work: u64) -> Result<()> {
        let fresh = prove_partition(
            &self.sources[0],
            &self.sources[1],
            &self.partitions[0],
            &self.partitions[1],
            self.alpha_beta,
            self.reverse,
            max_work,
        )?;
        check(
            fresh.0 && fresh.1 == self.work,
            "Original projective partition homotopy remains unproven",
        )
    }
}
fn prove_partition(
    a: &Curve,
    b: &Curve,
    pa: &super::Report,
    pb: &super::Report,
    alpha_beta: [f64; 2],
    reverse: bool,
    max_work: u64,
) -> Result<(bool, u64)> {
    a.validate()?;
    b.validate()?;
    check(
        a.control_points[0].len() == 2 && b.control_points[0].len() == 2,
        "Projective UV homotopy requires two-dimensional source curves",
    )?;
    let (qualified, mut work) = prove(a, b, alpha_beta, reverse, max_work)?;
    if !qualified {
        return Ok((false, work));
    }
    check(
        !pa.pieces.is_empty() && pa.pieces.len() <= 256 && pb.pieces.len() <= 256,
        "Projective homotopy requires bounded nonempty partitions",
    )?;
    for (source, partition) in [(a, pa), (b, pb)] {
        let mut end = source.domain()[0];
        for piece in &partition.pieces {
            piece.curve.validate()?;
            check(
                piece.source_domain == piece.curve.domain()
                    && piece.source_domain[0] == end
                    && piece.source_domain[0] < piece.source_domain[1]
                    && piece.translation.iter().all(|x| x.is_finite()),
                "Projective partition omits or changes an original source interval",
            )?;
            end = piece.source_domain[1];
        }
        check(
            end == source.domain()[1],
            "Projective partition omits original source endpoint",
        )?;
    }
    if pa.pieces.len() != pb.pieces.len() {
        return Ok((false, work));
    }
    for (i, x) in pa.pieces.iter().enumerate() {
        let y = &pb.pieces[if reverse { pb.pieces.len() - 1 - i } else { i }];
        if x.translation != y.translation
            || x.tile != y.tile
            || !bezier(&x.curve)
            || !bezier(&y.curve)
            || x.curve.control_points[0].len() != 2
            || y.curve.control_points[0].len() != 2
        {
            return Ok((false, work));
        }
        let mut values = Vec::new();
        for c in [&x.curve, &y.curve] {
            for (p, w) in c.control_points.iter().zip(&c.weights) {
                values.extend([p[0], p[1], 0., *w]);
            }
        }
        let n = x.curve.control_points.len() + y.curve.control_points.len();
        values.extend(alpha_beta);
        values.extend(a.domain());
        values.extend(b.domain());
        values.extend(x.source_domain);
        values.extend(y.source_domain);
        let arena = SourceArena::authored(
            "original-projective-partition-homotopy",
            1,
            values
                .iter()
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .map_err(|_| crate::input("Projective homotopy input admission failed"))?;
        let rows = |start: usize, n: usize| {
            (start..start + n)
                .map(|i| std::array::from_fn(|k| arena.leaf(4 * i + k).unwrap()))
                .collect::<Vec<_>>()
        };
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
        let leaf = |i| arena.leaf(4 * n + i).unwrap();
        let proof = cad_predicates::rational_bezier_projective_subinterval_correspondence(
            &mut ctx,
            &rows(0, x.curve.control_points.len()),
            &rows(x.curve.control_points.len(), y.curve.control_points.len()),
            [leaf(0), leaf(1)],
            [[leaf(2), leaf(3)], [leaf(4), leaf(5)]],
            [[leaf(6), leaf(7)], [leaf(8), leaf(9)]],
            reverse,
        )
        .map_err(|_| crate::input("Projective subinterval predicate failed"))?;
        work += proof.work_used;
        if proof.outcome != BezierIdentity::Equal {
            return Ok((false, work));
        }
    }
    Ok((true, work))
}
/// Prove synchronized straight-line lifted homotopy of source and candidate
/// copies under one exact projective bijection. This does not prove embedding
/// of the representative, or regularize contacts between distinct images.
pub fn verify_projective_partition_homotopy(
    a: &Curve,
    b: &Curve,
    pa: &super::Report,
    pb: &super::Report,
    alpha_beta: [f64; 2],
    reverse: bool,
    max_work: u64,
) -> Result<Option<ProjectivePartitionHomotopy>> {
    let (qualified, work) = prove_partition(a, b, pa, pb, alpha_beta, reverse, max_work)?;
    Ok(qualified.then(|| ProjectivePartitionHomotopy {
        sources: [a.clone(), b.clone()],
        partitions: [pa.clone(), pb.clone()],
        alpha_beta,
        reverse,
        work,
    }))
}

impl ProjectivePartitionHomotopy {
    pub(in crate::trim::periodic_trim) fn matches(
        &self,
        a: &Curve,
        b: &Curve,
        pa: &super::Report,
        pb: &super::Report,
        reverse: bool,
    ) -> bool {
        fn same(a: &super::Report, b: &super::Report) -> bool {
            a.pieces.len() == b.pieces.len()
                && a.pieces.iter().zip(&b.pieces).all(|(x, y)| {
                    x.source_domain == y.source_domain
                        && x.translation == y.translation
                        && x.tile == y.tile
                        && x.curve == y.curve
                })
        }
        self.reverse == reverse
            && self.sources[0] == *a
            && self.sources[1] == *b
            && same(&self.partitions[0], pa)
            && same(&self.partitions[1], pb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_projective_homotopy_receipt_replays_sources_partitions_and_work() {
        let a = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap();
        let mut b = a.clone();
        b.weights = vec![1., 2.];
        let part = |c: &Curve| super::super::Report {
            pieces: vec![crate::periodic_trim::Piece {
                source_domain: c.domain(),
                translation: [0.; 2],
                tile: [0; 2],
                curve: c.clone(),
            }],
            candidate_hulls_contained: false,
        };
        let proof = verify_projective_partition_homotopy(
            &a,
            &b,
            &part(&a),
            &part(&b),
            [2., 1.],
            false,
            1_000_000,
        )
        .unwrap()
        .unwrap();
        proof.reverify(1_000_000).unwrap();
        let mut changed = proof.clone();
        changed.sources[1].control_points[1][0] = 1.0_f64.next_up();
        assert!(changed.reverify(1_000_000).is_err());
        let mut changed = proof.clone();
        changed.partitions[1].pieces[0].curve.weights[1] = 1.;
        assert!(changed.reverify(1_000_000).is_err());
        let mut changed = proof.clone();
        changed.partitions[1].pieces[0].tile[0] = 1;
        assert!(changed.reverify(1_000_000).is_err());
        let mut changed = proof.clone();
        changed.work -= 1;
        assert!(changed.reverify(1_000_000).is_err());
        let mut changed = proof.clone();
        changed.alpha_beta = [1., 1.];
        assert!(changed.reverify(1_000_000).is_err());
    }
}
