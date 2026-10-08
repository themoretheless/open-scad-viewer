//! Rebuild source regions and shared incidence from authored support topology.
//! Closed flags and Body records in the input confer no geometric authority.
use crate::source_shell_incidence::{Address, Pair, Pole};
use std::collections::BTreeMap;
pub struct Limits {
    pub tolerance_uv: f64,
    pub faces: usize,
    pub uses: usize,
    /// Independently bounded original-region work per face.
    pub regions: crate::trimmed_face_recipe::Limits,
    pub exact_work: u64,
}
pub fn prepare(
    model: &crate::Model,
    limits: Limits,
) -> crate::Result<crate::source_shell_incidence::Report> {
    if !limits.tolerance_uv.is_finite()
        || limits.tolerance_uv <= 0.
        || !(1..=4096).contains(&limits.faces)
        || !(1..=100000).contains(&limits.uses)
        || !(1..=100_000_000).contains(&limits.exact_work)
    {
        return Err(crate::invalid(
            "Choose bounded source support work and positive UV tolerance",
        ));
    }
    if model.faces.len() > limits.faces
        || model.shells.len() != 1
        || model.shells[0].faces.len() != model.faces.len()
    {
        return Err(crate::invalid(
            "Source support assembly requires one complete connected shell",
        ));
    }
    let count = model.loops.iter().map(|w| w.coedges.len()).sum::<usize>();
    if count > limits.uses {
        return Err(crate::invalid("Source support use limit"));
    }
    model.validate()?;
    // Propose consistently oriented charts. Every proposed lift and shared
    // carrier is freshly checked below; reflected floats grant no authority.
    let oriented = crate::step_interchange_v3::canonical_face_senses(model)?;
    let model = &oriented;
    let mut regions = Vec::new();
    let mut pairs = Vec::new();
    let mut poles = Vec::new();
    let mut pending = BTreeMap::new();
    for (face, support) in model.faces.iter().enumerate() {
        let mut wires = Vec::new();
        for (wire, index) in std::iter::once(support.outer)
            .chain(support.holes.iter().copied())
            .enumerate()
        {
            let mut boundaries = Vec::new();
            for (edge, use_) in model.loops[index].coedges.iter().enumerate() {
                let address = Address { face, wire, edge };
                let original = &model.edges[use_.edge];
                let pcurve = use_.pcurve.clone();
                let reversed = use_.reversed;
                boundaries.push(crate::trimmed_face_recipe::Boundary {
                    curve: original.curve.clone(),
                    pcurve,
                    reversed,
                });
                if original.degenerate {
                    poles.push(Pole {
                        use_: address,
                        point: model.vertices[original.vertices[0]].point,
                    });
                } else if let Some((other, reversed)) = pending.remove(&use_.edge) {
                    pairs.push(Pair {
                        uses: [other, address],
                        world: original.curve.clone(),
                        world_reversed: [reversed, use_.reversed],
                        cutters: [None, None],
                    });
                } else {
                    pending.insert(use_.edge, (address, reversed));
                }
            }
            wires.push(boundaries);
        }
        let qualified = crate::source_contour_proposal::qualify_original_region(
            &cad_predicates::ToleranceContext::default_valid(),
            &support.surface,
            &wires,
            limits.tolerance_uv,
            crate::trimmed_face_recipe::Limits {
                pairs: limits.regions.pairs,
                region_cells: limits.regions.region_cells,
                domain_cells: limits.regions.domain_cells,
                agreement_cells: limits.regions.agreement_cells,
            },
        )?;
        let Some(region) = qualified.region else {
            return Err(crate::invalid(format!(
                "Source support face {face}: {}",
                qualified.audit.reason
            )));
        };
        regions.push(region);
    }
    if !pending.is_empty() {
        return Err(crate::invalid(
            "Source supports retain unpaired ordinary edges",
        ));
    }
    crate::source_shell_incidence::assemble_regions_with_poles(
        &regions,
        &pairs,
        &poles,
        limits.exact_work,
    )
    .map_err(|e| crate::Error::new(e.code, e.message))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn limits() -> Limits {
        Limits {
            tolerance_uv: 1e-8,
            faces: 4096,
            uses: 100000,
            regions: crate::trimmed_face_recipe::Limits {
                pairs: 10000,
                region_cells: 10000,
                domain_cells: 10000,
                agreement_cells: 10000,
            },
            exact_work: 100_000_000,
        }
    }
    fn geometry_limits() -> crate::source_shell_geometry::Limits {
        crate::source_shell_geometry::Limits {
            tolerance_uv: 1e-8,
            corners: 10000,
            spans: 100000,
            linear_cells: 100000,
            exact_work: 100_000_000,
            driver_cells: 100000,
            pairs: crate::face_contacts::Limits {
                pairs: 10000,
                cells: 10000,
                domain_cells: 100000,
                cells_per_pair: 32,
                domain_cells_per_pair: 512,
            },
        }
    }
    #[test]
    fn source_retained_gap_covers_owned_opposite_faces_and_preserves_exhaustion() {
        let model=crate::cuboid([0.,0.,0.],[10.,20.,30.]).unwrap();
        let shell=prepare(&model,limits()).unwrap().shell.unwrap();
        let geometry=crate::source_shell_geometry::qualify(shell,geometry_limits()).unwrap().geometry.unwrap();
        let volume=crate::source_volume::qualify(geometry,crate::source_volume::Limits{
            axis:2,origin:0.,absolute_error:0.02,tolerance_uv:1e-8,cells:100000,spans:100000,domain_cells:1000000}).unwrap();
        let body=volume.body.unwrap();
        let options=||crate::source_retained_face_gap::Limits{cells:8,spans:16,domain_cells:1,domain_cells_per_query:1,tolerance_uv:1e-8};
        let proved=crate::source_retained_face_gap::qualify(&body,[0,1],29.99,options()).unwrap();
        let c=proved.certificate.unwrap();assert!(std::ptr::eq(c.body(),&body));assert_eq!(c.faces(),[0,1]);assert_eq!(c.lower_mm(),29.99);
        assert!(proved.unresolved_products.is_empty());assert_eq!(proved.cells,1);assert_eq!(proved.spans,2);assert!(proved.domain_cells<=1);
        let mixed_options=||crate::source_wall_pair::Limits{normal_spans:1000,max_sine_squared:1e-6,
            cells:64,spans:128,domain_cells:1000,domain_cells_per_query:100,tolerance_uv:1e-8};
        let perpendicular=crate::source_wall_pair::qualify(&body,[0,2],100.,mixed_options()).unwrap();
        let proof=perpendicular.certificate.unwrap();
        assert!(std::ptr::eq(proof.body(),&body));assert_eq!(proof.faces(),[0,2]);
        assert_eq!(proof.lower_mm(),100.);assert_eq!(proof.max_sine_squared(),1e-6);
        assert_eq!(perpendicular.normal_products,1);assert!(perpendicular.unresolved_products.is_empty());
        // Adjacent faces have zero gap, but their orthogonal normal directions
        // exclude the aligned chords required by this wall criterion.
        assert!(crate::source_retained_face_gap::qualify(&body,[0,2],100.,options()).unwrap().certificate.is_none());
        let too_thin=crate::source_wall_pair::qualify(&body,[0,1],30.01,mixed_options()).unwrap();
        assert!(too_thin.certificate.is_none());assert!(!too_thin.unresolved_products.is_empty());
        // UV resolution is a refusal boundary, never a thickness certificate.
        let mut coarse=mixed_options();coarse.tolerance_uv=100.;
        let stopped=crate::source_wall_pair::qualify(&body,[0,1],30.01,coarse).unwrap();
        assert!(stopped.certificate.is_none());assert_eq!(stopped.reason,"source-wall-pair-resolution-limit");
        assert_eq!(stopped.cells,1);assert_eq!(stopped.unresolved_products.len(),1);

        assert!(crate::source_wall_pair::qualify(&body,[0,1],29.99,mixed_options()).unwrap().certificate.is_some());
        let coverage_options=||crate::source_wall_coverage::Limits{pairs:21,plane_controls:10000,normal_spans:10000,gap:crate::source_face_gap::Limits{cells:10000,spans:20000},max_sine_squared:1e-6};
        let no_products=||crate::source_wall_coverage::NormalProductLimits{cells:0,cells_per_pair:0};
        let no_factors=||crate::source_wall_coverage::FactoredNormalLimits{exact_work:0,queries:0,cells:0,cells_per_pair:0};
        let rejected=crate::source_wall_coverage::qualify_with_retained_gap_facts(&body,30.01,coverage_options(),Vec::new(),no_products(),no_factors(),vec![c]);assert!(rejected.is_err());
        let c1=crate::source_retained_face_gap::qualify(&body,[0,1],29.99,options()).unwrap().certificate.unwrap();
        let c2=crate::source_retained_face_gap::qualify(&body,[1,0],29.99,options()).unwrap().certificate.unwrap();
        assert!(crate::source_wall_coverage::qualify_with_retained_gap_facts(&body,9.99,coverage_options(),Vec::new(),no_products(),no_factors(),vec![c1,c2]).is_err());
        let owned=crate::source_retained_face_gap::qualify(&body,[0,1],29.99,options()).unwrap().certificate.unwrap();
        let coverage=crate::source_wall_coverage::qualify_with_retained_gap_facts(&body,9.99,coverage_options(),Vec::new(),no_products(),no_factors(),vec![owned]).unwrap();
        let complete=coverage.certificate.unwrap();assert_eq!(complete.retained_gap_certificates().len(),1);assert!(std::ptr::eq(complete.body(),&body));
        let shell=prepare(&model,limits()).unwrap().shell.unwrap();
        let geometry=crate::source_shell_geometry::qualify(shell,geometry_limits()).unwrap().geometry.unwrap();
        let other=crate::source_volume::qualify(geometry,crate::source_volume::Limits{axis:2,origin:0.,absolute_error:0.02,tolerance_uv:1e-8,cells:100000,spans:100000,domain_cells:1000000}).unwrap().body.unwrap();
        macro_rules! mixed_coverage {
            ($target:expr,$minimum:expr,$angle:expr,$proofs:expr) => {{
                let mut l=coverage_options();l.max_sine_squared=$angle;
                crate::source_wall_coverage::qualify_with_mixed_pair_facts($target,$minimum,l,
                    Vec::new(),no_products(),no_factors(),Vec::new(),None,$proofs)
            }};
        }
        let mixed=crate::source_wall_pair::qualify(&body,[0,2],100.,mixed_options()).unwrap().certificate.unwrap();
        let complete=mixed_coverage!(&body,9.99,1e-6,vec![mixed]).unwrap().certificate.unwrap();
        assert_eq!(complete.mixed_pair_certificates().len(),1);
        let mixed=crate::source_wall_pair::qualify(&body,[0,2],100.,mixed_options()).unwrap().certificate.unwrap();
        assert!(mixed_coverage!(&body,100.01,1e-6,vec![mixed]).is_err());
        let mixed=crate::source_wall_pair::qualify(&body,[0,2],100.,mixed_options()).unwrap().certificate.unwrap();
        assert!(mixed_coverage!(&body,9.99,1e-5,vec![mixed]).is_err());
        let mixed=crate::source_wall_pair::qualify(&other,[0,2],100.,mixed_options()).unwrap().certificate.unwrap();
        assert!(mixed_coverage!(&body,9.99,1e-6,vec![mixed]).is_err());
        let first=crate::source_wall_pair::qualify(&body,[0,2],100.,mixed_options()).unwrap().certificate.unwrap();
        let reversed=crate::source_wall_pair::qualify(&body,[2,0],100.,mixed_options()).unwrap().certificate.unwrap();
        assert!(mixed_coverage!(&body,9.99,1e-6,vec![first,reversed]).is_err());
        let foreign=crate::source_retained_face_gap::qualify(&other,[0,1],29.99,options()).unwrap().certificate.unwrap();
        assert!(crate::source_wall_coverage::qualify_with_retained_gap_facts(&body,9.99,coverage_options(),Vec::new(),no_products(),no_factors(),vec![foreign]).is_err());
        let pass=crate::source_wall_coverage::RetainedGapLimits{cells:1,spans:2,domain_cells:1,cells_per_pair:1,spans_per_pair:2,domain_cells_per_pair:1,domain_cells_per_query:1,tolerance_uv:1e-8};
        let bounded=crate::source_wall_coverage::qualify_with_retained_gap_pass(&body,30.01,coverage_options(),Vec::new(),no_products(),no_factors(),Vec::new(),Some(pass)).unwrap();
        assert!(bounded.certificate.is_none());let extra=bounded.retained_gap.unwrap();assert_eq!(extra.cells,1);assert!(extra.spans<=2);assert!(extra.domain_cells<=1);assert_eq!(extra.pairs.len(),1);assert!(!extra.pairs[0].qualified);assert!(!extra.pairs[0].unresolved_products.is_empty());
        let mixed_pass=crate::source_wall_coverage::MixedPairLimits{domains:pass,normal_spans:1,normal_spans_per_pair:1};
        let limited=crate::source_wall_coverage::qualify_with_mixed_pair_pass(&body,30.01,coverage_options(),
            Vec::new(),no_products(),no_factors(),Vec::new(),None,Vec::new(),Some(mixed_pass)).unwrap();
        assert!(limited.certificate.is_none());
        for old in bounded.pairs.iter().filter(|p|p.proven) {
            assert!(limited.pairs.iter().find(|p|p.faces==old.faces).unwrap().proven);
        }
        let diagnostics=limited.mixed.unwrap();assert_eq!(diagnostics.pairs.len(),1);
        assert_eq!(diagnostics.cells,1);assert!(diagnostics.spans<=2);assert!(diagnostics.domain_cells<=1);
        assert!(diagnostics.normal_spans<=1);assert!(!diagnostics.pairs[0].retained.qualified);
        assert!(!diagnostics.pairs[0].retained.unresolved_products.is_empty());
        let mut invalid_mixed=mixed_pass;invalid_mixed.normal_spans_per_pair=0;
        assert!(crate::source_wall_coverage::qualify_with_mixed_pair_pass(&body,9.99,coverage_options(),
            Vec::new(),no_products(),no_factors(),Vec::new(),None,Vec::new(),Some(invalid_mixed)).is_err());
        let mut invalid=pass;invalid.cells_per_pair=0;assert!(crate::source_wall_coverage::qualify_with_retained_gap_pass(&body,9.99,coverage_options(),Vec::new(),no_products(),no_factors(),Vec::new(),Some(invalid)).is_err());
        let reversed=crate::source_retained_face_gap::qualify(&body,[1,0],29.99,options()).unwrap();assert!(reversed.certificate.is_some());
        let refused=crate::source_retained_face_gap::qualify(&body,[0,1],30.01,options()).unwrap();
        assert!(refused.certificate.is_none());assert!(!refused.unresolved_products.is_empty());assert_eq!(refused.cells,8);assert!(refused.spans<=16);assert!(refused.domain_cells<=1);
        let repeated=crate::source_retained_face_gap::qualify(&body,[0,1],30.01,
            crate::source_retained_face_gap::Limits{cells:8,spans:16,domain_cells:10000,domain_cells_per_query:1000,tolerance_uv:1e-8}).unwrap();
        assert!(repeated.certificate.is_none());
        assert!(repeated.domain_queries < 2*repeated.cells,"Repeated rectangles should reuse bounded classifications: {} queries for {} cells",repeated.domain_queries,repeated.cells);
        assert!(crate::source_retained_face_gap::qualify(&body,[0,0],29.99,options()).is_err());
        assert!(crate::source_retained_face_gap::qualify(&body,[0,6],29.99,options()).is_err());
        let mut invalid=options();invalid.domain_cells=0;assert!(crate::source_retained_face_gap::qualify(&body,[0,1],29.99,invalid).is_err());
    }
    #[test]
    fn original_cuboid_whole_wall_coverage_requires_all_pairs() {
        let model=crate::cuboid([0.,0.,0.],[10.,20.,30.]).unwrap();
        let shell=prepare(&model,limits()).unwrap().shell.expect("original cuboid incidence");
        let admission=crate::source_shell_geometry::qualify(shell,geometry_limits()).unwrap();
        let geometry=admission.geometry.expect(admission.reason);
        let volume=crate::source_volume::qualify(geometry,crate::source_volume::Limits {
            axis:2,origin:0.,absolute_error:0.02,tolerance_uv:1e-8,
            cells:100000,spans:100000,domain_cells:1000000,
        }).unwrap();
        let body=volume.body.expect(volume.reason);
        if let Some(path)=std::env::var_os("CAD_CUBOID_SOURCE_BODY_OUTPUT") {
            std::fs::write(path,value_codec::to_string_pretty(&body.definition().unwrap()).unwrap()).unwrap();
        }
        for (minimum,pairs,normal_spans,success) in [(9.99,21,10000,true),(10.01,21,10000,false),(9.99,20,10000,false),(9.99,21,6,true),(9.99,21,1,false)] {
            let report=crate::source_wall_coverage::qualify(&body,minimum,
                crate::source_wall_coverage::Limits {
                    pairs,plane_controls:10000,normal_spans,
                    gap:crate::source_face_gap::Limits{cells:10000,spans:20000},
                    max_sine_squared:1e-6,
                }).unwrap();
            eprintln!("original cuboid wall minimum={minimum} reason={} pairs={}/{} cells={} normals={}",
                report.reason,report.pairs.len(),report.total_pairs,report.cells,report.normal_spans);
            let factored=crate::source_wall_coverage::qualify_with_self_facts_and_factored_normals(
                &body,minimum,crate::source_wall_coverage::Limits{pairs,plane_controls:10000,normal_spans,
                    gap:crate::source_face_gap::Limits{cells:10000,spans:20000},max_sine_squared:1e-6},
                Vec::new(),crate::source_wall_coverage::NormalProductLimits{cells:0,cells_per_pair:0},
                crate::source_wall_coverage::FactoredNormalLimits{exact_work:1,queries:1,cells:1,cells_per_pair:1}).unwrap();
            assert_eq!(factored.certificate.is_some(),report.certificate.is_some());
            assert_eq!(factored.pairs.len(),report.pairs.len());
            assert_eq!(factored.normal_spans,report.normal_spans);
            assert!(factored.factored_exact_work<=1);
            assert_eq!(factored.factored_queries,0);assert_eq!(factored.factored_product_cells,0);
            for (old,new) in report.pairs.iter().zip(&factored.pairs) {
                assert_eq!(old.faces,new.faces);assert_eq!(old.proven,new.proven);assert_eq!(old.reason,new.reason);
            }
            assert_eq!(report.total_pairs,21);
            assert_eq!(report.enumeration_complete,pairs==21);
            assert_eq!(report.certificate.is_some(),success,"{}",report.reason);
            assert!(report.normal_spans<=normal_spans.min(6),"whole-chart normals must be paid once per original face");
            if let Some(certificate)=report.certificate {
                assert!(std::ptr::eq(certificate.body(),&body));
                assert_eq!(certificate.minimum_mm(),minimum);
                assert_eq!(certificate.max_sine_squared(),1e-6);
                assert!(!certificate.gap_certificates().is_empty());
            }
        }
        for (minimum,angle,accepted) in [(9.99,1e-6,true),(10.01,1e-6,false),(9.99,2e-6,false)] {
            let self_facts=crate::source_wall_self_coverage::qualify(&body,9.99,1e-6,
                crate::source_wall_self_coverage::Limits{plane_controls:10000,cells:1,spans:1,cells_per_face:1,spans_per_face:1}).unwrap();
            assert!(self_facts.all_self_pairs_qualified());assert_eq!(self_facts.certificates().len(),6);
            let result=crate::source_wall_coverage::qualify_with_self_facts(&body,minimum,
                crate::source_wall_coverage::Limits{pairs:21,plane_controls:10000,normal_spans:10000,
                    gap:crate::source_face_gap::Limits{cells:10000,spans:20000},max_sine_squared:angle},self_facts.into_certificates());
            if accepted {
                let report=result.unwrap();let proof=report.certificate.expect("original self facts plus distinct gaps qualify the cuboid");
                assert_eq!(proof.adaptive_self_certificates().len(),6);assert!(std::ptr::eq(proof.body(),&body));
            } else {assert_eq!(result.err().expect("stale threshold or angle must refuse").code,"BREP_SOURCE_SELF_WALL_FACT");}
        }
        let twin_shell=prepare(&model,limits()).unwrap().shell.unwrap();
        let twin_geometry=crate::source_shell_geometry::qualify(twin_shell,geometry_limits()).unwrap().geometry.unwrap();
        let twin=crate::source_volume::qualify(twin_geometry,crate::source_volume::Limits{
            axis:2,origin:0.,absolute_error:0.02,tolerance_uv:1e-8,cells:100000,spans:100000,domain_cells:1000000,
        }).unwrap().body.unwrap();
        assert!(!std::ptr::eq(&body,&twin));
        let extension_limits=crate::source_wall_self_coverage::Limits{
            plane_controls:10000,cells:1,spans:1,cells_per_face:1,spans_per_face:1};
        let mut extension=crate::source_wall_self_coverage::qualify(&body,9.99,1e-6,extension_limits).unwrap();
        assert!(crate::source_wall_self_coverage::extend_with_pole_facts(&twin,9.99,1e-6,&mut extension,extension_limits,1).is_err());
        assert!(crate::source_wall_self_coverage::extend_with_pole_facts(&body,10.01,1e-6,&mut extension,extension_limits,1).is_err());
        assert!(crate::source_wall_self_coverage::extend_with_pole_facts(&body,9.99,2e-6,&mut extension,extension_limits,1).is_err());
        assert!(crate::source_wall_self_coverage::extend_with_pole_facts(&body,9.99,1e-6,&mut extension,extension_limits,100000001).is_err());
        crate::source_wall_self_coverage::extend_with_pole_facts(&body,9.99,1e-6,&mut extension,extension_limits,1).unwrap();
        assert!(extension.all_self_pairs_qualified());
        assert_eq!((extension.pole_exact_work,extension.pole_cells,extension.pole_spans),(0,0,0));
        assert!(crate::source_wall_self_coverage::refine_unqualified(&twin,9.99,1e-6,&mut extension,extension_limits).is_err());
        assert!(crate::source_wall_self_coverage::refine_unqualified(&body,10.01,1e-6,&mut extension,extension_limits).is_err());
        assert!(crate::source_wall_self_coverage::refine_unqualified(&body,9.99,2e-6,&mut extension,extension_limits).is_err());
        crate::source_wall_self_coverage::refine_unqualified(&body,9.99,1e-6,&mut extension,extension_limits).unwrap();
        assert!(extension.all_self_pairs_qualified());
        assert_eq!((extension.refinement_cells,extension.refinement_spans),(0,0));
        assert!(crate::source_wall_self_coverage::refine_unqualified(&body,9.99,1e-6,&mut extension,extension_limits).is_err());


        for wrong_owner in [true,false] {
            let make_facts=||crate::source_wall_self_coverage::qualify(&body,9.99,1e-6,
                crate::source_wall_self_coverage::Limits{plane_controls:10000,cells:1,spans:1,cells_per_face:1,spans_per_face:1}).unwrap().into_certificates();
            let mut facts=make_facts();
            if !wrong_owner {facts.push(make_facts().remove(0));}
            let target=if wrong_owner {&twin}else{&body};
            let result=crate::source_wall_coverage::qualify_with_self_facts(target,9.99,
                crate::source_wall_coverage::Limits{pairs:21,plane_controls:10000,normal_spans:10000,
                    gap:crate::source_face_gap::Limits{cells:10000,spans:20000},max_sine_squared:1e-6},facts);
            assert_eq!(result.err().expect("identical geometry does not grant Body identity; repeated face facts refuse").code,"BREP_SOURCE_SELF_WALL_FACT");
        }
    }
    #[test]
    fn restricted_profile_seams_match_owned_neighbor_supports() {
        let model=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let shell=prepare(&model,limits()).unwrap().shell.unwrap();
        let geometry=crate::source_shell_geometry::qualify(shell,geometry_limits()).unwrap().geometry.unwrap();
        let body=crate::source_volume::qualify(geometry,crate::source_volume::Limits {
            axis:2,origin:0.,absolute_error:20.,tolerance_uv:1e-8,cells:100000,spans:100000,domain_cells:1000000,
        }).unwrap().body.unwrap();
        let specs=[crate::source_profile_body::FaceSpec{face:0,anchor:[5,0]},crate::source_profile_body::FaceSpec{face:5,anchor:[0,0]}];
        let report=crate::source_profile_body::prepare(&body,&specs,
            crate::source_profile_body::Limits{controls:1000,fragments:100,position_tolerance:1e-7}).unwrap();
        let proposal=report.proposal.unwrap();
        let mut rooted_seams=Vec::new();
        for (edge,canonical,faces) in [(0,[0.1,0.25],[0,1]),(1,[0.75,0.9],[0,2])] {
            let report=proposal.qualify_edge_interval(edge,canonical,1e-6,100000,100000).unwrap();
            let seam=report.seam.expect(report.angular.reason);
            assert!(std::ptr::eq(seam.proposal(),&proposal));
            assert_eq!(seam.edge(),edge);assert_eq!(seam.faces(),faces);
            assert_eq!(seam.canonical_interval(),canonical);
            assert_eq!(report.angular.aligned,Some(true));assert!(report.angular.pending.is_empty());
            assert!(seam.upper_sine_squared()<=seam.threshold());
            eprintln!("restricted seam edge={edge} interval={canonical:?} cells={} spans={} upper={}",report.angular.cells,report.angular.spans,seam.upper_sine_squared());
            let high=edge==1;
            let a=proposal.cut_endpoint(0,0,0.75,high,edge,16).unwrap().endpoint.unwrap();
            let b=proposal.cut_endpoint(0,0,0.9,high,edge,16).unwrap().endpoint.unwrap();
            let pa=a.split_source_use(100).unwrap().partition.unwrap();
            let pb=b.split_source_use(100).unwrap().partition.unwrap();
            let rooted=proposal.qualify_root_ended_seam([&pa,&pb],1e-6,100000,100000).unwrap();
            let rooted_seam=rooted.seam.expect(rooted.reason);
            assert!(std::ptr::eq(rooted_seam.restricted().proposal(),&proposal));
            assert_eq!(rooted_seam.restricted().edge(),edge);
            assert!(rooted_seam.propose_incident_contours(0).unwrap().proposal.is_none());
            assert!(rooted_seam.propose_incident_contours(100001).is_err());
            let contours=rooted_seam.propose_incident_contours(100).unwrap();
            let contours=contours.proposal.expect(contours.reason);
            let incident=rooted_seam.incident_fragments().unwrap();
            for (side,contour) in contours.contours().iter().enumerate() {
                let original=&body.geometry().shell().regions().unwrap()[contour.face()];
                assert_eq!(contour.loops().len(),original.loops().len());
                let [wire,index]=contour.middle();
                assert_eq!(contour.loops()[wire].len(),original.loops()[wire].len()+2);
                assert_eq!(contour.loops()[wire][index].definition(),incident[side].definition());
                for (loop_index,loop_) in original.loops().iter().enumerate() {
                    for (edge_index,fragment) in loop_.iter().enumerate() {
                        if loop_index==wire && edge_index==index-1 {continue;}
                        let shifted=if loop_index==wire && edge_index>=index {edge_index+2}else{edge_index};
                        assert_eq!(contour.loops()[loop_index][shifted].definition(),fragment.definition());
                    }
                }
                for loop_ in contour.loops() {
                    assert!((0..loop_.len()).all(|i|loop_[i].joins(&loop_[(i+1)%loop_.len()])));
                }
                for fragment in &contour.loops()[wire][index-1..=index+1] {
                    let restored=crate::source_boundary_fragment::restore(fragment.definition(),100).unwrap();
                    assert_eq!(restored.definition(),fragment.definition());
                }
            }
            let shared=rooted_seam.qualify_incident_edge(100000000,100000).unwrap();
            let shared_edge=shared.edge.expect(shared.reason);
            assert!(shared_edge.uses().iter().all(|u|u.endpoints().iter().all(|e|matches!(e,crate::source_boundary_fragment::Endpoint::Carrier{..}))));
            let fresh_shared=crate::source_shared_edge_restore::restore(shared_edge.definition(),crate::source_shared_edge_restore::Limits{mapping_cells_per_use:100,exact_work:100000000,driver_cells:100000}).unwrap();
            let fresh_shared=fresh_shared.edge.expect(fresh_shared.reason);
            assert_eq!(fresh_shared.definition(),shared_edge.definition());

            assert!(rooted_seam.qualify_incident_edge(1,1).unwrap().edge.is_none());
            eprintln!("two-root retained edge={edge} work={} drivers={} rootChecks={}",shared.work_used,shared.driver_cells,shared.root_checks);
            for fragment in rooted_seam.incident_fragments().unwrap() {
                assert!(fragment.endpoints().iter().all(|e|matches!(e,crate::source_boundary_fragment::Endpoint::Carrier{..})));
                let restored=crate::source_boundary_fragment::restore(fragment.definition(),100).unwrap();
                assert_eq!(restored.definition(),fragment.definition());
                assert_eq!(restored.reversed(),fragment.reversed());
                assert!(crate::source_boundary_fragment::restore(fragment.definition(),1).is_err());
            }

            for point in rooted_seam.points() {
                let q=point.canonical_bounds();let whole=rooted_seam.restricted().canonical_interval();
                assert!(whole[0]<=q[0]&&q[1]<=whole[1]);
                let fresh=crate::source_carrier_point::restore(point.definition(),crate::source_carrier_point::RestoreLimits{edge:crate::source_shared_edge_restore::Limits{mapping_cells_per_use:16,exact_work:100000000,driver_cells:10000},root_mapping_cells:16,spans:3}).unwrap().point.unwrap();
                assert_eq!(fresh.definition(),point.definition());
            }
            let twin=crate::source_profile_body::prepare(&body,&specs,crate::source_profile_body::Limits{controls:1000,fragments:100,position_tolerance:1e-7}).unwrap().proposal.unwrap();
            fn make_bridge<'p,'b>(profile:&'p nurbs_core::normal_alignment::WeightProfile<'b>,end:f64)->nurbs_core::normal_alignment::ProfileSectionBridge<'p,'b,'p,'b> {
                nurbs_core::normal_alignment::ProfileSectionBridge::new(profile.section(0,0.75).unwrap(),profile.section(0,end).unwrap(),[end-0.75;2]).unwrap()
            }
            let original_bridge=make_bridge(proposal.profile(0).unwrap(),0.9);
            let original_bridge=original_bridge.constrain_original_rails(1000).unwrap().0.unwrap();
            let bound=rooted_seam.qualify_transition_side(&original_bridge,1000,1000).unwrap();
            let bound=bound.side.expect(bound.reason);
            assert!(std::ptr::eq(bound.rooted(),&rooted_seam));assert_eq!(bound.high_free_end(),high);
            assert_eq!(bound.parameters(),[0.75,0.9]);assert_eq!(bound.axis(),0);assert_eq!(bound.face(),0);
            let wrong_bridge=make_bridge(proposal.profile(0).unwrap(),0.85);
            let wrong_bridge=wrong_bridge.constrain_original_rails(1000).unwrap().0.unwrap();
            assert!(rooted_seam.qualify_transition_side(&wrong_bridge,1000,1000).unwrap().side.is_none());
            let foreign_bridge=make_bridge(twin.profile(0).unwrap(),0.9);
            let foreign_bridge=foreign_bridge.constrain_original_rails(1000).unwrap().0.unwrap();
            assert!(rooted_seam.qualify_transition_side(&foreign_bridge,1000,1000).unwrap().side.is_none());
            assert!(rooted_seam.qualify_transition_side(&original_bridge,0,1000).unwrap().side.is_none());

            assert!(twin.qualify_root_ended_seam([&pa,&pb],1e-6,100,100).unwrap().seam.is_none());
            assert!(proposal.qualify_root_ended_seam([&pa,&pa],1e-6,100,100).unwrap().seam.is_none());
            assert!(proposal.qualify_root_ended_seam([&pa,&pb],1e-6,100,1).unwrap().seam.is_none());
            eprintln!("root-ended seam edge={edge} carrierSpans={} canonical={:?}",rooted.carrier_spans,rooted_seam.restricted().canonical_interval());
            let limited=proposal.qualify_edge_interval(edge,canonical,1e-6,1,1).unwrap();
            assert!(limited.seam.is_none());assert!(!limited.angular.pending.is_empty());
            rooted_seams.push(rooted_seam);

        }
        let bridge=nurbs_core::normal_alignment::ProfileSectionBridge::new(
            proposal.profile(0).unwrap().section(0,0.75).unwrap(),proposal.profile(0).unwrap().section(0,0.9).unwrap(),[0.9-0.75;2]).unwrap();
        let jet=bridge.derivative_bounds([0.5;2],[0.5;2],8).unwrap().0.unwrap();
        let vectors=jet.map(|r|r.map(|b|b[0]*0.5+b[1]*0.5));
        let dot=|a:[f64;3],b:[f64;3]|a.iter().zip(b).map(|(x,y)|x*y).sum::<f64>();
        let aa=dot(vectors[0],vectors[0]);let bb=dot(vectors[1],vectors[1]);let ab=dot(vectors[0],vectors[1]);
        let determinant=aa*bb-ab*ab;assert!(determinant>0.);
        let projection=[std::array::from_fn(|k|(bb*vectors[0][k]-ab*vectors[1][k])/determinant),
            std::array::from_fn(|k|(aa*vectors[1][k]-ab*vectors[0][k])/determinant)];
        let bridge=bridge.constrain_original_rails(1000).unwrap().0.unwrap();
        let sides=rooted_seams.iter().map(|root|root.qualify_transition_side(&bridge,1000,1000).unwrap().side.unwrap()).collect::<Vec<_>>();
        let boundary=crate::source_profile_body::propose_transition_boundary([&sides[1],&sides[0]]).unwrap();
        assert!(crate::source_profile_body::propose_transition_boundary([&sides[0],&sides[0]]).is_err());
        let other_boundary=crate::source_profile_body::propose_transition_boundary([&sides[0],&sides[1]]).unwrap();
        let mut other_sections=Vec::new();
        let mut section_bindings=Vec::new();
        for high in [false,true] {
            let parameter=if high {0.9}else{0.75};
            let ends=[proposal.cut_endpoint(0,0,parameter,false,0,16).unwrap().endpoint.unwrap(),
                proposal.cut_endpoint(0,0,parameter,true,1,16).unwrap().endpoint.unwrap()];
            let parts=ends.each_ref().map(|end|end.split_source_use(100).unwrap().partition.unwrap());
            let section=boundary.propose_section_contours(high,[&parts[0],&parts[1]],100).unwrap();
            let section=section.section.expect(section.reason);
            assert!(std::ptr::eq(section.boundary(),&boundary));
            assert_eq!(section.edge_index(),if high {1}else{3});
            assert!(section.qualify_exterior(0,0).unwrap().region.is_none());
            let exterior=section.qualify_exterior(1000,1000).unwrap();
            let exterior=exterior.region.expect(exterior.reason);
            assert!(std::ptr::eq(exterior.section(),&section));
            assert!(!exterior.kept_loop().is_empty());
            assert!(!exterior.removed_loop().is_empty());
            let injectivity=exterior.inspect_quotient_injectivity(1,16,256).unwrap();
            assert!(injectivity.proven,"owned profile must qualify globally");
            let retained=exterior.qualify_face(1,16,256,2048,12288).unwrap();
            let retained_face=retained.face.expect(retained.reason);
            assert!(std::ptr::eq(retained_face.exterior(),&exterior));
            assert_eq!(retained_face.factored_boundary(),if high {Some(1)}else{None});
            assert!(exterior.qualify_face(1,16,255,2048,12288).unwrap().face.is_none());
            assert!(exterior.qualify_face(1,16,256,0,12288).unwrap().face.is_none());
            eprintln!("retained profile quotient high={high} proven={} cells={} reason={} bounds={:?} margins={:?}",injectivity.proven,injectivity.cells,injectivity.reason,injectivity.weighted_bounds,injectivity.band_margins_lower);
            let cover=exterior.inspect_regularity(128,256).unwrap();
            let area:f64=cover.regular.iter().chain(&cover.pending).map(|r|(r[0][1]-r[0][0])*(r[1][1]-r[1][0])).sum();
            assert!((area-if high {0.1}else{0.75}).abs()<1e-12);
            assert!(cover.cells<=128 && cover.spans<=256);
            if !high {assert!(cover.complete_regular);assert!(cover.pending.is_empty());}
            eprintln!("retained profile high={high} regular={} pending={} cells={} spans={}",cover.regular.len(),cover.pending.len(),cover.cells,cover.spans);
            if high {
                let profile=proposal.profile(0).unwrap();let mut factored_regular=0;
                for domain in &cover.pending {
                    let (normal,spent)=profile.factored_pole_normal_bounds(*domain,true,7).unwrap();assert!((6..=7).contains(&spent));
                    let normal=normal.unwrap();assert_eq!(normal.point,[20.,0.,6.]);
                    if normal.components.iter().any(|r|r[0]>0. || r[1]<0.) {factored_regular+=1;}
                }
                eprintln!("retained terminal factored normal accepted={} pending={}",factored_regular,cover.pending.len()-factored_regular);
                let factored=exterior.inspect_factored_pole_regularity(true,2048,12288).unwrap();
                assert_eq!(factored.point,Some([20.,0.,6.]));
                assert!(factored.complete_factored,"entire retained terminal factored cover must qualify");
                assert!(factored.pending.is_empty());assert_eq!(factored.power,Some(2));
                let area:f64=factored.regular.iter().chain(&factored.pending).map(|r|(r[0][1]-r[0][0])*(r[1][1]-r[1][0])).sum();
                assert!((area-0.1).abs()<1e-12);
                eprintln!("terminal factored cover complete={} regular={} pending={} cells={} spans={} power={:?}",factored.complete_factored,factored.regular.len(),factored.pending.len(),factored.cells,factored.spans,factored.power);
                for free in [0.,0.5,1.] {
                    let normal=profile.factored_pole_normal_bounds([[1.;2],[free;2]],true,7).unwrap().0.unwrap();
                    eprintln!("pole scaled normal free={free} components={:?}",normal.components);
                }
                eprintln!("factored pending sample={:?}",&factored.pending[..factored.pending.len().min(3)]);
            }
            let refused=exterior.inspect_regularity(0,256).unwrap();
            assert!(!refused.complete_regular);assert_eq!(refused.pending.len(),1);
            let expected=if high {1.}else{0.};
            assert!(exterior.kept_loop().iter().any(|f|f.curve().control_points.iter().all(|p|p[0]==expected)));
            assert!(!exterior.removed_loop().iter().any(|f|f.curve().control_points.iter().all(|p|p[0]==expected)));
            for loop_ in section.contours().loops() {
                assert!((0..loop_.len()).all(|i|loop_[i].joins(&loop_[(i+1)%loop_.len()])));
            }
            assert!(boundary.propose_section_contours(!high,[&parts[0],&parts[1]],100).unwrap().section.is_none());
            assert!(boundary.propose_section_contours(high,[&parts[1],&parts[0]],100).unwrap().section.is_none());
            assert!(boundary.propose_section_contours(high,[&parts[0],&parts[1]],0).unwrap().section.is_none());
            other_sections.push(other_boundary.propose_section_contours(high,[&parts[0],&parts[1]],100).unwrap().section.unwrap());
            section_bindings.push(section);
        }
        assert!(boundary.enclose_edge(4,[0.,1.],1).is_err());
        assert!(boundary.enclose_edge(0,[-0.1,0.5],1).is_err());
        assert!(boundary.enclose_edge(0,[0.75,0.25],1).is_err());

        let refused=boundary.qualify_face([[0.;3];2],1000,0,0).unwrap();
        assert!(refused.face.is_none());
        let interior=refused.interior.unwrap();
        assert!(!interior.complete_regular);
        assert_eq!(interior.pending.len(),1);
        assert!(boundary.qualify_face([[f64::NAN;3];2],1000,1,100).is_err());
        assert!(boundary.qualify_face([[0.;3];2],0,1,100).unwrap().face.is_none());
        let admitted=boundary.qualify_face(projection,1000,1000,61000).unwrap();
        let face=admitted.face.expect("complete interior admission for the owned annular transition");
        assert!(std::ptr::eq(face.boundary(),&boundary));
        assert_eq!(face.projection(),projection);
        assert!(face.qualify_rail_incidence(1,1,100).unwrap().incidence.is_none());
        assert!(face.qualify_rail_incidence(100000000,100000,0).unwrap().incidence.is_none());
        let sewn=face.qualify_rail_incidence(100000000,100000,100).unwrap();
        let sewn=sewn.incidence.expect(sewn.reason);
        assert!(std::ptr::eq(sewn.face(),&face));
        let exterior=section_bindings.iter().map(|s|s.qualify_exterior(1000,1000).unwrap().region.unwrap()).collect::<Vec<_>>();
        assert!(sewn.qualify_sections([&exterior[0],&exterior[0]]).unwrap().patch.is_none());
        let other_exterior=other_sections.iter().map(|s|s.qualify_exterior(1000,1000).unwrap().region.unwrap()).collect::<Vec<_>>();
        assert!(sewn.qualify_sections([&other_exterior[0],&other_exterior[1]]).unwrap().patch.is_none());
        let patch=sewn.qualify_sections([&exterior[1],&exterior[0]]).unwrap();
        let patch=patch.patch.expect(patch.reason);
        assert!(std::ptr::eq(patch.rails(),&sewn));
        let retained_faces=exterior.iter().map(|e|e.qualify_face(1,16,256,2048,12288).unwrap().face.unwrap()).collect::<Vec<_>>();
        assert!(patch.qualify_outer_edges([&retained_faces[0],&retained_faces[0]],100000000,100000).unwrap().assembly.is_none());
        assert!(patch.qualify_outer_edges([&retained_faces[0],&retained_faces[1]],1,1).unwrap().assembly.is_none());
        let outer=patch.qualify_outer_edges([&retained_faces[1],&retained_faces[0]],100000000,100000).unwrap();
        eprintln!("outer retained edges work={} drivers={} reason={}",outer.work,outer.driver_cells,outer.reason);
        let outer=outer.assembly.expect(outer.reason);
        assert!(std::ptr::eq(outer.patch(),&patch));assert_eq!(outer.edges().len(),4);
        assert_eq!(outer.retained_factored(),[None,Some(1)]);
        assert!(outer.assemble_shell(0).unwrap().shell.is_none());
        let assembled=outer.assemble_shell(1000).unwrap();
        eprintln!("expression shell uses={} reason={}",assembled.uses,assembled.reason);
        let expression=assembled.shell.expect(assembled.reason);
        assert!(std::ptr::eq(expression.assembly(),&outer));
        assert_eq!(expression.faces().len(),body.geometry().shell().faces().len()+2);
        assert_eq!(expression.pairs().len(),body.geometry().shell().edges().len()+6);
        assert_eq!(expression.poles().len(),body.geometry().shell().poles().len());
        assert_eq!(expression.components(),1);
        assert!(outer.assemble_shell(assembled.uses-1).unwrap().shell.is_none());
        let exact_budget=outer.assemble_shell(assembled.uses).unwrap();
        assert!(exact_budget.shell.is_some());
        assert!(expression.pairs().iter().all(|p|p.reversed()[0]!=p.reversed()[1]));

        use crate::source_profile_body::ShellFaceKind;
        let profile=proposal.profile(0).unwrap();
        for (id,face) in expression.faces().iter().enumerate() {
            match face.kind() {
                ShellFaceKind::Original(index) => {
                    let surface=body.geometry().shell().faces()[index][0].edges()[0].surface();
                    let u=(surface.knots_u[surface.degree_u]+surface.knots_u[surface.control_points.len()])/2.;
                    let v=(surface.knots_v[surface.degree_v]+surface.knots_v[surface.control_points[0].len()])/2.;
                    let domain=[[u;2],[v;2]];
                    assert!(expression.enclose_support(id,domain,1000,0).unwrap().0.is_none());
                    let bounds=expression.enclose_support(id,domain,1000,100).unwrap().0.unwrap();
                    let sample=surface.evaluate(u,v).unwrap().point;
                    for k in 0..3 {assert!(sample[k]>=bounds[k][0]-1e-12 && sample[k]<=bounds[k][1]+1e-12);}
                }
                ShellFaceKind::Retained(end) => {
                    let parameter=if end==0 {0.5}else{0.95};
                    let domain=[[parameter;2],[0.5;2]];
                    assert_eq!(expression.enclose_support(id,domain,1000,100).unwrap(),profile.enclose(domain,100).unwrap());
                }
                ShellFaceKind::Transition => {
                    assert!(expression.enclose_support(id,[[0.5;2],[0.;2]],0,100).unwrap().0.is_none());
                    let entry=expression.enclose_support(id,[[0.5;2],[0.;2]],1000,100).unwrap();
                    assert_eq!(entry,profile.section(0,0.75).unwrap().enclose([0.5;2],100).unwrap());
                    assert!(expression.enclose_support(id,[[0.5;2],[0.5;2]],1000,100).unwrap().0.is_some());
                }
            }
        }
        assert!(expression.enclose_support(expression.faces().len(),[[0.;2];2],1000,100).is_err());

        for edge in outer.edges() {
            assert_ne!(edge.reversed()[0],edge.reversed()[1]);
            let restored=crate::source_shared_edge_restore::restore(edge.definition(),crate::source_shared_edge_restore::Limits{mapping_cells_per_use:100,exact_work:100000000,driver_cells:100000}).unwrap();
            let restored=restored.edge.expect(restored.reason);
            assert_eq!(restored.definition(),edge.definition());
        }
        for index in 0..2 {
            assert_eq!(patch.retained()[index].iter().map(|f|f.definition()).collect::<Vec<_>>(),exterior[index].kept_loop().iter().map(|f|f.definition()).collect::<Vec<_>>());
        }
        for rail in sewn.rails() {
            assert_ne!(rail.transition_reversed()^sewn.reverse_face(),rail.edge().reversed()[rail.neighbor_side()]);
            let contour=rail.neighbor_contour();let [wire,index]=contour.middle();
            assert_eq!(contour.loops()[wire][index].definition(),rail.edge().uses()[rail.neighbor_side()].definition());
        }
        let interior=admitted.interior.unwrap();
        assert!(interior.complete_regular);
        assert!(interior.pending.is_empty());
        let area:f64=interior.regular.iter().map(|r|(r[0][1]-r[0][0])*(r[1][1]-r[1][0])).sum();
        assert_eq!(area,1.);
        eprintln!("owned transition face accepted={} cells={} spans={}",interior.regular.len(),interior.cells,interior.spans);
        for edge in 0..4 {
            assert!(boundary.enclose_edge(edge,[0.,1.],1).unwrap().0.is_some());
            for (t,corner) in [(0.,edge),(1.,(edge+1)%4)] {
                let bounds=boundary.enclose_edge(edge,[t;2],1).unwrap().0.unwrap();
                let root=boundary.corners()[corner].world_bounds();
                for k in 0..3 {assert!(bounds[k][0]<=root[k][1]&&root[k][0]<=bounds[k][1]);}
                let next=boundary.enclose_edge((edge+1)%4,[0.;2],1).unwrap().0.unwrap();
                if t==1. {assert_eq!(bounds,next);}

            }
        }
        for canonical in [[0.5,0.5],[-0.1,0.5],[0.5,1.1],[f64::NAN,1.]] {
            assert!(proposal.qualify_edge_interval(0,canonical,1e-6,100,100).is_err());
        }
        assert!(proposal.qualify_edge_interval(usize::MAX,[0.,1.],1e-6,100,100).is_err());
    }
    #[test]
    fn corrected_seam_endpoint_preflight_preserves_work_refusals() {
        let model=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let shell=prepare(&model,limits()).unwrap().shell.unwrap();
        let geometry=crate::source_shell_geometry::qualify(shell,geometry_limits()).unwrap().geometry.unwrap();
        let body=crate::source_volume::qualify(geometry,crate::source_volume::Limits {
            axis:2,origin:0.,absolute_error:20.,tolerance_uv:1e-8,cells:100000,spans:100000,domain_cells:1000000,
        }).unwrap().body.unwrap();
        let specs=[crate::source_profile_body::FaceSpec{face:0,anchor:[5,0]},crate::source_profile_body::FaceSpec{face:5,anchor:[0,0]}];
        let report=crate::source_profile_body::prepare(&body,&specs,
            crate::source_profile_body::Limits{controls:1000,fragments:100,position_tolerance:1e-7}).unwrap();
        let proposal=report.proposal.unwrap();
        let (pole,spent)=proposal.profile(0).unwrap().pole_limits(true,1e-6,4).unwrap();
        let pole=pole.expect("exact corrected pole limit diagnostic");assert_eq!(spent,4);
        assert_eq!(pole.point,[20.,0.,6.]);assert!(pole.distinct_tangent_limits);
        eprintln!("corrected profile pole distinct={} sine={:?} normals={:?}",pole.distinct_tangent_limits,pole.sine_squared_interval,pole.components);
        let cut=proposal.profile(0).unwrap().section(0,0.75).unwrap();
        let transition=proposal.profile(0).unwrap().section(0,0.75).unwrap().end_transition([20.,0.,6.],0.25).unwrap();
        for i in 0..64 {
            let free=[i as f64/64.,(i+1) as f64/64.];
            assert_eq!(transition.regular_join(free,1).unwrap().0,Some(true));
            assert_eq!(transition.enclose(free,[0.,0.],1).unwrap(),cut.enclose(free,1).unwrap());
            assert!(transition.enclose(free,[0.,1.],2).unwrap().0.is_some());
        }
        for high in [false,true] {
            let rail=transition.rail(high);
            assert_eq!(rail.enclose([0.,0.],1).unwrap(),cut.enclose([rail.free_parameter();2],1).unwrap());
            assert_eq!(rail.enclose([1.,1.],0).unwrap().0.unwrap(),[[20.,20.],[0.,0.],[6.,6.]]);
            assert_eq!(rail.tangent_bounds([1.,1.],0).unwrap().0.unwrap(),[[0.,0.];3]);
            for i in 0..64 {
                let range=[i as f64/64.,(i+1) as f64/64.];
                assert!(rail.enclose(range,2).unwrap().0.is_some());
                assert!(rail.tangent_bounds(range,2).unwrap().0.is_some());
            }
        }
        let mut regular_cells=0;let mut pending_cells=Vec::new();
        for free_index in 0..8 {for across_index in 0..8 {
            let free=[free_index as f64/8.,(free_index+1) as f64/8.];
            let across=[across_index as f64/8.,(across_index+1) as f64/8.];
            let (bounds,spent)=transition.normal_bounds(free,across,4).unwrap();
            assert_eq!(spent,4);let bounds=bounds.unwrap();
            if bounds.iter().any(|r|r[0]>0.||r[1]<0.) {regular_cells+=1;} else {pending_cells.push([free,across]);}
        }}
        assert_eq!(regular_cells+pending_cells.len(),64);
        eprintln!("transition Jacobian regular={} pending={} domains={:?}",regular_cells,pending_cells.len(),pending_cells);
        assert!(transition.normal_bounds([0.2,0.3],[0.2,0.3],3).unwrap().0.is_none());
        let refined=transition.inspect_regularity([0.,1.],[0.,1.],1000,4000).unwrap();
        let area:f64=refined.regular.iter().chain(&refined.pending).map(|r|(r[0][1]-r[0][0])*(r[1][1]-r[1][0])).sum();
        assert_eq!(area,1.);assert!(!refined.complete_regular);
        assert!(refined.pending.iter().any(|r|r[1][1]==1.));
        eprintln!("adaptive transition regular={} pending={} cells={} spans={}",refined.regular.len(),refined.pending.len(),refined.cells,refined.spans);
        assert!(refined.regular.iter().all(|r|r[1][1]<1.));
        let pending_area:f64=refined.pending.iter().map(|r|(r[0][1]-r[0][0])*(r[1][1]-r[1][0])).sum();
        eprintln!("adaptive transition pendingArea={pending_area:.17}");
        let (tip,tip_spans)=transition.tip_limits(1e-6,8).unwrap();let tip=tip.unwrap();
        assert_eq!(tip_spans,8);assert_eq!(tip.point,[20.,0.,6.]);
        assert!(tip.distinct_tangent_limits);assert!(tip.sine_squared_interval.unwrap()[0]>0.999999);
        eprintln!("transition tip distinct={} sine={:?} normals={:?}",tip.distinct_tangent_limits,tip.sine_squared_interval,tip.components);
        assert!(transition.tip_limits(1e-6,7).unwrap().0.is_none());
        let bridge=nurbs_core::normal_alignment::ProfileSectionBridge::new(
            proposal.profile(0).unwrap().section(0,0.75).unwrap(),
            proposal.profile(0).unwrap().section(0,0.9).unwrap(),[0.15,0.15]).unwrap();
        for high in [false,true] {for i in 0..64 {
            let free=[i as f64/64.,(i+1) as f64/64.];
            assert_eq!(bridge.regular_join(high,free,1).unwrap().0,Some(true));
            assert_eq!(bridge.enclose(free,[if high {1.}else{0.};2],1).unwrap(),bridge.section(high).enclose(free,1).unwrap());
        }}
        for high in [false,true] {
            let rail=bridge.rail(high);let free=rail.free_parameter();
            for across in [0.,0.5,1.] {
                assert_eq!(rail.enclose([across;2],4).unwrap(),bridge.enclose([free;2],[across;2],4).unwrap());
                assert!(rail.tangent_bounds([across;2],8).unwrap().0.is_some());
            }
        }
        let boundary=bridge.rail(true).enclose([0.5;2],4).unwrap().0.unwrap();
        use nurbs_core::interval_eval::Interval as RailInterval;
        let x=RailInterval::new(boundary[0][0],boundary[0][1]).unwrap();
        let y=RailInterval::new(boundary[1][0],boundary[1][1]).unwrap();
        let residual=x.mul(x).unwrap().add(y.mul(y).unwrap()).unwrap().sub(RailInterval::point(400.)).unwrap();
        assert!(residual.hi<0.,"Hermite rail must not be admitted as the unchanged cylinder boundary");
        eprintln!("bridge cylinder rail midpoint squared-radius residual=[{},{}]",residual.lo,residual.hi);
        assert!(bridge.constrain_original_rails(1000).unwrap().0.is_none(),"rounded 0.15 scale is not the exact difference of section coordinates");
        let exact_width=0.9-0.75;
        let exact_bridge=nurbs_core::normal_alignment::ProfileSectionBridge::new(
            proposal.profile(0).unwrap().section(0,0.75).unwrap(),proposal.profile(0).unwrap().section(0,0.9).unwrap(),[exact_width;2]).unwrap();
        let constrained=exact_bridge.constrain_original_rails(1000).unwrap().0.unwrap();
        let corrected=constrained.enclose([1.,1.],[0.5;2],1).unwrap().0.unwrap();
        let x=RailInterval::new(corrected[0][0],corrected[0][1]).unwrap();let y=RailInterval::new(corrected[1][0],corrected[1][1]).unwrap();
        let residual=x.mul(x).unwrap().add(y.mul(y).unwrap()).unwrap().sub(RailInterval::point(400.)).unwrap();
        assert!(residual.lo.abs()<1e-8&&residual.hi.abs()<1e-8);
        eprintln!("constrained cylinder rail residual=[{},{}]",residual.lo,residual.hi);
        assert!(constrained.enclose([0.4,0.6],[0.4,0.6],14).unwrap().0.is_some());
        assert!(constrained.enclose([0.4,0.6],[0.4,0.6],13).unwrap().0.is_none());
        assert!(constrained.derivative_bounds([0.5;2],[0.5;2],36).unwrap().0.is_some());
        assert!(constrained.derivative_bounds([0.5;2],[0.5;2],35).unwrap().0.is_none());
        for high in [false,true] {
            let report=constrained.inspect_support_rail_normals(high,[0.5;2],1e-6,37).unwrap();
            assert_eq!(report.aligned,Some(true));
            if high {assert!(report.sine_squared_interval.unwrap()[0]>1e-10,"position-only correction still needs a tangent-field correction");}
            assert_eq!(constrained.inspect_support_rail_normals(high,[0.5;2],1e-6,36).unwrap().aligned,None);
            eprintln!("constrained rail high={} aligned={:?} sine={:?} spans={}",high,report.aligned,report.sine_squared_interval,report.spans);
        }
        for sample in [0.25,0.5,0.75] {
            let (normal,spans)=constrained.normals_with_transverse_correction([sample;2],[sample;2],61).unwrap();
            let normal=normal.unwrap();
            assert!(normal.iter().any(|r|r[0]>0.||r[1]<0.));
            eprintln!("full Hermite point={sample} normal={normal:?} spans={spans}");
        }
        for high in [false,true] {
            let join=constrained.inspect_transverse_corrected_side_join(high,[0.,1.],1000,1000).unwrap();
            assert!(join.complete_regular);
            eprintln!("full Hermite side={high} cells={} spans={}",join.cells,join.spans);
        }
        let full_cover=constrained.inspect_transverse_corrected_regularity([0.,1.],[0.,1.],1000,72000).unwrap();
        assert!(full_cover.complete_regular,"full corrected bridge regularity must cover the entire rectangle");
        let full_area:f64=full_cover.regular.iter().chain(&full_cover.pending).map(|r|(r[0][1]-r[0][0])*(r[1][1]-r[1][0])).sum();
        assert_eq!(full_area,1.);
        eprintln!("full Hermite regular={} pending={} cells={} spans={}",full_cover.regular.len(),full_cover.pending.len(),full_cover.cells,full_cover.spans);
        let mut bridge_regular=0;
        for i in 0..8 {for j in 0..8 {
            let n=bridge.normal_bounds([i as f64/8.,(i+1) as f64/8.],[j as f64/8.,(j+1) as f64/8.],8).unwrap().0.unwrap();
            if n.iter().any(|r|r[0]>0.||r[1]<0.) {bridge_regular+=1;}
        }}
        eprintln!("noncollapsed bridge coarse regular={bridge_regular} pending={}",64-bridge_regular);
        let cover=bridge.inspect_regularity([0.,1.],[0.,1.],1000,8000).unwrap();
        assert!(cover.complete_regular,"{} pending={}",cover.reason,cover.pending.len());
        let area:f64=cover.regular.iter().chain(&cover.pending).map(|r|(r[0][1]-r[0][0])*(r[1][1]-r[1][0])).sum();assert_eq!(area,1.);
        eprintln!("noncollapsed bridge adaptive regular={} pending={} cells={} spans={}",cover.regular.len(),cover.pending.len(),cover.cells,cover.spans);
        let jet=bridge.derivative_bounds([0.5,0.5],[0.5,0.5],8).unwrap().0.unwrap();
        let vectors=jet.map(|r|r.map(|b|b[0]*0.5+b[1]*0.5));
        let dot=|a:[f64;3],b:[f64;3]|a.iter().zip(b).map(|(x,y)|x*y).sum::<f64>();
        let aa=dot(vectors[0],vectors[0]);let bb=dot(vectors[1],vectors[1]);let ab=dot(vectors[0],vectors[1]);
        let determinant=aa*bb-ab*ab;assert!(determinant>0.);
        let projection=[std::array::from_fn(|k|(bb*vectors[0][k]-ab*vectors[1][k])/determinant),
            std::array::from_fn(|k|(aa*vectors[1][k]-ab*vectors[0][k])/determinant)];
        let injectivity=bridge.inspect_injectivity([0.,1.],[0.,1.],projection,4000,32000).unwrap();
        assert!(injectivity.complete_regular,"{} pending={}",injectivity.reason,injectivity.pending.len());
        let area:f64=injectivity.regular.iter().chain(&injectivity.pending).map(|r|(r[0][1]-r[0][0])*(r[1][1]-r[1][0])).sum();assert_eq!(area,1.);
        eprintln!("bridge injectivity qualified={} accepted={} pending={} cells={} spans={} projection={:?}",injectivity.complete_regular,injectivity.regular.len(),injectivity.pending.len(),injectivity.cells,injectivity.spans,projection);

        let full_injective=constrained.inspect_transverse_corrected_injectivity([0.,1.],[0.,1.],projection,1000,61000).unwrap();
        let full_injective_area:f64=full_injective.regular.iter().chain(&full_injective.pending).map(|r|(r[0][1]-r[0][0])*(r[1][1]-r[1][0])).sum();
        assert_eq!(full_injective_area,1.);
        assert!(full_injective.complete_regular,"full corrected transition injectivity must cover the entire rectangle");
        eprintln!("full Hermite injectivity={} accepted={} pending={} cells={} spans={}",full_injective.complete_regular,full_injective.regular.len(),full_injective.pending.len(),full_injective.cells,full_injective.spans);
        let middle=cut.enclose([0.5,0.5],1).unwrap().0.unwrap();
        assert!(middle.iter().zip(pole.point).any(|(r,p)|r[1]<p-0.01||r[0]>p+0.01));
        for i in 0..64 {
            let r=cut.support_normals([i as f64/64.,(i+1) as f64/64.],1).unwrap().0.unwrap();
            for bounds in [cut.tangent_bounds([i as f64/64.,(i+1) as f64/64.],1).unwrap().0.unwrap(),
                cut.transverse_bounds([i as f64/64.,(i+1) as f64/64.],1).unwrap().0.unwrap()] {
                assert!(bounds.iter().any(|r|r[0]>0.||r[1]<0.));
            }
            assert!(r.iter().any(|r|r[0]>0.||r[1]<0.),"cut support normal unresolved at {i}: {r:?}");
        }
        eprintln!("corrected pole cut U=0.75 midpoint={:?} regularIntervals=64",middle);
        let low=proposal.cut_endpoint(0,0,0.75,false,0,16).unwrap().endpoint.unwrap();
        let high=proposal.cut_endpoint(0,0,0.75,true,1,16).unwrap().endpoint.unwrap();
        let low_partition=low.split_source_use(100).unwrap().partition.unwrap();
        let high_partition=high.split_source_use(100).unwrap().partition.unwrap();
        let contours=crate::source_profile_body::propose_cut_contours(&low_partition,&high_partition,100).unwrap();
        let contour_cost=contours.fragments;
        let contours=contours.proposal.expect(contours.reason);
        for wire in contours.loops() {
            for i in 0..wire.len() {assert!(wire[i].joins(&wire[(i+1)%wire.len()]));}
            let restored=wire.iter().map(|f|crate::source_boundary_fragment::restore(f.definition(),100).unwrap()).collect::<Vec<_>>();
            for i in 0..wire.len() {assert!(restored[i].joins(&restored[(i+1)%wire.len()]));}
        }
        assert!(crate::source_profile_body::propose_cut_contours(&low_partition,&high_partition,contour_cost-1).unwrap().proposal.is_none());
        assert!(crate::source_profile_body::propose_cut_contours(&low_partition,&low_partition,100).unwrap().proposal.is_none());
        let different=proposal.cut_endpoint(0,0,0.7,true,1,16).unwrap().endpoint.unwrap();
        let different_partition=different.split_source_use(100).unwrap().partition.unwrap();
        assert!(crate::source_profile_body::propose_cut_contours(&low_partition,&different_partition,100).unwrap().proposal.is_none());
        for (high,index,neighbor) in [(false,0,1),(true,1,2)] {
            let report=proposal.cut_endpoint(0,0,0.75,high,index,16).unwrap();
            let endpoint=report.endpoint.expect(report.reason);
            assert!(std::ptr::eq(endpoint.proposal(),&proposal));assert_eq!(endpoint.faces(),[0,neighbor]);
            assert_eq!(endpoint.edge(),index);assert_eq!(endpoint.definition()["parameter"],value_codec::json!(0.75));
            let own=cut.enclose([if high {1.}else{0.};2],1).unwrap().0.unwrap();
            let world=endpoint.world_bounds();
            for k in 0..3 {assert!(own[k][0]<=world[k][1]&&world[k][0]<=own[k][1]);}
            eprintln!("owned cut endpoint edge={} canonical={:?} neighborUV={:?} world={:?} spans={}",index,
                endpoint.canonical_bounds(),endpoint.neighbor_uv_bounds(),world,report.spans);
            let split=endpoint.split_source_use(16).unwrap();
            let partition=split.partition.expect(split.reason);
            assert!(std::ptr::eq(partition.endpoint(),&endpoint));
            let fragments=partition.selected_fragments();assert!(fragments[0].joins(&fragments[1]));
            assert_eq!(fragments[0].curve(),fragments[1].curve());
            let restored=fragments.each_ref().map(|f|crate::source_boundary_fragment::restore(f.definition(),16).unwrap());
            assert!(restored[0].joins(&restored[1]));
            for i in 0..2 {assert_eq!(restored[i].definition(),fragments[i].definition());}

            let neighbor=partition.neighbor_ranges();assert_eq!(neighbor[0][1],neighbor[1][0]);
            let point=partition.point();assert_eq!(point.surface(),fragments[0].surface());
            assert!(matches!(&fragments[0].endpoints()[1],crate::source_boundary_fragment::Endpoint::Crossing{..}));
            assert!(matches!(&fragments[1].endpoints()[0],crate::source_boundary_fragment::Endpoint::Crossing{..}));
            assert!(endpoint.split_source_use(0).is_err());
            let original=&body.geometry().shell().edges()[index];
            let addresses=&body.geometry().shell().uses()[index];
            let side=usize::from(addresses[0].face!=0);
            let transported=crate::source_carrier_point::qualify(original,point,side,3).unwrap();
            let carrier=transported.point.expect(transported.reason);assert_eq!(transported.spans,3);
            assert_eq!(carrier.root().definition(),point.definition());
            for use_side in 0..2 {
                let parts=carrier.retained_parts(use_side).unwrap();
                let fragments=original.uses()[use_side].split_at_carrier(&carrier,use_side).unwrap();
                assert!(fragments[0].joins(&fragments[1]));
                for fragment in &fragments {
                    let restored=crate::source_boundary_fragment::restore(fragment.definition(),100).unwrap();
                    assert_eq!(restored.definition(),fragment.definition());
                    assert_eq!(restored.parameter_bounds(),fragment.parameter_bounds());
                    assert!(crate::source_boundary_fragment::restore(fragment.definition(),0).is_err());
                    let mut foreign=fragment.definition();
                    for end in foreign["endpoints"].as_array_mut().unwrap() {
                        if end["kind"].as_str()==Some("carrier") {
                            end["side"]=value_codec::json!(1-use_side);
                        }
                    }
                    assert!(crate::source_boundary_fragment::restore(foreign,100).is_err());
                }
                let mut nested=carrier.definition();
                nested["edge"]["uses"][0]["endpoints"][0]["kind"]=value_codec::json!("carrier");
                assert!(crate::source_carrier_point::restore(nested,crate::source_carrier_point::RestoreLimits {
                    edge:crate::source_shared_edge_restore::Limits {mapping_cells_per_use:100,exact_work:1000000,driver_cells:100},
                    root_mapping_cells:100,spans:100 }).is_err());
                assert!(parts[0].joins(&parts[1]));
                assert!(!parts[1].joins(&parts[0]));
                assert_eq!(parts[0].parameter_bounds()[1],parts[1].parameter_bounds()[0]);
                assert_eq!(parts[0].source().definition(),original.uses()[use_side].definition());
                assert_eq!(parts[1].parameter_bounds()[1],original.uses()[use_side].parameter_bounds()[1]);
                assert!(!parts[0].enclose(0).unwrap().complete);
                assert!(parts[0].enclose(100).unwrap().complete);
                assert!(parts[1].enclose(100).unwrap().complete);
                let foreign_parts=carrier.retained_parts(1-use_side).unwrap();
                assert!(!parts[0].joins(&foreign_parts[1]));
            }
            let transported_partition=partition.qualify_transported_neighbor(3,100000000,100000).unwrap();
            assert_eq!(transported_partition.carrier_spans,3);
            assert!(transported_partition.partition.is_some(),"{}",transported_partition.reason);
            assert!(partition.qualify_transported_neighbor(2,100000000,100000).unwrap().partition.is_none());
            assert!(partition.qualify_transported_neighbor(3,1,100).unwrap().partition.is_none());
            let neighbor_fragments=original.uses()[1-side].split_at_carrier(&carrier,1-side).unwrap();
            let selected_fragments=partition.selected_fragments();
            let carrier_selected=original.uses()[side].split_at_carrier(&carrier,side).unwrap();
            let maps=crate::source_seam_tangency::ranges(original);
            for half in 0..2 {
                let pair=[&selected_fragments[half],&neighbor_fragments[1-half]];
                let ranges=[maps[side],maps[1-side]];
                let report=crate::source_mapped_edge::qualify(original.world(),pair,ranges,[None,None],100000000,100000).unwrap();
                let admitted=report.edge.expect(report.reason);
                assert!(!admitted.covers_complete_canonical_source());
                let restored=crate::source_shared_edge_restore::restore(admitted.definition(),crate::source_shared_edge_restore::Limits {
                    mapping_cells_per_use:100,exact_work:100000000,driver_cells:100000 }).unwrap();
                assert_eq!(restored.edge.expect(restored.reason).definition(),admitted.definition());
                let endpoint_index=if half==0 {1}else{0};
                let ends=[&pair[0].endpoints()[endpoint_index],&pair[1].endpoints()[1-endpoint_index]];
                assert!(crate::source_carrier_point::same_mapped_endpoint(original.world(),pair,ranges,ends));
                let both_pair=[&carrier_selected[half],&neighbor_fragments[1-half]];
                let both_ends=[&both_pair[0].endpoints()[endpoint_index],&both_pair[1].endpoints()[1-endpoint_index]];
                assert!(crate::source_carrier_point::same_mapped_endpoint(original.world(),both_pair,ranges,both_ends));
                assert!(!crate::source_carrier_point::same_mapped_endpoint(original.world(),both_pair,ranges,[both_ends[0],both_ends[0]]));
                assert!(crate::source_mapped_edge::qualify(original.world(),both_pair,ranges,[None,None],100000000,100000).unwrap().edge.is_some());
                let mut changed=original.world().clone();changed.control_points[0][0]+=0.01;
                assert!(!crate::source_carrier_point::same_mapped_endpoint(&changed,pair,ranges,ends));
                let mut changed_ranges=ranges;changed_ranges[0][0][0]+=0.01;
                assert!(!crate::source_carrier_point::same_mapped_endpoint(original.world(),pair,changed_ranges,ends));
                assert!(crate::source_mapped_edge::qualify(original.world(),pair,ranges,[None,None],1,100).unwrap().edge.is_none());
            }
            assert!(carrier.retained_parts(2).is_err());
            let b=carrier.parameter_bounds()[1-side];assert!(b[0]<if high {0.75}else{0.25}&&b[1]>if high {0.75}else{0.25});
            assert!(crate::source_carrier_point::qualify(original,point,side,2).unwrap().point.is_none());
            assert!(crate::source_carrier_point::qualify(original,point,1-side,3).unwrap().point.is_none());
            let restore_limits=||crate::source_carrier_point::RestoreLimits {
                edge:crate::source_shared_edge_restore::Limits{mapping_cells_per_use:16,exact_work:100000000,driver_cells:10000},
                root_mapping_cells:16,spans:3};
            let mut definition=carrier.definition();definition["canonical"]=value_codec::json!([0.,0.]);
            let restored=crate::source_carrier_point::restore(definition,restore_limits()).unwrap();
            let restored=restored.point.expect(restored.reason);
            assert_eq!(restored.definition(),carrier.definition());
            let mut foreign=carrier.definition();foreign["sourceSide"]=value_codec::json!(1-side);
            assert!(crate::source_carrier_point::restore(foreign,restore_limits()).unwrap().point.is_none());
            eprintln!("carrier root edge={} side={} target={:?} canonical={:?} spans=3",index,side,b,carrier.canonical_bounds());

            if high {
                let cutter=nurbs_core::curve::Curve::from_polyline(vec![vec![0.75,-1.],vec![0.75,2.]]).unwrap();
                let joined=partition.qualify_neighbor(&cutter,[[0.7,0.8],[0.61,0.72]],16,100000000,10000).unwrap();
                eprintln!("neighbor cut edge={} reason={} work={} roots={}",index,joined.reason,joined.exact_work,joined.root_checks);
                let halves=joined.partition.expect(joined.reason);
                assert!(halves.fragments()[0].joins(&halves.fragments()[1]));
                assert!(halves.edges().iter().all(|e|!e.covers_complete_canonical_source()));
                for half in halves.edges() {
                    let restored=crate::source_shared_edge_restore::restore(half.definition(),
                        crate::source_shared_edge_restore::Limits{mapping_cells_per_use:16,exact_work:100000000,driver_cells:10000}).unwrap();
                    let restored=restored.edge.expect(restored.reason);
                    assert_eq!(restored.definition(),half.definition());
                    assert!(!restored.covers_complete_canonical_source());
                }

                let wrong=nurbs_core::curve::Curve::from_polyline(vec![vec![0.8,-1.],vec![0.8,2.]]).unwrap();
                assert!(partition.qualify_neighbor(&wrong,[[0.7,0.8],[0.61,0.72]],16,100000000,10000).unwrap().partition.is_none());
                assert!(partition.qualify_neighbor(&cutter,[[0.7,0.8],[0.61,0.72]],16,1,1).unwrap().partition.is_none());
            }

            eprintln!("owned cut partition edge={} roots={:?} neighborRanges={:?} cells={}",index,
                point.root_parameters(),neighbor,split.mapping_cells);
            let short=proposal.cut_endpoint(0,0,0.75,high,index,1).unwrap();
            assert!(short.endpoint.is_none());assert_eq!(short.reason,"profile-cut-work-limit");
        }
        assert!(proposal.cut_endpoint(0,0,0.75,false,5,16).unwrap().endpoint.is_none());
        assert!(proposal.cut_endpoint(0,2,0.75,false,0,16).is_err());

        let report=proposal.qualify_edge(1,100,1e-6,2,4).unwrap();
        assert!(report.seam.is_none());assert_eq!(report.reason,"profile-seam-endpoint-normal-unresolved");
        let r=report.angular.unwrap();assert_eq!((r.cells,r.spans,r.accepted_cells),(2,4,1));
        assert_eq!(r.pending,vec![[0.,1.],[1.,1.]]);
        let report=proposal.qualify_edge(1,100,1e-6,1,1).unwrap();
        assert!(report.seam.is_none());assert_eq!(report.reason,"profile-path-work-limit");
        assert!(!report.angular.unwrap().pending.is_empty());
    }
    #[test]
    fn admitted_annular_body_checks_original_radius_patches() {
        let model=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let shell=prepare(&model,limits()).unwrap().shell.unwrap();
        let geometry=crate::source_shell_geometry::qualify(shell,geometry_limits()).unwrap().geometry.unwrap();
        let body=crate::source_volume::qualify(geometry,crate::source_volume::Limits {
            axis:2,origin:0.,absolute_error:20.,tolerance_uv:1e-8,cells:100000,spans:100000,domain_cells:1000000,
        }).unwrap().body.unwrap();
        if let Some(path)=std::env::var_os("CAD_ANNULAR_SOURCE_BODY_OUTPUT") {
            std::fs::write(path,value_codec::to_string_pretty(&body.definition().unwrap()).unwrap()).unwrap();
        }
        let combined=crate::source_wall_pair::qualify(&body,[4,5],5.99,
            crate::source_wall_pair::Limits{normal_spans:10000,max_sine_squared:1e-6,
                cells:4096,spans:10000,domain_cells:10000,domain_cells_per_query:256,tolerance_uv:1e-8}).unwrap();
        assert!(combined.normal_products>0);assert!(combined.spans>0);
        assert!(combined.unresolved_products.is_empty());
        let mixed=combined.certificate.unwrap();assert!(std::ptr::eq(mixed.body(),&body));
        let aggregate=crate::source_wall_coverage::qualify_with_mixed_pair_facts(&body,5.99,
            crate::source_wall_coverage::Limits{pairs:378,plane_controls:10000,normal_spans:1000,
                gap:crate::source_face_gap::Limits{cells:1000,spans:2000},max_sine_squared:1e-6},
            Vec::new(),crate::source_wall_coverage::NormalProductLimits{cells:0,cells_per_pair:0},
            crate::source_wall_coverage::FactoredNormalLimits{exact_work:0,queries:0,cells:0,cells_per_pair:0},
            Vec::new(),None,vec![mixed]).unwrap();
        let pair=aggregate.pairs.iter().find(|p|p.faces==[4,5]).unwrap();
        assert!(pair.proven);assert_eq!(pair.reason,"source-wall-mixed-product-qualified");
        assert!(aggregate.certificate.is_none());

        for faces in [[0,7],[2,5],[5,12],[7,10]] {
            let end=crate::source_wall_pair::qualify(&body,faces,5.99,
                crate::source_wall_pair::Limits{normal_spans:10000,max_sine_squared:1e-6,
                    cells:4096,spans:10000,domain_cells:10000,domain_cells_per_query:256,tolerance_uv:1e-8}).unwrap();
            assert!(end.unresolved_products.is_empty(),"original corner product {faces:?}");
            assert!(end.spans<=10000);assert!(end.normal_spans<=10000);
            let proof=end.certificate.expect("continuous source-bound corner exclusion");
            assert!(std::ptr::eq(proof.body(),&body));assert_eq!(proof.faces(),faces);
            assert_eq!(proof.lower_mm(),5.99);
        }
        let shared=crate::source_shared_chord_map::prepare(&body,[0,2],100000).unwrap().unwrap();
        assert!(std::ptr::eq(shared.body(),&body));assert_eq!(shared.faces(),[0,2]);
        assert_eq!(shared.fixed_axes(),[1,1]);assert_eq!(shared.fixed_values(),[1.,1.]);
        assert_eq!(shared.free_endpoints(),[[0.,1.],[3.,0.]]);
        let source=shared.surfaces()[0];
        let (profile,profile_work)=nurbs_core::normal_alignment::qualify_weight_profile(source,[5,0],18).unwrap();
        let profile=profile.unwrap();assert_eq!(profile_work,18);assert!(std::ptr::eq(profile.source(),source));
        assert!(profile.within_position_tolerance(1e-7).unwrap());
        let (pole_box,pole_spans)=profile.enclose([[1.,1.],[0.5,0.5]],16).unwrap();
        assert!(pole_spans>0);let pole_box=pole_box.unwrap();
        for (interval,position) in pole_box.into_iter().zip([20.,0.,6.]) {
            assert!(interval[0]<=position&&position<=interval[1]&&interval[1]-interval[0]<1e-10);
        }

        eprintln!("annular original exact weight-profile proposal axis upper {:?} weight upper {}",profile.axis_deviation_upper(),profile.weight_deviation_upper());
        assert!(nurbs_core::normal_alignment::qualify_weight_profile(source,[5,0],17).unwrap().0.is_none());
        let (profile_direction,direction_spans)=profile.boundary_direction([[1.,1.],[0.5,0.5]],1,1.,16).unwrap();
        let profile_direction=profile_direction.unwrap();assert_eq!(direction_spans,1);
        assert!(std::ptr::eq(profile_direction.profile(),&profile));assert_eq!(profile_direction.factor(),Some((true,2)));
        assert_eq!(profile_direction.boundary(),1.);
        assert!(profile_direction.components().iter().any(|r|r[0]>0.||r[1]<0.));
        eprintln!("annular exact profile pole direction {:?}",profile_direction.components());
        let proposal_limits=||crate::source_profile_body::Limits{controls:1000,fragments:100,position_tolerance:1e-7};
        let specs=[crate::source_profile_body::FaceSpec{face:0,anchor:[5,0]}];
        let report=crate::source_profile_body::prepare(&body,&specs,proposal_limits()).unwrap();
        let proposal=report.proposal.expect(report.reason);assert!(std::ptr::eq(proposal.original_body(),&body));
        assert!(proposal.profile(0).is_some()&&proposal.profile(1).is_none());
        assert_eq!(proposal.specs()[0].anchor,[5,0]);
        let definition=proposal.definition().unwrap();assert_eq!(definition["originalBody"],body.definition().unwrap());
        assert_eq!(definition["profiles"][0]["face"],value_codec::json!(0));
        eprintln!("annular profile body proposal {} controls={} fragments={}",report.reason,report.controls,report.fragments);
        let mut short=proposal_limits();short.controls=17;
        assert!(crate::source_profile_body::prepare(&body,&specs,short).unwrap().proposal.is_none());
        assert!(crate::source_profile_body::prepare(&body,&[specs[0],specs[0]],proposal_limits()).is_err());
        assert_eq!((report.controls,report.fragments),(44,4));
        for missing in [0,1,3] {
            let mut limits=proposal_limits();limits.fragments=missing;
            let refused=crate::source_profile_body::prepare(&body,&specs,limits).unwrap();
            assert!(refused.proposal.is_none());
            assert_eq!(refused.fragments,missing);
            assert_eq!(refused.reason,"profile-body-work-limit");
        }
        let neighbor=body.geometry().shell().regions().unwrap()[5].loops()[0][0].surface();
        let profile=proposal.profile(0).unwrap();
        let seam=profile.inspect_original_normals(neighbor,
            [[[0.,0.],[0.49,0.51]],[[1.,1.],[0.49,0.51]]],1e-6,16).unwrap();
        eprintln!("corrected profile middle seam aligned={:?} sine={:?} reason={}",seam.aligned,seam.sine_squared_interval,seam.reason);
        assert!(seam.normal_components.iter().all(|n|n.is_some()));
        assert_eq!(seam.aligned,None);
        let point=profile.inspect_original_normals(neighbor,
            [[[0.,0.],[0.5,0.5]],[[1.,1.],[0.5,0.5]]],1e-6,16).unwrap();
        eprintln!("corrected profile seam midpoint aligned={:?} sine={:?}",point.aligned,point.sine_squared_interval);
        assert_eq!(point.aligned,Some(true));
        let owned=proposal.qualify_seam(0,5,100,1e-6,50000,100000).unwrap();
        let certificate=owned.seam.as_ref().expect(owned.reason);
        assert!(std::ptr::eq(certificate.proposal(),&proposal));
        assert_eq!(certificate.faces(),[0,5]);assert_eq!(certificate.edge(),5);
        let whole=owned.angular.unwrap();
        eprintln!("owned corrected whole seam aligned={:?} upper={:?} cells={} spans={} accepted={} pending={} reason={}",
            whole.aligned,whole.upper_sine_squared,whole.cells,whole.spans,whole.accepted_cells,whole.pending.len(),whole.reason);
        assert_eq!(whole.aligned,Some(true));assert!(whole.pending.is_empty());
        assert!(certificate.upper_sine_squared()<=certificate.threshold());
        assert_eq!(whole.cells,14699);assert_eq!(whole.spans,29398);
        let both_specs=[specs[0],crate::source_profile_body::FaceSpec{face:5,anchor:[0,0]}];
        let both_report=crate::source_profile_body::prepare(&body,&both_specs,proposal_limits()).unwrap();
        let both=both_report.proposal.expect(both_report.reason);
        let both_seam=both.qualify_edge(5,100,1e-6,50000,100000).unwrap();
        let proof=both_seam.seam.expect(both_seam.reason);
        assert!(std::ptr::eq(proof.proposal(),&both));assert_eq!(proof.edge(),5);
        let work=both_seam.angular.unwrap();assert!(work.pending.is_empty());
        eprintln!("two-profile owned seam cells={} spans={} accepted={} upper={:?}",work.cells,work.spans,work.accepted_cells,work.upper_sine_squared);
        assert!(crate::source_shared_chord_map::prepare_edge(&body,[0,5],1,100).unwrap().is_none());
        assert!(crate::source_shared_chord_map::prepare_edge(&body,[0,5],5,5).unwrap().is_none());
        assert!(both.qualify_edge(usize::MAX,100,1e-6,1,2).is_err());
        for (index,uses) in body.geometry().shell().uses().iter().enumerate() {
            if uses.iter().any(|u|both.profile(u.face).is_some()) {
                let audit=both.qualify_edge(index,100,1e-6,1,2).unwrap();
                assert!(audit.seam.is_none());
                eprintln!("incident corrected edge {} faces={:?} reason={} pending={:?}",index,
                    [uses[0].face,uses[1].face],audit.reason,audit.angular.map(|r|r.pending));
            }
        }
        for index in [1,8,15] {
            let report=both.qualify_edge(index,100,1e-6,50000,100000).unwrap();
            let angular=report.angular.expect("owned seam route");
            if index==1 {
                assert!(report.seam.is_none());assert_eq!(angular.aligned,None);
                assert_eq!(report.reason,"profile-seam-endpoint-normal-unresolved");
                assert!(angular.cells<=2);assert!(!angular.pending.is_empty());
            } else {assert!(report.seam.is_some());assert_eq!(angular.aligned,Some(true));}

            eprintln!("remaining corrected edge {} aligned={:?} cells={} spans={} accepted={} pending={} reason={} upper={:?}",
                index,angular.aligned,angular.cells,angular.spans,angular.accepted_cells,angular.pending.len(),report.reason,angular.upper_sine_squared);
            if let Some(proof)=report.seam {
                assert_eq!(proof.edge(),index);assert!(std::ptr::eq(proof.proposal(),&both));
                assert!(angular.pending.is_empty());assert!(proof.upper_sine_squared()<=1e-6);
            } else {assert!(!angular.pending.is_empty());}
        }
        for index in [0,6] {
            let report=both.qualify_edge(index,100,1e-6,10000,40000).unwrap();
            let angular=report.angular.expect("original pcurve route");
            if index==0 {
                assert!(report.seam.is_none());assert_eq!(angular.aligned,None);
                assert_eq!(report.reason,"profile-seam-endpoint-normal-unresolved");
                assert!(!angular.pending.is_empty());
            } else {
                let proof=report.seam.expect(report.reason);assert_eq!(proof.edge(),6);
                assert!(std::ptr::eq(proof.proposal(),&both));
                assert_eq!(angular.aligned,Some(true));assert!(angular.pending.is_empty());
                assert!(proof.upper_sine_squared()<=1e-6);
                assert_eq!((angular.cells,angular.spans,angular.accepted_cells),(291,1164,147));
            }

            eprintln!("curved corrected edge {} aligned={:?} cells={} spans={} accepted={} pending={} reason={} upper={:?}",
                index,angular.aligned,angular.cells,angular.spans,angular.accepted_cells,angular.pending.len(),report.reason,angular.upper_sine_squared);
        }
        let limited=proposal.qualify_seam(0,5,0,1e-6,50000,100000).unwrap();
        assert!(limited.seam.is_none()&&limited.angular.is_none());
        let limited=proposal.qualify_seam(0,5,100,1e-6,1,1).unwrap();
        assert!(limited.seam.is_none());assert!(!limited.angular.unwrap().pending.is_empty());
        let mut exact=proposal_limits();exact.controls=44;exact.fragments=4;
        assert!(crate::source_profile_body::prepare(&body,&specs,exact).unwrap().proposal.is_some());
        let mut missing=proposal_limits();missing.controls=43;
        let refused=crate::source_profile_body::prepare(&body,&specs,missing).unwrap();
        assert!(refused.proposal.is_none());assert_eq!(refused.controls,43);




        let first=shared.canonical_bounds(0,[[0.25,0.5],[0.,1.]]).unwrap();
        let second=shared.canonical_bounds(1,[[1.5,2.25],[0.,1.]]).unwrap();
        assert!(first[0]<=0.25&&first[1]>=0.5);assert!(second[0]<=0.25&&second[1]>=0.5);
        assert!(shared.canonical_bounds(0,[[-0.01,0.5],[0.,1.]]).is_err());
        let (tangent,spans)=shared.tangent_bounds([[[0.25,0.5],[0.,1.]],[[1.5,2.25],[0.,1.]]],64).unwrap();
        assert!(spans>0);assert!(tangent.unwrap().iter().all(|r|r[0].is_finite()&&r[1].is_finite()&&r[0]<=r[1]));
        let shared_rects=[[[0.25,0.5],[0.8,1.]],[[1.5,2.25],[0.8,1.]]];
        let (_,shared_spans,shared_normals)=shared.exclude_normal_chords(shared_rects,1e-6,64,64).unwrap();
        assert!(shared_spans>0&&shared_spans<=64&&shared_normals<=64);
        assert_eq!(shared.exclude_normal_chords(shared_rects,1e-6,0,64).unwrap(),(false,0,0));
        assert_eq!(shared.exclude_normal_chords(shared_rects,1e-6,64,0).unwrap(),(false,0,0));
        assert!(shared.exclude_normal_chords(shared_rects,f64::NAN,64,64).is_err());
        let narrow=[[[0.4,0.400000001],[0.999999999,1.]],[[1.799999997,1.8],[0.999999999,1.]]];
        let (excluded,used,normal_used)=shared.exclude_normal_chords(narrow,1e-6,64,64).unwrap();
        assert!(excluded,"shared rolling boundary exclusion used {used}/{normal_used}");
        assert!(!shared.exclude_normal_chords(narrow,1e-6,1,64).unwrap().0);
        assert!(shared.exclude_normal_chords_with_mean(narrow,1e-6,64,64).unwrap().0);
        assert!(!shared.exclude_normal_chords_with_mean(narrow,1e-6,8,64).unwrap().0);
        assert!(shared.exclude_normal_chords_with_secant(narrow,1e-6,64,64).unwrap().0);
        assert!(shared.exclude_normal_chords_with_factored_secant(narrow,1e-6,64,64).unwrap().0);




        assert!(crate::source_shared_chord_map::prepare(&body,[0,2],0).unwrap().is_none());
        let reverse=crate::source_shared_chord_map::prepare(&body,[2,0],100000).unwrap().unwrap();
        assert_eq!(reverse.edge(),shared.edge());assert_eq!(reverse.free_endpoints(),[[3.,0.],[0.,1.]]);
        let prepared=crate::source_wall_anchor::prepare(&body,[0,1],[[1.,0.],[20.,0.]],[20.,0.,6.],10000000).unwrap();
        assert!(prepared.prepared.is_some());
        assert!(prepared.exact_work>0 && prepared.exact_work<=10000000);
        let automatic=crate::source_wall_anchor::prepare_boundary_anchor(&body,[0,1],2000000).unwrap();
        assert!(automatic.prepared.is_some());assert!(automatic.exact_work<=2000000);
        assert!(crate::source_wall_anchor::prepare_boundary_anchor(&body,[0,1],0).unwrap().prepared.is_none());
        let paths=prepared.prepared.unwrap();
        let product=[[[0.9999999925494194,1.],[0.,7.450580596923828e-9]],
            [[19.999999990686774,20.],[-9.313225746154785e-9,0.]]];
        let (proof,used)=paths.qualify_product(product,[-1.,-1.,-1.],1e-6,16).unwrap();
        assert!(proof.is_some(),"charged product spans {used}");
        assert!(used<=16);
        let projections=[[0.,0.,0.],[-1.,-1.,-1.]];
        let (batch,batch_spans)=paths.qualify_product_projections(product,&projections,1e-6,16).unwrap();
        assert!(batch.is_some());assert!(batch_spans<=16);
        assert!(paths.qualify_product_projections(product,&[],1e-6,16).is_err());
        assert!(paths.qualify_product_projections(product,&[[-1.,-1.,-1.],[f64::NAN,0.,0.]],1e-6,16).is_err());
        assert!(paths.qualify_product_projections(product,&[[1.,1.,1.];27],1e-6,16).is_err());
        assert!(paths.qualify_product_projections(product,&projections,1e-6,1).unwrap().0.is_none());

        let limits=||crate::source_wall_pair::Limits{normal_spans:100,max_sine_squared:1e-6,
            cells:1,spans:100,domain_cells:100,domain_cells_per_query:16,tolerance_uv:1e-8};
        assert!(crate::source_wall_pair::qualify_with_anchors(&body,[1,0],5.99,limits(),&paths).is_err());
        let partial=crate::source_wall_pair::qualify_with_anchors(&body,[0,1],5.99,limits(),&paths).unwrap();
        assert!(partial.certificate.is_none());assert!(!partial.unresolved_products.is_empty());

        assert!(paths.qualify_product(product,[-1.,-1.,-1.],1e-6,1).unwrap().0.is_none());

        assert!(crate::source_wall_anchor::prepare(&body,[0,1],[[1.,0.],[20.,0.]],[20.,0.,6.],0).unwrap().prepared.is_none());

        let numerical=crate::source_wall_pair::qualify(&body,[16,20],5.99,
            crate::source_wall_pair::Limits{normal_spans:10000,max_sine_squared:1e-6,
                cells:4096,spans:10000,domain_cells:10000,domain_cells_per_query:256,tolerance_uv:1e-8}).unwrap();
        assert!(numerical.certificate.is_none());
        // The UV stop now avoids the former underflow path; work remains unproven.
        assert_eq!(numerical.reason,"source-wall-pair-work-limit");
        assert_eq!(numerical.cells,4096);
        assert!(!numerical.unresolved_products.is_empty());assert!(numerical.normal_spans<=10000);
        let shell=body.geometry().shell();
        let flat_at=|z:f64|shell.faces().iter().enumerate().filter_map(|(i,w)| {
            let s=w[0].edges()[0].surface();
            s.control_points.iter().flatten().all(|p|p[2]==z).then_some(i)
        }).collect::<Vec<_>>();
        let bottom=flat_at(0.);let top=flat_at(6.);
        assert!(!bottom.is_empty()&&!top.is_empty());
        let gap=crate::source_face_gap::qualify(&body,[&bottom,&top],5.99,
            crate::source_face_gap::Limits{cells:1000,spans:2000}).unwrap();
        eprintln!("original annular flat face gap {} cells={} spans={} bottom={bottom:?} top={top:?}",gap.reason,gap.cells,gap.spans);
        let proof=gap.certificate.expect("all original flat top/bottom pairs need clearance");
        assert!(proof.lower_mm()>=5.99&&proof.lower_mm()<=6.);
        assert_eq!(proof.faces(),[bottom.as_slice(),top.as_slice()]);
        assert!(std::ptr::eq(proof.body(),&body));
        let too_large=crate::source_face_gap::qualify(&body,[&bottom,&top],7.,
            crate::source_face_gap::Limits{cells:8,spans:16}).unwrap();
        assert!(too_large.certificate.is_none());
        assert!(too_large.uncertain_faces.is_some()&&too_large.uncertain_uv.is_some());
        assert!(too_large.cells<=8&&too_large.spans<=16);
        assert!(crate::source_face_gap::qualify(&body,[&bottom,&top],5.99,
            crate::source_face_gap::Limits{cells:1,spans:1}).unwrap().certificate.is_none());
        assert!(crate::source_face_gap::qualify(&body,[&bottom,&bottom],5.99,
            crate::source_face_gap::Limits{cells:1000,spans:2000}).is_err());
        let curved_gap=crate::source_face_gap::qualify(&body,
            [&[2,7,12,16,20,24],&[3,8,13,17,21,25]],14.5,
            crate::source_face_gap::Limits{cells:10000,spans:20000}).unwrap();
        eprintln!("original annular curved gap {} cells={} spans={} uncertain={:?}",curved_gap.reason,curved_gap.cells,curved_gap.spans,curved_gap.uncertain_faces);
        let curved_proof=curved_gap.certificate.expect("all original cylindrical inner/outer pairs need clearance");
        assert!(curved_proof.lower_mm()>=14.5&&curved_proof.lower_mm()<=15.);
        assert!(curved_gap.cells>=36&&curved_gap.cells<=10000&&curved_gap.spans<=20000);
        let outside_segment=crate::source_material_segment::inspect_boundary(
            &body,[30.,30.,1.],[0.,0.,1.],1e-7,10000,10000).unwrap();
        assert!(outside_segment.boundary_free);
        assert!(outside_segment.contacts.is_empty() && outside_segment.unresolved.is_empty());
        let crossing_segment=crate::source_material_segment::inspect_boundary(
            &body,[10.,2.,-1.],[0.,0.,8.],1e-7,10000,10000).unwrap();
        eprintln!("original source segment contacts={} unresolved={} cells={} domains={}",
            crossing_segment.contacts.len(),crossing_segment.unresolved.len(),
            crossing_segment.cells,crossing_segment.domain_cells);
        assert!(!crossing_segment.boundary_free);
        assert_eq!(crossing_segment.contacts.len(),2);
        assert!(crossing_segment.unresolved.is_empty());
        assert!(crossing_segment.contacts[0].parameter[1] < crossing_segment.contacts[1].parameter[0]
            || crossing_segment.contacts[1].parameter[1] < crossing_segment.contacts[0].parameter[0]);
        assert!(crate::source_material_segment::inspect_boundary(
            &body,[0.;3],[0.;3],1e-7,10000,10000).is_err());
        let chord=crate::source_material_chord::qualify(&body,[10.,2.,-1.],[0.,0.,8.],1e-7,
            crate::source_material_chord::Limits{cells:10000,domain_cells:10000,normal_spans:1000,max_sine_squared:1e-6}).unwrap();
        eprintln!("source normal chord {} length={:?}",chord.reason,chord.certificate.as_ref().map(|c|c.length_mm()));
        let proof=chord.certificate.expect("original annular flat normal material chord");
        assert!(std::ptr::eq(proof.body(),&body));
        assert!(proof.length_mm()[0]<=6. && proof.length_mm()[1]>=6.);
        assert!(proof.length_mm()[1]-proof.length_mm()[0]<1e-5);
        for (origin,direction,cells,normal_spans) in [
            ([10.,2.,1.],[0.,0.,4.],10000,1000),
            ([30.,30.,1.],[0.,0.,1.],10000,1000),
            ([10.,2.,-1.],[0.,0.,8.],1,1000),
            ([10.,2.,-1.],[1.,0.,8.],10000,1000),
            ([10.,2.,-1.],[0.,0.,8.],10000,1),
        ] {
            let denied=crate::source_material_chord::qualify(&body,origin,direction,1e-7,
                crate::source_material_chord::Limits{cells,domain_cells:10000,normal_spans,max_sine_squared:1e-6}).unwrap();
            eprintln!("source chord refusal {}",denied.reason);
            assert!(denied.certificate.is_none());
        }
        for (groups,minimum,tolerance,expected) in [([&bottom[..],&top[..]],5.99,0.02,true),
            ([&bottom[..],&top[..]],5.,1e-16,false),
            ([&[2,7,12,16,20,24][..],&[3,8,13,17,21,25][..]],14.5,0.02,false)] {
            let wall=crate::source_material_wall::qualify(&body,groups,minimum,tolerance,
                [10.,2.,-1.],[0.,0.,8.],1e-7,
                crate::source_material_wall::Limits{
                    gap:crate::source_face_gap::Limits{cells:10000,spans:20000},
                    chord:crate::source_material_chord::Limits{cells:10000,domain_cells:10000,normal_spans:1000,max_sine_squared:1e-6}}).unwrap();
            eprintln!("source wall {} interval={:?} converged={}",wall.reason,
                wall.certificate.as_ref().map(|c|c.interval_mm()),wall.converged);
            assert_eq!(wall.converged,expected);
            if minimum==14.5 {assert!(wall.certificate.is_none());}
            else {let proof=wall.certificate.unwrap();assert!(std::ptr::eq(proof.body(),&body));
                assert!(proof.interval_mm()[0]<=6. && proof.interval_mm()[1]>=6.);}
        }
        let search=crate::source_wall_search::search(&body,[&bottom,&top],3,54,1e-7,
            crate::source_material_chord::Limits{cells:10000,domain_cells:10000,normal_spans:1000,max_sine_squared:1e-6}).unwrap();
        eprintln!("source wall search attempts={} refused={} exhausted={} best={:?}",
            search.attempts,search.refused,search.candidates_exhausted,search.best.as_ref().map(|c|c.length_mm()));
        assert_eq!(search.attempts,54);assert!(search.candidates_exhausted);
        let best=search.best.expect("automatic original annular axial material chord");
        assert!(best.length_mm()[0]<=6. && best.length_mm()[1]>=6.);
        assert!(best.length_mm()[1]-best.length_mm()[0]<1e-5);
        let limited=crate::source_wall_search::search(&body,[&bottom,&top],3,1,1e-7,
            crate::source_material_chord::Limits{cells:10000,domain_cells:10000,normal_spans:1000,max_sine_squared:1e-6}).unwrap();
        assert_eq!(limited.attempts,1);assert!(!limited.candidates_exhausted);
        assert!(crate::source_wall_search::search(&body,[&bottom,&bottom],3,1,1e-7,
            crate::source_material_chord::Limits{cells:10000,domain_cells:10000,normal_spans:1000,max_sine_squared:1e-6}).is_err());
        for (cells,expected) in [(1000,true),(1,false)] {
            let automatic=crate::source_material_wall::search_and_qualify(&body,[&bottom,&top],5.99,0.02,1e-7,3,54,
                crate::source_material_wall::Limits{gap:crate::source_face_gap::Limits{cells,spans:2000},
                    chord:crate::source_material_chord::Limits{cells:10000,domain_cells:10000,normal_spans:1000,max_sine_squared:1e-6}}).unwrap();
            eprintln!("source automatic wall {} converged={} interval={:?}",automatic.reason,
                automatic.converged,automatic.certificate.as_ref().map(|c|c.interval_mm()));
            assert_eq!(automatic.converged,expected);assert_eq!(automatic.certificate.is_some(),expected);
            if let Some(proof)=automatic.certificate {assert!(std::ptr::eq(proof.body(),&body));
                assert!(proof.interval_mm()[0]<=6. && proof.interval_mm()[1]>=6.);}
            else {assert!(automatic.search.best.is_some(),"witness alone must not admit minimum thickness");}
        }
        let radial=crate::source_material_chord::qualify_between(&body,
            [&[2,7,12,16,20,24],&[3,8,13,17,21,25]],[25.,5.,3.],[-50.,-10.,0.],1e-7,
            crate::source_material_chord::Limits{cells:10000,domain_cells:10000,normal_spans:1000,max_sine_squared:1e-6}).unwrap();
        eprintln!("source cavity radial chord {} contacts={} length={:?}",radial.reason,radial.boundary.contacts.len(),
            radial.certificate.as_ref().map(|c|c.length_mm()));
        assert_eq!(radial.boundary.contacts.len(),4);
        let proof=radial.certificate.expect("original annular radial material interval around cavity");
        assert!(proof.length_mm()[0]<=15. && proof.length_mm()[1]>=15.);
        assert!(proof.length_mm()[1]-proof.length_mm()[0]<1e-5);
        assert_eq!(proof.line(),[[25.,5.,3.],[-50.,-10.,0.]]);
        let endpoints=proof.endpoints();
        for (i,endpoint) in endpoints.iter().enumerate() {
            let crossing=&radial.boundary.contacts[i];
            assert_eq!(endpoint.face,crossing.face);
            assert_eq!(endpoint.uv,crossing.uv);
            assert_eq!(endpoint.parameter,crossing.parameter);
            let surface=body.geometry().shell().regions().unwrap()[endpoint.face].loops()[0][0].surface();
            assert_eq!(endpoint.world_mm.to_vec(),nurbs_core::surface_distance::rectangle_bounds(surface,endpoint.uv).unwrap());
            assert!(endpoint.world_mm.iter().all(|v|v[0].is_finite() && v[0]<=v[1] && v[1].is_finite()));
        }
        assert!(endpoints[0].parameter[1]<endpoints[1].parameter[0]);
        // Independent analytic radial oracle; rounded oracle coordinates are
        // checked with an explicit 1e-9 mm tolerance, never used for admission.
        let radial_unit=[25./650f64.sqrt(),5./650f64.sqrt(),0.];
        for (end,radius) in endpoints.iter().zip([20.,5.]) {
            let expected=[radius*radial_unit[0],radius*radial_unit[1],3.];
            for (bounds,value) in end.world_mm.iter().zip(expected) {
                assert!(bounds[0]-1e-9<=value && value<=bounds[1]+1e-9);
                assert!(bounds[1]-bounds[0]<1e-5);
            }
            let expected_t=(650f64.sqrt()-radius)/(2.*650f64.sqrt());
            assert!(end.parameter[0]-1e-11<=expected_t && expected_t<=end.parameter[1]+1e-11);
        }

        let scan=crate::source_wall_scan::search(&body,7.,3,243,1e-7,
            crate::source_material_chord::Limits{cells:10000,domain_cells:10000,normal_spans:1000,max_sine_squared:1e-6}).unwrap();
        eprintln!("original all-face wall scan attempts={} refused={} visited={} exhausted={} best={:?}",scan.attempts,scan.refused,scan.faces_visited,scan.proposals_exhausted,scan.best().map(|c|c.length_mm()));
        assert_eq!(scan.attempts,243);assert_eq!(scan.faces_visited,27);assert!(scan.proposals_exhausted);
        let thin=scan.thin_witness().expect("automatic native scan must find the 6 mm wall");
        assert!(thin.length_mm()[1]<7.);assert!(std::ptr::eq(thin.body(),&body));
        let coarse_scan=crate::source_wall_scan::search(&body,7.,1,27,1e-7,
            crate::source_material_chord::Limits{cells:10000,domain_cells:10000,normal_spans:1000,max_sine_squared:1e-6}).unwrap();
        assert!(coarse_scan.proposals_exhausted);assert_eq!(coarse_scan.faces_visited,27);
        assert!(coarse_scan.thin_witness().is_none(),"exhausted coarse proposals must not assert the missing 6 mm witness");
        let coarse=coarse_scan.best().unwrap();assert!(coarse.length_mm()[0]<=15. && coarse.length_mm()[1]>=15.);
        for (minimum,grid,attempts) in [(f64::NAN,3,243),(0.,3,243),(7.,0,243),(7.,9,243),(7.,3,0),(7.,3,257)] {
            assert!(crate::source_wall_scan::search(&body,minimum,grid,attempts,1e-7,
                crate::source_material_chord::Limits{cells:10000,domain_cells:10000,normal_spans:1000,max_sine_squared:1e-6}).is_err());
        }
        let limited_scan=crate::source_wall_scan::search(&body,1.,1,1,1e-7,
            crate::source_material_chord::Limits{cells:10000,domain_cells:10000,normal_spans:1000,max_sine_squared:1e-6}).unwrap();
        assert_eq!(limited_scan.attempts,1);assert!(!limited_scan.proposals_exhausted);assert!(limited_scan.thin_witness().is_none());
        for pair_budget in [1,378] {
            let coverage=crate::source_wall_coverage::qualify(&body,5.99,crate::source_wall_coverage::Limits{
                pairs:pair_budget,plane_controls:10000,normal_spans:1000,gap:crate::source_face_gap::Limits{cells:1000,spans:2000},max_sine_squared:1e-6}).unwrap();
            eprintln!("original continuous wall coverage reason={} pairs={}/{} unresolved={} cells={} normals={}",coverage.reason,
                coverage.pairs.len(),coverage.total_pairs,coverage.pairs.iter().filter(|p|!p.proven).count(),coverage.cells,coverage.normal_spans);
            assert_eq!(coverage.total_pairs,378);assert_eq!(coverage.enumeration_complete,pair_budget==378);
            assert!(coverage.certificate.is_none());assert!(coverage.cells<=1000&&coverage.spans<=2000&&coverage.normal_spans<=1000);
            if pair_budget==378 {assert!(coverage.pairs.iter().any(|p|p.faces[0]==p.faces[1]&&!p.proven));}
        }
        let self_coverage=crate::source_wall_self_coverage::qualify(&body,5.99,1e-6,
            crate::source_wall_self_coverage::Limits{plane_controls:10000,cells:10000,spans:100000,cells_per_face:512,spans_per_face:4096}).unwrap();
        eprintln!("original adaptive self coverage qualified={}/{} cells={} spans={} pending={}",
            self_coverage.certificates().len(),self_coverage.faces.len(),self_coverage.cells,self_coverage.spans,
            self_coverage.faces.iter().map(|f|f.pending).sum::<usize>());
        assert_eq!(self_coverage.faces.len(),27);
        assert!(self_coverage.certificates().len()>=14);
        assert!(self_coverage.cells<=10000&&self_coverage.spans<=100000&&self_coverage.plane_controls<=10000);
        for (i,face) in self_coverage.faces.iter().enumerate() {
            assert_eq!(face.face,i);assert!(face.cells<=512&&face.spans<=4096);
            assert_eq!(face.qualified,face.pending==0);assert_eq!(face.qualified,face.uncertain.is_none());
            assert_eq!(face.pending,face.unresolved_pairs.len());
            assert_eq!(face.uncertain,face.unresolved_pairs.last().copied());
        }
        for c in self_coverage.certificates() {
            assert!(std::ptr::eq(c.body(),&body));assert_eq!(c.minimum_mm(),5.99);assert_eq!(c.max_sine_squared(),1e-6);
            if let Some(proof)=c.adaptive() {assert!(std::ptr::eq(proof.surface(),body.geometry().shell().regions().unwrap()[c.face()].loops()[0][0].surface()));}
        }
        assert!(!self_coverage.all_self_pairs_qualified());
        let mut self_coverage=self_coverage;
        self_coverage.faces.retain(|f|f.qualified);
        assert!(!self_coverage.all_self_pairs_qualified(),"public diagnostics cannot change original face coverage");
        let issued_self_faces:std::collections::BTreeSet<_>=self_coverage.certificates()
            .iter().map(|c|c.face()).collect();
        assert!(issued_self_faces.len()>=16,"retain the previously qualified face classes");
        let reused=crate::source_wall_coverage::qualify_with_self_facts(&body,5.99,
            crate::source_wall_coverage::Limits{pairs:378,plane_controls:10000,normal_spans:1000,
                gap:crate::source_face_gap::Limits{cells:1000,spans:2000},max_sine_squared:1e-6},self_coverage.into_certificates()).unwrap();
        eprintln!("original reused adaptive wall pairs={} unresolved={} cells={} normals={}",
            reused.pairs.len(),reused.pairs.iter().filter(|p|!p.proven).count(),reused.cells,reused.normal_spans);
        assert!(reused.enumeration_complete);assert!(reused.certificate.is_none());
        for product_cells in [1,50000] {
            let refined=crate::source_wall_coverage::qualify_with_self_facts_and_normal_products(
                &body,5.99,crate::source_wall_coverage::Limits{
                    pairs:378,plane_controls:10000,normal_spans:20000,
                    gap:crate::source_face_gap::Limits{cells:1000,spans:2000},max_sine_squared:1e-6},
                Vec::new(),crate::source_wall_coverage::NormalProductLimits{
                    cells:product_cells,cells_per_pair:128}).unwrap();
            let unresolved=refined.pairs.iter().filter(|p|!p.proven).count();
            eprintln!("native normal-product cells={} normals={} unresolved={}",
                refined.normal_product_cells,refined.normal_spans,unresolved);
            assert!(refined.enumeration_complete);assert!(refined.certificate.is_none());
            assert!(refined.normal_product_cells<=product_cells&&refined.normal_spans<=20000);
            if product_cells==50000 {assert!(unresolved<257,"full-product exclusions improve original whole-wall coverage");}
            for previous in reused.pairs.iter().filter(|p|p.proven&&p.faces[0]!=p.faces[1]) {
                assert!(refined.pairs.iter().any(|p|p.faces==previous.faces&&p.proven),
                    "retain previously confirmed distinct pairs {:?}",previous.faces);
            }
        }
        for pair in &reused.pairs {
            let adjacent=body.geometry().shell().uses().iter().any(|u|
                (u[0].face==pair.faces[0]&&u[1].face==pair.faces[1])||
                (u[1].face==pair.faces[0]&&u[0].face==pair.faces[1]));
            if adjacent {assert!(!pair.reason.starts_with("source-face-gap-"),"shared-boundary pairs cannot consume clearance subdivision");}
        }

        let transported_self_faces:std::collections::BTreeSet<_>=reused.pairs.iter()
            .filter(|p|p.reason=="source-wall-self-certified").map(|p| {
                assert!(p.proven && p.faces[0]==p.faces[1]);p.faces[0]
            }).collect();
        assert_eq!(transported_self_faces,issued_self_faces,
            "every issued source-owned self certificate must be reused exactly once");
        assert!(reused.pairs.iter().filter(|p|!p.proven).count()<=256);
        let cavity_faces=[radial.boundary.contacts[1].face,radial.boundary.contacts[2].face];
        let cavity=crate::source_material_chord::qualify_between(&body,
            [&[cavity_faces[0]],&[cavity_faces[1]]],[25.,5.,3.],[-50.,-10.,0.],1e-7,
            crate::source_material_chord::Limits{cells:10000,domain_cells:10000,normal_spans:1000,max_sine_squared:1e-6}).unwrap();
        assert!(cavity.certificate.is_none());assert_eq!(cavity.reason,"material-pair-outside-groups");
        let automatic_radial=crate::source_material_wall::search_and_qualify(&body,
            [&[2,7,12,16,20,24],&[3,8,13,17,21,25]],14.5,0.6,1e-7,3,54,
            crate::source_material_wall::Limits{gap:crate::source_face_gap::Limits{cells:10000,spans:20000},
                chord:crate::source_material_chord::Limits{cells:10000,domain_cells:10000,normal_spans:1000,max_sine_squared:1e-6}}).unwrap();
        eprintln!("source automatic radial wall {} attempts={} refused={} interval={:?}",automatic_radial.reason,
            automatic_radial.search.attempts,automatic_radial.search.refused,automatic_radial.certificate.as_ref().map(|c|c.interval_mm()));
        assert!(automatic_radial.converged);
        let proof=automatic_radial.certificate.expect("automatically found curved cavity wall");
        assert!(proof.interval_mm()[0]<=15. && proof.interval_mm()[1]>=15.);
        for (cells,spans,tolerance,expected) in [(50000,100000,0.02,true),(10000,20000,1e-16,false)] {
            let refined=crate::source_material_wall::search_and_refine(&body,
                [&[2,7,12,16,20,24],&[3,8,13,17,21,25]],14.5,tolerance,1e-7,3,54,
                crate::source_material_wall::Limits{gap:crate::source_face_gap::Limits{cells,spans},
                    chord:crate::source_material_chord::Limits{cells:10000,domain_cells:10000,normal_spans:1000,max_sine_squared:1e-6}}).unwrap();
            eprintln!("source refined radial wall converged={} cells={} spans={} interval={:?} refinement={:?}",
                refined.result.converged,refined.cells,refined.spans,refined.result.certificate.as_ref().map(|c|c.interval_mm()),
                refined.refinement.as_ref().map(|r|r.reason));
            assert!(refined.cells<=cells && refined.spans<=spans);
            assert_eq!(refined.result.converged,expected);
            let proof=refined.result.certificate.expect("retain valid original lower bounds even if refinement exhausts");
            assert!(proof.interval_mm()[0]<=15. && proof.interval_mm()[1]>=15.);
            if expected {assert!(proof.interval_mm()[1]-proof.interval_mm()[0]<=0.02);}
        }
        let blended_gap=crate::source_face_gap::qualify(&body,
            [&[0,2,5,7,10,12,16,20,24],&[3,8,13,17,21,25]],13.5,
            crate::source_face_gap::Limits{cells:50000,spans:100000}).unwrap();
        eprintln!("original annular blend wall gap {} cells={} spans={} uncertain={:?}",blended_gap.reason,blended_gap.cells,blended_gap.spans,blended_gap.uncertain_faces);
        let blended_proof=blended_gap.certificate.expect("all original transition/cylinder versus inner wall pairs need clearance");
        assert!(blended_proof.lower_mm()>=13.5&&blended_proof.lower_mm()<=13.75);
        assert!(blended_gap.cells>=54&&blended_gap.cells<=50000&&blended_gap.spans<=100000);
        let q=std::f64::consts::FRAC_PI_2;
        let spans=[
            crate::circular_blend::plane_cylinder_transition(20.,6.,0.,1.25,0.,q/4.).unwrap(),
            crate::circular_blend::plane_cylinder_rim(20.,6.,1.25,q/4.,q/2.).unwrap().remove(0),
            crate::circular_blend::plane_cylinder_transition(20.,6.,1.25,0.,q*0.75,q/4.).unwrap(),
        ];
        for (face,span) in [0,5,10].into_iter().zip(spans) {
            let actual=body.geometry().shell().faces()[face][0].edges()[0].surface();
            let mut centers=span.centers.clone();let mut law=span.radius_curve();
            if actual!=&span.surface {
                let mut reversed=span.surface.clone();reversed.control_points.reverse();reversed.weights.reverse();
                assert_eq!(actual,&reversed,"original face {face} must match the authored parameter orientation");
                centers=centers.reverse().unwrap();law=law.reverse().unwrap();
            }
            let tolerance=if face==5 {1e-9}else{1e-6};
            let r=body.qualify_face_radius(face,&centers,&law,tolerance,100,100_000_000).unwrap();
            eprintln!("original body radius face={face} reason={} bound={:?} work={}",r.reason,r.error_upper,r.work);
            let certificate=r.certificate.expect("original body patch radius needs full-domain certificate");
            assert_eq!(certificate.surface(),actual);
            assert!(body.qualify_face_radius(face,&centers,&law,tolerance,100,1).unwrap().certificate.is_none());
            let mut wrong=law.clone();wrong.control_points[0][0]+=0.1;
            assert!(body.qualify_face_radius(face,&centers,&wrong,tolerance,100,100_000_000).unwrap().certificate.is_none());
        }
        for neighbor in [6,7] {
            let edge=body.geometry().shell().uses().iter().position(|uses|uses.iter().all(|a|[5,neighbor].contains(&a.face))).unwrap();
            let r=body.qualify_edge_tangency(edge,crate::source_seam_tangency::Limits {
                max_sine_squared:1e-6,cells:100000,curve_spans:100000,normal_spans:100000,
            }).unwrap();
            eprintln!("original middle seam edge={edge} neighbor={neighbor} reason={} cells={} normals={}",r.reason,r.cells,r.normal_spans);
            let certificate=r.seam.expect("middle fillet rails need full interval G1");
            assert!(certificate.sine_squared_bounds()[1]<=1e-6);
            eprintln!("original middle seam edge={edge} sine2={:?}",certificate.sine_squared_bounds());
            assert!(body.qualify_edge_tangency(edge,crate::source_seam_tangency::Limits {
                max_sine_squared:1e-6,cells:1,curve_spans:1,normal_spans:1,
            }).unwrap().seam.is_none());
        }
        for faces in [[0,1],[0,2],[10,11],[10,12]] {
            let edge=body.geometry().shell().uses().iter().position(|uses|uses.iter().all(|a|faces.contains(&a.face))).unwrap();
            let r=body.qualify_edge_tangency(edge,crate::source_seam_tangency::Limits {
                max_sine_squared:1e-6,cells:1024,curve_spans:2048,normal_spans:2048,
            }).unwrap();
            eprintln!("original pole seam edge={edge} faces={faces:?} reason={} cells={} uncertain={:?}",r.reason,r.cells,r.uncertain_canonical);
            assert!(r.seam.is_none(),"collapsed endpoint must not acquire a regular G1 certificate");
            assert!(r.uncertain_canonical.is_some());
        }
    }
    #[test]
    fn original_annular_poles_are_preserved_in_step_exchange() {
        let model=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let shell=prepare(&model,limits()).unwrap().shell.unwrap();
        let geometry=crate::source_shell_geometry::qualify(shell,geometry_limits()).unwrap().geometry.unwrap();
        let volume=crate::source_volume::qualify(geometry,crate::source_volume::Limits {
            axis:2,origin:0.,absolute_error:20.,tolerance_uv:1e-8,cells:100000,spans:100000,domain_cells:1000000,
        }).unwrap();
        let body=volume.body.unwrap();
        let endpoint_limits=||crate::source_exchange_endpoints::Limits {
            root_checks:100000,mapping_cells:100000,replay_mapping_per_use:10000,
            exact_work:100_000_000,driver_cells:100000,spans:100000,endpoints:100000,
        };
        assert!(crate::source_exchange_step::prepare(&body,1e-7,endpoint_limits(),1).unwrap().is_none());
        assert!(crate::source_exchange_step::prepare(&body,1e-7,endpoint_limits(),0).is_err());
        assert!(crate::source_exchange_step::prepare(&body,1e-7,endpoint_limits(),10_000_001).is_err());
        let candidate=crate::source_exchange_step::prepare(&body,1e-7,endpoint_limits(),100000).unwrap().expect("original poles must export");
        assert_eq!(candidate.faces,27);
        assert_eq!(candidate.edges,body.geometry().shell().edges().len()+2);
        assert!(candidate.endpoint_error_upper<=1e-7);
        eprintln!("original annular STEP faces={} edges={} vertices={} bound={}",candidate.faces,candidate.edges,candidate.vertices,candidate.endpoint_error_upper);
        if let Some(path)=std::env::var_os("CAD_ANNULAR_SOURCE_STEP_OUTPUT") {std::fs::write(path,candidate.text).unwrap();}
    }
    #[test]
    fn annular_planar_flux_keeps_budget_and_curved_fallback_guards() {
        let model=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let shell=prepare(&model,limits()).unwrap().shell.unwrap();
        let geometry=crate::source_shell_geometry::qualify(shell,geometry_limits()).unwrap().geometry.unwrap();
        assert!(crate::source_planar_flux::bound(&geometry,0,2,0.,0.01,1000).unwrap().bound.is_none());
        assert!(crate::source_planar_flux::bound(&geometry,1,2,0.,0.01,1).unwrap().bound.is_none());
        let zero=crate::source_planar_flux::bound(&geometry,4,2,0.,0.01,1).unwrap();
        assert_eq!(zero.cells,0);
        let zero=zero.bound.unwrap();assert_eq!([zero.lo,zero.hi],[0.,0.]);
        let limited=crate::source_volume::qualify(geometry,crate::source_volume::Limits {
            axis:2,origin:0.,absolute_error:0.25,tolerance_uv:1e-8,cells:1,spans:1,domain_cells:1,
        }).unwrap();
        assert!(limited.body.is_none());assert!(limited.cells<=1 && limited.spans<=1);
    }
    #[test]
    fn annular_original_geometry_admits_volume_and_orientation() {
        let model=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let shell=prepare(&model,limits()).unwrap().shell.unwrap();
        let geometry=crate::source_shell_geometry::qualify(shell,geometry_limits()).unwrap().geometry.unwrap();
        let r=crate::source_volume::qualify(geometry,crate::source_volume::Limits {
            axis:2,origin:0.,absolute_error:0.25,tolerance_uv:1e-8,
            cells:100000,spans:100000,domain_cells:1000000,
        }).unwrap();
        eprintln!("annular volume {} bounds={:?} cells={} spans={} domain={} face={:?}",r.reason,r.signed_bounds,r.cells,r.spans,r.domain_cells,r.uncertain_face);
        let body=r.body.expect("embedded original annulus must admit bounded volume");
        eprintln!("annular body volume {:?} reverse={}",body.volume(),body.reverse_orientation());
        let independent=7061.4573708387725_f64;
        assert!(body.volume()[0]<=independent && independent<=body.volume()[1]);
        assert!(!body.reverse_orientation());
        assert!(body.volume()[0]>0. && body.volume()[1]-body.volume()[0]<=0.25);
        let original=body.definition().unwrap();
        let mut saved=original.clone();
        saved["volume"]=value_codec::json!([1.,1.]);
        saved["success"]=value_codec::json!(true);
        saved["reverseOrientation"]=value_codec::json!(true);
        let replay=crate::source_body_restore::restore(saved,crate::source_body_restore::Limits {
            shell:crate::source_shell_restore::Limits {
                regions:crate::source_region_restore::test_limits(),exact_work:100_000_000,driver_cells:100000,
            },
            embedding:geometry_limits(),
            volume:crate::source_volume::Limits {
                axis:2,origin:0.,absolute_error:0.25,tolerance_uv:1e-8,cells:100000,spans:100000,domain_cells:1000000,
            },
        }).unwrap();
        let recovered=replay.body().expect("original annular body must freshly restore all gates");
        assert_eq!(recovered.definition().unwrap(),original);
        assert_eq!(recovered.volume(),body.volume());
        assert_eq!(recovered.reverse_orientation(),body.reverse_orientation());
        assert_eq!(recovered.geometry().shell().vertices(),body.geometry().shell().vertices());
        if let Some(path)=std::env::var_os("CAD_ANNULAR_ORIGINAL_FLUX_OUTPUT") {
            let faces=body.geometry().shell().regions().unwrap().iter().map(|r| {
                let loops=r.loops().iter().map(|wire|wire.iter().map(|f| {
                    let endpoints=f.endpoints().each_ref().map(|e|match e {
                        crate::source_boundary_fragment::Endpoint::Parameter(t)=>*t,
                        _=>panic!("independent fixture requires original literal endpoints"),
                    });
                    value_codec::json!({"curve":f.curve(),"parameters":endpoints})
                }).collect::<Vec<_>>()).collect::<Vec<_>>();
                value_codec::json!({"surface":r.loops()[0][0].surface(),"wholeChartMaterial":r.whole_chart_material(),"chartWinding":r.chart_winding(),"loops":loops})
            }).collect::<Vec<_>>();
            let fixture=value_codec::json!({"schema":"cad-original-annular-flux/1","faces":faces,"nativeVolumeBounds":body.volume()});
            std::fs::write(path,value_codec::to_string_pretty(&fixture).unwrap()).unwrap();
        }

    }
    #[test]
    fn annular_outer_wall_bottom_contact() {
        let model=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let shell=prepare(&model,limits()).unwrap().shell.unwrap();
        let r=crate::source_pole_planar_contact::certify(&shell,[2,4],100_000_000,100000).unwrap();
        eprintln!("wall/bottom {} work={} driver={}",r.reason,r.exact_work,r.driver_cells);

        assert!(r.certificate.is_some(),"{}",r.reason);
        assert!(crate::source_pole_planar_contact::certify(&shell,[4,2],100_000_000,100000).unwrap().certificate.is_some());
        assert!(crate::source_pole_planar_contact::certify(&shell,[2,4],1,100000).unwrap().certificate.is_none());
    }
    #[test]
    fn annular_top_and_distant_inner_wall_separation() {
        let model=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let shell=prepare(&model,limits()).unwrap().shell.unwrap();
        let r=crate::source_hull_separation::certify_shell(&shell,[1,13],100_000_000).unwrap();
        eprintln!("distant inner separation {} work={}",r.reason,r.exact_work);
        assert!(r.certificate.is_some(),"{}",r.reason);
        assert!(crate::source_hull_separation::certify_shell(&shell,[13,1],100_000_000).unwrap().certificate.is_some());
        assert!(crate::source_hull_separation::certify_shell(&shell,[1,13],1).unwrap().certificate.is_none());
    }
    #[test]
    fn annular_distant_top_material_separation() {
        let model=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let shell=prepare(&model,limits()).unwrap().shell.unwrap();
        let r=crate::source_hull_separation::certify_shell(&shell,[1,10],100_000_000).unwrap();
        eprintln!("distant material separation {} work={}",r.reason,r.exact_work);
        let c=r.certificate.expect("original material hull must separate distant transition");
        assert!(c.material_hulls()[0].is_some());
        assert!(crate::source_hull_separation::certify_shell(&shell,[10,1],100_000_000).unwrap().certificate.is_some());
        assert!(crate::source_hull_separation::certify_shell(&shell,[1,10],1).unwrap().certificate.is_none());
        assert!(crate::source_hull_separation::certify_shell(&shell,[1,6],100_000_000).unwrap().certificate.is_none());
    }
    #[test]
    fn annular_neighboring_top_material_contact() {
        let model=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let shell=prepare(&model,limits()).unwrap().shell.unwrap();
        let r=crate::source_allowed_contact::certify(&shell,[1,6],100_000_000,100000).unwrap();
        eprintln!("top contact {} work={}",r.reason,r.exact_work);
        assert!(r.certificate.is_some(),"{}",r.reason);
        assert!(crate::source_allowed_contact::certify(&shell,[6,1],100_000_000,100000).unwrap().certificate.is_some());
        assert!(crate::source_allowed_contact::certify(&shell,[1,6],1,100000).unwrap().certificate.is_none());
        assert!(crate::source_allowed_contact::certify(&shell,[1,11],100_000_000,100000).unwrap().certificate.is_none());
    }
    #[test]
    fn annular_top_and_ruled_wall_only_contact_at_owned_pole() {
        let model=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let shell=prepare(&model,limits()).unwrap().shell.unwrap();
        let report=crate::source_vertex_contact::certify(&shell,[1,2],100_000_000).unwrap();
        eprintln!("top/cylinder owned pole: {} work={}",report.reason,report.exact_work);
        let certificate=report.certificate.expect("original top/cylinder pole needs contact certificate");
        assert!(certificate.corner_image().is_some());
        assert_eq!(certificate.corner_image().unwrap().point(),certificate.point());
    }
    #[test]
    fn trimmed_plane_vertex_owns_an_exact_original_carrier_endpoint() {
        let model=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let shell=prepare(&model,limits()).unwrap().shell.unwrap();
        let regions=shell.regions().unwrap();
        let mut owned=std::collections::BTreeMap::new();
        for face in [0,6] {
            for (wire,fragments) in regions[face].loops().iter().enumerate() {
                for (edge,fragment) in fragments.iter().enumerate() {
                    for end in 0..2 {
                        let address=crate::source_shell_incidence::Address {face,wire,edge};
                        if let Some(point)=crate::source_vertex_contact::carrier_corner(&shell,address,end) {
                            if face==6 {assert!(crate::source_vertex_contact::corner(fragment,end).is_none());}
                            owned.entry(shell.vertices()[face][wire][edge][end]).or_insert_with(Vec::new).push((face,point));
                        }
                    }
                }
            }
        }
        let shared=owned.values().filter(|ends|ends.iter().any(|e|e.0==0)&&ends.iter().any(|e|e.0==6)).collect::<Vec<_>>();
        assert_eq!(shared.len(),1);
        assert!(shared[0].iter().all(|e|e.1==shared[0][0].1));
        let report=crate::source_vertex_contact::certify(&shell,[0,6],100_000_000).unwrap();
        eprintln!("trimmed planar vertex: {} work={}",report.reason,report.exact_work);
        assert!(report.certificate.is_some(),"{}",report.reason);
        assert!(report.certificate.unwrap().material_hulls()[1].is_some());
    }
    #[test]
    fn original_annular_transition_cylinder_contact_is_owned() {
        let model=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let shell=prepare(&model,limits()).unwrap().shell.unwrap();
        let proof=crate::source_ruled_projection_contact::certify(&shell,[0,2],1e-8,100_000_000,100000).unwrap();
        eprintln!("original annular ruled contact: {} work={} driver={}",proof.reason,proof.exact_work,proof.driver_cells);
        assert!(proof.certificate.is_some(),"{}",proof.reason);
        let certificate=proof.certificate.unwrap();
        assert_eq!(certificate.faces(),[0,2]);
        assert_eq!(certificate.projection().surface(),shell.regions().unwrap()[0].loops()[0][0].surface());
        assert!(!certificate.edges().is_empty());
        assert!(crate::source_ruled_projection_contact::certify(&shell,[0,2],1e-8,1,100000).unwrap().certificate.is_none());
        assert!(crate::source_ruled_projection_contact::certify(&shell,[0,3],1e-8,100_000_000,100000).unwrap().certificate.is_none());
        let reverse=crate::source_ruled_projection_contact::certify(&shell,[2,0],1e-8,100_000_000,100000).unwrap();
        assert_eq!(reverse.certificate.unwrap().faces(),[2,0]);
    }
    #[test]
    fn canonical_original_annular_transition_has_jordan_projection() {
        let model=crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        let shell=prepare(&model,limits()).unwrap().shell.unwrap();
        let surface=shell.regions().unwrap()[0].loops()[0][0].surface();
        let report=nurbs_core::surface_projected_jordan::certify(surface,[0,1],1e-8,100_000_000,100000).unwrap();
        eprintln!("canonical original annular Jordan: {} work={} cells={}",report.reason,report.exact_work,report.boundary_cells);
        assert!(report.certificate.is_some(),"{}",report.reason);
        assert_eq!(report.certificate.unwrap().surface(),surface);
    }
    #[test]
    fn original_annular_projection_orientation_and_wall_rank_deficiency() {
        let model = crate::circular_blend::partial_annular_quarter(20.,5.,6.,1.25,1.,1e-7).unwrap();
        for face in [0,2] {
            let surface = &model.faces[face].surface;
            let report = nurbs_core::surface_projection_jacobian::certify(surface,[0,1],100_000_000).unwrap();
            let counts = report.signs.as_ref().map(|rows| {
                let mut counts=[0usize;3];
                for s in rows.iter().flatten() { counts[match s {
                    cad_predicates::Sign::Negative=>0,
                    cad_predicates::Sign::Zero=>1,
                    cad_predicates::Sign::Positive=>2,
                }]+=1; } counts
            });
            eprintln!("original annular face {face}: {} work={} exact={:?} signs={counts:?}",report.reason,report.exact_work,report.exact_reason);
            if face==0 {
                let certificate=report.certificate.as_ref().expect("corrected original transition needs orientation proof");
                assert_eq!(certificate.orientation(),cad_predicates::Sign::Negative);
                assert_eq!(certificate.surface(),surface);
                assert_eq!(counts,Some([77,13,0]));
                assert_eq!(report.opposite_v_boundary_signs,None);
                assert!(nurbs_core::surface_projection_jacobian::certify(surface,[0,1],1).unwrap().certificate.is_none());
            } else {
                assert!(report.signs.is_some(),"{}",report.reason);
                assert!(report.signs.as_ref().unwrap().iter().flatten().all(|s| *s==cad_predicates::Sign::Zero));
                assert!(report.certificate.is_none());
            }
        }
    }
    #[test]
    fn annular_incidence_and_support_contacts_qualify_embedded_geometry() {
        let model =
            crate::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
        let report = prepare(&model, limits());
        match report {
            Ok(report) => {
                eprintln!(
                    "annular source incidence: {} pair={:?}",
                    report.reason, report.uncertain_pair
                );
                assert!(report.shell.is_some(), "{}", report.reason);
                let shell = report.shell.unwrap();
                assert_eq!(shell.faces().len(), 27);
                assert_eq!(shell.poles().len(), 2);
                let contact =
                    crate::source_pole_planar_contact::certify(&shell, [0, 1], 100_000_000, 100000)
                        .unwrap();
                assert!(contact.certificate.is_some(), "{}", contact.reason);
                assert!(
                    crate::source_pole_planar_contact::certify(&shell, [0, 1], 1, 100000)
                        .unwrap()
                        .certificate
                        .is_none()
                );
                let geometry = crate::source_shell_geometry::qualify(
                    shell,
                    geometry_limits(),
                )
                .unwrap();
                eprintln!(
                    "annular source geometry: {} face={:?} pair={:?}",
                    geometry.reason, geometry.uncertain_face, geometry.next_pair
                );
                eprintln!("annular resources exact={} spans={} driver={} linear={} pairs={}",geometry.exact_work,geometry.spans,geometry.driver_cells,geometry.linear_cells,geometry.pairs);
                assert!(geometry.exact_work<=100_000_000 && geometry.driver_cells<=100000);
                assert_eq!(geometry.reason,"source-shell-embedded-geometry-qualified");
                assert_eq!(geometry.next_pair,None);
                let geometry=geometry.geometry.expect("all original annular contacts must qualify");
                assert!(geometry.contacts().all_pairs_qualified);
                assert_eq!(geometry.contacts().pairs.len(),351);
                assert_eq!(geometry.contacts().total_pairs,351);
                assert!(geometry.inverse_shear().is_none());
            }
            Err(e) => panic!("annular source support: {}: {}", e.code, e.message),
        }
    }
    #[test]
    fn source_support_claims_do_not_authorize_geometric_lifts() {
        let model =
            crate::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
        let snapshot = format!("{model:?}");
        let original = prepare(&model, limits()).unwrap().shell.unwrap();
        let mut without_claims = model.clone();
        without_claims.0.bodies.clear();
        without_claims.0.shells[0].closed = false;
        without_claims.rebuild_topology_ids();
        let fresh = prepare(&without_claims, limits()).unwrap().shell.unwrap();
        assert_eq!(fresh.definition().unwrap(), original.definition().unwrap());
        assert_eq!(format!("{model:?}"), snapshot);
        let mut damaged = model.clone();
        let wire = damaged.faces[1].outer;
        damaged.0.loops[wire].coedges[0].pcurve.control_points[2][0] += 0.01;
        assert!(prepare(&damaged, limits()).is_err());
        let mut low = limits();
        low.faces = 26;
        assert!(prepare(&model, low).is_err());
    }
}
