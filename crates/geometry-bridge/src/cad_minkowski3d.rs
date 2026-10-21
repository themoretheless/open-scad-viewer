use crate::{Result, field};
use brep_core::minkowski3d::{self, Certificate, SourceMesh};
use value_codec::{Value, json};
fn precision(v: &Value) -> Result<bool> {
    match v.get("meshPrecision") {
        None => Ok(false),
        Some(p) if p.as_str() == Some("f64") => Ok(false),
        Some(p) if p.as_str() == Some("f32") => Ok(true),
        _ => Err(crate::input("Minkowski mesh precision must be f32 or f64")),
    }
}
fn topology(r: &minkowski3d::RoundingTopologyReport) -> Value {
    json!({"status":"proved","precision":r.precision,"identity":r.identity,"pairs":r.pairs,"separatedPairs":r.separated,"sharedEdges":r.shared_edges,"sharedVertices":r.shared_vertices,"wholeLinearHomotopy":true})
}
pub fn dispatch(v: Value) -> Result<Value> {
    let f32_mesh = precision(&v)?;
    let op = v["op"].as_str();
    if matches!(
        op,
        Some("nurbs_source_atlas_certify" | "nurbs_source_atlas_verify")
    ) {
        use minkowski3d::curved;
        let m = field(&v, "model")?;
        let c = if op == Some("nurbs_source_atlas_certify") {
            curved::certify_source_atlas(&m, field(&v, "depth")?)?
        } else {
            let c = field(&v, "certificate")?;
            curved::verify_source_atlas(&m, &c)?;
            c
        };
        return Ok(json!({"status":"verified","certificate":c}));
    }
    if matches!(op, Some("minkowski_nurbs_certify" | "minkowski_nurbs_verify")) {
        use minkowski3d::curved;
        let a = field(&v, "a")?;
        let b = field(&v, "b")?;
        let r = if op == Some("minkowski_nurbs_verify") {
            let c = field(&v, "certificate")?;
            curved::verify(&a, &b, &c, f32_mesh)?
        } else {
            curved::certify(&a, &b, field(&v, "maximumSourceDeviationMm")?, f32_mesh)?
        };
        let t = topology(&r.nominal.rounding_topology);
        let deviation = r.maximum_set_deviation();
        let source_deviations = r.source_surface_deviations_mm();
        let approximation_topology = r.approximation_topology();
        let source_approximation_topology = r.source_approximation_topology();
        let (max_dev_exact, max_dev_mm) = match deviation {
            Some((exact, mm)) => (Some(exact), Some(mm)),
            None => (None, None),
        };
        return Ok(
            json!({"status":"verified","scope":curved::SCOPE,"certificate":r.certificate,"mesh":{"positions":r.nominal.positions,"indices":r.nominal.indices},"volumeMm3":r.nominal.volume_mm3,"exactVolumeMm3":r.nominal.exact_volume_mm3,"maximumCoordinateRoundingMm":r.nominal.maximum_coordinate_rounding_mm,"curvedSourceErrorCertified":true,"displayCoordinatesExact":r.nominal.maximum_coordinate_rounding_mm==0.,"roundingTopology":t,"sourceAuthority":"authored-oriented-face-images","nominalIsExactCurvedSum":false,"maximumSetDeviationMm":max_dev_mm,"maximumSetDeviationExactMm":max_dev_exact,"sourceSurfaceDeviationMm":source_deviations,"approximationTopology":approximation_topology,"sourceApproximationTopology":source_approximation_topology}),
        );
    }
    let a: SourceMesh = field(&v, "a")?;
    let b: SourceMesh = field(&v, "b")?;
    let r = if op == Some("minkowski_3d_verify") {
        let c: Certificate = field(&v, "certificate")?;
        if f32_mesh {
            minkowski3d::verify_f32(&a, &b, &c)?
        } else {
            minkowski3d::verify(&a, &b, &c)?
        }
    } else {
        if f32_mesh {
            minkowski3d::certify_f32(&a, &b)?
        } else {
            minkowski3d::certify(&a, &b)?
        }
    };
    Ok(
        json!({"status":"verified","scope":minkowski3d::SCOPE,"certificate":r.certificate,"mesh":{"positions":r.positions,"indices":r.indices},"volumeMm3":r.volume_mm3,"exactVolumeMm3":r.exact_volume_mm3,"maximumCoordinateRoundingMm":r.maximum_coordinate_rounding_mm,"curvedSourceErrorCertified":false,"displayCoordinatesExact":r.maximum_coordinate_rounding_mm==0.,"roundingTopology":topology(&r.rounding_topology),"maximumSetDeviationMm":r.maximum_coordinate_rounding_mm*2.}),
    )
}
