//! Closed quarter-sector control; only two circular joins claim G2.
use nurbs_core::radius_transition::Options;
use sha2::{Digest, Sha256};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::args().nth(1).ok_or("Need output directory")?;
    let o = Options::default();
    let out = brep_core::radius_transition_body::build(o)?;
    let saved = value_codec::to_string(&out.model)?;
    let restored: brep_core::Model = value_codec::from_str(&saved)?;
    if restored != out.model {
        return Err("Saved model changed on reload".into());
    }
    let (step, _, _) = brep_core::export_step_v5(&restored)?;
    let digest = |b: &[u8]| {
        Sha256::digest(b)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    let d = o.radii_mm[1] - o.radii_mm[0];
    // Integral smoothstep5²=181/462; source bands have the same authored width.
    let volume = std::f64::consts::PI / 4.
        * o.width_mm
        * (o.radii_mm[0].powi(2)
            + o.radii_mm[1].powi(2)
            + o.radii_mm[0] * o.radii_mm[1]
            + d * d * 181. / 462.);
    let manifest = value_codec::json!({"stepSha256":digest(step.as_bytes()),"modelSha256":digest(saved.as_bytes()),
        "cells":out.cells,"coefficientWork":out.transition.coefficient_work,"exactWork":out.exact_work,
        "radiiMm":o.radii_mm,"transitionWidthMm":o.width_mm,
        "radiusErrorUpperMm":out.transition.radius.radial_error_upper_mm,
        "circularRadiusErrorUpperMm":out.transition.circular_radius_error_upper_mm,
        "stationPlaneErrorUpperMm":out.transition.station_plane_error_upper_mm,
        "widthErrorUpperMm":out.transition.width_error_upper_mm,
        "seamUpper":out.transition.seams.iter().map(|s|s.upper).collect::<Vec<_>>(),
        "smoothEdges":out.smooth_edges,"nativeClosureWithinTolerance":out.closure.closure.status==brep_core::watertight::Closure::WithinTolerance,
        "trimRegionsValid":out.trims.all_valid,"nativeTransitionAdmitted":out.transition.admitted,
        "expected":{"solids":1,"faces":11,"edges":21,"vertices":12,"boundsMm":[[0.,0.,-0.1],[0.3,0.3,0.2]],"volumeMm3":volume},
        "closedSolidClaim":true,"wholeBodyG2Claim":false,"generalCornerClaim":false,
        "constantRollingBallRadiusClaim":false});
    std::fs::create_dir_all(&directory)?;
    let path = std::path::Path::new(&directory);
    std::fs::write(path.join("model.json"), saved)?;
    std::fs::write(path.join("transition.step"), step)?;
    std::fs::write(
        path.join("manifest.json"),
        value_codec::to_string_pretty(&manifest)?,
    )?;
    println!("{}", value_codec::to_string_pretty(&manifest)?);
    Ok(())
}
