//! Fresh shared ownership for differently covered original source curves.
use crate::{
    source_affine_use::{self, MappedUse},
    source_boundary_fragment::{Endpoint, Fragment, Role},
    source_contact_point::SourcePoint,
    source_shared_edge::{self, Report, SharedEdge},
};
use cad_predicates::{
    AuthoredScalar, Limits, ParameterIdentity, PredicateContext, SourceArena, ToleranceContext,
};
use nurbs_core::{curve::Curve, Error, Result};
type Range = [[f64; 2]; 2];
fn error() -> Error {
    Error::new(
        "BREP_SOURCE_MAPPED_EDGE",
        "Invalid exact mapped endpoint input",
    )
}
fn fixed_identity(
    parameters: [[f64; 3]; 2],
    ranges: [Range; 2],
    out: &mut Report,
    max_work: u64,
) -> Result<bool> {
    if out.work_used == max_work {
        return Ok(false);
    }
    let values = parameters
        .iter()
        .flatten()
        .chain(ranges.iter().flatten().flatten())
        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
        .collect();
    let source = SourceArena::authored("mapped-fixed-endpoints", 1, values).map_err(|_| error())?;
    let pp = std::array::from_fn(|i| std::array::from_fn(|k| source.leaf(3 * i + k).unwrap()));
    let ranges = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            std::array::from_fn(|k| source.leaf(6 + 4 * i + 2 * j + k).unwrap())
        })
    });
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &source,
        &tolerance,
        Limits {
            max_work: (max_work - out.work_used).min(cad_predicates::MAX_WORK),
            ..Limits::default()
        },
        None,
    );
    let r = cad_predicates::affine_parameter_identity(&mut ctx, pp, ranges).map_err(|_| error())?;
    out.work_used += r.work_used;
    Ok(r.outcome == ParameterIdentity::Equal)
}
fn linear_identity(
    points: [(&SourcePoint, Role); 2],
    ranges: [Range; 2],
    out: &mut Report,
    max_work: u64,
) -> Result<bool> {
    let line = |c: &Curve| {
        let d = c.domain();
        c.degree == 1
            && c.control_points.len() == 2
            && c.knots[..2].iter().all(|&t| t == d[0])
            && c.knots[2..].iter().all(|&t| t == d[1])
    };
    let mut values = Vec::new();
    for (point, role) in points {
        let (main, other) = match role {
            Role::Boundary => (point.boundary(), point.contact()),
            Role::Contact => (point.contact(), point.boundary()),
        };
        if !line(main) || !line(other) {
            return Ok(false);
        }
        for (p, w) in main.control_points.iter().zip(&main.weights) {
            values.extend(p.iter().copied());
            values.push(*w);
        }
        for p in &other.control_points {
            values.extend(p.iter().copied());
        }
    }
    if out.work_used == max_work {
        return Ok(false);
    }
    values.extend(ranges.iter().flatten().flatten().copied());
    let source = SourceArena::authored(
        "mapped-linear-roots",
        1,
        values
            .iter()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .map_err(|_| error())?;
    let main = std::array::from_fn(|i| {
        std::array::from_fn(|j| std::array::from_fn(|k| source.leaf(10 * i + 3 * j + k).unwrap()))
    });
    let cutter = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            std::array::from_fn(|k| source.leaf(10 * i + 6 + 2 * j + k).unwrap())
        })
    });
    let ranges = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            std::array::from_fn(|k| source.leaf(20 + 4 * i + 2 * j + k).unwrap())
        })
    });
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &source,
        &tolerance,
        Limits {
            max_work: (max_work - out.work_used).min(cad_predicates::MAX_WORK),
            ..Limits::default()
        },
        None,
    );
    let r = cad_predicates::line_crossing_parameter_identity_affine(&mut ctx, main, cutter, ranges)
        .map_err(|_| error())?;
    out.work_used += r.work_used;
    Ok(r.outcome == ParameterIdentity::Equal)
}
fn mixed_identity(
    world: &Curve,
    point: &SourcePoint,
    role: Role,
    parameter: f64,
    fixed: &MappedUse,
    plane: [[f64; 3]; 3],
    out: &mut Report,
    max_work: u64,
    max_driver: usize,
) -> Result<bool> {
    out.root_checks += 1;
    if !source_shared_edge::source_point_cut_plane(point, role, plane, out, max_work)?
        || out.work_used == max_work
    {
        return Ok(false);
    }
    let d = fixed.fragment().curve().domain();
    let Some(r) = nurbs_core::curve_surface_plane::verify_curve_point_affine(
        world,
        plane,
        [parameter, d[0], d[1]],
        fixed.range(),
        (max_work - out.work_used).min(cad_predicates::MAX_WORK),
    )?
    else {
        return Ok(false);
    };
    out.work_used += r.work_used;
    if r.outcome != cad_predicates::BezierIdentity::Equal {
        return Ok(false);
    }
    source_shared_edge::unique_world_plane(world, plane, out, max_work, max_driver)
}
/// Full source compositions are freshly verified for both affine maps before
/// matching their actual retained ends. Scalar fractions are compared exactly;
/// nonlinear roots require a proven unique canonical world-plane intersection.
pub fn qualify(
    world: &Curve,
    fragments: [&Fragment; 2],
    ranges: [Range; 2],
    planes: [Option<[[f64; 3]; 3]>; 2],
    max_work: u64,
    max_driver: usize,
) -> Result<Report> {
    if !(1..=100_000_000).contains(&max_work) || max_driver > 100000 {
        return Err(error());
    }
    let mut out = Report {
        edge: None,
        work_used: 0,
        root_checks: 0,
        driver_cells: 0,
        reason: "mapped-source-composition-unproven",
    };
    let mut mapped = Vec::new();
    for i in 0..2 {
        if out.work_used == max_work {
            return Ok(out);
        }
        let r =
            source_affine_use::qualify(world, fragments[i], ranges[i], max_work - out.work_used)?;
        out.work_used += r.exact_work;
        let Some(m) = r.mapped else {
            out.reason = r.reason;
            return Ok(out);
        };
        mapped.push(m);
    }
    let uses = [mapped.remove(0), mapped.remove(0)];
    if uses[0].reversed() == uses[1].reversed() {
        out.reason = "mapped-source-uses-not-opposite";
        return Ok(out);
    }
    out.reason = "mapped-source-endpoints-unproven";
    for i in 0..2 {
        let same = match (
            &fragments[0].endpoints()[i],
            &fragments[1].endpoints()[1 - i],
        ) {
            (Endpoint::Parameter(a), Endpoint::Parameter(b)) => {
                let da = fragments[0].curve().domain();
                let db = fragments[1].curve().domain();
                fixed_identity(
                    [[*a, da[0], da[1]], [*b, db[0], db[1]]],
                    ranges,
                    &mut out,
                    max_work,
                )?
            }
            (
                Endpoint::Crossing { point: a, role: ar },
                Endpoint::Crossing { point: b, role: br },
            ) => {
                if linear_identity([(a, *ar), (b, *br)], ranges, &mut out, max_work)? {
                    true
                } else if let Some(plane) = planes[i] {
                    source_shared_edge::common_plane_root(
                        world,
                        plane,
                        [(a, *ar), (b, *br)],
                        &mut out,
                        max_work,
                        max_driver,
                    )?
                } else {
                    false
                }
            }
            (Endpoint::Crossing { point, role }, Endpoint::Parameter(t)) => {
                if let Some(plane) = planes[i] {
                    mixed_identity(
                        world, point, *role, *t, &uses[1], plane, &mut out, max_work, max_driver,
                    )?
                } else {
                    false
                }
            }
            (Endpoint::Parameter(t), Endpoint::Crossing { point, role }) => {
                if let Some(plane) = planes[i] {
                    mixed_identity(
                        world, point, *role, *t, &uses[0], plane, &mut out, max_work, max_driver,
                    )?
                } else {
                    false
                }
            }
        };
        if !same {
            return Ok(out);
        }
    }
    out.edge = Some(SharedEdge::from_mapped(uses));
    out.reason = "mapped-source-shared-edge-qualified";
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    use nurbs_core::surface::Surface;
    fn fixture() -> (Surface, Surface, Curve, Curve, Curve) {
        let surface = |vertical: bool| Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..2)
                .map(|u| {
                    (0..2)
                        .map(|v| {
                            if vertical {
                                vec![u as f64, 0., v as f64]
                            } else {
                                vec![u as f64, v as f64, 0.]
                            }
                        })
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let a = Curve {
            degree: 1,
            knots: vec![2., 2., 4., 4.],
            control_points: vec![vec![0., 0.], vec![1., 0.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        let b = Curve {
            degree: 1,
            knots: vec![10., 10., 18., 18.],
            control_points: vec![vec![0.25, 0.], vec![0.75, 0.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        let world = Curve {
            degree: 1,
            knots: vec![-5., -5., 3., 3.],
            control_points: vec![vec![0., 0., 0.], vec![1., 0., 0.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        (surface(false), surface(true), a, b, world)
    }
    const RANGES: [Range; 2] = [[[0., 1.], [1., 1.]], [[1., 4.], [3., 4.]]];
    #[test]
    fn different_geometric_coverage_has_exact_fixed_and_mixed_owned_ends() {
        let (sa, sb, a, b, world) = fixture();
        let fa =
            Fragment::new(&sa, &a, Endpoint::Parameter(2.5), Endpoint::Parameter(3.5)).unwrap();
        let fb =
            Fragment::new(&sb, &b, Endpoint::Parameter(18.), Endpoint::Parameter(10.)).unwrap();
        assert!(
            source_shared_edge::qualify(&world, [&fa, &fb], [false, false], 100_000_000)
                .unwrap()
                .edge
                .is_none()
        );
        let r = qualify(&world, [&fa, &fb], RANGES, [None, None], 100_000_000, 0).unwrap();
        assert!(r.edge.is_some(), "{}", r.reason);
        let e = r.edge.unwrap();
        assert_eq!(e.world(), &world);
        assert_eq!(e.uses()[1].curve(), &b);
        assert_eq!(e.ranges(), Some(RANGES));
        assert!(!e.covers_complete_canonical_source());
        assert_eq!(e.reversed(), [false, true]);
        let wrong = Fragment::new(
            &sb,
            &b,
            Endpoint::Parameter(18. - 1e-12),
            Endpoint::Parameter(10.),
        )
        .unwrap();
        assert!(
            qualify(&world, [&fa, &wrong], RANGES, [None, None], 100_000_000, 0)
                .unwrap()
                .edge
                .is_none()
        );
        let cut = Curve::from_polyline(vec![vec![0.75, -1.], vec![0.75, 1.]]).unwrap();
        let point =
            crate::source_contact_point::qualify(&sa, &a, &cut, [[3.4, 3.6], [0.4, 0.6]], 10000)
                .unwrap();
        assert!(point.point.is_some(), "{}", point.reason);
        let root = Fragment::new(
            &sa,
            &a,
            Endpoint::Parameter(2.5),
            Endpoint::Crossing {
                point: point.point.unwrap(),
                role: Role::Boundary,
            },
        )
        .unwrap();
        let plane = [[0.75, 0., 0.], [0.75, 1., 0.], [0.75, 0., 1.]];
        let r = qualify(
            &world,
            [&root, &fb],
            RANGES,
            [None, Some(plane)],
            100_000_000,
            1000,
        )
        .unwrap();
        assert!(r.edge.is_some(), "{}", r.reason);
        assert!(r.driver_cells > 0);
        assert!(qualify(
            &world,
            [&fb, &root],
            [RANGES[1], RANGES[0]],
            [Some(plane), None],
            100_000_000,
            1000
        )
        .unwrap()
        .edge
        .is_some());
        assert!(qualify(
            &world,
            [&root, &fb],
            RANGES,
            [None, Some(plane)],
            100_000_000,
            0
        )
        .unwrap()
        .edge
        .is_none());
        assert!(qualify(&world, [&fa, &fb], RANGES, [None, None], 1, 0)
            .unwrap()
            .edge
            .is_none());
    }
    #[test]
    fn linear_roots_on_different_source_coverage_keep_exact_canonical_parameters() {
        let (sa, sb, a, b, world) = fixture();
        let point = |s: &Surface, c: &Curve, x: f64, selector: [f64; 2]| {
            let cut = Curve::from_polyline(vec![vec![x, -1.], vec![x, 1.]]).unwrap();
            let r = crate::source_contact_point::qualify(s, c, &cut, [selector, [0.4, 0.6]], 10000)
                .unwrap();
            assert!(r.point.is_some(), "{}", r.reason);
            Endpoint::Crossing {
                point: r.point.unwrap(),
                role: Role::Boundary,
            }
        };
        let fa = Fragment::new(
            &sa,
            &a,
            point(&sa, &a, 0.4, [2.7, 2.9]),
            point(&sa, &a, 0.6, [3.1, 3.3]),
        )
        .unwrap();
        let fb = Fragment::new(
            &sb,
            &b,
            point(&sb, &b, 0.6, [15.5, 15.7]),
            point(&sb, &b, 0.4, [12.3, 12.5]),
        )
        .unwrap();
        let r = qualify(&world, [&fa, &fb], RANGES, [None, None], 100_000_000, 0).unwrap();
        assert!(r.edge.is_some(), "{}", r.reason);
        assert_eq!(r.driver_cells, 0);
        let e = r.edge.unwrap();
        assert!(e.uses().iter().all(|f| f
            .endpoints()
            .iter()
            .all(|p| matches!(p, Endpoint::Crossing { .. }))));
        let wrong = Fragment::new(
            &sb,
            &b,
            point(&sb, &b, 0.6 + 1e-12, [15.5, 15.7]),
            point(&sb, &b, 0.4, [12.3, 12.5]),
        )
        .unwrap();
        assert!(
            qualify(&world, [&fa, &wrong], RANGES, [None, None], 100_000_000, 0)
                .unwrap()
                .edge
                .is_none()
        );
    }
}
