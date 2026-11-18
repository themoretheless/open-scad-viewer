//! Source-owned endpoint binding for complete regular open SS components.
use super::*;
use crate::edge_partition;

#[derive(Clone, Copy)]
struct InteriorBinding {
    edge: usize,
    wire: usize,
    coedge: usize,
    edge_fraction: f64,
}

#[derive(Clone, Copy, Default)]
struct BindingWork {
    cells: usize,
    inversions: usize,
}

fn same_uv(point: &[f64], goal: [f64; 2]) -> bool {
    point == goal
}

fn trace_endpoint(
    trace: &ss::QualifiedPhysicalFiberTrace,
    support: usize,
    last: bool,
) -> Result<([f64; 3], [f64; 2])> {
    let segment = if last {
        trace.segments().last()
    } else {
        trace.segments().first()
    }
    .ok_or_else(|| unproven("Cannot bind an empty open SS trace"))?;
    let world = if last {
        segment.curve().control_points.last()
    } else {
        segment.curve().control_points.first()
    }
    .ok_or_else(|| unproven("Open SS trace endpoint has no world pole"))?
    .as_slice()
    .try_into()
    .map_err(|_| unproven("Open SS trace world endpoint must be three-dimensional"))?;
    let uv = if last {
        segment.pcurves()[support].control_points.last()
    } else {
        segment.pcurves()[support].control_points.first()
    }
    .ok_or_else(|| unproven("Open SS trace endpoint has no UV pole"))?
    .as_slice()
    .try_into()
    .map_err(|_| unproven("Open SS trace UV endpoint must be two-dimensional"))?;
    Ok((world, uv))
}

fn linear_nonrational_pcurve(c: &Coedge) -> bool {
    let pcurve = &c.pcurve;
    pcurve.degree == 1
        && !pcurve.periodic
        && pcurve.control_points.len() == 2
        && pcurve.weights.iter().all(|&w| w == 1.)
        && pcurve.control_points.iter().all(|p| p.len() == 2)
}

/// Return the normalized pcurve/edge position only for an exact straight,
/// nonrational linear pcurve whose interior comes within the declared UV proof
/// tolerance of the saved trace endpoint. No nearest-point projection is used.
fn linear_interior_fraction(c: &Coedge, goal: [f64; 2], tolerance_uv: f64) -> Option<f64> {
    if !linear_nonrational_pcurve(c) {
        return None;
    }
    let pcurve = &c.pcurve;
    let p = &pcurve.control_points[0];
    let q = &pcurve.control_points[1];
    let delta = [q[0] - p[0], q[1] - p[1]];
    let axis = usize::from(delta[1].abs() > delta[0].abs());
    if delta[axis] == 0. {
        return None;
    }
    let fraction = (goal[axis] - p[axis]) / delta[axis];
    if !fraction.is_finite() || !(0. < fraction && fraction < 1.) {
        return None;
    }
    for axis in 0..2 {
        let predicted = p[axis] + fraction * delta[axis];
        if !predicted.is_finite() || (predicted - goal[axis]).abs() > tolerance_uv {
            return None;
        }
    }
    Some(fraction)
}

fn interior_fraction(
    c: &Coedge,
    goal: [f64; 2],
    tolerance_uv: f64,
    remaining_inversions: usize,
    literal_ends: [bool; 2],
) -> Result<(Option<f64>, usize)> {
    if linear_nonrational_pcurve(c) {
        Ok((linear_interior_fraction(c, goal, tolerance_uv), 0))
    } else {
        nonlinear_interior_fraction(c, goal, tolerance_uv, remaining_inversions, literal_ends)
    }
}

/// Invert a nonlinear pcurve only when its globally unique closest point is an
/// interior candidate within the trace UV tolerance. The complete stationary
/// census excludes every farther candidate. A nonunique or unresolved nearby
/// projection is a refusal, never permission to select a convenient root.
fn nonlinear_interior_fraction(
    c: &Coedge,
    goal: [f64; 2],
    tolerance_uv: f64,
    remaining_inversions: usize,
    literal_ends: [bool; 2],
) -> Result<(Option<f64>, usize)> {
    let pcurve = &c.pcurve;
    if pcurve.periodic
        || pcurve.control_points.is_empty()
        || pcurve.control_points.iter().any(|p| p.len() != 2)
    {
        return Ok((None, 0));
    }
    let mut lower_squared = 0.;
    for axis in 0..2 {
        let min = pcurve
            .control_points
            .iter()
            .map(|p| p[axis])
            .fold(f64::INFINITY, f64::min);
        let max = pcurve
            .control_points
            .iter()
            .map(|p| p[axis])
            .fold(f64::NEG_INFINITY, f64::max);
        let delta = if goal[axis] < min {
            min - goal[axis]
        } else if goal[axis] > max {
            goal[axis] - max
        } else {
            0.
        };
        lower_squared += delta * delta;
    }
    if lower_squared > tolerance_uv * tolerance_uv {
        return Ok((None, 0));
    }
    if remaining_inversions == 0 {
        return Err(unproven("Open SS pcurve inversion allowance exhausted"));
    }
    let projection = nurbs_core::foundation::project_curve_report(pcurve, &goal, None)?;
    if projection.global_distance_upper > tolerance_uv {
        return Ok((None, 1));
    }
    if projection.status != nurbs_core::foundation::ProjectionStatus::Unique {
        return Err(unproven(
            "Open SS nonlinear pcurve projection is nonunique or unresolved",
        ));
    }
    let winner = projection
        .winner_index
        .and_then(|i| projection.candidates.get(i))
        .ok_or_else(|| unproven("Open SS unique pcurve projection has no candidate"))?;
    if winner.distance_upper > tolerance_uv {
        return Ok((None, 1));
    }
    let [a, b] = pcurve.domain();
    let [lo, hi] = winner.parameter_interval;
    let unique_literal_endpoint = lo <= hi
        && ((lo <= a && hi < b && literal_ends[0]) || (hi >= b && a < lo && literal_ends[1]));
    if unique_literal_endpoint {
        return Ok((None, 1));
    }
    if !(a < lo && lo <= hi && hi < b) {
        return Err(unproven(
            "Open SS nonlinear projection is too close to a boundary endpoint",
        ));
    }
    let parameter = lo + (hi - lo) * 0.5;
    let fraction = (parameter - a) / (b - a);
    if !fraction.is_finite() || !(0. < fraction && fraction < 1.) {
        return Err(unproven("Open SS nonlinear inversion left its interior"));
    }
    Ok((Some(fraction), 1))
}

fn face_wires(model: &Model, face: usize) -> Result<Vec<usize>> {
    let face = model
        .faces
        .get(face)
        .ok_or_else(|| invalid("Missing automatic open SS binding face"))?;
    Ok(std::iter::once(face.outer)
        .chain(face.holes.iter().copied())
        .collect())
}

fn has_partial_trim(source: &Model, c: &Coedge) -> bool {
    c.trim().is_some_and(|trim| {
        trim.curve_parameter != source.edges[c.edge].curve.domain()
            || trim.pcurve_parameter != c.pcurve.domain()
    })
}

fn bind_endpoint(
    source: &Model,
    face: usize,
    goal_world: [f64; 3],
    goal_uv: [f64; 2],
    tolerance_uv: f64,
    remaining: BindingWork,
) -> Result<(Model, usize, BindingWork)> {
    let mut existing = std::collections::BTreeSet::new();
    let mut interior = Vec::new();
    let mut inversions = 0usize;
    for wire in face_wires(source, face)? {
        for (coedge, c) in source.loops[wire].coedges.iter().enumerate() {
            let edge = &source.edges[c.edge];
            let mut literal_ends = [false; 2];
            for end in 0..2 {
                let uv = if end == 0 {
                    c.pcurve.control_points.first()
                } else {
                    c.pcurve.control_points.last()
                }
                .ok_or_else(|| unproven("Open SS boundary pcurve has no endpoint"))?;
                let vertex = edge.vertices[if end == 0 {
                    usize::from(c.reversed)
                } else {
                    usize::from(!c.reversed)
                }];
                if same_uv(uv, goal_uv) && source.vertices[vertex].point.as_slice() == goal_world {
                    existing.insert(vertex);
                    literal_ends[end] = true;
                }
            }
            let (fraction, spent) = interior_fraction(
                c,
                goal_uv,
                tolerance_uv,
                remaining
                    .inversions
                    .checked_sub(inversions)
                    .ok_or_else(|| {
                        unproven("Open SS pcurve inversion allowance exhausted during search")
                    })?,
                literal_ends,
            )?;
            inversions = inversions
                .checked_add(spent)
                .filter(|&total| total <= remaining.inversions)
                .ok_or_else(|| unproven("Open SS pcurve inversion allowance overflow"))?;
            if let Some(fraction) = fraction {
                interior.push(InteriorBinding {
                    edge: c.edge,
                    wire,
                    coedge,
                    edge_fraction: if c.reversed { 1. - fraction } else { fraction },
                });
            }
        }
    }
    if existing.len() == 1 && interior.is_empty() {
        return Ok((
            source.clone(),
            existing.into_iter().next().unwrap(),
            BindingWork {
                cells: 0,
                inversions,
            },
        ));
    }
    if !existing.is_empty() || interior.len() != 1 {
        return Err(unproven(
            "Open SS endpoint needs one literal vertex or one unique boundary interior",
        ));
    }
    if remaining.cells == 0 {
        return Err(unproven("Open SS binding cell allowance exhausted"));
    }
    let binding = interior[0];
    let [a, b] = source.edges[binding.edge].curve.domain();
    let parameter = a + (b - a) * binding.edge_fraction;
    let mut uses = Vec::new();
    for (other_face, f) in source.faces.iter().enumerate() {
        for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            for (coedge, c) in source.loops[wire].coedges.iter().enumerate() {
                if c.edge == binding.edge {
                    uses.push((other_face, wire, coedge, c));
                }
            }
        }
    }
    if uses.iter().any(|(_, _, _, c)| has_partial_trim(source, c)) {
        return Err(unproven(
            "Automatic open SS binding does not yet support partial incident coedge windows",
        ));
    }
    let mut cuts = Vec::with_capacity(uses.len());
    let mut uv_vertices = Vec::with_capacity(uses.len());
    for &(_, wire, coedge, c) in &uses {
        let fraction = if c.reversed {
            1. - binding.edge_fraction
        } else {
            binding.edge_fraction
        };
        let [p0, p1] = c.pcurve.domain();
        let parameter = p0 + (p1 - p0) * fraction;
        cuts.push(edge_partition::PcurveCut {
            wire,
            coedge,
            parameter,
        });
        let uv = if wire == binding.wire && coedge == binding.coedge {
            goal_uv.to_vec()
        } else {
            c.pcurve.evaluate(parameter)?.point
        };
        uv_vertices.push(uv.try_into().map_err(|_| {
            unproven("Every incident open SS binding pcurve must be two-dimensional")
        })?);
    }
    let split = edge_partition::partition_to_vertex_and_uv(
        source,
        binding.edge,
        parameter,
        &cuts,
        goal_world,
        &uv_vertices,
        remaining.cells,
    )?;
    if split.cells > remaining.cells {
        return Err(unproven("Open SS binding exceeded its cell allowance"));
    }
    if split.model.vertices[split.vertex].point.as_slice() != goal_world {
        return Err(unproven(
            "Open SS binding changed its literal world endpoint",
        ));
    }
    Ok((
        split.model,
        split.vertex,
        BindingWork {
            cells: split.cells,
            inversions,
        },
    ))
}

fn bind_components(
    source: &Model,
    face: usize,
    support: usize,
    traces: &[ss::QualifiedPhysicalFiberTrace],
    limits: OpenSsLimits,
) -> Result<(Model, Vec<[usize; 2]>, BindingWork)> {
    if !(1..=1000000).contains(&limits.binding_cells)
        || !(1..=100000).contains(&limits.binding_inversions)
    {
        return Err(invalid(
            "Open SS automatic binding work limits are out of range",
        ));
    }
    for wire in face_wires(source, face)? {
        if source.loops[wire]
            .coedges
            .iter()
            .any(|c| has_partial_trim(source, c))
        {
            return Err(unproven(
                "Automatic open SS binding does not yet support partial coedge windows",
            ));
        }
    }
    let mut model = source.clone();
    let mut endpoints = Vec::with_capacity(traces.len());
    let mut work = BindingWork::default();
    for trace in traces {
        let mut pair = [0; 2];
        for end in 0..2 {
            let (world, uv) = trace_endpoint(trace, support, end == 1)?;
            let remaining = BindingWork {
                cells: limits
                    .binding_cells
                    .checked_sub(work.cells)
                    .ok_or_else(|| {
                        unproven("Open SS binding cell allowance exhausted before endpoint search")
                    })?,
                inversions: limits
                    .binding_inversions
                    .checked_sub(work.inversions)
                    .ok_or_else(|| {
                        unproven("Open SS inversion allowance exhausted before endpoint search")
                    })?,
            };
            let (next, vertex, spent) = bind_endpoint(
                &model,
                face,
                world,
                uv,
                limits.trace.tolerance_uv,
                remaining,
            )?;
            work.cells = work
                .cells
                .checked_add(spent.cells)
                .filter(|&total| total <= limits.binding_cells)
                .ok_or_else(|| unproven("Open SS binding cell allowance overflow"))?;
            work.inversions = work
                .inversions
                .checked_add(spent.inversions)
                .filter(|&total| total <= limits.binding_inversions)
                .ok_or_else(|| unproven("Open SS pcurve inversion allowance overflow"))?;
            model = next;
            pair[end] = vertex;
        }
        endpoints.push(pair);
    }
    Ok((model, endpoints, work))
}

/// Automatically bind every trace endpoint to the original trimmed boundary,
/// then imprint all components through the same audited path as explicit input.
/// Existing endpoints must match literally. New endpoints require either one
/// nonrational linear boundary interior or one globally unique, certified
/// nonlinear pcurve inversion within UV tolerance. All incident uses are split
/// atomically and saved trace world/UV poles are preserved on the target use.
/// Ambiguity and exhausted budgets refuse the whole pair without publishing a
/// partially bound model, choosing a nearest root or refitting a trace.
pub fn partition_open_ss_components_automatic(
    first: &Model,
    first_face: usize,
    second: &Model,
    second_face: usize,
    limits: OpenSsLimits,
) -> Result<OpenSsComponentsPartition> {
    if !(1..=1000000).contains(&limits.binding_cells)
        || !(1..=100000).contains(&limits.binding_inversions)
    {
        return Err(invalid(
            "Open SS automatic binding work limits are out of range",
        ));
    }
    let traces = super::construct_component_traces(first, first_face, second, second_face, limits)?;
    let (first, ends_first, first_work) = bind_components(first, first_face, 0, &traces, limits)?;
    let (second, ends_second, second_work) =
        bind_components(second, second_face, 1, &traces, limits)?;
    let endpoints: Vec<_> = ends_first
        .into_iter()
        .zip(ends_second)
        .map(|(a, b)| [a, b])
        .collect();
    super::partition_components_with_traces(
        &first,
        first_face,
        &second,
        second_face,
        &endpoints,
        traces,
        limits,
        [first_work.cells, second_work.cells],
        [first_work.inversions, second_work.inversions],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonlinear_inversion_rejects_repeated_points_and_exhaustion() {
        let curve = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![0.5, 1.], vec![0., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let coedge = Coedge::new(0, false, curve);
        let repeated = [0.1875, 0.375];
        assert!(nonlinear_interior_fraction(&coedge, repeated, 1e-7, 1, [false; 2]).is_err());
        assert!(nonlinear_interior_fraction(&coedge, repeated, 1e-7, 0, [false; 2]).is_err());
        assert_eq!(
            nonlinear_interior_fraction(&coedge, [2., 2.], 1e-7, 0, [false; 2]).unwrap(),
            (None, 0)
        );
    }

    #[test]
    fn linear_miss_does_not_consume_nonlinear_inversions() {
        let curve = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap();
        let coedge = Coedge::new(0, false, curve);
        assert_eq!(
            interior_fraction(&coedge, [0.5, 0.9], 0.1, 0, [false; 2]).unwrap(),
            (None, 0)
        );
    }

    #[test]
    fn nonlinear_literal_endpoint_still_rejects_repeated_interior() {
        let repeated = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![-1., 0.], vec![1., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let coedge = Coedge::new(0, false, repeated);
        assert!(interior_fraction(&coedge, [0., 0.], 1e-7, 1, [true, false]).is_err());

        let unique = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![0.5, 0.], vec![1., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let coedge = Coedge::new(0, false, unique);
        assert_eq!(
            interior_fraction(&coedge, [0., 0.], 1e-7, 1, [true, false]).unwrap(),
            (None, 1)
        );
        assert!(interior_fraction(&coedge, [0., 0.], 1e-7, 0, [true, false]).is_err());
    }
}
