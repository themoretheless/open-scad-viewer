use super::*;
use crate::region_replacement::{EdgeRadius, StripLimits, build_trihedral_strips};
fn with_corner(f: impl FnOnce(&CentralLoftCandidate<'_, '_, '_>)) {
    with_corner_mode(false, f)
}
fn with_corner_mode(bounded: bool, f: impl FnOnce(&CentralLoftCandidate<'_, '_, '_>)) {
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
    let builder = |source, shell, vertex, requested: &[EdgeRadius], runout, span, limits| {
        if bounded {
            crate::region_replacement::build_trihedral_strips_with_bounded_profile(
                source, shell, vertex, requested, runout, span, limits, 10.,
            )
        } else {
            build_trihedral_strips(source, shell, vertex, requested, runout, span, limits)
        }
    };
    let strips = builder(
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
fn exterior_g2_does_not_admit_a_discontinuous_multispan_center() {
    with_corner_mode(true, |c| {
        let exterior = c.inspect_exterior_jets(100000, 1000000).unwrap();
        assert_eq!(exterior.len(), 6);
        assert!(exterior.iter().all(|r|
            r.decision == nurbs_core::surface_join::Decision::WithinTolerance
                && r.regularity_proven && r.pending == 0
        ));
        let joins = c.inspect_internal_span_joins(6000).unwrap();
        assert_eq!(joins.len(), 8);
        assert!(joins.iter().all(|(_, r)|
            r.decision == nurbs_core::surface_join::Decision::CorrespondenceExceedsTolerance
        ));
        let error = match c.sew_corner(limits()) {
            Err(e) => e,
            Ok(_) => panic!("Exterior evidence cannot admit defective internal joins"),
        };
        assert_eq!(error.code, "BREP_CENTRAL_LOFT_UNPROVEN");
        assert!(error.message.contains("internal G2"), "{error:?}");
        assert!(error.message.contains("knot="), "{error:?}");
    });
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

#[test]
fn actual_trimmed_sewn_corner_curvature_coverage_is_explicit() {
    with_corner(|c| {
        let sewn = c.sew_corner(limits()).unwrap();
        let before = sewn.model().clone();
        let report = sewn
            .inspect_curvature(
                1e-8,
                crate::face_curvature::Limits {
                    cells: 5000,
                    domain_cells: 100000,
                    shape_cells: 100000,
                },
            )
            .unwrap();
        eprintln!(
            "sewn curvature: complete={} within={} checked={} next={:?} cells={} domain={} shape={}",
            report.complete,
            report.within_limit,
            report.faces.len(),
            report.next_face,
            report.cells,
            report.domain_cells,
            report.shape_cells
        );
        for f in &report.faces {
            eprintln!(
                "face {}: {:?} bound={:?} witness={:?} reason={}",
                f.face,
                f.report.decision,
                f.report.absolute_curvature_upper,
                f.report.witness_curvature,
                f.report.reason
            );
        }
        assert!(
            report.cells <= 5000 && report.domain_cells <= 100000 && report.shape_cells <= 100000
        );
        assert_eq!(
            report.complete,
            report.faces.len() == sewn.model().faces.len()
                && report
                    .faces
                    .iter()
                    .all(|f| f.report.decision != crate::face_curvature::Decision::Unresolved)
        );
        let geometric = sewn.inspect_internal_g2(1e-5, 1e-4, 100000).unwrap();
        eprintln!(
            "actual geometric network: within={} required={} inspected={} cells={} next={:?}",
            geometric.within_tolerance,
            geometric.required_joins,
            geometric.joins.len(),
            geometric.cells,
            geometric.next_join
        );
        for j in &geometric.joins {
            eprintln!(
                "actual G2 face={} axis={} knot={} result={:?}",
                j.face,
                j.axis,
                j.knot,
                j.report.as_ref().map(|r| (
                    r.decision,
                    r.normal,
                    r.tensor,
                    r.regularity_proven,
                    r.pending
                ))
            );
        }
        assert!(geometric.cells <= 100000);
        assert!(
            geometric.within_tolerance,
            "Actual saved original knot network must meet geometric G2 tolerances"
        );
        let owned_g2 = sewn.qualify_internal_g2(1e-5, 1e-4, 100000).unwrap();
        assert!(std::ptr::eq(owned_g2.model(), sewn.model()));
        assert!(owned_g2.report().within_tolerance);
        let fields = owned_g2
            .inspect_curvature(
                1e-8,
                crate::face_curvature::Limits {
                    cells: 5000,
                    domain_cells: 100000,
                    shape_cells: 100000,
                },
            )
            .unwrap();
        eprintln!(
            "geometric curvature: complete={} within={} faces={} next={:?} cells={} domain={} shape={}",
            fields.complete,
            fields.within_limit,
            fields.faces.len(),
            fields.next_face,
            fields.cells,
            fields.domain_cells,
            fields.shape_cells
        );
        for f in &fields.faces {
            eprintln!(
                "geometric field face {}: {:?} bound={:?} witness={:?} region={:?} reason={}",
                f.face,
                f.report.decision,
                f.report.absolute_curvature_upper,
                f.report.witness_curvature,
                f.report.unresolved_region,
                f.report.reason
            );
        }
        assert!(fields.one_sided_fields);
        assert!(
            fields.complete,
            "The full face pass must reuse leftover work before reporting an unresolved face"
        );
        assert_eq!(
            fields
                .faces
                .iter()
                .filter(|f| f.report.decision == crate::face_curvature::Decision::Exceeds)
                .map(|f| f.face)
                .collect::<Vec<_>>(),
            vec![6, 12]
        );
        let empty_fields = owned_g2
            .inspect_curvature(
                1e-8,
                crate::face_curvature::Limits {
                    cells: 0,
                    domain_cells: 100000,
                    shape_cells: 100000,
                },
            )
            .unwrap();
        assert!(!empty_fields.within_limit && !empty_fields.complete);
        assert_eq!(empty_fields.next_face, Some(0));

        assert_eq!(fields.internal_g2_tolerances, Some([1e-5, 1e-4]));
        assert!(
            fields.cells <= 5000 && fields.domain_cells <= 100000 && fields.shape_cells <= 100000
        );
        assert!(!fields.within_limit);

        assert!(sewn.qualify_internal_g2(1e-5, 1e-4, 0).is_err());
        let exact_fields = sewn
            .inspect_curvature_with_continuity(
                1e-8,
                crate::face_curvature::Limits {
                    cells: 5000,
                    domain_cells: 100000,
                    shape_cells: 100000,
                },
                100000,
                1000000,
            )
            .unwrap();
        eprintln!(
            "exact C2 faces={:?} cells={} exact={} within={} complete={}",
            exact_fields.continuity_faces,
            exact_fields.continuity_cells,
            exact_fields.exact_work,
            exact_fields.within_limit,
            exact_fields.complete
        );
        eprintln!(
            "actual raw C2 decisions: {:?}",
            exact_fields.continuity_decisions
        );
        // These immutable control-fixture supports have different RAW C2 jets.
        // This is deliberately not a geometric G2 rejection. Preserve the
        // decisive result instead of collapsing it back to a budget stop.
        assert_eq!(
            exact_fields.continuity_decisions,
            vec![
                (10, cad_predicates::BezierIdentity::Different),
                (12, cad_predicates::BezierIdentity::Different)
            ]
        );
        for f in &exact_fields.faces {
            if matches!(f.face, 10 | 12) {
                eprintln!(
                    "exact C2 face {}: {:?} reason={} region={:?}",
                    f.face, f.report.decision, f.report.reason, f.report.unresolved_region
                );
            }
        }
        assert!(exact_fields.continuity_cells <= 100000 && exact_fields.exact_work <= 1000000);
        assert!(!exact_fields.within_limit);
        assert_eq!(sewn.model(), &before);
        assert!(
            !report.within_limit,
            "The 0.1 mm profile must not bypass its witnessed curvature excess"
        );
        assert!(report.faces.iter().any(|f| f.report.decision
            == crate::face_curvature::Decision::Exceeds
            && f.report.witness_curvature.is_some_and(|k| k[0] > 10.)));
        let empty = sewn
            .inspect_curvature(
                1e-8,
                crate::face_curvature::Limits {
                    cells: 0,
                    domain_cells: 100000,
                    shape_cells: 100000,
                },
            )
            .unwrap();
        assert!(!empty.within_limit && !empty.complete);
        let body_limits = CornerAuditLimits {
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
        };
        let no_cover = sewn
            .qualify_g2(
                1e-8,
                body_limits,
                crate::face_curvature::Limits {
                    cells: 0,
                    domain_cells: 100000,
                    shape_cells: 100000,
                },
            )
            .err()
            .unwrap();
        assert!(format!("{no_cover:?}").contains("BREP_SEWN_CURVATURE_UNPROVEN"));
        let excess = sewn
            .qualify_g2_with_continuity(
                1e-8,
                body_limits,
                crate::face_curvature::Limits {
                    cells: 5000,
                    domain_cells: 100000,
                    shape_cells: 100000,
                },
                100000,
                1000000,
            )
            .err()
            .unwrap();
        assert!(format!("{excess:?}").contains("BREP_SEWN_CURVATURE_LIMIT"));
    });
}
