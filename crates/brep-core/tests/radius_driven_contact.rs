#[path = "../examples/support/compatible_contact_fixture.rs"]
mod fixture;
#[path = "../examples/support/natural_patch_sheet.rs"]
mod natural_sheet;
use brep_core::{fillet_contact, fillet_radius, region_replacement};
use nurbs_core::offset::fillet_surfaces::{FilletOptions, RadiusLaw};
#[test]
fn automatic_r1_contacts_on_original_curved_nurbs_stay_on_selected_branch() {
    let (source, request, _, _) = fixture::fixture();
    let before = source.clone();
    let plan = region_replacement::plan_radius_network(&source, 0, &[request], 10000).unwrap();
    let supports = plan.edges[0].supports.map(|f| &source.faces[f].surface);
    let options = FilletOptions { side_a:1., side_b:-1., max_sections:21,
        tolerance:1e-9, ..Default::default() };
    let built = fillet_contact::build_g2_on_domains(supports, [[0.5,1.5,0.,1.];2],
        request.radius_mm, &options, 1e-7, 100000, 1000000).unwrap();
    assert_eq!(built.status, fillet_contact::Status::VerifiedContacts);
    assert_eq!(built.fillet.spine.len(),21);
    assert!(built.cells<=100000 && built.exact_work<=1000000);
    assert!(built.skin_edit_error_upper.is_some_and(|e| e<=0.25e-7));
    assert!(built.skin_edit_work<=2000000);
    let contacts=built.contacts.as_ref().unwrap();
    for contact in contacts {
        assert!(contact.positions.proven);
        assert!(contact.support_pcurve.control_points.iter()
            .all(|p| (0.5..=1.5).contains(&p[0])));
    }
    let radius=fillet_radius::inspect(
        supports,
        built.fillet.surface.as_ref().unwrap(),
        [&contacts[0],&contacts[1]],
        fillet_radius::RadiusCheck {
            law: &RadiusLaw::Constant(request.radius_mm),
            sides: [1.,-1.],
            tolerance_mm: 1e-7,
            max_cells: 100000,
            max_exact: 1000000,
        },
    )
    .unwrap();
    assert_eq!(radius.decision,fillet_radius::Decision::WithinTolerance,"{}",radius.reason);
    assert!(radius.maximum_center_gap_mm.unwrap()[1]<=1e-7);
    let mut seam_cells=0;
    let mut seam_exact=0;
    for side in 0..2 {
        let c=&contacts[side];
        let seam=nurbs_core::surface_g2_pcurves::inspect(
            nurbs_core::surface_g2_pcurves::PcurveSeam { curve: &c.curve, pcurves: [&c.support_pcurve,&c.fillet_pcurve], surfaces: [supports[side],built.fillet.surface.as_ref().unwrap()], reversed: [false;2] },
            nurbs_core::surface_g2_pcurves::G2PcurveTolerance { position: 1e-7, normal: 1e-5, curvature: 1e-4 },
            100000-seam_cells,
            1000000-seam_exact,
        )
        .unwrap();
        assert!(seam.positions.proven);
        assert_eq!(seam.geometric.decision,nurbs_core::surface_join::Decision::WithinTolerance);
        seam_cells+=seam.geometric.cells;
        seam_exact+=seam.positions.exact_work;
    }
    eprintln!("automatic_curved_r1 g2_cells={seam_cells} g2_exact={seam_exact}");
    assert!(seam_cells<=100000 && seam_exact<=1000000);
    let skin=built.fillet.surface.as_ref().unwrap();
    let sheet=natural_sheet::unit_sheet(skin);
    assert!(sheet.bodies.is_empty() && !sheet.shells[0].closed);
    let field=brep_core::face_curvature::inspect(&sheet,0,10.,1e-8,
        brep_core::face_curvature::Limits{cells:4000,domain_cells:1000000,shape_cells:100000},&[]).unwrap();
    assert_eq!(field.decision,brep_core::face_curvature::Decision::Within,"{}",field.reason);
    assert!(field.absolute_curvature_upper.is_some_and(|k| k<=10.));
    let roots=brep_core::edge_partition::contact_interval::between_contacts(
        brep_core::edge_partition::contact_interval::EdgeContacts {
            source: &source,
            shell: 0,
            request,
            skin,
            contacts: [&contacts[0],&contacts[1]],
        },
        "qualified-fair-r1-roots",
        1e-8,
        Default::default(),
    ).unwrap();
    assert!(roots.branch().contacts.iter().all(|c|c.contact_reversed));
    assert!(roots.root_error_upper().iter().all(|&e|e<=source.tolerance_mm));
    roots.reverify().unwrap();
    eprintln!("automatic_curved_r1 curvature={:?} roots={:?} root_errors={:?}",
        field.absolute_curvature_upper,roots.root_parameters(),roots.root_error_upper());
    let retained=brep_core::face_cut::plan_contact_retention(
        brep_core::edge_partition::contact_interval::EdgeContacts {
            source: &source,
            shell: 0,
            request,
            skin,
            contacts: [&contacts[0],&contacts[1]],
        },
        "qualified-fair-r1-material",
        1e-8,
        Default::default(),
        |r|Ok(fixture::paths(&source,request,contacts,r)),
    ).unwrap();
    assert_eq!(retained.plan().regions.len(),2);
    assert_eq!(retained.plan().model.faces.len(),source.faces.len()+2);
    for region in &retained.plan().regions {
        assert_eq!(region.discard_faces.len(),1);
        assert!(region.descendants.iter().all(|&f|retained.plan().model.faces[f].surface
            ==source.faces[region.source_face].surface));
    }
    retained.reverify().unwrap();
    let ends=region_replacement::inspect_contact_end_normals(&retained).unwrap();
    assert!(ends.needs_expansion());
    eprintln!("automatic_curved_r1 material_faces={} end_expansion={}",
        retained.plan().model.faces.len(),ends.needs_expansion());
    let called=std::cell::Cell::new(false);
    let limits=region_replacement::EmbeddedPatchLimits {
        construction:Default::default(),linear_cells:1,
        global:brep_core::boundary_embedding::Limits {exact_work:1,trim_pairs:1,
            trim_cells:1,trim_domain_cells:1,spans:1,
            contacts:brep_core::face_contacts::Limits{pairs:1,cells:1,domain_cells:1,
                cells_per_pair:1,domain_cells_per_pair:1}},
    };
    match region_replacement::sew_contact_g2_patch_set(retained,"r1-end-gate",1e-8,limits,|_| {
        called.set(true);
        Err(nurbs_core::Error::new("UNEXPECTED_END_BUILD","End region must be expanded first"))
    }) {
        Err(region_replacement::ContactPatchFailure::EndRegion(r))=>assert!(r.needs_expansion()),
        Err(e)=>panic!("Expected end-region refusal, got {e}"),
        Ok(_)=>panic!("An unexpanded R1 stop must not be admitted"),
    }
    assert!(!called.get());
    assert_eq!(source,before);
    eprintln!("automatic_curved_r1 contacts_cells={} contact_exact={} c2_work={} radius_upper={:?}",
        built.cells,built.exact_work,built.skin_edit_work,radius.maximum_center_gap_mm);
}
