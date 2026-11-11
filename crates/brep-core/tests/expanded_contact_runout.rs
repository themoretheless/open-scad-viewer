#[path = "../examples/support/compatible_contact_fixture.rs"]
mod fixture;
#[path = "../examples/support/natural_patch_sheet.rs"]
mod natural_sheet;
#[path = "../examples/support/expanded_contact_runout.rs"]
mod runout;
use brep_core::{face_cut, fillet_radius, region_replacement as r};
use nurbs_core::offset::fillet_surfaces::RadiusLaw;
#[test]
fn expanded_runout_keeps_central_radius_and_removes_endpoint_obstruction() {
    let (source, request, _, _) = fixture::fixture();
    let before = source.clone();
    let proposal = runout::build();
    assert!(proposal.skin.control_points.len() <= 256);
    assert!(proposal.storage_error <= 2.5e-8);
    assert!(proposal.skin.weights.iter().flatten().all(|&w| w == 1.));
    let plan = r::plan_radius_network(&source, 0, &[request], 10000).unwrap();
    let supports = plan.edges[0].supports.map(|f| &source.faces[f].surface);
    let contacts = runout::contacts(&proposal.skin, supports);
    assert!(contacts.iter().all(|c| c.positions.proven));
    let center = contacts.clone().map(|mut c| {
        c.curve = c.curve.trim(0.5, 1.5).unwrap();
        c.support_pcurve = c.support_pcurve.trim(0.5, 1.5).unwrap();
        c.fillet_pcurve = c.fillet_pcurve.trim(0.5, 1.5).unwrap();
        c
    });
    let radius = fillet_radius::inspect_contacts_in_domain(
        supports,
        &proposal.skin,
        [&center[0], &center[1]],
        [0.5, 1.5],
        fillet_radius::RadiusCheck {
            law: &RadiusLaw::Constant(1.),
            sides: [1., -1.],
            tolerance_mm: 1e-7,
            max_cells: 100000,
            max_exact: 1000000,
        },
    )
    .unwrap();
    assert_eq!(
        radius.decision,
        fillet_radius::Decision::WithinTolerance,
        "{}",
        radius.reason
    );
    assert!(radius.maximum_center_gap_mm.unwrap()[1] <= 1e-7);
    let retention = face_cut::plan_contact_retention(
        &source,
        0,
        request,
        &proposal.skin,
        [&contacts[0], &contacts[1]],
        "qualified-expanded-runout",
        1e-8,
        Default::default(),
        |roots| Ok(fixture::paths(&source, request, &contacts, roots)),
    )
    .unwrap();
    assert_eq!(retention.roots().root_parameters(), [[0., 0.], [2., 2.]]);
    assert_eq!(retention.plan().regions.len(), 2);
    assert!(
        !r::inspect_contact_end_normals(&retention)
            .unwrap()
            .needs_expansion()
    );
    retention.reverify().unwrap();
    let mut field_skin = proposal.skin.clone();
    for k in &mut field_skin.knots_u {
        *k *= 0.5;
    }
    let sheet = natural_sheet::unit_sheet(&field_skin);
    let field = brep_core::face_curvature::inspect(
        &sheet,
        0,
        10.,
        1e-8,
        brep_core::face_curvature::Limits {
            cells: 10000,
            domain_cells: 1000000,
            shape_cells: 100000,
        },
        &[],
    )
    .unwrap();
    assert_eq!(
        field.decision,
        brep_core::face_curvature::Decision::Within,
        "{}",
        field.reason
    );
    assert!(field.absolute_curvature_upper.is_some_and(|k| k <= 10.));
    assert_eq!(field.omitted_cells, 0);
    eprintln!(
        "expanded_runout curvature={:?} cells={} shape={}",
        field.absolute_curvature_upper, field.cells, field.shape_cells
    );
    assert_eq!(source, before);
    eprintln!(
        "expanded_runout poles={} central_radius={:?} roots={:?}",
        proposal.skin.control_points.len() * proposal.skin.control_points[0].len(),
        radius.maximum_center_gap_mm,
        retention.roots().root_parameters()
    );
}
