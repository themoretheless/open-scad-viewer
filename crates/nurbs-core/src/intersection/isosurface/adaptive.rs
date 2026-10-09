use super::*;

/// Octree node used during adaptive refinement.
struct OctreeCell {
    min: [f64; 3],
    depth: u32,
    corners: [f64; 8],
}

impl OctreeCell {
    fn sign_change(&self) -> bool {
        let neg = self.corners[0] < 0.;
        self.corners.iter().any(|&v| (v < 0.) != neg)
    }
}

/// Refine adaptively and return the extraction depth (deepest sign-changing
/// cell seen) together with statistics. `None` means no sign change anywhere.
pub(super) fn adaptive_depth(
    eval: &mut Evaluator,
    min: [f64; 3],
    max: [f64; 3],
    config: &IsosurfaceConfig,
    report: &mut IsosurfaceReport,
) -> Result<Option<u32>> {
    let mut corners = [0.; 8];
    for (c, off) in corners.iter_mut().zip(CORNER_OFFSET.iter()) {
        *c = eval.value([
            if off[0] == 0 { min[0] } else { max[0] },
            if off[1] == 0 { min[1] } else { max[1] },
            if off[2] == 0 { min[2] } else { max[2] },
        ])?;
    }
    let root = OctreeCell {
        min,
        depth: 0,
        corners,
    };
    let extents = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    let mut stack = vec![(root, extents)];
    let mut cells_created = 1usize;
    let mut depth_reached = 0u32;
    let mut sign_leaves = 0usize;
    // Unified budget guard backing the configured max_depth/max_cells limits:
    // one tick per processed cell, wall-clock checked at the same waypoint.
    let mut guard = Budget::new(
        config.max_cells.max(1),
        (config.max_depth as usize + 1).max(1),
        u64::MAX,
    )?
    .guard("isosurface_adaptive_refinement");
    // Deterministic traversal: children are pushed in a fixed lexicographic
    // (dz, dy, dx) order and popped from the end.
    while let Some((cell, ext)) = stack.pop() {
        guard.tick()?;
        guard.check()?;
        let center = [
            cell.min[0] + ext[0] / 2.,
            cell.min[1] + ext[1] / 2.,
            cell.min[2] + ext[2] / 2.,
        ];
        let trilinear: f64 = cell.corners.iter().sum::<f64>() / 8.;
        let lin_err = (eval.value(center)? - trilinear).abs();
        // Subdivide on linearization (curvature) error alone: a surface fully
        // contained in a cell shows no corner sign change but a large
        // center-vs-trilinear discrepancy, so this criterion cannot miss it.
        let subdivide = cell.depth < config.max_depth && lin_err > config.linearization_tolerance;
        if !subdivide {
            if cell.sign_change() {
                sign_leaves += 1;
                depth_reached = depth_reached.max(cell.depth);
            }
            continue;
        }
        if cells_created + 8 > config.max_cells {
            return Err(resource(
                "octree cell budget exhausted during adaptive refinement",
            ));
        }
        cells_created += 8;
        let half = [ext[0] / 2., ext[1] / 2., ext[2] / 2.];
        for dz in 0..2u32 {
            for dy in 0..2u32 {
                for dx in 0..2u32 {
                    let child_min = [
                        cell.min[0] + f64::from(dx) * half[0],
                        cell.min[1] + f64::from(dy) * half[1],
                        cell.min[2] + f64::from(dz) * half[2],
                    ];
                    let mut cc = [0.; 8];
                    for (c, off) in cc.iter_mut().zip(CORNER_OFFSET.iter()) {
                        *c = eval.value([
                            child_min[0] + f64::from(off[0]) * half[0],
                            child_min[1] + f64::from(off[1]) * half[1],
                            child_min[2] + f64::from(off[2]) * half[2],
                        ])?;
                    }
                    stack.push((
                        OctreeCell {
                            min: child_min,
                            depth: cell.depth + 1,
                            corners: cc,
                        },
                        half,
                    ));
                }
            }
        }
    }
    report.octree_cells = cells_created;
    report.sign_leaves = sign_leaves;
    Ok((depth_reached > 0 || sign_leaves > 0).then_some(depth_reached))
}
