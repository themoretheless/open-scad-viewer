//! Finite original rational graph contacts with one proved implicit branch.
use super::affine_plane::q;
use super::*;
use cad_predicates::{BezierIdentity, BezierIdentityDecision};
use num_rational::BigRational as Q;
mod chart;
mod clock;
mod coupled;
mod edge;
mod identity;
pub use edge::GraphBoundaryProof;
mod membership;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalQuadricGraphParameterization {
    PolynomialAffine,
    SeparableProjective,
    SeparableRational,
    CoupledBilinear,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalQuadricGraphUvProof {
    ExactSingleton,
    OriginalAffineControlBounds,
    QuadricCoordinateSign,
}
#[derive(Debug)]
pub struct OriginalQuadricGraphContact {
    /// Natural coordinates [curve t, target u, target v].
    pub parameter_box: Box3,
    pub uv_proof: OriginalQuadricGraphUvProof,
    pub position_bounds: [[f64; 2]; 3],
    pub source_spans: Vec<usize>,
    pub multiplicities: Vec<usize>,
    pub normalized_original_span_parameters: Vec<[f64; 2]>,
}
#[derive(Debug)]
pub struct OriginalQuadricGraphContacts {
    pub contacts: Vec<OriginalQuadricGraphContact>,
    pub ambiguous_parameters: Vec<[f64; 2]>,
    pub target_graph_proven: bool,
    pub target_parameterization: OriginalQuadricGraphParameterization,
    pub target_membership: BezierIdentityDecision,
    pub carrier: Option<OriginalQuadricCarrierRoots>,
    pub boundary_exact_work: u64,
    pub boundary_proofs: Vec<GraphBoundaryProof>,
    pub exact_work: u64,
    pub complete: bool,
}
/// Original clamped B-spline graph with positive exactly separable weights
/// and exact Greville projection. Multi-span clocks require a complete exact
/// monotone homogeneous Bernstein restriction census and continuous joins.
/// Single Bezier graphs use weighted Bernstein clocks and their exact shortcut.
/// Planar degree-(1,1) patches additionally admit nonseparable positive weights
/// through a joint bilinear inverse and a proved global projection bijection;
/// implicit residual is linear in the remaining coordinate with a nonzero
/// coefficient throughout the projected rectangle. Exact tensor membership
/// and bijective projection prove the actual finite surface, not only its
/// carrier. Other charts/branches refuse this contract. No trim/Body credit.
/// max_exact_work bounds target, carrier and coordinate predicate work. Fixed
/// control/degree-bounded rational chart, inverse-clock schedules and singleton checks
/// are separate operations.
pub fn original_quadric_graph_contacts(
    c: &Curve,
    s: &Surface,
    matrix: [[f64; 4]; 4],
    products: &[[f64; 2]],
    tolerance: f64,
    max_spans: usize,
    max_exact_work: u64,
) -> Result<OriginalQuadricGraphContacts> {
    c.validate()?;
    check(
        !c.periodic
            && c.control_points[0].len() == 3
            && (1..=25).contains(&c.degree)
            && c.control_points.len() <= 256,
        "Invalid original quadric graph source curve",
    )?;
    check(
        tolerance.is_finite()
            && tolerance > 0.
            && (1..=100000).contains(&max_spans)
            && products.len() <= 16
            && matrix
                .iter()
                .flatten()
                .chain(products.iter().flatten())
                .all(|x| x.is_finite()),
        "Invalid quadric graph source or limits",
    )?;
    let [lo, hi] = c.domain();
    check(
        c.knots
            .iter()
            .filter(|&&t| lo < t && t < hi)
            .all(|t| c.knots.iter().filter(|v| *v == t).count() <= c.degree),
        "Original quadric graph contacts require C0 source knots",
    )?;
    let chart = chart::Chart::original(s, &matrix)?;
    let target_membership = identity::prove(s, matrix, products, max_exact_work)?;
    let work = target_membership.work_used;
    let proven = target_membership.outcome == BezierIdentity::Equal;
    let mut out = OriginalQuadricGraphContacts {
        contacts: vec![],
        ambiguous_parameters: vec![],
        target_graph_proven: proven,
        target_parameterization: chart.parameterization,
        target_membership,
        carrier: None,
        exact_work: work,
        boundary_exact_work: 0,
        boundary_proofs: vec![],
        complete: false,
    };
    if !proven {
        return Ok(out);
    }
    let carrier = original_quadric_carrier_roots(
        c,
        matrix,
        products,
        tolerance,
        max_spans,
        max_exact_work - work,
    )?;
    out.exact_work += carrier.exact_work;
    for root in &carrier.roots {
        let mut decision = membership::contact(c, s, &chart, matrix, products, root, &[None; 4])?;
        let eligible = match &decision {
            membership::Decision::Unresolved(edges) => Some(*edges),
            _ => None,
        };
        if let Some(eligible) = eligible {
            let proofs = edge::prove(
                c,
                s,
                &chart,
                matrix,
                products,
                root,
                eligible,
                max_exact_work - out.exact_work,
            )?;
            out.exact_work += proofs.work;
            out.boundary_exact_work += proofs.work;
            decision = membership::contact(c, s, &chart, matrix, products, root, &proofs.signs)?;
            out.boundary_proofs.extend(proofs.records);
        }
        match decision {
            membership::Decision::Outside => {}
            membership::Decision::Unresolved(_) => out.ambiguous_parameters.push(root.parameter),
            membership::Decision::Contact(contact) => out.contacts.push(contact),
        }
    }
    out.complete = carrier.complete && out.ambiguous_parameters.is_empty();
    out.carrier = Some(carrier);
    Ok(out)
}
