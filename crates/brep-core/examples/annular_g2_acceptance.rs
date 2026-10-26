//! Fresh full-boundary G2 and material audit for the native rounded annulus.
use brep_core::{
    analytic::g2_annular, boundary_embedding, face_contacts, sweep_cap_contacts, sweep_smoothness,
    volume_validity,
};
use sha2::{Digest, Sha256};
use value_codec::json;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::path::PathBuf::from(std::env::args().nth(1).expect("Output directory"));
    std::fs::create_dir_all(&output)?;
    let contacts = face_contacts::Limits {
        pairs: 10000,
        cells: 100000,
        domain_cells: 1000000,
        cells_per_pair: 1000,
        domain_cells_per_pair: 10000,
    };
    let limits = volume_validity::Limits {
        boundary: boundary_embedding::Limits {
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
    let mut results = Vec::new();
    for (name, outer, inner, height, extent) in [
        ("dyadic-ring", 8., 4., 16., 0.5),
        ("ui-default-ring", 10., 7., 20., 0.5),
        ("decimal-ring", 5.15, 2.3, 10.3, 0.25),
        ("small-decimal-transition", 10., 7., 20., 0.1),
    ] {
        let now = std::time::Instant::now();
        let built = g2_annular::cylinder(outer, inner, height, extent, 1e-8)?;
        let jets = sweep_smoothness::inspect_profile(&built.model, &[], 2_000_000)?;
        std::fs::write(
            output.join(format!("{name}-model.json")),
            value_codec::to_string(&built.model)?,
        )?;
        let global = volume_validity::inspect_sweep_with_projections(
            &built.model,
            1e-8,
            limits,
            100000,
            &[],
            sweep_cap_contacts::Budgets {
                max_walls: 1024,
                max_exact_work: 1000000,
                max_chart_cells: 1000,
                max_trim_pairs: 10000,
                max_trim_cells: 100000,
                max_trim_domain_cells: 1000000,
            },
            &built.projection_hints(),
        )?;
        let pairs = &global.boundary.intersections.pairs;
        eprintln!(
            "faces={} agreement={} joins={} trim={} hulls={} unresolved_faces={:?}",
            global.boundary.intersections.faces.all_faces_injective,
            global.boundary.agreement.all_equal,
            global.boundary.agreement.all_joins_exact,
            global.boundary.trim.all_valid,
            global.boundary.hull_contacts.len(),
            global
                .boundary
                .intersections
                .faces
                .faces
                .iter()
                .filter(|f| !f.result.as_ref().is_some_and(|r| r.proven)
                    && !f.linear.as_ref().is_some_and(|r| r.certified))
                .collect::<Vec<_>>()
        );
        eprintln!(
            "{name}: G2={}/{} global={} faces={} pairs={}/{} next={:?} unresolved={:?}",
            jets.profile.g2_certified,
            jets.station.g2_certified,
            global.proven,
            built.model.faces.len(),
            pairs.pairs.len() + pairs.grouped_pairs,
            pairs.total_pairs,
            pairs.next_pair,
            pairs
                .pairs
                .iter()
                .filter(|p| p.reason == "pair-unresolved")
                .map(|p| p.faces)
                .collect::<Vec<_>>()
        );
        let (step, _, _) = brep_core::export_step_v5(&built.model)?;
        let file = format!("{name}.step");
        std::fs::write(output.join(&file), &step)?;
        let sha = Sha256::digest(step.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let row = json!({"name":name,"file":file,"sha256":sha,"parameters":{"outer":outer,"inner":inner,"height":height,"extent":extent},"faces":built.model.faces.len(),"edges":built.model.edges.len(),"profileG2":jets.profile.g2_certified,"stationG2":jets.station.g2_certified,"exactWork":jets.total_exact_work,"radialWallThicknessLower":built.radial_wall_thickness_lower,"globalProven":global.proven,"facesInjective":global.boundary.intersections.faces.all_faces_injective,"allPairsClassified":pairs.all_pairs_classified,"totalPairs":pairs.total_pairs,"coveredPairs":pairs.pairs.len()+pairs.grouped_pairs,"nextPair":pairs.next_pair,"milliseconds":now.elapsed().as_millis() as u64});
        results.push(row);
        std::fs::write(
            output.join("native.json"),
            value_codec::to_string_pretty(&json!({"cases":results}))?,
        )?;
    }
    Ok(())
}
