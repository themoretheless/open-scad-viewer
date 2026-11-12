#[path = "support/exact_contact_witness.rs"]
mod exact_witness;
#[path = "support/compatible_contact_fixture.rs"]
mod fixture;
#[path = "support/natural_patch_sheet.rs"]
mod natural_sheet;
#[path = "support/expanded_contact_runout.rs"]
mod runout;
use brep_core::{face_cut, fillet_radius, region_replacement as r};
use nurbs_core::offset::fillet_surfaces::RadiusLaw;
fn main() {
    let (source, request, _, _) = fixture::fixture();
    let before = source.clone();
    let proposal = runout::build();
    let skin = &proposal.skin;
    println!(
        "expanded_proposal pieces={} poles={} storage_error={}",
        proposal.pieces,
        skin.control_points.len() * skin.control_points[0].len(),
        proposal.storage_error
    );
    let plan = r::plan_radius_network(&source, 0, &[request], 10000).unwrap();
    let supports = plan.edges[0].supports.map(|f| &source.faces[f].surface);
    if std::env::args().any(|a| a == "--exact-source-panels" || a == "--exact-source-curvature" || a == "--exact-source-brep" || a == "--exact-source-brep-split") {
        let source_brep=std::env::args().any(|a|a=="--exact-source-brep" || a=="--exact-source-brep-split");
        let check_curvature=source_brep || std::env::args().any(|a|a=="--exact-source-curvature");
        let mut budget=nurbs_core::original_graph_transition::Budget::new(if check_curvature {10_000_000}else{1_000_000}).unwrap();
        let mut exact_panels=Vec::new();let mut preview_error=0f64;
        let mut source_laws=Vec::new();
        for width in &proposal.width_segments {
            let law=nurbs_core::original_graph_transition::SourceContactCurvatureLaw::new(supports[0],width,&mut budget).unwrap();
            source_laws.push((exact_panels.len(),law));
            let patch=if std::env::args().any(|a|a=="--convex-g2"){nurbs_core::original_graph_transition::build_convex_g2(supports,width,&mut budget).unwrap()}else if std::env::args().any(|a|a=="--quadratic-core-g2"){nurbs_core::original_graph_transition::build_quadratic_core_g2(supports,width,&mut budget).unwrap()}else{nurbs_core::original_graph_transition::build(supports,width,&mut budget).unwrap()};
            assert_eq!(patch.contact_jets(),[true;2]);
            let (_display,error)=patch.preview(&mut budget).unwrap();
            preview_error=preview_error.max(error);exact_panels.push(patch);
        }
        if source_brep {
            let mut strip=brep_core::exact_graph_strip::assemble(exact_panels,1e-7,&mut budget).unwrap();
            if std::env::args().any(|a|a=="--exact-source-brep-split") {
                let edge=strip.topology().loops[0].coedges[1].edge;
                let first=strip.split_edge(edge,num_rational::BigRational::new(1.into(),3.into()),&mut budget).unwrap();
                let second=strip.split_edge(first.right_edge,num_rational::BigRational::new(1.into(),2.into()),&mut budget).unwrap();
                println!("source_owned_shared_splits root={} first={} second={} face_uses={},{}",first.root_id,first.root_parameter,second.root_parameter,first.face_uses,second.face_uses);
                let edge=strip.topology().loops[0].coedges[0].edge;
                let rim=strip.split_edge(edge,num_rational::BigRational::new(1.into(),3.into()),&mut budget).unwrap();
                println!("source_owned_rim_split root={} parameter={} face_uses={}",rim.root_id,rim.root_parameter,rim.face_uses);
            }
            for proposal in strip.source_end_splits().unwrap() {strip.apply_source_end_split(&proposal,&mut budget).unwrap();}
            let end_bindings=strip.source_end_contacts(&mut budget).unwrap();
            assert!(end_bindings.iter().all(|binding|binding.is_current(&strip)));
            println!("source_owned_original_end_contacts={} current=true exact_source_lifts=true original_second_order_contact=true",end_bindings.len());
            let bindings=strip.source_contacts(&mut budget).unwrap();
            assert!(bindings.iter().all(|binding|binding.is_current(&strip)));
            let covered=bindings.iter().map(|binding|binding.edge()).chain(end_bindings.iter().map(|binding|binding.edge())).collect::<std::collections::BTreeSet<_>>();
            assert_eq!(covered.len(),bindings.len()+end_bindings.len());
            assert_eq!(covered,strip.geometry().open_edges.iter().copied().collect());
            println!("source_owned_source_perimeter_covered={}/{}",covered.len(),strip.geometry().open_edges.len());

            println!("source_owned_original_contacts={} current=true exact_source_lifts=true",bindings.len());
            let regions=strip.retained_regions(&mut budget).unwrap();
            assert!(regions.iter().all(|region|region.is_current(&strip)));
            assert_eq!(regions[0].parameter_area(),regions[1].parameter_area());
            println!("source_owned_retained_regions=2 exact_closed_loops=true graph_domains=true uses={},{} areas={},{}",regions[0].uses().len(),regions[1].uses().len(),regions[0].parameter_area(),regions[1].parameter_area());
            assert_eq!(source,before);
            let top=strip.topology();
            let curvature=strip.fields().iter().map(|r|r.absolute_curvature_upper.unwrap()).fold(0f64,f64::max);
            println!("source_owned_brep faces={} edges={} vertices={} exact_uses={} open_edges={} g2_joins={} curvature_upper={} shared_coefficients={} closed={} bodies={}",
                top.faces.len(),top.edges.len(),top.vertices.len(),strip.geometry().boundary_uses,strip.geometry().open_edges.len(),strip.joins().len(),curvature,budget.used(),top.shells[0].closed,top.bodies.len());
            let roof=strip.sew_retained(&mut budget).unwrap();let top=roof.topology();
            let original_map=roof.bind_original_boundaries(&source,plan.edges[0].supports,&mut budget).unwrap();
            assert!(original_map.is_current(&source,&roof));
            println!("source_owned_original_edge_links={} named=true current=true outer_covered={}/{}",original_map.edges().len(),roof.geometry().open_edges.iter().filter(|edge|original_map.edges().iter().any(|b|b.roof_edge==**edge)).count(),roof.geometry().open_edges.len());

            println!("source_owned_retained_field_bounds={:?}",roof.source_fields().iter().map(|r|(r.absolute_curvature_upper,r.cells,r.shape_cells)).collect::<Vec<_>>());

            println!("source_owned_sewn_roof faces={} edges={} vertices={} exact_uses={} open_edges={} shared_coefficients={} closed={} bodies={}",top.faces.len(),top.edges.len(),top.vertices.len(),roof.geometry().boundary_uses,roof.geometry().open_edges.len(),budget.used(),top.shells[0].closed,top.bodies.len());

            let closed=roof.close_original_shell(&source,plan.edges[0].supports,&mut budget).unwrap();
            let top=closed.topology();
            assert_eq!((top.faces.len(),top.edges.len(),top.vertices.len()),(27,82,57));
            assert!(top.shells[0].closed&&top.bodies.is_empty());
            assert!(closed.geometry().open_edges.is_empty());
            assert_eq!(source,before);
            println!("source_owned_closed_shell faces={} edges={} vertices={} exact_uses={} open_edges={} imported_field_bounds={:?} shared_coefficients={} closed={} bodies={} solid_admitted=false",top.faces.len(),top.edges.len(),top.vertices.len(),closed.geometry().boundary_uses,closed.geometry().open_edges.len(),closed.imported_fields().iter().map(|r|r.absolute_curvature_upper).collect::<Vec<_>>(),budget.used(),top.shells[0].closed,top.bodies.len());
            // Control-fixture acceptance bound, derived from the ORIGINAL full
            // support hull box. This is not a user-requested volume tolerance.
            let mut box_=std::array::from_fn::<_,3,_>(|k|{let q=num_rational::BigRational::from_float(source.faces[0].surface.control_points[0][0][k]).unwrap();[q.clone(),q]});
            for p in source.faces.iter().flat_map(|f|f.surface.control_points.iter().flatten()) {for k in 0..3{let q=num_rational::BigRational::from_float(p[k]).unwrap();box_[k][0]=box_[k][0].clone().min(q.clone());box_[k][1]=box_[k][1].clone().max(q);}}
            let maximum=box_.iter().map(|r|&r[1]-&r[0]).product::<num_rational::BigRational>();
            let bounds=brep_core::exact_graph_strip::VolumeBounds::new(num_rational::BigRational::from_integer(0.into()),maximum).unwrap();
            let native=closed.admit_volume(bounds,351,&mut budget).unwrap();
            println!("source_owned_native_volume volume={} bounds=[{},{}] bodies={} outer_shell={} global_embedding={} native_volume_admitted=true fillet_radius_thickness_qualified=false application_admitted=false shared_coefficients={}",native.volume(),native.bounds().minimum(),native.bounds().maximum(),native.topology().bodies.len(),native.topology().bodies[0].outer_shell,native.qualification().embedding().globally_embedded(),budget.used());
            // Reuse the immutable proofs produced by the admission itself;
            // rerunning the same audits here wastes the shared work budget.
            let original=native.qualification().embedding();
            let retained=original.retained();let transverse=retained.transverse();let hulls=transverse.hulls();
            println!("source_owned_admitted_proofs pairs={} separated={} transverse={} retained={} original={} unresolved={:?} charts={} proofs_reused=true shared_coefficients={}",hulls.pairs().len(),hulls.separated(),transverse.contacts().len(),retained.contacts().len(),original.contacts().len(),original.unresolved(),original.charts().proven().len(),budget.used());
            assert_eq!(native.original_shell().original_source(),&before);
            let radius=native.inspect_transition_normal_radius(request.radius_mm,0.001,[[0.,1.],[0.25,0.75]],1000,&mut budget).unwrap();
            println!("source_owned_normal_radius target_mm={} tolerance_mm=0.001 domain=[[0,1],[0.25,0.75]] declared_direction_within={} reports={:?} shared_coefficients={}",request.radius_mm,radius.declared_direction_within(),radius.reports().iter().map(|(face,r)|(*face,r.decision,r.witness_radius_mm,r.witness_uv,r.coefficient_work)).collect::<Vec<_>>(),budget.used());
            if std::env::args().any(|a|a=="--convex-g2") {
                let core=native.inspect_transition_normal_radius(request.radius_mm,0.001,[[0.,1.],[0.499,0.501]],1000,&mut budget).unwrap();
                println!("source_owned_convex_core_radius target_mm={} domain=[[0,1],[0.499,0.501]] reports={:?} shared_coefficients={}",request.radius_mm,core.reports().iter().map(|(face,r)|(*face,r.decision)).collect::<Vec<_>>(),budget.used());
            }
            let laws=native.inspect_transition_source_laws(&source_laws,0.001,0.001, 0.001,[[0.,1.],[0.25,0.75]],&mut budget).unwrap();
            println!("source_owned_source_curvature_laws complete={} curvature_tolerance_inverse_mm=0.001 radial_distance_tolerance_mm=0.001 normal_chord_tolerance=0.001 reports={:?} shared_coefficients={}",laws.all_within(),laws.reports().iter().map(|(face,r)|(*face,r.decision,r.source_inverse_radius_range,r.error_inverse_mm,r.radial_distance_error_upper_mm,r.normal_chord_error_upper,r.coefficient_work)).collect::<Vec<_>>(),budget.used());
            let wall=native.inspect_aligned_wall_lower_bound(num_rational::BigRational::from_integer(2.into()),1e-6,378,&mut budget).unwrap();
            println!("source_owned_aligned_wall complete={} minimum_mm={} max_sine_squared={} products={} distance={} normal_excluded={} unresolved={:?} global_unconstrained_thickness=false attained_witness=false shared_coefficients={}",wall.complete(),wall.minimum_mm(),wall.max_sine_squared(),wall.pairs().len(),wall.pairs().iter().filter(|p|p.decision==brep_core::exact_graph_strip::AlignedWallPairDecision::DistanceLowerBound).count(),wall.pairs().iter().filter(|p|p.decision==brep_core::exact_graph_strip::AlignedWallPairDecision::OpposedNormalsExcluded).count(),wall.pairs().iter().filter(|p|p.decision==brep_core::exact_graph_strip::AlignedWallPairDecision::Unresolved).map(|p|p.faces).collect::<Vec<_>>(),budget.used());
            let witness=wall.find_axis_material_witness(&mut budget).unwrap();
            println!("source_owned_aligned_wall_witness {:?} exact_minimum={:?} unconstrained_thickness=false shared_coefficients={}",witness.as_ref().map(|w|(w.axis(),w.faces(),w.length_mm(),w.endpoint_uv(),w.endpoints(),w.excluded_faces().len())),witness.as_ref().and_then(|w|wall.exact_minimum_with(w)),budget.used());
            #[cfg(feature="codec")]
            if std::env::args().any(|a|a=="--frozen-graph-body") {
                use brep_core::exact_graph_strip::{VolumeBounds,transport::{FrozenGraphVolume,RootCut,VolumeRecipe}};
                use num_rational::BigRational as Q;
                assert!(std::env::args().any(|a|a=="--convex-g2"));
                let frozen=FrozenGraphVolume::construct(VolumeRecipe{source:source.clone(),supports:plan.edges[0].supports,widths:proposal.width_segments.clone(),cuts:vec![RootCut{root:1,parameter:Q::new(1.into(),3.into())},RootCut{root:1,parameter:Q::new(2.into(),3.into())},RootCut{root:21,parameter:Q::new(1.into(),3.into())}],tolerance_mm:1e-7,bounds:VolumeBounds::new(Q::from_integer(0.into()),Q::from_integer(40.into())).unwrap()},351,&mut budget).unwrap();
                assert_eq!(frozen.body().volume(),native.volume());
                let artifact=frozen.export(4<<20).unwrap();
                let bytes=value_codec::encode_binary(&artifact).unwrap();
                let decoded=value_codec::decode_binary(&bytes).unwrap();
                let restored=FrozenGraphVolume::restore(&decoded,4<<20,351,&mut budget).unwrap();
                assert_eq!(restored.export(4<<20).unwrap(),artifact);
                let top=restored.body().topology();
                println!("source_owned_frozen_graph_body format={} binary_bytes={} faces={} edges={} vertices={} bodies={} exact_volume_equal=true exact_geometry_equal=true all_pairs_reproved={} application_admitted=false shared_coefficients={}",brep_core::exact_graph_strip::transport::FORMAT,bytes.len(),top.faces.len(),top.edges.len(),top.vertices.len(),top.bodies.len(),restored.body().qualification().embedding().globally_embedded(),budget.used());
            }
            return;
        }
        let mut joined=0;
        for pair in exact_panels.windows(2) {
            if pair[0].joins_position_exact(&pair[1],&mut budget).unwrap() {joined+=1;}
        }
        let mut joins_g2=0;
        if check_curvature {
            for (i,pair) in exact_panels.windows(2).enumerate() {
                let report=pair[0].inspect_join_g2(&pair[1],1e-5,1e-5,&mut budget).unwrap();
                if report.within {joins_g2+=1;}
                println!("source_owned_internal_g2 join={i} report={report:?}");
            }
            println!("source_owned_internal_g2_covered={joins_g2}/{}",exact_panels.len()-1);
        }
        let mut panel_curvature_admitted=false;
        if check_curvature {
            let mut within=0;let mut upper=0f64;
            for (i,panel) in exact_panels.iter().enumerate() {
                let report=panel.inspect_curvature(10.,100_000,&mut budget).unwrap();
                println!("source_owned_curvature panel={i} report={report:?}");
                if report.decision==nurbs_core::original_graph_transition::CurvatureDecision::Within {
                    within+=1;upper=upper.max(report.absolute_curvature_upper.unwrap());
                }
            }
            println!("source_owned_curvature_covered={within}/{} upper={upper} shared_coefficients={}",exact_panels.len(),budget.used());
            panel_curvature_admitted=within==exact_panels.len();
        }
        assert_eq!(source,before);
        println!("source_owned_internal_positions={joined}/{}",exact_panels.len()-1);
        println!("source_owned_panels={} whole_contact_two_jets={} coefficient_work={} preview_error_upper={} curvature_admitted={panel_curvature_admitted} closed_body_admitted=false",
            exact_panels.len(),2*exact_panels.len(),budget.used(),preview_error);
        if check_curvature && (!panel_curvature_admitted || joins_g2+1!=exact_panels.len() || joined+1!=exact_panels.len()) {
            std::process::exit(8);
        }
        return;
    }
    let contacts = runout::contacts(skin, supports);
    if std::env::args().any(|a| a == "--native-point-lift") {
        for side in 0..2 {
            let report=nurbs_core::curve_surface_agreement::verify_exact_point(
                &contacts[side].curve,&contacts[side].support_pcurve,supports[side],false,0.5,1_000_000
            ).unwrap().unwrap();
            println!("native_point_lift side={side} normalized_t=0.5 result={:?} work={} whole_edge_admitted=false",report.point_equality,report.work_used);
        }
        return;
    }

    if std::env::args().any(|a| a == "--exact-contact-witness") {
        for side in 0..2 {
            for t in [0.25, 0.5, 1., 1.5, 1.75] {
                exact_witness::print(&contacts[side].curve, &contacts[side].support_pcurve, supports[side], side, t);
            }
        }
        return;
    }

    let middle = contacts.clone().map(|mut c| {
        c.curve = c.curve.trim(0.5, 1.5).unwrap();
        c.support_pcurve = c.support_pcurve.trim(0.5, 1.5).unwrap();
        c.fillet_pcurve = c.fillet_pcurve.trim(0.5, 1.5).unwrap();
        c
    });
    let radius = fillet_radius::inspect_contacts_in_domain(
        supports,
        skin,
        [&middle[0], &middle[1]],
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
    println!(
        "central_radius={:?} reason={} upper={:?}",
        radius.decision, radius.reason, radius.maximum_center_gap_mm
    );
    let retention = face_cut::plan_contact_retention(
        brep_core::edge_partition::contact_interval::EdgeContacts {
            source: &source,
            shell: 0,
            request,
            skin,
            contacts: [&contacts[0], &contacts[1]],
        },
        "expanded-runout",
        1e-8,
        Default::default(),
        |roots| Ok(fixture::paths(&source, request, &contacts, roots)),
    )
    .unwrap();
    let ends = r::inspect_contact_end_normals(&retention).unwrap();
    println!(
        "end_expansion_required={} roots={:?}",
        ends.needs_expansion(),
        retention.roots().root_parameters()
    );
    if radius.decision != fillet_radius::Decision::WithinTolerance || ends.needs_expansion() {
        std::process::exit(3);
    }
    if std::env::args().any(|a| a == "--graph-seams") {
        let coefficient_limit = std::env::args()
            .find_map(|a| a.strip_prefix("--graph-coefficients=").map(str::to_owned))
            .map(|a| {
                a.parse::<usize>()
                    .expect("integer graph coefficient budget")
            })
            .unwrap_or(100_000_000);
        assert!(
            coefficient_limit <= 50_000_000_000,
            "graph API budget maximum"
        );
        let mut cells = 0;
        let mut exact = 0;
        let mut coefficients = 0;
        println!(
            "graph_coefficient_limit={coefficient_limit} body_admission_limits_unchanged=true"
        );
        for side in 0..2 {
            let c = &contacts[side];
            let proof = nurbs_core::surface_g2_pcurves::inspect_owned_graph(
                &c.curve,
                [&c.support_pcurve, &c.fillet_pcurve],
                [supports[side], skin],
                [1e-7, 1e-5, 1e-4],
                100000 - cells,
                1000000 - exact,
                coefficient_limit - coefficients,
            );
            let proof = match proof {
                Ok(p) => p,
                Err(e) => {
                    println!("graph_side={side} error={e}");
                    std::process::exit(6);
                }
            };
            cells += proof.owned.report.geometric.cells;
            exact += proof.owned.report.positions.exact_work;
            coefficients += proof.coefficient_work;
            println!(
                "graph_side={side} decision={:?} cells={cells} exact={exact} coefficients={coefficients} accepted={} reason={:?}",
                proof.owned.report.geometric.decision,
                proof.accepted_cells,
                proof.last_unresolved_reason
            );
            println!(
                "graph_side={side} pending_range={:?} last_bound_failure={:?}",
                proof.pending_range, proof.last_bound_failure
            );
            if proof.owned.report.geometric.decision
                != nurbs_core::surface_join::Decision::WithinTolerance
            {
                std::process::exit(6);
            }
        }
        println!("graph_seams=true closed_body_admitted=false");
        return;
    }
    if std::env::args().any(|a| a == "--field-only") {
        let mut field_skin = skin.clone();
        for k in &mut field_skin.knots_u {
            *k *= 0.5;
        } // Exact parameter scaling, unchanged spatial surface.
        let sheet = natural_sheet::unit_sheet(&field_skin);
        let field = brep_core::face_curvature::inspect(
            &sheet,
            0,
            10.,
            1e-8,
            brep_core::face_curvature::Limits {
                cells: 50000,
                domain_cells: 1000000,
                shape_cells: 100000,
            },
            &[],
        )
        .unwrap();
        println!(
            "runout_field={:?} upper={:?} reason={} cells={} shape={} witness={:?}",
            field.decision,
            field.absolute_curvature_upper,
            field.reason,
            field.cells,
            field.shape_cells,
            field.witness_curvature
        );
        println!(
            "runout_field_cover={} omitted={} region={:?} curvature={:?}",
            field.covered_cells,
            field.omitted_cells,
            field.unresolved_region,
            field.unresolved_curvature
        );
        if field.decision != brep_core::face_curvature::Decision::Within {
            std::process::exit(4);
        }
        assert_eq!(source, before);
        if radius.decision != fillet_radius::Decision::WithinTolerance || ends.needs_expansion() {
            std::process::exit(3);
        }
        println!("field_only=true closed_body_admitted=false");
        return;
    }
    let authored = fixture::patch(&retention, skin);
    let selected = retention
        .plan()
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
    let saved_global = std::env::args().any(|a| a == "--saved-global-audit");
    let saved_graph = saved_global || std::env::args().any(|a| a == "--saved-graph-seams");
    let local = if saved_graph {
        r::sew_g2_patch_set_with_graph_work(
            &retention.plan().model,
            r::PatchReplacement {
                selected: &selected,
                new_vertices: &authored.vertices,
                new_edges: &authored.edges,
                patches: &patches,
            },
            "expanded-runout-local",
            1e-8,
            Default::default(),
            1_000_000_000,
        )
    } else {
        r::sew_g2_patch_set(
            &retention.plan().model,
            r::PatchReplacement {
                selected: &selected,
                new_vertices: &authored.vertices,
                new_edges: &authored.edges,
                patches: &patches,
            },
            "expanded-runout-local",
            1e-8,
            Default::default(),
        )
    };
    let local_body = match local {
        Ok(body) => {
            println!(
                "saved_seam_coefficients={} cells={} exact={}",
                body.seam_coefficient_work(),
                body.work().0,
                body.work().1
            );
            println!(
                "local_closed=true faces={} seams={} curvature={:?}",
                body.model().faces.len(),
                body.seams().seams.len(),
                body.fields()
                    .iter()
                    .filter_map(|f| f.absolute_curvature_upper)
                    .reduce(f64::max)
            );
            body
        }
        Err(e) => {
            println!("local_unproven={e}");
            std::process::exit(2);
        }
    };
    if saved_graph {
        assert_eq!(source, before);
        if saved_global {
            let limits = global_limits();
            let report = brep_core::boundary_embedding::inspect_with_linear(
                local_body.model(),
                1e-8,
                limits.global,
                limits.linear_cells,
            )
            .unwrap();
            println!(
                "saved_global proven={} exact_lifts={} exact_joins={} trim={} injective={} classified={} pairs={}/{} next={:?}",
                report.proven,
                report.agreement.all_equal,
                report.agreement.all_joins_exact,
                report.trim.all_valid,
                report.intersections.faces.all_faces_injective,
                report.intersections.pairs.all_pairs_classified,
                report.intersections.pairs.pairs.len(),
                report.intersections.pairs.total_pairs,
                report.intersections.pairs.next_pair
            );
            let unequal = report
                .agreement
                .uses
                .iter()
                .filter(|u| {
                    u.decision
                        .as_ref()
                        .is_none_or(|d| d.outcome != cad_predicates::BezierIdentity::Equal)
                })
                .take(4)
                .collect::<Vec<_>>();
            println!("saved_global_agreement_frontier={unequal:?}");
            println!("saved_global_exact_counterexample={}", report.agreement.has_exact_counterexample());
            for pair in report
                .intersections
                .pairs
                .pairs
                .iter()
                .filter(|p| {
                    p.boundary.is_none()
                        && !p.hull_disjoint
                        && p.volume_separation.is_none()
                        && p.result.as_ref().is_none_or(|r| !r.absence_proven)
                })
                .take(8)
            {
                println!("saved_global_pair={:?} reason={}", pair.faces, pair.reason);
            }
            println!(
                "saved_global_unproven_faces={:?}",
                report
                    .intersections
                    .faces
                    .faces
                    .iter()
                    .filter(|f| !f.absence_proven())
                    .map(|f| f.face)
                    .collect::<Vec<_>>()
            );
            if !report.proven {
                std::process::exit(5);
            }
            println!("saved_global_audit=true contact_body_transaction_complete=false");
            return;
        }
        println!("local_saved_graph=true global_embedding_admitted=false");
        return;
    }
    assert_eq!(source, before);
    if radius.decision != fillet_radius::Decision::WithinTolerance || ends.needs_expansion() {
        std::process::exit(3);
    }
    let limits = global_limits();
    match r::sew_contact_g2_patch_set(retention, "expanded-runout-global", 1e-8, limits, |p| {
        Ok(fixture::patch(p, skin))
    }) {
        Ok(body) => println!(
            "global_admitted=true faces={} pairs={}",
            body.model().faces.len(),
            body.global().intersections.pairs.total_pairs
        ),
        Err(r::ContactPatchFailure::Unproven(report)) => {
            println!(
                "global_unproven exact_lifts={} trims={} injective={} classified={}",
                report.agreement.all_equal,
                report.trim.all_valid,
                report.intersections.faces.all_faces_injective,
                report.intersections.pairs.all_pairs_classified
            );
            std::process::exit(5);
        }
        Err(e) => {
            println!("global_unproven={e}");
            std::process::exit(5);
        }
    }
}

fn global_limits() -> r::EmbeddedPatchLimits {
    r::EmbeddedPatchLimits {
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
    }
}
