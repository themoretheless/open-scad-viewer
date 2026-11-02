//! Fresh trimmed-face curvature inspection from a stored native model.
use brep_core::face_curvature as c;
use c::Limits;
fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    assert!(
        args.len() == 5 || args.len() == 7,
        "report.json face maximum_per_mm output.json [u v]"
    );
    let data: value_codec::Value =
        value_codec::from_str(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();
    let model: brep_core::Model = value_codec::from_value(data["model"].clone()).unwrap();
    let face = args[2].parse::<usize>().unwrap();
    let maximum = args[3].parse::<f64>().unwrap();
    let hints = if args.len() == 7 {
        vec![[
            args[5].parse::<f64>().unwrap(),
            args[6].parse::<f64>().unwrap(),
        ]]
    } else {
        vec![]
    };
    let out = c::inspect(
        &model,
        face,
        maximum,
        1e-8,
        Limits {
            cells: 50000,
            domain_cells: 1000000,
            shape_cells: 1000000,
        },
        &hints,
    )
    .unwrap();
    let report = value_codec::json!({"face":face,"sourceFaceIdentity":model.1.faces[face],"maximumAbsolutePrincipalCurvaturePerMm":maximum,
        "decision":format!("{:?}",out.decision),"reason":out.reason,"cells":out.cells,"domainCells":out.domain_cells,"shapeCells":out.shape_cells,
        "translationInvariantAxes":out.translation_invariant_axes,"unresolvedRegion":out.unresolved_region,"unresolvedCurvature":out.unresolved_curvature,"coveredCells":out.covered_cells,"omittedCells":out.omitted_cells,"absoluteCurvatureUpper":out.absolute_curvature_upper,
        "witnessUV":out.witness_uv,"witnessRegion":out.witness_region,"witnessCurvaturePerMm":out.witness_curvature,
        "scope":"Declared principal-curvature magnitude bound on this trimmed face only; no fillet radius law or solid admission","solidQualified":false});
    std::fs::write(&args[4], value_codec::to_string(&report).unwrap()).unwrap();
    println!(
        "decision={:?} reason={} cells={} domain={} shape={} witness={:?}",
        out.decision,
        out.reason,
        out.cells,
        out.domain_cells,
        out.shape_cells,
        out.witness_curvature
    );
}
