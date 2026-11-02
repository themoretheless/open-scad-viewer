//! Diagnostic on actual trim/sew controls, never a substitute for admission.
#[path = "support/retained_patch_fixture.rs"]
mod fixture;
use brep_core::{boundary_embedding, face_cut, region_replacement as r};
fn main() -> std::process::ExitCode {
    let mut all_proven = true;
    for n in [3, 5] {
        let (mut m, face, cut, seeds) = fixture::source();
        if n == 5 {
            let e = m.loops[m.faces[face].outer].coedges[2].edge;
            let d = m.edges[e].curve.domain();
            m = brep_core::feature_ops::edge_partition::split(
                &m,
                e,
                &[d[0] + (d[1] - d[0]) * 0.25, d[0] + (d[1] - d[0]) * 0.75],
                "embedding-five-source-rim",
                Default::default(),
            )
            .unwrap()
            .model()
            .clone();
        }
        let base = brep_core::boundary_agreement::verify_exact(&m, 1_000_000).unwrap();
        eprintln!(
            "base patches={n} positions={} exact_work={} unproven_lifts={:?}",
            base.all_equal,
            base.work,
            base.uses
                .iter()
                .filter(|u| !u
                    .decision
                    .as_ref()
                    .is_some_and(|d| d.outcome == cad_predicates::BezierIdentity::Equal))
                .map(|u| (
                    u.face,
                    u.edge,
                    u.decision
                        .as_ref()
                        .map(|d| (d.outcome.clone(), d.work_used))
                ))
                .collect::<Vec<_>>()
        );
        let cuts = [cut];
        let requests = [face_cut::FaceRetention {
            face,
            cuts: &cuts,
            keep_uv: &seeds,
        }];
        let admitted = r::trim_and_sew_embedded_g2_patch_set(
            &m,
            &requests,
            "embedding-control",
            1e-8,
            r::EmbeddedPatchLimits {
                construction: Default::default(),
                linear_cells: 100_000,
                global: boundary_embedding::Limits {
                    exact_work: 1_000_000,
                    trim_pairs: 10_000,
                    trim_cells: 100_000,
                    trim_domain_cells: 1_000_000,
                    spans: 10_000,
                    contacts: brep_core::face_contacts::Limits {
                        pairs: 10_000,
                        cells: 1_000_000,
                        domain_cells: 1_000_000,
                        cells_per_pair: 1000,
                        domain_cells_per_pair: 10_000,
                    },
                },
            },
            fixture::fan,
        );
        let report = match &admitted {
            Ok(body) => body.global(),
            Err(r::EmbeddedPatchFailure::Unproven(d)) => d.report(),
            Err(e) => panic!("Body construction/audit failed: {e}"),
        };
        eprintln!("Atomic admission patches={n} admitted={}", admitted.is_ok());
        eprintln!(
            "exact_work={} unproven_lifts={:?}",
            report.agreement.work,
            report
                .agreement
                .uses
                .iter()
                .filter(|u| !u
                    .decision
                    .as_ref()
                    .is_some_and(|d| d.outcome == cad_predicates::BezierIdentity::Equal))
                .map(|u| (
                    u.face,
                    u.edge,
                    u.decision
                        .as_ref()
                        .map(|d| (d.outcome.clone(), d.work_used))
                ))
                .collect::<Vec<_>>()
        );
        all_proven &= report.proven;
        let unproven_faces = report
            .intersections
            .faces
            .faces
            .iter()
            .filter(|f| !f.absence_proven())
            .map(|f| f.face)
            .collect::<Vec<_>>();
        let unproven_pairs = report
            .intersections
            .pairs
            .pairs
            .iter()
            .filter(|p| {
                p.boundary.is_none()
                    && !p.hull_disjoint
                    && p.volume_separation.is_none()
                    && !p.result.as_ref().is_some_and(|r| r.absence_proven)
            })
            .map(|p| (p.faces, p.reason))
            .collect::<Vec<_>>();
        println!(
            "patches={n} proven={} positions={} joins={} trims={} unproven_faces={:?} total_pairs={} visited={} grouped={} next={:?} unproven_pairs={:?}",
            report.proven,
            report.agreement.all_equal,
            report.agreement.all_joins_exact,
            report.trim.all_valid,
            unproven_faces,
            report.intersections.pairs.total_pairs,
            report.intersections.pairs.pairs.len(),
            report.intersections.pairs.grouped_pairs,
            report.intersections.pairs.next_pair,
            unproven_pairs
        );
    }
    if all_proven {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::from(2)
    }
}
