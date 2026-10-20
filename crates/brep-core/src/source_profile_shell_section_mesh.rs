//! Conservative partition of an admitted profile section in its actual charts.
use super::*;
use crate::source_shared_edge::{
    BoundaryMeshLimits, BoundaryMeshPending, BoundaryMeshReport, BoundaryMeshSegment,
    BoundaryMeshVertex,
};
use nurbs_core::interval_eval::Interval as I;
use std::collections::VecDeque;
pub(super) fn prepare(
    shell: &ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_>,
    end: usize,
    limits: BoundaryMeshLimits,
) -> Result<BoundaryMeshReport> {
    let root = shell.assembly.patch.rails.face.boundary.rooted[0];
    let profile = root.restricted.proposal.profile(root.face).unwrap();
    let range = profile.section(root.axis, root.parameters[end])?.range();
    let fragment = shell.assembly.patch.retained[end].last().unwrap();
    let descending = (fragment.curve().control_points[1][1 - root.axis]
        < fragment.curve().control_points[0][1 - root.axis])
        ^ fragment.reversed();
    let ends = if descending {
        [range[1], range[0]]
    } else {
        range
    };
    let uv = |t: [f64; 2]| -> Result<[[[f64; 2]; 2]; 2]> {
        let free = I::point(ends[0])
            .add(I::new(t[0], t[1])?.mul(I::point(ends[1]).sub(I::point(ends[0]))?)?)?
            .intersect(range[0], range[1])?;
        let mut source = [[root.parameters[end]; 2]; 2];
        source[1 - root.axis] = [free.lo, free.hi];
        Ok([source, [[free.lo, free.hi], [end as f64; 2]]])
    };
    let mut report = BoundaryMeshReport {
        vertices: vec![],
        segments: vec![],
        pending: vec![],
        cells: 0,
        spans: 0,
    };
    let sample = |t: f64, remaining: usize| -> Result<(Option<BoundaryMeshVertex>, usize)> {
        let charts = uv([t, t])?;
        let (bounds, spent) = profile.enclose(charts[0], remaining)?;
        Ok((
            bounds.map(|world| BoundaryMeshVertex {
                canonical: [t, t],
                parameters: [[t, t]; 2],
                uv: charts,
                world,
                point: std::array::from_fn(|k| world[k][0] * 0.5 + world[k][1] * 0.5),
            }),
            spent,
        ))
    };
    for t in [0., 1.] {
        let (v, spent) = sample(t, limits.spans - report.spans)?;
        report.spans += spent;
        let Some(v) = v else {
            report.pending.push(BoundaryMeshPending {
                vertices: None,
                reason: "boundary-mesh-work-limit",
            });
            return Ok(report);
        };
        report.vertices.push(v);
    }
    let mut queue = VecDeque::from([([0, 1], 0)]);
    while let Some((indices, depth)) = queue.pop_front() {
        if report.cells == limits.cells || report.spans == limits.spans {
            report.pending.push(BoundaryMeshPending {
                vertices: Some(indices),
                reason: "boundary-mesh-work-limit",
            });
            continue;
        }
        report.cells += 1;
        let a = &report.vertices[indices[0]];
        let b = &report.vertices[indices[1]];
        let charts = uv([a.canonical[0], b.canonical[1]])?;
        let (bounds, spent) = profile.enclose(charts[0], limits.spans - report.spans)?;
        report.spans += spent;
        if let Some(bounds) = bounds {
            // The exact section and its rounded chord both lie in this box.
            let mut deviation = I::point(0.);
            for k in 0..3 {
                let lo = bounds[k][0].min(a.point[k]).min(b.point[k]);
                let hi = bounds[k][1].max(a.point[k]).max(b.point[k]);
                deviation = deviation.add(I::point(hi).sub(I::point(lo))?)?;
            }
            if deviation.hi <= limits.max_deviation_mm {
                report.segments.push(BoundaryMeshSegment {
                    vertices: indices,
                    deviation_upper_mm: deviation.hi,
                });
                continue;
            }
        }
        if depth == limits.max_depth {
            report.pending.push(BoundaryMeshPending {
                vertices: Some(indices),
                reason: "boundary-mesh-depth-limit",
            });
            continue;
        }
        let t = a.canonical[1] * 0.5 + b.canonical[0] * 0.5;
        if t <= a.canonical[1] || t >= b.canonical[0] {
            report.pending.push(BoundaryMeshPending {
                vertices: Some(indices),
                reason: "boundary-mesh-parameter-resolution",
            });
            continue;
        }
        let (v, spent) = sample(t, limits.spans - report.spans)?;
        report.spans += spent;
        let Some(v) = v else {
            report.pending.push(BoundaryMeshPending {
                vertices: Some(indices),
                reason: "boundary-mesh-work-limit",
            });
            continue;
        };
        let middle = report.vertices.len();
        report.vertices.push(v);
        queue.push_back(([indices[0], middle], depth + 1));
        queue.push_back(([middle, indices[1]], depth + 1));
    }
    Ok(report)
}
