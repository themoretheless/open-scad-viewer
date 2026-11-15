use nurbs_core::{
    editing::exact_surface_derivative::{CurvatureGapComparison, Refusal, compare_curvature_gap},
    surface::Surface,
};

const WORK: u64 = 1_000_000;

fn cylinder() -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [(1., 0.), (1., 1.), (0., 1.)]
            .map(|(x, y)| vec![vec![x, y, 0.], vec![x, y, 1.]])
            .to_vec(),
        weights: vec![vec![1.; 2], vec![1.; 2], vec![2.; 2]],
        periodic_u: false,
        periodic_v: false,
    }
}

#[test]
fn unit_cylinder_compares_exactly_at_adjacent_binary64_levels() {
    let s = cylinder();
    for (level, expected) in [
        (1f64.next_down(), CurvatureGapComparison::AtLeast),
        (1., CurvatureGapComparison::EqualEverywhere),
        (1f64.next_up(), CurvatureGapComparison::AtMost),
    ] {
        let report = compare_curvature_gap(&s, level, WORK).unwrap();
        let cells = report
            .cells
            .unwrap_or_else(|| panic!("{:?}", report.refusal));
        assert_eq!(cells[0].domain, [[0., 1.], [0., 1.]]);
        assert_eq!(cells[0].comparison, expected);
    }
}

#[test]
fn parabolic_cylinder_has_exact_opposite_corner_signs() {
    let mut s = cylinder();
    s.weights = vec![vec![1.; 2]; 3];
    s.control_points = [(0., 0.), (0.5, 0.), (1., 1.)]
        .map(|(x, z)| vec![vec![x, 0., z], vec![x, 1., z]])
        .to_vec();
    // The squared gap is 4/(1+4u²)^3, with maximum 4 at u=0.
    for (level, expected) in [
        (0.03125, CurvatureGapComparison::AtLeast),
        (1., CurvatureGapComparison::CornerStraddle),
        (4., CurvatureGapComparison::AtMost),
    ] {
        let report = compare_curvature_gap(&s, level, WORK).unwrap();
        assert_eq!(report.cells.unwrap()[0].comparison, expected);
    }
}

#[test]
fn singular_charts_refuse_even_a_large_threshold() {
    let mut s = cylinder();
    let point = s.control_points[0][0].clone();
    for row in &mut s.control_points {
        row.fill(point.clone());
    }
    let report = compare_curvature_gap(&s, 1e9, WORK).unwrap();
    assert_eq!(report.refusal, Some(Refusal::UnprovedDenominator));
    assert!(report.cells.is_none());
}

#[test]
fn invalid_levels_and_insufficient_work_refuse() {
    let s = cylinder();
    for invalid in [0., -1., f64::INFINITY, f64::NAN] {
        assert!(compare_curvature_gap(&s, invalid, WORK).is_err());
    }
    let report = compare_curvature_gap(&s, 1., 1).unwrap();
    assert_eq!(report.refusal, Some(Refusal::ResourceLimit));
    assert!(report.cells.is_none());
}

#[test]
fn coefficient_ratio_range_rounds_exact_extrema_outward() {
    use num_rational::BigRational;
    use nurbs_core::editing::exact_surface_derivative::curvature_gap_ranges;
    let mut s = cylinder();
    s.weights = vec![vec![1.; 2]; 3];
    s.control_points = [(0., 0.), (0.5, 0.), (1., 1.)]
        .map(|(x, z)| vec![vec![x, 0., z], vec![x, 1., z]])
        .to_vec();
    let r = curvature_gap_ranges(&s, WORK).unwrap();
    let cells = r.cells.unwrap_or_else(|| panic!("{:?}", r.refusal));
    let [lo, hi] = cells[0].bounds.unwrap();
    let minimum = BigRational::new(4.into(), 125.into());
    assert!(BigRational::from_float(lo).unwrap() <= minimum);
    assert!(BigRational::from_float(lo.next_up()).unwrap() > minimum);
    assert_eq!(hi, 4.);
}

#[test]
fn exact_constant_ranges_do_not_depend_on_the_raw_denominator_degree() {
    use nurbs_core::editing::exact_surface_derivative::curvature_gap_ranges;
    let mut s = cylinder();
    let r = curvature_gap_ranges(&s, WORK).unwrap();
    assert_eq!(r.cells.unwrap()[0].bounds, Some([1., 1.]));
    for point in s.control_points.iter_mut().flatten() {
        point[1] = point[2];
        point[2] = 0.;
    }
    s.weights = vec![vec![1.; 2]; 3];
    for (row, x) in s.control_points.iter_mut().zip([0., 0.5, 1.]) {
        for point in row {
            point[0] = x;
        }
    }
    let r = curvature_gap_ranges(&s, WORK).unwrap();
    assert_eq!(r.cells.unwrap()[0].bounds, Some([0., 0.]));
}

#[test]
fn range_budget_and_late_singularity_are_atomic() {
    use nurbs_core::editing::exact_surface_derivative::curvature_gap_ranges;
    let mut s = cylinder();
    let short = curvature_gap_ranges(&s, 1).unwrap();
    assert_eq!(short.refusal, Some(Refusal::ResourceLimit));
    assert!(short.cells.is_none());
    s.degree_u = 1;
    s.knots_u = vec![0., 0., 0.5, 1., 1.];
    s.weights = vec![vec![1.; 2]; 3];
    s.control_points = [0., 1., 1.]
        .map(|x| vec![vec![x, 0., 0.], vec![x, 1., 0.]])
        .to_vec();
    let r = curvature_gap_ranges(&s, WORK).unwrap();
    assert_eq!(r.refusal, Some(Refusal::UnprovedDenominator));
    assert!(r.cells.is_none());
}

#[test]
fn subdivides_opposite_threshold_zones_and_keeps_an_exact_cover() {
    use nurbs_core::editing::exact_surface_derivative::refine_curvature_gap;
    let mut s = cylinder();
    s.knots_u = vec![2., 2., 2., 6., 6., 6.];
    s.weights = vec![vec![1.; 2]; 3];
    s.control_points = [(0., 0.), (0.5, 0.), (1., 1.)]
        .map(|(x, z)| vec![vec![x, 0., z], vec![x, 1., z]])
        .to_vec();
    let r = refine_curvature_gap(&s, 1., 6, WORK).unwrap();
    let regions = r.regions.unwrap_or_else(|| panic!("{:?}", r.refusal));
    let mut area = [0.; 3];
    for region in &regions {
        assert_eq!(region.original_domain, [[2., 6.], [0., 1.]]);
        let [[lo, hi], [v0, v1]] = region.unit_domain;
        let residual = |u: f64| 4. - (1. + 4. * u * u).powi(3);
        let index = match region.comparison {
            CurvatureGapComparison::AtLeast => {
                assert!(residual(hi) >= 0.);
                0
            }
            CurvatureGapComparison::AtMost => {
                assert!(residual(lo) <= 0.);
                1
            }
            CurvatureGapComparison::CornerStraddle => {
                assert!(residual(lo) > 0. && residual(hi) < 0.);
                assert!(hi - lo <= 0.125);
                2
            }
            status => panic!("unexpected {status:?}"),
        };
        area[index] += (hi - lo) * (v1 - v0);
    }
    assert!(area.iter().all(|value| *value > 0.));
    assert_eq!(area.iter().sum::<f64>(), 1.);
    for (i, a) in regions.iter().enumerate() {
        for b in &regions[i + 1..] {
            assert!(
                (0..2).any(|axis| a.unit_domain[axis][1] <= b.unit_domain[axis][0]
                    || b.unit_domain[axis][1] <= a.unit_domain[axis][0])
            );
        }
    }
    let short = refine_curvature_gap(&s, 1., 6, r.work_used - 1).unwrap();
    assert_eq!(short.refusal, Some(Refusal::ResourceLimit));
    assert!(short.regions.is_none());
}

#[test]
fn a_constant_identity_stops_without_splitting() {
    use nurbs_core::editing::exact_surface_derivative::refine_curvature_gap;
    let s = cylinder();
    let r = refine_curvature_gap(&s, 1., 8, WORK).unwrap();
    let regions = r.regions.unwrap();
    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].unit_domain, [[0., 1.]; 2]);
    assert_eq!(
        regions[0].comparison,
        CurvatureGapComparison::EqualEverywhere
    );
    assert!(refine_curvature_gap(&s, 1., 9, WORK).is_err());
}
