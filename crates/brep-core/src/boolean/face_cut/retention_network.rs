//! Atomic material planning across original support faces of a corner.
use super::{OwnedCut, invalid, plan_projected_retention, unproven};
use crate::Model;
use nurbs_core::Result;

pub struct FaceRetention<'a> {
    /// Index in the immutable input model, never an intermediate descendant.
    pub face: usize,
    pub cuts: &'a [OwnedCut],
    pub keep_uv: &'a [[f64; 2]],
}
pub struct RetentionRegion {
    pub source_face: usize,
    pub descendants: Vec<usize>,
    pub retained_faces: Vec<usize>,
    pub discard_faces: Vec<usize>,
    pub cut_edges: Vec<usize>,
}
pub struct RetentionNetwork {
    /// Every sector is still present. Discarding material requires replacement
    /// surfaces and a separate closed-body admission.
    pub model: Model,
    pub regions: Vec<RetentionRegion>,
    pub cells: usize,
    pub exact_work: u64,
    pub domain_cells: usize,
}
/// Project and classify a set of support-face cuts as one transaction. The
/// immutable source supplies the topology and charts; all intermediate models
/// remain private. Budgets are shared across faces, including seed winding.
/// Contact generation, endpoint binding and corner surfaces are separate steps.
pub fn plan_retention_network(
    source: &Model,
    requests: &[FaceRetention<'_>],
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
    max_domain_cells: usize,
) -> Result<RetentionNetwork> {
    source.validate()?;
    if !(1..=256).contains(&requests.len())
        || !(1..=100000).contains(&max_cells)
        || max_exact > 1000000
        || !(1..=100000).contains(&max_domain_cells)
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
    {
        return Err(invalid(
            "Need bounded support-face requests and shared work",
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    for request in requests {
        if request.face >= source.faces.len() || !seen.insert(request.face) {
            return Err(invalid("Support faces must be distinct original faces"));
        }
        if request
            .cuts
            .iter()
            .any(|cut| cut.endpoints.iter().any(|&v| v >= source.vertices.len()))
        {
            return Err(invalid("Cut endpoints must belong to the original model"));
        }
    }
    let mut out = RetentionNetwork {
        model: source.clone(),
        regions: Vec::new(),
        cells: 0,
        exact_work: 0,
        domain_cells: 0,
    };
    for request in requests {
        if out.cells == max_cells || out.domain_cells == max_domain_cells {
            return Err(unproven("Shared support-face planning work exhausted"));
        }
        let plan = plan_projected_retention(
            &out.model,
            request.face,
            request.cuts,
            request.keep_uv,
            tolerance_uv,
            max_cells - out.cells,
            max_exact - out.exact_work,
            max_domain_cells - out.domain_cells,
        )?;
        out.cells += plan.network.cells;
        out.exact_work += plan.network.exact_work;
        out.domain_cells += plan.domain_cells;
        out.regions.push(RetentionRegion {
            source_face: request.face,
            descendants: plan.network.faces,
            retained_faces: plan.retained_faces,
            discard_faces: plan.discard_faces,
            cut_edges: plan.network.edges,
        });
        out.model = plan.network.model;
    }
    out.model.refresh_change_set(&[source]);
    Ok(out)
}
