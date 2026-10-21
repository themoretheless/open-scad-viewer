//! Actual original curved/curved section; open sheets, no closed-solid claim.
#[path = "support/fiber_cut_fixture.rs"]
mod fixture;
use nurbs_core::ss_intersection as ss;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::args().nth(1).ok_or("Need output directory")?;
    let (source, cutter) = fixture::pair();
    let out = brep_core::face_cut::partition_fiber_at_face(
        &source,
        0,
        &cutter,
        0,
        [[0., 1.], [0.375, 0.625], [0.2, 0.55], [0.3, 0.7]],
        ss::FiberAxis::FirstU,
        1e-8,
        100000,
        1000000,
    )?;
    let saved = value_codec::to_string(&out.partition.model)?;
    let restored: brep_core::Model = value_codec::from_str(&saved)?;
    if restored != out.partition.model {
        return Err("Saved model changed on reload".into());
    }
    let edges = out
        .partition
        .edges
        .iter()
        .map(|&id| {
            value_codec::json!({
                "edge":id,"curve":restored.edges[id].curve,"vertices":restored.edges[id].vertices,
                "points":restored.edges[id].vertices.map(|v|restored.vertices[v].point),
                "uses":restored.loops.iter().flat_map(|l|&l.coedges).filter(|c|c.edge==id).count()
            })
        })
        .collect::<Vec<_>>();
    let manifest = value_codec::json!({
        "tracePieces":out.trace_pieces,"cells":out.cells,"evaluations":out.evaluations,"exactWork":out.partition.exact_work,
        "rootErrorUpperMm":out.root_error_upper_mm,"curves":edges,
        "endpoints":out.endpoints,"allBoundaryRootsCovered":out.boundary_coverage.iter().all(|(_,r)|r.complete),
        "actualOriginalSurfaces":[source.faces[0].surface,cutter.faces[0].surface],
        "bothSectorsRetained":true,"savedReloadEqual":true,"completeSurfaceIntersectionClaim":false,
        "closedSolidClaim":false,"cornerClaim":false
    });
    std::fs::create_dir_all(&directory)?;
    let path = std::path::Path::new(&directory);
    std::fs::write(path.join("model.json"), saved)?;
    std::fs::write(
        path.join("manifest.json"),
        value_codec::to_string_pretty(&manifest)?,
    )?;
    println!(
        "{}",
        value_codec::to_string(&value_codec::json!({"tracePieces":out.trace_pieces,"cells":out.cells,
        "segments":out.partition.edges.len(),"rootErrorUpperMm":out.root_error_upper_mm}))?
    );
    Ok(())
}
