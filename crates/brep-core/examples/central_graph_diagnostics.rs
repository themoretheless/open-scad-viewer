use brep_core::region_replacement as r;
fn main() {
    let m = brep_core::cuboid([0.; 3], [10.; 3]).unwrap();
    let requests: Vec<_> = m
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| e.vertices.contains(&0))
        .enumerate()
        .map(|(i, (edge, _))| r::EdgeRadius {
            edge,
            radius_mm: [0.1, 0.2, 0.35][i],
        })
        .collect();
    let limits = r::StripLimits {
        tolerances: [1e-7, 1e-5, 1e-4],
        tolerance_uv: 1e-8,
        max_geometry_work: 10000,
        max_cells: 300000,
        max_exact: 1000000,
        max_coefficients: 2000000,
    };
    let strips = r::build_trihedral_strips(&m, 0, 0, &requests, 0.1, 2., limits).unwrap();
    let collars = r::prepare_trihedral_collars(
        &m,
        &strips,
        [1e-7, 1e-5, 1e-4],
        1e-8,
        0.001,
        100000,
        20000000,
        1000000,
    )
    .unwrap();
    if let Some(path) = std::env::var_os("OSV_CENTRAL_GRAPH_CANDIDATE") {
        let path = std::path::PathBuf::from(path);
        let data: value_codec::Value =
            value_codec::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let stored = data["boundaries"].as_array().unwrap();
        assert_eq!(stored.len(), collars.boundaries.len());
        for (saved, current) in stored.iter().zip(&collars.boundaries) {
            let curve: nurbs_core::curve::Curve =
                value_codec::from_value(saved["curve"].clone()).unwrap();
            let pc: nurbs_core::curve::Curve =
                value_codec::from_value(saved["originalPcurve"].clone()).unwrap();
            let surface: nurbs_core::surface::Surface =
                value_codec::from_value(saved["originalSurface"].clone()).unwrap();
            assert_eq!(curve, current.curve, "Original source curve changed");
            assert_eq!(
                pc, current.original_pcurve,
                "Original source pcurve changed"
            );
            assert_eq!(
                surface, current.original_surface,
                "Original support changed"
            );
        }
        let candidate = r::CentralGraph {
            surface: value_codec::from_value(data["surface"].clone()).unwrap(),
            pcurves: stored
                .iter()
                .map(|b| value_codec::from_value(b["candidatePcurve"].clone()).unwrap())
                .collect(),
            sampled_jet_residual: [f64::INFINITY; 6],
        };
        let cells = std::env::var("OSV_CENTRAL_GRAPH_CELLS")
            .ok()
            .map(|s| s.parse().unwrap())
            .unwrap_or(600000);
        if std::env::var_os("OSV_CENTRAL_GRAPH_TRIM").is_some() {
            let context =
                cad_predicates::ToleranceContext::from_brep_tolerance_mm(m.tolerance_mm).unwrap();
            let proof = r::trim_central_graph(
                &context,
                &collars,
                &candidate,
                1e-8,
                brep_core::trimmed_face_recipe::Limits {
                    pairs: 10000,
                    region_cells: cells.min(100000),
                    domain_cells: cells.min(100000),
                    agreement_cells: cells.min(100000),
                },
            )
            .unwrap();
            let output = value_codec::json!({"trimmedFaceQualified":proof.face.is_some(),"reason":proof.reason,
                "regionValid":proof.region.valid,"winding":proof.region.winding,"agreementCells":proof.agreement_cells,
                "edges":proof.face.as_ref().map(|f|f.edges().len()),"closedBody":false});
            std::fs::write(
                path.with_extension("trim.json"),
                value_codec::to_string(&output).unwrap(),
            )
            .unwrap();
            println!("{}", value_codec::to_string(&output).unwrap());
            return;
        }
        if std::env::var_os("OSV_CENTRAL_GRAPH_GRAFT").is_some() {
            let graft =
                r::propose_trihedral_graft(&m, &strips, &collars, &candidate, 1e-8).unwrap();
            let report =
                brep_core::watertight::inspect_with_exact(&graft.model, cells.min(100000), 1000000)
                    .unwrap();
            let trim = brep_core::face_domain::audit_trim_regions(
                &graft.model,
                1e-8,
                10000,
                cells.min(100000),
                cells.min(100000),
            )
            .unwrap();
            // This example's ORIGINAL material is the explicit [0,10]^3 cube.
            // XYZ containment is only a necessary condition for general solids.
            let box_proof = brep_core::face_domain::inspect_box_containment(
                &graft.model,
                graft.central_face,
                [[-m.tolerance_mm, 10. + m.tolerance_mm]; 3],
                1e-8,
                cells.min(50000),
                1000000,
            )
            .unwrap();
            let step_written = if std::env::var_os("OSV_CENTRAL_GRAPH_STEP").is_some() {
                let (text, _, _) = brep_core::export_step_v5(&graft.model).unwrap();
                std::fs::write(path.with_extension("graft.step"), text).unwrap();
                true
            } else {
                false
            };
            let output = value_codec::json!({"model":graft.model,"centralFace":graft.central_face,
                "stripFaces":graft.strip_faces,"farStopFaces":graft.far_stop_faces,
                "farStops":"planar-sharp-stops-not-G2","partitionCells":graft.partition_cells,
                "closure":format!("{:?}",report.closure.status),"allTrimsValid":trim.all_valid,
                "centralBoxContainment":format!("{:?}",box_proof.decision),"boxCells":box_proof.cells,
                "boxDomainCells":box_proof.domain_cells,"boxWitnessUV":box_proof.witness_uv,
                "boxWitnessBounds":box_proof.witness_bounds,
                "stepWritten":step_written,
                "trimFaces":trim.faces.iter().enumerate().map(|(i,f)|match f {
                    Some(f)=>value_codec::json!({"face":i,"valid":f.valid,"reason":f.reason,
                        "loops":f.loops.iter().map(|l|format!("{:?}",l.outcome())).collect::<Vec<_>>()}),
                    None=>value_codec::json!({"face":i,"valid":false,"reason":"unvisited"}),
                }).collect::<Vec<_>>(),
                "materialQualified":false,"solidQualified":false});
            std::fs::write(
                path.with_extension("graft.json"),
                value_codec::to_string(&output).unwrap(),
            )
            .unwrap();
            println!(
                "closure={:?} trims={} faces={} edges={} vertices={}",
                report.closure.status,
                trim.all_valid,
                graft.model.faces.len(),
                graft.model.edges.len(),
                graft.model.vertices.len()
            );
            return;
        }
        if std::env::var_os("OSV_CENTRAL_GRAPH_CORRELATED").is_some() {
            let selected = std::env::var("OSV_CENTRAL_GRAPH_SEAM")
                .ok()
                .map(|v| v.parse::<usize>().unwrap());
            let coefficient_limit = std::env::var("OSV_CENTRAL_GRAPH_COEFFICIENTS")
                .ok()
                .map(|v| v.parse::<usize>().unwrap())
                .unwrap_or(2_000_000_000);
            let mut results = Vec::new();
            for (i, c) in collars.boundaries.iter().enumerate() {
                if selected.is_some_and(|selected| i != selected) {
                    continue;
                }
                eprintln!("Correlated original-graph seam {i}: starting");
                let start = std::time::Instant::now();
                let proof = nurbs_core::surface_g2_pcurves::inspect_owned_graph(
                    &c.curve,
                    [&c.original_pcurve, &candidate.pcurves[i]],
                    [&c.original_surface, &candidate.surface],
                    [1e-7, 1e-5, 1e-4],
                    cells.min(100000),
                    1000000,
                    coefficient_limit,
                );
                let item = match proof {
                    Ok(proof) => {
                        let report = proof.owned.report;
                        value_codec::json!({"seam":i,"decision":format!("{:?}",report.geometric.decision),
                            "positionsProven":report.positions.proven,"positionCells":report.positions.cells,
                            "cells":report.geometric.cells,"coefficientWork":proof.coefficient_work,
                            "acceptedCells":proof.accepted_cells,"lastUnresolvedReason":proof.last_unresolved_reason,
                            "orientation":format!("{:?}",report.geometric.orientation),
                            "branches":report.geometric.branches.iter().map(|b|value_codec::json!({
                                "normalUpper":b.normal_upper,"shapeTensorUpper":b.tensor_upper})).collect::<Vec<_>>(),
                            "seconds":start.elapsed().as_secs_f64()})
                    }
                    Err(error) => value_codec::json!({"seam":i,"error":format!("{error:?}"),
                        "seconds":start.elapsed().as_secs_f64()}),
                };
                eprintln!("{}", value_codec::to_string(&item).unwrap());
                results.push(item);
            }
            let output = value_codec::json!({"closedBody":false,"originalDefinitionsMatch":true,"seams":results});
            std::fs::write(
                path.with_extension("correlated.json"),
                value_codec::to_string(&output).unwrap(),
            )
            .unwrap();
            println!("{}", value_codec::to_string(&output).unwrap());
            return;
        }
        let report = r::inspect_central_graph(
            &collars,
            &candidate,
            [[1, -1, 0], [1, 1, -2]],
            [1e-7, 1e-5, 1e-4],
            cells,
            1000000,
        )
        .unwrap();
        let seams=report.seams.iter().map(|s|value_codec::json!({"positionsProven":s.positions.proven,"positionUpperMm":s.positions.upper,"positionCells":s.positions.cells,"geometricCells":s.geometric.cells,"regularityProven":s.geometric.regularity_proven,
            "decision":format!("{:?}",s.geometric.decision),"branches":s.geometric.branches.iter().map(|b|value_codec::json!({"normalUpper":b.normal_upper,"shapeTensorUpper":b.tensor_upper})).collect::<Vec<_>>()
        })).collect::<Vec<_>>();
        let output = value_codec::json!({"admitted":report.admitted,"closedBody":false,"originalDefinitionsMatch":true,
            "interiorCertified":report.interior.certified,"interiorReason":report.interior.reason,"interiorCells":report.interior.cells,"rimSimple":report.rim.proven_simple,"cells":report.cells,"exactWork":report.exact_work,"seams":seams});
        std::fs::write(
            path.with_extension("native.json"),
            value_codec::to_string(&output).unwrap(),
        )
        .unwrap();
        println!("{}", value_codec::to_string(&output).unwrap());
        return;
    }
    let a = 1. / 2_f64.sqrt();
    let b = 1. / 6_f64.sqrt();
    let c = 1. / 3_f64.sqrt();
    let requested: Vec<usize> = std::env::args()
        .skip(1)
        .map(|s| s.parse().expect("knot interval count"))
        .collect();
    let requested = if requested.is_empty() {
        vec![2, 4, 6]
    } else {
        requested
    };
    let degree: usize = std::env::var("OSV_CENTRAL_GRAPH_DEGREE")
        .ok()
        .map(|s| s.parse().expect("graph degree"))
        .unwrap_or(5);
    for spans in requested {
        let options = r::GraphOptions {
            frame: [[a, -a, 0.], [b, b, -2. * b], [c, c, c]],
            degree,
            spans,
            align_junctions: true,
            junction_refinement: std::env::var("OSV_CENTRAL_GRAPH_JUNCTION_REFINEMENT")
                .ok()
                .map(|s| s.parse().expect("junction refinement"))
                .unwrap_or(0.),
            stations_per_side: std::env::var("OSV_CENTRAL_GRAPH_STATIONS")
                .ok()
                .map(|s| s.parse().expect("base stations"))
                .unwrap_or(129),
            regularization: std::env::var("OSV_CENTRAL_GRAPH_REGULARIZATION")
                .ok()
                .map(|s| s.parse().expect("regularization"))
                .unwrap_or(1e-12),
        };
        let degree = options.degree;
        let guides = std::env::var_os("OSV_CENTRAL_GRAPH_GUIDES")
            .map(|path| {
                let data: value_codec::Value =
                    value_codec::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
                data.as_array()
                    .unwrap()
                    .iter()
                    .map(|guide| r::GraphGuide {
                        point: std::array::from_fn(|k| guide["point"][k].as_f64().unwrap()),
                        weight: guide["weight"].as_f64().unwrap(),
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let anchors = std::env::var_os("OSV_CENTRAL_GRAPH_ANCHORS")
            .map(|path| {
                let data: value_codec::Value =
                    value_codec::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
                data.as_array()
                    .unwrap()
                    .iter()
                    .map(|p| std::array::from_fn(|k| p[k].as_f64().unwrap()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let fairness = std::env::var("OSV_CENTRAL_GRAPH_FAIRNESS")
            .ok()
            .map(|s| s.parse::<f64>().unwrap())
            .unwrap_or(0.);
        let mesh = std::env::var_os("OSV_CENTRAL_GRAPH_MESH")
            .map(|path| {
                let data: value_codec::Value =
                    value_codec::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
                ["u", "v"].map(|axis| {
                    data[axis]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|node| {
                            (
                                node[0].as_f64().unwrap(),
                                node[1].as_u64().unwrap() as usize,
                            )
                        })
                        .collect::<Vec<_>>()
                })
            })
            .unwrap_or_else(|| [Vec::new(), Vec::new()]);
        match r::propose_central_graph_constrained(
            &collars,
            options,
            &guides,
            fairness,
            [&mesh[0], &mesh[1]],
            &anchors,
        ) {
            Ok(candidate) => {
                println!(
                    "degree={degree} spans={spans} sampled_jet_residual={:?}",
                    candidate.sampled_jet_residual
                );
                if let Some(path) = std::env::var_os("OSV_CENTRAL_GRAPH_EVIDENCE") {
                    let directory = std::path::PathBuf::from(path);
                    std::fs::create_dir_all(&directory).unwrap();
                    let boundaries=collars.boundaries.iter().zip(&candidate.pcurves).map(|(c,p)|value_codec::json!({
                        "curve":c.curve,"originalSurface":c.original_surface,"originalPcurve":c.original_pcurve,"candidatePcurve":p
                    })).collect::<Vec<_>>();
                    let data = value_codec::json!({"surface":candidate.surface,"boundaries":boundaries,
                        "sampledJetResidual":candidate.sampled_jet_residual,"degree":degree,"spans":spans,"baseStationsPerSide":options.stations_per_side,"junctionRefinement":options.junction_refinement,"regularization":options.regularization,
                        "hardAnchors":anchors,"extraKnots":mesh,"fairness":fairness,"guides":guides.iter().map(|g|value_codec::json!({"point":g.point,"weight":g.weight})).collect::<Vec<_>>(),"closedBody":false,"admitted":false});
                    std::fs::write(
                        directory.join(format!("candidate-{degree}-{spans}.json")),
                        value_codec::to_string(&data).unwrap(),
                    )
                    .unwrap();
                }
                if std::env::var_os("OSV_CENTRAL_GRAPH_INSPECT").is_some() {
                    let report = r::inspect_central_graph(
                        &collars,
                        &candidate,
                        [[1, -1, 0], [1, 1, -2]],
                        [1e-7, 1e-5, 1e-4],
                        600000,
                        1000000,
                    )
                    .unwrap();
                    println!(
                        "degree={degree} spans={spans} admitted={} interior={} cells={} seams={:?}",
                        report.admitted,
                        report.interior.certified,
                        report.cells,
                        report
                            .seams
                            .iter()
                            .map(|s| (s.positions.proven, s.positions.upper, s.geometric.decision))
                            .collect::<Vec<_>>()
                    );
                }
            }
            Err(e) => println!("degree={degree} spans={spans} refused={e:?}"),
        }
    }
}
