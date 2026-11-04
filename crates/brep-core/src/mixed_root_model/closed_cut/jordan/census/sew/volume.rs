//! Outward original-source flux on complete root-trimmed domains. Unknown
//! cells keep a full pointwise [0,1] mask; failed children never erase parents.
use super::*;
use nurbs_core::interval_eval::Interval as I;
use std::collections::BinaryHeap;
mod domain;
#[derive(Clone, Copy)]
pub struct RootBoundaryVolumeLimits {
    pub axis: usize,
    pub origin: f64,
    pub absolute_error: f64,
    pub cells: usize,
    pub spans: usize,
    pub arc_subdivisions: usize,
    pub band_queries: usize,
    pub winding_queries: usize,
    pub winding: ss::OriginalSsWindingLimits,
    pub geometry: ss::OriginalSsCurveLimits,
}
impl Default for RootBoundaryVolumeLimits {
    fn default() -> Self {
        Self {
            axis: 2,
            origin: 0.,
            absolute_error: 1e-4,
            cells: 16384,
            spans: 32768,
            arc_subdivisions: 8,
            band_queries: 16384,
            winding_queries: 1024,
            winding: Default::default(),
            geometry: Default::default(),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct RootBoundaryVolumeReport {
    pub signed_bounds: Option<[f64; 2]>,
    pub accuracy_proven: bool,
    pub cells: usize,
    pub spans: usize,
    pub band_queries: usize,
    pub winding_queries: usize,
    pub seeds: usize,
    pub propagations: usize,
    pub reason: &'static str,
}
struct Node {
    face: usize,
    rectangle: [[f64; 2]; 2],
    bound: I,
    parent: Option<usize>,
    children: Option<[usize; 2]>,
}
#[derive(Clone, Copy)]
struct Priority {
    width: f64,
    index: usize,
}
impl PartialEq for Priority {
    fn eq(&self, b: &Self) -> bool {
        self.width.total_cmp(&b.width).is_eq() && self.index == b.index
    }
}
impl Eq for Priority {}
impl PartialOrd for Priority {
    fn partial_cmp(&self, b: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(b))
    }
}
impl Ord for Priority {
    fn cmp(&self, b: &Self) -> std::cmp::Ordering {
        self.width
            .total_cmp(&b.width)
            .then(self.index.cmp(&b.index))
    }
}
fn evaluate(
    boundary: &QualifiedPairedRootBoundary,
    domains: &mut domain::Domains,
    face: usize,
    rectangle: [[f64; 2]; 2],
    limits: RootBoundaryVolumeLimits,
    out: &mut RootBoundaryVolumeReport,
) -> Result<Option<I>> {
    if out.cells == limits.cells || out.spans == limits.spans {
        return Ok(None);
    }
    out.cells += 1;
    let surface = &boundary.model.topology().faces[face].surface;
    let flux = nurbs_core::surface_flux::bound(
        surface,
        rectangle,
        limits.axis,
        limits.origin,
        (limits.spans - out.spans).min(100000),
    )?;
    out.spans += flux.spans;
    let Some(flux) = flux.bound else {
        return Ok(None);
    };
    let area = I::point(rectangle[0][1])
        .sub(I::point(rectangle[0][0]))?
        .mul(I::point(rectangle[1][1]).sub(I::point(rectangle[1][0]))?)?;
    let mut bound = I::new(flux.density[0].min(0.), flux.density[1].max(0.))?.mul(area)?;
    match domains.classify(face, rectangle, limits, out)? {
        Some(true) => bound = I::new(flux.integral[0], flux.integral[1])?,
        Some(false) => bound = I::point(0.),
        None => {}
    }
    if boundary.model.topology().shells[0].faces[face].reversed {
        bound = I::new(-bound.hi, -bound.lo)?;
    }
    Ok(Some(bound))
}
impl QualifiedPairedRootBoundary {
    pub fn volume_bounds(
        &self,
        limits: RootBoundaryVolumeLimits,
    ) -> Result<RootBoundaryVolumeReport> {
        if limits.axis >= 3
            || !limits.origin.is_finite()
            || !limits.absolute_error.is_finite()
            || limits.absolute_error <= 0.
            || !(2..=1000000).contains(&limits.cells)
            || !(2..=1000000).contains(&limits.spans)
            || !(1..=64).contains(&limits.arc_subdivisions)
            || !limits.arc_subdivisions.is_power_of_two()
            || limits.band_queries > 1000000
            || limits.winding_queries > 65536
        {
            return Err(unproved("Invalid root-boundary volume work or accuracy"));
        }
        let mut out = RootBoundaryVolumeReport {
            signed_bounds: None,
            accuracy_proven: false,
            cells: 0,
            spans: 0,
            band_queries: 0,
            winding_queries: 0,
            seeds: 0,
            propagations: 0,
            reason: "root-band-work-unproved",
        };
        let Some(mut domains) = domain::Domains::new(self, limits, &mut out)? else {
            return Ok(out);
        };
        let mut nodes = Vec::new();
        let mut heap = BinaryHeap::new();
        for face in 0..2 {
            let rectangle = domains.roots[face];
            let Some(bound) = evaluate(self, &mut domains, face, rectangle, limits, &mut out)?
            else {
                out.reason = "initial-flux-work-unproved";
                return Ok(out);
            };
            let index = nodes.len();
            nodes.push(Node {
                face,
                rectangle,
                bound,
                parent: None,
                children: None,
            });
            heap.push(Priority {
                width: bound.hi - bound.lo,
                index,
            });
        }
        loop {
            let total = nodes[0].bound.add(nodes[1].bound)?;
            out.signed_bounds = Some([total.lo, total.hi]);
            if (total.hi - total.lo).next_up() <= limits.absolute_error {
                out.accuracy_proven = true;
                out.reason = "complete-root-domain-flux";
                break;
            }
            let Some(priority) = heap.pop() else {
                out.reason = "root-domain-refinement-unproved";
                break;
            };
            let node = &nodes[priority.index];
            let face = node.face;
            let rectangle = node.rectangle;
            let axis = if rectangle[0][1] - rectangle[0][0] >= rectangle[1][1] - rectangle[1][0] {
                0
            } else {
                1
            };
            let mid = rectangle[axis][0] * 0.5 + rectangle[axis][1] * 0.5;
            if !(rectangle[axis][0] < mid && mid < rectangle[axis][1]) {
                out.reason = "root-flux-subdivision-unrepresentable";
                break;
            }
            let mut a = rectangle;
            let mut b = rectangle;
            a[axis][1] = mid;
            b[axis][0] = mid;
            let Some(left) = evaluate(self, &mut domains, face, a, limits, &mut out)? else {
                out.reason = "root-flux-work-limit";
                break;
            };
            let Some(right) = evaluate(self, &mut domains, face, b, limits, &mut out)? else {
                out.reason = "root-flux-work-limit";
                break;
            };
            let children = [nodes.len(), nodes.len() + 1];
            for (rectangle, bound, index) in [(a, left, children[0]), (b, right, children[1])] {
                nodes.push(Node {
                    face,
                    rectangle,
                    bound,
                    parent: Some(priority.index),
                    children: None,
                });
                heap.push(Priority {
                    width: bound.hi - bound.lo,
                    index,
                });
            }
            nodes[priority.index].children = Some(children);
            let mut current = Some(priority.index);
            while let Some(i) = current {
                let [a, b] = nodes[i].children.unwrap();
                nodes[i].bound = nodes[a].bound.add(nodes[b].bound)?;
                current = nodes[i].parent;
            }
        }
        debug_assert_eq!(out.propagations, domains.propagation_links());
        Ok(out)
    }
}

mod point;
pub use point::{RootSolidPointLimits,RootSolidPointReport,RootSolidRayReport};

mod material_box;
pub use material_box::{RootMaterialBoxLimits,RootMaterialBoxWork,QualifiedRootMaterialBox};

mod source_box;
pub use source_box::{RootSurfaceMaterialLimits,QualifiedRootSurfaceMaterialBox};

mod cavity;
pub use cavity::{RootCavityLimits,RootCavityCoverageWork,QualifiedRootCavitySolid};

mod relation;
pub use relation::{RootRelationKind,RootRelationLimits,RootRelationWork,QualifiedRootRelation};

mod shell_forest;
pub use shell_forest::QualifiedRootShellForest;

mod root_boolean;
pub use root_boolean::QualifiedRootBoolean;
