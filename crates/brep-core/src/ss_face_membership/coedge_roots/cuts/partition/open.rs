//! Source-bound SS paths from immutable boundary roots and exact branch ports.
use super::*;
mod apply;
mod geometry;
mod pair;
pub use pair::{OpenSSPairSide, QualifiedOpenSSPair, partition_open_ss_pair};
mod route;
pub use apply::{OpenSSFaceLimits, QualifiedOpenSSFace, partition_open_ss_path};
use geometry::*;
use route::*;
#[cfg(test)]
mod tests;
#[derive(Clone, Debug, PartialEq)]
pub struct OpenPathStep {
    pub original_edge: usize,
    pub reversed: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct OpenSSPathRequest {
    pub side: usize,
    pub start_node: usize,
    pub end_node: usize,
    pub steps: Vec<OpenPathStep>,
}
#[derive(Clone, Copy, Debug)]
pub struct OpenSSPathLimits {
    pub operations: usize,
    pub cells: usize,
    pub world_tolerance: f64,
    pub uv_tolerance: f64,
}
impl Default for OpenSSPathLimits {
    fn default() -> Self {
        Self {
            operations: 1000000,
            cells: 1000000,
            world_tolerance: 1e-7,
            uv_tolerance: 1e-7,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct OpenSSSegment {
    pub original_edge: usize,
    /// Increasing representative clocks; direction is independent.
    pub clock: [f64; 2],
    pub reversed: bool,
    pub curve: Curve,
    pub pcurves: [Curve; 2],
}
#[derive(Clone, Debug, PartialEq)]
pub struct OpenSSRootGuard {
    pub node: usize,
    pub original_edge: usize,
    pub clock: [f64; 2],
    pub world_error_upper: f64,
    pub uv_error_upper: [f64; 2],
}
/// The numeric clocks represent the exact root bands; they are not claims of
/// exact floating-point root parameters. Guards cover uncertain endpoint caps.
/// Saved curves are source-error/lift qualified. Actual face trim construction
/// and Boolean material/solid admission remain separate obligations.
pub struct QualifiedOpenSSPath<'i, 'p, 'r, 'a> {
    input: &'i QualifiedCutPartitions<'p, 'r, 'a>,
    request: OpenSSPathRequest,
    limits: OpenSSPathLimits,
    segments: Vec<OpenSSSegment>,
    guards: [OpenSSRootGuard; 2],
    endpoints: [Option<usize>; 2],
    world_tolerance: f64,
    uv_tolerance: f64,
    operations: usize,
    cells: usize,
}
impl QualifiedOpenSSPath<'_, '_, '_, '_> {
    pub fn segments(&self) -> &[OpenSSSegment] {
        &self.segments
    }
    pub fn guards(&self) -> &[OpenSSRootGuard; 2] {
        &self.guards
    }
    pub fn endpoints(&self) -> [Option<usize>; 2] {
        self.endpoints
    }
    pub fn request(&self) -> &OpenSSPathRequest {
        &self.request
    }
    pub fn operations(&self) -> usize {
        self.operations
    }
    pub fn cells(&self) -> usize {
        self.cells
    }
    pub fn world_tolerance(&self) -> f64 {
        self.world_tolerance
    }
    pub fn uv_tolerance(&self) -> f64 {
        self.uv_tolerance
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = qualify_open_ss_path(self.input, self.request.clone(), self.limits)?;
        if self.segments != fresh.segments
            || self.guards != fresh.guards
            || self.endpoints != fresh.endpoints
            || self.world_tolerance != fresh.world_tolerance
            || self.uv_tolerance != fresh.uv_tolerance
            || self.operations != fresh.operations
            || self.cells != fresh.cells
        {
            return Err(unproven("Source-bound open SS path changed"));
        }
        Ok(())
    }
}
#[derive(Clone)]
struct End {
    node: usize,
    clock: f64,
    band: [f64; 2],
    position: [f64; 3],
    anchors: [Option<(usize, Vec<f64>)>; 2],
}
#[derive(Clone)]
struct Part {
    step: OpenPathStep,
    clock: [f64; 2],
    core: [f64; 2],
    coverage: [f64; 2],
}
/// Caller chooses only original edge IDs/directions, never replacement geometry
/// or unchecked clocks. Exact source ports establish continuity. Every audited
/// intermediate boundary event requires an explicit network split; proximity
/// cannot merge it with either endpoint. Guard-free original cores are freshly
/// classified against both immutable original retained faces.
pub fn qualify_open_ss_path<'i, 'p, 'r, 'a>(
    input: &'i QualifiedCutPartitions<'p, 'r, 'a>,
    request: OpenSSPathRequest,
    limits: OpenSSPathLimits,
) -> Result<QualifiedOpenSSPath<'i, 'p, 'r, 'a>> {
    if request.side > 1
        || request.start_node == request.end_node
        || !(1..=256).contains(&request.steps.len())
        || !(1..=1000000).contains(&limits.operations)
        || !(1..=1000000).contains(&limits.cells)
        || !limits.world_tolerance.is_finite()
        || limits.world_tolerance <= 0.
        || !limits.uv_tolerance.is_finite()
        || limits.uv_tolerance <= 0.
    {
        return Err(invalid(
            "Need distinct owned SS nodes, an original route and bounded path work",
        ));
    }
    let mut operations = 0;
    let mut cells = 0;
    spend(&mut operations, limits.operations, input.operations)?;
    spend(&mut cells, limits.cells, input.cells)?;
    input.reverify()?;
    let original = input.source.roots.original;
    let (parts, ends) = route(input, &request, &mut operations, limits.operations)?;
    no_omitted_events(input, &request, &parts, &mut operations, limits.operations)?;
    let mut world = limits
        .world_tolerance
        .min(input.source.limits.world_tolerance);
    for (m, _) in input.source.roots.faces {
        world = world.min(m.tolerance_mm);
    }
    for cut in &input.source.cuts {
        if cut.node == request.start_node || cut.node == request.end_node {
            world = world.min(cut.world_tolerance);
        }
    }
    let uv = limits.uv_tolerance.min(input.source.limits.uv_tolerance);
    if operations == limits.operations {
        return Err(unproven("No original SS proxy work remains"));
    }
    let proxies = ss::construct_graph_curves(
        original,
        ss::GraphCurveTolerances { world, uv },
        ss::GraphCurveBuildLimits {
            max_operations: limits.operations - operations,
        },
    )?;
    spend(&mut operations, limits.operations, proxies.operations())?;
    classify_cores(input, &parts, uv, &mut operations, &mut cells, limits)?;
    let mut segments = Vec::new();
    for (i, p) in parts.iter().enumerate() {
        let source = &proxies.edges()[p.step.original_edge];
        let mut curve = trim(&source.world, p.clock, p.step.reversed)?;
        let mut pc = [
            trim(&source.pcurves[0], p.clock, p.step.reversed)?,
            trim(&source.pcurves[1], p.clock, p.step.reversed)?,
        ];
        for (end, index) in [(0, 0), (1, curve.control_points.len() - 1)] {
            let root = if i == 0 && end == 0 {
                Some(&ends[0])
            } else if i + 1 == parts.len() && end == 1 {
                Some(&ends[1])
            } else {
                None
            };
            if let Some(root) = root {
                curve.control_points[index] = root.position.to_vec();
                for side in 0..2 {
                    if let Some((_, uv)) = &root.anchors[side] {
                        let index = if end == 0 {
                            0
                        } else {
                            pc[side].control_points.len() - 1
                        };
                        pc[side].control_points[index] = uv.clone();
                    }
                }
            }
        }
        prove(
            input,
            source,
            &curve,
            &pc,
            p,
            world,
            uv,
            &mut cells,
            limits.cells,
        )?;
        if let Some(previous) = segments.last() {
            let previous: &OpenSSSegment = previous;
            if previous.curve.control_points.last() != Some(&curve.control_points[0])
                || (0..2).any(|side| {
                    previous.pcurves[side].control_points.last()
                        != Some(&pc[side].control_points[0])
                })
            {
                return Err(unproven(
                    "Original SS port proposals lost literal world/UV joins",
                ));
            }
        }
        segments.push(OpenSSSegment {
            original_edge: p.step.original_edge,
            clock: p.clock,
            reversed: p.step.reversed,
            curve,
            pcurves: pc,
        });
    }
    let guards = [
        guard(
            input,
            &ends[0],
            &parts[0],
            &segments[0],
            0,
            world,
            uv,
            &mut operations,
            limits.operations,
        )?,
        guard(
            input,
            &ends[1],
            parts.last().unwrap(),
            segments.last().unwrap(),
            1,
            world,
            uv,
            &mut operations,
            limits.operations,
        )?,
    ];
    let endpoints = [
        ends[0].anchors[request.side].as_ref().map(|a| a.0),
        ends[1].anchors[request.side].as_ref().map(|a| a.0),
    ];
    Ok(QualifiedOpenSSPath {
        input,
        request,
        limits,
        segments,
        guards,
        endpoints,
        world_tolerance: world,
        uv_tolerance: uv,
        operations,
        cells,
    })
}
