//! Original finite-interval coverage and material preconditions; no wall minimum claim.
use brep_core::{Model, material_chord};
fn report(
    model: &Model,
    origin: [f64; 3],
    interval: [f64; 2],
) -> Result<value_codec::Value, Box<dyn std::error::Error>> {
    let r = material_chord::inspect_finite_interval(
        model,
        origin,
        [1., 0., 0.],
        interval,
        &[[1., 0.07, 0.03], [-1., 0.09, 0.02]],
        1e-8,
        100000,
        1000000,
        1000000,
    )?;
    let roots = r
        .interval
        .crossings
        .iter()
        .map(|c| value_codec::json!({"face":c.face,"uv":c.uv,"parameter":c.parameter}))
        .collect::<Vec<_>>();
    let uncertain = r
        .interval
        .unresolved
        .iter()
        .map(|u| value_codec::json!({"face":u.face,"uv":u.uv,"reason":u.reason}))
        .collect::<Vec<_>>();
    Ok(
        value_codec::json!({"decision":format!("{:?}",r.decision),"origin":origin,"direction":[1.,0.,0.],"interval":interval,"intervalLengthMm":r.interval.length_mm,"intervalCoverageComplete":r.interval.coverage_complete,"boundaryFreeInterval":r.interval.boundary_free,"crossings":roots,"unresolvedIntervalCells":uncertain,"originParity":r.parity.as_ref().and_then(|p|p.parity),"trimRegionsValid":r.trim_regions.as_ref().map(|t|t.all_valid),"exactClosure":r.closure.as_ref().map(|c|c.exact.all_equal && c.exact.all_joins_exact),"volumePreconditions":r.volume_preconditions,"volumeRefusal":r.volume_refusal,"boundaryAbsenceProven":r.boundary.as_ref().map(|b|b.proven),"boundaryUnresolvedPairs":r.boundary.as_ref().map(|b|b.intersections.pairs.pairs.iter().filter(|p|p.reason=="pair-unresolved").map(|p|p.faces).collect::<Vec<_>>()),"nextBoundaryPair":r.boundary.as_ref().and_then(|b|b.intersections.pairs.next_pair),"totalBoundaryPairs":r.boundary.as_ref().map(|b|b.intersections.pairs.total_pairs),"materialIntervalQualified":r.material_interval_qualified,"cells":r.cells,"domainCells":r.domain_cells,"wholeWallThicknessClaim":false}),
    )
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::PathBuf::from(std::env::args().nth(1).ok_or("Need output directory")?);
    let cube = brep_core::cuboid([0.; 3], [2.; 3])?;
    let outer = brep_core::cuboid([0.; 3], [10.; 3])?;
    let inner = brep_core::cuboid([2.; 3], [8.; 3])?;
    let cavity = brep_core::operations::boolean(&outer, &inner, "difference")?;
    let cases = value_codec::json!({"cubeInterior":report(&cube,[1.;3],[-0.99,0.99])?,"cavityCrossing":report(&cavity,[1.,4.,5.],[-0.99,8.99])?,"cavityLocalWall":report(&cavity,[1.,4.,5.],[-0.99,0.99])?});
    std::fs::create_dir_all(&root)?;
    std::fs::write(
        root.join("cases.json"),
        value_codec::to_string_pretty(&cases)?,
    )?;
    println!("{}", value_codec::to_string_pretty(&cases)?);
    Ok(())
}
