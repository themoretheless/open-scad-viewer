use super::*;
use crate::region_replacement::{EdgeRadius, StripLimits, build_trihedral_strips};
fn with_corner(f: impl FnOnce(&CentralLoftCandidate<'_, '_, '_>)) {
    let source = crate::cuboid([0.; 3], [10.; 3]).unwrap();
    let before = source.clone();
    let budget = crate::source_box::Limits {
        operations: 1000000,
        body: crate::graph_body::tests::limits().body,
    };
    let stock = crate::source_box::qualify(&source, &budget).unwrap();
    let requested = source
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| e.vertices.contains(&0))
        .enumerate()
        .map(|(i, (edge, _))| EdgeRadius {
            edge,
            radius_mm: [0.1, 0.2, 0.35][i],
        })
        .collect::<Vec<_>>();
    let strips = build_trihedral_strips(
        &source,
        0,
        0,
        &requested,
        0.1,
        2.,
        StripLimits {
            tolerances: [1e-7, 1e-5, 1e-4],
            tolerance_uv: 1e-8,
            max_geometry_work: 10000,
            max_cells: 300000,
            max_exact: 1000000,
            max_coefficients: 2000000,
        },
    )
    .unwrap();
    let c = propose_trihedral_center(
        &stock,
        &strips,
        CentralLoftOptions {
            handle_mm: 1. / 256.,
            tolerances: [1e-7, 1e-5, 1e-4],
            tolerance_uv: 1e-8,
            operations: 1000000,
            cells: 100000,
        },
    )
    .unwrap();
    f(&c);
    assert_eq!(source, before);
}
fn limits() -> CornerSewLimits {
    CornerSewLimits {
        cells: 100000,
        exact_work: 1000000,
        trim_pairs: 10000,
        domain_cells: 1000000,
    }
}
#[test]
fn actual_central_surface_and_three_strips_sew_to_owned_closed_corner() {
    with_corner(|c| {
        let sewn = c.sew_corner(limits()).unwrap();
        let m = sewn.model();
        eprintln!(
            "sewn corner: V={} E={} F={} closure={:?} trim={} cells={} exact={} Euler={:?}",
            m.vertices.len(),
            m.edges.len(),
            m.faces.len(),
            sewn.closure.closure.status,
            sewn.trims.all_valid,
            sewn.cells,
            sewn.exact_work,
            m.0.euler_characteristic()
        );
        assert!(sewn.closed_boundary_proven());
        assert_eq!(sewn.strip_fits().len(), 3);
        assert!(
            sewn.strip_fits()
                .iter()
                .all(|r| r.displacement_upper_mm <= c.options.tolerances[0])
        );
        assert!(sewn.final_jets().len() >= 12);
        assert_eq!(sewn.radii().len(), 3);
        assert!(
            sewn.radii()
                .iter()
                .all(|r| r.decision == crate::fillet_radius::Decision::WithinTolerance)
        );
        let mut outside = m.clone();
        outside.faces[sewn.central_face].surface.control_points[1][2][0] = -1e-4;
        assert!(
            !material::inspect(&outside, sewn.stock_bounds, true)
                .unwrap()
                .boundary_inside_stock
        );
        let no_projection =
            projection::inspect(&m.faces[sewn.central_face].surface, [1; 3], 10000, 0).unwrap();
        assert!(no_projection.certificate.is_none() && !no_projection.report.proven);
        assert_eq!(m.faces.len(), 13);
        assert_eq!(m.vertices.len(), 22);
        assert_eq!(m.edges.len(), 35);
        assert_eq!(m.edges.iter().filter(|e| e.degenerate).count(), 2);
        assert_eq!(m.bodies.len(), 1);
        assert!(m.shells[0].closed);
        assert_eq!(sewn.source_vertices[0], None);
        assert_eq!(sewn.source_vertices.iter().flatten().count(), 7);
        m.validate().unwrap();
        assert!(sewn.cells <= limits().cells && sewn.exact_work <= limits().exact_work);
    });
}
#[test]
fn exhausted_or_invalid_sewing_budget_cannot_publish_a_closed_body() {
    with_corner(|c| {
        assert!(
            c.sew_corner(CornerSewLimits {
                cells: 0,
                ..limits()
            })
            .is_err()
        );
        assert!(
            c.sew_corner(CornerSewLimits {
                exact_work: 0,
                ..limits()
            })
            .is_err()
        );
        assert!(
            c.sew_corner(CornerSewLimits {
                cells: 100,
                ..limits()
            })
            .is_err()
        );
    });
}

#[test]
fn actual_sewn_corner_runs_fresh_material_intersection_and_original_flux_checks() {
    with_corner(|c| {
        let sewn = c.sew_corner(limits()).unwrap();
        let qualified = sewn
            .qualify(
                1e-8,
                CornerAuditLimits {
                    spans: 1000,
                    linear_cells: 5000,
                    contacts: crate::face_contacts::Limits {
                        pairs: 100,
                        cells: 100000,
                        domain_cells: 1000000,
                        cells_per_pair: 1000,
                        domain_cells_per_pair: 10000,
                    },
                    flux: FluxLimits {
                        axis: 0,
                        origin: 0.,
                        absolute_error: 1e-4,
                        cells: 10000,
                        spans: 100000,
                        planar_cells: 10000,
                    },
                },
            )
            .unwrap();
        let report = qualified.report();
        eprintln!(
            "qualified volume={:?} removed={:?} strip fits={:?} final jets={}",
            qualified.volume_mm3(),
            qualified.removed_volume_mm3(),
            sewn.strip_fits(),
            sewn.final_jets().len()
        );
        eprintln!(
            "corner exact uses={} all={} joins={} missing={:?}",
            sewn.closure.exact.uses.len(),
            sewn.closure.exact.all_equal,
            sewn.closure.exact.all_joins_exact,
            sewn.closure
                .exact
                .uses
                .iter()
                .filter(|u| u
                    .decision
                    .as_ref()
                    .is_none_or(|d| d.outcome != cad_predicates::BezierIdentity::Equal))
                .map(|u| (u.face, u.edge))
                .collect::<Vec<_>>()
        );
        eprintln!(
            "corner central embedding={} boundary={} jacobian={} poles={} cells={} exact={} pending={} reason={}",
            report.central_embedding.proven,
            report.central_embedding.boundary_simple,
            report.central_embedding.jacobian_positive,
            report.central_embedding.pole_quotients,
            report.central_embedding.cells,
            report.central_embedding.exact_work,
            report.central_embedding.pending,
            report.central_embedding.reason
        );
        eprintln!(
            "corner material boundary={} support-violations={:?} trim-violations={:?}",
            report.material.boundary_inside_stock,
            report.material.support_violations,
            report.material.trim_violations
        );
        eprintln!(
            "corner generic embedding absence={} all-faces={} all-pairs={} next={:?} exact-boundary={} volume={:?} material={}",
            report.intersections.absence_proven,
            report.intersections.faces.all_faces_injective,
            report.intersections.pairs.all_pairs_classified,
            report.intersections.pairs.next_pair,
            report.exact_boundary,
            report.volume_bounds_mm3,
            report.material_preservation_proven
        );
        for f in &report.intersections.faces.faces {
            eprintln!(
                "corner face {}: injective={} result={:?} linear={:?} quotient={}",
                f.face,
                f.absence_proven(),
                f.result,
                f.linear,
                f.quotient.as_ref().is_some_and(|q| q.proven())
            );
        }
        for p in &report.intersections.pairs.pairs {
            if p.boundary.is_none()
                && !p.hull_disjoint
                && !p.result.as_ref().is_some_and(|r| r.absence_proven)
            {
                eprintln!(
                    "corner pair {:?}: reason={} contact={}",
                    p.faces,
                    p.reason,
                    p.result.as_ref().is_some_and(|r| r.contact.is_some())
                );
            }
        }
        eprintln!(
            "corner flux={:?} converged={} cells={} spans={} planar={} reason={}",
            report.flux.signed_flux_mm3,
            report.flux.converged,
            report.flux.cells,
            report.flux.spans,
            report.flux.planar_cells,
            report.flux.reason
        );
        eprintln!(
            "corner additional contacts={:?} fit={}",
            report.contacts,
            sewn.surface_fit_upper_mm()
        );
        assert!(report.material.boundary_inside_stock);
        assert!(report.central_embedding.proven);
        assert!(report.flux.converged);
        assert!(report.exact_boundary);
        assert!(report.absence_proven);
        assert!(report.material_preservation_proven);
        let v = report.flux.signed_flux_mm3.unwrap();
        assert!(v[0] > 0. && v[1] < 1000.);
        assert!(report.intersections.pairs.next_pair.is_none());
        assert_eq!(
            report.volume_bounds_mm3.is_some(),
            report.exact_boundary && report.absence_proven && report.flux.converged && v[0] > 0.
        );
    });
}
