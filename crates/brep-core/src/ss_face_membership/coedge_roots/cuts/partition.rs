//! Atomic batches on actual models; final checks always use original curves.
use super::*;
use crate::edge_partition::PcurveCut;
use nurbs_core::surface::Surface;
mod open;
pub use open::{
    OpenPathStep, OpenSSFaceLimits, OpenSSPathLimits, OpenSSPathRequest, OpenSSRootGuard,
    OpenSSSegment, QualifiedOpenSSFace, QualifiedOpenSSPath, partition_open_ss_path,
    qualify_open_ss_path,
};
mod proof;
use proof::*;
#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug)]
pub struct CutPartitionLimits {
    pub operations: usize,
    pub cells: usize,
    pub curve_spans: usize,
    pub jobs: usize,
}
impl Default for CutPartitionLimits {
    fn default() -> Self {
        Self {
            operations: 1000000,
            cells: 1000000,
            curve_spans: 100000,
            jobs: 512,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct PartitionedCutVertex {
    pub model: usize,
    pub node: usize,
    pub vertex: usize,
    pub source_edge: usize,
    pub source_parameter: f64,
}
/// Candidate edge subdivision only; no new SS face loops or Solid admission.
/// The same input model on both sides is cloned once, with both side indexes
/// pointing to the single result. Errors publish no partially modified model.
pub struct QualifiedCutPartitions<'p, 'r, 'a> {
    source: &'p QualifiedCutRepresentatives<'r, 'a>,
    limits: CutPartitionLimits,
    models: Vec<Model>,
    side_model: [usize; 2],
    vertices: Vec<PartitionedCutVertex>,
    operations: usize,
    cells: usize,
    curve_spans: usize,
}
impl QualifiedCutPartitions<'_, '_, '_> {
    pub fn models(&self) -> &[Model] {
        &self.models
    }
    pub fn model_for_side(&self, side: usize) -> Option<&Model> {
        self.side_model.get(side).map(|&i| &self.models[i])
    }
    pub fn side_model(&self) -> [usize; 2] {
        self.side_model
    }
    pub fn vertices(&self) -> &[PartitionedCutVertex] {
        &self.vertices
    }
    pub fn operations(&self) -> usize {
        self.operations
    }
    pub fn cells(&self) -> usize {
        self.cells
    }
    pub fn curve_spans(&self) -> usize {
        self.curve_spans
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = partition_cut_representatives(self.source, self.limits)?;
        if self.models != fresh.models
            || self.side_model != fresh.side_model
            || self.vertices != fresh.vertices
            || self.operations != fresh.operations
            || self.cells != fresh.cells
            || self.curve_spans != fresh.curve_spans
        {
            return Err(unproven("Actual SS cut partition replay changed"));
        }
        Ok(())
    }
}
#[derive(Clone)]
struct Job {
    model: usize,
    edge: usize,
    node: usize,
    band: [f64; 2],
    cuts: Vec<usize>,
    tolerance: f64,
}
fn jobs(
    input: &QualifiedCutRepresentatives<'_, '_>,
    side_model: [usize; 2],
    used: &mut usize,
    limits: CutPartitionLimits,
) -> Result<Vec<Job>> {
    let mut jobs = Vec::<Job>::new();
    for (ci, c) in input.cuts.iter().enumerate() {
        let r = &input.roots.records[c.record];
        let e = &r.events[c.event];
        let model = side_model[r.address.side];
        spend(used, limits.operations, jobs.len() + 1)?;
        if let Some(job) = jobs
            .iter_mut()
            .find(|j| j.model == model && j.edge == r.address.edge && j.node == c.node)
        {
            job.band = [
                job.band[0].max(e.edge_parameter[0]),
                job.band[1].min(e.edge_parameter[1]),
            ];
            job.cuts.push(ci);
            job.tolerance = job.tolerance.min(c.world_tolerance);
            if job.band[0] > job.band[1] {
                return Err(unproven(
                    "One original cut identity has disjoint actual-edge clock bands",
                ));
            }
        } else {
            if jobs.len() == limits.jobs {
                return Err(unproven("Actual cut job ceiling exhausted"));
            }
            jobs.push(Job {
                model,
                edge: r.address.edge,
                node: c.node,
                band: e.edge_parameter,
                cuts: vec![ci],
                tolerance: c.world_tolerance,
            });
        }
    }
    jobs.sort_by(|a, b| {
        a.model
            .cmp(&b.model)
            .then(a.edge.cmp(&b.edge))
            .then(b.band[0].total_cmp(&a.band[0]))
    });
    for pair in jobs.windows(2) {
        let [a, b] = pair else { unreachable!() };
        if a.model == b.model && a.edge == b.edge && b.band[1] >= a.band[0] {
            return Err(unproven(
                "Distinct original cut identities need separated actual-edge clock bands",
            ));
        }
    }
    Ok(jobs)
}
/// Update all actual uses for every retained source event, in descending source
/// parameter order. Final world and UV descendants are independently compared
/// with their immutable originals, preventing accumulation across earlier cuts.
pub fn partition_cut_representatives<'p, 'r, 'a>(
    input: &'p QualifiedCutRepresentatives<'r, 'a>,
    limits: CutPartitionLimits,
) -> Result<QualifiedCutPartitions<'p, 'r, 'a>> {
    if !(1..=1000000).contains(&limits.operations)
        || !(1..=1000000).contains(&limits.cells)
        || !(1..=100000).contains(&limits.curve_spans)
        || !(1..=512).contains(&limits.jobs)
    {
        return Err(invalid("Need bounded actual SS cut partition work"));
    }
    let mut operations = 0;
    let mut cells = 0;
    let mut curve_spans = 0;
    spend(&mut operations, limits.operations, input.operations)?;
    spend(&mut cells, limits.cells, input.roots.work.cells)?;
    spend(&mut curve_spans, limits.curve_spans, input.curve_spans)?;
    input.reverify()?;
    let roots = input.roots;
    let same = std::ptr::eq(roots.faces[0].0, roots.faces[1].0);
    let originals = if same {
        vec![roots.faces[0].0]
    } else {
        vec![roots.faces[0].0, roots.faces[1].0]
    };
    let side_model = if same { [0, 0] } else { [0, 1] };
    let mut models: Vec<_> = originals.iter().map(|m| (*m).clone()).collect();
    let jobs = jobs(input, side_model, &mut operations, limits)?;
    let mut vertices = Vec::new();
    let mut descendants = std::collections::BTreeMap::<(usize, usize), Vec<usize>>::new();
    for job in &jobs {
        let source = originals[job.model];
        let current = &models[job.model];
        let original_edge = &source.edges[job.edge].curve;
        let t = representative(
            job.band,
            original_edge.domain(),
            &mut operations,
            limits.operations,
        )?;
        let mut cuts = Vec::new();
        // Resolve shifted coedge ordinals from current topology. Original loop
        // and edge identities supply provenance, never an obsolete ordinal.
        for (face, f) in current.faces.iter().enumerate() {
            for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
                for (coedge, c) in current.loops[wire].coedges.iter().enumerate() {
                    if c.edge != job.edge {
                        continue;
                    }
                    let originals_on_loop: Vec<_> = source.loops[wire]
                        .coedges
                        .iter()
                        .enumerate()
                        .filter(|(_, c)| c.edge == job.edge)
                        .collect();
                    if originals_on_loop.len() != 1 {
                        return Err(unproven("Shared edge needs one original use per loop"));
                    }
                    let (old_index, old) = originals_on_loop[0];
                    if old.pcurve.periodic {
                        return Err(unproven(
                            "Shared cut requires nonperiodic original pcurve clocks",
                        ));
                    }
                    agreement(
                        original_edge,
                        &old.pcurve,
                        &source.faces[face].surface,
                        old.reversed,
                        job.tolerance,
                        &mut cells,
                        limits.cells,
                    )?;
                    spend(&mut operations, limits.operations, 16)?;
                    let mut band = edge_parameter(
                        old.pcurve.domain(),
                        unit_clock(original_edge.domain(), job.band, old.reversed)?,
                        false,
                    )?;
                    // For selected supports, use their own certified UV query
                    // band, rather than an affine world-clock reconstruction.
                    for &ci in &job.cuts {
                        let cut = &input.cuts[ci];
                        let r = &roots.records[cut.record];
                        if r.address.wire == wire && r.address.coedge == old_index {
                            let e = &r.events[cut.event];
                            band = [
                                band[0].max(e.pcurve_parameter[0]),
                                band[1].min(e.pcurve_parameter[1]),
                            ];
                        }
                    }
                    let pt = representative(
                        band,
                        c.pcurve.domain(),
                        &mut operations,
                        limits.operations,
                    )?;
                    cuts.push(PcurveCut {
                        wire,
                        coedge,
                        parameter: pt,
                    });
                }
            }
        }
        if cells == limits.cells {
            return Err(unproven("Actual edge partition cell budget exhausted"));
        }
        let split = crate::edge_partition::partition_with_position(
            current,
            job.edge,
            t,
            &cuts,
            input.nodes[job.node].position,
            job.tolerance,
            limits.cells - cells,
        )?;
        spend(&mut cells, limits.cells, split.cells)?;
        let descendants = descendants
            .entry((job.model, job.edge))
            .or_insert_with(|| vec![job.edge]);
        descendants.push(split.child_edges[1]);
        vertices.push(PartitionedCutVertex {
            model: job.model,
            node: job.node,
            vertex: split.vertex,
            source_edge: job.edge,
            source_parameter: t,
        });
        models[job.model] = split.model;
    }
    for (&(slot, original_edge), edges) in &descendants {
        let original = originals[slot];
        let model = &models[slot];
        let tolerance = jobs
            .iter()
            .filter(|j| j.model == slot && j.edge == original_edge)
            .map(|j| j.tolerance)
            .fold(f64::INFINITY, f64::min);
        let world_carrier = carrier(&original.edges[original_edge].curve);
        for &edge in edges {
            let c = &model.edges[edge].curve;
            let d = c.domain();
            let pc = crate::line(vec![d[0], 0.], vec![d[1], 0.]);
            agreement(
                c,
                &pc,
                &world_carrier,
                false,
                tolerance,
                &mut cells,
                limits.cells,
            )?;
        }
        for (face, f) in original.faces.iter().enumerate() {
            for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
                let uses: Vec<_> = original.loops[wire]
                    .coedges
                    .iter()
                    .filter(|c| c.edge == original_edge)
                    .collect();
                if uses.is_empty() {
                    continue;
                }
                if uses.len() != 1 {
                    return Err(unproven("Ambiguous original UV descendant provenance"));
                }
                let uv_carrier = carrier(&embed_uv(&uses[0].pcurve));
                for c in model.loops[wire]
                    .coedges
                    .iter()
                    .filter(|c| edges.contains(&c.edge))
                {
                    let uv = embed_uv(&c.pcurve);
                    let d = uv.domain();
                    let pc = crate::line(vec![d[0], 0.], vec![d[1], 0.]);
                    agreement(
                        &uv,
                        &pc,
                        &uv_carrier,
                        false,
                        input.limits.uv_tolerance,
                        &mut cells,
                        limits.cells,
                    )?;
                    agreement(
                        &model.edges[c.edge].curve,
                        &c.pcurve,
                        &original.faces[face].surface,
                        c.reversed,
                        tolerance,
                        &mut cells,
                        limits.cells,
                    )?;
                }
            }
        }
    }
    // Direct checks against original root UV boxes use actual final endpoint
    // poles, not a sum of independent proposal and subdivision error estimates.
    for cut in &input.cuts {
        let r = &roots.records[cut.record];
        let e = &r.events[cut.event];
        let slot = side_model[r.address.side];
        let v = vertices
            .iter()
            .find(|v| v.model == slot && v.node == cut.node && v.source_edge == r.address.edge)
            .ok_or_else(|| unproven("Missing actual source event cut vertex"))?;
        let model = &models[slot];
        let descendants = &descendants[&(slot, r.address.edge)];
        let mut found = 0;
        let source_root = &r.bindings.census().roots()[e.original_root];
        for c in model.loops[r.address.wire]
            .coedges
            .iter()
            .filter(|c| descendants.contains(&c.edge))
        {
            let edge = &model.edges[c.edge];
            if !edge.vertices.contains(&v.vertex) {
                continue;
            }
            let start = (edge.vertices[0] == v.vertex) != c.reversed;
            let p = if start {
                &c.pcurve.control_points[0]
            } else {
                c.pcurve.control_points.last().unwrap()
            };
            spend(&mut operations, limits.operations, 8)?;
            if distance(p, &source_root.native_uv[r.address.side])? > input.limits.uv_tolerance {
                return Err(unproven(
                    "Final actual pcurve endpoint exceeds original root UV tolerance",
                ));
            }
            found += 1;
        }
        if found != 2 {
            return Err(unproven(
                "Source event needs both final incident pcurve endpoints",
            ));
        }
        if model.vertices[v.vertex].point != input.nodes[cut.node].position {
            return Err(unproven("Authored source node position changed"));
        }
    }
    for model in &models {
        model.validate()?;
        if !model.persistent_naming_complete() {
            return Err(unproven("Actual partition lost topology identity"));
        }
    }
    Ok(QualifiedCutPartitions {
        source: input,
        limits,
        models,
        side_model,
        vertices,
        operations,
        cells,
        curve_spans,
    })
}
