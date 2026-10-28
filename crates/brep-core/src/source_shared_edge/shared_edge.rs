use super::*;

#[derive(Clone)]
pub struct SharedEdge {
    world: Curve,
    uses: [Fragment; 2],
    reversed: [bool; 2],
    ranges: Option<[[[f64; 2]; 2]; 2]>,
    recipe: Recipe,
}
impl SharedEdge {
    /// Same original definitions and proposal inputs, never proximity or cached
    /// direction/range/interval equality. Mirrors the existing wire identity.
    pub fn same_definition(&self, other: &Self) -> bool {
        self.world == other.world
            && self.recipe == other.recipe
            && self
                .uses
                .iter()
                .zip(&other.uses)
                .all(|(a, b)| a.same_definition(b))
    }
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
        // Composition identity covers original curves, not their retained
        // fragments. Actual owned ends must also cover each complete domain.
        let complete_uses =
            self.uses.iter().all(|fragment| {
                let domain = fragment.curve().domain();
                let expected = if fragment.reversed() {
                    [domain[1], domain[0]]
                } else {
                    domain
                };
                fragment.endpoints().iter().zip(expected).all(
                    |(endpoint, t)| matches!(endpoint, Endpoint::Parameter(value) if *value == t),
                )
            });
        complete_uses && self.maps_complete_canonical_source()
    }
    /// Full original composition, independently of the actual retained ends.
    /// This is not a retained edge or ordered-chain coverage certificate.
    pub(crate) fn maps_complete_canonical_source(&self) -> bool {
        match self.ranges {
            Some(ranges) => ranges.iter().all(|r| {
                (r[0][0] == 0. && r[1][0] == r[1][1]) || (r[1][0] == 0. && r[0][0] == r[0][1])
            }),
            None => true,
        }
    }
    /// Sufficient retained-domain coverage, checked before an inverse could
    /// clip away unsupported canonical tails. Interval overlap is not authority.
    pub(crate) fn covers_retained_canonical_interval(&self, canonical: [f64; 2]) -> Result<bool> {
        use nurbs_core::interval_eval::Interval as I;
        let q = I::new(canonical[0], canonical[1])?;
        for side in 0..2 {
            let t = crate::source_seam_tangency::inverse_unclipped(self, side, q)?;
            let ends = self.uses()[side].parameter_bounds();
            if t.lo < ends[0][1].min(ends[1][1]) || t.hi > ends[0][0].max(ends[1][0]) {
                return Ok(false);
            }
        }
        Ok(true)
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
            recipe: Recipe::Mapped {
                ranges: [uses[0].range(), uses[1].range()],
                planes,
                candidates,
            },
        }
    }
    /// Original definitions and proposal inputs only; restoration recomputes authority.
    pub fn definition(&self) -> value_codec::Value {
        use value_codec::Serialize;
        value_codec::json!({"version":1,"world":self.world.to_value(),
            "uses":self.uses.each_ref().map(|f|f.definition()),"recipe":self.recipe.definition()})
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
    for cutter in cutters.iter().flatten() {
        cutter.validate()?;
    }
    if planes
        .iter()
        .flatten()
        .flatten()
        .flatten()
        .any(|v| !v.is_finite())
    {
        return Err(Error::new(
            "BREP_SOURCE_SHARED_EDGE",
            "Original plane proposals must be finite",
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
                (Endpoint::Carrier { .. }, _) | (_, Endpoint::Carrier { .. }) => false,
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
        recipe: Recipe::Direct {
            world_reversed,
            cutters: cutters.map(|c| c.cloned()),
            planes,
        },
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
pub(crate) fn common_plane_root_in_domain(
    world: &Curve,
    plane: [[f64; 3]; 3],
    points: [(&crate::source_contact_point::SourcePoint, Role); 2],
    domain: [f64; 2],
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
    unique_world_plane_domain(world, plane, Some(domain), out, max_work, max_driver_cells)
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
    unique_world_plane_domain(world, plane, None, out, max_work, max_driver_cells)
}
pub(crate) fn unique_world_plane_domain(
    world: &Curve,
    plane: [[f64; 3]; 3],
    domain: Option<[f64; 2]>,
    out: &mut Report,
    max_work: u64,
    max_driver_cells: usize,
) -> Result<bool> {
    if let Some(domain) = domain {
        if out.driver_cells == max_driver_cells {
            return Ok(false);
        }
        let report = nurbs_core::curve_axis_driver::certify_plane_domain(
            world,
            plane,
            domain,
            max_driver_cells - out.driver_cells,
        )?;
        out.driver_cells += report.visited;
        return Ok(report.monotonic_proven);
    }
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
    // A general signed-plane derivative can remain strict while several
    // original coordinates vary. Keep the exact coordinate path above for
    // endpoint-zero derivatives and use only its remaining driver budget.
    if out.driver_cells < max_driver_cells && !world.periodic {
        let report = nurbs_core::curve_axis_driver::certify_plane_domain(
            world,
            plane,
            world.domain(),
            max_driver_cells - out.driver_cells,
        )?;
        out.driver_cells += report.visited;
        return Ok(report.monotonic_proven);
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
