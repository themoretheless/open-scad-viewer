use super::*;
use crate::region_replacement::{EdgeRadius, StripLimits, build_trihedral_strips};
fn strip_limits() -> StripLimits {
    StripLimits {
        tolerances: [1e-7, 1e-5, 1e-4],
        tolerance_uv: 1e-8,
        max_geometry_work: 10000,
        max_cells: 300000,
        max_exact: 1000000,
        max_coefficients: 2000000,
    }
}
fn options() -> CentralLoftOptions {
    CentralLoftOptions {
        handle_mm: 1. / 256.,
        tolerances: [1e-7, 1e-5, 1e-4],
        tolerance_uv: 1e-8,
        operations: 1000000,
        cells: 100000,
    }
}
#[test]
fn unequal_radius_center_preserves_the_whole_rim_and_stays_inside_original_stock() {
    let source = crate::cuboid([0.; 3], [10.; 3]).unwrap();
    let saved = source.clone();
    let graph = crate::graph_body::tests::limits();
    let budget = crate::source_box::Limits {
        operations: 1000000,
        body: graph.body,
    };
    let body = crate::source_box::qualify(&source, &budget).unwrap();
    let requested: Vec<_> = source
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| e.vertices.contains(&0))
        .enumerate()
        .map(|(i, (edge, _))| EdgeRadius {
            edge,
            radius_mm: [0.1, 0.2, 0.35][i],
        })
        .collect();
    let strips =
        build_trihedral_strips(
            crate::region_replacement::TrihedralCorner {
                source: &source,
                shell: 0,
                vertex: 0,
                requested: &requested,
                runout_mm: 0.1,
                span_mm: 2.,
            },
            strip_limits(),
        ).unwrap();
    let candidate = propose_trihedral_center(&body, &strips, options()).unwrap();
    let network=candidate.propose_common_denominator_patches(1000000).unwrap();
    let reports=candidate.inspect_network_exterior_jets(&network,6000,100000).unwrap();
    let specialized=candidate.inspect_network_exterior_support_jets(&network,6000,100000).unwrap();
    assert_eq!(specialized.len(),reports.len());
    assert!(specialized.iter().all(|r|r.report.as_ref().map_or(true,|p|p.cells<=6000/reports.len())));

    for i in 0..6 {assert!(reports.iter().any(|r|r.boundary==i));}
    assert!(reports.iter().all(|r|r.parameter[0]<r.parameter[1]));
    assert!(candidate.inspect_network_exterior_jets(&network[1..],6000,100000).is_err());
    let mut overlap=network.clone();overlap.push(network[0].clone());
    assert!(candidate.inspect_network_exterior_jets(&overlap,6000,100000).is_err());

    assert_eq!(candidate.curves().len(), 6);
    assert_eq!(candidate.pcurves().len(), 6);
    assert_eq!(candidate.positions().len(), 12);
    assert!(
        candidate
            .positions()
            .iter()
            .all(|r| r.status == agreement::Status::WithinTolerance)
    );
    assert_eq!(candidate.stock_bounds(), [[0., 10.]; 3]);
    let actual = candidate.control_bounds();
    assert!(actual.iter().all(|b| b[0] >= 0. && b[1] <= 10.));
    let s = candidate.surface();
    assert!(s.control_points.len() * s.control_points[0].len() <= 4096);
    let mut cached = nurbs_core::surface::SurfaceEvaluator::new(s).unwrap();
    for u in 0..21 {
        for v in 0..21 {
            let p = s.evaluate(u as f64 / 20., v as f64 / 20.).unwrap().point;
            assert!(
                p.iter().all(|&x| x >= 0. && x <= 10.),
                "u={u} v={v} p={p:?}"
            );
            let q = cached
                .evaluate(u as f64 / 20., v as f64 / 20.)
                .unwrap()
                .point;
            assert!(
                q.iter().all(|&x| x >= 0. && x <= 10.),
                "cached u={u} v={v} q={q:?}"
            );
        }
    }
    // The two pole boundaries are declared, not passed off as regular charts.
    assert!(
        s.control_points[0]
            .iter()
            .all(|p| p == &s.control_points[0][0])
    );
    assert!(
        s.control_points
            .last()
            .unwrap()
            .iter()
            .all(|p| p == &s.control_points.last().unwrap()[0])
    );
    let seams = candidate.inspect_original_g2(6000, 100000).unwrap();
    assert_eq!(seams.len(), 6);
    assert!(
        seams
            .iter()
            .map(|p| p.report.geometric.cells)
            .sum::<usize>()
            <= 6000
    );
    assert!(
        seams
            .iter()
            .map(|p| p.report.positions.exact_work)
            .sum::<u64>()
            <= 100000
    );
    for (i, proof) in seams.iter().enumerate() {
        eprintln!(
            "central original seam {i}: positions={} geometric={:?}",
            proof.report.positions.proven, proof.report.geometric
        );
    }
    super::span_diagnostics::inspect(&candidate);
    let fields = candidate
        .inspect_original_span_fields(6000, 100000)
        .unwrap();
    assert_eq!(fields.len(), 6);
    assert_eq!(
        fields[1].decision,
        nurbs_core::surface_g2_pcurves::FieldDecision::Enclosed
    );
    assert!(fields[1].one_sided_regular && fields[1].pending == 0);
    assert!(fields.iter().all(|r| r.positions.proven));
    assert!(fields.iter().map(|r| r.cells).sum::<usize>() <= 6000);
    assert!(fields.iter().map(|r| r.positions.exact_work).sum::<u64>() <= 100000);
    for (i, r) in fields.iter().enumerate() {
        eprintln!(
            "central whole original span fields {i}: positions={} decision={:?} regular={} pending={} cells={} branches={:?}",
            r.positions.proven, r.decision, r.one_sided_regular, r.pending, r.cells, r.branches
        );
    }

    let internal = candidate
        .inspect_internal_span_fields(6000, 100000)
        .unwrap();
    assert!(!internal.is_empty());
    assert!(internal.iter().all(|(_, r)| r.positions.proven));
    assert!(internal.iter().map(|(_, r)| r.cells).sum::<usize>() <= 6000);
    assert!(
        internal
            .iter()
            .map(|(_, r)| r.positions.exact_work)
            .sum::<u64>()
            <= 100000
    );
    for (u, r) in internal {
        eprintln!(
            "central internal u={u}: decision={:?} regular={} pending={} cells={} branches={:?}",
            r.decision, r.one_sided_regular, r.pending, r.cells, r.branches
        );
    }
    let joins = candidate.inspect_internal_span_joins(6000).unwrap();
    assert_eq!(joins.len(), 4);
    assert!(joins.iter().all(|(_, r)| r.decision
        == nurbs_core::surface_join::Decision::WithinTolerance
        && r.regularity_proven
        && r.pending == 0
        && r.normal[1] <= options().tolerances[1]
        && r.tensor[1] <= options().tolerances[2]));
    assert!(joins.iter().map(|(_, r)| r.cells).sum::<usize>() <= 6000);
    for (u, r) in joins {
        eprintln!(
            "central correlated internal u={u}: decision={:?} regular={} pending={} cells={} normal={:?} tensor={:?}",
            r.decision, r.regularity_proven, r.pending, r.cells, r.normal, r.tensor
        );
    }
    let poles = candidate.inspect_pole_limits(1. / 64., 6000).unwrap();
    assert_eq!(poles.len(), 2);
    assert!(
        poles
            .iter()
            .all(|p| p.report.proven && p.report.quotient_proven && p.report.differential_proven)
    );
    assert!(poles.iter().map(|p| p.report.cells).sum::<usize>() <= 6000);
    for (i, p) in poles.iter().enumerate() {
        eprintln!(
            "central pole {i}: proven={} differential={} quotient={} reason={} axis={} sign={:?} cells={} angle={:?} radius={:?} lower={:?} normal-coeff={:?} tensor-coeff={:?}",
            p.report.proven,
            p.report.differential_proven,
            p.report.quotient_proven,
            p.report.reason,
            p.report.normal_axis,
            p.report.normal_sign,
            p.report.cells,
            p.report.fan_angle_upper,
            p.report.reference_radius,
            p.report.quotient_lower,
            p.report.normal_coefficient,
            p.report.tensor_coefficient
        );
        if let Some(c) = &p.certificate {
            c.reverify().unwrap();
        } else {
            let axis = p.report.normal_axis;
            let n = s.control_points.len();
            let heights: Vec<Vec<_>> = (0..3)
                .map(|j| {
                    s.control_points[if i == 0 { j } else { n - 1 - j }]
                        .iter()
                        .map(|x| x[axis])
                        .collect()
                })
                .collect();
            eprintln!("central pole {i} original heights: {heights:?}");
        }
    }
    let exterior = candidate.inspect_exterior_jets(6000, 100000).unwrap();
    assert_eq!(exterior.len(), 6);
    for (i, r) in exterior.iter().enumerate() {
        eprintln!(
            "central exterior {i}: decision={:?} orientation={:?} regular={} cells={} pending={} normal={:?} tensor={:?} G0={}",
            r.decision,
            r.orientation,
            r.regularity_proven,
            r.cells,
            r.pending,
            r.normal,
            r.tensor,
            r.positions.proven
        );
    }
    assert!(exterior.iter().all(|r| r.decision
        == nurbs_core::surface_join::Decision::WithinTolerance
        && r.regularity_proven
        && r.pending == 0));
    candidate.reverify().unwrap();
    let mut exact = options();
    exact.operations = candidate.operations();
    assert!(propose_trihedral_center(&body, &strips, exact).is_ok());
    exact.operations -= 1;
    assert!(propose_trihedral_center(&body, &strips, exact).is_err());
    let mut outside = options();
    outside.handle_mm = 100.;
    assert!(propose_trihedral_center(&body, &strips, outside).is_err());
    assert_eq!(source, saved);
}

#[test]
fn cyclic_rim_origins_preserve_owned_source_curves_and_reverify_their_own_chart() {
    let source=crate::cuboid([0.;3],[10.;3]).unwrap();let before=source.clone();
    let limits=crate::source_box::Limits {operations:1000000,body:crate::graph_body::tests::limits().body};
    let body=crate::source_box::qualify(&source,&limits).unwrap();
    let requested:Vec<_>=source.edges.iter().enumerate().filter(|(_,e)|e.vertices.contains(&0)).enumerate()
        .map(|(i,(edge,_))|EdgeRadius {edge,radius_mm:[0.1,0.2,0.35][i]}).collect();
    let strips=build_trihedral_strips(
        crate::region_replacement::TrihedralCorner {
            source: &source,
            shell: 0,
            vertex: 0,
            requested: &requested,
            runout_mm: 0.1,
            span_mm: 2.,
        },
        strip_limits(),
    ).unwrap();
    let canonical=propose_trihedral_center(&body,&strips,options()).unwrap();
    for origin in 0..3 {
        let c=propose_trihedral_center_at_rim(&body,&strips,options(),origin).unwrap();
        for i in 0..6 {assert_eq!(c.curves()[i],canonical.curves()[(i+2*origin)%6]);}
        assert!(c.positions().iter().all(|r|r.status==agreement::Status::WithinTolerance));
        c.reverify().unwrap();
        if origin==0 {assert_eq!(c.surface(),canonical.surface());}
    }
    assert!(propose_trihedral_center_at_rim(&body,&strips,options(),3).is_err());
    assert_eq!(source,before);
}

#[test]
fn quartic_rim_chart_changes_retain_full_original_boundary_positions() {
    let source=crate::cuboid([0.;3],[10.;3]).unwrap();let before=source.clone();
    let limits=crate::source_box::Limits {operations:1000000,body:crate::graph_body::tests::limits().body};
    let body=crate::source_box::qualify(&source,&limits).unwrap();
    let requested:Vec<_>=source.edges.iter().enumerate().filter(|(_,e)|e.vertices.contains(&0)).enumerate()
        .map(|(i,(edge,_))|EdgeRadius {edge,radius_mm:[0.1,0.2,0.35][i]}).collect();
    let strips=crate::region_replacement::build_trihedral_strips_with_quartic_profile(
        crate::region_replacement::TrihedralCorner {
            source: &source,
            shell: 0,
            vertex: 0,
            requested: &requested,
            runout_mm: 0.1,
            span_mm: 2.,
        },
        strip_limits(),
        10.,
    ).unwrap();
    let canonical=propose_trihedral_center(&body,&strips,options()).unwrap();
    for origin in 0..3 {
        let c=propose_trihedral_center_at_rim(&body,&strips,options(),origin).unwrap();
        assert_eq!(c.positions().len(),12);
        assert!(c.positions().iter().all(|r|r.status==agreement::Status::WithinTolerance));
        for i in 0..6 {assert_eq!(c.curves()[i],canonical.curves()[(i+2*origin)%6]);}
        c.reverify().unwrap();
    }
    assert_eq!(source,before);
}

#[test]
fn actual_unequal_radius_cyclic_network_proves_all_four_internal_original_g2_seams() {
    let source=crate::cuboid([0.;3],[10.;3]).unwrap();let before=source.clone();
    let limits=crate::source_box::Limits {operations:1000000,body:crate::graph_body::tests::limits().body};
    let body=crate::source_box::qualify(&source,&limits).unwrap();
    let requested:Vec<_>=source.edges.iter().enumerate().filter(|(_,e)|e.vertices.contains(&0)).enumerate()
        .map(|(i,(edge,_))|EdgeRadius {edge,radius_mm:[0.1,0.2,0.35][i]}).collect();
    let strips=crate::region_replacement::build_trihedral_strips_with_quartic_profile(
        crate::region_replacement::TrihedralCorner {
            source: &source,
            shell: 0,
            vertex: 0,
            requested: &requested,
            runout_mm: 0.1,
            span_mm: 2.,
        },
        strip_limits(),
        10.,
    ).unwrap();
    let c=propose_trihedral_center_at_rim(&body,&strips,options(),1).unwrap();
    let net=c.propose_curvature_search_patches(1000000,50000,6).unwrap();
    assert_eq!(net.len(),5);let mut cells=0;
    for pair in net.windows(2) {
        assert_eq!(pair[0].control_points.last(),pair[1].control_points.first());
        let p=nurbs_core::surface_g2_pcurves::inspect_original_patch_pair(&pair[0],&pair[1],1e-5,1e-4,6000).unwrap();
        assert_eq!(p.decision,nurbs_core::surface_join::Decision::WithinTolerance);
        assert!(p.regularity_proven && p.pending==0 && p.normal[1]<=1e-5 && p.tensor[1]<=1e-4);
        cells+=p.cells;
    }
    assert!(cells<=24000);c.reverify().unwrap();assert_eq!(source,before);
    // Only internal G2 is asserted. This candidate has no whole-field curvature,
    // exterior-contact, material, volume, closed-body, STEP or UI certificate.
}

#[test]
fn actual_degree9_network_assembles_one_open_shell_with_owned_shared_roots() {
    let source=crate::cuboid([0.;3],[10.;3]).unwrap();let before=source.clone();
    let limits=crate::source_box::Limits {operations:1000000,body:crate::graph_body::tests::limits().body};
    let body=crate::source_box::qualify(&source,&limits).unwrap();
    let requested:Vec<_>=source.edges.iter().enumerate().filter(|(_,e)|e.vertices.contains(&0)).enumerate()
        .map(|(i,(edge,_))|EdgeRadius {edge,radius_mm:[0.1,0.2,0.35][i]}).collect();
    let strips=crate::region_replacement::build_trihedral_strips_with_quartic_profile(
        crate::region_replacement::TrihedralCorner {
            source: &source,
            shell: 0,
            vertex: 0,
            requested: &requested,
            runout_mm: 0.1,
            span_mm: 2.,
        },
        strip_limits(),
        10.,
    ).unwrap();
    let mut o=options();o.handle_mm=0.03125;
    let c=propose_trihedral_center_at_rim(&body,&strips,o,1).unwrap();
    let net=c.propose_joint_seam_degree9_patches(1000000,95000,3).unwrap();
    let saved=net.clone();let shell=c.assemble_open_network(&net,100000,1000000).unwrap();
    let m=shell.model();assert_eq!((m.faces.len(),m.edges.len(),m.vertices.len()),(5,16,10));
    assert_eq!(shell.shared_edges().len(),4);assert_eq!(shell.support_binding_proposals().len(),10);
    assert_eq!(shell.support_agreements().len(),10);
    assert!(shell.support_agreements().iter().all(|p|p.status==nurbs_core::curve_surface_agreement::Status::WithinTolerance));

    assert!(m.bodies.is_empty() && m.shells.len()==1 && !m.shells[0].closed);
    assert!(shell.cells<=100000);m.validate().unwrap();
    for binding in shell.support_binding_proposals() {
        let proof=nurbs_core::curve_surface_agreement::verify(&m.edges[binding.edge].curve,&binding.pcurve,&binding.support,false,1e-7,10000).unwrap();
        assert_eq!(proof.status,nurbs_core::curve_surface_agreement::Status::WithinTolerance,"boundary {} patch {}: {proof:?}",binding.boundary,binding.patch);
    }

    for &edge in shell.shared_edges() {
        let uses=m.loops.iter().flat_map(|l|&l.coedges).filter(|c|c.edge==edge).collect::<Vec<_>>();
        assert_eq!(uses.len(),2);assert_ne!(uses[0].reversed,uses[1].reversed);
    }
    for edge in shell.pole_edges() {assert!(m.edges[edge].degenerate);assert_eq!(m.edges[edge].vertices[0],m.edges[edge].vertices[1]);}
    assert!(c.assemble_open_network(&net[1..],100000,1000000).is_err());
    assert!(c.assemble_open_network(&net,100,100).is_err());
    let mut changed=net.clone();changed[1].control_points[0][3][0]+=1e-3;
    assert!(c.assemble_open_network(&changed,100000,1000000).is_err());
    assert_eq!(net,saved);assert_eq!(source,before);
}

#[test]
fn actual_network_closed_stock_candidate_keeps_original_supports_and_cap_refusal() {
    let source=crate::cuboid([0.;3],[10.;3]).unwrap();let before=source.clone();
    let limits=crate::source_box::Limits {operations:1000000,body:crate::graph_body::tests::limits().body};
    let stock=crate::source_box::qualify(&source,&limits).unwrap();
    let requested:Vec<_>=source.edges.iter().enumerate().filter(|(_,e)|e.vertices.contains(&0)).enumerate()
        .map(|(i,(edge,_))|EdgeRadius {edge,radius_mm:[0.1,0.2,0.35][i]}).collect();
    let strips=crate::region_replacement::build_trihedral_strips_with_quartic_profile(
        crate::region_replacement::TrihedralCorner {
            source: &source,
            shell: 0,
            vertex: 0,
            requested: &requested,
            runout_mm: 0.1,
            span_mm: 2.,
        },
        strip_limits(),
        10.,
    ).unwrap();
    let mut o=options();o.handle_mm=0.03125;
    let c=propose_trihedral_center_at_rim(&stock,&strips,o,1).unwrap();
    let net=c.propose_joint_seam_degree9_patches(1000000,95000,3).unwrap();
    let body=c.sew_network_candidate(&net,CornerSewLimits {cells:100000,exact_work:1000000,trim_pairs:10000,domain_cells:100000}).unwrap();
    let m=body.model();m.validate().unwrap();
    assert_eq!(body.final_jets().len(),10);
    assert!(body.final_jets().iter().all(|p|p.pending==0 && p.regularity_proven && p.decision==nurbs_core::surface_join::Decision::WithinTolerance));

    assert_eq!((m.faces.len(),m.edges.len(),m.vertices.len()),(17,43,26));
    assert_eq!(m.bodies.len(),1);assert!(m.shells[0].closed && body.trims().all_valid);
    assert_eq!(body.closure().closure.status,crate::watertight::Closure::WithinTolerance);
    for (i,f) in source.faces.iter().enumerate() {assert_eq!(m.faces[i].surface,f.surface);}
    assert_eq!(source,before);
    // Closed incidence alone MUST NOT admit this curvature-violating shape.
    let uv=[0.5285379208722983,0.125];
    let r=nurbs_core::surface_shape_operator::rectangle_bounds(&m.faces[body.central_faces()[2]].surface,[[uv[0];2],[uv[1];2]],8).unwrap();
    let k=nurbs_core::surface_shape_operator::absolute_spectral_bounds(r.shape.tensor.unwrap()).unwrap().unwrap();
    assert!(k[0]>10.);
    let audit=body.inspect_one_sided_curvature(1e-8,crate::face_curvature::Limits {cells:5000,domain_cells:100000,shape_cells:100000}).unwrap();
    assert!(!audit.within_limit);
    assert_eq!(audit.faces.len(),17);
    assert!(audit.faces.iter().any(|f|f.face==body.central_faces()[2] && f.report.decision==crate::face_curvature::Decision::Exceeds && f.report.witness_curvature.is_some_and(|w|w[0]>10.)));
    let zero_audit=body.inspect_curvature(1e-8,crate::face_curvature::Limits {cells:0,domain_cells:100000,shape_cells:100000}).unwrap();
    assert!(!zero_audit.complete && !zero_audit.within_limit && zero_audit.next_face==Some(0));
}
