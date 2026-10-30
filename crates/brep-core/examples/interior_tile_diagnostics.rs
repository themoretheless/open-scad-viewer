//! Actual closed central-face subdivision. No material/solid admission.
use brep_core::region_replacement::{InteriorTileLimits, propose_interior_tile};
use nurbs_core::{
    curve::basis,
    interval_eval::{self, Interval},
};
fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    assert_eq!(args.len(), 4, "graft.json audit.json output_directory");
    let read = |p: &str| {
        value_codec::from_str::<value_codec::Value>(&std::fs::read_to_string(p).unwrap()).unwrap()
    };
    let graft = read(&args[1]);
    let audit = read(&args[2]);
    let source: brep_core::Model = value_codec::from_value(graft["model"].clone()).unwrap();
    let face = graft["centralFace"].as_u64().unwrap() as usize;
    let rectangle: [[f64; 2]; 2] =
        value_codec::from_value(audit["insideRefinementBox"]["rectangle"].clone()).unwrap();
    let uv: [f64; 2] = value_codec::from_value(audit["witnessUV"].clone()).unwrap();
    let s = &source.faces[face].surface;
    let prototype = s
        .trim([
            rectangle[0][0],
            rectangle[0][1],
            rectangle[1][0],
            rectangle[1][1],
        ])
        .unwrap();
    let counts = [
        prototype.control_points.len(),
        prototype.control_points[0].len(),
    ];
    let pole = counts.map(|n| n / 2);
    let u = basis(
        prototype.degree_u,
        &prototype.knots_u,
        counts[0],
        uv[0],
        false,
    )
    .unwrap()
    .basis;
    let v = basis(
        prototype.degree_v,
        &prototype.knots_v,
        counts[1],
        uv[1],
        false,
    )
    .unwrap()
    .basis;
    let denominator = u
        .iter()
        .enumerate()
        .map(|(i, a)| {
            v.iter()
                .enumerate()
                .map(|(j, b)| a * b * prototype.weights[i][j])
                .sum::<f64>()
        })
        .sum::<f64>();
    let weight = u[pole[0]] * v[pole[1]] * prototype.weights[pole[0]][pole[1]] / denominator;
    let before = s.evaluate(uv[0], uv[1]).unwrap().point;
    let delta = (1e-6 - before.iter().copied().fold(f64::INFINITY, f64::min)) / weight;
    assert!(delta.is_finite() && delta > 0.);
    let directory = std::path::Path::new(&args[3]);
    std::fs::create_dir_all(directory).unwrap();
    eprintln!("phase=construct-and-qualify-tile");
    let out = match propose_interior_tile(
        &source,
        face,
        rectangle,
        [delta; 3],
        1e-8,
        [source.tolerance_mm, 1e-5, 1e-4],
        InteriorTileLimits {
            domain_cells: 100000,
            agreement_cells: 100000,
            seam_cells: 100000,
            exact_work: 1000000,
            graph_coefficients: Some(2_000_000_000),
        },
    ) {
        Ok(out) => out,
        Err(e) => {
            std::fs::write(directory.join("refused.json"),value_codec::to_string(&value_codec::json!({"reason":format!("{e}"),"rectangle":rectangle,"delta":delta,"materialQualified":false})).unwrap()).unwrap();
            eprintln!("{e}");
            std::process::exit(1)
        }
    };
    eprintln!("phase=tile-closed-g2-qualified; inspecting trims and material");
    let tile = &out.model.faces[out.tile_face].surface;
    let after = tile.evaluate(uv[0], uv[1]).unwrap().point;
    let bounds = interval_eval::evaluate_surface_interval(
        tile,
        Interval::new(uv[0], uv[0]).unwrap(),
        Interval::new(uv[1], uv[1]).unwrap(),
    )
    .unwrap();
    let witness_inside = brep_core::face_domain::FaceDomain::new(&out.model, out.tile_face, 1e-8)
        .unwrap()
        .classify([[uv[0], uv[0]], [uv[1], uv[1]]], 10000)
        .unwrap();
    let trim = brep_core::face_domain::audit_trim_regions(&out.model, 1e-8, 10000, 100000, 100000)
        .unwrap();
    let tile_box = brep_core::face_domain::inspect_box_containment(
        &out.model,
        out.tile_face,
        [[-source.tolerance_mm, 10. + source.tolerance_mm]; 3],
        1e-8,
        50000,
        1000000,
    )
    .unwrap();
    let remaining_box = brep_core::face_domain::inspect_box_containment(
        &out.model,
        face,
        [[-source.tolerance_mm, 10. + source.tolerance_mm]; 3],
        1e-8,
        50000,
        1000000,
    )
    .unwrap();
    let (step, _, _) = brep_core::export_step_v5(&out.model).unwrap();
    std::fs::write(directory.join("candidate.step"), step).unwrap();
    let report = value_codec::json!({"model":out.model,"originalFace":face,"tileFace":out.tile_face,"rectangle":rectangle,
        "tilePoles":counts,"liftedPole":out.lifted_pole,"displacement":[delta,delta,delta],"sharedEdges":out.shared_edges,
        "coefficientWork":out.coefficient_work,"closure":format!("{:?}",out.closure.closure.status),"allTrimsValid":trim.all_valid,
        "seams":out.seams.iter().map(|r|value_codec::json!({"positions":r.positions.proven,"positionUpper":r.positions.upper,"decision":format!("{:?}",r.geometric.decision),"cells":r.geometric.cells,"orientation":format!("{:?}",r.geometric.orientation),"normalUpper":r.geometric.branches.iter().filter_map(|b|b.normal_upper).collect::<Vec<_>>(),"tensorUpper":r.geometric.branches.iter().filter_map(|b|b.tensor_upper).collect::<Vec<_>>()})).collect::<Vec<_>>(),
        "witnessUV":uv,"before":before,"after":after,"witnessInside":format!("{:?}",witness_inside.location),"witnessBounds":bounds.iter().map(|b|[b.lo,b.hi]).collect::<Vec<_>>(),
        "tileBoxContainment":format!("{:?}",tile_box.decision),"remainingBoxContainment":format!("{:?}",remaining_box.decision),
        "remainingWitnessUV":remaining_box.witness_uv,"remainingWitnessBounds":remaining_box.witness_bounds,
        "originalSurfaceUnchanged":source.faces[face].surface==out.model.faces[face].surface,
        "persistentNaming":out.model.persistent_naming_complete(),"materialQualified":false,"solidQualified":false});
    std::fs::write(
        directory.join("report.json"),
        value_codec::to_string(&report).unwrap(),
    )
    .unwrap();
    println!(
        "closure={:?} trims={} seams={} faces={} tile_box={:?} remaining_box={:?} before={before:?} after={after:?}",
        out.closure.closure.status,
        trim.all_valid,
        out.seams.len(),
        out.model.faces.len(),
        tile_box.decision,
        remaining_box.decision
    );
}
