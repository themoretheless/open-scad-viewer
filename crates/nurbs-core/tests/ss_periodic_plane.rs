use num_rational::BigRational as Q;
use nurbs_core::{ss_intersection as ss, surface::Surface};
#[path = "common/periodic_extrusion.rs"]
mod fixture;
fn plane(height: f64, radius: f64) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-3., -3., 5., 5.],
        knots_v: vec![10., 10., 14., 14.],
        control_points: vec![
            vec![
                vec![-radius, -radius, height],
                vec![-radius, radius, height],
            ],
            vec![vec![radius, -radius, height], vec![radius, radius, height]],
        ],
        weights: vec![vec![3.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn pair(scale: f64, height: f64) -> ss::FiberSources {
    let mut s = fixture::source(1);
    for row in &mut s.control_points {
        row[1][0] *= scale;
        row[1][1] *= scale;
    }
    ss::FiberSources::new(&s, &plane(height, 2.)).unwrap()
}
fn own(source: &ss::FiberSources) -> ss::QualifiedPeriodicPlaneSS {
    let o = ss::qualify_periodic_plane_ss(source, Default::default()).unwrap();
    o.reverify().unwrap();
    o
}
#[test]
fn periodic_cylinder_section_has_one_complete_closed_contour_and_original_seam() {
    let source = pair(1., 1.);
    let saved = (source.first().clone(), source.second().clone());
    let o = own(&source);
    assert_eq!(o.kind(), ss::PeriodicPlaneSSKind::ClosedContour);
    assert!(o.covers_entire_surfaces());
    assert!(o.support_proof().proven);
    assert_eq!(o.transverse_parameter(), Some(&Q::new(1.into(), 2.into())));
    assert!(
        o.level_control_points()
            .iter()
            .all(|p| p[2] == Q::from_integer(1.into()))
    );
    let generic = ss::construct_qualified_ss_branches(&source, Default::default()).unwrap();
    assert!(matches!(
        generic,
        ss::QualifiedSurfaceIntersection::PeriodicPlaneSection(_)
    ));
    generic.reverify().unwrap();
    assert!(generic.covers_entire_surfaces());
    assert_eq!(saved, (source.first().clone(), source.second().clone()));
}
#[test]
fn positive_taper_keeps_exact_level_coefficients() {
    let o = own(&pair(0.5, 1.));
    assert_eq!(o.kind(), ss::PeriodicPlaneSSKind::ClosedContour);
    assert_eq!(o.level_control_points()[0][0], Q::new(3.into(), 4.into()));
    assert!(o.pole_point().is_none());
}
#[test]
fn rational_clock_is_not_rounded_and_natural_domains_are_preserved() {
    let source = pair(1., 1.);
    let mut s = source.first().clone();
    for row in &mut s.weights {
        row[1] *= 2.;
    }
    s.knots_v = vec![-2., -2., 6., 6.];
    let source = ss::FiberSources::new(&s, source.second()).unwrap();
    let o = own(&source);
    assert_eq!(o.transverse_parameter(), Some(&Q::new(2.into(), 3.into())));
    assert_eq!(o.kind(), ss::PeriodicPlaneSSKind::ClosedContour);
}
#[test]
fn either_collapsed_shore_is_one_point_instead_of_a_degenerate_closed_edge() {
    for reverse in [false, true] {
        let source = pair(0., 2.);
        let mut s = source.first().clone();
        if reverse {
            for row in &mut s.control_points {
                row.swap(0, 1);
            }
            for row in &mut s.weights {
                row.swap(0, 1);
            }
        }
        let source = ss::FiberSources::new(&s, &plane(2., 0.125)).unwrap();
        let o = own(&source);
        assert_eq!(o.kind(), ss::PeriodicPlaneSSKind::CollapsedPoleContact);
        assert_eq!(
            o.pole_point(),
            Some(&[
                Q::from_integer(0.into()),
                Q::from_integer(0.into()),
                Q::from_integer(2.into())
            ])
        );
        assert!(
            o.level_control_points()
                .iter()
                .all(|p| Some(p) == o.pole_point())
        );
        let generic = ss::construct_qualified_ss_branches(&source, Default::default()).unwrap();
        generic.reverify().unwrap();
    }
}
#[test]
fn outside_height_is_a_complete_empty_intersection() {
    for h in [-1., 3.] {
        let o = own(&pair(1., h));
        assert_eq!(o.kind(), ss::PeriodicPlaneSSKind::Disjoint);
        assert!(o.level_control_points().is_empty());
        assert!(o.transverse_parameter().is_none());
    }
}
#[test]
fn parameter_axis_and_operand_reversal_keep_original_quotient_ownership() {
    let source = pair(0.5, 1.);
    let mut s = source.first().clone();
    let old = s.control_points.clone();
    let w = s.weights.clone();
    s.control_points = (0..2)
        .map(|j| (0..old.len()).map(|i| old[i][j].clone()).collect())
        .collect();
    s.weights = (0..2)
        .map(|j| (0..w.len()).map(|i| w[i][j]).collect())
        .collect();
    std::mem::swap(&mut s.degree_u, &mut s.degree_v);
    std::mem::swap(&mut s.knots_u, &mut s.knots_v);
    s.periodic_u = false;
    s.periodic_v = true;
    for reverse in [false, true] {
        let source = if reverse {
            ss::FiberSources::new(source.second(), &s)
        } else {
            ss::FiberSources::new(&s, source.second())
        }
        .unwrap();
        let o = own(&source);
        assert_eq!(o.periodic_axis(), 1);
        assert_eq!(o.periodic_surface(), usize::from(reverse));
    }
}
#[test]
fn finite_plane_clipping_double_winding_and_damaged_seams_cannot_claim_a_full_contour() {
    let s = fixture::source(1);
    assert!(
        ss::qualify_periodic_plane_ss(
            &ss::FiberSources::new(&s, &plane(1., 0.5)).unwrap(),
            Default::default()
        )
        .is_err()
    );
    let double = fixture::source(2);
    assert!(
        ss::qualify_periodic_plane_ss(
            &ss::FiberSources::new(&double, &plane(1., 2.)).unwrap(),
            Default::default()
        )
        .is_err()
    );
    let mut bad = s.clone();
    bad.control_points[0][0][0] = bad.control_points[0][0][0].next_up();
    let repeated = bad.control_points.len() - 2;
    bad.control_points[repeated][0][0] = bad.control_points[0][0][0];
    assert!(
        ss::qualify_periodic_plane_ss(
            &ss::FiberSources::new(&bad, &plane(1., 2.)).unwrap(),
            Default::default()
        )
        .is_err()
    );
    let mut collapse = s.clone();
    for row in &mut collapse.control_points {
        row[1][0] *= -0.5;
        row[1][1] *= -0.5;
    }
    assert!(
        ss::qualify_periodic_plane_ss(
            &ss::FiberSources::new(&collapse, &plane(1., 2.)).unwrap(),
            Default::default()
        )
        .is_err()
    );
}
#[test]
fn budgets_and_invalid_limits_never_publish_partial_periodic_owners() {
    let source = pair(1., 1.);
    for limits in [
        ss::PeriodicSSBuildLimits {
            max_pairs: 1,
            max_cells: 100000,
        },
        ss::PeriodicSSBuildLimits {
            max_pairs: 10000,
            max_cells: 1,
        },
    ] {
        assert_eq!(
            ss::qualify_periodic_plane_ss(&source, limits)
                .unwrap_err()
                .code,
            "NURBS_RESOURCE_LIMIT"
        );
        let limits = ss::QualifiedSSBuildLimits {
            periodic: limits,
            ..Default::default()
        };
        assert_eq!(
            ss::construct_qualified_ss_branches(&source, limits)
                .unwrap_err()
                .code,
            "NURBS_RESOURCE_LIMIT"
        );
    }
    assert!(
        ss::qualify_periodic_plane_ss(
            &source,
            ss::PeriodicSSBuildLimits {
                max_pairs: 0,
                ..Default::default()
            }
        )
        .is_err()
    );
    let full = own(&source);
    // The observed work is not a minimal required budget: the underlying
    // bounded proof may choose a different valid partition. Every admitted
    // owner must fit its actual caller allowance and independently replay.
    for budget in [full.cells() - 1, full.cells(), full.cells() + 1] {
        if let Ok(owner) = ss::qualify_periodic_plane_ss(
            &source,
            ss::PeriodicSSBuildLimits {
                max_cells: budget,
                ..Default::default()
            },
        ) {
            assert!(owner.cells() <= budget);
            owner.reverify().unwrap();
        }
    }
}

#[test]
fn world_coordinate_permutation_preserves_the_exact_section() {
    let source = pair(0.5, 1.);
    let mut s = source.first().clone();
    let mut p = source.second().clone();
    for surface in [&mut s, &mut p] {
        for point in surface.control_points.iter_mut().flatten() {
            point.swap(1, 2);
        }
    }
    let source = ss::FiberSources::new(&s, &p).unwrap();
    let o = own(&source);
    assert_eq!(o.kind(), ss::PeriodicPlaneSSKind::ClosedContour);
    assert!(
        o.level_control_points()
            .iter()
            .all(|p| p[1] == Q::from_integer(1.into()))
    );
}
