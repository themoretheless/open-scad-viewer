//! Curved-skin/end-plane surface construction; not a closed runout body.
#[path = "support/curved_contact_body_fixture.rs"]
mod fixture;
use nurbs_core::{
    surface::Axis,
    surface_blend::{self, Boundary, Options, Trace},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let f = fixture::build()?;
    let skin = &f.patch.faces[0].surface;
    let cap = &f
        .source
        .faces
        .iter()
        .find(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[1] == 7.)
        })
        .ok_or("No end plane")?
        .surface;
    let ud = [
        skin.knots_u[skin.degree_u],
        skin.knots_u[skin.control_points.len()],
    ];
    let vd = [
        skin.knots_v[skin.degree_v],
        skin.knots_v[skin.control_points[0].len()],
    ];
    let increasing = skin.evaluate((ud[0] + ud[1]) * 0.5, vd[1])?.point[1]
        > skin.evaluate((ud[0] + ud[1]) * 0.5, vd[0])?.point[1];
    let (mut lo, mut hi) = (vd[0], vd[1]);
    for _ in 0..48 {
        let mid = (lo + hi) * 0.5;
        if (skin.evaluate((ud[0] + ud[1]) * 0.5, mid)?.point[1] < 6.9) == increasing {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let at = (lo + hi) * 0.5;
    let mut target = skin.iso(Axis::V, at)?;
    for p in &mut target.control_points {
        p[0] += 0.1;
        p[1] = 7.;
        p[2] += 0.1;
    }
    let pc = nurbs_core::curve_pullback::project(
        &target,
        cap,
        nurbs_core::curve_pullback::Limits {
            tolerance: 1e-7,
            max_segments: 256,
            max_iterations: 32,
            agreement_cells: 100000,
        },
    )?
    .pcurve
    .ok_or("Curved target plane pullback unproven")?;
    let origin = &cap.control_points[0][0];
    let mut transverse = [0.; 2];
    for (k, a) in [0, 1].into_iter().enumerate() {
        let p = if a == 0 {
            &cap.control_points[1][0]
        } else {
            &cap.control_points[0][1]
        };
        let norm = (0..3).map(|j| (p[j] - origin[j]).powi(2)).sum::<f64>();
        let scale = if a == 0 {
            cap.knots_u[2] - cap.knots_u[1]
        } else {
            cap.knots_v[2] - cap.knots_v[1]
        };
        transverse[k] = ((p[0] - origin[0]) + (p[2] - origin[2])) / norm * scale;
    }
    let report = surface_blend::g2_to_plane_trace(
        skin,
        Trace { at, along: ud },
        cap,
        &pc,
        transverse,
        Options {
            boundaries: [
                if increasing {
                    Boundary::VEnd
                } else {
                    Boundary::VStart
                },
                Boundary::VStart,
            ],
            gains: [0.01, 0.1],
            ..Default::default()
        },
        [1e-7, 1e-5, 1e-4],
        100000,
        1000000,
    )?;
    println!(
        "curved runout: complete={} within={} cells={} exact={} patches={}",
        report.complete,
        report.within_tolerance,
        report.cells,
        report.exact_work,
        report.construction.patches.len()
    );
    for (patch, side, seam) in &report.support_seams {
        println!(
            "patch={patch} side={side} positions={} geometric={:?}",
            seam.positions.proven, seam.geometric
        );
    }
    if !report.within_tolerance {
        return Err("Original-support curved runout remains unproven".into());
    }
    let mut cells=report.cells;
    let mut injective=true;
    for patch in &report.construction.patches {
        let interior=nurbs_core::surface_linear_monotonicity::inspect_candidate(patch,100000-cells)?;
        cells+=interior.cells;injective&=interior.certified;
        println!("interior={interior:?}");
    }
    if let Some(directory)=std::env::args().nth(1) {
        let path=std::path::Path::new(&directory);std::fs::create_dir_all(path)?;
        std::fs::write(path.join("surfaces.json"),value_codec::to_string(&report.construction.patches)?)?;
        std::fs::write(path.join("supports.json"),value_codec::to_string(&[skin.clone(),cap.clone()])?)?;
        std::fs::write(path.join("target-pcurve.json"),value_codec::to_string(&pc)?)?;
        let manifest=value_codec::json!({"originalSupportSeamsG2":report.within_tolerance,
            "allPatchChartsInjective":injective,"cells":cells,"exactWork":report.exact_work,
            "sourceTraceAt":at,"sourceTraceAlong":ud,"planeTransverseUv":transverse,
            "sourceMidpointTargetYmm":6.9,"endPlaneYmm":7.0,
            "targetShiftXZmm":0.1,"closedBodyClaim":false,"prescribedRadiusClaim":false});
        std::fs::write(path.join("manifest.json"),value_codec::to_string_pretty(&manifest)?)?;
    }
    Ok(())
}
