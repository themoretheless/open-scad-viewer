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
