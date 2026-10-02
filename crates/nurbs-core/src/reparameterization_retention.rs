//! Whole-domain retention of R(u) against C(map(u)). Mean-value cells use
//! outward center values and an enclosure of the derivative difference. Every source
//! and result knot span participates; incomplete work never accepts.
use super::certify_reparameterization;
use super::preimages::Map;
use crate::curve::Curve;
use crate::distance_bounds::{Interval as I, box_distance};
use crate::{Result, check};
use value_codec::{Value, json};

fn derivative(curve: &Curve, parameter: I) -> Result<Vec<I>> {
    let mut bounds = vec![[f64::INFINITY, f64::NEG_INFINITY]; curve.control_points[0].len()];
    for span in curve.degree..curve.control_points.len() {
        if curve.knots[span] >= curve.knots[span + 1] {
            continue;
        }
        let a = parameter.lo.max(curve.knots[span]);
        let b = parameter.hi.min(curve.knots[span + 1]);
        // An isolated adjacent endpoint contributes no derivative on the cell.
        if a >= b {
            continue;
        }
        let jets = crate::curve_jets::enclose(curve, span, [a, b])?;
        let extent = I::point(b).sub(I::point(a))?;
        for (bound, value) in bounds.iter_mut().zip(&jets[1]) {
            let value = value.div(extent)?;
            bound[0] = bound[0].min(value.lo);
            bound[1] = bound[1].max(value.hi);
        }
    }
    bounds.into_iter().map(|[a, b]| I::new(a, b)).collect()
}

/// Prove numerical retention over the complete map domain, without asserting
/// an exact algebraic identity. Exhausted work and unresolved precision return
/// `accepted:false`; malformed definitions return an input error.
pub fn certify_reparameterized_curve_retention(
    source: &Curve,
    mapping: &Value,
    result: &Curve,
    tolerance: f64,
    max_cells: usize,
    max_map_evaluations: usize,
) -> Result<Value> {
    source.validate()?;
    result.validate()?;
    check(
        !source.periodic && !result.periodic,
        "Retention requires open fundamental covers",
    )?;
    check(
        source.control_points[0].len() == result.control_points[0].len(),
        "Retention curve dimensions must agree",
    )?;
    check(
        tolerance.is_finite() && tolerance > 0.,
        "Retention tolerance must be finite and positive",
    )?;
    check(
        max_cells > 0
            && max_cells <= 1_000_000
            && max_map_evaluations > 0
            && max_map_evaluations <= 1_000_000,
        "Retention budgets must be in 1..1000000",
    )?;
    let certificate = certify_reparameterization(mapping, None)?;
    let domain: [f64; 2] = value_codec::from_value(certificate["domain"].clone())
        .map_err(|e| crate::input(e.to_string()))?;
    let range: [f64; 2] = value_codec::from_value(certificate["range"].clone())
        .map_err(|e| crate::input(e.to_string()))?;
    check(
        result.domain() == domain,
        "Result must cover the complete map domain",
    )?;
    let source_domain = source.domain();
    check(
        range[0] >= source_domain[0] && range[1] <= source_domain[1],
        "Map range lies outside the source domain",
    )?;
    let map = Map::parse(mapping)?;
    let mut pending = vec![domain];
    let mut cells = 0;
    let mut map_evaluations = 0;
    let mut error_upper: f64 = 0.;
    let mut status = "within_tolerance";
    let mut witness = None;
    let mut unresolved_reason: Option<String> = None;
    while let Some([a, b]) = pending.pop() {
        if cells >= max_cells {
            status = "unresolved_cell_budget";
            break;
        }
        cells += 1;
        let mid = a + (b - a) * 0.5;
        if mid <= a || mid >= b {
            status = "unresolved_parameter_precision";
            break;
        }
        let attempt = (|| -> Result<(f64, f64)> {
            let mapped_center =
                map.enclosure(I::point(mid), &mut map_evaluations, max_map_evaluations)?;
            let center_source =
                crate::curve_surface_agreement::curve_bounds(source, mapped_center)?;
            let center_result =
                crate::curve_surface_agreement::curve_bounds(result, I::point(mid))?;
            let (center_lower, _) = box_distance(&center_source, &center_result)?;
            let (mapped, map_derivative) =
                map.jet(I::new(a, b)?, &mut map_evaluations, max_map_evaluations)?;
            let source_derivative = derivative(source, mapped)?;
            let result_derivative = derivative(result, I::new(a, b)?)?;
            let radius = I::new(a, b)?.sub(I::point(mid))?;
            let differences = center_source
                .into_iter()
                .zip(center_result)
                .zip(source_derivative.into_iter().zip(result_derivative))
                .map(|((s, r), (ds, dr))| {
                    s.sub(r)?.add(ds.mul(map_derivative)?.sub(dr)?.mul(radius)?)
                })
                .collect::<Result<Vec<_>>>()?;
            let zero = vec![I::point(0.); differences.len()];
            Ok((center_lower, box_distance(&differences, &zero)?.1))
        })();
        let (lower, upper) = match attempt {
            Ok(bounds) => bounds,
            Err(error) if error.code == crate::RESOURCE_LIMIT => {
                let _ = error;
                status = "unresolved_map_budget";
                break;
            }
            Err(error) => {
                unresolved_reason = Some(error.to_string());
                status = "unresolved_numeric_enclosure";
                break;
            }
        };
        if lower > tolerance {
            status = "mismatch";
            witness = Some(mid);
            break;
        }
        if upper <= tolerance {
            error_upper = error_upper.max(upper);
        } else {
            pending.push([mid, b]);
            pending.push([a, mid]);
        }
    }
    let accepted = status == "within_tolerance" && pending.is_empty();
    Ok(
        json!({"certificate":{"operation":"reparameterized-curve-retention",
        "method":"outward-center-plus-whole-cell-chain-rule-difference","accepted":accepted,"exact":false,
        "status":status,"tolerance":tolerance,"errorUpper":if accepted {Some(error_upper)} else {None},
        "cells":cells,"mapEvaluations":map_evaluations,"maxCells":max_cells,"maxMapEvaluations":max_map_evaluations,
        "witnessParameter":witness,"unresolvedReason":unresolved_reason,"domain":domain,"mapCertificate":certificate}}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn map() -> Value {
        json!({"pieces":[{"domain":[0.,1.],"range":[0.,1.],"controlValues":[0.,0.25,1.],"weights":[1.,1.,1.]}]})
    }
    fn line() -> Curve {
        crate::primitives::line([0., 0., 0.], [1., 0., 0.]).unwrap()
    }
    #[test]
    fn whole_composition_bound_accepts_the_materialized_polynomial() {
        let source = line();
        let materialized =
            super::super::materialize_reparameterized_curve(&source, &map(), None).unwrap();
        let result: Curve = value_codec::from_value(materialized["curve"].clone()).unwrap();
        let report =
            certify_reparameterized_curve_retention(&source, &map(), &result, 1e-5, 10000, 50000)
                .unwrap();
        assert_eq!(report["certificate"]["accepted"], true, "{report:?}");
        assert!(report["certificate"]["errorUpper"].as_f64().unwrap() <= 1e-5);
        assert_eq!(report["certificate"]["exact"], false);
    }
    #[test]
    fn unchanged_endpoints_and_midpoint_do_not_hide_an_interior_distortion() {
        let source = line();
        let identity = json!({"pieces":[{"domain":[0.,1.],"range":[0.,1.],"controlValues":[0.,1.],"weights":[1.,1.]}]});
        let mut result = source.elevate(3).unwrap();
        result.control_points[1][1] = 0.25;
        result.control_points[2][1] = -0.25;
        for t in [0., 0.5, 1.] {
            assert!((result.evaluate(t).unwrap().point[1]).abs() < 1e-15);
        }
        let report = certify_reparameterized_curve_retention(
            &source, &identity, &result, 1e-4, 10000, 50000,
        )
        .unwrap();
        assert_eq!(report["certificate"]["accepted"], false);
        assert_eq!(report["certificate"]["status"], "mismatch");
    }
    #[test]
    fn all_c0_spans_are_covered_and_exhaustion_is_not_acceptance() {
        let source = Curve {
            degree: 1,
            knots: vec![0., 0., 0.5, 1., 1.],
            control_points: vec![vec![0., 0.], vec![0.5, 1.], vec![1., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let identity = json!({"pieces":[{"domain":[0.,1.],"range":[0.,1.],"controlValues":[0.,1.],"weights":[1.,1.]}]});
        let report =
            certify_reparameterized_curve_retention(&source, &identity, &source, 1e-9, 1000, 10000)
                .unwrap();
        assert_eq!(report["certificate"]["accepted"], true, "{report:?}");
        let refused =
            certify_reparameterized_curve_retention(&source, &identity, &source, 1e-9, 1, 10000)
                .unwrap();
        assert_eq!(refused["certificate"]["accepted"], false);
        assert_eq!(refused["certificate"]["errorUpper"], Value::Null);
        let refused =
            certify_reparameterized_curve_retention(&source, &identity, &source, 1e-9, 1000, 1)
                .unwrap();
        assert_eq!(refused["certificate"]["status"], "unresolved_map_budget");
    }
}
