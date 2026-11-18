use super::*;
use crate::graph_body::{
    self,
    tests::{column, limits as body_limits},
};
use nurbs_core::ss_intersection as ss;
fn limits() -> GraphAnnulusLimits {
    GraphAnnulusLimits {
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
fn annular_difference_authors_roof_to_roof_strips_and_material_under_operand_swap() {
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

        let minuend = usize::from(swapped);
        let result = assemble_closed_graph_boolean(
            &material,
            GraphBooleanOperation::Difference { minuend },
            limits(),
        )
        .unwrap();
        assert_eq!(
            result.operation(),
            GraphBooleanOperation::Difference { minuend }
        );
        assert_eq!(result.component(), GraphBooleanComponent::AnnularDifference);
        assert_eq!(result.model().faces.len(), 6);
        assert_eq!(result.model().edges.len(), 16);
        assert_eq!(result.model().vertices.len(), 12);
        for side in 0..2 {
            assert_eq!(
                result.model().faces[side].surface,
                models[side].faces[0].surface
            );
            assert_eq!(result.model().faces[side].holes.len(), 1);
            for c in &models[side].loops[models[side].faces[0].outer].coedges {
                assert_eq!(
                    result
                        .model()
                        .edges
                        .iter()
                        .filter(|e| e.curve == models[side].edges[c.edge].curve)
                        .count(),
                    1
                );
            }
        }
        let audit = crate::solid_audit::audit_solid(result.model()).unwrap();
        assert!(audit.ok && audit.sew.complete);
        for (p, inside) in [
            ([0.125, 0.125, 2.125], true),
            ([0.125, 0.5, 2.03125], true),
            ([0.5, 0.5, 1.96875], false),
            ([0.125, 0.125, 1.875], false),
            ([0.125, 0.125, 3.], false),
        ] {
            let parity = crate::ray_parity::classify_point(
                result.model(),
                p,
                &[[0., 0., 1.]],
                1e-8,
                10000,
                100000,
            )
            .unwrap();
            assert_eq!(parity.parity, Some(inside), "{p:?}: {parity:?}");
        }
        let mut tight = limits();
        tight.operations = result.operations();
        assert!(assemble_closed_graph_annulus(&material, minuend, tight).is_ok());
        tight.operations -= 1;
        assert!(assemble_closed_graph_annulus(&material, minuend, tight).is_err());
        assert!(assemble_closed_graph_annulus(&material, 1 - minuend, limits()).is_err());
        let stopped = assemble_closed_graph_boolean(
            &material,
            GraphBooleanOperation::Difference { minuend },
            limits(),
        )
        .unwrap();
        let mut small = volume_limits();
        small.boundary.spans = 100;
        let stopped = stopped.audit_volume_with_linear(1e-8, small, 1).unwrap();
        assert!(!stopped.report().proven);
        assert!(stopped.into_qualified().is_err());
        if !swapped {
            let mut folded = result.model().clone();
            folded.0.faces[2].surface = nurbs_core::surface_edit::elevate(
                &folded.faces[2].surface,
                nurbs_core::surface::Axis::V,
                2,
            )
            .unwrap();
            folded.0.faces[2].surface.control_points[1][1][0] = 100.;
            folded.rebuild_topology_ids();
            folded.validate().unwrap();
            let mut work = volume_limits();
            work.boundary.spans = 100;
            let invalid =
                crate::volume_validity::inspect_with_linear(&folded, 1e-8, work, 900).unwrap();
            assert!(invalid.boundary.agreement.all_equal);
            assert!(!invalid.boundary.intersections.faces.all_faces_injective);
            assert!(!invalid.proven);
        }
        let mut admission = volume_limits();
        let original_chart_budget = admission.boundary.spans;
        admission.boundary.spans = 100;
        let linear_cells = original_chart_budget - admission.boundary.spans;
        let report = result
            .audit_volume_with_linear(1e-8, admission, linear_cells)
            .unwrap();
        let charts = &report.report().boundary.intersections.faces;
        assert!(charts.linear_cells > 0);
        assert!(charts.spans + charts.linear_cells <= original_chart_budget);
        assert!(
            report.report().proven,
            "swapped={swapped}: boundary={}, faces={:?}, pairs={:?}",
            report.report().boundary.proven,
            report.report().boundary.intersections.faces,
            report
                .report()
                .boundary
                .intersections
                .pairs
                .pairs
                .iter()
                .map(|p| (p.faces, p.reason))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            report
                .into_qualified()
                .unwrap_or_else(|_| panic!("unproven annulus"))
                .model()
                .faces
                .len(),
            6
        );
        let lens = assemble_closed_graph_boolean(
            &material,
            GraphBooleanOperation::Difference {
                minuend: 1 - minuend,
            },
            limits(),
        )
        .unwrap();
        assert_eq!(lens.component(), GraphBooleanComponent::InnerDifference);
        assert_eq!(lens.model().faces.len(), 2);
        let p = crate::ray_parity::classify_point(
            lens.model(),
            [0.5, 0.5, 1.96875],
            &[[0., 0., 1.]],
            1e-8,
            10000,
            100000,
        )
        .unwrap();
        assert_eq!(p.parity, Some(true));
        assert!(
            lens.audit_volume(1e-8, volume_limits())
                .unwrap()
                .report()
                .proven
        );
        for operation in [
            GraphBooleanOperation::Union,
            GraphBooleanOperation::Intersection,
        ] {
            let envelope = assemble_closed_graph_boolean(&material, operation, limits()).unwrap();
            assert_eq!(envelope.component(), GraphBooleanComponent::Envelope);
            assert_eq!(envelope.model().faces.len(), 7);
            let p = crate::ray_parity::classify_point(
                envelope.model(),
                [0.5, 0.5, 1.96875],
                &[[0., 0., 1.]],
                1e-8,
                10000,
                100000,
            )
            .unwrap();
            assert_eq!(p.parity, Some(operation == GraphBooleanOperation::Union));
            assert!(
                envelope
                    .audit_volume(1e-8, volume_limits())
                    .unwrap()
                    .report()
                    .proven
            );
        }
        assert_eq!(models, saved);
    }
}
