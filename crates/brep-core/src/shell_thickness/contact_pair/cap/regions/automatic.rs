//! Owned actual-face intersections drive root binding, trace construction and
//! native sector cuts. One admitted local branch never grants a global census.
use super::{Limits as PostLimits, Record, Regions, require};
use crate::{Model, Result, TopoId, face_cut};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Fiber,
    Ruled,
    Branches,
}
#[derive(Clone, Copy)]
pub struct Limits {
    pub construction_cells: usize,
    pub exact_work: u64,
    pub boundary_cells: usize,
    pub trim_pairs: usize,
    pub trim_cells: usize,
    pub trim_domain_cells: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            construction_cells: 100000,
            exact_work: 1000000,
            boundary_cells: 100000,
            trim_pairs: 100000,
            trim_cells: 100000,
            trim_domain_cells: 1000000,
        }
    }
}
pub struct Evidence {
    pub method: Method,
    pub original_face: TopoId,
    pub endpoints: [usize; 2],
    pub boundary_coverage: Vec<(usize, crate::face_crossings::Report)>,
    pub cells: usize,
    pub evaluations: usize,
    pub root_error_upper_mm: Option<f64>,
    pub trace_pieces: usize,
    pub root_fibers: usize,
    pub branches: Vec<face_cut::FiberBranch>,
    pub rejected_pairs: usize,
    pub refusals: Vec<face_cut::FiberBranchRefusal>,
}
pub(super) struct Input {
    pub(super) cutter: Regions,
    face: usize,
    cutter_face: usize,
    method: Method,
    limits: Limits,
}
impl Regions {
    /// Choose only source/cutter faces and the construction method. Original edge
    /// roots, owned vertices, drive/search axis and the entire saved trace are
    /// generated and qualified internally. Both source sectors remain present.
    pub fn intersect(
        mut self,
        cutter: Regions,
        face: usize,
        cutter_face: usize,
        method: Method,
        limits: Limits,
    ) -> Result<Self> {
        require(self.cuts.len() < 64, "At most 64 retained cap face cuts")?;
        let nodes = self
            .provenance_nodes
            .checked_add(cutter.provenance_nodes)
            .ok_or_else(|| {
                crate::Error::new("BREP_OFFSET_CAP_REFUSED", "Cap provenance count overflow")
            })?;
        let depth = self.provenance_depth.max(cutter.provenance_depth + 1);
        require(
            nodes <= 256 && depth <= 32,
            "Automatic cap provenance exceeds 256 nodes or depth 32",
        )?;
        let input = Box::new(Input {
            cutter,
            face,
            cutter_face,
            method,
            limits,
        });
        let (model, mut record) = apply(&self.source, &self.model, &input)?;
        record.automatic = Some(input);
        self.model = model;
        self.cuts.push(record);
        self.provenance_nodes = nodes;
        self.provenance_depth = depth;
        Ok(self)
    }
}
pub(super) fn apply(
    source: &crate::shell_thickness::contact_pair::cap::Sheet,
    model: &Model,
    input: &Input,
) -> Result<(Model, Record)> {
    let l = input.limits;
    require(
        (32..=100000).contains(&l.construction_cells),
        "Choose bounded original root/trace/cut construction work",
    )?;
    let post = PostLimits {
        cut_cells: l.construction_cells,
        exact_work: l.exact_work,
        boundary_cells: l.boundary_cells,
        trim_pairs: l.trim_pairs,
        trim_cells: l.trim_cells,
        trim_domain_cells: l.trim_domain_cells,
        ..Default::default()
    };
    super::validate(post)?;
    let cutter = input.cutter.model();
    require(
        model.tolerance_mm == cutter.tolerance_mm
            && source.cap().tolerance_uv() == input.cutter.source.cap().tolerance_uv(),
        "Automatic cap intersection needs compatible world/UV tolerances",
    )?;
    require(
        input.cutter_face < cutter.faces.len(),
        "Missing actual automatic cutter face",
    )?;
    let original = *model.1.faces.get(input.face).ok_or_else(|| {
        crate::Error::new(
            "BREP_OFFSET_CAP_REFUSED",
            "Missing original automatic source face",
        )
    })?;
    let (partition, evidence) = match input.method {
        Method::Branches => return apply_branches(source, model, input, post, original),
        Method::Fiber => {
            let r = face_cut::partition_fiber_at_face_automatic(
                model,
                input.face,
                cutter,
                input.cutter_face,
                source.cap().tolerance_uv(),
                l.construction_cells,
                l.exact_work,
            )?;
            let evidence = Evidence {
                method: input.method,
                original_face: original,
                endpoints: r.endpoints,
                boundary_coverage: r.boundary_coverage,
                cells: r.cells,
                evaluations: r.evaluations,
                root_error_upper_mm: Some(r.root_error_upper_mm),
                trace_pieces: r.trace_pieces,
                root_fibers: r.root_fibers,
                branches: vec![face_cut::FiberBranch {
                    endpoints: r.endpoints,
                    edges: r.partition.edges.clone(),
                    root_fibers: r.root_fibers,
                }],
                rejected_pairs: 0,
                refusals: vec![],
            };
            (r.partition, evidence)
        }
        Method::Ruled => {
            let r = face_cut::partition_ruled_at_face(
                model,
                input.face,
                cutter,
                input.cutter_face,
                source.cap().tolerance_uv(),
                l.construction_cells,
                l.exact_work,
            )?;
            let evidence = Evidence {
                method: input.method,
                original_face: r.original_face_identity,
                endpoints: r.endpoints,
                boundary_coverage: r.boundary_coverage,
                cells: r.cells,
                evaluations: 0,
                root_error_upper_mm: None,
                trace_pieces: 1,
                root_fibers: 0,
                branches: vec![face_cut::FiberBranch {
                    endpoints: r.endpoints,
                    edges: r.partition.edges.clone(),
                    root_fibers: 0,
                }],
                rejected_pairs: 0,
                refusals: vec![],
            };
            (r.partition, evidence)
        }
    };
    require(
        evidence.cells <= l.construction_cells
            && partition.exact_work <= l.exact_work
            && evidence
                .boundary_coverage
                .iter()
                .all(|(_, r)| r.complete && r.source_is_owned)
            && evidence
                .boundary_coverage
                .iter()
                .map(|(_, r)| r.eligible.len())
                .sum::<usize>()
                == 2,
        "Automatic cap endpoints lack complete original owned boundary coverage",
    )?;
    if input.method == Method::Fiber {
        require(
            evidence
                .root_error_upper_mm
                .is_some_and(|e| e.is_finite() && e <= model.tolerance_mm * 0.5)
                && evidence.trace_pieces > 0
                && evidence.root_fibers > 0,
            "Automatic original root-fiber evidence remains unproved",
        )?;
    }
    let endpoints = evidence.endpoints;
    let (model, mut record) = super::finish(
        source,
        input.face,
        endpoints,
        vec![],
        None,
        vec![],
        post,
        partition,
    )?;
    record.intersection = Some(evidence);
    Ok((model, record))
}

pub(super) fn same(a: Option<&Evidence>, b: Option<&Evidence>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            a.method == b.method
                && a.original_face == b.original_face
                && a.endpoints == b.endpoints
                && a.cells == b.cells
                && a.evaluations == b.evaluations
                && a.root_error_upper_mm == b.root_error_upper_mm
                && a.trace_pieces == b.trace_pieces
                && a.root_fibers == b.root_fibers
                && a.branches.len() == b.branches.len()
                && a.rejected_pairs == b.rejected_pairs
                && a.refusals.len() == b.refusals.len()
                && a.branches.iter().zip(&b.branches).all(|(a, b)| {
                    a.endpoints == b.endpoints
                        && a.edges == b.edges
                        && a.root_fibers == b.root_fibers
                })
                && a.refusals.iter().zip(&b.refusals).all(|(a, b)| {
                    a.endpoints == b.endpoints && a.code == b.code && a.message == b.message
                })
                && a.boundary_coverage.len() == b.boundary_coverage.len()
                && a.boundary_coverage
                    .iter()
                    .zip(&b.boundary_coverage)
                    .all(|((ae, a), (be, b))| {
                        ae == be
                            && a.source_edge_identity == b.source_edge_identity
                            && a.support_face_identity == b.support_face_identity
                            && a.complete == b.complete
                            && a.eligible == b.eligible
                            && a.roots.len() == b.roots.len()
                            && a.unresolved.len() == b.unresolved.len()
                            && a.roots.iter().zip(&b.roots).all(|(a, b)| {
                                a.location == b.location
                                    && a.root.parameter_box == b.root.parameter_box
                                    && a.root.position_bounds == b.root.position_bounds
                            })
                    })
        }
        _ => false,
    }
}

fn apply_branches(
    source: &crate::shell_thickness::contact_pair::cap::Sheet,
    model: &Model,
    input: &Input,
    post: PostLimits,
    original: TopoId,
) -> Result<(Model, Record)> {
    let l = input.limits;
    let r = face_cut::partition_fiber_branches_at_face(
        model,
        input.face,
        input.cutter.model(),
        input.cutter_face,
        source.cap().tolerance_uv(),
        l.construction_cells,
        l.exact_work,
    )?;
    let owned = r
        .boundary_coverage
        .iter()
        .map(|(_, r)| r.eligible.len())
        .sum::<usize>();
    let ends = r
        .branches
        .iter()
        .flat_map(|b| b.endpoints)
        .collect::<std::collections::BTreeSet<_>>();
    require(
        !r.branches.is_empty()
            && owned == 2 * r.branches.len()
            && ends.len() == owned
            && r.branches
                .iter()
                .all(|b| b.root_fibers > 0 && !b.edges.is_empty())
            && r.boundary_coverage
                .iter()
                .all(|(_, r)| r.complete && r.source_is_owned)
            && r.charged_cells <= l.construction_cells
            && r.charged_exact_work <= l.exact_work
            && r.charged_evaluations <= 4096,
        "Boundary-connected branch matching remains unproved or exhausted",
    )?;
    let endpoints = r.branches[0].endpoints;
    let edges = r
        .branches
        .iter()
        .flat_map(|b| b.edges.iter().copied())
        .collect();
    let identities = r.faces.iter().map(|&f| r.model.1.faces[f]).collect();
    let evidence = Evidence {
        method: Method::Branches,
        original_face: original,
        endpoints,
        boundary_coverage: r.boundary_coverage,
        cells: r.charged_cells,
        evaluations: r.charged_evaluations,
        root_error_upper_mm: None,
        trace_pieces: 0,
        root_fibers: r.branches.iter().map(|b| b.root_fibers).sum(),
        branches: r.branches,
        rejected_pairs: r.rejected_pairs,
        refusals: r.refusals,
    };
    let (model, mut record) = super::finish_model(
        source,
        input.face,
        endpoints,
        vec![],
        None,
        vec![],
        post,
        r.model,
        original,
        r.faces,
        identities,
        edges,
        r.charged_cells,
        r.charged_exact_work,
    )?;
    record.intersection = Some(evidence);
    Ok((model, record))
}
