//! Proposed cut representatives bound directly to immutable original events.
use super::*;
use nurbs_core::{curve::Curve, interval_eval, ss_intersection::GraphRootLocation};
mod arithmetic;
mod partition;
use arithmetic::*;
pub use partition::{
    CutPartitionLimits, OpenPathStep, OpenSSFaceLimits, OpenSSNetworkCut, OpenSSNetworkPort,
    OpenSSPairSide, OpenSSPathLimits, OpenSSPathRequest, OpenSSRootGuard, OpenSSSegment,
    PartitionedCutVertex, QualifiedCutPartitions, QualifiedOpenSSFace, QualifiedOpenSSNetwork,
    QualifiedOpenSSPair, QualifiedOpenSSPath, partition_cut_representatives,
    partition_open_ss_network, partition_open_ss_pair, partition_open_ss_path,
    qualify_open_ss_path,
};
#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug)]
pub struct CutRepresentativeLimits {
    /// Input replay plus proxy construction and explicit selection bookkeeping.
    pub operations: usize,
    /// Shared nonempty curve-span visits in certified representative evaluation.
    pub curve_spans: usize,
    pub cuts: usize,
    pub world_tolerance: f64,
    pub uv_tolerance: f64,
}
impl Default for CutRepresentativeLimits {
    fn default() -> Self {
        Self {
            operations: 1000000,
            curve_spans: 100000,
            cuts: 512,
            world_tolerance: 1e-7,
            uv_tolerance: 1e-7,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    Vertex(usize),
    Interior(usize, usize),
}
#[derive(Clone, Debug, PartialEq)]
pub struct CutNode {
    pub position: [f64; 3],
    pub original_vertex: Option<usize>,
    pub original_edge: Option<usize>,
    pub original_clock: Option<f64>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CutRepresentative {
    pub record: usize,
    pub event: usize,
    pub node: usize,
    pub edge_parameter: f64,
    pub pcurve_parameter: f64,
    pub world_tolerance: f64,
    pub source_error_upper: f64,
    pub actual_edge_error_upper: f64,
    pub uv_error_upper: f64,
}
/// Selection only. Does not partition topology or authorize a Solid. Public
/// snapshots cannot construct this owner or replace its immutable source.
pub struct QualifiedCutRepresentatives<'r, 'a> {
    roots: &'r QualifiedFacePairRoots<'a>,
    limits: CutRepresentativeLimits,
    nodes: Vec<CutNode>,
    cuts: Vec<CutRepresentative>,
    operations: usize,
    curve_spans: usize,
}
impl QualifiedCutRepresentatives<'_, '_> {
    pub fn nodes(&self) -> &[CutNode] {
        &self.nodes
    }
    pub fn cuts(&self) -> &[CutRepresentative] {
        &self.cuts
    }
    pub fn operations(&self) -> usize {
        self.operations
    }
    pub fn curve_spans(&self) -> usize {
        self.curve_spans
    }
    pub fn roots(&self) -> &QualifiedFacePairRoots<'_> {
        self.roots
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = qualify_cut_representatives(self.roots, self.limits)?;
        if self.nodes != fresh.nodes
            || self.cuts != fresh.cuts
            || self.operations != fresh.operations
            || self.curve_spans != fresh.curve_spans
        {
            return Err(unproven("Original SS cut representatives changed"));
        }
        Ok(())
    }
}
fn same_frame(roots: &QualifiedFacePairRoots<'_>, a: usize, b: usize) -> bool {
    let surfaces = [
        roots.original.source().first(),
        roots.original.source().second(),
    ];
    let domain = |s: &nurbs_core::surface::Surface| {
        [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ]
    };
    domain(surfaces[a]) == domain(surfaces[b])
}
fn canonical_records(
    roots: &QualifiedFacePairRoots<'_>,
    used: &mut usize,
    max: usize,
) -> Result<Vec<usize>> {
    let mut out = Vec::new();
    for (i, r) in roots.records.iter().enumerate() {
        let mut first = i;
        for (j, s) in roots.records[..i].iter().enumerate() {
            spend(used, max, 1)?;
            if same_frame(roots, r.address.side, s.address.side)
                && r.bindings.census().boundary() == s.bindings.census().boundary()
            {
                first = out[j];
                break;
            }
        }
        out.push(first);
    }
    Ok(out)
}
fn key(r: &QualifiedCoedgeRoots, canonical: usize, event: &CoedgeRootEvent) -> Result<Key> {
    let binding = r
        .bindings
        .bindings()
        .iter()
        .find(|b| b.original_root == event.original_root)
        .ok_or_else(|| unproven("Missing original event binding"))?;
    Ok(match binding.location {
        GraphRootLocation::Vertex {
            original_vertex, ..
        } => Key::Vertex(original_vertex),
        GraphRootLocation::Interior(_) => Key::Interior(canonical, event.original_root),
    })
}
/// Every retained event gets independent actual edge and pcurve parameters.
/// Node identity uses an original vertex or literally identical query/frame/root
/// ordinal; distance is never an equivalence relation. Outside events remain in
/// the input owner and do not create cut requests. Zero cuts do not prove no SS.
pub fn qualify_cut_representatives<'r, 'a>(
    roots: &'r QualifiedFacePairRoots<'a>,
    limits: CutRepresentativeLimits,
) -> Result<QualifiedCutRepresentatives<'r, 'a>> {
    if !(1..=1000000).contains(&limits.operations)
        || !(1..=100000).contains(&limits.curve_spans)
        || !(1..=512).contains(&limits.cuts)
        || !limits.world_tolerance.is_finite()
        || limits.world_tolerance <= 0.
        || !limits.uv_tolerance.is_finite()
        || limits.uv_tolerance <= 0.
    {
        return Err(invalid(
            "Need bounded cut selection work and positive tolerances",
        ));
    }
    let mut operations = 0;
    spend(&mut operations, limits.operations, roots.operations)?;
    roots.reverify()?;
    let canonical = canonical_records(roots, &mut operations, limits.operations)?;
    let mut groups = std::collections::BTreeMap::<Key, Vec<(usize, usize)>>::new();
    let mut count = 0;
    let mut world = limits.world_tolerance;
    for (ri, r) in roots.records.iter().enumerate() {
        for (ei, e) in r.events.iter().enumerate() {
            if e.other_face_location == Location::Outside {
                continue;
            }
            if e.other_face_location != Location::Inside {
                return Err(unproven("Cut event membership remains unresolved"));
            }
            count += 1;
            if count > limits.cuts {
                return Err(unproven("Cut request ceiling exhausted"));
            }
            let m = roots.faces[r.address.side].0;
            world = world.min(m.tolerance_mm).min(
                *m.1.local_tolerances_mm
                    .get(&r.address.edge_identity)
                    .unwrap_or(&m.tolerance_mm),
            );
            groups
                .entry(key(r, canonical[ri], e)?)
                .or_default()
                .push((ri, ei));
        }
    }
    let mut nodes = Vec::new();
    let mut cuts = Vec::new();
    let mut curve_spans = 0;
    if count == 0 {
        return Ok(QualifiedCutRepresentatives {
            roots,
            limits,
            nodes,
            cuts,
            operations,
            curve_spans,
        });
    }
    let available = limits.operations - operations;
    if available == 0 {
        return Err(unproven("No source proxy construction work remains"));
    }
    let proxies = ss::construct_graph_curves(
        roots.original,
        ss::GraphCurveTolerances {
            world,
            uv: limits.uv_tolerance,
        },
        ss::GraphCurveBuildLimits {
            max_operations: available,
        },
    )?;
    spend(&mut operations, limits.operations, proxies.operations())?;
    for (identity, events) in groups {
        let node = match identity {
            Key::Vertex(v) => CutNode {
                position: proxies.vertices()[v].world,
                original_vertex: Some(v),
                original_edge: None,
                original_clock: None,
            },
            Key::Interior(_, _) => {
                let mut edge = None;
                let mut branch = None;
                let mut band = [0f64, 1f64];
                for &(ri, ei) in &events {
                    let r = &roots.records[ri];
                    let event = &r.events[ei];
                    let binding = r
                        .bindings
                        .bindings()
                        .iter()
                        .find(|b| b.original_root == event.original_root)
                        .unwrap();
                    let GraphRootLocation::Interior(c) = &binding.location else {
                        return Err(unproven(
                            "Literal event identity changes original incidence",
                        ));
                    };
                    if edge.is_some_and(|i| i != c.original_edge)
                        || branch.is_some_and(|i| i != c.original_branch)
                    {
                        return Err(unproven("Literal event identity changes original branch"));
                    }
                    edge = Some(c.original_edge);
                    branch = Some(c.original_branch);
                    band = [band[0].max(c.clock[0]), band[1].min(c.clock[1])];
                }
                let t = representative(band, [0., 1.], &mut operations, limits.operations)?;
                let edge = edge.unwrap();
                let bounds = evaluate(
                    &proxies.edges()[edge].world,
                    t,
                    &mut curve_spans,
                    limits.curve_spans,
                )?;
                let position =
                    std::array::from_fn(|a| bounds[a].lo + (bounds[a].hi - bounds[a].lo) * 0.5);
                CutNode {
                    position,
                    original_vertex: None,
                    original_edge: Some(edge),
                    original_clock: Some(t),
                }
            }
        };
        if node
            .position
            .iter()
            .any(|x| !x.is_finite() || x.abs() > 1e6)
        {
            return Err(unproven("Cut point cannot be authored as a model vertex"));
        }
        if nodes.iter().any(|n: &CutNode| n.position == node.position) {
            return Err(unproven(
                "Distinct original cut identities collapse to one binary64 position",
            ));
        }
        let ni = nodes.len();
        for (ri, ei) in events {
            let r = &roots.records[ri];
            let e = &r.events[ei];
            let model = roots.faces[r.address.side].0;
            let edge = &model.edges[r.address.edge].curve;
            let pc = &model.loops[r.address.wire].coedges[r.address.coedge].pcurve;
            let tolerance = limits.world_tolerance.min(model.tolerance_mm).min(
                *model
                    .1
                    .local_tolerances_mm
                    .get(&r.address.edge_identity)
                    .unwrap_or(&model.tolerance_mm),
            );
            let et = representative(
                e.edge_parameter,
                edge.domain(),
                &mut operations,
                limits.operations,
            )?;
            let pt = representative(
                e.pcurve_parameter,
                pc.domain(),
                &mut operations,
                limits.operations,
            )?;
            spend(&mut operations, limits.operations, 32)?;
            let source_error_upper = distance(&node.position, &e.source_world)?;
            let actual = evaluate(edge, et, &mut curve_spans, limits.curve_spans)?;
            let actual_edge_error_upper = distance(
                &node.position,
                &actual.iter().map(|v| [v.lo, v.hi]).collect::<Vec<_>>(),
            )?;
            let query = evaluate(pc, pt, &mut curve_spans, limits.curve_spans)?;
            let root = &r.bindings.census().roots()[e.original_root];
            let uv_error_upper = box_distance(&query, &root.native_uv[r.address.side])?;
            if source_error_upper > tolerance
                || actual_edge_error_upper > tolerance
                || uv_error_upper > limits.uv_tolerance
            {
                return Err(unproven(
                    "Proposed cut fails original SS/edge/UV enclosure tolerance",
                ));
            }
            cuts.push(CutRepresentative {
                record: ri,
                event: ei,
                node: ni,
                edge_parameter: et,
                pcurve_parameter: pt,
                world_tolerance: tolerance,
                source_error_upper,
                actual_edge_error_upper,
                uv_error_upper,
            });
        }
        nodes.push(node);
    }
    Ok(QualifiedCutRepresentatives {
        roots,
        limits,
        nodes,
        cuts,
        operations,
        curve_spans,
    })
}
