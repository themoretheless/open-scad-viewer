//! Numeric scale indicators, not a condition-number or regularity certificate.
use crate::{bounds::Bounds, curve::Curve, surface::Surface, Result};
#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub weight_ratio: f64,
    /// Smallest positive active knot interval divided by the active domain span.
    pub min_relative_knot_span: Vec<f64>,
    /// Maximum absolute control coordinate divided by longest box extent.
    /// None for a point box; an extreme finite-input ratio may be infinite.
    pub coordinate_offset_ratio: Option<f64>,
    pub zero_extent: bool,
}
fn span(knots: &[f64], degree: usize, count: usize) -> f64 {
    let a = knots[degree];
    let b = knots[count];
    knots[degree..=count]
        .windows(2)
        .filter(|k| k[1] > k[0])
        .map(|k| (k[1] - k[0]) / (b - a))
        .fold(1_f64, f64::min)
}
fn report(weights: impl Iterator<Item = f64>, b: Bounds, spans: Vec<f64>) -> Report {
    let (lo, hi) = weights.fold((f64::INFINITY, 0_f64), |(a, b), w| (a.min(w), b.max(w)));
    let extent = b
        .min
        .iter()
        .zip(&b.max)
        .map(|(a, z)| z - a)
        .fold(0_f64, f64::max);
    let offset = b
        .min
        .iter()
        .chain(&b.max)
        .map(|x| x.abs())
        .fold(0_f64, f64::max);
    Report {
        weight_ratio: hi / lo,
        min_relative_knot_span: spans,
        coordinate_offset_ratio: if extent == 0. {
            None
        } else {
            Some(offset / extent)
        },
        zero_extent: extent == 0.,
    }
}
pub fn curve(source: &Curve) -> Result<Report> {
    source.validate()?;
    Ok(report(
        source.weights.iter().copied(),
        source.bounds()?,
        vec![span(
            &source.knots,
            source.degree,
            source.control_points.len(),
        )],
    ))
}
pub fn surface(source: &Surface) -> Result<Report> {
    source.validate()?;
    Ok(report(
        source.weights.iter().flatten().copied(),
        source.bounds()?,
        vec![
            span(
                &source.knots_u,
                source.degree_u,
                source.control_points.len(),
            ),
            span(
                &source.knots_v,
                source.degree_v,
                source.control_points[0].len(),
            ),
        ],
    ))
}
