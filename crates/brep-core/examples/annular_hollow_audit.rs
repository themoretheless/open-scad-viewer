//! Native diagnostic for the unchanged spatial hollow source and retained caps.
use brep_core::{
    boundary_embedding, face_contacts, retained_annular_cap, sweep_cap_contacts, volume_validity,
};
fn main() -> nurbs_core::Result<()> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let correct_circle = arguments.iter().any(|a| a == "--circle");
    let outer = nurbs_core::primitives::circle([0.; 3], [0., 0., 1.], 0.5)?;
    let hole = nurbs_core::primitives::circle([0.; 3], [0., 0., -1.], 0.2)?;
    let loops = vec![vec![outer.clone()], vec![hole.clone()]];
    let sections = nurbs_core::paths::miter_sections(
        &[outer, hole],
        &[[0., 0., 0.], [0., 0., 10.], [10., 0., 10.], [10., 10., 15.]],
        [1., 0., 0.],
        2.,
    )?;
    let sections = sections
        .into_iter()
        .map(|s| s.into_iter().map(|c| vec![c]).collect())
        .collect::<Vec<_>>();
    let source = brep_core::rational_section_loft(&sections)?;
    let cap = retained_annular_cap::replace(&source, 25, 1_000_000)?;
    // Planar cap contacts optimize only the unchanged first cap. Every new
    // annular patch is included in generic face and pair classification.
    let limits = volume_validity::Limits {
        boundary: boundary_embedding::Limits {
            exact_work: 1_000_000,
            trim_pairs: 10_000,
            trim_cells: 100_000,
            trim_domain_cells: 1_000_000,
            spans: 1024,
            contacts: face_contacts::Limits {
                pairs: 10_000,
                cells: 100_000,
                domain_cells: 1_000_000,
                cells_per_pair: 1000,
                domain_cells_per_pair: 10_000,
            },
        },
        nesting_pairs: 1000,
        nesting_cells: 100_000,
        nesting_domain_cells: 1_000_000,
        orientation_cells: 100_000,
        orientation_domain_cells: 1_000_000,
        orientation_spans: 100,
    };
    let cap_limits = sweep_cap_contacts::Budgets {
        max_walls: 1024,
        max_exact_work: 1_000_000,
        max_chart_cells: 1000,
        max_trim_pairs: 100_000,
        max_trim_cells: 100_000,
        max_trim_domain_cells: 1_000_000,
    };
    let filling = retained_annular_cap::reference_filling::inspect(
        &source,
        &cap,
        1e-10,
        1e-8,
        retained_annular_cap::reference_filling::Limits {
            exact_work: 1_000_000,
            products: 100_000,
            trim_pairs: 10_000,
            trim_cells: 100_000,
            trim_domain_cells: 1_000_000,
            spans: 1024,
            linear_cells: 100_000,
            contacts: limits.boundary.contacts,
        },
    )?;
    println!(
        "reference_filling_covered={} reference_error_upper={:?} boundary_error_upper={:?} bilinear_defect_upper={:?} exact_work={} products={} authored_source_bound_proven=false",
        filling.uv_region_covered,
        filling.error_upper,
        filling.boundary_error_upper,
        filling.bilinear_defect_upper,
        filling.exact_work,
        filling.products
    );
    let result =
        volume_validity::inspect_sweep(&cap.model, 1e-8, limits, 100_000, &[24], cap_limits)?;
    let pairs = &result.boundary.intersections.pairs;
    println!(
        "faces={} exact_uses={} exact_joins={} injective={} pairs_classified={} total_pairs={} visited_pairs={} grouped_pairs={} next_pair={:?} cells={} domain_cells={} orientation_attempts={} volume_proven={}",
        cap.model.faces.len(),
        result.boundary.agreement.all_equal,
        result.boundary.agreement.all_joins_exact,
        result.boundary.intersections.faces.all_faces_injective,
        pairs.all_pairs_classified,
        pairs.total_pairs,
        pairs.pairs.len(),
        pairs.grouped_pairs,
        pairs.next_pair,
        pairs.cells,
        pairs.domain_cells,
        result
            .orientations
            .iter()
            .map(|s| s.attempts.len())
            .sum::<usize>(),
        result.proven
    );
    for p in &pairs.pairs {
        if p.reason == "pair-unresolved" {
            println!("unresolved={:?}", p.faces);
            let a = &cap.model.faces[p.faces[0]];
            let b = &cap.model.faces[p.faces[1]];
            let common = cap.model.loops[a.outer].coedges.iter().find_map(|ca| {
                cap.model.loops[b.outer]
                    .coedges
                    .iter()
                    .find(|cb| cb.edge == ca.edge)
                    .map(|cb| (ca, cb))
            });
            if let Some((ca, cb)) = common {
                let aa = nurbs_core::surface_edit::transpose(&a.surface)?;
                let bb = nurbs_core::surface_edit::transpose(&b.surface)?;
                let ends = [
                    ca.pcurve.control_points[0][1] == 1.,
                    cb.pcurve.control_points[0][1] == 1.,
                ];
                let proof = nurbs_core::surface_injectivity::certify_shared_natural_boundary(
                    &aa, &bb, ends, 1000,
                )?;
                let reverse = nurbs_core::surface_edit::reverse(&bb, nurbs_core::surface::Axis::V)?;
                let reversed = nurbs_core::surface_injectivity::certify_shared_natural_boundary(
                    &aa, &reverse, ends, 1000,
                )?;
                println!(
                    "edge={} direct={} cells={} q={:?}; reversed={} cells={} q={:?}",
                    ca.edge,
                    proof.reason,
                    proof.spans,
                    proof.contraction_upper,
                    reversed.reason,
                    reversed.spans,
                    reversed.contraction_upper
                );
            }
        }
    }
    let points = [[0., 0., 0.], [0., 0., 10.], [10., 0., 10.], [10., 10., 15.]];
    let owned = brep_core::sweep_miter_owned::annular::construct_direct(
        &brep_core::sweep_miter_owned::DirectRequest {
            loops: &loops,
            points: &points,
            normal: [1., 0., 0.],
            closed: false,
            miter_limit: 2.,
            max_deviation: 0.01,
            circle: correct_circle.then_some(nurbs_core::miter_section_correction::Budget {
                quantum: 2_f64.powi(-40),
                tolerance: 1e-9,
                max_work: Some(1000000.),
            }),
            caps: None,
            cap_endpoints: None,
        },
        &brep_core::sweep_miter_owned::Limits {
            max_products: 100_000,
            max_faces: 1024,
            correspondence_faces: 1024,
            cap_max_edges: 1024,
            wall_cells: 10_000,
            exact_work: 1_000_000,
            domain_exact_work: 1_000_000,
            projection_exact_work: 1_000_000,
            domain_tolerance: 0.001,
            domain_pairs: 1000,
            domain_cells: 10_000,
            projection_cells: 10_000,
            cap_regions: cap_limits,
        },
    )?;
    let volume =
        volume_validity::inspect_sweep(owned.model(), 1e-8, limits, 100_000, &[], cap_limits)?;
    let smooth = owned.inspect_smoothness(2_000_000)?;
    println!(
        "owned_source_complete={} owned_source_error={:?} filled_caps={:?} cap_face_groups={:?} faces={} all_pairs={} volume_proven={} retained_profile_g1={} retained_profile_g2={}",
        owned.complete_boundary().is_some(),
        owned.boundary().error_upper,
        owned.boundary().filled_cap_error_upper,
        owned.cap_face_groups(),
        owned.model().faces.len(),
        volume.boundary.intersections.pairs.total_pairs,
        volume.proven,
        smooth.walls.profile.g1_certified,
        smooth.walls.profile.g2_certified
    );
    println!(
        "circle_correction={} internal_cap_g1={} internal_cap_g2={} smoothness_work={} projective_patches={}",
        correct_circle,
        smooth.cap_seams.iter().all(|r| r.g1_certified),
        smooth.cap_seams.iter().all(|r| r.g2_certified),
        smooth.exact_work,
        owned
            .filled_caps()
            .map_or(0, |r| r.projective_reparameterized_patches)
    );
    if let Some(path) = arguments.iter().find(|a| !a.starts_with("--")) {
        let (step, certificate, _) = brep_core::export_step_v5(owned.model())?;
        std::fs::write(&path, step).map_err(|_| {
            nurbs_core::Error::new("ANNULAR_AUDIT_IO", "Cannot save native STEP audit artifact")
        })?;
        println!(
            "step_exported=true capability={} complete={}",
            certificate.capability, certificate.complete
        );
    }
    Ok(())
}
