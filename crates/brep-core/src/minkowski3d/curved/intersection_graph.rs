//! Complete transverse boundary-intersection graphs for admitted original
//! carriers and trims. No retained-solid or ordinary-Model authoring authority.
use super::*;
use nurbs_core::{ss_intersection as ss, trim_domain::Location};
use std::sync::Arc;
mod bounds;
mod events;
mod pairs;
mod physical;
use physical::Physical;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NurbsGraphLimits {
    /// Existing source-atlas gates retain their own bounded proof budgets.
    pub source_depth: u8,
    /// Shared face-root queries and trim anchor classifications.
    pub max_cells: usize,
    /// Shared SS discovery/replays, physical endpoints and anchor roots.
    pub max_rectangles: usize,
    /// Reported exact queried trim-edge/carrier identity work.
    pub max_exact_work: u64,
    /// Distinct curve/carrier roots, including those outside actual trims.
    pub max_roots: usize,
}
impl Default for NurbsGraphLimits {
    fn default() -> Self {
        Self {
            source_depth: 1,
            max_cells: 1000000,
            max_rectangles: 1000000,
            max_exact_work: 1000000,
            max_roots: 512,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OriginalRootKey {
    pub operand: usize,
    pub edge: usize,
    pub other_face: usize,
    /// Rank in the complete strictly ordered original curve/surface census.
    pub rank: usize,
}
#[derive(Clone, Debug, PartialEq)]
pub enum GraphEndpoint {
    Natural { branch: usize, end: usize },
    Boundary(OriginalRootKey),
}
#[derive(Clone, Debug, PartialEq)]
pub struct OriginalGraphEvent {
    pub key: OriginalRootKey,
    pub branch: usize,
    /// All original fiber charts whose root enclosure can contain this point.
    pub charts: Vec<usize>,
    pub parameter_box: [[f64; 2]; 4],
    pub station: [f64; 2],
    pub inside_other: bool,
    /// Original edge traversal against the oriented other face.
    pub crossing_sign: i8,
    pub edge_parameter: [f64; 2],
    /// Certified original root enclosure, never an authored/snapped vertex.
    pub world_box: [[f64; 2]; 3],
}
#[derive(Clone, Debug, PartialEq)]
pub struct OriginalGraphPiece {
    pub branch: usize,
    pub endpoints: [GraphEndpoint; 2],
    /// A verified root enclosure, not a midpoint containment guess.
    pub anchor: f64,
    pub anchor_box: [[f64; 2]; 4],
    /// Membership in the two FACE trim domains, not inside-solid labels.
    pub inside: [bool; 2],
}
#[derive(Clone, Debug, PartialEq)]
pub struct OriginalFacePairGraph {
    pub faces: [usize; 2],
    pub separating_axis: Option<usize>,
    pub source_order_swapped: bool,
    pub branches: Vec<ss::OriginalPhysicalFiberBranch>,
    pub events: Vec<OriginalGraphEvent>,
    /// ALL pieces, including those excluded by actual original trims.
    pub pieces: Vec<OriginalGraphPiece>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalIntersectionArc {
    pub pair: usize,
    pub piece: usize,
    pub vertices: [usize; 2],
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalIntersectionCycle {
    /// Signed one-based arc IDs in traversal order. This is not a
    /// material-left or outward orientation for Boolean authoring.
    pub arcs: Vec<i32>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NurbsGraphWork {
    pub cells: usize,
    /// Charged discovery/replay/endpoint/anchor rectangles. Failed physical
    /// assembly reserves its complete offered endpoint allowance.
    /// Injectivity/source-atlas gates retain their own fixed native bounds.
    pub rectangles: usize,
    pub exact_work: u64,
    pub queries: usize,
}
#[derive(Clone, Debug, PartialEq)]
pub struct NurbsIntersectionGraphCertificate {
    pub version: u32,
    pub sources: [String; 2],
    pub limits: NurbsGraphLimits,
    pub atlases: [SourceAtlasCertificate; 2],
    pub face_bounds: [Vec<[[f64; 2]; 3]>; 2],
    pub pairs: Vec<OriginalFacePairGraph>,
    pub vertices: Vec<OriginalRootKey>,
    pub arcs: Vec<OriginalIntersectionArc>,
    pub cycles: Vec<OriginalIntersectionCycle>,
    pub work: NurbsGraphWork,
}
pub struct QualifiedNurbsIntersectionGraph {
    certificate: NurbsIntersectionGraphCertificate,
}
impl QualifiedNurbsIntersectionGraph {
    pub fn certificate(&self) -> &NurbsIntersectionGraphCertificate {
        &self.certificate
    }
    pub fn vertex_enclosure(&self, vertex: usize) -> Result<[[f64; 2]; 3]> {
        let key = self
            .certificate
            .vertices
            .get(vertex)
            .ok_or_else(|| fail("missing original intersection vertex"))?;
        self.certificate
            .pairs
            .iter()
            .flat_map(|p| &p.events)
            .find(|e| &e.key == key)
            .map(|e| e.world_box)
            .ok_or_else(|| fail("original vertex lost its source root enclosure"))
    }
    pub fn reverify(&self, a: &Model, b: &Model) -> Result<()> {
        verify_nurbs_intersection_graph(a, b, &self.certificate).map(|_| ())
    }
}
struct Build<'a> {
    models: [&'a Model; 2],
    limits: NurbsGraphLimits,
    work: NurbsGraphWork,
    queries: BTreeMap<(usize, usize, usize), Arc<crate::face_crossings::Report>>,
}
fn axis(a: ss::FiberAxis) -> usize {
    match a {
        ss::FiberAxis::FirstU => 0,
        ss::FiberAxis::FirstV => 1,
        ss::FiberAxis::SecondU => 2,
        ss::FiberAxis::SecondV => 3,
    }
}
fn image(f: &ss::QualifiedFiber) -> [[f64; 2]; 4] {
    let mut b = f.domain();
    let mut j = 0;
    for k in 0..4 {
        if k != axis(f.free_axis()) {
            b[k] = f.report().root_image.unwrap()[j];
            j += 1;
        }
    }
    b
}
impl Build<'_> {
    fn cells(&self) -> Result<usize> {
        self.limits
            .max_cells
            .checked_sub(self.work.cells)
            .filter(|&n| n > 0)
            .map(|n| n.min(100000))
            .ok_or_else(|| fail("intersection graph cell budget exhausted"))
    }
    fn rectangles(&self) -> Result<usize> {
        self.limits
            .max_rectangles
            .checked_sub(self.work.rectangles)
            .filter(|&n| n > 0)
            .map(|n| n.min(100000))
            .ok_or_else(|| fail("intersection graph rectangle budget exhausted"))
    }
}
mod build;
pub use build::{certify_nurbs_intersection_graph, verify_nurbs_intersection_graph};

/// Proposal-only sampling. Every resulting saved curve needs a fresh exact
/// coefficient identity; root enclosures alone never authorize authoring.
pub(super) fn proposal_samples(
    models: [&Model; 2],
    graph: &QualifiedNurbsIntersectionGraph,
    pair: usize,
    requests: &[(usize, f64)],
) -> Result<Vec<[[f64; 2]; 4]>> {
    if requests.len() > 256 {
        return Err(fail("arc proposal sample budget exceeded"));
    }
    if requests.is_empty() {
        return Ok(Vec::new());
    }
    let c = graph.certificate();
    let faces = c
        .pairs
        .get(pair)
        .ok_or_else(|| fail("missing proposal face pair"))?
        .faces;
    let mut build = Build {
        models,
        limits: c.limits,
        work: Default::default(),
        queries: Default::default(),
    };
    let owner = physical::construct(&mut build, faces)?;
    let mut out = Vec::new();
    for &(branch, t) in requests {
        let mut point = owner.native.station(branch, t, build.rectangles()?)?;
        build.work.rectangles += point.report().rectangles;
        for _ in 0..8 {
            let raw = image(&point);
            let free = axis(point.free_axis());
            let old = point.domain();
            if (0..4)
                .filter(|&k| k != free)
                .all(|k| raw[k][1] - raw[k][0] < 1e-11)
            {
                break;
            }
            let proposed = std::array::from_fn(|k| {
                if k == free {
                    raw[k]
                } else {
                    let pad = (old[k][1] - old[k][0]) * 0.125;
                    [
                        (raw[k][0] - pad).max(old[k][0]),
                        (raw[k][1] + pad).min(old[k][1]),
                    ]
                }
            });
            let attempt = point.source().certify(
                proposed,
                point.free_axis(),
                ss::FiberLimits {
                    max_rectangles: build.rectangles()?,
                    preconditioner: None,
                },
            )?;
            build.work.rectangles += attempt.report().rectangles;
            let Some(next) = attempt.into_fiber() else {
                break;
            };
            point = next;
        }
        out.push(owner.canonical(image(&point)));
    }
    Ok(out)
}
