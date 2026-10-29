use super::*;
use crate::graph_body::{
    self,
    tests::{column, limits as body_limits},
};
use nurbs_core::ss_intersection as ss;
fn limits() -> GraphEnvelopeLimits {
    GraphEnvelopeLimits {
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
fn actual_union_and_intersection_retain_original_walls_and_material_under_operand_swap() {
    for swapped in [false, true] {
        let models: [Model; 2] = std::array::from_fn(|side| {
            let mut roof = crate::ss_face_membership::tests::surface((side == 0) ^ swapped, 0.0625);
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
            ss::FiberSources::new(&models[0].faces[0].surface, &models[1].faces[0].surface)
                .unwrap();
        let original = ss::qualify_quadratic_graph_ss(&sources, Default::default()).unwrap();
        let curves = original
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
        for operation in [
            GraphEnvelopeOperation::Union,
            GraphEnvelopeOperation::Intersection,
        ] {
            let result = assemble_closed_graph_envelope(&material, operation, limits()).unwrap();
            let union = operation == GraphEnvelopeOperation::Union;
            let outer = usize::from(swapped ^ !union);
            assert_eq!(result.outer_side(), outer);
            assert_eq!(result.operation(), operation);
            assert_eq!(result.model().faces.len(), 7);
            assert_eq!(result.model().edges.len(), 16);
            assert_eq!(result.model().vertices.len(), 12);
            assert_eq!(result.model().faces[0].holes.len(), 1);
            for original_edge in &models[outer].edges {
                assert_eq!(
                    result
                        .model()
                        .edges
                        .iter()
                        .filter(|e| e.curve == original_edge.curve)
                        .count(),
                    1
                );
            }
            if union && !swapped {
                let (contact, work) =
                    crate::shared_boundary::closed_graph::certify(result.model(), [0, 1], 1000);
                let contact = contact.expect("whole shared ellipse includes the actual hole cycle");
                assert_eq!(contact.edges().len(), 4);
                assert_eq!(contact.operations(), work);
                assert!(
                    crate::shared_boundary::closed_graph::certify(result.model(), [0, 1], work)
                        .0
                        .is_some()
                );
                assert!(
                    crate::shared_boundary::closed_graph::certify(result.model(), [0, 1], work - 1)
                        .0
                        .is_none()
                );
                let mut damaged = result.model().clone();
                let wire = damaged.faces[0].holes[0];
                let pole = &mut damaged.0.loops[wire].coedges[0].pcurve.control_points[1][0];
                *pole = pole.next_up();
                damaged.rebuild_topology_ids();
                damaged.validate().unwrap();
                let invalid =
                    crate::boundary_embedding::inspect(&damaged, 1e-8, volume_limits().boundary)
                        .unwrap();
                assert!(!invalid.proven && !invalid.agreement.all_equal);
                assert!(invalid.intersections.pairs.pairs.iter().all(|p| !matches!(
                    p.boundary,
                    Some(crate::face_contacts::SharedBoundary::ClosedGraph(_))
                )));
            }
            for f in 2..7 {
                assert_eq!(
                    result.model().faces[f].surface,
                    models[outer].faces[f - 1].surface
                );
            }
            for (point, inside) in [
                ([0.5, 0.5, 1.96875], union),
                ([0.125, 0.125, 2.125], union),
                ([0.5, 0.5, 1.875], true),
                ([0.5, 0.5, -0.125], false),
                ([0.5, 0.5, 2.5], false),
            ] {
                let parity = crate::ray_parity::classify_point(
                    result.model(),
                    point,
                    &[[0., 0., 1.]],
                    1e-8,
                    10000,
                    100000,
                )
                .unwrap();
                assert_eq!(
                    parity.parity,
                    Some(inside),
                    "{operation:?} swapped={swapped}: {point:?}"
                );
            }
            let mut tight = limits();
            tight.operations = result.operations();
            assert!(assemble_closed_graph_envelope(&material, operation, tight).is_ok());
            tight.operations -= 1;
            assert!(assemble_closed_graph_envelope(&material, operation, tight).is_err());
            let report = result.audit_volume(1e-8, volume_limits()).unwrap();
            assert!(
                report.report().proven,
                "{operation:?} swapped={swapped}: boundary={} pairs={:?}",
                report.report().boundary.proven,
                report.report().boundary.intersections.pairs
            );
            assert_eq!(
                report
                    .into_qualified()
                    .unwrap_or_else(|_| panic!("unproven envelope"))
                    .model()
                    .faces
                    .len(),
                7
            );
        }
        assert_eq!(models, saved);
    }
}
