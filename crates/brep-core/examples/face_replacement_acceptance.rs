//! Independent STEP control for a changed surface with preserved owned topology.
use sha2::{Digest, Sha256};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::PathBuf::from(std::env::args().nth(1).expect("Output directory"));
    std::fs::create_dir_all(&path)?;
    let source = brep_core::cuboid([0.; 3], [8.; 3])?;
    let face = source
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == 8.)
        })
        .unwrap();
    let original = &source.faces[face].surface;
    let surface = nurbs_core::surface::Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        let mut p = original
                            .evaluate(i as f64 * 0.5, j as f64 * 0.5)
                            .unwrap()
                            .point
                            .to_vec();
                        if i == 1 && j == 1 {
                            p[2] -= 0.4;
                        }
                        p
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let pcurves = std::iter::once(source.faces[face].outer)
        .chain(source.faces[face].holes.iter().copied())
        .map(|w| {
            source.loops[w]
                .coedges
                .iter()
                .map(|c| c.pcurve.clone())
                .collect()
        })
        .collect::<Vec<_>>();
    let built =
        brep_core::face_replacement::replace(&source, face, &surface, &pcurves, 1e-8, 1000000)?;
    assert_eq!(built.model.edges, source.edges);
    assert_eq!(built.model.vertices, source.vertices);
    let closure = brep_core::watertight::inspect_with_exact(&built.model, 100000, 1000000)?;
    assert_eq!(
        closure.closure.status,
        brep_core::watertight::Closure::WithinTolerance
    );
    let depth = 8. - surface.evaluate(0.5, 0.5)?.point[2];
    assert!((depth - 0.1).abs() < 1e-12);
    let (step, _, _) = brep_core::export_step_v5(&built.model)?;
    let sha = Sha256::digest(step.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    std::fs::write(path.join("replaced-surface.step"), step)?;
    // Integral of B(2,1) is 1/3 on each coordinate. Expected volume comes
    // directly from the independent design polynomial, not a native mass.
    let expected = 512. - 64. * 0.4 / 9.;
    let manifest = value_codec::json!({"schema":"cad-roadmap-step/1","toleranceMm":1e-6,"relativeVolumeTolerance":1e-10,
        "parts":[{"name":"owned-boundary-surface-replacement","file":"replaced-surface.step","sha256":sha,
            "nativeProven":false,"nativeClosureWithinTolerance":true,"centerDepthMm":depth,
            "expected":{"boundsMm":[[0.,0.,0.],[8.,8.,8.]],"volumeMm3":expected,"solids":1,"edges":12,"vertices":8}}]});
    std::fs::write(
        path.join("manifest.json"),
        value_codec::to_string_pretty(&manifest)?,
    )?;
    println!(
        "center_depth_mm={depth} expected_volume_mm3={expected} owned_topology_preserved=true closure=true"
    );
    Ok(())
}
