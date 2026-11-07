//! Absolute curvature of actual stored Bezier coefficients, including endpoints.
use crate::{Result, check, curve::Curve};
use cad_predicates::{
    AuthoredScalar, CurvatureBoundDecision, CurvatureBoundOutcome, Limits, PredicateContext,
    SourceArena, ToleranceContext,
};

pub struct Certificate<'a> {
    source: &'a Curve,
    maximum: f64,
}
impl<'a> Certificate<'a> {
    pub fn source(&self) -> &'a Curve {
        self.source
    }
    pub fn maximum(&self) -> f64 {
        self.maximum
    }
}
pub struct Report<'a> {
    pub decision: CurvatureBoundDecision,
    pub certificate: Option<Certificate<'a>>,
}
/// No decomposition or fitting: only a literal original clamped Bezier is
/// supported. A certificate cannot be transferred to an edited curve.
pub fn inspect(source: &Curve, maximum: f64, cells: usize, work: u64) -> Result<Report<'_>> {
    source.validate()?;
    let n = source.degree + 1;
    check(
        !source.periodic
            && (2..=6).contains(&n)
            && source.control_points.len() == n
            && source.control_points[0].len() == 3
            && source.knots[..n].iter().all(|x| *x == source.knots[0])
            && source.knots[n..].iter().all(|x| *x == source.knots[n])
            && maximum.is_finite()
            && maximum > 0.
            && cells <= 4096
            && work <= cad_predicates::MAX_WORK,
        "Choose original XYZ Bezier, positive curvature limit and bounded work",
    )?;
    let mut raw = Vec::new();
    for (p, w) in source.control_points.iter().zip(&source.weights) {
        raw.extend([p[0], p[1], p[2], *w]);
    }
    raw.push(maximum);
    let arena = SourceArena::authored(
        "actual-Bezier-absolute-curvature",
        1,
        raw.into_iter()
            .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect(),
    )
    .map_err(|_| crate::Error::new("BEZIER_CURVATURE_INPUT", "Invalid source leaves"))?;
    let controls = (0..n)
        .map(|i| std::array::from_fn(|k| arena.leaf(4 * i + k).unwrap()))
        .collect::<Vec<_>>();
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work: work,
            ..Default::default()
        },
        None,
    );
    let decision = cad_predicates::rational_bezier_curvature_bound(
        &mut ctx,
        &controls,
        arena.leaf(4 * n).unwrap(),
        cells,
    )
    .map_err(|_| crate::Error::new("BEZIER_CURVATURE_INPUT", "Unsupported source input"))?;
    let certificate = (decision.outcome == CurvatureBoundOutcome::Within)
        .then_some(Certificate { source, maximum });
    Ok(Report {
        decision,
        certificate,
    })
}
