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
    let CollectionReport {
        certificate,
        cells,
        curve_spans,
        normal_spans,
        completed_edges,
        uncertain_edge,
        reason,
    } = collection::qualify_owned_edges(body, classes, limits)?;
    out.ordinary_cells = cells;
    out.ordinary_curve_spans = curve_spans;
    out.ordinary_normal_spans = normal_spans;
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
