use super::{
    Curve, Result, ToleranceContext, box_of, check, context, distance, isolate_stationary,
    next_down, next_up, point_box_distance, stationary_coefficients,
};
#[path = "curve_projection/serialization.rs"]
#[cfg(feature = "codec")]
mod serialization;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectionStatus {
    Unique,
    Nonunique,
    NonuniqueOrUnresolved,
    IsolatedCandidate,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandidateClassification {
    Endpoint,
    SimpleStationary,
    MultipleOrClusteredStationary,
}
#[derive(Clone, Debug)]
pub struct ProjectionCandidate {
    pub domain: [f64; 2],
    pub parameter_interval: [f64; 2],
    pub point: Vec<f64>,
    pub distance_lower: f64,
    pub distance_upper: f64,
    pub classification: CandidateClassification,
    pub root_variation: usize,
}
#[derive(Debug)]
pub struct CurveProjection {
    pub status: ProjectionStatus,
    pub global_distance_upper: f64,
    pub candidates: Vec<ProjectionCandidate>,
    pub stationary_continuum: bool,
    pub winner_index: Option<usize>,
    pub tolerance: ToleranceContext,
}

/// Complete stationary candidate isolation on every rational Bézier span.
/// Bernstein sign variation cannot discard a real root; endpoint candidates are
/// always included and winner separation provides the global uniqueness proof.
pub fn project_curve_report(
    curve: &Curve,
    point: &[f64],
    tolerance: Option<ToleranceContext>,
) -> Result<CurveProjection> {
    curve.validate()?;
    check(
        point.len() == curve.control_points[0].len()
            && point.iter().all(|coordinate| coordinate.is_finite()),
        "Projection point must be finite and match curve dimension",
    )?;
    let tolerance = context(tolerance);
    let floor = tolerance.parametric_bounds().floor;
    let mut candidates: Vec<ProjectionCandidate> = Vec::new();
    let mut best_upper = f64::INFINITY;
    let mut stationary_continuum = false;
    for segment in curve.decompose()? {
        let c = segment.definition();
        let [a, b] = segment.domain();
        let analytic_linear = if c.degree == 1 {
            let first = &c.control_points[0];
            let last = &c.control_points[1];
            let direction = last
                .iter()
                .zip(first)
                .map(|(x, y)| x - y)
                .collect::<Vec<_>>();
            let denominator = direction.iter().map(|value| value * value).sum::<f64>();
            if denominator > 0. {
                let lambda = point
                    .iter()
                    .zip(first)
                    .zip(&direction)
                    .map(|((x, o), d)| (x - o) * d)
                    .sum::<f64>()
                    / denominator;
                if lambda > 0. && lambda < 1. {
                    Some(
                        lambda * c.weights[0]
                            / (c.weights[1] * (1. - lambda) + lambda * c.weights[0]),
                    )
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        let (roots, continuum) = if analytic_linear.is_some() {
            (vec![], false)
        } else {
            isolate_stationary(stationary_coefficients(c, point), floor / (b - a))
        };
        stationary_continuum |= continuum;
        let mut intervals = vec![
            (0., 0., CandidateClassification::Endpoint),
            (1., 1., CandidateClassification::Endpoint),
        ];
        intervals.extend(
            analytic_linear.map(|root| (root, root, CandidateClassification::SimpleStationary)),
        );
        intervals.extend(roots.iter().map(|root| {
            (
                root.lo,
                root.hi,
                if root.variation == 1 {
                    CandidateClassification::SimpleStationary
                } else {
                    CandidateClassification::MultipleOrClusteredStationary
                },
            )
        }));
        for (lo, hi, classification) in intervals {
            let parameter = a + (b - a) * (lo + hi) * 0.5;
            let evaluated = c.evaluate(parameter)?.point;
            let upper = distance(&evaluated, point);
            let restricted = if lo == hi {
                None
            } else {
                let mut piece = c.clone();
                if hi < 1. {
                    piece = piece.split(a + (b - a) * hi)?[0].clone();
                }
                if lo > 0. {
                    let mapped = a + (b - a) * lo;
                    piece = piece.split(mapped)?[1].clone();
                }
                Some(piece)
            };
            let (min, max) = restricted
                .as_ref()
                .map(|value| box_of(&value.control_points))
                .unwrap_or_else(|| (evaluated.clone(), evaluated.clone()));
            let lower = point_box_distance(&min, &max, point);
            best_upper = best_upper.min(upper);
            candidates.push(ProjectionCandidate {
                domain: [a, b],
                parameter_interval: [next_down(a + (b - a) * lo), next_up(a + (b - a) * hi)],
                point: evaluated,
                distance_lower: next_down(lower.max(0.)),
                distance_upper: next_up(upper),
                classification,
                root_variation: if classification == CandidateClassification::Endpoint {
                    0
                } else {
                    roots
                        .iter()
                        .find(|r| r.lo == lo && r.hi == hi)
                        .map(|r| r.variation)
                        .unwrap_or(0)
                },
            });
        }
    }
    let mut kept: Vec<ProjectionCandidate> = candidates
        .into_iter()
        .filter(|candidate| candidate.distance_lower <= next_up(best_upper))
        .collect();
    kept.sort_by(|a, b| {
        a.distance_upper
            .total_cmp(&b.distance_upper)
            .then(a.parameter_interval[0].total_cmp(&b.parameter_interval[0]))
    });
    let winner = kept.iter().enumerate().find(|(i, candidate)| {
        kept.iter()
            .enumerate()
            .all(|(j, other)| i == &j || candidate.distance_upper < other.distance_lower)
    });
    let winner_index = winner.map(|(index, _)| index);
    let status = if stationary_continuum {
        ProjectionStatus::Nonunique
    } else if winner_index.is_some() {
        ProjectionStatus::Unique
    } else if kept.len() > 1 {
        ProjectionStatus::NonuniqueOrUnresolved
    } else {
        ProjectionStatus::IsolatedCandidate
    };
    Ok(CurveProjection {
        status,
        global_distance_upper: next_up(best_upper),
        candidates: kept,
        stationary_continuum,
        winner_index,
        tolerance,
    })
}
