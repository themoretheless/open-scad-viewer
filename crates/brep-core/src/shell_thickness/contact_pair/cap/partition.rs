//! Certified cap-rim subdivision. Every resulting edge and UV use is checked
//! directly against the original sealed cap definitions, not accumulated splits.
use super::{Sheet, require};
mod parameter;
use crate::{Model, Result, TopoId};
use nurbs_core::{
    curve_partition_agreement::mapped,
    surface_offset::contact_curve::chain::region::compound::cap::Cap, trim_region_audit::nesting,
};
pub use parameter::Search;
#[derive(Clone, Copy)]
pub struct Limits {
    pub edge_cells: usize,
    pub original_cells: usize,
    pub boundary_cells: usize,
    pub nesting: nesting::Limits,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            edge_cells: 100000,
            original_cells: 100000,
            boundary_cells: 100000,
            nesting: Default::default(),
        }
    }
}
pub struct Record {
    pub edge: usize,
    pub parameter: f64,
    pub vertex: Option<[f64; 3]>,
    pub search: Option<Search>,
    pub source_identity: TopoId,
    pub children: [TopoId; 2],
    pub edge_cells: usize,
    pub original_cells: usize,
    pub nesting: nesting::Report,
    pub(super) limits: Limits,
}
impl Sheet {
    pub fn partitions(&self) -> &[Record] {
        &self.partitions
    }
    /// Natural world-edge parameter is a proposal. UV parameters are proposed by
    /// normalized clock correspondence, then independently qualified over the
    /// complete resulting curves. Original roots remain the authority on replay.
    pub fn split_edge(self, edge: usize, parameter: f64, limits: Limits) -> Result<Self> {
        self.split_proposal(edge, parameter, None, limits)
    }
    /// The neighbor's literal vertex is only an authoring proposal. All world,
    /// UV, original-root and actual-surface checks must qualify the result.
    pub fn split_edge_to_vertex(
        self,
        edge: usize,
        parameter: f64,
        vertex: [f64; 3],
        limits: Limits,
    ) -> Result<Self> {
        self.split_proposal(edge, parameter, Some(vertex), limits)
    }
    /// Bounded samples/Newton steps propose a natural parameter only. No sampled
    /// distance grants a cut: complete original/UV/support proofs still decide.
    pub fn split_edge_at_vertex(
        self,
        edge: usize,
        vertex: [f64; 3],
        evaluations: usize,
        limits: Limits,
    ) -> Result<Self> {
        let curve = &self
            .model
            .edges
            .get(edge)
            .ok_or_else(|| crate::Error::new("BREP_OFFSET_CAP_REFUSED", "Unknown cap edge"))?
            .curve;
        let (parameter, used) = parameter::propose(curve, vertex, evaluations)?;
        let mut result = self.split_proposal(edge, parameter, Some(vertex), limits)?;
        result.partitions.last_mut().unwrap().search = Some(parameter::Search {
            budget: evaluations,
            evaluations: used,
        });
        Ok(result)
    }
    fn split_proposal(
        mut self,
        edge: usize,
        parameter: f64,
        vertex: Option<[f64; 3]>,
        limits: Limits,
    ) -> Result<Self> {
        require(
            self.partitions.len() < 64,
            "At most 64 original cap-rim subdivisions",
        )?;
        let (model, boundary, record) = apply(
            &self.cap,
            &self.model,
            &mut self.edge_roots,
            edge,
            parameter,
            vertex,
            limits,
        )?;
        self.model = model;
        self.boundary = boundary;
        self.partitions.push(record);
        Ok(self)
    }
}
fn reflect(t: f64, d: [f64; 2]) -> f64 {
    if t == d[0] {
        d[1]
    } else if t == d[1] {
        d[0]
    } else {
        d[0] + (d[1] - t)
    }
}
pub(super) fn apply(
    cap: &Cap,
    model: &Model,
    roots: &mut Vec<usize>,
    edge: usize,
    parameter: f64,
    vertex: Option<[f64; 3]>,
    l: Limits,
) -> Result<(Model, crate::boundary_agreement::Report, Record)> {
    validate_limits(l)?;
    let original = model
        .edges
        .get(edge)
        .ok_or_else(|| crate::Error::new("BREP_OFFSET_CAP_REFUSED", "Unknown cap edge"))?;
    let d = original.curve.domain();
    require(
        parameter.is_finite() && d[0] < parameter && parameter < d[1],
        "Subdivision parameter must be strictly inside the original edge",
    )?;
    let t = (parameter - d[0]) / (d[1] - d[0]);
    let mut cuts = vec![];
    for (wire, w) in model.loops.iter().enumerate() {
        for (coedge, c) in w.coedges.iter().enumerate() {
            if c.edge == edge {
                let p = c.pcurve.domain();
                let t = if c.reversed { 1. - t } else { t };
                cuts.push(crate::edge_partition::PcurveCut {
                    wire,
                    coedge,
                    parameter: p[0] + t * (p[1] - p[0]),
                });
            }
        }
    }
    let split = if let Some(vertex) = vertex {
        crate::edge_partition::partition_to_vertex(
            model,
            edge,
            parameter,
            &cuts,
            vertex,
            l.edge_cells,
        )?
    } else {
        crate::edge_partition::partition(model, edge, parameter, &cuts, l.edge_cells)?
    };
    let mut next_roots = roots.clone();
    next_roots.push(roots[edge]);
    let next = &split.model;
    require(
        next_roots.len() == next.edges.len(),
        "Subdivision lost original edge ownership",
    )?;
    let originals = cap
        .region()
        .world_loops()
        .iter()
        .flatten()
        .collect::<Vec<_>>();
    let uv = cap
        .region()
        .uv_domain()
        .loops()
        .iter()
        .flatten()
        .collect::<Vec<_>>();
    let mut cells = 0;
    // Recheck ALL current pieces directly against their original raw definitions.
    // Repeated splits therefore cannot accumulate an unchecked proxy error.
    for (i, e) in next.edges.iter().enumerate() {
        require(
            cells < l.original_cells,
            "Shared direct-original subdivision budget exhausted",
        )?;
        let proof = mapped::verify(
            originals[next_roots[i]],
            &e.curve,
            e.curve.domain(),
            e.curve.domain(),
            cap.boundary_tolerance(),
            (l.original_cells - cells).min(100000),
        )?;
        cells += proof.cells;
        require(
            proof.accepted,
            "World child lost direct original cap-rim correspondence",
        )?;
    }
    let mut loops = vec![];
    for w in &next.loops {
        let mut curves = vec![];
        for c in &w.coedges {
            let raw = uv[next_roots[c.edge]];
            let d = c.pcurve.domain();
            let source = if c.reversed {
                d.map(|t| reflect(t, raw.domain()))
            } else {
                d
            };
            require(
                cells < l.original_cells,
                "Shared direct-original UV subdivision budget exhausted",
            )?;
            let proof = mapped::verify(
                raw,
                &c.pcurve,
                source,
                d,
                cap.tolerance_uv(),
                (l.original_cells - cells).min(100000),
            )?;
            cells += proof.cells;
            require(
                proof.accepted,
                "UV child lost direct original cap-region correspondence",
            )?;
            curves.push(c.pcurve.clone());
        }
        loops.push(curves);
    }
    let forest = nesting::inspect(&loops, cap.tolerance_uv(), l.nesting)?;
    require(
        forest.proven
            && forest.nodes.as_ref().is_some_and(|nodes| {
                nodes.len() == cap.region().uv_domain().nodes().len()
                    && nodes
                        .iter()
                        .zip(cap.region().uv_domain().nodes())
                        .all(|(a, b)| a.parent == b.parent && a.depth == b.depth && !a.reversed)
            }),
        "Actual subdivided UV region nesting remains unproved",
    )?;
    let boundary = crate::boundary_agreement::verify(next, l.boundary_cells)?;
    require(
        boundary.complete,
        "Actual subdivided cap boundary agreement remains unproved",
    )?;
    let record = Record {
        edge,
        parameter,
        vertex,
        search: None,
        source_identity: split.source_identity,
        children: split.child_identities,
        edge_cells: split.cells,
        original_cells: cells,
        nesting: forest,
        limits: l,
    };
    *roots = next_roots;
    Ok((split.model, boundary, record))
}

pub(super) fn replay_search(model: &Model, record: &Record) -> Result<()> {
    if let Some(search) = record.search {
        let (parameter, used) = parameter::propose(
            &model.edges[record.edge].curve,
            record.vertex.unwrap(),
            search.budget,
        )?;
        require(
            parameter == record.parameter && used == search.evaluations,
            "Automatic partition parameter proposal differs on replay",
        )?;
    }
    Ok(())
}

pub(super) fn validate_limits(l: Limits) -> Result<()> {
    let n = l.nesting;
    require(
        (1..=1000000).contains(&l.edge_cells)
            && (1..=1000000).contains(&l.original_cells)
            && (1..=1000000).contains(&l.boundary_cells)
            && (1..=64).contains(&n.max_loops)
            && (2..=256).contains(&n.max_curves)
            && (1..=100000).contains(&n.pairs)
            && (1..=100000).contains(&n.cells)
            && (1..=1000000).contains(&n.domain_cells),
        "Choose bounded subdivision and complete nesting proof allowances",
    )
}
pub(super) fn propose_parameter(
    c: &nurbs_core::curve::Curve,
    p: [f64; 3],
    budget: usize,
) -> Result<(f64, usize)> {
    parameter::propose(c, p, budget)
}
