use super::*;
use crate::graph_body::{
    self,
    tests::{column, limits as body_limits},
};
use nurbs_core::ss_intersection as ss;
fn limits() -> GraphLensLimits {
    GraphLensLimits {
        operations: 1000000,
        tolerance_uv: 1e-8,
        face: face::Limits {
            pairs: 10000,
            region_cells: 100000,
            domain_cells: 100000,
            agreement_cells: 100000,
        },
    }
}
fn volume_limits() -> crate::volume_validity::Limits {
    crate::volume_validity::Limits {
        boundary: crate::boundary_embedding::Limits {
            exact_work: 1000000,
            trim_pairs: 10000,
            trim_cells: 100000,
            trim_domain_cells: 1000000,
            spans: 1000,
            contacts: crate::face_contacts::Limits {
                pairs: 10000,
                cells: 100000,
                domain_cells: 1000000,
                cells_per_pair: 1000,
                domain_cells_per_pair: 10000,
            },
        },
        nesting_pairs: 100,
        nesting_cells: 100000,
        nesting_domain_cells: 1000000,
        orientation_cells: 100000,
        orientation_domain_cells: 1000000,
        orientation_spans: 100,
    }
}
#[test]
fn actual_inner_difference_sews_original_ss_edges_into_a_closed_lens() {
    let models: [Model; 2] = std::array::from_fn(|side| {
        let mut roof = crate::ss_face_membership::tests::surface(side == 0, 0.0625);
        for p in roof.control_points.iter_mut().flatten() {
            p[2] += 2.;
        }
        column(roof)
    });
    let saved = models.clone();
    let budget = body_limits();
    let bodies = [
        graph_body::qualify(&models[0], 0, 1, &budget).unwrap(),
        graph_body::qualify(&models[1], 0, 1, &budget).unwrap(),
    ];
    let sources =
        ss::FiberSources::new(&models[0].faces[0].surface, &models[1].faces[0].surface).unwrap();
    let intersection = ss::qualify_quadratic_graph_ss(&sources, Default::default()).unwrap();
    let curves = intersection
        .construct_graph_curves_with_dyadic_proposals(
            ss::GraphCurveTolerances {
                world: 1e-7,
                uv: 1e-9,
            },
            Default::default(),
        )
        .unwrap();
    let member = qualify(
        &curves,
        [(&models[0], 0), (&models[1], 0)],
        1e-8,
        Default::default(),
    )
    .unwrap();
    let closed = partition_closed_branches(&member, Default::default()).unwrap();
    let sides = qualify_closed_graph_sides(&closed, Default::default()).unwrap();
    let material =
        qualify_closed_graph_material(&sides, [&bodies[0], &bodies[1]], Default::default())
            .unwrap();
    let lens = assemble_closed_graph_lens(&material, 1, limits()).unwrap();
    assert_eq!(lens.model().faces.len(), 2);
    assert_eq!(
        lens.model().edges.len(),
        intersection.branches()[0].edges.len()
    );
    assert_eq!(
        lens.original_edges(),
        intersection.branches()[0].edges.as_slice()
    );
    assert_eq!(lens.model().vertices.len(), lens.model().edges.len());
    assert_eq!(lens.minuend(), 1);
    let mut tight = limits();
    tight.operations = lens.operations();
    assert!(assemble_closed_graph_lens(&material, 1, tight).is_ok());
    tight.operations -= 1;
    assert!(assemble_closed_graph_lens(&material, 1, tight).is_err());
    assert!(assemble_closed_graph_lens(&material, 0, limits()).is_err());
    let audit = crate::solid_audit::audit_solid(lens.model()).unwrap();
    assert!(audit.ok && audit.sew.complete);
    assert_eq!(models, saved);
    let (contact, work) = crate::shared_boundary::closed_graph::certify(lens.model(), [0, 1], 1000);
    let contact = contact.expect("original coefficient census admits the complete shared cycle");
    assert_eq!(contact.operations(), work);
    assert_eq!(contact.faces(), [0, 1]);
    assert_eq!(contact.edges().len(), lens.model().edges.len());
    assert!(
        crate::shared_boundary::closed_graph::certify(lens.model(), [0, 1], work)
            .0
            .is_some()
    );
    let exhausted = crate::shared_boundary::closed_graph::certify(lens.model(), [0, 1], work - 1);
    assert!(exhausted.0.is_none());
    assert_eq!(exhausted.1, work - 1);
    for (point, inside) in [
        ([0.5, 0.5, 1.96875], true),
        ([0.5, 0.5, 1.875], false),
        ([0.5, 0.5, 2.125], false),
        ([0.125, 0.125, 1.96875], false),
    ] {
        let parity = crate::ray_parity::classify_point(
            lens.model(),
            point,
            &[[0., 0., 1.]],
            1e-8,
            10000,
            100000,
        )
        .unwrap();
        assert_eq!(parity.parity, Some(inside), "{point:?}: {parity:?}");
    }
    let mut damaged = lens.model().clone();
    let pole = &mut damaged.0.loops[0].coedges[0].pcurve.control_points[1][0];
    *pole = pole.next_up();
    damaged.rebuild_topology_ids();
    damaged.validate().unwrap();
    let invalid =
        crate::boundary_embedding::inspect(&damaged, 1e-8, volume_limits().boundary).unwrap();
    assert!(!invalid.proven && !invalid.agreement.all_equal);
    assert!(invalid.intersections.pairs.pairs.iter().all(|p| !matches!(
        p.boundary,
        Some(crate::face_contacts::SharedBoundary::ClosedGraph(_))
    )));
    let report = lens.audit_volume(1e-8, volume_limits()).unwrap();
    assert!(
        report.report().proven,
        "boundary={}, agreement={}, intersections={}, detail={:?}",
        report.report().boundary.proven,
        report.report().boundary.agreement.all_equal,
        report.report().boundary.intersections.absence_proven,
        report.report().boundary.intersections
    );
    let admitted = report
        .into_qualified()
        .unwrap_or_else(|_| panic!("unproven result acquired volume authority"));
    assert_eq!(admitted.model().faces.len(), 2);
}
