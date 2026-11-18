//! Shared adaptive edge partition. Face clipping and vertex links are separate.
use super::SharedEdge;
use crate::source_seam_tangency::Carrier;
use nurbs_core::{
    Error, Result,
    interval_eval::{self, Interval as I},
};
use std::collections::VecDeque;
pub struct BoundaryMeshLimits {
    pub max_deviation_mm: f64,
    pub cells: usize,
    pub spans: usize,
    pub max_depth: usize,
}
pub struct BoundaryMeshVertex {
    pub canonical: [f64; 2],
    pub parameters: [[f64; 2]; 2],
    pub uv: [[[f64; 2]; 2]; 2],
    pub world: [[f64; 2]; 3],
    pub point: [f64; 3],
}
pub struct BoundaryMeshSegment {
    pub vertices: [usize; 2],
    pub deviation_upper_mm: f64,
}
pub struct BoundaryMeshPending {
    pub vertices: Option<[usize; 2]>,
    pub reason: &'static str,
}
pub struct BoundaryMeshReport {
    pub vertices: Vec<BoundaryMeshVertex>,
    pub segments: Vec<BoundaryMeshSegment>,
    pub pending: Vec<BoundaryMeshPending>,
    pub cells: usize,
    pub spans: usize,
}
fn vertex(
    canonical: [f64; 2],
    parameters: [[f64; 2]; 2],
    uv: [[[f64; 2]; 2]; 2],
    world: [[f64; 2]; 3],
) -> BoundaryMeshVertex {
    BoundaryMeshVertex {
        canonical,
        parameters,
        uv,
        world,
        point: std::array::from_fn(|k| world[k][0] * 0.5 + world[k][1] * 0.5),
    }
}
fn diameter(world: [[f64; 2]; 3]) -> Result<f64> {
    let mut total = I::point(0.);
    for bounds in world {
        total = total.add(I::point(bounds[1]).sub(I::point(bounds[0]))?)?;
    }
    Ok(total.hi)
}
impl SharedEdge {
    pub fn prepare_boundary_mesh(&self,limits:BoundaryMeshLimits)->Result<BoundaryMeshReport> {
        prepare_boundary_mesh(self,limits)
    }
}

    /// One vertex list and partition for both incident uses. Root endpoint caps
    /// retain interval-valued identities. Error bounds cover curve-to-chord and
    /// chord-to-retained-curve distance; incomplete leaves remain explicit.
pub(crate) fn prepare_boundary_mesh<E:Carrier>(edge:&E, limits: BoundaryMeshLimits) -> Result<BoundaryMeshReport> {
        if !limits.max_deviation_mm.is_finite()
            || limits.max_deviation_mm <= 0.
            || limits.cells > 100000
            || limits.spans > 100000
            || limits.max_depth > 32
        {
            return Err(Error::new(
                "BREP_BOUNDARY_MESH",
                "Choose a positive deviation and bounded work",
            ));
        }
        let mut report = BoundaryMeshReport {
            vertices: Vec::new(),
            segments: Vec::new(),
            pending: Vec::new(),
            cells: 0,
            spans: 0,
        };
        for end in 0..2 {
            let queried = super::endpoint::sample_endpoint(edge,end, limits.spans - report.spans)?;
            report.spans += queried.spans;
            let Some(endpoint) = queried.endpoint else {
                report.pending.push(BoundaryMeshPending {
                    vertices: None,
                    reason: queried.reason,
                });
                return Ok(report);
            };
            report.vertices.push(vertex(
                endpoint.canonical_bounds(),
                endpoint.parameter_bounds(),
                endpoint.uv_bounds(),
                endpoint.world_bounds(),
            ));
        }
        let order = if report.vertices[0].canonical[1] < report.vertices[1].canonical[0] {
            [0, 1]
        } else if report.vertices[1].canonical[1] < report.vertices[0].canonical[0] {
            [1, 0]
        } else {
            report.pending.push(BoundaryMeshPending {
                vertices: Some([0, 1]),
                reason: "boundary-mesh-end-order-unproven",
            });
            return Ok(report);
        };
        let mut queue = VecDeque::from([(order, 0)]);
        while let Some((ends, depth)) = queue.pop_front() {
            if report.cells == limits.cells || report.spans == limits.spans {
                report.pending.push(BoundaryMeshPending {
                    vertices: Some(ends),
                    reason: "boundary-mesh-work-limit",
                });
                continue;
            }
            report.cells += 1;
            let a = &report.vertices[ends[0]];
            let b = &report.vertices[ends[1]];
            let d = edge.world().domain();
            let mapped = I::point(d[0])
                .add(
                    I::new(a.canonical[0], b.canonical[1])?
                        .mul(I::point(d[1]).sub(I::point(d[0]))?)?,
                )?
                .intersect(d[0], d[1])?;
            let (error, spent) = interval_eval::curve_affine_error_upper(
                edge.world(),
                [mapped.lo, mapped.hi],
                [a.point, b.point],
                limits.spans - report.spans,
            )?;
            report.spans += spent;
            if let Some(error) = error {
                // On the expanded parameter domain every chord point has a matching curve
                // point. If that point lies in an uncertain endpoint cap, clamp to the true
                // retained endpoint and add the cap image diameter. Forward distance uses
                // the residual alone; this sum bounds both directions conservatively.
                let bound = I::point(error)
                    .add(I::point(diameter(a.world)?.max(diameter(b.world)?)))?
                    .hi;
                if bound <= limits.max_deviation_mm {
                    report.segments.push(BoundaryMeshSegment {
                        vertices: ends,
                        deviation_upper_mm: bound,
                    });
                    continue;
                }
            }
            if depth == limits.max_depth {
                report.pending.push(BoundaryMeshPending {
                    vertices: Some(ends),
                    reason: "boundary-mesh-depth-limit",
                });
                continue;
            }
            let canonical = a.canonical[1] * 0.5 + b.canonical[0] * 0.5;
            if canonical <= a.canonical[1] || canonical >= b.canonical[0] {
                report.pending.push(BoundaryMeshPending {
                    vertices: Some(ends),
                    reason: "boundary-mesh-parameter-resolution",
                });
                continue;
            }
            let queried = super::sample::sample_interior(edge,canonical, limits.spans - report.spans)?;
            report.spans += queried.spans;
            let Some(sample) = queried.sample else {
                report.pending.push(BoundaryMeshPending {
                    vertices: Some(ends),
                    reason: queried.reason,
                });
                continue;
            };
            let middle = report.vertices.len();
            report.vertices.push(vertex(
                [canonical, canonical],
                sample.parameter_bounds(),
                sample.uv_bounds(),
                sample.world_bounds(),
            ));
            queue.push_back(([ends[0], middle], depth + 1));
            queue.push_back(([middle, ends[1]], depth + 1));
        }
        Ok(report)
    }
