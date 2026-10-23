use super::{Result, Value, field};
use value_codec::json;

pub fn measure(v: Value) -> Result<Value> {
    let a: brep_core::Model = field(&v, "a")?;
    let b: brep_core::Model = field(&v, "b")?;
    let tolerance_mm: f64 = field(&v, "toleranceMm")?;
    let tolerance_uv: f64 = field(&v, "toleranceUv")?;
    let max_cells: usize = field(&v, "maxCells")?;
    let max_domain_cells: usize = field(&v, "maxDomainCells")?;
    let r = brep_core::shell_distance::distance(
        &a,
        &b,
        tolerance_mm,
        tolerance_uv,
        max_cells,
        max_domain_cells,
    )?;
    let witness = r.witness.as_ref();
    Ok(json!({
        "method":"interval-trimmed-face-pairs",
        "scope":"boundary-shells-bounded-joins",
        "containment":"not-classified",
        "distanceIntervalMm":[r.lower_bound_mm,r.upper_bound_mm],
        "faces":r.faces,
        "parameters":witness.and_then(|w|w.parameters),
        "points":witness.and_then(|w|w.points),
        "pointEnclosures":witness.and_then(|w|w.point_enclosures.as_ref()),
        "converged":r.converged,"reason":r.reason,
        "pairs":r.pairs,"evaluatedPairs":r.evaluated_pairs,
        "cells":r.cells,"maxCells":max_cells,
        "domainCells":r.domain_cells,"maxDomainCells":max_domain_cells,
        "toleranceMm":tolerance_mm,"toleranceUv":tolerance_uv
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shell_distance_dispatch_keeps_scope_and_nullable_witnesses() {
        let a = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
        let b = brep_core::cuboid([3., 0., 0.], [4., 1., 1.]).unwrap();
        for (budget, complete) in [(1, false), (100000, true)] {
            let value=crate::dispatch(json!({"op":"cad_shell_distance","a":a,"b":b,"toleranceMm":0.001,"toleranceUv":1e-7,"maxCells":budget,"maxDomainCells":budget*10})).unwrap();
            assert_eq!(
                field::<String>(&value, "scope").unwrap(),
                "boundary-shells-bounded-joins"
            );
            assert_eq!(
                field::<String>(&value, "containment").unwrap(),
                "not-classified"
            );
            assert_eq!(field::<bool>(&value, "converged").unwrap(), complete);
            let bounds: Vec<Option<f64>> = field(&value, "distanceIntervalMm").unwrap();
            assert!(bounds[0].unwrap() <= 2.);
            if complete {
                assert!(bounds[1].unwrap() >= 2.);
                assert!(
                    field::<Option<[usize; 2]>>(&value, "faces")
                        .unwrap()
                        .is_some()
                )
            } else {
                assert!(bounds[1].is_none());
                assert!(
                    field::<Option<Vec<Vec<f64>>>>(&value, "points")
                        .unwrap()
                        .is_none()
                )
            }
        }
    }
}
