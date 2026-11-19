//! Qualified original-source open SS arcs to paired finite-face BRep cuts.
//! The saved curves are supplied unchanged; exact pre-owned world and UV
//! endpoints replace neither root proofs nor the existing partition audits.
use super::*;
use nurbs_core::ss_intersection as ss;

#[derive(Clone, Copy, Debug)]
pub struct OpenSsLimits {
    pub branches: ss::QualifiedSSBuildLimits,
    pub trace: ss::PhysicalFiberTraceLimits,
    /// Independent per-face allowances for automatic boundary prebinding.
    pub binding_cells: usize,
    pub binding_inversions: usize,
    pub cut_cells: usize,
    pub max_exact: u64,
}
impl Default for OpenSsLimits {
    fn default() -> Self {
        Self {
            branches: Default::default(),
            trace: Default::default(),
            binding_cells: 100000,
            binding_inversions: 10000,
            cut_cells: 100000,
            max_exact: 1000000,
        }
    }
}

pub struct OpenSsPartition {
    pub first: Partition,
    pub second: Partition,
    pub trace: ss::QualifiedPhysicalFiberTrace,
}

/// Partition two faces in distinct models along one complete regular open
/// component. Every natural-product root must belong to this branch. Extra
/// components, singular or closed results, unbound endpoints and any failed
/// proof refuse atomically; no approximate legacy SSI fallback is attempted.
/// `cut_cells` and `max_exact` are independent per-face work allowances.
pub fn partition_open_ss(
    first: &Model,
    first_face: usize,
    second: &Model,
    second_face: usize,
    endpoints: [[usize; 2]; 2],
    limits: OpenSsLimits,
) -> Result<OpenSsPartition> {
    first.validate()?;
    second.validate()?;
    if std::ptr::eq(first, second) {
        return Err(invalid(
            "Open SS paired cuts require distinct source models",
        ));
    }
    if !(1..=100000).contains(&limits.cut_cells) || limits.max_exact > 1000000 {
        return Err(invalid("Open SS per-face work limits are out of range"));
    }
    let a = first
        .faces
        .get(first_face)
        .ok_or_else(|| invalid("Missing first open SS face"))?;
    let b = second
        .faces
        .get(second_face)
        .ok_or_else(|| invalid("Missing second open SS face"))?;
    if limits.trace.tolerance_world > first.tolerance_mm.min(second.tolerance_mm) {
        return Err(invalid(
            "Open SS trace tolerance exceeds original model tolerance",
        ));
    }
    let source = ss::FiberSources::new(&a.surface, &b.surface)?;
    let dispatched = ss::construct_qualified_ss_branches(&source, limits.branches)?;
    let ss::QualifiedSurfaceIntersection::RegularOpenArcs(owner) = &dispatched else {
        return Err(unproven(
            "Paired open SS cut requires regular open original arcs",
        ));
    };
    if !owner.covers_entire_surfaces() || owner.branches().len() != 1 {
        return Err(unproven(
            "Paired open SS cut requires exactly one complete original component",
        ));
    }
    let trace = ss::construct_physical_fiber_trace(&owner, 0, limits.trace)?;
    trace.reverify()?;
    if trace.segments().is_empty() {
        return Err(unproven("Qualified open SS trace has no saved segments"));
    }

    let models = [first, second];
    let faces = [first_face, second_face];
    let paths: [Vec<Segment>; 2] = std::array::from_fn(|k| {
        trace
            .segments()
            .iter()
            .map(|s| Segment {
                curve: s.curve().clone(),
                pcurve: s.pcurves()[k].clone(),
            })
            .collect()
    });
    for support in 0..2 {
        let (wire, indices) = owned_boundary(models[support], faces[support], endpoints[support])
            .ok_or_else(|| {
            invalid("Open SS endpoints need unique occurrences on one original boundary")
        })?;
        let endpoint_uv: Vec<_> = indices
            .iter()
            .map(|&i| models[support].loops[wire].coedges[i].pcurve.control_points[0].clone())
            .collect();
        let path = &paths[support];
        if path[0].pcurve.control_points[0] != endpoint_uv[0]
            || path.last().unwrap().pcurve.control_points.last().unwrap() != &endpoint_uv[1]
            || path
                .windows(2)
                .any(|w| w[0].pcurve.control_points.last() != Some(&w[1].pcurve.control_points[0]))
        {
            return Err(Error::new(
                "BREP_FACE_CUT_INPUT",
                &format!(
                    "Open SS UV endpoints must equal the existing owned boundary UVs: support {support}, trace [{:?}, {:?}], owned [{:?}, {:?}]",
                    path[0].pcurve.control_points[0],
                    path.last().unwrap().pcurve.control_points.last().unwrap(),
                    endpoint_uv[0],
                    endpoint_uv[1],
                ),
            ));
        }
    }
    let first = partition_with_budget(
        first,
        first_face,
        endpoints[0],
        &paths[0],
        limits.trace.tolerance_uv,
        limits.cut_cells,
        limits.max_exact,
    )?;
    let second = partition_with_budget(
        second,
        second_face,
        endpoints[1],
        &paths[1],
        limits.trace.tolerance_uv,
        limits.cut_cells,
        limits.max_exact,
    )?;
    Ok(OpenSsPartition {
        first,
        second,
        trace,
    })
}

/// Complete paired imprint of every regular open component, in original branch
/// order. Face and edge indices address the accumulated models. Trace work is
/// shared by all components; binding and cut work are accumulated independently
/// per face. `endpoints[i]` belongs to original branch `i` on both final models.
pub struct OpenSsComponentsPartition {
    pub models: [Model; 2],
    pub faces: [Vec<usize>; 2],
    pub edges: [Vec<usize>; 2],
    pub traces: Vec<ss::QualifiedPhysicalFiberTrace>,
    pub endpoints: Vec<[[usize; 2]; 2]>,
    pub binding_cells: [usize; 2],
    pub binding_inversions: [usize; 2],
    pub cells: [usize; 2],
    pub exact_work: [u64; 2],
    pub trim_pairs: [usize; 2],
}

fn saved_paths(trace: &ss::QualifiedPhysicalFiberTrace) -> [Vec<Segment>; 2] {
    std::array::from_fn(|support| {
        trace
            .segments()
            .iter()
            .map(|s| Segment {
                curve: s.curve().clone(),
                pcurve: s.pcurves()[support].clone(),
            })
            .collect()
    })
}

fn validate_saved_binding(
    model: &Model,
    face: usize,
    endpoints: [usize; 2],
    path: &[Segment],
    support: usize,
) -> Result<()> {
    let (wire, indices) = owned_boundary(model, face, endpoints).ok_or_else(|| {
        invalid("Open SS endpoints need unique occurrences on one original boundary")
    })?;
    let endpoint_uv: Vec<_> = indices
        .iter()
        .map(|&i| model.loops[wire].coedges[i].pcurve.control_points[0].clone())
        .collect();
    if path.is_empty()
        || path[0].pcurve.control_points[0] != endpoint_uv[0]
        || path.last().unwrap().pcurve.control_points.last().unwrap() != &endpoint_uv[1]
        || path.windows(2).any(|w| {
            w[0].pcurve.control_points.last() != Some(&w[1].pcurve.control_points[0])
                || w[0].curve.control_points.last() != Some(&w[1].curve.control_points[0])
        })
        || path[0].curve.control_points[0].as_slice() != model.vertices[endpoints[0]].point
        || path
            .last()
            .unwrap()
            .curve
            .control_points
            .last()
            .unwrap()
            .as_slice()
            != model.vertices[endpoints[1]].point
    {
        return Err(Error::new(
            "BREP_FACE_CUT_INPUT",
            &format!("Open SS saved endpoints do not equal owned support {support} boundaries"),
        ));
    }
    Ok(())
}

fn construct_component_traces(
    first: &Model,
    first_face: usize,
    second: &Model,
    second_face: usize,
    limits: OpenSsLimits,
) -> Result<Vec<ss::QualifiedPhysicalFiberTrace>> {
    first.validate()?;
    second.validate()?;
    if std::ptr::eq(first, second) {
        return Err(invalid(
            "Open SS paired cuts require distinct source models",
        ));
    }
    if !(1..=100000).contains(&limits.cut_cells) || limits.max_exact > 1000000 {
        return Err(invalid("Open SS per-face work limits are out of range"));
    }
    let a = first
        .faces
        .get(first_face)
        .ok_or_else(|| invalid("Missing first open SS face"))?;
    let b = second
        .faces
        .get(second_face)
        .ok_or_else(|| invalid("Missing second open SS face"))?;
    if limits.trace.tolerance_world > first.tolerance_mm.min(second.tolerance_mm) {
        return Err(invalid(
            "Open SS trace tolerance exceeds original model tolerance",
        ));
    }
    let source = ss::FiberSources::new(&a.surface, &b.surface)?;
    let dispatched = ss::construct_qualified_ss_branches(&source, limits.branches)?;
    let ss::QualifiedSurfaceIntersection::RegularOpenArcs(owner) = &dispatched else {
        return Err(unproven(
            "Paired open SS cuts require regular open original arcs",
        ));
    };
    if !owner.covers_entire_surfaces() || owner.branches().is_empty() {
        return Err(unproven(
            "Open SS components need complete regular original branches",
        ));
    }

    let mut trace_allowance = limits.trace;
    let mut traces = Vec::with_capacity(owner.branches().len());
    for branch in 0..owner.branches().len() {
        let trace = ss::construct_physical_fiber_trace(&owner, branch, trace_allowance)?;
        let remaining_cells = trace_allowance
            .max_cells
            .checked_sub(trace.cells())
            .ok_or_else(|| unproven("Open SS shared trace cell allowance exhausted"))?;
        let replay_cells = trace.reverify_with_budget(remaining_cells)?;
        if trace.segments().is_empty() {
            return Err(unproven("Qualified open SS trace has no saved segments"));
        }
        trace_allowance.max_cells = remaining_cells
            .checked_sub(replay_cells)
            .ok_or_else(|| unproven("Open SS shared trace cell allowance exhausted"))?;
        trace_allowance.max_evaluations = trace_allowance
            .max_evaluations
            .checked_sub(trace.evaluations())
            .ok_or_else(|| unproven("Open SS shared evaluation allowance exhausted"))?;
        trace_allowance.max_segments = trace_allowance
            .max_segments
            .checked_sub(trace.segments().len())
            .ok_or_else(|| unproven("Open SS shared segment allowance exhausted"))?;
        traces.push(trace);
    }
    Ok(traces)
}

fn partition_components_with_traces(
    first: &Model,
    first_face: usize,
    second: &Model,
    second_face: usize,
    endpoints: &[[[usize; 2]; 2]],
    traces: Vec<ss::QualifiedPhysicalFiberTrace>,
    limits: OpenSsLimits,
    binding_cells: [usize; 2],
    binding_inversions: [usize; 2],
) -> Result<OpenSsComponentsPartition> {
    if endpoints.len() != traces.len() {
        return Err(unproven(
            "Open SS components need one endpoint pair per complete original branch",
        ));
    }
    let paths: Vec<_> = traces.iter().map(saved_paths).collect();
    let models = [first, second];
    let original_faces = [first_face, second_face];
    for (component, path) in paths.iter().enumerate() {
        for support in 0..2 {
            validate_saved_binding(
                models[support],
                original_faces[support],
                endpoints[component][support],
                &path[support],
                support,
            )?;
        }
    }

    let mut models = [first.clone(), second.clone()];
    let mut faces = [vec![first_face], vec![second_face]];
    let mut edges = [Vec::new(), Vec::new()];
    let mut remaining_cells = [limits.cut_cells; 2];
    let mut remaining_exact = [limits.max_exact; 2];
    let mut remaining_pairs = [10000; 2];
    let mut cells = [0; 2];
    let mut exact_work = [0; 2];
    let mut trim_pairs = [0; 2];
    for (component, path) in paths.iter().enumerate() {
        for support in 0..2 {
            let candidates: Vec<_> = faces[support]
                .iter()
                .copied()
                .enumerate()
                .filter(|&(_, face)| {
                    owned_boundary(&models[support], face, endpoints[component][support]).is_some()
                })
                .collect();
            if candidates.len() != 1 {
                return Err(unproven(
                    "Open SS component does not identify one current face sector",
                ));
            }
            let (position, face) = candidates[0];
            let result = partition_with_shared_limits(
                &models[support],
                face,
                endpoints[component][support],
                &path[support],
                CutLimits {
                    tolerance_uv: limits.trace.tolerance_uv,
                    max_cells: remaining_cells[support],
                    max_exact: remaining_exact[support],
                },
                remaining_pairs[support],
            )?;
            remaining_cells[support] = remaining_cells[support]
                .checked_sub(result.cells)
                .ok_or_else(|| unproven("Open SS face cell allowance exhausted"))?;
            remaining_exact[support] = remaining_exact[support]
                .checked_sub(result.exact_work)
                .ok_or_else(|| unproven("Open SS exact-work allowance exhausted"))?;
            remaining_pairs[support] = remaining_pairs[support]
                .checked_sub(result.trim_pairs)
                .ok_or_else(|| unproven("Open SS trim-pair allowance exhausted"))?;
            cells[support] += result.cells;
            exact_work[support] += result.exact_work;
            trim_pairs[support] += result.trim_pairs;
            faces[support].remove(position);
            faces[support].insert(position, result.faces[0]);
            faces[support].push(result.faces[1]);
            edges[support].extend(result.edges);
            models[support] = result.model;
        }
    }
    Ok(OpenSsComponentsPartition {
        models,
        faces,
        edges,
        traces,
        endpoints: endpoints.to_vec(),
        binding_cells,
        binding_inversions,
        cells,
        exact_work,
        trim_pairs,
    })
}

/// Imprint all complete regular open components without joining their traces.
/// `endpoints[i]` contains the pre-owned endpoint pairs for original branch `i`
/// on the first and second model. Every endpoint is validated before any cut.
/// Each subsequent cut must identify exactly one current descendant face.
/// Failure publishes no partial pair and never invokes a legacy SSI fallback.
pub fn partition_open_ss_components(
    first: &Model,
    first_face: usize,
    second: &Model,
    second_face: usize,
    endpoints: &[[[usize; 2]; 2]],
    limits: OpenSsLimits,
) -> Result<OpenSsComponentsPartition> {
    let traces = construct_component_traces(first, first_face, second, second_face, limits)?;
    partition_components_with_traces(
        first,
        first_face,
        second,
        second_face,
        endpoints,
        traces,
        limits,
        [0; 2],
        [0; 2],
    )
}

mod automatic;
pub use automatic::partition_open_ss_components_automatic;
