//! Ray parity over authored trimmed surface images. This does not establish
//! that a model is a closed embedded geometric boundary of a valid volume.
use crate::{Error, Model, Result, face_domain::FaceDomain};
use nurbs_core::{ray_surface, trim_domain::Location};
#[derive(Debug)]
pub struct Crossing {
    pub face: usize,
    pub uv: [[f64; 2]; 2],
    pub parameter: [f64; 2],
}
#[derive(Debug)]
pub struct Unresolved {
    pub face: usize,
    pub uv: [[f64; 2]; 2],
    pub reason: &'static str,
}
#[derive(Debug)]
pub struct Report {
    pub parity: Option<bool>,
    pub crossings: Vec<Crossing>,
    pub unresolved: Vec<Unresolved>,
    pub cells: usize,
    pub domain_cells: usize,
}
/// Bounded direction retries for point parity. A certified ray suffices only
/// after the caller establishes an embedded closed boundary. This report does
/// not certify that precondition or turn unresolved rays into outside points.
#[derive(Debug)]
pub struct PointReport {
    pub parity: Option<bool>,
    pub attempts: Vec<Report>,
    pub cells: usize,
    pub domain_cells: usize,
}
pub fn classify_point(
    model: &Model,
    point: [f64; 3],
    directions: &[[f64; 3]],
    tolerance_uv: f64,
    max_cells: usize,
    max_domain_cells: usize,
) -> Result<PointReport> {
    model.validate()?;
    if model.faces.is_empty()
        || directions.is_empty() || directions.len() > 16
        || !point.iter().all(|x| x.is_finite())
        || directions.iter().any(|d| !d.iter().all(|x| x.is_finite()) || d.iter().all(|&x| x == 0.))
        || !tolerance_uv.is_finite() || tolerance_uv <= 0.
        || !(1..=1000000).contains(&max_cells)
        || !(1..=8000000).contains(&max_domain_cells)
    {
        return Err(Error::new("BREP_INVALID_INPUT", "Point parity requires 1..16 finite nonzero directions and bounded positive tolerances and work"));
    }
    let mut result = PointReport { parity: None, attempts: Vec::new(), cells: 0, domain_cells: 0 };
    for (i, &direction) in directions.iter().enumerate() {
        let cells = max_cells - result.cells;
        let domains = max_domain_cells - result.domain_cells;
        if cells == 0 || domains == 0 { break; }
        // Reserve work for later directions instead of letting a tangent ray
        // consume the complete budget. Unused work carries forward.
        let remaining = directions.len() - i;
        let attempt = classify_ray(model, point, direction, tolerance_uv,
            (cells / remaining).max(1), (domains / remaining).max(1))?;
        result.cells += attempt.cells;
        result.domain_cells += attempt.domain_cells;
        result.parity = attempt.parity;
        result.attempts.push(attempt);
        if result.parity.is_some() { break; }
    }
    Ok(result)
}
pub fn classify_ray(
    model: &Model,
    point: [f64; 3],
    direction: [f64; 3],
    tolerance_uv: f64,
    max_cells: usize,
    max_domain_cells: usize,
) -> Result<Report> {
    model.validate()?;
    if model.faces.is_empty()
        || !point.iter().chain(&direction).all(|x| x.is_finite())
        || direction.iter().all(|&x| x == 0.)
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || !(1..=1000000).contains(&max_cells)
        || !(1..=8000000).contains(&max_domain_cells)
    {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Ray parity needs a nonempty model, finite point and nonzero direction, positive UV tolerance and bounded work",
        ));
    }
    let domains = (0..model.faces.len())
        .map(|i| FaceDomain::new(model, i, tolerance_uv))
        .collect::<Result<Vec<_>>>()?;
    let mut report = Report {
        parity: None,
        crossings: Vec::new(),
        unresolved: Vec::new(),
        cells: 0,
        domain_cells: 0,
    };
    for (face, f) in model.faces.iter().enumerate() {
        let s = &f.surface;
        let uv = [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ];
        if ray_surface::parameter_bounds(s, uv, point, direction)?[1] < 0. {
            continue;
        }
        let remaining = (max_cells - report.cells).min(100000);
        let spans = |knots: &[f64], degree: usize, count: usize| {
            (degree..count).filter(|&k| knots[k] < knots[k + 1]).count()
        };
        let initial = spans(&s.knots_u, s.degree_u, s.control_points.len()).saturating_mul(spans(
            &s.knots_v,
            s.degree_v,
            s.control_points[0].len(),
        ));
        if remaining < initial {
            report.unresolved.push(Unresolved {
                face,
                uv,
                reason: "work-limit",
            });
            continue;
        }
        let roots = ray_surface::intersections(s, point, direction, tolerance_uv, remaining)?;
        report.cells += roots.cells;
        // An unresolved line cell can only be discarded if its whole image is
        // behind the origin or its complete UV region is outside the trim.
        for uv in roots.unresolved {
            if ray_surface::parameter_bounds(s, uv, point, direction)?[1] < 0. {
                continue;
            }
            let remaining = max_domain_cells - report.domain_cells;
            if remaining == 0 {
                report.unresolved.push(Unresolved {
                    face,
                    uv,
                    reason: "domain-work-limit",
                });
                continue;
            }
            let c = domains[face].classify(uv, remaining.min(4096))?;
            report.domain_cells += c.cells;
            if c.location != Location::Outside {
                report.unresolved.push(Unresolved {
                    face,
                    uv,
                    reason: "root-not-isolated",
                })
            }
        }
        for root in roots.roots {
            if root.parameter[1] < 0. {
                continue;
            }
            let remaining = max_domain_cells - report.domain_cells;
            if remaining == 0 {
                report.unresolved.push(Unresolved {
                    face,
                    uv: root.uv,
                    reason: "domain-work-limit",
                });
                continue;
            }
            let c = domains[face].classify(root.uv, remaining.min(4096))?;
            report.domain_cells += c.cells;
            if c.location == Location::Outside {
                continue;
            }
            let reason = if c.location != Location::Inside {
                Some("trim-boundary")
            } else if root.parameter[0] <= 0. {
                Some("origin-band")
            } else {
                None
            };
            if let Some(reason) = reason {
                report.unresolved.push(Unresolved {
                    face,
                    uv: root.uv,
                    reason,
                })
            } else {
                report.crossings.push(Crossing {
                    face,
                    uv: root.uv,
                    parameter: root.parameter,
                })
            }
        }
    }
    report
        .crossings
        .sort_by(|a, b| a.parameter[0].total_cmp(&b.parameter[0]));
    for pair in report.crossings.windows(2) {
        if pair[0].parameter[1] >= pair[1].parameter[0] {
            report.unresolved.push(Unresolved {
                face: pair[1].face,
                uv: pair[1].uv,
                reason: "overlapping-root-intervals",
            })
        }
    }
    if report.unresolved.is_empty() {
        report.parity = Some(report.crossings.len() % 2 == 1)
    }
    Ok(report)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn point_retries_a_boundary_hit_without_treating_it_as_outside() {
        let m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let before = format!("{m:?}");
        let r = classify_point(&m, [-1., 0., 0.5], &[[1., 0., 0.], [1., 0.3, 0.11]], 1e-7, 10000, 100000).unwrap();
        assert_eq!(r.attempts.len(), 2);
        assert_eq!(r.attempts[0].parity, None);
        assert_eq!(r.parity, Some(false));
        assert_eq!(r.cells, r.attempts.iter().map(|a| a.cells).sum::<usize>());
        assert_eq!(r.domain_cells, r.attempts.iter().map(|a| a.domain_cells).sum::<usize>());
        assert!(r.cells <= 10000 && r.domain_cells <= 100000);
        assert_eq!(format!("{m:?}"), before);
    }
    #[test]
    fn point_retry_budget_and_boundary_origins_remain_unresolved() {
        let m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let directions = [[1., 0.3, 0.11], [-1., 0.17, 0.29]];
        let r = classify_point(&m, [0., 0.4, 0.5], &directions, 1e-7, 10000, 100000).unwrap();
        assert_eq!(r.parity, None);
        let r = classify_point(&m, [0.3, 0.4, 0.5], &directions, 1e-7, 1, 1).unwrap();
        assert_eq!(r.parity, None);
        assert!(r.cells <= 1 && r.domain_cells <= 1);
        assert!(classify_point(&m, [0.; 3], &[], 1e-7, 100, 100).is_err());
        assert!(classify_point(&m, [0.; 3], &[[1., 0., 0.], [f64::NAN, 0., 0.]], 1e-7, 100, 100).is_err());
    }
    #[test]
    fn cavity_and_rigid_placement_preserve_material_parity() {
        let outer = crate::cuboid([0.; 3], [10.; 3]).unwrap();
        let inner = crate::cuboid([2.; 3], [8.; 3]).unwrap();
        let cavity = crate::operations::boolean(&outer, &inner, "difference").unwrap();
        assert_eq!(cavity.bodies[0].inner_shells.len(), 1);
        let angle = 0.37_f64;
        let (s, c) = angle.sin_cos();
        let matrix = [[c, -s, 0., 13.], [s, c, 0., -7.], [0., 0., 1., 3.], [0., 0., 0., 1.]];
        let placed = crate::transform::affine(&cavity, matrix).unwrap();
        for (point, expected) in [([1., 4., 5.], true), ([5., 4., 5.], false), ([-1., 4., 5.], false)] {
            for direction in [[1., 0.07, 0.03], [-1., 0.09, 0.02]] {
                let moved_point = [c*point[0]-s*point[1]+13., s*point[0]+c*point[1]-7., point[2]+3.];
                let moved_direction = [c*direction[0]-s*direction[1], s*direction[0]+c*direction[1], direction[2]];
                for (model, p, d) in [(&cavity, point, direction), (&placed, moved_point, moved_direction)] {
                    let report = classify_ray(model, p, d, 1e-7, 100000, 1000000).unwrap();
                    assert_eq!(report.parity, Some(expected), "{p:?} {d:?}: {:?}", report.unresolved);
                    let retry = classify_point(model, p, &[d, [-d[0], -d[1], -d[2]]], 1e-7, 100000, 1000000).unwrap();
                    assert_eq!(retry.parity, Some(expected));
                    assert!(retry.cells <= 100000 && retry.domain_cells <= 1000000);
                }
            }
        }
    }

    #[test]
    fn cube_inside_outside_and_boundary() {
        let m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        for (p, expected, count) in [
            ([0.3, 0.4, 0.5], true, 1),
            ([-1., 0.4, 0.5], false, 2),
            ([2., 0.4, 0.5], false, 0),
        ] {
            let r = classify_ray(&m, p, [1., 0., 0.], 1e-7, 10000, 100000).unwrap();
            assert_eq!(r.parity, Some(expected), "{:?}", r.unresolved);
            assert_eq!(r.crossings.len(), count);
        }
        let r = classify_ray(&m, [0., 0.4, 0.5], [1., 0., 0.], 1e-7, 10000, 100000).unwrap();
        assert!(r.parity.is_none());
        assert!(r.unresolved.iter().any(|c| c.reason == "origin-band"));
    }
    #[test]
    fn rational_sphere_uses_complete_trimmed_crossings() {
        let m = crate::analytic::sphere(2.).unwrap();
        let r = classify_ray(&m, [0.; 3], [1., 0.37, 0.19], 1e-7, 50000, 1000000).unwrap();
        assert_eq!(r.parity, Some(true), "{:?}", r.unresolved);
        assert_eq!(r.crossings.len(), 1);
    }
    #[test]
    fn exhausted_budget_and_edge_hits_do_not_return_a_parity() {
        let m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let r = classify_ray(&m, [0.3, 0.4, 0.5], [1., 0., 0.], 1e-7, 1, 1).unwrap();
        assert!(r.parity.is_none());
        assert!(r.cells <= 1 && r.domain_cells <= 1);
        let r = classify_ray(&m, [-1., 0., 0.5], [1., 0., 0.], 1e-7, 100, 1000).unwrap();
        assert!(r.parity.is_none());
    }
    #[test]
    fn profile_hole_changes_ray_parity() {
        use nurbs_core::curve::Curve;
        let curve = |p: Vec<[f64; 2]>| {
            Curve::from_polyline(p.into_iter().map(|p| p.to_vec()).collect()).unwrap()
        };
        let outer = curve(vec![[0., 0.], [10., 0.], [10., 10.], [0., 10.], [0., 0.]]);
        let hole = curve(vec![[4., 4.], [4., 6.], [6., 6.], [6., 4.], [4., 4.]]);
        let m = crate::prism::extrude(&[vec![outer], vec![hole]], 0., 1.).unwrap();
        for (p, parity, count) in [([5., 5., 0.5], false, 2), ([3., 5., 0.5], true, 3)] {
            let r = classify_ray(&m, p, [1., 0., 0.], 1e-7, 10000, 100000).unwrap();
            assert_eq!(r.parity, Some(parity), "{:?}", r.unresolved);
            assert_eq!(r.crossings.len(), count)
        }
    }
}
