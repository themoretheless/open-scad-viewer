//! Fresh original-definition diagnostics on the retained-source closed candidate.
#[path = "support/contact_graft_fixture.rs"]
mod fixture;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture::build();
    let graft = brep_core::contact_graft::attach(
        &fixture.source,
        &fixture.patch,
        &fixture.mapping,
        &fixture.targets,
        1e-8,
        100000,
        1000000,
    )?;
    let limits = brep_core::face_contacts::Limits {
        pairs: 10000,
        cells: 100000,
        domain_cells: 1000000,
        cells_per_pair: 1000,
        domain_cells_per_pair: 10000,
    };
    let faces = brep_core::face_injectivity::inspect_with_linear(&graft.model, 1024, 100000)?;
    println!(
        "faces_injective={} spans={} linear_cells={}",
        faces.all_faces_injective, faces.spans, faces.linear_cells
    );
    for f in &faces.faces {
        println!(
            "face={} injective={} linear={}",
            f.face,
            f.result.as_ref().is_some_and(|r| r.proven),
            f.linear.as_ref().is_some_and(|r| r.certified)
        );
    }
    let pairs = brep_core::face_contacts::inspect(&graft.model, 1e-8, limits)?;
    println!(
        "pairs_classified={} total={} checked={} next={:?} cells={} domains={}",
        pairs.all_pairs_classified,
        pairs.total_pairs,
        pairs.pairs.len(),
        pairs.next_pair,
        pairs.cells,
        pairs.domain_cells
    );
    let mut tube_cells = 0;
    let mut tube_exact = 0;
    let mut qualified = 0;
    let mut remaining = Vec::new();
    for pair in &pairs.pairs {
        println!("pair={:?} reason={}", pair.faces, pair.reason);
        if pair.reason == "pair-unresolved" {
            let attempt = brep_core::boundary_contact_tube::inspect_pair(
                &graft.model,
                pair.faces,
                100000 - pairs.cells - faces.linear_cells - tube_cells,
                1000000 - tube_exact,
            )?;
            tube_cells += attempt.cells;
            tube_exact += attempt.exact_work;
            if let Some(c) = attempt.certificate {
                qualified += 1;
                println!(
                    "tube_pair={:?} edges={:?} error_upper={}",
                    pair.faces, c.edges, c.boundary_error_upper
                );
            } else {
                remaining.push(pair.faces);
            }
        }
    }
    println!(
        "tube_qualified={qualified} remaining={remaining:?} added_cells={tube_cells} added_exact={tube_exact} exact_absence_claim=false"
    );
    Ok(())
}
