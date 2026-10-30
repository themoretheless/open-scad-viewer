//! Immutable original source use on an exact projective portion of a canonical curve.
//! Endpoint consumers freshly prove cross-face identity; no Model is admitted.
use crate::{source_allowed_contact, source_boundary_fragment::Fragment};
use cad_predicates::{BezierIdentity, Sign};
use nurbs_core::{Error, Result, curve::Curve, interval_eval::Interval as I};
#[cfg(feature = "codec")]
use value_codec::{Deserialize, Serialize, Value, json};
#[derive(Clone)]
pub struct ProjectiveUse {
    world: Curve,
    fragment: Fragment,
    range: [[f64; 2]; 2],
    reversed: bool,
    weights: [f64; 2],
}
impl ProjectiveUse {
    /// Save original definitions only. Authority is reconstructed on restore.
    #[cfg(feature = "codec")]
    pub fn definition(&self) -> Value {
        json!({"version":1,"world":self.world.to_value(),"fragment":self.fragment.definition(),
            "range":self.range,"weights":self.weights})
    }
    pub fn world(&self) -> &Curve {
        &self.world
    }
    pub fn fragment(&self) -> &Fragment {
        &self.fragment
    }
    /// Exact normalized rational endpoints for the forward original pcurve.
    pub fn range(&self) -> [[f64; 2]; 2] {
        self.range
    }
    pub fn weights(&self) -> [f64; 2] {
        self.weights
    }
    pub fn reversed(&self) -> bool {
        self.reversed
    }
    /// Map an original source interval to normalized canonical parameters.
    pub fn map_normalized(&self, parameter: I) -> Result<I> {
        let d = self.fragment.curve().domain();
        let t = parameter
            .sub(I::point(d[0]))?
            .div(I::point(d[1]).sub(I::point(d[0]))?)?
            .intersect(0., 1.)?;
        let a = I::point(self.range[0][0]).div(I::point(self.range[0][1]))?;
        let b = I::point(self.range[1][0]).div(I::point(self.range[1][1]))?;
        let low = I::point(1.).sub(t)?.mul(I::point(self.weights[0]))?;
        let high = t.mul(I::point(self.weights[1]))?;
        let den = low.add(high)?.intersect(
            self.weights[0].min(self.weights[1]),
            self.weights[0].max(self.weights[1]),
        )?;
        a.mul(low)?.add(b.mul(high)?)?.div(den)?.intersect(0., 1.)
    }
    /// Unclipped inverse: unsupported canonical tails remain visible to the
    /// caller. Clipping here would hide missing retained-domain coverage.
    pub fn inverse_normalized_unclipped(&self, canonical: I) -> Result<I> {
        let a = I::point(self.range[0][0]).div(I::point(self.range[0][1]))?;
        let b = I::point(self.range[1][0]).div(I::point(self.range[1][1]))?;
        let q = canonical.sub(a)?.div_signed(b.sub(a)?)?;
        let numerator = q.mul(I::point(self.weights[0]))?;
        let mut denominator = I::point(1.)
            .sub(q)?
            .mul(I::point(self.weights[1]))?
            .add(numerator)?;
        if q.lo >= 0. && q.hi <= 1. {
            denominator = denominator.intersect(
                self.weights[0].min(self.weights[1]),
                self.weights[0].max(self.weights[1]),
            )?;
        }
        let t = numerator.div_signed(denominator)?;
        let d = self.fragment.curve().domain();
        I::point(d[0]).add(t.mul(I::point(d[1]).sub(I::point(d[0]))?)?)
    }
    /// Outward enclosures only: root expressions and original domains remain
    /// authoritative. These bounds must never replace a root by a fixed vertex.
    pub fn parameter_bounds(&self) -> Result<[[f64; 2]; 2]> {
        let target = self.world.domain();
        let map = |r: [f64; 2]| -> Result<[f64; 2]> {
            let q = self.map_normalized(I::new(r[0], r[1])?)?;
            let t = I::point(target[0])
                .add(q.mul(I::point(target[1]).sub(I::point(target[0]))?)?)?
                .intersect(target[0], target[1])?;
            Ok([t.lo, t.hi])
        };
        let b = self.fragment.parameter_bounds();
        Ok([map(b[0])?, map(b[1])?])
    }
}
pub struct Report {
    pub mapped: Option<ProjectiveUse>,
    pub exact_work: u64,
    pub reason: &'static str,
}
/// Fragment authority proves actual chart membership. Fresh exact composition
/// then binds its unchanged source curve to the supplied canonical subinterval.
pub fn qualify(
    world: &Curve,
    fragment: &Fragment,
    range: [[f64; 2]; 2],
    weights: [f64; 2],
    max_work: u64,
) -> Result<Report> {
    if !(1..=100_000_000).contains(&max_work) {
        return Err(Error::new(
            "BREP_SOURCE_PROJECTIVE_USE",
            "Choose bounded exact mapping work",
        ));
    }
    let mut out = Report {
        mapped: None,
        exact_work: 0,
        reason: "source-projective-layout-unproven",
    };
    let Some(r) = nurbs_core::curve_surface_affine::verify_projective_algebraic(
        world,
        fragment.curve(),
        fragment.surface(),
        range,
        weights,
        max_work.min(cad_predicates::MAX_WORK),
    )?
    else {
        return Ok(out);
    };
    out.exact_work += r.work_used;
    out.reason = "source-projective-composition-unproven";
    if r.outcome != BezierIdentity::Equal {
        return Ok(out);
    }
    out.reason = "source-projective-orientation-unproven";
    let points = [
        [0., 0., 0.],
        [range[0][0], range[0][1], 0.],
        [range[1][0], range[1][1], 0.],
    ];
    let Some(sign) =
        source_allowed_contact::orient(&points, Some([0, 1]), &mut out.exact_work, max_work)?
    else {
        return Ok(out);
    };
    if sign == Sign::Zero {
        return Ok(out);
    }
    out.mapped = Some(ProjectiveUse {
        world: world.clone(),
        fragment: fragment.clone(),
        range,
        weights,
        reversed: fragment.reversed() ^ (sign == Sign::Positive),
    });
    out.reason = "source-projective-use-qualified";
    Ok(out)
}

/// Restore roots from original equations/selectors, then freshly prove the
/// projective composition and orientation. Cached bounds have no authority.
#[cfg(feature = "codec")]
pub fn restore(value: Value, max_mapping_cells: usize, max_work: u64) -> Result<Report> {
    let invalid = || {
        Error::new(
            "BREP_SOURCE_PROJECTIVE_USE",
            "Invalid projective source definition or restoration budget",
        )
    };
    if value["version"].as_u64() != Some(1)
        || !(1..=100000).contains(&max_mapping_cells)
        || !(1..=100000000).contains(&max_work)
    {
        return Err(invalid());
    }
    let world = Curve::from_value(value["world"].clone()).map_err(|_| invalid())?;
    let range = <[[f64; 2]; 2]>::from_value(value["range"].clone()).map_err(|_| invalid())?;
    let weights = <[f64; 2]>::from_value(value["weights"].clone()).map_err(|_| invalid())?;
    let fragment =
        crate::source_boundary_fragment::restore(value["fragment"].clone(), max_mapping_cells)?;
    qualify(&world, &fragment, range, weights, max_work)
}

/// Prove that two retained root expressions identify the same canonical point.
/// Both compositions were proved when the ProjectiveUse values were created;
/// the supplied plane must freshly bind both roots and have one world root on
/// the entire union of their outward parameter bounds.
pub fn same_crossing_endpoint(
    uses: [&ProjectiveUse; 2],
    endpoints: [usize; 2],
    plane: [[f64; 3]; 3],
    max_work: u64,
    max_driver: usize,
) -> Result<(bool, crate::source_shared_edge::Report)> {
    if endpoints.iter().any(|&i| i > 1)
        || !(1..=100_000_000).contains(&max_work)
        || max_driver > 100000
    {
        return Err(Error::new(
            "BREP_SOURCE_PROJECTIVE_USE",
            "Invalid root correspondence bounds",
        ));
    }
    let mut report = crate::source_shared_edge::Report {
        edge: None,
        work_used: 0,
        root_checks: 0,
        driver_cells: 0,
        reason: "projective-root-correspondence-unproven",
    };
    if uses[0].world() != uses[1].world() {
        return Ok((false, report));
    }
    let ends = [
        &uses[0].fragment().endpoints()[endpoints[0]],
        &uses[1].fragment().endpoints()[endpoints[1]],
    ];
    let [
        crate::source_boundary_fragment::Endpoint::Crossing { point: a, role: ar },
        crate::source_boundary_fragment::Endpoint::Crossing { point: b, role: br },
    ] = ends
    else {
        return Ok((false, report));
    };
    let aa = uses[0].parameter_bounds()?[endpoints[0]];
    let bb = uses[1].parameter_bounds()?[endpoints[1]];
    let domain = [aa[0].min(bb[0]), aa[1].max(bb[1])];
    let same = crate::source_shared_edge::common_plane_root_in_domain(
        uses[0].world(),
        plane,
        [(a, *ar), (b, *br)],
        domain,
        &mut report,
        max_work,
        max_driver,
    )?;
    if same {
        report.reason = "projective-root-correspondence-qualified";
    }
    Ok((same, report))
}

/// Compare fixed ends or root ends with freshly verified exact parameter
/// candidates. A candidate is never accepted from its rounded root enclosure.
pub fn same_parameter_endpoint(
    uses: [&ProjectiveUse; 2],
    endpoints: [usize; 2],
    candidates: [Option<[f64; 2]>; 2],
    max_work: u64,
) -> Result<(bool, u64)> {
    let (same, work, _) = same_parameter_endpoint_counted(uses, endpoints, candidates, max_work)?;
    Ok((same, work))
}
pub(crate) fn same_parameter_endpoint_counted(
    uses: [&ProjectiveUse; 2],
    endpoints: [usize; 2],
    candidates: [Option<[f64; 2]>; 2],
    max_work: u64,
) -> Result<(bool, u64, usize)> {
    use crate::source_boundary_fragment::{Endpoint, Role};
    use cad_predicates::{
        AuthoredScalar, Limits, ParameterIdentity, PredicateContext, SourceArena, ToleranceContext,
    };
    if endpoints.iter().any(|&i| i > 1) || !(1..=100_000_000).contains(&max_work) {
        return Err(Error::new(
            "BREP_SOURCE_PROJECTIVE_USE",
            "Invalid exact endpoint bounds",
        ));
    }
    if uses[0].world() != uses[1].world() {
        return Ok((false, 0, 0));
    }
    let mut work = 0;
    let mut checks = 0;
    let mut parameters = Vec::new();
    for i in 0..2 {
        let t = match &uses[i].fragment().endpoints()[endpoints[i]] {
            Endpoint::Parameter(t) => *t,
            Endpoint::Crossing { point, role } => {
                let Some(candidate) = candidates[i] else {
                    return Ok((false, work, checks));
                };
                if work == max_work {
                    return Ok((false, work, checks));
                }
                checks += 1;
                let r = crate::source_root_parameter::verify(
                    point,
                    candidate,
                    (max_work - work).min(cad_predicates::MAX_WORK),
                )?;
                work += r.work_used;
                let Some(p) = r.parameters else {
                    return Ok((false, work, checks));
                };
                p[match role {
                    Role::Boundary => 0,
                    Role::Contact => 1,
                }]
            }
            Endpoint::Carrier { .. } => return Ok((false, work, checks)),
        };
        let d = uses[i].fragment().curve().domain();
        parameters.extend([t, d[0], d[1]]);
    }
    if work == max_work {
        return Ok((false, work, checks));
    }
    let values: Vec<_> = parameters
        .iter()
        .copied()
        .chain(uses.iter().flat_map(|u| u.range().into_iter().flatten()))
        .chain(uses.iter().flat_map(|u| u.weights()))
        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
        .collect();
    let source = SourceArena::authored("projective-retained-endpoints", 1, values)
        .map_err(|_| Error::new("BREP_SOURCE_PROJECTIVE_USE", "Invalid endpoint source"))?;
    let p = std::array::from_fn(|i| std::array::from_fn(|j| source.leaf(3 * i + j).unwrap()));
    let r = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            std::array::from_fn(|k| source.leaf(6 + 4 * i + 2 * j + k).unwrap())
        })
    });
    let w = std::array::from_fn(|i| std::array::from_fn(|j| source.leaf(14 + 2 * i + j).unwrap()));
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &source,
        &tolerance,
        Limits {
            max_work: (max_work - work).min(cad_predicates::MAX_WORK),
            ..Limits::default()
        },
        None,
    );
    let r = cad_predicates::projective_parameter_identity(&mut ctx, p, r, w)
        .map_err(|_| Error::new("BREP_SOURCE_PROJECTIVE_USE", "Invalid endpoint comparison"))?;
    work += r.work_used;
    Ok((r.outcome == ParameterIdentity::Equal, work, checks))
}

/// Freshly bind exact rational candidates to original root expressions,
/// then compare their mapped canonical parameters without rounded division.
pub(crate) fn same_fraction_endpoint_counted(
    uses: [&ProjectiveUse; 2],
    endpoints: [usize; 2],
    candidates: [Option<[[f64; 2]; 2]>; 2],
    max_work: u64,
) -> Result<(bool, u64, usize)> {
    use crate::source_boundary_fragment::{Endpoint, Role};
    use cad_predicates::{
        AuthoredScalar, Limits, ParameterIdentity, PredicateContext, SourceArena, ToleranceContext,
    };
    if endpoints.iter().any(|&i| i > 1) || !(1..=100000000).contains(&max_work) {
        return Err(Error::new(
            "BREP_SOURCE_PROJECTIVE_USE",
            "Invalid rational endpoint bounds",
        ));
    }
    if uses[0].world() != uses[1].world() {
        return Ok((false, 0, 0));
    }
    let mut work = 0;
    let mut checks = 0;
    let mut values = Vec::new();
    for i in 0..2 {
        let parameter = match &uses[i].fragment().endpoints()[endpoints[i]] {
            Endpoint::Parameter(t) => [*t, 1.],
            Endpoint::Crossing { point, role } => {
                let Some(candidate) = candidates[i] else {
                    return Ok((false, work, checks));
                };
                if work == max_work {
                    return Ok((false, work, checks));
                }
                checks += 1;
                let proof = crate::source_root_parameter::verify_fraction(
                    point,
                    candidate,
                    (max_work - work).min(cad_predicates::MAX_WORK),
                )?;
                work += proof.work_used;
                let Some(parameters) = proof.parameters else {
                    return Ok((false, work, checks));
                };
                parameters[match role {
                    Role::Boundary => 0,
                    Role::Contact => 1,
                }]
            }
            Endpoint::Carrier { .. } => return Ok((false, work, checks)),
        };
        let d = uses[i].fragment().curve().domain();
        values.extend([parameter[0], parameter[1], d[0], d[1]]);
    }
    if work == max_work {
        return Ok((false, work, checks));
    }
    values.extend(uses.iter().flat_map(|u| u.range().into_iter().flatten()));
    values.extend(uses.iter().flat_map(|u| u.weights()));
    let arena = SourceArena::authored(
        "projective-fraction-root-identity",
        1,
        values
            .into_iter()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .map_err(|_| Error::new("BREP_SOURCE_PROJECTIVE_USE", "Invalid rational root source"))?;
    let parameters =
        std::array::from_fn(|i| std::array::from_fn(|j| arena.leaf(4 * i + j).unwrap()));
    let ranges = std::array::from_fn(|i| {
        std::array::from_fn(|j| std::array::from_fn(|k| arena.leaf(8 + 4 * i + 2 * j + k).unwrap()))
    });
    let weights =
        std::array::from_fn(|i| std::array::from_fn(|j| arena.leaf(16 + 2 * i + j).unwrap()));
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work: (max_work - work).min(cad_predicates::MAX_WORK),
            ..Limits::default()
        },
        None,
    );
    let proof = cad_predicates::projective_parameter_fraction_identity(
        &mut ctx, parameters, ranges, weights,
    )
    .map_err(|_| {
        Error::new(
            "BREP_SOURCE_PROJECTIVE_USE",
            "Invalid exact rational root comparison",
        )
    })?;
    work += proof.work_used;
    Ok((proof.outcome == ParameterIdentity::Equal, work, checks))
}

/// Exact canonical identity of two original transverse linear UV crossings.
/// Private SourcePoints supply uniqueness; line equations supply unrounded roots.
pub(crate) fn same_linear_endpoint_counted(
    uses: [&ProjectiveUse; 2],
    endpoints: [usize; 2],
    max_work: u64,
) -> Result<(bool, u64, usize)> {
    use crate::source_boundary_fragment::{Endpoint, Role};
    use cad_predicates::{
        AuthoredScalar, Limits, ParameterIdentity, PredicateContext, SourceArena, ToleranceContext,
    };
    if endpoints.iter().any(|&i| i > 1) || !(1..=100000000).contains(&max_work) {
        return Err(Error::new(
            "BREP_SOURCE_PROJECTIVE_USE",
            "Invalid linear root comparison bounds",
        ));
    }
    if uses[0].world() != uses[1].world() {
        return Ok((false, 0, 0));
    }
    let line = |c: &Curve| {
        let d = c.domain();
        c.degree == 1
            && c.control_points.len() == 2
            && c.knots[..2].iter().all(|&t| t == d[0])
            && c.knots[2..].iter().all(|&t| t == d[1])
    };
    let mut values = Vec::new();
    for i in 0..2 {
        let Endpoint::Crossing { point, role } = &uses[i].fragment().endpoints()[endpoints[i]]
        else {
            return Ok((false, 0, 0));
        };
        let (main, cutter) = match role {
            Role::Boundary => (point.boundary(), point.contact()),
            Role::Contact => (point.contact(), point.boundary()),
        };
        if !line(main) || !line(cutter) {
            return Ok((false, 0, 0));
        }
        for (p, w) in main.control_points.iter().zip(&main.weights) {
            values.extend([p[0], p[1], *w]);
        }
        for p in &cutter.control_points {
            values.extend([p[0], p[1]]);
        }
    }
    values.extend(uses.iter().flat_map(|u| u.range().into_iter().flatten()));
    values.extend(uses.iter().flat_map(|u| u.weights()));
    let arena = SourceArena::authored(
        "projective-linear-root-identity",
        1,
        values
            .into_iter()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .map_err(|_| {
        Error::new(
            "BREP_SOURCE_PROJECTIVE_USE",
            "Invalid original linear source",
        )
    })?;
    let main = std::array::from_fn(|i| {
        std::array::from_fn(|j| std::array::from_fn(|k| arena.leaf(10 * i + 3 * j + k).unwrap()))
    });
    let cutter = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            std::array::from_fn(|k| arena.leaf(10 * i + 6 + 2 * j + k).unwrap())
        })
    });
    let ranges = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            std::array::from_fn(|k| arena.leaf(20 + 4 * i + 2 * j + k).unwrap())
        })
    });
    let weights =
        std::array::from_fn(|i| std::array::from_fn(|j| arena.leaf(28 + 2 * i + j).unwrap()));
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work: max_work.min(cad_predicates::MAX_WORK),
            ..Limits::default()
        },
        None,
    );
    let proof = cad_predicates::line_crossing_parameter_identity_projective(
        &mut ctx, main, cutter, ranges, weights,
    )
    .map_err(|_| {
        Error::new(
            "BREP_SOURCE_PROJECTIVE_USE",
            "Invalid exact linear root comparison",
        )
    })?;
    Ok((
        proof.outcome == ParameterIdentity::Equal,
        proof.work_used,
        2,
    ))
}

#[cfg(test)]
#[path = "source_projective_use/tests/mod.rs"]
mod tests;
