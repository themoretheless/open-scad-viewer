#[path = "../../brep-core/examples/support/compatible_contact_fixture.rs"]
mod fixture;
use brep_core::{face_cut, region_replacement as r};
fn main() {
    let (source, request, skin, contacts) = fixture::fixture();
    if let Ok(path) = std::env::var("CONTACT_SOURCE_STEP") {
        let (step, _, _) = cad_step::export_step_v9(&source).unwrap();
        std::fs::write(path, step).unwrap();
    }
    let retention = face_cut::plan_contact_retention(
        brep_core::edge_partition::contact_interval::EdgeContacts {
            source: &source,
            shell: 0,
            request,
            skin: &skin,
            contacts: [&contacts[0], &contacts[1]],
        },
        "compatible-nurbs-contact",
        1e-8,
        Default::default(),
        |roots| Ok(fixture::paths(&source, request, &contacts, roots)),
    )
    .unwrap();
    let normals = r::inspect_contact_end_normals(&retention).unwrap();
    println!("end_normals={normals:?}");
    assert!(!normals.needs_expansion());
    let authored = fixture::patch(&retention, &skin);
    let plan = retention.plan();
    let selected = plan
        .regions
        .iter()
        .flat_map(|r| r.discard_faces.iter().copied())
        .collect::<Vec<_>>();
    let patches = authored
        .patches
        .iter()
        .map(|p| r::SewPatch {
            surface: &p.surface,
            loops: &p.loops,
            reversed: p.reversed,
        })
        .collect::<Vec<_>>();
    let local = r::sew_g2_patch_set(
        &plan.model,
        r::PatchReplacement {
            selected: &selected,
            new_vertices: &authored.vertices,
            new_edges: &authored.edges,
            patches: &patches,
        },
        "compatible-nurbs-local",
        1e-8,
        Default::default(),
    )
    .unwrap();
    println!(
        "local_g2=true faces={} seams={} max_curvature={:?}",
        local.model().faces.len(),
        local.seams().seams.len(),
        local
            .fields()
            .iter()
            .filter_map(|f| f.absolute_curvature_upper)
            .reduce(f64::max)
    );
    if let Ok(path) = std::env::var("CONTACT_CANDIDATE_STEP") {
        let (step, _, _) = cad_step::export_step_v9(local.model()).unwrap();
        std::fs::write(path, step).unwrap();
    }
    let limits = r::EmbeddedPatchLimits {
        construction: Default::default(),
        linear_cells: 100000,
        global: brep_core::boundary_embedding::Limits {
            exact_work: 1000000,
            trim_pairs: 10000,
            trim_cells: 100000,
            trim_domain_cells: 1000000,
            spans: 10000,
            contacts: brep_core::face_contacts::Limits {
                pairs: 10000,
                cells: 1000000,
                domain_cells: 1000000,
                cells_per_pair: 1000,
                domain_cells_per_pair: 10000,
            },
        },
    };
    match r::sew_contact_g2_patch_set(retention, "compatible-nurbs-body", 1e-8, limits, |p| {
        Ok(fixture::patch(p, &skin))
    }) {
        Ok(body) => {
            assert_eq!(body.model().0, local.model().0);
            if let Ok(path) = std::env::var("CONTACT_BODY_STEP") {
                let (step, _, _) = cad_step::export_step_v9(body.model()).unwrap();
                std::fs::write(path, step).unwrap();
            }
            let networks = body
                .global()
                .intersections
                .pairs
                .pairs
                .iter()
                .filter(|p| {
                    matches!(
                        p.boundary,
                        Some(brep_core::face_contacts::SharedBoundary::ProjectedTrimNetwork(_))
                    )
                })
                .count();
            println!(
                "admitted=true faces={} global={} pairs={} network_certificates={}",
                body.model().faces.len(),
                body.global().proven,
                body.global().intersections.pairs.total_pairs,
                networks
            );
        }
        Err(r::ContactPatchFailure::Unproven(report)) => {
            println!(
                "admitted=false exact={} trims={} injective={} classified={} pending={:?}",
                report.agreement.all_equal,
                report.trim.all_valid,
                report.intersections.faces.all_faces_injective,
                report.intersections.pairs.all_pairs_classified,
                report.intersections.pairs.next_pair
            );
            for p in &report.intersections.pairs.pairs {
                println!(
                    "pair={:?} reason={} hull={} boundary={}",
                    p.faces,
                    p.reason,
                    p.hull_disjoint,
                    p.boundary.is_some()
                );
            }
            std::process::exit(2);
        }
        Err(e) => panic!("{e:?}"),
    }
}
