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
use nurbs_core::{Error, Result, curve::Curve};
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
    domain: [f64; 2],
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
    if source_shared_edge::unique_world_plane_domain(
        world,
        plane,
        Some(domain),
        out,
        max_work,
        max_driver,
    )? {
        return Ok(true);
    }
    // The global original-coordinate proof also handles a zero derivative
    // at an endpoint. Keep that proof under the same remaining work budget.
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
    qualify_with_candidates(
        world,
        fragments,
        ranges,
        planes,
        [[None; 2]; 2],
        max_work,
        max_driver,
    )
}
/// Raw exact parameter candidates indexed by use and directed endpoint. Each
/// root candidate is rechecked against its original UV equations and selector.
pub fn qualify_with_candidates(
    world: &Curve,
    fragments: [&Fragment; 2],
    ranges: [Range; 2],
    planes: [Option<[[f64; 3]; 3]>; 2],
    candidates: [[Option<[f64; 2]>; 2]; 2],
    max_work: u64,
    max_driver: usize,
) -> Result<Report> {
    if planes
        .iter()
        .flatten()
        .flatten()
        .flatten()
        .any(|v| !v.is_finite())
        || candidates
            .iter()
            .flatten()
            .flatten()
            .flatten()
            .any(|v| !v.is_finite())
    {
        return Err(error());
    }
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
        let mut authored = [None; 2];
        for use_index in 0..2 {
            let endpoint_index = if use_index == 0 { i } else { 1 - i };
            authored[use_index] = match &fragments[use_index].endpoints()[endpoint_index] {
                Endpoint::Carrier { .. } => None,
                Endpoint::Parameter(t) => Some(*t),
                Endpoint::Crossing { point, role } => {
                    if let Some(candidate) = candidates[use_index][endpoint_index] {
                        if out.work_used == max_work {
                            return Ok(out);
                        }
                        let r = crate::source_root_parameter::verify(
                            point,
                            candidate,
                            (max_work - out.work_used).min(cad_predicates::MAX_WORK),
                        )?;
                        out.work_used += r.work_used;
                        out.root_checks += 1;
                        r.parameters.map(|p| {
                            p[match role {
                                Role::Boundary => 0,
                                Role::Contact => 1,
                            }]
                        })
                    } else {
                        None
                    }
                }
            };
        }
        let same = if let [Some(a), Some(b)] = authored {
            let da = fragments[0].curve().domain();
            let db = fragments[1].curve().domain();
            fixed_identity(
                [[a, da[0], da[1]], [b, db[0], db[1]]],
                ranges,
                &mut out,
                max_work,
            )?
        } else {
            match (
                &fragments[0].endpoints()[i],
                &fragments[1].endpoints()[1 - i],
            ) {
                (Endpoint::Carrier { .. }, _) | (_, Endpoint::Carrier { .. }) => {
                    crate::source_carrier_point::same_mapped_endpoint(
                        world,
                        fragments,
                        ranges,
                        [
                            &fragments[0].endpoints()[i],
                            &fragments[1].endpoints()[1 - i],
                        ],
                    )
                }
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
                        // Both original root enclosures are mapped outward
                        // into the same canonical parameter domain. Uniqueness
                        // is required on their entire contiguous union.
                        let aa = uses[0].parameter_bounds()?[i];
                        let bb = uses[1].parameter_bounds()?[1 - i];
                        let domain = [aa[0].min(bb[0]), aa[1].max(bb[1])];
                        source_shared_edge::common_plane_root_in_domain(
                            world,
                            plane,
                            [(a, *ar), (b, *br)],
                            domain,
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
                            world,
                            point,
                            *role,
                            *t,
                            &uses[1],
                            {
                                let aa = uses[0].parameter_bounds()?[i];
                                let bb = uses[1].parameter_bounds()?[1 - i];
                                [aa[0].min(bb[0]), aa[1].max(bb[1])]
                            },
                            plane,
                            &mut out,
                            max_work,
                            max_driver,
                        )?
                    } else {
                        false
                    }
                }
                (Endpoint::Parameter(t), Endpoint::Crossing { point, role }) => {
                    if let Some(plane) = planes[i] {
                        mixed_identity(
                            world,
                            point,
                            *role,
                            *t,
                            &uses[0],
                            {
                                let aa = uses[0].parameter_bounds()?[i];
                                let bb = uses[1].parameter_bounds()?[1 - i];
                                [aa[0].min(bb[0]), aa[1].max(bb[1])]
                            },
                            plane,
                            &mut out,
                            max_work,
                            max_driver,
                        )?
                    } else {
                        false
                    }
                }
            }
        };
        if !same {
            return Ok(out);
        }
    }
    out.edge = Some(SharedEdge::from_mapped(uses, planes, candidates));
    out.reason = "mapped-source-shared-edge-qualified";
    Ok(out)
}
#[cfg(test)]
mod tests;
