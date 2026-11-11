//! Contact-law diagnostics only. Exit 0 requires one proven contact-center law;
//! G2, skin radius, curvature, end closure and solid admission remain separate.
#[path = "support/compatible_contact_fixture.rs"]
mod fixture;
use brep_core::{fillet_contact, fillet_radius, region_replacement};
use nurbs_core::offset::fillet_surfaces::{FilletOptions, RadiusLaw};
fn main() {
    let (source, request, _, _) = fixture::fixture();
    let plan = region_replacement::plan_radius_network(&source, 0, &[request], 10000).unwrap();
    let supports = plan.edges[0].supports.map(|f| &source.faces[f].surface);
    let sections = std::env::var("CONTACT_SECTIONS").ok()
        .map(|s| s.parse::<usize>().expect("CONTACT_SECTIONS must be an integer"))
        .unwrap_or(21);
    let mut radius_qualified = false;
    for sides in [[1., -1.], [-1., 1.]] {
        let options = FilletOptions { side_a: sides[0], side_b: sides[1], max_sections:sections,
            tolerance:1e-9, ..Default::default() };
        let built = fillet_contact::build_g2_on_domains(supports, [[0.5, 1.5, 0., 1.]; 2],
            request.radius_mm, &options, 1e-7, 100000, 1000000).unwrap();
        println!("sides={sides:?} status={:?} failure={:?} spine={} cells={} exact={} warnings={:?}",
            built.status, built.fillet.failure, built.fillet.spine.len(), built.cells,
            built.exact_work, built.fillet.warnings);
        println!("contact_span={:?}", built.fillet.spine.first().zip(built.fillet.spine.last())
            .map(|(a,b)| [a.contact_a,b.contact_a]));
        for (side, contact) in built.contact_attempts.iter().enumerate() {
            if let Some(c) = contact { println!("attempt={side} upper={:?} intervals={:?}",
                c.positions.upper, c.positions.sides.iter().map(|p| p.interval.as_ref()).collect::<Vec<_>>()); }
        }
        if let (Some(skin), Some(contacts)) = (&built.fillet.surface, &built.contacts) {
            let proof = fillet_radius::inspect(supports, skin, [&contacts[0], &contacts[1]],
                &RadiusLaw::Constant(request.radius_mm), sides, 1e-7, 100000, 1000000).unwrap();
            radius_qualified |= proof.decision == fillet_radius::Decision::WithinTolerance;
            println!("radius={:?} reason={} bound={:?}", proof.decision, proof.reason, proof.maximum_center_gap_mm);
            for side in 0..2 {
                let c = &contacts[side];
                let seam = nurbs_core::surface_g2_pcurves::inspect(
                    nurbs_core::surface_g2_pcurves::PcurveSeam { curve: &c.curve, pcurves: [&c.support_pcurve, &c.fillet_pcurve], surfaces: [supports[side], skin], reversed: [false;2] },
                    nurbs_core::surface_g2_pcurves::G2PcurveTolerance { position: 1e-7, normal: 1e-5, curvature: 1e-4 },
                    100000,
                    1000000,
                ).unwrap();
                println!("g2_side={side} decision={:?} cells={} positions={}",
                    seam.geometric.decision, seam.geometric.cells, seam.positions.proven);
            }
        }
    }
    if !radius_qualified { std::process::exit(2); }
}
