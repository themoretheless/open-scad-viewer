#[path = "../examples/support/nurbs_contact_stock.rs"]
mod fixture;
use brep_core::region_replacement as r;
#[test]
fn trim_original_rational_supports_and_sew_one_closed_blend() {
    let f = fixture::build();
    let before = f.prepared.clone();
    let cuts = f
        .cuts
        .iter()
        .map(|c| r::ContactCut {
            face: c.face,
            endpoints: c.endpoints,
            segments: &c.segments,
            replacement_pcurves: &c.replacement,
            remove: c.anchor,
            smooth: c.smooth,
        })
        .collect::<Vec<_>>();
    let out =
        r::trim_and_replace_contacts(&f.prepared, &cuts, &f.blend, true, 1e-8, 100_000, 1_000_000)
            .unwrap();
    assert_eq!(f.prepared, before);
    assert_eq!(out.replacement.source_faces.len(), f.prepared.faces.len());
    assert_eq!(out.replacement.source_edges.len(), f.prepared.edges.len());
    assert_eq!(
        out.replacement.source_vertices.len(),
        f.prepared.vertices.len()
    );
    for (original, kept) in out.replacement.source_faces.iter().enumerate() {
        assert_eq!(
            out.replacement.model.faces[kept.unwrap()].surface,
            f.prepared.faces[original].surface
        );
    }
    assert_eq!(out.replacement.model.faces.len(), 7);
    assert_eq!(out.replacement.model.edges.len(), 15);
    assert_eq!(out.replacement.model.vertices.len(), 10);
    assert_eq!(out.smooth_edges.len(), 2);
    assert_eq!(out.retained_supports.len(), 4);
    assert_eq!(
        out.replacement.closure.closure.status,
        brep_core::watertight::Closure::WithinTolerance
    );
    for (id, face) in &out.retained_supports {
        let old = f.prepared.1.faces.iter().position(|x| x == id).unwrap();
        assert_eq!(
            out.replacement.model.faces[*face].surface,
            f.prepared.faces[old].surface
        );
    }
    for original in &f.source.faces {
        assert!(
            out.replacement
                .model
                .faces
                .iter()
                .any(|face| face.surface == original.surface)
        );
    }
}

#[test]
fn sewn_contacts_have_whole_original_support_g2_curvature_and_global_embedding() {
    let f = fixture::build();
    let cuts = f.cuts.iter().map(|c| r::GraphContactCut {
        face:c.face,endpoints:c.endpoints,segments:&c.segments,remove:c.anchor,smooth:c.smooth,
    }).collect::<Vec<_>>();
    let graph=r::CentralGraph {surface:f.blend.clone(),pcurves:f.cuts.iter().flat_map(|c|c.replacement.clone()).collect(),
        // Constructor residuals cannot authorize sewing or any fresh G2 check.
        sampled_jet_residual:[f64::NAN;6]};
    let out=r::trim_and_replace_graph_contacts(&f.prepared,&cuts,&graph,true,1e-8,100_000,1_000_000).unwrap();
    let model = &out.replacement.model;
    for &edge in &out.smooth_edges {
        let mut uses = Vec::new();
        for (i, face) in model.faces.iter().enumerate() {
            for c in &model.loops[face.outer].coedges {
                if c.edge == edge {
                    uses.push((
                        i,
                        if c.reversed {
                            c.pcurve.reverse().unwrap()
                        } else {
                            c.pcurve.clone()
                        },
                    ));
                }
            }
        }
        assert_eq!(uses.len(), 2);
        let proof = nurbs_core::surface_g2_pcurves::inspect_owned(nurbs_core::surface_g2_pcurves::PcurveSeam { curve: &model.edges[edge].curve, pcurves: [&uses[0].1, &uses[1].1], surfaces: [
                &model.faces[uses[0].0].surface,
                &model.faces[uses[1].0].surface,
            ], reversed: [false; 2] }, nurbs_core::surface_g2_pcurves::G2PcurveTolerance { position: 1e-7, normal: 1e-5, curvature: 1e-4 }, 100_000, 1_000_000)
        .unwrap();
        assert!(proof.report.positions.proven);
        assert_eq!(
            proof.report.geometric.decision,
            nurbs_core::surface_join::Decision::WithinTolerance
        );
    }
    for face in 0..model.faces.len() {
        let proof = brep_core::face_curvature::inspect(
            model,
            face,
            10.,
            1e-8,
            brep_core::face_curvature::Limits {
                cells: 50_000,
                domain_cells: 1_000_000,
                shape_cells: 1_000_000,
            },
            &[],
        )
        .unwrap();
        assert_eq!(
            proof.decision,
            brep_core::face_curvature::Decision::Within,
            "face {face}"
        );
    }
    let retained = brep_core::boundary_embedding::inspect_retained_rectangles(
        model,
        1e-8,
        brep_core::boundary_embedding::Limits {
            exact_work: 1_000_000,
            trim_pairs: 10_000,
            trim_cells: 100_000,
            trim_domain_cells: 1_000_000,
            spans: 1024,
            contacts: brep_core::face_contacts::Limits {
                pairs: 1000,
                cells: 100_000,
                domain_cells: 1_000_000,
                cells_per_pair: 1000,
                domain_cells_per_pair: 10_000,
            },
        },
        100_000,
        100_000,
    )
    .unwrap();
    let global = retained.proof;
    assert!(!retained.restrictions.is_empty());
    assert!(
        global.proven,
        "exact={} joins={} trims={} injective={} pairs={:?}",
        global.agreement.all_equal,
        global.agreement.all_joins_exact,
        global.trim.all_valid,
        global.intersections.faces.all_faces_injective,
        global
            .intersections
            .pairs
            .pairs
            .iter()
            .map(|p| (p.faces, p.reason))
            .collect::<Vec<_>>()
    );
}
#[test]
fn contradictory_contact_or_exhausted_work_cannot_publish_partial_sectors() {
    let f = fixture::build();
    let before = f.prepared.clone();
    let mut cuts = f
        .cuts
        .iter()
        .map(|c| r::ContactCut {
            face: c.face,
            endpoints: c.endpoints,
            segments: &c.segments,
            replacement_pcurves: &c.replacement,
            remove: c.anchor,
            smooth: c.smooth,
        })
        .collect::<Vec<_>>();
    assert!(
        r::trim_and_replace_contacts(&f.prepared, &cuts, &f.blend, true, 1e-8, 8, 1_000_000)
            .is_err()
    );
    let mut wrong = f.blend.clone();
    wrong.control_points[0][0][0] += 0.01;
    assert!(
        r::trim_and_replace_contacts(&f.prepared, &cuts, &wrong, true, 1e-8, 100_000, 1_000_000)
            .is_err()
    );
    cuts[1].remove = r::RemovalAnchor::Vertex(cuts[1].endpoints[0]);
    assert!(
        r::trim_and_replace_contacts(&f.prepared, &cuts, &f.blend, true, 1e-8, 100_000, 1_000_000)
            .is_err()
    );
    assert_eq!(f.prepared, before);
}
#[test]
fn graph_sewing_refuses_missing_contacts_and_changed_skin_atomically() {
    let f=fixture::build();let before=f.prepared.clone();
    let cuts=f.cuts.iter().map(|c|r::GraphContactCut{face:c.face,endpoints:c.endpoints,segments:&c.segments,remove:c.anchor,smooth:c.smooth}).collect::<Vec<_>>();
    let mut graph=r::CentralGraph{surface:f.blend.clone(),pcurves:f.cuts.iter().flat_map(|c|c.replacement.clone()).collect(),sampled_jet_residual:[0.;6]};
    graph.pcurves.pop();
    assert!(r::trim_and_replace_graph_contacts(&f.prepared,&cuts,&graph,true,1e-8,100000,1000000).is_err());
    graph.pcurves=f.cuts.iter().flat_map(|c|c.replacement.clone()).collect();
    for row in &mut graph.surface.control_points {for p in row {p[2]+=1.;}}
    assert!(r::trim_and_replace_graph_contacts(&f.prepared,&cuts,&graph,true,1e-8,100000,1000000).is_err());
    assert_eq!(f.prepared,before);
}
