//! Exact canonical world ownership for original source edges with qualified restrictions.
use crate::source_boundary_fragment::{Endpoint, Fragment, Role};
use cad_predicates::BezierIdentity;
use nurbs_core::{curve::Curve, curve_surface_agreement, Error, Result};
#[derive(Clone)]
pub struct SharedEdge {
    world: Curve,
    uses: [Fragment; 2],
    reversed: [bool; 2],
    ranges: Option<[[[f64; 2]; 2]; 2]>,
    recipe: value_codec::Value,
}
impl SharedEdge {
    pub fn world(&self) -> &Curve {
        &self.world
    }
    pub fn uses(&self) -> &[Fragment; 2] {
        &self.uses
    }
    pub fn ranges(&self) -> Option<[[[f64; 2]; 2]; 2]> {
        self.ranges
    }
    pub fn covers_complete_canonical_source(&self) -> bool {
        self.ranges.is_none_or(|ranges| {
            ranges.iter().all(|r| {
                (r[0][0] == 0. && r[1][0] == r[1][1]) || (r[1][0] == 0. && r[0][0] == r[0][1])
            })
        })
    }
    pub(crate) fn from_mapped(
        uses: [crate::source_affine_use::MappedUse; 2],
        planes: [Option<[[f64; 3]; 3]>; 2],
        candidates: [[Option<[f64; 2]>; 2]; 2],
    ) -> Self {
        Self {
            world: uses[0].world().clone(),
            uses: [uses[0].fragment().clone(), uses[1].fragment().clone()],
            reversed: [uses[0].reversed(), uses[1].reversed()],
            ranges: Some([uses[0].range(), uses[1].range()]),
            recipe: value_codec::json!({"kind":"mapped","ranges":[uses[0].range(),uses[1].range()],"planes":planes,"candidates":candidates}),
        }
    }
    /// Original definitions and proposal inputs only; restoration recomputes authority.
    pub fn definition(&self) -> value_codec::Value {
        use value_codec::Serialize;
        value_codec::json!({"version":1,"world":self.world.to_value(),
            "uses":self.uses.each_ref().map(|f|f.definition()),"recipe":self.recipe.clone()})
    }
    /// Direction of each directed fragment relative to the canonical world curve.
    pub fn reversed(&self) -> [bool; 2] {
        self.reversed
    }
}
pub struct Report {
    pub edge: Option<SharedEdge>,
    pub work_used: u64,
    pub root_checks: usize,
    pub driver_cells: usize,
    pub reason: &'static str,
}
/// `world_reversed` relates canonical world traversal to each forward UV curve.
/// No caller-supplied certificate, shared key or world proximity is authority.
pub fn qualify(
    world: &Curve,
    uses: [&Fragment; 2],
    world_reversed: [bool; 2],
    max_work: u64,
) -> Result<Report> {
    qualify_with_cutters(world, uses, world_reversed, [None, None], max_work)
}
/// Optional canonical crossing curves for endpoints of the first directed use.
/// Their full source compositions are independently checked before root identity.
pub fn qualify_with_cutters(
    world: &Curve,
    uses: [&Fragment; 2],
    world_reversed: [bool; 2],
    cutters: [Option<&Curve>; 2],
    max_work: u64,
) -> Result<Report> {
    qualify_impl(
        world,
        uses,
        world_reversed,
        cutters,
        [None, None],
        max_work,
        0,
    )
}
/// Optional exact source planes for root-valued ends of the first directed use.
/// Both local crossing curves must map into the supplied plane; the canonical
/// world curve must meet it at at most one parameter by strict monotonicity.
pub fn qualify_with_planes(
    world: &Curve,
    uses: [&Fragment; 2],
    world_reversed: [bool; 2],
    planes: [Option<[[f64; 3]; 3]>; 2],
    max_work: u64,
    max_driver_cells: usize,
) -> Result<Report> {
    qualify_impl(
        world,
        uses,
        world_reversed,
        [None, None],
        planes,
        max_work,
        max_driver_cells,
    )
}
/// Recheck optional canonical cutters and planes under the same identity budget.
pub fn qualify_with_cutters_and_planes(
    world: &Curve,
    uses: [&Fragment; 2],
    world_reversed: [bool; 2],
    cutters: [Option<&Curve>; 2],
    planes: [Option<[[f64; 3]; 3]>; 2],
    max_work: u64,
    max_driver_cells: usize,
) -> Result<Report> {
    qualify_impl(
        world,
        uses,
        world_reversed,
        cutters,
        planes,
        max_work,
        max_driver_cells,
    )
}
fn qualify_impl(
    world: &Curve,
    uses: [&Fragment; 2],
    world_reversed: [bool; 2],
    cutters: [Option<&Curve>; 2],
    planes: [Option<[[f64; 3]; 3]>; 2],
    max_work: u64,
    max_driver_cells: usize,
) -> Result<Report> {
    if max_work == 0 || max_work > 100_000_000 || max_driver_cells > 100000 {
        return Err(Error::new(
            "BREP_SOURCE_SHARED_EDGE",
            "Choose bounded exact identity work",
        ));
    }
    world.validate()?;
    if world.control_points[0].len() != 3 {
        return Err(Error::new(
            "BREP_SOURCE_SHARED_EDGE",
            "Canonical world edge must be 3D",
        ));
    }
    let mut out = Report {
        edge: None,
        work_used: 0,
        root_checks: 0,
        driver_cells: 0,
        reason: "source-restriction-identity-unproven",
    };
    let complete = uses.iter().all(|edge| {
        let d = edge.curve().domain();
        let expected = if edge.reversed() { [d[1], d[0]] } else { d };
        (0..2).all(|i| matches!(&edge.endpoints()[i], Endpoint::Parameter(t) if t.to_bits() == expected[i].to_bits()))
    });
    if !complete {
        // Exact full-source identity uses normalized traversal. Compare source
        // parameters by exact affine fractions, even when their domains differ.
        // Root identity follows exact equations and a fresh common-root proof, not
        // overlapping isolating intervals. Different UV equations require
        // independent canonical cutter identity and projected root uniqueness.
        for i in 0..2 {
            let same = match (&uses[0].endpoints()[i], &uses[1].endpoints()[1 - i]) {
                (Endpoint::Parameter(a), Endpoint::Parameter(b)) => same_normalized_parameter(
                    *a,
                    *b,
                    [uses[0].curve().domain(), uses[1].curve().domain()],
                    world_reversed,
                    &mut out,
                    max_work,
                )?,
                (
                    Endpoint::Crossing { point: a, role: ar },
                    Endpoint::Crossing { point: b, role: br },
                ) => {
                    if world_reversed[0] != world_reversed[1]
                        || ar != br
                        || a.boundary() != b.boundary()
                        || a.contact() != b.contact()
                    {
                        if let Some(cutter) = cutters[i] {
                            let same = if world_reversed == [false, false] {
                                common_world_root(
                                    world,
                                    cutter,
                                    [(a, *ar), (b, *br)],
                                    &mut out,
                                    max_work,
                                )?
                            } else {
                                false
                            };
                            if same {
                                true
                            } else if let Some(plane) = planes[i] {
                                common_plane_root(
                                    world,
                                    plane,
                                    [(a, *ar), (b, *br)],
                                    &mut out,
                                    max_work,
                                    max_driver_cells,
                                )?
                            } else {
                                false
                            }
                        } else {
                            let same = same_linear_parameter(
                                [(a, *ar), (b, *br)],
                                world_reversed,
                                &mut out,
                                max_work,
                            )?;
                            let (chart_same, work, checks) = if same {
                                (false, 0, 0)
                            } else {
                                crate::source_line_chart_root::same(
                                    [(a, *ar), (b, *br)],
                                    world_reversed,
                                    max_work - out.work_used,
                                )?
                            };
                            out.work_used += work;
                            out.root_checks += checks;
                            if same || chart_same {
                                true
                            } else if let Some(plane) = planes[i] {
                                common_plane_root(
                                    world,
                                    plane,
                                    [(a, *ar), (b, *br)],
                                    &mut out,
                                    max_work,
                                    max_driver_cells,
                                )?
                            } else {
                                false
                            }
                        }
                    } else if a.selector() == b.selector() {
                        true
                    } else {
                        let selector: [[f64; 2]; 2] = std::array::from_fn(|axis| {
                            [
                                a.selector()[axis][0].max(b.selector()[axis][0]),
                                a.selector()[axis][1].min(b.selector()[axis][1]),
                            ]
                        });
                        if selector.iter().any(|r| r[0] >= r[1]) {
                            false
                        } else {
                            // A freshly certified root inside both unique-root
                            // selectors is their common root. Overlap alone is
                            // never an identity argument. At most two fresh
                            // one-box queries are needed for this edge pair.
                            out.root_checks += 1;
                            nurbs_core::uv_curve_crossings::certify_box(
                                a.boundary(),
                                a.contact(),
                                selector,
                            )?
                            .state
                                == nurbs_core::uv_curve_crossings::State::Unique
                        }
                    }
                }
                (Endpoint::Crossing { point, role }, Endpoint::Parameter(t)) => {
                    if let Some(plane) = planes[i] {
                        common_plane_fixed_root(
                            world,
                            plane,
                            point,
                            *role,
                            [*t, uses[1].curve().domain()[0], uses[1].curve().domain()[1]],
                            world_reversed[1],
                            &mut out,
                            max_work,
                            max_driver_cells,
                        )?
                    } else {
                        false
                    }
                }
                (Endpoint::Parameter(t), Endpoint::Crossing { point, role }) => {
                    if let Some(plane) = planes[i] {
                        common_plane_fixed_root(
                            world,
                            plane,
                            point,
                            *role,
                            [*t, uses[0].curve().domain()[0], uses[0].curve().domain()[1]],
                            world_reversed[0],
                            &mut out,
                            max_work,
                            max_driver_cells,
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
    }
    let effective = [
        uses[0].reversed() ^ world_reversed[0],
        uses[1].reversed() ^ world_reversed[1],
    ];
    if effective[0] == effective[1] {
        out.reason = "source-edge-uses-not-opposite";
        return Ok(out);
    }
    for (i, edge) in uses.iter().enumerate() {
        if out.work_used == max_work {
            out.reason = "source-edge-identity-work-limit";
            return Ok(out);
        }
        // Fragment construction independently proves its whole restriction in
        // the positive-weight source chart. Formal homogeneous identity then
        // proves equality there, without requiring the unused UV tails inside.
        let Some(proof) = curve_surface_agreement::verify_exact_algebraic(
            world,
            edge.curve(),
            edge.surface(),
            world_reversed[i],
            (max_work - out.work_used).min(cad_predicates::MAX_WORK),
        )?
        else {
            out.reason = "source-world-identity-layout-unproven";
            return Ok(out);
        };
        out.work_used += proof.work_used;
        if proof.outcome != BezierIdentity::Equal {
            out.reason = "source-world-identity-unproven";
            return Ok(out);
        }
    }
    out.edge = Some(SharedEdge {
        world: world.clone(),
        uses: [uses[0].clone(), uses[1].clone()],
        reversed: effective,
        ranges: None,
        recipe: value_codec::json!({"kind":"direct","worldReversed":world_reversed,
            "cutters":cutters.map(|c|c.cloned()),"planes":planes}),
    });
    out.reason = "source-shared-world-edge-qualified";
    Ok(out)
}
pub(crate) fn common_plane_root(
    world: &Curve,
    plane: [[f64; 3]; 3],
    points: [(&crate::source_contact_point::SourcePoint, Role); 2],
    out: &mut Report,
    max_work: u64,
    max_driver_cells: usize,
) -> Result<bool> {
    out.root_checks += 1;
    for (point, role) in points {
        if !source_point_cut_plane(point, role, plane, out, max_work)? {
            return Ok(false);
        }
    }
    unique_world_plane(world, plane, out, max_work, max_driver_cells)
}
pub(crate) fn source_point_cut_plane(
    point: &crate::source_contact_point::SourcePoint,
    role: Role,
    plane: [[f64; 3]; 3],
    out: &mut Report,
    max_work: u64,
) -> Result<bool> {
    if out.work_used == max_work {
        return Ok(false);
    }
    let other = match role {
        Role::Boundary => point.contact(),
        Role::Contact => point.boundary(),
    };
    // Actual source crossing membership and positive chart denominator are
    // immutable SourcePoint authority, separate from this formal plane identity.
    let Some(r) = nurbs_core::curve_surface_plane::verify_algebraic(
        other,
        point.surface(),
        plane,
        (max_work - out.work_used).min(cad_predicates::MAX_WORK),
    )?
    else {
        return Ok(false);
    };
    out.work_used += r.work_used;
    Ok(r.outcome == BezierIdentity::Equal)
}
fn common_plane_fixed_root(
    world: &Curve,
    plane: [[f64; 3]; 3],
    point: &crate::source_contact_point::SourcePoint,
    role: Role,
    parameter: [f64; 3],
    reversed: bool,
    out: &mut Report,
    max_work: u64,
    max_driver_cells: usize,
) -> Result<bool> {
    out.root_checks += 1;
    if !source_point_cut_plane(point, role, plane, out, max_work)? || out.work_used == max_work {
        return Ok(false);
    }
    let Some(r) = nurbs_core::curve_surface_plane::verify_curve_point(
        world,
        plane,
        parameter,
        reversed,
        (max_work - out.work_used).min(cad_predicates::MAX_WORK),
    )?
    else {
        return Ok(false);
    };
    out.work_used += r.work_used;
    if r.outcome != BezierIdentity::Equal {
        return Ok(false);
    }
    unique_world_plane(world, plane, out, max_work, max_driver_cells)
}
pub(crate) fn unique_world_plane(
    world: &Curve,
    plane: [[f64; 3]; 3],
    out: &mut Report,
    max_work: u64,
    max_driver_cells: usize,
) -> Result<bool> {
    // Restricting an oblique plane to the original world curve must give a
    // nonconstant affine function of one monotone coordinate. Every other
    // nonzero normal coefficient requires an exactly constant source coordinate.
    // Normal coefficients are tested by exact projected anchor determinants.
    let mut components = [false; 3];
    for k in 0..3 {
        let axes = [(k + 1) % 3, (k + 2) % 3];
        let Some(sign) = crate::source_allowed_contact::orient(
            &plane,
            Some(axes),
            &mut out.work_used,
            max_work,
        )?
        else {
            return Ok(false);
        };
        components[k] = sign != cad_predicates::Sign::Zero;
    }
    for axis in 0..3 {
        if !components[axis]
            || (0..3).any(|k| {
                k != axis
                    && components[k]
                    && world
                        .control_points
                        .iter()
                        .any(|p| p[k] != world.control_points[0][k])
            })
        {
            continue;
        }
        if out.driver_cells == max_driver_cells {
            return Ok(false);
        }
        let d = world.domain();
        if world.periodic
            || world.knots[..=world.degree].iter().any(|&k| k != d[0])
            || world.knots[world.control_points.len()..]
                .iter()
                .any(|&k| k != d[1])
        {
            return Ok(false);
        }
        let r = nurbs_core::curve_axis_driver::certify(
            world,
            axis,
            max_driver_cells - out.driver_cells,
        )?;
        out.driver_cells += r.visited;
        if r.monotonic_proven {
            return Ok(true);
        }
    }
    Ok(false)
}
fn same_normalized_parameter(
    a: f64,
    b: f64,
    domains: [[f64; 2]; 2],
    reversed: [bool; 2],
    out: &mut Report,
    max_work: u64,
) -> Result<bool> {
    use cad_predicates::{
        AuthoredScalar, Limits, ParameterIdentity, PredicateContext, SourceArena, ToleranceContext,
    };
    if out.work_used == max_work {
        return Ok(false);
    }
    let arena = SourceArena::authored(
        "source-normalized-parameter",
        1,
        [
            a,
            domains[0][0],
            domains[0][1],
            b,
            domains[1][0],
            domains[1][1],
        ]
        .iter()
        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
        .collect(),
    )
    .map_err(|_| {
        Error::new(
            "BREP_SOURCE_SHARED_EDGE",
            "Invalid normalized parameter source",
        )
    })?;
    let tol = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tol,
        Limits {
            max_work: (max_work - out.work_used).min(cad_predicates::MAX_WORK),
            ..Limits::default()
        },
        None,
    );
    let r = cad_predicates::normalized_parameter_identity(
        &mut ctx,
        std::array::from_fn(|i| std::array::from_fn(|k| arena.leaf(3 * i + k).unwrap())),
        reversed,
    )
    .map_err(|_| {
        Error::new(
            "BREP_SOURCE_SHARED_EDGE",
            "Invalid normalized parameter request",
        )
    })?;
    out.work_used += r.work_used;
    Ok(r.outcome == ParameterIdentity::Equal)
}
// For transverse original linear Beziers, the UV equations themselves give
// an exact rational parameter. Comparing fractions does not round either root.
fn same_linear_parameter(
    points: [(&crate::source_contact_point::SourcePoint, Role); 2],
    reversed: [bool; 2],
    out: &mut Report,
    max_work: u64,
) -> Result<bool> {
    use cad_predicates::{
        AuthoredScalar, Limits, ParameterIdentity, PredicateContext, SourceArena, ToleranceContext,
    };
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
    let arena = SourceArena::authored(
        "source-crossing-parameter",
        1,
        values
            .iter()
            .map(|v: &f64| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .map_err(|_| Error::new("BREP_SOURCE_SHARED_EDGE", "Invalid exact root source"))?;
    let main = std::array::from_fn(|i| {
        std::array::from_fn(|j| std::array::from_fn(|k| arena.leaf(10 * i + 3 * j + k).unwrap()))
    });
    let cutter = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            std::array::from_fn(|k| arena.leaf(10 * i + 6 + 2 * j + k).unwrap())
        })
    });
    let tol = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tol,
        Limits {
            max_work: (max_work - out.work_used).min(cad_predicates::MAX_WORK),
            ..Limits::default()
        },
        None,
    );
    let proof =
        cad_predicates::line_crossing_parameter_identity_oriented(&mut ctx, main, cutter, reversed)
            .map_err(|_| {
                Error::new(
                    "BREP_SOURCE_SHARED_EDGE",
                    "Invalid exact root identity request",
                )
            })?;
    out.work_used += proof.work_used;
    Ok(proof.outcome == ParameterIdentity::Equal)
}
// Both known source roots must lie inside a box on which a projection of the
// canonical 3D crossing equations has exactly one root. Thus projection cannot
// introduce ambiguity between the two actual roots, even on different charts.
fn common_world_root(
    world: &Curve,
    cutter: &Curve,
    points: [(&crate::source_contact_point::SourcePoint, Role); 2],
    out: &mut Report,
    max_work: u64,
) -> Result<bool> {
    let mut ranges = [[[0.; 2]; 2]; 2];
    for (i, (point, role)) in points.iter().enumerate() {
        let (main, other, index) = match role {
            Role::Boundary => (point.boundary(), point.contact(), 0),
            Role::Contact => (point.contact(), point.boundary(), 1),
        };
        if out.work_used == max_work {
            return Ok(false);
        }
        let Some(proof) = curve_surface_agreement::verify_exact_algebraic(
            cutter,
            other,
            point.surface(),
            false,
            (max_work - out.work_used).min(cad_predicates::MAX_WORK),
        )?
        else {
            return Ok(false);
        };
        out.work_used += proof.work_used;
        if proof.outcome != BezierIdentity::Equal {
            return Ok(false);
        }
        let mapped = |range: [f64; 2], source: [f64; 2], target: [f64; 2]| -> Result<[f64; 2]> {
            use nurbs_core::interval_eval::Interval as I;
            let normalized = I::new(range[0], range[1])?
                .sub(I::point(source[0]))?
                .div(I::point(source[1]).sub(I::point(source[0]))?)?
                .intersect(0., 1.)?;
            let value = I::point(target[0])
                .add(normalized.mul(I::point(target[1]).sub(I::point(target[0]))?)?)?
                .intersect(target[0], target[1])?;
            Ok([value.lo, value.hi])
        };
        ranges[i] = [
            mapped(point.selector()[index], main.domain(), world.domain())?,
            mapped(point.selector()[1 - index], other.domain(), cutter.domain())?,
        ];
    }
    let box_: [[f64; 2]; 2] = std::array::from_fn(|axis| {
        [
            ranges[0][axis][0].min(ranges[1][axis][0]),
            ranges[0][axis][1].max(ranges[1][axis][1]),
        ]
    });
    for axes in [[0, 1], [0, 2], [1, 2]] {
        let project = |c: &Curve| {
            let mut p = c.clone();
            p.control_points = c
                .control_points
                .iter()
                .map(|v| vec![v[axes[0]], v[axes[1]]])
                .collect();
            p
        };
        out.root_checks += 1;
        if nurbs_core::uv_curve_crossings::certify_box(&project(world), &project(cutter), box_)?
            .state
            == nurbs_core::uv_curve_crossings::State::Unique
        {
            return Ok(true);
        }
    }
    Ok(false)
}
#[cfg(test)]
mod tests {
    use super::*;
    use nurbs_core::surface::Surface;
    #[test]
    fn mixed_root_and_fixed_endpoints_need_exact_point_membership_and_unique_world_root() {
        use crate::source_boundary_fragment::{Endpoint, Role};
        use nurbs_core::surface::Surface;
        let cap = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let side = Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![
                vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 1.], vec![1., 1., 1.], vec![0., 1., 1.]],
            ],
            weights: vec![vec![1.; 3]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let main_a = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let main_b = Curve {
            degree: 1,
            knots: vec![10., 10., 18., 18.],
            control_points: vec![vec![0., 1.], vec![0., 0.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        let cut = Curve::from_polyline(vec![vec![0.609375, 1.25], vec![0.609375, -0.25]]).unwrap();
        let point = crate::source_contact_point::qualify(
            &cap,
            &main_a,
            &cut,
            [[0.6, 0.65], [0.24, 0.28]],
            10000,
        )
        .unwrap();
        assert!(point.point.is_some(), "{}", point.reason);
        let fa = Fragment::new(
            &cap,
            &main_a,
            Endpoint::Parameter(0.),
            Endpoint::Crossing {
                point: point.point.unwrap(),
                role: Role::Boundary,
            },
        )
        .unwrap();
        let fb = Fragment::new(
            &side,
            &main_b,
            Endpoint::Parameter(13.),
            Endpoint::Parameter(18.),
        )
        .unwrap();
        let world = Curve {
            degree: 2,
            knots: vec![-5., -5., -5., 3., 3., 3.],
            control_points: side.control_points[0].clone(),
            weights: vec![1.; 3],
            periodic: false,
        };
        let plane = [[0.609375, 0., 0.], [0.609375, 1., 0.], [0.609375, 0., 1.]];
        assert!(qualify(&world, [&fa, &fb], [false, true], 100_000_000)
            .unwrap()
            .edge
            .is_none());
        let r = qualify_with_planes(
            &world,
            [&fa, &fb],
            [false, true],
            [None, Some(plane)],
            100_000_000,
            10000,
        )
        .unwrap();
        assert!(r.edge.is_some(), "{}", r.reason);
        assert_eq!(r.root_checks, 1);
        assert!(r.driver_cells > 0);
        let e = r.edge.unwrap();
        crate::source_edge_restriction::assert_replay(&e);
        let restriction = crate::source_edge_restriction::Restriction::from_edge(&e);
        let b = restriction.parameter_bounds().unwrap()[1];
        assert!(b[0] <= 0.0 && 0.0 <= b[1]);
        let b = restriction.endpoint_boxes(100).unwrap()[1];
        for (axis, expected) in [0.609375, 0.859375, 0.].into_iter().enumerate() {
            assert!(b[axis][0] <= expected && expected <= b[axis][1]);
        }
        let restore = |definition, work| crate::source_shared_edge_restore::restore(definition,
            crate::source_shared_edge_restore::Limits {mapping_cells_per_use:10000,exact_work:work,driver_cells:10000}).unwrap();
        let restored = restore(e.definition(),100_000_000).edge.unwrap();
        assert_eq!(restored.definition(),e.definition());
        assert_eq!(restored.reversed(),e.reversed());
        assert!(restore(e.definition(),1).edge.is_none());
        let mut missing_plane=e.definition();
        missing_plane["recipe"]["planes"][1]=value_codec::Value::Null;
        missing_plane["certificateQualified"]=value_codec::Value::Bool(true);
        assert!(restore(missing_plane,100_000_000).edge.is_none());

        assert_eq!(e.world(), &world);
        assert_eq!(e.uses()[1].curve(), &main_b);
        assert!(qualify_with_planes(
            &world,
            [&fb, &fa],
            [true, false],
            [Some(plane), None],
            100_000_000,
            10000
        )
        .unwrap()
        .edge
        .is_some());
        let wrong = Fragment::new(
            &side,
            &main_b,
            Endpoint::Parameter(13. + 1e-12),
            Endpoint::Parameter(18.),
        )
        .unwrap();
        assert!(qualify_with_planes(
            &world,
            [&fa, &wrong],
            [false, true],
            [None, Some(plane)],
            100_000_000,
            10000
        )
        .unwrap()
        .edge
        .is_none());
        assert!(qualify_with_planes(
            &world,
            [&fa, &fb],
            [false, true],
            [None, Some(plane)],
            100_000_000,
            0
        )
        .unwrap()
        .edge
        .is_none());
        assert!(qualify_with_planes(
            &world,
            [&fa, &fb],
            [false, true],
            [None, Some(plane)],
            1,
            10000
        )
        .unwrap()
        .edge
        .is_none());
    }
    #[test]
    fn distinct_original_domains_share_exact_normalized_parameters_and_rational_roots() {
        use crate::source_boundary_fragment::{Endpoint, Role};
        use nurbs_core::surface::Surface;
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
        let sa = surface(false);
        let sb = surface(true);
        let a = Curve {
            degree: 1,
            knots: vec![2., 2., 4., 4.],
            control_points: vec![vec![0., 0.], vec![1., 0.]],
            weights: vec![1., 2.],
            periodic: false,
        };
        let b = Curve {
            degree: 1,
            knots: vec![10., 10., 18., 18.],
            control_points: vec![vec![1., 0.], vec![0., 0.]],
            weights: vec![2., 1.],
            periodic: false,
        };
        let world = Curve {
            degree: 1,
            knots: vec![-5., -5., 3., 3.],
            control_points: vec![vec![0., 0., 0.], vec![1., 0., 0.]],
            weights: vec![1., 2.],
            periodic: false,
        };
        let fa =
            Fragment::new(&sa, &a, Endpoint::Parameter(2.5), Endpoint::Parameter(3.5)).unwrap();
        let fb =
            Fragment::new(&sb, &b, Endpoint::Parameter(12.), Endpoint::Parameter(16.)).unwrap();
        let r = qualify(&world, [&fa, &fb], [false, true], 100_000_000).unwrap();
        assert!(r.edge.is_some(), "{}", r.reason);
        let edge = r.edge.unwrap();
        assert_eq!(edge.world(), &world);
        assert_eq!(edge.uses()[0].curve(), &a);
        assert_eq!(edge.uses()[1].curve(), &b);
        let wrong = Fragment::new(
            &sb,
            &b,
            Endpoint::Parameter(12.),
            Endpoint::Parameter(16. + 1e-12),
        )
        .unwrap();
        assert!(qualify(&world, [&fa, &wrong], [false, true], 100_000_000)
            .unwrap()
            .edge
            .is_none());
        let cut_a = Curve {
            degree: 1,
            knots: vec![20., 20., 24., 24.],
            control_points: vec![vec![0.5, -1.], vec![0.5, 1.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        let cut_b = Curve {
            degree: 1,
            knots: vec![-4., -4., 2., 2.],
            control_points: cut_a.control_points.clone(),
            weights: vec![1.; 2],
            periodic: false,
        };
        let pa = crate::source_contact_point::qualify(
            &sa,
            &a,
            &cut_a,
            [[2.6, 2.8], [21.6, 22.4]],
            10000,
        )
        .unwrap();
        let pb = crate::source_contact_point::qualify(
            &sb,
            &b,
            &cut_b,
            [[15.2, 15.5], [-1.5, -0.5]],
            10000,
        )
        .unwrap();
        assert!(pa.point.is_some(), "{}", pa.reason);
        assert!(pb.point.is_some(), "{}", pb.reason);
        let ra = Fragment::new(
            &sa,
            &a,
            Endpoint::Parameter(2.),
            Endpoint::Crossing {
                point: pa.point.unwrap(),
                role: Role::Boundary,
            },
        )
        .unwrap();
        let rb = Fragment::new(
            &sb,
            &b,
            Endpoint::Crossing {
                point: pb.point.unwrap(),
                role: Role::Boundary,
            },
            Endpoint::Parameter(18.),
        )
        .unwrap();
        let root = qualify(&world, [&ra, &rb], [false, true], 100_000_000).unwrap();
        assert!(root.edge.is_some(), "{}", root.reason);
        assert_eq!(root.edge.unwrap().uses()[0].curve().domain(), [2., 4.]);
        assert!(qualify(&world, [&ra, &rb], [false, true], 1)
            .unwrap()
            .edge
            .is_none());
    }
    #[test]
    fn two_world_plane_roots_never_become_one_shared_endpoint() {
        use crate::source_boundary_fragment::{Endpoint, Role};
        use nurbs_core::surface::Surface;
        let cap = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let side = Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![
                vec![vec![0.5, 0., 0.], vec![0., 0.5, 0.], vec![0.5, 1., 0.]],
                vec![vec![0.5, 0., 1.], vec![0., 0.5, 1.], vec![0.5, 1., 1.]],
            ],
            weights: vec![vec![1.; 3]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let main_a = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0.5, 0.], vec![0., 0.5], vec![0.5, 1.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let main_b = Curve::from_polyline(vec![vec![0., 1.], vec![0., 0.]]).unwrap();
        let cut_a = Curve::from_polyline(vec![vec![0.375, -0.25], vec![0.375, 1.25]]).unwrap();
        let cut_b = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0.125, 0.], vec![-0.375, 0.5], vec![0.125, 1.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let a = crate::source_contact_point::qualify(
            &cap,
            &main_a,
            &cut_a,
            [[0.14, 0.16], [0.25, 0.28]],
            10000,
        )
        .unwrap();
        let b = crate::source_contact_point::qualify(
            &side,
            &main_b,
            &cut_b,
            [[0.14, 0.16], [0.84, 0.87]],
            10000,
        )
        .unwrap();
        assert!(a.point.is_some(), "{}", a.reason);
        assert!(b.point.is_some(), "{}", b.reason);
        let fa = Fragment::new(
            &cap,
            &main_a,
            Endpoint::Parameter(0.),
            Endpoint::Crossing {
                point: a.point.unwrap(),
                role: Role::Boundary,
            },
        )
        .unwrap();
        let fb = Fragment::new(
            &side,
            &main_b,
            Endpoint::Crossing {
                point: b.point.unwrap(),
                role: Role::Boundary,
            },
            Endpoint::Parameter(1.),
        )
        .unwrap();
        let world = Curve {
            degree: 2,
            knots: main_a.knots.clone(),
            control_points: side.control_points[0].clone(),
            weights: vec![1.; 3],
            periodic: false,
        };
        let plane = [[0.375, 0., 0.], [0.375, 1., 0.], [1.375, 0., 1.]];
        // Both exact crossing images lie in this plane, but its two world
        // roots own different canonical parameters. Plane membership alone
        // must never turn these restrictions into a shared edge.
        let r = qualify_with_planes(
            &world,
            [&fa, &fb],
            [false, true],
            [None, Some(plane)],
            100_000_000,
            100,
        )
        .unwrap();
        assert!(r.edge.is_none());
        assert!(r.driver_cells <= 100);
        let fixed_cut =
            Curve::from_polyline(vec![vec![0.3125, -0.25], vec![0.3125, 1.25]]).unwrap();
        let fixed_root = crate::source_contact_point::qualify(
            &cap,
            &main_a,
            &fixed_cut,
            [[0.2, 0.3], [0.3, 0.36]],
            10000,
        )
        .unwrap();
        assert!(fixed_root.point.is_some(), "{}", fixed_root.reason);
        let root_use = Fragment::new(
            &cap,
            &main_a,
            Endpoint::Parameter(0.),
            Endpoint::Crossing {
                point: fixed_root.point.unwrap(),
                role: Role::Boundary,
            },
        )
        .unwrap();
        let parameter_use = Fragment::new(
            &side,
            &main_b,
            Endpoint::Parameter(0.25),
            Endpoint::Parameter(1.),
        )
        .unwrap();
        let fixed_plane = [[0.3125, 0., 0.], [0.3125, 1., 0.], [1.3125, 0., 1.]];
        // Fixed q=.75 and crossing q=.25 both satisfy the same plane exactly.
        // Mixed endpoint admission still requires a unique canonical plane root.
        let mixed = qualify_with_planes(
            &world,
            [&root_use, &parameter_use],
            [false, true],
            [None, Some(fixed_plane)],
            100_000_000,
            100,
        )
        .unwrap();
        assert!(mixed.edge.is_none());
        assert!(mixed.driver_cells > 0 && mixed.driver_cells <= 100);
    }
    #[test]
    fn nonlinear_local_crossings_share_only_a_fresh_unique_world_plane_root() {
        use crate::source_boundary_fragment::{Endpoint, Role};
        use nurbs_core::surface::Surface;
        let cap = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let side = Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![
                vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 1.], vec![1., 1., 1.], vec![0., 1., 1.]],
            ],
            weights: vec![vec![1.; 3]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let main_a = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let main_b = Curve::from_polyline(vec![vec![0., 1.], vec![0., 0.]]).unwrap();
        let cut_a = Curve::from_polyline(vec![vec![0.5, 1.25], vec![0.5, -0.25]]).unwrap();
        let cut_b = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0.4375, -0.25], vec![0.8125, 0.5], vec![-1.0625, 1.25]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let a = crate::source_contact_point::qualify(
            &cap,
            &main_a,
            &cut_a,
            [[0.65, 0.8], [0.15, 0.3]],
            10000,
        )
        .unwrap();
        let b = crate::source_contact_point::qualify(
            &side,
            &main_b,
            &cut_b,
            [[0.2, 0.4], [0.55, 0.75]],
            10000,
        )
        .unwrap();
        assert!(a.point.is_some(), "{}", a.reason);
        assert!(b.point.is_some(), "{}", b.reason);
        let a = a.point.unwrap();
        let b = b.point.unwrap();
        let fa = Fragment::new(
            &cap,
            &main_a,
            Endpoint::Parameter(0.),
            Endpoint::Crossing {
                point: a,
                role: Role::Boundary,
            },
        )
        .unwrap();
        let fb = Fragment::new(
            &side,
            &main_b,
            Endpoint::Crossing {
                point: b,
                role: Role::Boundary,
            },
            Endpoint::Parameter(1.),
        )
        .unwrap();
        let world = Curve {
            degree: 2,
            knots: main_a.knots.clone(),
            control_points: vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![0., 1., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let plane = [[0.5, 0., 0.], [0.5, 1., 0.], [1.5, 0., 1.]];
        assert!(qualify(&world, [&fa, &fb], [false, true], 100_000_000)
            .unwrap()
            .edge
            .is_none());
        let r = qualify_with_planes(
            &world,
            [&fa, &fb],
            [false, true],
            [None, Some(plane)],
            100_000_000,
            10000,
        )
        .unwrap();
        assert!(r.edge.is_some(), "{} {}", r.reason, r.work_used);
        assert_eq!(r.edge.unwrap().world(), &world);
        assert!(r.driver_cells > 0);
        let mut alternate = world.clone();
        alternate.knots = vec![-5., -5., -5., 3., 3., 3.];
        let remapped = qualify_with_planes(
            &alternate,
            [&fa, &fb],
            [false, true],
            [None, Some(plane)],
            100_000_000,
            10000,
        )
        .unwrap();
        assert!(remapped.edge.is_some(), "{}", remapped.reason);
        assert_eq!(remapped.edge.unwrap().world(), &alternate);

        let mut displaced = plane;
        for p in &mut displaced {
            p[0] += 1e-12;
        }
        assert!(qualify_with_planes(
            &world,
            [&fa, &fb],
            [false, true],
            [None, Some(displaced)],
            100_000_000,
            10000
        )
        .unwrap()
        .edge
        .is_none());
        assert!(qualify_with_planes(
            &world,
            [&fa, &fb],
            [false, true],
            [None, Some(plane)],
            100_000_000,
            0
        )
        .unwrap()
        .edge
        .is_none());
        assert!(qualify_with_planes(
            &world,
            [&fa, &fb],
            [false, true],
            [None, Some(plane)],
            1,
            10000
        )
        .unwrap()
        .edge
        .is_none());
        let mut wrong = world.clone();
        wrong.control_points[1][2] = 1e-12;
        assert!(qualify_with_planes(
            &wrong,
            [&fa, &fb],
            [false, true],
            [None, Some(plane)],
            100_000_000,
            10000
        )
        .unwrap()
        .edge
        .is_none());
    }
    #[test]
    fn root_valued_rational_restriction_uses_original_composition_without_extrapolated_chart_admission(
    ) {
        let surface = |vertical: bool| Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![
                    if vertical {
                        vec![0., 0., -0.5]
                    } else {
                        vec![0., -0.5, 0.]
                    },
                    if vertical {
                        vec![0., 0., 0.5]
                    } else {
                        vec![0., 0.5, 0.]
                    },
                ],
                vec![
                    if vertical {
                        vec![1., 0., -0.5]
                    } else {
                        vec![1., -0.5, 0.]
                    },
                    if vertical {
                        vec![1., 0., 0.5]
                    } else {
                        vec![1., 0.5, 0.]
                    },
                ],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let surfaces = [surface(false), surface(true)];
        let mut p = Curve::from_polyline(vec![vec![-0.25, 0.5], vec![1.25, 0.5]]).unwrap();
        p.weights[1] = 0.75;
        let mut world =
            Curve::from_polyline(vec![vec![-0.25, 0., 0.], vec![1.25, 0., 0.]]).unwrap();
        world.weights = p.weights.clone();
        let ends = |s: &Surface| -> Vec<Endpoint> {
            [(0.125, [0.2, 0.4]), (0.875, [0.7, 0.9])]
                .iter()
                .map(|&(u, selector)| {
                    let boundary = Curve::from_polyline(vec![vec![u, 0.], vec![u, 1.]]).unwrap();
                    let r = crate::source_contact_point::qualify(
                        s,
                        &boundary,
                        &p,
                        [[0.4, 0.6], selector],
                        16,
                    )
                    .unwrap();
                    assert!(r.point.is_some(), "{}", r.reason);
                    Endpoint::Crossing {
                        point: r.point.unwrap(),
                        role: Role::Contact,
                    }
                })
                .collect()
        };
        let ea = ends(&surfaces[0]);
        let eb = ends(&surfaces[1]);
        let a = Fragment::new(&surfaces[0], &p, ea[0].clone(), ea[1].clone()).unwrap();
        let b = Fragment::new(&surfaces[1], &p, eb[1].clone(), eb[0].clone()).unwrap();
        assert!(
            curve_surface_agreement::verify_exact(&world, &p, &surfaces[0], false, 1_000_000)
                .unwrap()
                .is_none()
        );
        let r = qualify(&world, [&a, &b], [false, false], 1_000_000).unwrap();
        assert!(r.edge.is_some(), "{}", r.reason);
        assert_eq!(r.edge.unwrap().uses()[0].definition(), a.definition());
        assert!(Fragment::new(
            &surfaces[0],
            &p,
            Endpoint::Parameter(0.),
            Endpoint::Parameter(1.)
        )
        .is_err());
        world.control_points[0][2] += 1e-12;
        assert!(qualify(&world, [&a, &b], [false, false], 1_000_000)
            .unwrap()
            .edge
            .is_none());
    }
    #[test]
    fn adjacent_non_coplanar_faces_have_exact_common_root_without_shared_cutter() {
        let surface = |vertical: bool| Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![
                    vec![0., 0., 0.],
                    if vertical {
                        vec![0., 0., 1.]
                    } else {
                        vec![0., 1., 0.]
                    },
                ],
                vec![
                    vec![1., 0., 0.],
                    if vertical {
                        vec![1., 0., 1.]
                    } else {
                        vec![1., 1., 0.]
                    },
                ],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let p = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
        let cutter = |x: f64, extent: f64| {
            Curve::from_polyline(vec![vec![x, -extent], vec![x, extent]]).unwrap()
        };
        let q = [cutter(0.5, 0.2), cutter(0.5, 0.3)];
        let surfaces = [surface(false), surface(true)];
        let points: Vec<_> = (0..2)
            .map(|i| {
                crate::source_contact_point::qualify(&surfaces[i], &p, &q[i], [[0., 1.]; 2], 16)
                    .unwrap()
                    .point
                    .unwrap()
            })
            .collect();
        let a = Fragment::new(
            &surfaces[0],
            &p,
            Endpoint::Parameter(0.),
            Endpoint::Crossing {
                point: points[0].clone(),
                role: Role::Boundary,
            },
        )
        .unwrap();
        let b = Fragment::new(
            &surfaces[1],
            &p,
            Endpoint::Crossing {
                point: points[1].clone(),
                role: Role::Boundary,
            },
            Endpoint::Parameter(0.),
        )
        .unwrap();
        let world = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.]]).unwrap();
        let r = qualify(&world, [&a, &b], [false, false], 1_000_000).unwrap();
        assert!(r.edge.is_some(), "{}", r.reason);
        let other = crate::source_contact_point::qualify(
            &surfaces[1],
            &p,
            &cutter(0.5 + 1e-12, 0.3),
            [[0., 1.]; 2],
            16,
        )
        .unwrap()
        .point
        .unwrap();
        let b = Fragment::new(
            &surfaces[1],
            &p,
            Endpoint::Crossing {
                point: other,
                role: Role::Boundary,
            },
            Endpoint::Parameter(0.),
        )
        .unwrap();
        assert!(qualify(&world, [&a, &b], [false, false], 1_000_000)
            .unwrap()
            .edge
            .is_none());
    }
    #[test]
    fn different_uv_equations_require_exact_world_cutter_and_unique_world_root() {
        let s = |swap: bool| Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..2)
                .map(|u| {
                    (0..2)
                        .map(|v| {
                            if swap {
                                vec![v as f64, u as f64, 0.]
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
        let line = |p| Curve::from_polyline(p).unwrap();
        let p = [
            line(vec![vec![0., 0.5], vec![1., 0.5]]),
            line(vec![vec![0.5, 0.], vec![0.5, 1.]]),
        ];
        let q = [
            line(vec![vec![0.5, 0.2], vec![0.5, 0.8]]),
            line(vec![vec![0.2, 0.5], vec![0.8, 0.5]]),
        ];
        let surface = [s(false), s(true)];
        let roots: Vec<_> = (0..2)
            .map(|i| {
                crate::source_contact_point::qualify(&surface[i], &p[i], &q[i], [[0., 1.]; 2], 16)
                    .unwrap()
                    .point
                    .unwrap()
            })
            .collect();
        let a = Fragment::new(
            &surface[0],
            &p[0],
            Endpoint::Parameter(0.),
            Endpoint::Crossing {
                point: roots[0].clone(),
                role: Role::Boundary,
            },
        )
        .unwrap();
        let b = Fragment::new(
            &surface[1],
            &p[1],
            Endpoint::Crossing {
                point: roots[1].clone(),
                role: Role::Boundary,
            },
            Endpoint::Parameter(0.),
        )
        .unwrap();
        let world = line(vec![vec![0., 0.5, 0.], vec![1., 0.5, 0.]]);
        let cutter = line(vec![vec![0.5, 0.2, 0.], vec![0.5, 0.8, 0.]]);
        assert!(qualify_with_cutters(
            &p[0],
            [&a, &b],
            [false, false],
            [None, Some(&cutter)],
            1_000_000
        )
        .is_err());
        assert!(qualify(&world, [&a, &b], [false, false], 1_000_000)
            .unwrap()
            .edge
            .is_some());
        let r = qualify_with_cutters(
            &world,
            [&a, &b],
            [false, false],
            [None, Some(&cutter)],
            1_000_000,
        )
        .unwrap();
        assert!(r.edge.is_some(), "{}", r.reason);
        assert_eq!(r.root_checks, 1);
        let mut mapped_world = world.clone();
        mapped_world.knots = vec![-5., -5., 3., 3.];
        let mut mapped_cutter = cutter.clone();
        mapped_cutter.knots = vec![10., 10., 18., 18.];
        let mapped = qualify_with_cutters(
            &mapped_world,
            [&a, &b],
            [false, false],
            [None, Some(&mapped_cutter)],
            1_000_000,
        )
        .unwrap();
        assert!(mapped.edge.is_some(), "{}", mapped.reason);
        assert_eq!(mapped.root_checks, 1);
        assert_eq!(mapped.edge.unwrap().world(), &mapped_world);
        let mut wrong = cutter.clone();
        wrong.control_points[0][2] = 1e-12;
        assert!(qualify_with_cutters(
            &world,
            [&a, &b],
            [false, false],
            [None, Some(&wrong)],
            1_000_000
        )
        .unwrap()
        .edge
        .is_none());
    }
    #[test]
    fn different_surface_charts_share_only_exact_opposite_world_uses() {
        let surface = |z: f64| Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., z]],
                vec![vec![1., 0., 0.], vec![1., 1., z]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let p = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
        let a = Fragment::new(
            &surface(0.),
            &p,
            Endpoint::Parameter(0.),
            Endpoint::Parameter(1.),
        )
        .unwrap();
        let b = Fragment::new(
            &surface(1.),
            &p,
            Endpoint::Parameter(1.),
            Endpoint::Parameter(0.),
        )
        .unwrap();
        let c = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.]]).unwrap();
        let r = qualify(&c, [&a, &b], [false, false], 1_000_000).unwrap();
        assert!(r.edge.is_some(), "{}", r.reason);
        assert_eq!(r.edge.unwrap().reversed(), [false, true]);
        assert!(qualify(&c, [&a, &a], [false, false], 1_000_000)
            .unwrap()
            .edge
            .is_none());
        let mut shifted = c.clone();
        shifted.control_points[0][2] = 1e-12;
        assert!(qualify(&shifted, [&a, &b], [false, false], 1_000_000)
            .unwrap()
            .edge
            .is_none());
        let cutter = Curve::from_polyline(vec![vec![0.5, -0.2], vec![0.5, 0.2]]).unwrap();
        let point = |surface: &Surface| {
            crate::source_contact_point::qualify(surface, &p, &cutter, [[0., 1.]; 2], 16)
                .unwrap()
                .point
                .unwrap()
        };
        let root_a = Endpoint::Crossing {
            point: point(a.surface()),
            role: crate::source_boundary_fragment::Role::Boundary,
        };
        let root_b = Endpoint::Crossing {
            point: point(b.surface()),
            role: crate::source_boundary_fragment::Role::Boundary,
        };
        let cut_a = Fragment::new(a.surface(), &p, Endpoint::Parameter(0.), root_a).unwrap();
        let cut_b = Fragment::new(b.surface(), &p, root_b, Endpoint::Parameter(0.)).unwrap();
        let shared = qualify(&c, [&cut_a, &cut_b], [false, false], 1_000_000).unwrap();
        assert!(shared.edge.is_some(), "{}", shared.reason);
        assert_eq!(
            shared.edge.unwrap().uses()[0].definition(),
            cut_a.definition()
        );
        let refined =
            crate::source_contact_point::qualify(b.surface(), &p, &cutter, [[0.2, 0.8]; 2], 16)
                .unwrap()
                .point
                .unwrap();
        let refined = Fragment::new(
            b.surface(),
            &p,
            Endpoint::Crossing {
                point: refined,
                role: crate::source_boundary_fragment::Role::Boundary,
            },
            Endpoint::Parameter(0.),
        )
        .unwrap();
        let independently_selected =
            qualify(&c, [&cut_a, &refined], [false, false], 1_000_000).unwrap();
        assert!(
            independently_selected.edge.is_some(),
            "{}",
            independently_selected.reason
        );
        assert_eq!(independently_selected.root_checks, 1);
        let mut other = cutter.clone();
        other.control_points[0][0] += 1e-12;
        other.control_points[1][0] += 1e-12;
        let other =
            crate::source_contact_point::qualify(b.surface(), &p, &other, [[0., 1.]; 2], 16)
                .unwrap()
                .point
                .unwrap();
        let mismatch = Fragment::new(
            b.surface(),
            &p,
            Endpoint::Crossing {
                point: other,
                role: crate::source_boundary_fragment::Role::Boundary,
            },
            Endpoint::Parameter(0.),
        )
        .unwrap();
        assert!(qualify(&c, [&cut_a, &mismatch], [false, false], 1_000_000)
            .unwrap()
            .edge
            .is_none());
        let partial = Fragment::new(
            a.surface(),
            &p,
            Endpoint::Parameter(0.2),
            Endpoint::Parameter(1.),
        )
        .unwrap();
        assert!(qualify(&c, [&partial, &b], [false, false], 1_000_000)
            .unwrap()
            .edge
            .is_none());
    }
    #[test]
    fn curved_world_carrier_keeps_irrational_root_across_reflected_uv_charts() {
        let surface = |sign: f64| Surface {
            degree_u: 1, degree_v: 1, periodic_u: false, periodic_v: false,
            knots_u: vec![0.,0.,1.,1.], knots_v: vec![-1.,-1.,1.,1.],
            control_points: vec![
                vec![vec![0.,-sign,0.],vec![0.,sign,0.]],
                vec![vec![1.,-sign,0.],vec![1.,sign,0.]],
            ], weights: vec![vec![1.;2];2],
        };
        let main = Curve {degree:2,periodic:false,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![0.,0.],vec![0.5,0.25],vec![1.,0.]],weights:vec![1.,2.,1.]};
        let cutter=Curve {degree:2,periodic:false,knots:main.knots.clone(),
            control_points:vec![vec![0.,-0.5],vec![0.5,0.],vec![1.,0.5]],weights:vec![1.,2.,1.]};
        let mut other_main=main.clone();other_main.control_points.reverse();
        for p in &mut other_main.control_points {p[1] = -p[1];}
        let mut other_cutter=cutter.clone();
        for p in &mut other_cutter.control_points {p[1] = -p[1];}
        let a_surface=surface(1.);let b_surface=surface(-1.);
        let point = |surface:&Surface, m:&Curve, c:&Curve, selector| {
            let r=crate::source_contact_point::qualify(surface,m,c,selector,1000).unwrap();
            r.point.expect(r.reason)
        };
        let a=Fragment::new(&a_surface,&main,Endpoint::Parameter(0.),
            Endpoint::Crossing {point:point(&a_surface,&main,&cutter,[[0.7,0.71],[0.7,0.71]]),role:Role::Boundary}).unwrap();
        let b=Fragment::new(&b_surface,&other_main,
            Endpoint::Crossing {point:point(&b_surface,&other_main,&other_cutter,[[0.29,0.3],[0.7,0.71]]),role:Role::Boundary},Endpoint::Parameter(1.)).unwrap();
        let mut world=main.clone();for p in &mut world.control_points {p.push(0.);}
        let r=qualify(&world,[&a,&b],[false,true],1_000_000).unwrap();
        assert!(r.edge.is_some(),"{}",r.reason);
        assert!(r.root_checks>0);
        let edge=r.edge.unwrap();
        assert_eq!(edge.uses()[0].definition(),a.definition());
        assert_eq!(edge.uses()[1].definition(),b.definition());
        let range=edge.uses()[0].parameter_bounds()[1];
        assert!(range[0]<std::f64::consts::FRAC_1_SQRT_2 && range[1]>std::f64::consts::FRAC_1_SQRT_2);
        let mut damaged=world.clone();damaged.control_points[1][1]+=1e-12;
        assert!(qualify(&damaged,[&a,&b],[false,true],1_000_000).unwrap().edge.is_none());
    }

    #[test]
    fn curved_support_charts_share_original_root_ended_world_edge() {
        // Distinct curved NURBS sheets z=u^2+v, y=+/-v share v=0.
        let surface=|sign:f64|Surface{degree_u:2,degree_v:1,periodic_u:false,periodic_v:false,
            knots_u:vec![0.,0.,0.,1.,1.,1.],knots_v:vec![-1.,-1.,1.,1.],
            control_points:vec![vec![vec![0.,-sign,-1.],vec![0.,sign,1.]],
                vec![vec![0.5,-sign,-1.],vec![0.5,sign,1.]],vec![vec![1.,-sign,0.],vec![1.,sign,2.]]],
            weights:vec![vec![1.;2];3]};
        let main=Curve{degree:2,periodic:false,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![0.,0.],vec![0.5,0.],vec![1.,0.]],weights:vec![1.;3]};
        let cutter=Curve{control_points:vec![vec![0.,-0.5],vec![0.5,-0.5],vec![1.,0.5]],..main.clone()};
        let mut other_main=main.clone();other_main.control_points.reverse();
        let mut other_cutter=cutter.clone();for p in &mut other_cutter.control_points{p[1] = -p[1];}
        let sa=surface(1.);let sb=surface(-1.);
        let pa=crate::source_contact_point::qualify(&sa,&main,&cutter,[[0.7,0.71],[0.7,0.71]],1000).unwrap();
        let pb=crate::source_contact_point::qualify(&sb,&other_main,&other_cutter,[[0.29,0.3],[0.7,0.71]],1000).unwrap();
        let a=Fragment::new(&sa,&main,Endpoint::Parameter(0.),Endpoint::Crossing{point:pa.point.expect(pa.reason),role:Role::Boundary}).unwrap();
        let b=Fragment::new(&sb,&other_main,Endpoint::Crossing{point:pb.point.expect(pb.reason),role:Role::Boundary},Endpoint::Parameter(1.)).unwrap();
        let world=Curve{control_points:vec![vec![0.,0.,0.],vec![0.5,0.,0.],vec![1.,0.,1.]],..main.clone()};
        let r=qualify(&world,[&a,&b],[false,true],1_000_000).unwrap();
        let edge=r.edge.expect(r.reason);assert!(r.root_checks>0);
        let range=edge.uses()[0].parameter_bounds()[1];
        assert!(range[0]<std::f64::consts::FRAC_1_SQRT_2&&range[1]>std::f64::consts::FRAC_1_SQRT_2);
        assert_eq!(edge.uses()[0].definition(),a.definition());assert_eq!(edge.uses()[1].definition(),b.definition());
        let mut wrong=world.clone();wrong.control_points[1][2]=1e-12;
        assert!(qualify(&wrong,[&a,&b],[false,true],1_000_000).unwrap().edge.is_none());
    }

}
