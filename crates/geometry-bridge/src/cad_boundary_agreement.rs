//! Read-only geometric boundary diagnostics, including structurally valid
//! inputs that ordinary sampled geometry admission rejects.
use super::{Result, Value, field};
use nurbs_core::curve_surface_agreement::Status;
use value_codec::json;
pub fn diagnose(v: Value) -> Result<Value> {
    let model: brep_core::Model = field(&v, "model")?;
    let max_cells: usize = field(&v, "maxCells")?;
    let report = brep_core::boundary_agreement::verify(&model, max_cells)?;
    let mismatch = report
        .uses
        .iter()
        .filter(|u| u.status == Status::Mismatch)
        .count();
    let unresolved = report
        .uses
        .iter()
        .filter(|u| u.status == Status::Unresolved)
        .count();
    let uses=report.uses.iter().map(|u|json!({
        "face":u.face,"wire":u.wire,"coedge":u.coedge,"edge":u.edge,
        "status":match u.status {Status::WithinTolerance=>"within-tolerance",Status::Mismatch=>"mismatch",Status::Unresolved=>"unresolved"},
        "parameter":u.witness,"distanceIntervalMm":u.witness_distance
    })).collect::<Vec<_>>();
    Ok(
        json!({"method":"interval-curve-surface-agreement","scope":"edge-surface-correspondence",
        "solidGeometryStatus":"not-certified","complete":unresolved==0,
        "allWithinTolerance":report.complete,"uses":uses,"mismatchCount":mismatch,"unresolvedCount":unresolved,
        "cells":report.cells,"maxCells":max_cells,"toleranceMm":model.tolerance_mm}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dispatch_distinguishes_complete_mismatch_and_budget_exhaustion() {
        let model = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
        let partial =
            crate::dispatch(json!({"op":"cad_boundary_agreement","model":model,"maxCells":1}))
                .unwrap();
        assert!(!field::<bool>(&partial, "complete").unwrap());
        assert!(!field::<bool>(&partial, "allWithinTolerance").unwrap());
        assert_eq!(field::<usize>(&partial, "unresolvedCount").unwrap(), 23);
        let valid =
            crate::dispatch(json!({"op":"cad_boundary_agreement","model":model,"maxCells":4096}))
                .unwrap();
        assert!(field::<bool>(&valid, "complete").unwrap());
        assert!(field::<bool>(&valid, "allWithinTolerance").unwrap());
        let mut bad = model.clone();
        for row in &mut bad.faces[0].surface.control_points {
            for cp in row {
                cp[2] += 0.125;
            }
        }
        assert!(bad.validate().is_err());
        let before = value_codec::to_value(&bad).unwrap();
        let result =
            crate::dispatch(json!({"op":"cad_boundary_agreement","model":bad,"maxCells":4096}))
                .unwrap();
        assert!(field::<bool>(&result, "complete").unwrap());
        assert!(!field::<bool>(&result, "allWithinTolerance").unwrap());
        assert_eq!(field::<usize>(&result, "mismatchCount").unwrap(), 4);
        assert_eq!(
            field::<String>(&result, "solidGeometryStatus").unwrap(),
            "not-certified"
        );
        assert_eq!(value_codec::to_value(&bad).unwrap(), before);
        let uses: Vec<Value> = field(&result, "uses").unwrap();
        for u in uses.iter().filter(|u| u["status"] == json!("mismatch")) {
            assert_eq!(u["face"], json!(0));
            let distance: [f64; 2] = field(u, "distanceIntervalMm").unwrap();
            assert!(distance[0] > bad.tolerance_mm && distance[0] <= 0.125 && distance[1] >= 0.125);
        }
    }
}
