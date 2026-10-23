use super::{Result, Value, field};
use value_codec::json;

pub fn inspect(v: Value) -> Result<Value> {
    let model: brep_core::Model = field(&v, "model")?;
    let origin: [f64; 3] = field(&v, "origin")?;
    let direction: [f64; 3] = field(&v, "direction")?;
    let tolerance_uv: f64 = field(&v, "toleranceUv")?;
    let tolerance_mm: f64 = field(&v, "toleranceMm")?;
    let config: Value = field(&v, "limits")?;
    let normal: Value = field(&v, "normalAudit")?;
    let coverage: Value = field(&v, "coverageLimits")?;
    let cells: usize = field(&v, "maxDistanceCells")?;
    let domains: usize = field(&v, "maxDistanceDomainCells")?;
    let r = brep_core::material_wall_coverage::inspect(
        &model,
        origin,
        direction,
        tolerance_mm,
        tolerance_uv,
        field(&normal, "maxSineSquared")?,
        brep_core::material_wall_coverage::Limits {
            wall: super::cad_material_wall::decode_limits(&config, &normal, cells, domains)?,
            pairs: field(&coverage, "maxFacePairs")?,
            plane_controls: field(&coverage, "maxPlaneControls")?,
            normal_spans: field(&coverage, "maxNormalSpans")?,
        },
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
    Ok(json!({"method":"bounded-whole-material-wall",
        "scope":"aligned-material-chords-over-all-original-face-pairs",
        "toleranceMm":tolerance_mm,"maxDistanceCells":cells,"maxDistanceDomainCells":domains,
        "coverageLimits":coverage,"intervalMm":r.interval_mm,"converged":r.converged,"reason":r.reason,
        "candidate":candidate,"coverage":{"totalPairs":r.total_pairs,
        "enumerationComplete":r.enumeration_complete,"lowerBoundMm":r.lower_bound_mm,
        "planeControls":r.plane_controls,"normalSpans":r.normal_spans,"cells":r.cells,"domainCells":r.domain_cells,
        "pairs":r.pairs.iter().map(|p|json!({"faces":p.faces,"lowerBoundMm":p.lower_bound_mm,"reason":p.reason})).collect::<Vec<_>>()}}))
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(crate) fn request(
        model: &brep_core::Model,
        origin: [f64; 3],
        direction: [f64; 3],
    ) -> Value {
        let mut v = super::super::cad_material_segment::tests::request(model, origin, direction);
        v["op"] = json!("cad_whole_wall");
        v["normalAudit"] = json!({"maxSineSquared":1e-6,"maxSpans":100});
        v["toleranceMm"] = json!(1e-5);
        v["maxDistanceCells"] = json!(10000);
        v["maxDistanceDomainCells"] = json!(1000000);
        v["coverageLimits"] =
            json!({"maxFacePairs":10000,"maxPlaneControls":10000,"maxNormalSpans":10000});
        v
    }
    #[test]
    fn whole_wall_dispatch_binds_all_faces_and_budget_refusals_to_the_source() {
        let model = brep_core::cuboid([0.; 3], [10., 20., 30.]).unwrap();
        let mut v = request(&model, [-2., 10., 15.], [14., 0., 0.]);
        let r = crate::dispatch(v.clone()).unwrap();
        assert_eq!(r["converged"], json!(true));
        assert_eq!(r["coverage"]["totalPairs"], json!(21));
        assert_eq!(r["candidate"]["sourceModel"], v["model"]);
        assert_eq!(r["coverageLimits"], v["coverageLimits"]);
        v["coverageLimits"]["maxFacePairs"] = json!(1);
        let r = crate::dispatch(v.clone()).unwrap();
        assert_eq!(r["converged"], json!(false));
        assert_eq!(r["reason"], json!("face-pair-limit"));
        assert_eq!(r["coverage"]["enumerationComplete"], json!(false));
        v["coverageLimits"]["maxFacePairs"] = json!(0);
        assert!(crate::dispatch(v).is_err());
    }
}
