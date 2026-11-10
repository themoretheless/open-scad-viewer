//! Whole-field and source-root diagnostics for the automatic R1 proposal.
//! The authored patch is an OPEN sheet; it is never represented as a solid.
#[path = "support/compatible_contact_fixture.rs"]
mod fixture;
use brep_core::{fillet_contact, region_replacement};
use nurbs_core::{curve::Curve,offset::fillet_surfaces::FilletOptions};
#[path = "support/natural_patch_sheet.rs"]
mod natural_sheet;

fn main(){
    let (source,request,_,_)=fixture::fixture();
    let before=source.clone();
    let plan=region_replacement::plan_radius_network(&source,0,&[request],10000).unwrap();
    let supports=plan.edges[0].supports.map(|f|&source.faces[f].surface);
    let built=fillet_contact::build_g2_on_domains(supports,[[0.5,1.5,0.,1.];2],request.radius_mm,
        &FilletOptions{side_a:1.,side_b:-1.,max_sections:21,tolerance:1e-9,..Default::default()},
        1e-7,100000,1000000).unwrap();
    assert_eq!(built.status,fillet_contact::Status::VerifiedContacts);
    let skin=built.fillet.surface.as_ref().unwrap();let contacts=built.contacts.as_ref().unwrap();
    let branch=fillet_contact::convex_branch::inspect(&source,0,request,skin,
        [&contacts[0],&contacts[1]],100000,100000,1000000,100000).unwrap();
    println!("branch={:?} reason={} roots={:?} cells={} exact={} uv={}",
        branch.decision,branch.reason,branch.common_source_clock,branch.cells,branch.exact_work,branch.uv_work);
    let roots=brep_core::edge_partition::contact_interval::between_contacts(
        &source,0,request,skin,[&contacts[0],&contacts[1]],
        "automatic-r1-original-roots",1e-8,Default::default());
    let roots_proven=match &roots {
        Ok(r)=>{println!("shared_roots={:?} errors={:?} cells={} reversed={:?}",r.root_parameters(),
            r.root_error_upper(),r.cells(),r.branch().contacts.iter().map(|c|c.contact_reversed).collect::<Vec<_>>());true},
        Err(e)=>{println!("shared_roots_unproven={e}");false},
    };
    let material=brep_core::face_cut::plan_contact_retention(&source,0,request,skin,
        [&contacts[0],&contacts[1]],"automatic-r1-material",1e-8,Default::default(),|roots| {
            Ok(plan.edges[0].supports.iter().enumerate().map(|(side,&face)| {
                let mut ends=roots.endpoints().map(|v|roots.model().vertices[v].point.to_vec());
                if roots.branch().contacts[side].contact_reversed {ends.reverse();}
                let c=&contacts[side].curve;
                brep_core::face_cut::ContactFaceCut {face,curves:vec![
                    Curve::from_polyline(vec![ends[0].clone(),c.control_points[0].clone()]).unwrap(),
                    c.clone(),Curve::from_polyline(vec![c.control_points.last().unwrap().clone(),ends[1].clone()]).unwrap()],
                    keep_uv:vec![[1.,0.75]]}
            }).collect())
        });
    let material_proven=match &material {
        Ok(p)=>{println!("material_cuts=true faces={} regions={} cells={}",p.plan().model.faces.len(),p.plan().regions.len(),p.cells());
            let ends=region_replacement::inspect_contact_end_normals(p).unwrap();
            println!("end_expansion_needed={} diagnostics={ends:?}",ends.needs_expansion());true},
        Err(e)=>{println!("material_cuts_unproven={e}");false},
    };
    let patch=natural_sheet::unit_sheet(skin);
    let field=brep_core::face_curvature::inspect(&patch,0,10.,1e-8,
        brep_core::face_curvature::Limits{cells:4000,domain_cells:1000000,shape_cells:100000},&[]).unwrap();
    println!("field={:?} reason={} upper={:?} covered={} omitted={} cells={} shape={} region={:?} witness={:?}",
        field.decision,field.reason,field.absolute_curvature_upper,field.covered_cells,
        field.omitted_cells,field.cells,field.shape_cells,field.unresolved_region,field.witness_curvature);
    println!("field_witness_uv={:?}",field.witness_uv);
    assert_eq!(source,before);
    if !material_proven || !roots_proven || field.decision!=brep_core::face_curvature::Decision::Within {std::process::exit(2);}
}
