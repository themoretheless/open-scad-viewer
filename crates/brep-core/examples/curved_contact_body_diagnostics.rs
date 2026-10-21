//! Reproduce the remaining retained-region membership gate on a curved body.
#[path = "support/curved_contact_body_fixture.rs"]
mod fixture;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let f = fixture::build()?;
    let fitted = if std::env::args().any(|a| a == "--fit") {
        Some(brep_core::contact_graft::fit_boundaries_with_charts(
            &f.source,
            &f.patch,
            &f.mapping,
            &f.keeps,
            &f.source_pcurves,
            1e-8,
            100000,
            1000000,
        )?)
    } else {
        None
    };
    if let Some(p) = &fitted {
        println!(
            "fitted boundaries={} cells={} exact={}",
            p.adjustments.len(),
            p.cells,
            p.exact_work
        );
    }
    if std::env::args().any(|a| a == "--pair") {
        let targets =
            brep_core::contact_graft::plan_targets(&f.source, &f.patch, &f.mapping, &f.keeps)?;
        let edge = &fitted.as_ref().map(|p| &p.model).unwrap_or(&f.patch).edges[0];
        let face = &f.source.faces[targets[0].face];
        let mut pc = if let Some(p) = &fitted {
            p.source_pcurves[0].clone().unwrap()
        } else {
            nurbs_core::curve_pullback::project(
                &edge.curve,
                &face.surface,
                nurbs_core::curve_pullback::Limits {
                    tolerance: f.source.tolerance_mm * 0.5,
                    max_segments: 256,
                    max_iterations: 32,
                    agreement_cells: 100000,
                },
            )?
            .pcurve
            .ok_or("No actual pullback")?
        };
        let outer = &f.source.loops[face.outer].coedges;
        for (k, &v) in edge.vertices.iter().enumerate() {
            let use_ = outer
                .iter()
                .find(|c| f.source.edges[c.edge].vertices[usize::from(c.reversed)] == f.mapping[v])
                .unwrap();
            let i = if k == 0 {
                0
            } else {
                pc.control_points.len() - 1
            };
            pc.control_points[i] = use_.pcurve.control_points[0].clone();
        }
        let boundary = &outer[7].pcurve;
        let source = format!(
            "{{\"points\":{:?},\"weights\":{:?}}}",
            boundary.control_points, boundary.weights
        );
        let query = format!(
            "{{\"points\":{:?},\"weights\":{:?}}}",
            pc.control_points, pc.weights
        );
        println!("{{\"source\":{},\"query\":{}}}", source, query);
        return Ok(());
    }
    println!(
        "end curve degree={} controls={}",
        f.patch.edges[0].curve.degree,
        f.patch.edges[0].curve.control_points.len()
    );
    println!(
        "curved contacts root-bound; end patch qualified; domain={:?}",
        f.law_domain
    );
    let graft = brep_core::contact_graft::attach_with_charts(
        &f.source,
        fitted.as_ref().map(|p| &p.model).unwrap_or(&f.patch),
        &f.mapping,
        &brep_core::contact_graft::plan_targets(&f.source, &f.patch, &f.mapping, &f.keeps)?,
        fitted
            .as_ref()
            .map(|p| p.source_pcurves.as_slice())
            .unwrap_or(&f.source_pcurves),
        1e-8,
        100000 - fitted.as_ref().map(|p| p.cells).unwrap_or(0),
        1000000 - fitted.as_ref().map(|p| p.exact_work).unwrap_or(0),
    )?;
    println!(
        "closed body: faces={} edges={} vertices={} closure={:?}",
        graft.model.faces.len(),
        graft.model.edges.len(),
        graft.model.vertices.len(),
        graft.closure.closure.status
    );
    if std::env::args().any(|a| a == "--ends") {
        let ends = brep_core::sweep_smoothness::geometric::inspect_edges(
            &graft.model,
            &[graft.patch_edges[0], graft.patch_edges[2]],
            brep_core::sweep_smoothness::geometric::Tolerances {
                position: f.source.tolerance_mm,
                normal: 1e-5,
                curvature: 1e-4,
            },
            100000,
        )?;
        // A separate bounded diagnostic pass, not part of solid admission.
        println!("end audit: complete={} smooth={} cells={} exact={}",
            ends.complete, ends.within_tolerance, ends.cells, ends.exact_work);
        for seam in ends.seams {
            println!("end edge={} faces={:?} positions={:?} geometry={:?}",
                seam.edge, seam.faces, seam.positions.as_ref().map(|p| p.proven), seam.inspection);
        }
    }
    if let Some(args) = std::env::args()
        .collect::<Vec<_>>()
        .windows(2)
        .find(|w| w[0] == "--out")
    {
        use sha2::{Digest, Sha256};
        let directory = std::path::PathBuf::from(&args[1]);
        std::fs::create_dir_all(&directory)?;
        let (step, _, _) = brep_core::export_step_v5(&graft.model)?;
        let model = value_codec::to_string(&graft.model)?;
        let saved = fixture::inspect_saved(
            &f,
            &graft,
            graft.cells + fitted.as_ref().map(|p| p.cells).unwrap_or(0),
            graft.exact_work + fitted.as_ref().map(|p| p.exact_work).unwrap_or(0),
        )?;
        let seam_proven = saved.seams.iter().all(|p| {
            p.positions.proven
                && p.geometric.decision == nurbs_core::surface_join::Decision::WithinTolerance
        });
        let radius_proven =
            saved.radius.decision == brep_core::fillet_radius::Decision::WithinTolerance;
        if !seam_proven || !radius_proven {
            return Err("Saved-model seam/radius qualification incomplete".into());
        }
        let digest = |bytes: &[u8]| {
            Sha256::digest(bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        };
        std::fs::write(directory.join("curved-variable-graft.step"), &step)?;
        std::fs::write(directory.join("model.json"), &model)?;
        let adjustments=fitted.as_ref().map(|p|p.adjustments.iter().map(|a|value_codec::json!({"edge":a.edge,"sourceFace":a.source_face,"displacementUpperMm":a.displacement_upper_mm})).collect::<Vec<_>>()).unwrap_or_default();
        let contact = &graft.model.edges[graft.patch_edges[3]].curve;
        let domain = contact.domain();
        let radius_endpoints=(0..2).map(|i|{
            let cp=if i==0 {0}else{contact.control_points.len()-1};
            value_codec::json!({"yMm":contact.control_points[cp][1],"originalParameter":domain[i],
                "expectedBallRadiusMm":1.+0.5*(domain[i]-f.law_domain[0])/(f.law_domain[1]-f.law_domain[0])})
        }).collect::<Vec<_>>();
        let manifest = value_codec::json!({"schema":"curved-variable-graft/1","file":"curved-variable-graft.step",
            "stepSha256":digest(step.as_bytes()),"modelSha256":digest(model.as_bytes()),"modelToleranceMm":graft.model.tolerance_mm,
            "expected":{"boundsMm":[[0.,3.,0.],[10.,7.,10.]],"solids":1,"faces":9,"edges":21,"vertices":14},
            "sourceAnalyticVolumeUpperMm3":4.*(100.-4.5*std::f64::consts::PI),
            "nativeClosureWithinTolerance":graft.closure.closure.status==brep_core::watertight::Closure::WithinTolerance,
            "trimRegionsValid":graft.trims.all_valid,"boundaryAdjustments":adjustments,
            "cells":graft.cells+fitted.as_ref().map(|p|p.cells).unwrap_or(0),
            "exactWork":graft.exact_work+fitted.as_ref().map(|p|p.exact_work).unwrap_or(0),
            "savedChecks":{"cells":saved.cells,"exactWork":saved.exact_work,
                "filletLongitudinalG2Proven":seam_proven,"contactBallLawProven":radius_proven,
                "contactBallCenterGapMm":saved.radius.maximum_center_gap_mm,
                "retainedPrescribedRadiusRangeMm":saved.radius.prescribed_radius_range_mm,
                "seams":saved.seams.iter().map(|p|value_codec::json!({"cells":p.geometric.cells,
                    "branches":p.geometric.branches.iter().map(|b|value_codec::json!({"normalUpper":b.normal_upper,"tensorUpper":b.tensor_upper})).collect::<Vec<_>>() })).collect::<Vec<_>>()},
            "fillet":{"skinFace":graft.patch_face,"contactEdges":[graft.patch_edges[3],graft.patch_edges[1]],
                "originalLawDomain":f.law_domain,"sides":f.sides,"linearRadiusEndpointsMm":[1.,1.5]},
            "radiusEndpoints":radius_endpoints,"endSeamContinuity":"C0","smoothEndTransitionClaim":false,
            "wholeBodyG2Claim":false,"wholeWallClaim":false,"globalSelfIntersectionClaim":false});
        std::fs::write(
            directory.join("manifest.json"),
            value_codec::to_string_pretty(&manifest)?,
        )?;
    }
    Ok(())
}
