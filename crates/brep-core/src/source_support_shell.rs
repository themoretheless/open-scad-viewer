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
        eprintln!("annular original exact weight-profile proposal axis upper {:?} weight upper {}",profile.axis_deviation_upper(),profile.weight_deviation_upper());
        assert!(nurbs_core::normal_alignment::qualify_weight_profile(source,[5,0],17).unwrap().0.is_none());

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
