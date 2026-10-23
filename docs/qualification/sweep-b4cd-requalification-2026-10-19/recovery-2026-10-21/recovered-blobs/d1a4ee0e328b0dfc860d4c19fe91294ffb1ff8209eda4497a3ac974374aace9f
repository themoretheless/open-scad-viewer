//! A real retained face plus sewn nonplanar replacement, not a closure-only test.
#[path = "support/face_window_fixture.rs"]
mod fixture;
use sha2::{Digest, Sha256};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::path::PathBuf::from(std::env::args().nth(1).expect("Output directory"));
    std::fs::create_dir_all(&path)?;
    let (source, face, patch, boundary) = fixture::build()?;
    let built = brep_core::face_window::replace(&source, face, &patch, &boundary, 1e-8, 1000000)?;
    let exact = brep_core::boundary_agreement::verify_exact(&built.model, 1000000)?;
    let tolerances = brep_core::sweep_smoothness::geometric::Tolerances {
        position: 1e-7,
        normal: 1e-7,
        curvature: 1e-5,
    };
    let smooth = brep_core::sweep_smoothness::geometric::inspect_edges(
        &built.model,
        &built.edges,
        tolerances,
        100000,
    )?;
    println!(
        "whole_trim_g2={} complete={} seams={} cells={} exact_work={}",
        smooth.within_tolerance,
        smooth.complete,
        smooth.seams.len(),
        smooth.cells,
        smooth.exact_work
    );
    assert!(
        smooth.complete && smooth.within_tolerance,
        "All actual trimmed seams must pass whole-interval G2"
    );
    let contacts = brep_core::face_contacts::Limits {
        pairs: 10000,
        cells: 100000,
        domain_cells: 1000000,
        cells_per_pair: 1000,
        domain_cells_per_pair: 10000,
    };
    let limits = brep_core::volume_validity::Limits {
        boundary: brep_core::boundary_embedding::Limits {
            exact_work: 1000000,
            trim_pairs: 10000,
            trim_cells: 100000,
            trim_domain_cells: 1000000,
            spans: 1024,
            contacts,
        },
        nesting_pairs: 100,
        nesting_cells: 100000,
        nesting_domain_cells: 1000000,
        orientation_cells: 100000,
        orientation_domain_cells: 1000000,
        orientation_spans: 100,
    };
    let volume = brep_core::volume_validity::inspect(&built.model, 1e-8, limits)?;
    println!(
        "global={} exact_lifts={} exact_joins={} trims={} embedding={} faces_injective={} pairs_complete={} faces={} edges={} vertices={} work={}",
        volume.proven,
        exact.all_equal,
        exact.all_joins_exact,
        volume.boundary.trim.all_valid,
        volume.boundary.intersections.absence_proven,
        volume.boundary.intersections.faces.all_faces_injective,
        volume.boundary.intersections.pairs.all_pairs_classified,
        built.model.faces.len(),
        built.model.edges.len(),
        built.model.vertices.len(),
        built.cells
    );
    if !volume.proven {
        for f in &volume.boundary.intersections.faces.faces {
            println!(
                "face={} proven={}",
                f.face,
                f.result.as_ref().is_some_and(|r| r.proven)
            );
        }
        for p in &volume.boundary.intersections.pairs.pairs {
            println!("pair={:?} reason={}", p.faces, p.reason);
        }
    }
    // Closed ownership and exact seams are insufficient: a deeper patch
    // crosses the opposite wall while retaining those local certificates.
    let mut deep = patch.clone();
    deep.control_points[3][3][2] = 8. - 128.;
    let crossing = brep_core::face_window::replace(&source, face, &deep, &boundary, 1e-8, 1000000)?;
    let bad = brep_core::volume_validity::inspect(&crossing.model, 1e-8, limits)?;
    let deep_g2 = brep_core::sweep_smoothness::geometric::inspect_edges(
        &crossing.model,
        &crossing.edges,
        tolerances,
        100000,
    )?;
    println!(
        "opposite_wall_crossing_seams_g2={}",
        deep_g2.within_tolerance
    );
    assert!(
        deep_g2.complete && deep_g2.within_tolerance,
        "This negative control retains G2 and must fail the independent global gate"
    );
    println!(
        "opposite_wall_crossing_global={} exact_boundary={} trims={}",
        bad.proven, bad.boundary.agreement.all_equal, bad.boundary.trim.all_valid
    );
    assert!(
        !bad.proven,
        "Local seam certificates must not admit an opposite-wall crossing"
    );
    let (step, _, _) = brep_core::export_step_v5(&built.model)?;
    let sha = Sha256::digest(step.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    std::fs::write(path.join("window-dimple.step"), &step)?;
    std::fs::write(
        path.join("model.json"),
        value_codec::to_string(&built.model)?,
    )?;
    // Independent design integral: area 16, central tensor Bernstein coefficient
    // -1.024, and integral B(6,3)=1/7 in each coordinate. No measured native mass.
    let expected_volume = 512. - 16. * 1.024 / 49.;
    let manifest = value_codec::json!({"schema":"cad-roadmap-step/1","toleranceMm":1e-6,"relativeVolumeTolerance":1e-10,
        "parts":[{"name":"retained-face-window-dimple","file":"window-dimple.step","sha256":sha,
            "nativeProven":volume.proven,"wholeTrimG2":smooth.within_tolerance,"expected":{"boundsMm":[[0.,0.,0.],[8.,8.,8.]],"volumeMm3":expected_volume,"solids":1,"edges":16,"vertices":12}}]});
    std::fs::write(
        path.join("manifest.json"),
        value_codec::to_string_pretty(&manifest)?,
    )?;
    assert!(
        volume.proven,
        "A saved sewn candidate is not a certified solid"
    );
    Ok(())
}
