//! Fresh native source-owned smooth bodies, material audits and STEP fixtures.
//! Debug timings are not WASM/Rush/UI timeout qualification.
use brep_core::{
    analytic::progressive_profile_body_with_annular_caps, boundary_embedding, face_contacts,
    retained_annular_cap::reference_filling, sweep_cap_contacts, volume_validity,
};
use nurbs_core::{
    curve::Curve,
    primitives::{circle, line},
    progressive_sweep::{Options, Orientation, Spacing, constant_vector_law},
};
use sha2::{Digest, Sha256};
use std::{path::PathBuf, time::Instant};
use value_codec::json;

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn cap_limits() -> reference_filling::Limits {
    reference_filling::Limits {
        exact_work: 1_000_000,
        products: 100_000,
        trim_pairs: 10_000,
        trim_cells: 100_000,
        trim_domain_cells: 1_000_000,
        spans: 1024,
        linear_cells: 100_000,
        contacts: face_contacts::Limits {
            pairs: 10_000,
            cells: 100_000,
            domain_cells: 1_000_000,
            cells_per_pair: 1000,
            domain_cells_per_pair: 10_000,
        },
    }
}
fn volume_limits() -> volume_validity::Limits {
    volume_validity::Limits {
        boundary: boundary_embedding::Limits {
            exact_work: 1_000_000,
            trim_pairs: 10_000,
            trim_cells: 100_000,
            trim_domain_cells: 1_000_000,
            spans: 1024,
            contacts: cap_limits().contacts,
        },
        nesting_pairs: 10_000,
        nesting_cells: 100_000,
        nesting_domain_cells: 1_000_000,
        orientation_cells: 100_000,
        orientation_domain_cells: 1_000_000,
        orientation_spans: 1024,
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = PathBuf::from(std::env::args().nth(1).expect("Output directory required"));
    let automatic=std::env::args().any(|a|a=="--automatic");
    std::fs::create_dir_all(directory.join("step"))?;
    let binary = std::env::current_exe()?;
    let provenance = json!({"mode":"native-source-owned-progressive-annular-debug",
        "nativeExecutableSha256":hash(&std::fs::read(binary)?),"automaticCaps":automatic,"wasmQualified":false});
    let mut cases = Vec::new();
    let mut summaries = Vec::new();
    for (kind, name) in [
        "oblique-rmf-linear-scale",
        "quadratic-frenet",
        "quadratic-planar-rmf",
    ]
    .iter()
    .enumerate()
    {
        eprintln!("Native owned smooth {}/3: {name}", kind + 1);
        let axis = if kind == 0 {
            [3., 4., 0.]
        } else {
            [1., 0., 0.]
        };
        let loops = vec![
            vec![circle([0.; 3], axis, 0.1)?],
            vec![circle([0.; 3], axis, 0.05)?.reverse()?],
        ];
        let path = if kind == 0 {
            line([0.; 3], axis)?
        } else {
            Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: vec![vec![0.; 3], vec![0.5, 0., 0.], vec![1., 1., 0.]],
                weights: vec![1.; 3],
                periodic: false,
            }
        };
        let scale = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![1., 0., 0.],
                vec![if kind == 0 { 2. } else { 1. }, 0., 0.],
            ],
            weights: vec![1.; 2],
            periodic: false,
        };
        let twist = constant_vector_law([0.; 3])?;
        let options = Options {
            normal: [0., 0., 1.],
            orientation: if kind == 1 {
                Orientation::Frenet
            } else {
                Orientation::RotationMinimizing
            },
            spacing: Spacing::Parameter,
            initial_sections: if kind == 0 { 2 } else { 3 },
            max_sections: 129,
            max_deviation: 0.01,
        };
        // Exact original Rust coefficients for later ABI replay. No retained
        // geometry or scalar certificate is supplied to that constructor.
        let request=json!({"op":if automatic {"brep_nurbs_progressive_profile_auto_body"}else{"brep_nurbs_progressive_profile_annular_body"},
            "loops":loops,"path":path,"scale":scale,"twist":twist,"normal":options.normal,
            "orientation":if kind==1 {"frenet"}else{"rmf"},"spacing":"parameter",
            "initial_sections":options.initial_sections,"max_sections":options.max_sections,
            "max_deviation":options.max_deviation});
        let request_text=value_codec::to_string(&request)?;
        std::fs::write(directory.join(format!("{name}-source.json")),&request_text)?;
        let started = Instant::now();
        let construct=if automatic {brep_core::analytic::progressive_profile_body_with_automatic_caps}else{progressive_profile_body_with_annular_caps};
        let body = construct(
            &loops,
            &path,
            &scale,
            &twist,
            None,
            None,
            None,
            options,
            None,
            None,
            None,
            cap_limits(),
        )?;
        let construct_ms = started.elapsed().as_secs_f64() * 1000.;
        let bound = body
            .complete_boundary()
            .expect("Complete source-owned boundary required");
        let caps = body.caps().unwrap();
        let model = body.model();
        let (step, certificate, _) = brep_core::export_step_v5(model)?;
        assert!(certificate.complete);
        let file = format!("step/{name}.step");
        std::fs::write(directory.join(&file), &step)?;
        eprintln!(
            "  construct {construct_ms:.3}ms, {} faces; fresh full material audit",
            model.faces.len()
        );
        let started = Instant::now();
        // No selected planar-cap shortcut: all actual patches and every pair
        // participate in the fresh native global boundary/material proof.
        let material = volume_validity::inspect_sweep(
            model,
            1e-8,
            volume_limits(),
            100_000,
            &[],
            sweep_cap_contacts::Budgets {
                max_walls: 1024,
                max_exact_work: 1_000_000,
                max_chart_cells: 1000,
                max_trim_pairs: 100_000,
                max_trim_cells: 100_000,
                max_trim_domain_cells: 1_000_000,
            },
        )?;
        let material_ms = started.elapsed().as_secs_f64() * 1000.;
        let pairs = &material.boundary.intersections.pairs;
        let orientations = material
            .orientations
            .iter()
            .map(|o| {
                json!({"shell":o.shell,
            "expectedOutward":o.expected_outward,"outward":o.outward,"attempts":o.attempts.len()})
            })
            .collect::<Vec<_>>();
        let native = json!({"solidGeometryCertified":material.proven,"boundaryEmbeddingCertified":material.boundary.proven,
            "allFacesInjective":material.boundary.intersections.faces.all_faces_injective,
            "allPairsClassified":pairs.all_pairs_classified,"totalPairs":pairs.total_pairs,
            "groupedPairs":pairs.grouped_pairs,"individualPairs":pairs.pairs.len(),"nextPair":pairs.next_pair,
            "nesting":material.nesting.as_ref().map(|n|json!({"parents":n.parents,"rolesConsistent":n.roles_consistent})),
            "orientations":orientations});
        let construction = json!({"method":"native-source-owned-progressive-annular",
            "boundaryCertificate":{"continuousBound":bound.continuous_bound,"withinBudget":bound.within_budget,
                "errorUpper":bound.error_upper,"wallErrorUpper":bound.wall_error_upper,"filledCapErrorUpper":bound.filled_cap_error_upper,
                "budget":options.max_deviation,"closed":body.evidence().approximation.levels.last().unwrap().closed_path,"reason":bound.reason,"scope":"boundary-set-hausdorff"},
            "capFaceGroups":caps.face_groups(),"capSelectionWork":body.cap_selection_work(),"capConstructionWork":caps.construction_work(),"capExactWork":caps.exact_work(),
            "sourceFillingCovered":caps.filling().source_filling_covered,
            "retainedWallCoverage":body.evidence().retained_walls.certified,
            "stations":body.evidence().approximation.levels.last().unwrap().sections});
        let summary = json!({"name":name,"constructDebugMs":construct_ms,"materialDebugMs":material_ms,
            "faces":model.faces.len(),"construction":construction,"nativeVolume":native});
        std::fs::write(
            directory.join(format!("{name}-native.json")),
            value_codec::to_string(&summary)?,
        )?;
        std::fs::write(
            directory.join(format!("{name}-model.json")),
            value_codec::to_string(model)?,
        )?;
        let mut samples = Vec::new();
        let mut coedges = Vec::new();
        let mut face_loops = Vec::new();
        let mut reverse = Vec::new();
        for (index, face) in model.faces.iter().enumerate() {
            let owner = model
                .shells
                .iter()
                .flat_map(|s| s.faces.iter())
                .filter(|f| f.face == index)
                .collect::<Vec<_>>();
            assert_eq!(owner.len(), 1);
            reverse.push(owner[0].reversed);
            assert!(face.holes.is_empty());
            let mut face_samples = Vec::new();
            let (du, dv) = face.surface.evaluate(0.5, 0.5)?.domains();
            for a in [0., 0.17, 0.5, 0.83, 1.] {
                for b in [0., 0.13, 0.5, 0.87, 1.] {
                    let u = du[0] + a * (du[1] - du[0]);
                    let v = dv[0] + b * (dv[1] - dv[0]);
                    let sample = face.surface.evaluate(u, v)?;
                    let (first_u, first_v) = sample.first_derivatives().unwrap();
                    face_samples
                        .push(json!({"u":u,"v":v,"point":sample.point,"du":first_u,"dv":first_v}));
                }
            }
            samples.push(face_samples);
            coedges.push(
                model.loops[face.outer]
                    .coedges
                    .iter()
                    .map(|c| {
                        json!({"edge":c.edge,
                "reversed":c.reversed,"pcurve":c.pcurve})
                    })
                    .collect::<Vec<_>>(),
            );
            face_loops.push(json!({"outer":model.loops[face.outer].coedges.iter().map(|c|c.edge).collect::<Vec<_>>(),"holes":[]}));
        }
        let mut edges = Vec::new();
        for edge in &model.edges {
            let [a, b] = edge.curve.domain();
            let mut points = Vec::new();
            for t in [0., 0.25, 0.5, 0.75, 1.] {
                let u = a + t * (b - a);
                points.push(json!({"u":u,"point":edge.curve.evaluate(u)?.point}));
            }
            edges.push(json!({"curve":edge.curve,"samples":points}));
        }
        cases.push(json!({"file":file,"sha256":hash(step.as_bytes()),"faces":model.faces.len(),"edges":model.edges.len(),
            "solids":1,"shells":model.shells.len(),"capHoleFaces":0,"capFaces":[],"expectedVolume":0.,
            "volumeReferenceMethod":"rational-boundary-gauss-reference","outwardCapContours":[],
            "wallSurfaces":model.faces.iter().map(|f|f.surface.clone()).collect::<Vec<_>>(),"wallFaceReversed":reverse,
            "wallSamples":samples,"wallCoedges":coedges,"faceLoops":face_loops,"edgeCurves":edges,
            "surfaceToleranceMm":1e-8,"relativeVolumeTolerance":1e-7,"requireNativeSolid":true,
            "nativeVolume":native,"construction":construction,
            "sourceRequestFile":format!("{name}-source.json"),"sourceRequestSha256":hash(request_text.as_bytes())}));
        summaries.push(summary);
        eprintln!(
            "  material {material_ms:.3}ms, proven={}, pairs={}/{}",
            material.proven,
            pairs.grouped_pairs + pairs.pairs.len(),
            pairs.total_pairs
        );
        std::fs::write(
            directory.join("manifest.json"),
            value_codec::to_string(&json!({"schema":"sweep-external-step/2",
            "units":"mm","artifactProvenance":provenance,"cases":cases}))?,
        )?;
    }
    let passed = summaries
        .iter()
        .all(|s| s["nativeVolume"]["solidGeometryCertified"] == true);
    std::fs::write(
        directory.join("native-summary.json"),
        value_codec::to_string(&json!({"passed":passed,
        "artifactProvenance":provenance,"scope":"Fresh native body/material/STEP; not production WASM or UI delivery","cases":summaries}))?,
    )?;
    if !passed {
        return Err("Fresh native material proof refused; all receipts preserved".into());
    }
    Ok(())
}
