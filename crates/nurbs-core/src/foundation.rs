//! Conservative, deterministic certificates used by product NURBS consumers.
//!
//! Every geometric enclosure is derived from positive-weight convex-hull
//! properties. `next_down`/`next_up` make binary64 arithmetic outward rounded.
//! Ambiguous regularity and projection cases remain explicitly unresolved.
use crate::{
    Result, check,
    curve::Curve,
    numeric, resource,
    surface::{Axis, Surface},
};
use cad_predicates::ToleranceContext;
use math_core::{next_down, next_up};
#[cfg(feature = "codec")]
mod serialization;
#[cfg(feature = "codec")]
use serialization::tolerance_evidence;
#[cfg(feature = "codec")]
pub use serialization::{project_curve, project_surface};

pub mod approximate_edits;
#[cfg(feature = "codec")]
pub use approximate_edits::serialization::{
    rebuild_curve, rebuild_surface, reduce_curve_degree, reduce_surface_axis, remove_curve_knot,
};
pub use approximate_edits::{
    SurfaceSimplification, rebuild_curve_report, rebuild_surface_report,
    reduce_curve_degree_report, reduce_surface_axis_report, remove_curve_knot_report,
};
pub mod periodic_edits;
use periodic_edits::seam_certificate;
#[cfg(feature = "codec")]
pub use periodic_edits::serialization::{
    edit_periodic_curve, edit_periodic_surface, split_periodic_curve,
};
pub use periodic_edits::{
    PeriodicEditOperation, edit_periodic_curve_report, edit_periodic_surface_report,
    split_periodic_curve_report,
};
pub mod approximation;
#[cfg(feature = "codec")]
pub use approximation::serialization::{approximate_curve, interpolate_polyline};
pub use approximation::{approximate_curve_report, interpolate_polyline_report};
pub mod exact_edits;
#[cfg(feature = "codec")]
pub use exact_edits::serialization::{certify_exact_edit, reparameterize_curve};
pub use exact_edits::{ExactEditOperation, certify_exact_edit_report, reparameterize_curve_report};
pub mod fitting;
pub mod guards;
pub mod min_zone_fit;
pub mod interpolation;
pub mod bracketed;
pub mod oslo;
pub mod knot_removal;
#[cfg(feature = "codec")]
pub use fitting::serialization::{
    fit_curve_cloud_certified, fit_curve_points, fit_surface_cloud_certified,
    interpolate_surface_grid,
};
pub use fitting::{
    fit_curve_cloud_certified_report, fit_curve_points_report, fit_surface_cloud_certified_report,
    interpolate_surface_grid_report,
};
pub mod parameter_mapping;
#[cfg(feature = "codec")]
pub use parameter_mapping::serialization::{
    certify_reparameterization, evaluate_reparameterized_curve, materialize_reparameterized_curve,
};
pub use parameter_mapping::{
    MapPiece, ParameterMapping, certify_reparameterization_report,
    evaluate_reparameterized_curve_report, materialize_reparameterized_curve_report,
};
pub mod certificates;
#[cfg(test)]
use certificates::{
    NormalBoxClass, SurfaceRegularityClass, certify_surface_cell, classify_normal_box,
};
#[cfg(feature = "codec")]
pub use certificates::{certify_curve, certify_surface};
pub use certificates::{certify_curve_report, certify_surface_report};
mod surface_projection;
pub use surface_projection::{
    BoundaryProjection, SurfaceCandidateClassification, SurfaceProjection,
    SurfaceProjectionCandidate, SurfaceProjectionProof, project_surface_report,
};

mod common;
pub(crate) use common::*;
mod curve_projection;
pub use curve_projection::{
    CandidateClassification, CurveProjection, ProjectionCandidate, ProjectionStatus,
    project_curve_report,
};
#[cfg(test)]
#[path = "foundation/tests/surface_reduction_validation_tests.rs"]
mod surface_reduction_validation_tests;

#[cfg(test)]
#[path = "foundation/tests/deviation_span_tests.rs"]
mod deviation_span_tests;

#[cfg(test)]
#[path = "foundation/tests/rebuild_tests.rs"]
mod rebuild_tests;
#[cfg(test)]
#[path = "foundation/tests/surface_rebuild_tests.rs"]
mod surface_rebuild_tests;

#[cfg(test)]
#[path = "foundation/tests/periodic_rebuild_tests.rs"]
mod periodic_rebuild_tests;

#[cfg(test)]
#[path = "foundation/tests/periodic_surface_rebuild_tests.rs"]
mod periodic_surface_rebuild_tests;

#[cfg(test)]
#[path = "foundation/tests/normal_regularity_regressions.rs"]
mod normal_regularity_regressions;

#[cfg(feature="codec")]
use value_codec::Value;

#[cfg(feature="codec")]
fn map_pieces(mapping: &Value) -> Result<Vec<MapPiece>> {
    let values = mapping["pieces"]
        .as_array()
        .ok_or_else(|| crate::input("Reparameterization requires pieces"))?;
    check(
        !values.is_empty() && values.len() <= 64,
        "Piecewise reparameterization needs 1..64 pieces",
    )?;
    values
        .iter()
        .map(|piece| {
            let domain: [f64; 2] = value_codec::from_value(piece["domain"].clone())
                .map_err(|e| crate::input(e.to_string()))?;
            let range: [f64; 2] = value_codec::from_value(piece["range"].clone())
                .map_err(|e| crate::input(e.to_string()))?;
            let controls: Vec<f64> = value_codec::from_value(piece["controlValues"].clone())
                .map_err(|e| crate::input(e.to_string()))?;
            let weights: Vec<f64> = value_codec::from_value(piece["weights"].clone())
                .map_err(|e| crate::input(e.to_string()))?;
            check(
                (2..=26).contains(&controls.len()) && controls.len() == weights.len(),
                "Map piece needs 2..26 matched controls and weights",
            )?;
            check(
                domain[0] < domain[1]
                    && range[0] < range[1]
                    && domain.iter().chain(range.iter()).all(|x| x.is_finite()),
                "Map domains and ranges must increase finitely",
            )?;
            check(
                weights
                    .iter()
                    .all(|w| w.is_finite() && *w >= 1e-12 && *w <= 1e12),
                "Map weights must be positive and bounded",
            )?;
            check(
                controls.first() == Some(&range[0]) && controls.last() == Some(&range[1]),
                "Map endpoint controls must equal the declared range",
            )?;
            Ok(MapPiece {
                domain,
                range,
                values: controls,
                weights,
            })
        })
        .collect()
}

#[cfg(feature="codec")]
fn compose_bezier_with_piece(span: &Curve, piece: &MapPiece) -> Result<Curve> {
    let dimension = span.control_points[0].len();
    let n = span.degree;
    let source_domain = span.domain();
    // Map values are normalized into the source span's domain. A bounded
    // knot preimage may put a tiny part outside that span; the whole-composition
    // retention gate must cover this polynomial continuation as well.

    let map_p: Vec<f64> = piece
        .values
        .iter()
        .zip(&piece.weights)
        .map(|(value, weight)| {
            ((value - source_domain[0]) / (source_domain[1] - source_domain[0])) * weight
        })
        .collect();
    let map_q = piece.weights.clone();
    // Cleared form uses P^i (Q-P)^{n-i}; for polynomial maps Q≡1 this is φ^i (1-φ)^{n-i}.
    let map_one_minus: Vec<f64> = map_q.iter().zip(&map_p).map(|(q, p)| q - p).collect();
    let mut p_powers = vec![vec![1.]];
    let mut one_powers = vec![vec![1.]];
    for _ in 1..=n {
        p_powers.push(bernstein_product(p_powers.last().unwrap(), &map_p));
        one_powers.push(bernstein_product(
            one_powers.last().unwrap(),
            &map_one_minus,
        ));
    }
    let mut homogeneous = vec![Vec::new(); dimension + 1];
    for axis in 0..=dimension {
        let coeffs: Vec<f64> = if axis == dimension {
            span.weights.clone()
        } else {
            span.control_points
                .iter()
                .zip(&span.weights)
                .map(|(p, w)| p[axis] * w)
                .collect()
        };
        let mut composed = vec![0.; n * (piece.values.len() - 1) + 1];
        for (i, coefficient) in coeffs.iter().enumerate() {
            let term = bernstein_product(&p_powers[i], &one_powers[n - i]);
            for (target, value) in composed.iter_mut().zip(term) {
                *target += value * binomial(n, i) * coefficient;
            }
        }
        homogeneous[axis] = composed;
    }
    let degree = homogeneous[0].len() - 1;
    check(degree <= 25, "Composed degree exceeds 25")?;
    budget_controls(degree + 1)?;
    let weights = homogeneous[dimension].clone();
    numeric(
        weights.iter().all(|w| *w >= 1e-12 && *w <= 1e12),
        "Composed weights left the positive window",
    )?;
    let control_points = (0..=degree)
        .map(|i| {
            (0..dimension)
                .map(|axis| homogeneous[axis][i] / weights[i])
                .collect()
        })
        .collect();
    let domain = piece.domain;
    let curve = Curve {
        degree,
        knots: std::iter::repeat_n(domain[0], degree + 1)
            .chain(std::iter::repeat_n(domain[1], degree + 1))
            .collect(),
        control_points,
        weights,
        periodic: false,
    };
    curve.validate()?;
    Ok(curve)
}

#[cfg(feature="codec")]
#[path="reparameterization_preimages.rs"]
mod preimages;
#[cfg(feature="codec")]
pub use preimages::{bound_reparameterization_preimages};

#[cfg(feature="codec")]
#[path="reparameterization_residual.rs"]
mod reparameterization_residual;

#[cfg(feature="codec")]
#[path="reparameterization_retention.rs"]
mod reparameterization_retention;
#[cfg(feature="codec")]
pub use reparameterization_retention::{certify_reparameterized_curve_retention,certify_reparameterized_surface_section_retention};

#[cfg(feature="codec")]
#[path="reparameterization_materialization.rs"]
mod reparameterization_materialization;
#[cfg(feature="codec")]
pub use reparameterization_materialization::{materialize_reparameterized_curve_bounded};
