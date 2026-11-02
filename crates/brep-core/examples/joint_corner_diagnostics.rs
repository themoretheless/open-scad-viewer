//! Joint strip/central reconstruction, no admission without fresh receipts.
use brep_core::{face_curvature as c, region_replacement as r};
fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    assert_eq!(args.len(), 2, "output_directory");
    let dir = std::path::Path::new(&args[1]);
    std::fs::create_dir_all(dir).unwrap();
    let save = |name: &str, v: value_codec::Value| {
        std::fs::write(dir.join(name), value_codec::to_string(&v).unwrap()).unwrap()
    };
    let source = brep_core::cuboid([0.; 3], [10.; 3]).unwrap();
    let requested = source
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| e.vertices.contains(&0))
        .enumerate()
        .map(|(i, (edge, _))| r::EdgeRadius {
            edge,
            radius_mm: [0.13, 0.2, 0.35][i],
        })
        .collect::<Vec<_>>();
    let striplimits = r::StripLimits {
        tolerances: [1e-7, 1e-5, 1e-4],
        tolerance_uv: 1e-8,
        max_geometry_work: 10000,
        max_cells: 300000,
        max_exact: 1000000,
        max_coefficients: 2000000,
    };
    eprintln!("phase=joint-strips");
    let strips = match r::build_trihedral_strips_with_profile(
        &source,
        0,
        0,
        &requested,
        0.5,
        2.,
        striplimits,
        0.214,
    ) {
        Ok(v) => v,
        Err(e) => {
            save(
                "refused.json",
                value_codec::json!({"phase":"strips","reason":format!("{e}"),"accepted":false}),
            );
            return;
        }
    };
    let mut receipts = Vec::new();
    let mut all = true;
    for (i, s) in strips.strips.iter().enumerate() {
        eprintln!("phase=strip-curvature-{i}");
        let curve = c::inspect(
            &s.patch.model,
            0,
            10.,
            1e-8,
            c::Limits {
                cells: 50000,
                domain_cells: 1000000,
                shape_cells: 1000000,
            },
            &[],
        )
        .unwrap();
        let material = brep_core::face_domain::inspect_box_containment(
            &s.patch.model,
            0,
            [[-1e-7, 10.0000001]; 3],
            1e-8,
            50000,
            1000000,
        )
        .unwrap();
        all &= curve.decision == c::Decision::Within
            && material.decision == brep_core::face_domain::BoxDecision::Within;
        receipts.push(value_codec::json!({"strip":i,"nominalRollingRadiusMm":s.radius_mm,"curvature":format!("{:?}",curve.decision),"upper":curve.absolute_curvature_upper,"witness":curve.witness_curvature,"box":format!("{:?}",material.decision),"wholeContactG2":s.g2.iter().all(|r|r.report.geometric.decision==nurbs_core::surface_join::Decision::WithinTolerance)}));
        save(
            "strips.json",
            value_codec::json!({"strips":receipts,"model":s.patch.model,"accepted":false,"newHandleFraction":0.214,"runoutMm":0.5,"originalNominalRadiiMm":[0.1,0.2,0.35],"newNominalRadiiMm":[0.13,0.2,0.35]}),
        );
    }
    if !all {
        eprintln!("phase=strip-guard-refusal");
        return;
    }
    eprintln!("phase=new-boundary-collars");
    let collars = r::prepare_trihedral_collars(
        &source,
        &strips,
        [1e-7, 1e-5, 1e-4],
        1e-8,
        0.001,
        100000,
        20000000,
        1000000,
    )
    .unwrap();
    let a = 1. / 2f64.sqrt();
    let b = 1. / 6f64.sqrt();
    let d = 1. / 3f64.sqrt();
    eprintln!("phase=fit-new-central");
    let degree = std::env::var("OSV_JOINT_DEGREE")
        .ok()
        .map(|s| s.parse().unwrap())
        .unwrap_or(7);
    let fairness = std::env::var("OSV_JOINT_FAIRNESS")
        .ok()
        .map(|s| s.parse().unwrap())
        .unwrap_or(1e-7);
    let options = r::GraphOptions {
        frame: [[a, -a, 0.], [b, b, -2. * b], [d; 3]],
        degree,
        spans: std::env::var("OSV_JOINT_SPANS")
            .ok()
            .map(|s| s.parse().unwrap())
            .unwrap_or(2),
        align_junctions: true,
        junction_refinement: std::env::var("OSV_JOINT_JUNCTION_REFINEMENT")
            .ok()
            .map(|s| s.parse().unwrap())
            .unwrap_or(0.),
        stations_per_side: std::env::var("OSV_JOINT_STATIONS")
            .ok()
            .map(|s| s.parse().unwrap())
            .unwrap_or(65),
        regularization: 1e-12,
    };
    let hessians = std::env::var("OSV_JOINT_HESSIAN_ANCHORS")
        .ok()
        .map(|path| {
            let data: value_codec::Value =
                value_codec::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
            data["seams"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|v| {
                    Some(r::GraphHessianAnchor {
                        boundary: v["seam"].as_u64()? as usize,
                        parameter_fraction: v["parameterFraction"].as_f64()?,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let attempt = if std::env::var_os("OSV_JOINT_JUNCTION_JETS").is_some() {
        r::propose_central_graph_junction_constrained(
            &collars,
            options,
            &[],
            fairness,
            [&[], &[]],
            &hessians,
        )
    } else {
        r::propose_central_graph_constrained(&collars, options, &[], fairness, [&[], &[]], &[])
    };
    let candidate = match attempt {
        Ok(v) => v,
        Err(e) => {
            save(
                "refused.json",
                value_codec::json!({"phase":"fit","reason":format!("{e}"),"accepted":false}),
            );
            return;
        }
    };
    let boundaries=collars.boundaries.iter().zip(&candidate.pcurves).map(|(c,p)|value_codec::json!({"curve":c.curve,"originalSurface":c.original_surface,"originalPcurve":c.original_pcurve,"candidatePcurve":p})).collect::<Vec<_>>();
    save(
        "central-candidate.json",
        value_codec::json!({"surface":candidate.surface,"boundaries":boundaries,"sampledResidual":candidate.sampled_jet_residual,"accepted":false}),
    );
    if std::env::var_os("OSV_JOINT_FIT_ONLY").is_some() {
        return;
    }
    eprintln!("phase=graft-new-joint-candidate");
    let graft = match r::propose_trihedral_graft(&source, &strips, &collars, &candidate, 1e-8) {
        Ok(v) => v,
        Err(e) => {
            save(
                "refused.json",
                value_codec::json!({"phase":"graft","reason":format!("{e}"),"accepted":false}),
            );
            return;
        }
    };
    save(
        "graft.json",
        value_codec::json!({"model":graft.model,"centralFace":graft.central_face,"accepted":false}),
    );
    let curve = c::inspect(
        &graft.model,
        graft.central_face,
        10.,
        1e-8,
        c::Limits {
            cells: 50000,
            domain_cells: 1000000,
            shape_cells: 1000000,
        },
        &[],
    )
    .unwrap();
    let material = brep_core::face_domain::inspect_box_containment(
        &graft.model,
        graft.central_face,
        [[-1e-7, 10.0000001]; 3],
        1e-8,
        50000,
        1000000,
    )
    .unwrap();
    save(
        "central-guards.json",
        value_codec::json!({"curvature":format!("{:?}",curve.decision),"upper":curve.absolute_curvature_upper,"witness":curve.witness_curvature,"box":format!("{:?}",material.decision),"boxWitness":material.witness_bounds,"accepted":false,"sixCentralSeamsChecked":false}),
    );
}
