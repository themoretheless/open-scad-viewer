//! Four exact projective charts cover a punctured parameter cube about an
//! original SS root. Saturation separates directions without losing branches.
//! Completeness of this coordinate cover is distinct from real-root census.
use super::*;
#[derive(Clone, Debug)]
pub struct QualifiedSsRootNode {
    source: QualifiedSsDeflatedSystem,
    center: [Q; 4],
    world: [Q; 3],
    limits: ExactSsAlgebraLimits,
    work: usize,
}
impl QualifiedSsRootNode {
    pub fn source(&self) -> &QualifiedSsDeflatedSystem {
        &self.source
    }
    pub fn center(&self) -> &[Q; 4] {
        &self.center
    }
    pub fn world(&self) -> &[Q; 3] {
        &self.world
    }
    pub fn work(&self) -> usize {
        self.work
    }
    pub fn reverify(&self) -> Result<()> {
        self.source.reverify()?;
        let fresh = qualify_ss_root_node(&self.source, self.center.clone(), self.limits)?;
        check(
            fresh.world == self.world,
            "Original SS node world image changed",
        )
    }
}
/// Exact point ownership; singularity and valence are proved separately.
/// Rational local clocks allow arbitrary rational original world coordinates.
pub fn qualify_ss_root_node(
    source: &QualifiedSsDeflatedSystem,
    center: [Q; 4],
    limits: ExactSsAlgebraLimits,
) -> Result<QualifiedSsRootNode> {
    validate_limits(limits)?;
    let mut w = Work { used: 0, limits };
    for c in &center {
        w.coefficient(c)?;
    }
    check(
        center.iter().all(|v| Q::zero() <= *v && *v <= Q::one()),
        "SS node leaves its original span",
    )?;
    let original = source.original().original();
    for p in original.equations() {
        check(
            localization::evaluate(p, &center, &mut w)?.is_zero(),
            "Proposed node is not an exact original SS root",
        )?;
    }
    let h = &original.homogeneous()[0];
    let weight = localization::evaluate(&h[3], &center, &mut w)?;
    check(
        weight > Q::zero(),
        "Original SS node denominator is not positive",
    )?;
    let mut world = std::array::from_fn(|_| Q::zero());
    for k in 0..3 {
        w.charge()?;
        world[k] = localization::evaluate(&h[k], &center, &mut w)? / &weight;
        w.coefficient(&world[k])?;
    }
    Ok(QualifiedSsRootNode {
        source: source.clone(),
        center,
        world,
        limits,
        work: w.used,
    })
}
#[derive(Clone, Debug)]
pub struct QualifiedSsBlowupChart {
    node: QualifiedSsRootNode,
    radius: Q,
    radial: usize,
    map: [ExactSsPolynomial; 4],
    equations: Vec<ExactSsPolynomial>,
    limits: ExactSsGroebnerLimits,
    work: usize,
    pairs: usize,
}
impl QualifiedSsBlowupChart {
    pub fn node(&self) -> &QualifiedSsRootNode {
        &self.node
    }
    pub fn radial_axis(&self) -> usize {
        self.radial
    }
    pub fn radius(&self) -> &Q {
        &self.radius
    }
    pub fn original_coordinate_map(&self) -> &[ExactSsPolynomial; 4] {
        &self.map
    }
    pub fn equations(&self) -> &[ExactSsPolynomial] {
        &self.equations
    }
    pub fn work(&self) -> usize {
        self.work
    }
    pub fn pairs(&self) -> usize {
        self.pairs
    }
    pub fn reverify(&self) -> Result<()> {
        self.node.reverify()?;
        let fresh =
            qualify_ss_blowup_chart(&self.node, self.radius.clone(), self.radial, self.limits)?;
        check(
            self.map == fresh.map
                && self.equations == fresh.equations
                && self.work == fresh.work
                && self.pairs == fresh.pairs,
            "Original SS strict transform changed",
        )
    }
    /// For any original point in the centered cube other than the node, choose
    /// an axis attaining max |x-c|. This exact inverse belongs to [0,1]^4.
    /// The four charts therefore omit no punctured original root neighborhood.
    pub fn lift_dominant(&self, point: &[Q; 4]) -> Result<[Q; 4]> {
        check(
            point.iter().all(|v| Q::zero() <= *v && *v <= Q::one()),
            "Blowup lift leaves original patch",
        )?;
        let delta: [Q; 4] = std::array::from_fn(|k| &point[k] - &self.node.center[k]);
        let r = &delta[self.radial];
        check(
            !r.is_zero()
                && delta
                    .iter()
                    .all(|v| num_traits::Signed::abs(v) <= num_traits::Signed::abs(r))
                && num_traits::Signed::abs(r) <= self.radius,
            "Blowup lift needs a dominant nonzero radius",
        )?;
        let two = Q::from_integer(2.into());
        let four = Q::from_integer(4.into());
        Ok(std::array::from_fn(|k| {
            if k == self.radial {
                (r / &self.radius + Q::one()) / &two
            } else {
                (&delta[k] / r + &two) / &four
            }
        }))
    }
}
pub fn qualify_ss_blowup_chart(
    node: &QualifiedSsRootNode,
    radius: Q,
    radial: usize,
    limits: ExactSsGroebnerLimits,
) -> Result<QualifiedSsBlowupChart> {
    validate_limits(limits.algebra)?;
    let mut order = limits.variable_order;
    order.sort();
    check(
        radial < 4
            && Q::zero() < radius
            && radius <= Q::one()
            && order == [0, 1, 2, 3]
            && (1..=65536).contains(&limits.max_pairs)
            && (1..=256).contains(&limits.max_basis),
        "Invalid original SS blowup chart allowance",
    )?;
    let mut w = Work {
        used: 0,
        limits: limits.algebra,
    };
    w.coefficient(&radius)?;
    for c in &node.center {
        w.coefficient(c)?;
    }
    let mut r = division::constant(-radius.clone());
    let mut e = [0; 4];
    e[radial] = 1;
    r.insert(e, &radius * Q::from_integer(2.into()), &mut w)?;
    let mut map = std::array::from_fn(|k| division::constant(node.center[k].clone()));
    for k in 0..4 {
        let displacement = if k == radial {
            r.clone()
        } else {
            let mut direction = division::constant(-Q::from_integer(2.into()));
            let mut e = [0; 4];
            e[k] = 1;
            direction.insert(e, Q::from_integer(4.into()), &mut w)?;
            r.product(&direction, &mut w)?
        };
        for (e, c) in displacement.terms {
            map[k].insert(e, c, &mut w)?;
        }
    }
    let transformed = node
        .source
        .equations()
        .iter()
        .map(|p| localization::substitute(p, &map, &mut w))
        .collect::<Result<Vec<_>>>()?;
    let (equations, pairs) = localization::saturate(&transformed, &r, limits, &mut w)?;
    Ok(QualifiedSsBlowupChart {
        node: node.clone(),
        radius,
        radial,
        map,
        equations,
        limits,
        work: w.used,
        pairs,
    })
}
mod root;
pub use root::{QualifiedSsBlowupRootArc, qualify_ss_blowup_root_arc};
