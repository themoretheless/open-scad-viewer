//! Either original carrier order may supply a sufficient common-clock proof.
use super::*;
pub(super) struct Physical {
    pub(super) native: ss::QualifiedPhysicalFiberBranches,
    pub(super) swapped: bool,
}
impl Physical {
    pub(super) fn canonical(&self, b: [[f64; 2]; 4]) -> [[f64; 2]; 4] {
        if self.swapped {
            [b[2], b[3], b[0], b[1]]
        } else {
            b
        }
    }
    pub(super) fn branches(&self) -> Vec<ss::OriginalPhysicalFiberBranch> {
        self.native
            .branches()
            .iter()
            .cloned()
            .map(|mut b| {
                if self.swapped {
                    b.free_axis = match b.free_axis {
                        ss::FiberAxis::FirstU => ss::FiberAxis::SecondU,
                        ss::FiberAxis::FirstV => ss::FiberAxis::SecondV,
                        ss::FiberAxis::SecondU => ss::FiberAxis::FirstU,
                        ss::FiberAxis::SecondV => ss::FiberAxis::FirstV,
                    };
                }
                b
            })
            .collect()
    }
}
pub(super) fn construct(build: &mut Build<'_>, faces: [usize; 2]) -> Result<Physical> {
    let mut reasons = Vec::<String>::new();
    for swapped in [false, true] {
        let order = if swapped { [1, 0] } else { [0, 1] };
        let sources = ss::FiberSources::new(
            &build.models[order[0]].faces[faces[order[0]]].surface,
            &build.models[order[1]].faces[faces[order[1]]].surface,
        )?;
        let discovered = ss::discover_fiber_surfaces(
            &sources,
            ss::FiberDiscoveryLimits {
                coverage: ss::CoverageLimits {
                    max_cells: 4096,
                    max_rectangles: build.rectangles()?,
                    max_depth: 24,
                },
                max_fibers: 64,
            },
        )?;
        build.work.rectangles += discovered.report().rectangles;
        let Some(atlas) = discovered.into_atlas() else {
            reasons.push("original coverage".into());
            continue;
        };
        let linked = ss::connect_discovered_fiber_atlas(
            Arc::new(atlas),
            ss::DiscoveredComponentLimits {
                max_rectangles: build.rectangles()?,
                max_bridges: 64,
            },
        )?;
        build.work.rectangles += linked.rectangles();
        let Some(owner) = linked.into_components() else {
            reasons.push("component census".into());
            continue;
        };
        // Physical assembly replays this complete owner once more.
        build.work.rectangles += owner.rectangles();
        let ends = 2 * owner.components().len();
        let allowance = if ends == 0 {
            build.rectangles()?
        } else {
            build.rectangles()? / ends
        };
        if allowance == 0 {
            return Err(fail("physical branch endpoint allowance exhausted"));
        }
        match ss::assemble_physical_fiber_branches(
            Arc::new(owner),
            ss::PhysicalFiberBranchLimits {
                max_branches: build.limits.max_roots,
                max_injectivity_spans_per_surface: 4096,
                injectivity_refinement_depth: 2,
                max_rectangles_per_endpoint: allowance,
            },
        ) {
            Ok(native) => {
                build.work.rectangles += native
                    .vertices()
                    .iter()
                    .map(|v| v.point().report().rectangles)
                    .sum::<usize>();
                return Ok(Physical { native, swapped });
            }
            Err(error) => {
                // Failed assembly exposes no endpoint ledger. Reserve its
                // ENTIRE offered endpoint allowance before trying another order.
                build.work.rectangles += ends * allowance;
                reasons.push(format!("physical arc proof: {}", error.message));
            }
        }
    }
    Err(fail(&format!(
        "complete physical intersections for {faces:?} remain unproved in both orders: {reasons:?}"
    )))
}
