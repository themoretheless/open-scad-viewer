use super::{Result, Value, field};
use value_codec::json;
pub fn inspect(v: Value) -> Result<Value> {
    let model: brep_core::Model = field(&v, "model")?;
    let groups: [Vec<usize>; 2] = field(&v, "faceGroups")?;
    let origin: [f64; 3] = field(&v, "origin")?;
    let direction: [f64; 3] = field(&v, "direction")?;
    let tolerance_uv: f64 = field(&v, "toleranceUv")?;
    let tolerance_mm: f64 = field(&v, "toleranceMm")?;
    let config: Value = field(&v, "limits")?;
    let normal: Value = field(&v, "normalAudit")?;
    let cells: usize = field(&v, "maxDistanceCells")?;
    let domains: usize = field(&v, "maxDistanceDomainCells")?;
    let limits = brep_core::material_wall::Limits {
        material: brep_core::material_segment::Limits {
            volume: super::cad_solid_distance::validity_limits(&field::<Value>(
                &config, "validity",
            )?)?,
            point_cells: field(&config, "pointCells")?,
            point_domain_cells: field(&config, "pointDomainCells")?,
            segment_cells: field(&config, "segmentCells")?,
            segment_domain_cells: field(&config, "segmentDomainCells")?,
        },
        distance_cells: cells,
        distance_domain_cells: domains,
        normal_spans: field(&normal, "maxSpans")?,
    };
    let r = brep_core::material_wall::inspect(
        &model,
        [&groups[0], &groups[1]],
        origin,
        direction,
        tolerance_mm,
        tolerance_uv,
        field(&normal, "maxSineSquared")?,
        limits,
    )?;
    let alignment = match r.candidate.aligned {
        Some(true) => "angular-tolerance",
        Some(false) => "oblique",
        None => "unresolved",
    };
    let candidate = super::cad_material_chord::render(
        &model,
        &r.candidate.chord,
        origin,
        direction,
        tolerance_uv,
        &config,
        &Some(normal),
        Some(super::cad_material_chord::normal_evidence(&r.candidate)),
        alignment,
    )?;
    let d = &r.clearance;
    Ok(
        json!({"method":"bounded-material-wall","scope":"aligned-material-chords-between-selected-face-unions",
        "faceGroups":groups,"toleranceMm":tolerance_mm,"maxDistanceCells":cells,"maxDistanceDomainCells":domains,
        "intervalMm":r.interval_mm,"converged":r.converged,"reason":r.reason,"candidate":candidate,
        "clearance":{"lowerBoundMm":d.lower_bound_mm,"upperBoundMm":d.upper_bound_mm,
            "totalPairs":d.pairs,"evaluatedPairs":d.evaluated_pairs,"cells":d.cells,"domainCells":d.domain_cells,
            "converged":d.converged,"reason":d.reason}}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_dispatch_keeps_chord_proof_and_global_face_coverage_separate() {
        let m = brep_core::cuboid([0.; 3], [10.; 3]).unwrap();
        let face = |x| {
            m.faces
                .iter()
                .position(|f| f.surface.control_points.iter().flatten().all(|p| p[0] == x))
                .unwrap()
        };
        let mut input =
            super::super::cad_material_segment::tests::request(&m, [-2., 5., 5.], [14., 0., 0.]);
        input["op"] = json!("cad_material_wall");
        input["faceGroups"] = json!([[face(0.)], [face(10.)]]);
        input["normalAudit"] = json!({"maxSineSquared":1e-6,"maxSpans":100});
        input["toleranceMm"] = json!(1e-5);
        input["maxDistanceCells"] = json!(10000);
        input["maxDistanceDomainCells"] = json!(1000000);
        let r = crate::dispatch(input.clone()).unwrap();
        let d: [f64; 2] = field(&r, "intervalMm").unwrap();
        assert!(d[0] <= 10. && d[1] >= 10.);
        assert_eq!(r["converged"], json!(true));
        assert_eq!(r["candidate"]["sourceModel"], input["model"]);
        assert_eq!(r["faceGroups"], input["faceGroups"]);
        assert_eq!(r["clearance"]["totalPairs"], json!(1));
        input["normalAudit"]["maxSpans"] = json!(1);
        let r = crate::dispatch(input.clone()).unwrap();
        assert_eq!(r["intervalMm"], json!(null));
        assert_eq!(r["reason"], json!("candidate-normal-unresolved"));
        input["faceGroups"] = json!([[face(0.)], [face(0.)]]);
        assert!(crate::dispatch(input).is_err());
    }
}
