//! Complete ordinary-edge and collapsed-pole coverage of ONE original body.
//! This is a stratified boundary certificate, not whole-surface G1/G2.
use super::{Carrier, Class, Collection, CollectionReport, Limits, collection};
use crate::{source_pole_tangency as poles, source_volume::Body};
use nurbs_core::{Error, Result};

/// The requirement applies to every collapsed pole of the admitted body.
/// Sharp strata must be explicitly C0; no failed smooth proof is downgraded.
pub enum PoleRequirement {
    C0,
    TangentGraph(poles::GraphLimits),
    FacewiseCurvature(poles::PoleCurvatureLimits),
}
/// Additional aggregate work for every polar seam of this body.
#[derive(Clone, Copy)]
pub struct PolarBudgets {
    pub normal_coefficients: usize,
    pub shape_products: u64,
    pub shape_coefficients: usize,
}
/// Native proofs, never caller flags. Facewise curvature limits at poles do
/// not assert G2 on incident ordinary seams or a C2 neighborhood.
pub enum PoleStrata<'a, E: Carrier> {
    Absent,
    C0,
    TangentGraph(poles::PoleGraphAtlas<'a, E>),
    FacewiseCurvature(poles::PoleCurvatureAtlas<'a, E>),
}
pub struct Boundary<'a, E: Carrier> {
    body: &'a Body<E>,
    ordinary: Collection<'a, E>,
    poles: PoleStrata<'a, E>,
}
impl<'a, E: Carrier> Boundary<'a, E> {
    pub fn body(&self) -> &'a Body<E> {
        self.body
    }
    pub fn ordinary(&self) -> &Collection<'a, E> {
        &self.ordinary
    }
    pub fn poles(&self) -> &PoleStrata<'a, E> {
        &self.poles
    }
}
pub struct BoundaryReport<'a, E: Carrier> {
    pub certificate: Option<Boundary<'a, E>>,
    pub ordinary_cells: usize,
    pub ordinary_curve_spans: usize,
    pub ordinary_normal_spans: usize,
    pub ordinary_exact_work: u64,
    pub polar_normal_coefficients: usize,
    pub polar_shape_products: u64,
    pub polar_exact_work: u64,
    pub polar_shape_coefficients: usize,
    pub completed_edges: usize,
    pub uncertain_edge: Option<usize>,
    pub pole_graph_exact_work: u64,
    pub pole_jet_work: u64,
    pub reason: &'static str,
}
/// Fresh proofs on this body only. Every ordinary edge is classified exactly
/// once. Pole graph/jet budgets and ordinary seam budgets remain separately
/// explicit; a failure at either stage returns no complete boundary token.
/// The original body supplies embedded geometry, material orientation and
/// topology. No ideal source law or interior smoothness is inferred here.
pub fn qualify_boundary<'a, E: Carrier>(
    body: &'a Body<E>,
    classes: &[Class],
    requirement: PoleRequirement,
    limits: Limits,
) -> Result<BoundaryReport<'a, E>> {
    qualify_boundary_impl(body, classes, requirement, limits, BoundaryExtras::default())
}
/// Additionally qualify meridian tangent planes on the full closed carrier
/// using native factored-normal polynomials from this freshly admitted pole
/// atlas. A separate shared coefficient allowance covers every polar query.
/// Curvature-class ordinary seams still require their own regular G2 proof.
pub fn qualify_boundary_with_polar_seams<'a, E: Carrier>(
    body: &'a Body<E>,
    classes: &[Class],
    requirement: PoleRequirement,
    limits: Limits,
    normal_coefficients: usize,
) -> Result<BoundaryReport<'a, E>> {
    if !(1..=100000000).contains(&normal_coefficients) {
        return Err(Error::new(
            "BREP_SOURCE_BOUNDARY_SMOOTHNESS",
            "Choose bounded polar seam coefficients",
        ));
    }
    qualify_boundary_impl(
        body,
        classes,
        requirement,
        limits,
        BoundaryExtras {
            polar_coefficients: Some(normal_coefficients),
            ..BoundaryExtras::default()
        },
    )
}
/// Complete declared polar G2 carriers require this body's freshly admitted
/// common exact pole jets AND graph atlas, not only leading tangent planes.
pub fn qualify_boundary_with_polar_curvature<'a, E: Carrier>(
    body: &'a Body<E>,
    classes: &[Class],
    poles: poles::PoleCurvatureLimits,
    limits: Limits,
    budgets: PolarBudgets,
) -> Result<BoundaryReport<'a, E>> {
    if !(1..=100000000).contains(&budgets.normal_coefficients)
        || !(1..=100000000).contains(&budgets.shape_products)
        || !(1..=100000000).contains(&budgets.shape_coefficients)
    {
        return Err(Error::new(
            "BREP_SOURCE_BOUNDARY_SMOOTHNESS",
            "Choose bounded aggregate polar work",
        ));
    }
    qualify_boundary_impl(
        body,
        classes,
        PoleRequirement::FacewiseCurvature(poles),
        limits,
        BoundaryExtras {
            polar_coefficients: Some(budgets.normal_coefficients),
            curvature_budgets: Some(budgets),
            ..BoundaryExtras::default()
        },
    )
}
/// Exact original meridian jets plus full native regularity/limit coverage.
/// Exact work is one additional aggregate allowance for all polar G2 edges.
pub fn qualify_boundary_with_exact_polar_curvature<'a, E: Carrier>(
    body: &'a Body<E>,
    classes: &[Class],
    poles: poles::PoleCurvatureLimits,
    limits: Limits,
    budgets: PolarBudgets,
    max_exact_work: u64,
) -> Result<BoundaryReport<'a, E>> {
    if !(1..=100000000).contains(&max_exact_work)
        || !(1..=100000000).contains(&budgets.normal_coefficients)
        || !(1..=100000000).contains(&budgets.shape_products)
        || !(1..=100000000).contains(&budgets.shape_coefficients)
    {
        return Err(Error::new(
            "BREP_SOURCE_BOUNDARY_SMOOTHNESS",
            "Choose bounded exact polar work",
        ));
    }
    qualify_boundary_impl(
        body,
        classes,
        PoleRequirement::FacewiseCurvature(poles),
        limits,
        BoundaryExtras {
            polar_coefficients: Some(budgets.normal_coefficients),
            curvature_budgets: Some(budgets),
            exact_work: Some(max_exact_work),
            ..BoundaryExtras::default()
        },
    )
}
/// Exact G2 identities AND complete regularity on every curvature-class source
/// carrier, polar or regular. ONE exact allowance is shared across both kinds.
/// Tangent-plane classes retain their declared angular staging requirement;
/// whole exact G1 and material orientation remain separate native factories.
pub fn qualify_boundary_with_exact_curvature<'a, E: Carrier>(
    body: &'a Body<E>,
    classes: &[Class],
    axes: &[usize],
    poles: poles::PoleCurvatureLimits,
    limits: Limits,
    budgets: PolarBudgets,
    max_exact_work: u64,
) -> Result<BoundaryReport<'a, E>> {
    if axes.len() != classes.len()
        || axes.iter().any(|a| *a > 2)
        || !(1..=100000000).contains(&max_exact_work)
        || [budgets.normal_coefficients, budgets.shape_coefficients]
            .iter()
            .any(|n| !(1..=100000000).contains(n))
        || !(1..=100000000).contains(&budgets.shape_products)
    {
        return Err(Error::new(
            "BREP_SOURCE_BOUNDARY_SMOOTHNESS",
            "Choose complete exact projection proposals and shared bounded work",
        ));
    }
    qualify_boundary_impl(
        body,
        classes,
        PoleRequirement::FacewiseCurvature(poles),
        limits,
        BoundaryExtras {
            polar_coefficients: Some(budgets.normal_coefficients),
            curvature_budgets: Some(budgets),
            exact_work: Some(max_exact_work),
            exact_axes: Some(axes),
        },
    )
}
/// Exact G2 on ordinary regular carriers beside C1 physical pole stars.
/// Polar tangent coverage and exact ordinary work have distinct shared ledgers;
/// this does not assert C2 poles or substitute tolerance G1 for exact gluing.
pub fn qualify_boundary_with_exact_regular_curvature<'a, E: Carrier>(
    body: &'a Body<E>,
    classes: &[Class],
    axes: &[usize],
    graphs: poles::GraphLimits,
    limits: Limits,
    normal_coefficients: usize,
    max_exact_work: u64,
) -> Result<BoundaryReport<'a, E>> {
    if axes.len() != classes.len()
        || axes.iter().any(|a| *a > 2)
        || !(1..=100000000).contains(&normal_coefficients)
        || !(1..=100000000).contains(&max_exact_work)
    {
        return Err(Error::new(
            "BREP_SOURCE_BOUNDARY_SMOOTHNESS",
            "Choose complete exact projection proposals and bounded aggregate work",
        ));
    }
    qualify_boundary_impl(
        body,
        classes,
        PoleRequirement::TangentGraph(graphs),
        limits,
        BoundaryExtras {
            polar_coefficients: Some(normal_coefficients),
            exact_work: Some(max_exact_work),
            exact_axes: Some(axes),
            ..BoundaryExtras::default()
        },
    )
}
/// Optional polar-coefficient, curvature, exact-work and exact-axis
/// qualifications added to the boundary qualification.
#[derive(Clone, Copy, Default)]
struct BoundaryExtras<'x> {
    polar_coefficients: Option<usize>,
    curvature_budgets: Option<PolarBudgets>,
    exact_work: Option<u64>,
    exact_axes: Option<&'x [usize]>,
}
fn qualify_boundary_impl<'a, E: Carrier>(
    body: &'a Body<E>,
    classes: &[Class],
    requirement: PoleRequirement,
    limits: Limits,
    extras: BoundaryExtras<'_>,
) -> Result<BoundaryReport<'a, E>> {
    let BoundaryExtras {
        polar_coefficients,
        curvature_budgets,
        exact_work,
        exact_axes,
    } = extras;
    super::validate_limits(&limits)?;
    let shell = body.geometry().shell();
    if classes.is_empty() || classes.len() != shell.edges().len() || classes.len() > 100_000 {
        return Err(Error::new(
            "BREP_SOURCE_BOUNDARY_SMOOTHNESS",
            "Classify every original edge exactly once",
        ));
    }
    collection::validate_classes(classes)?;
    let mut out = BoundaryReport {
        certificate: None,
        ordinary_cells: 0,
        ordinary_curve_spans: 0,
        ordinary_normal_spans: 0,
        ordinary_exact_work: 0,
        polar_normal_coefficients: 0,
        polar_shape_products: 0,
        polar_exact_work: 0,
        polar_shape_coefficients: 0,
        completed_edges: 0,
        uncertain_edge: None,
        pole_graph_exact_work: 0,
        pole_jet_work: 0,
        reason: "source-boundary-pole-strata-unproven",
    };
    let strata = if shell.poles().is_empty() {
        PoleStrata::Absent
    } else {
        match requirement {
            PoleRequirement::C0 => PoleStrata::C0,
            PoleRequirement::TangentGraph(limits) => {
                let r = poles::qualify_body_pole_graphs(body, limits)?;
                out.pole_graph_exact_work = r.exact_work;
                let Some(proof) = r.certificate else {
                    out.reason = r.reason;
                    return Ok(out);
                };
                PoleStrata::TangentGraph(proof)
            }
            PoleRequirement::FacewiseCurvature(limits) => {
                let r = poles::qualify_body_pole_curvature(body, limits)?;
                out.pole_graph_exact_work = r.graph_exact_work;
                out.pole_jet_work = r.jet_work;
                let Some(proof) = r.certificate else {
                    out.reason = r.reason;
                    return Ok(out);
                };
                PoleStrata::FacewiseCurvature(proof)
            }
        }
    };
    let plane_atlas = match &strata {
        PoleStrata::TangentGraph(p) => Some(p.charts().transverse().atlas().planes()),
        PoleStrata::FacewiseCurvature(p) => Some(p.graphs().charts().transverse().atlas().planes()),
        _ => None,
    };
    let r = if let (PoleStrata::FacewiseCurvature(atlas), Some(budgets)) =
        (&strata, curvature_budgets)
    {
        collection::qualify_owned_edges_with_polar_curvature(
            body,
            classes,
            limits,
            atlas,
            budgets,
            exact_work,
            exact_axes.map(|a| (a, exact_work.unwrap())),
        )?
    } else if let (Some(planes), Some(coefficients)) = (plane_atlas, polar_coefficients) {
        collection::qualify_owned_edges_with_poles(
            body,
            classes,
            limits,
            planes,
            coefficients,
            exact_axes.map(|a| (a, exact_work.unwrap())),
        )?
    } else if let Some(axes) = exact_axes {
        collection::qualify_owned_edges_exact_regular(
            body,
            classes,
            limits,
            axes,
            exact_work.unwrap(),
        )?
    } else {
        collection::qualify_owned_edges(body, classes, limits)?
    };
    let CollectionReport {
        certificate,
        cells,
        curve_spans,
        normal_spans,
        polar_coefficients,
        polar_shape_products,
        polar_exact_work,
        regular_exact_work,
        polar_shape_coefficients,
        completed_edges,
        uncertain_edge,
        reason,
    } = r;
    out.ordinary_cells = cells;
    out.ordinary_curve_spans = curve_spans;
    out.ordinary_normal_spans = normal_spans;
    out.polar_normal_coefficients = polar_coefficients;
    out.polar_shape_products = polar_shape_products;
    out.polar_exact_work = polar_exact_work;
    out.ordinary_exact_work = regular_exact_work;
    out.polar_shape_coefficients = polar_shape_coefficients;
    out.completed_edges = completed_edges;
    out.uncertain_edge = uncertain_edge;
    out.reason = reason;
    if let Some(ordinary) = certificate {
        out.certificate = Some(Boundary {
            body,
            ordinary,
            poles: strata,
        });
        out.reason = "source-boundary-all-declared-edge-and-pole-strata-qualified";
    }
    Ok(out)
}
