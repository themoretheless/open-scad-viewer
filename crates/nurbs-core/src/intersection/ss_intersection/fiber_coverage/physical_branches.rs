//! Physical embedded open arcs for common-coordinate discovered components.
//! General mixed-coordinate/closed branches and periodic identifications refuse.
use super::*;
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub struct PhysicalFiberBranchLimits {
    pub max_branches: usize,
    pub max_injectivity_spans_per_surface: usize,
    pub injectivity_refinement_depth: usize,
    pub max_rectangles_per_endpoint: usize,
}
impl Default for PhysicalFiberBranchLimits {
    fn default() -> Self {
        Self {
            max_branches: 1024,
            max_injectivity_spans_per_surface: 10000,
            injectivity_refinement_depth: 2,
            max_rectangles_per_endpoint: 10000,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct OriginalPhysicalFiberBranch {
    pub component: usize,
    pub free_axis: FiberAxis,
    pub domain: [f64; 2],
    pub charts: Vec<usize>,
    pub vertices: [usize; 2],
}
/// An exact procedural trim: F=S1-S2=0 on the owned branch, projected to
/// the indicated ORIGINAL surface UV. This is not a fitted NURBS pcurve.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPhysicalFiberTrim {
    pub branch: usize,
    pub surface: usize,
}
#[derive(Clone, Debug)]
pub struct OriginalPhysicalFiberVertex {
    branch: usize,
    end: usize,
    chart: usize,
    point: QualifiedFiber,
}
impl OriginalPhysicalFiberVertex {
    pub fn branch(&self) -> usize {
        self.branch
    }
    pub fn end(&self) -> usize {
        self.end
    }
    pub fn chart(&self) -> usize {
        self.chart
    }
    /// Exact root selector at a singleton ORIGINAL free coordinate. Its
    /// enclosure is not an authored or snapped world endpoint.
    pub fn point(&self) -> &QualifiedFiber {
        &self.point
    }
}
#[derive(Clone, Debug)]
pub struct QualifiedPhysicalFiberBranches {
    owner: Arc<QualifiedDiscoveredFiberComponents>,
    limits: PhysicalFiberBranchLimits,
    branches: Vec<OriginalPhysicalFiberBranch>,
    vertices: Vec<OriginalPhysicalFiberVertex>,
    trims: Vec<OriginalPhysicalFiberTrim>,
}
fn build(
    owner: Arc<QualifiedDiscoveredFiberComponents>,
    limits: PhysicalFiberBranchLimits,
) -> Result<QualifiedPhysicalFiberBranches> {
    check(
        (1..=65536).contains(&limits.max_branches)
            && (1..=100000).contains(&limits.max_injectivity_spans_per_surface)
            && limits.injectivity_refinement_depth <= 6
            && (1..=100000).contains(&limits.max_rectangles_per_endpoint),
        "Invalid physical branch proof allowances",
    )?;
    owner.reverify()?;
    check(
        owner.components().len() <= limits.max_branches,
        "Complete physical branch census exceeds allowance",
    )?;
    let mut out = QualifiedPhysicalFiberBranches {
        owner,
        limits,
        branches: vec![],
        vertices: vec![],
        trims: vec![],
    };
    if out.owner.components().is_empty() {
        return Ok(out);
    }
    let atlas = out.owner.owner();
    let domain = atlas.domain();
    for (surface, uv) in [
        (atlas.source().first(), [domain[0], domain[1]]),
        (atlas.source().second(), [domain[2], domain[3]]),
    ] {
        let proof = crate::surface_injectivity::certify_on_refined(
            surface,
            uv,
            limits.max_injectivity_spans_per_surface,
            limits.injectivity_refinement_depth,
        )?;
        check(
            proof.proven,
            "Both complete original support rectangles need one global injectivity proof",
        )?;
    }
    for (component, indices) in out.owner.components().iter().enumerate() {
        let first = *indices
            .first()
            .ok_or_else(|| crate::input("Empty original root component"))?;
        let axis = atlas.fibers()[first].free_axis();
        let free = axis.index();
        let mut intervals = Vec::with_capacity(indices.len());
        for &i in indices {
            let f = &atlas.fibers()[i];
            check(
                f.free_axis() == axis,
                "Mixed-coordinate component needs a full branch transition census",
            )?;
            check(
                (0..4)
                    .all(|k| domain[k][0] <= f.domain()[k][0] && f.domain()[k][1] <= domain[k][1]),
                "Physical branch chart extends outside the proved original support rectangles",
            )?;
            let d = f.domain()[free];
            check(
                d[0] < d[1],
                "Physical arc requires a nondegenerate free interval",
            )?;
            intervals.push((d, i));
        }
        intervals.sort_by(|a, b| a.0[0].total_cmp(&b.0[0]).then(a.1.cmp(&b.1)));
        let low = intervals[0].0[0];
        let mut high = intervals[0].0[1];
        let start = intervals[0].1;
        let mut finish = start;
        for &(d, i) in &intervals[1..] {
            check(
                d[0] <= high,
                "Original branch free-coordinate cover has a gap",
            )?;
            if d[1] > high {
                high = d[1];
                finish = i;
            }
        }
        // All charts are compact continuous unique-root graphs in the SAME
        // coordinate. The replayed connection forest glues actual common
        // roots. Equality on any nonempty overlap is closed and relatively
        // open by local uniqueness and continuity, hence the graphs glue on
        // the entire common interval. Induction along that forest produces
        // one graph over [low,high], not a nerve-derived loop/branch degree.
        // Both support injections make the parameter-root -> world map
        // injective: equal world points force both UV pairs equal. Compactness
        // then gives an embedded physical arc, and distinct parameter
        // components remain physically disjoint. No C1 claim across knots.
        let vertices = [out.vertices.len(), out.vertices.len() + 1];
        for (end, (chart, parameter)) in [(start, low), (finish, high)].into_iter().enumerate() {
            let mut box4 = atlas.fibers()[chart].domain();
            box4[free] = [parameter; 2];
            let point = atlas
                .source()
                .certify(
                    box4,
                    axis,
                    FiberLimits {
                        max_rectangles: limits.max_rectangles_per_endpoint,
                        preconditioner: None,
                    },
                )?
                .into_fiber()
                .ok_or_else(|| {
                    crate::input("Original physical arc endpoint root remains unproved")
                })?;
            out.vertices.push(OriginalPhysicalFiberVertex {
                branch: component,
                end,
                chart,
                point,
            });
        }
        out.branches.push(OriginalPhysicalFiberBranch {
            component,
            free_axis: axis,
            domain: [low, high],
            charts: indices.clone(),
            vertices,
        });
        for surface in 0..2 {
            out.trims.push(OriginalPhysicalFiberTrim {
                branch: component,
                surface,
            });
        }
    }
    Ok(out)
}
impl QualifiedPhysicalFiberBranches {
    pub fn owner(&self) -> &Arc<QualifiedDiscoveredFiberComponents> {
        &self.owner
    }
    pub fn branches(&self) -> &[OriginalPhysicalFiberBranch] {
        &self.branches
    }
    pub fn vertices(&self) -> &[OriginalPhysicalFiberVertex] {
        &self.vertices
    }
    pub fn trims(&self) -> &[OriginalPhysicalFiberTrim] {
        &self.trims
    }
    /// Complete with respect to the original requested atlas domain only.
    pub fn covers_entire_surfaces(&self) -> bool {
        self.owner.owner().covers_entire_surfaces()
    }
    /// Select the exact original intersection point at an ORIGINAL free
    /// coordinate. The result owns an isolating root box, never its midpoint.
    pub fn station(
        &self,
        branch: usize,
        parameter: f64,
        max_rectangles: usize,
    ) -> Result<QualifiedFiber> {
        check(
            parameter.is_finite() && (1..=100000).contains(&max_rectangles),
            "Invalid original branch station or proof allowance",
        )?;
        let b = self
            .branches
            .get(branch)
            .ok_or_else(|| crate::input("Unknown original physical branch"))?;
        check(
            b.domain[0] <= parameter && parameter <= b.domain[1],
            "Station lies outside the complete original branch",
        )?;
        let atlas = self.owner.owner();
        let free = b.free_axis.index();
        let chart = b
            .charts
            .iter()
            .copied()
            .find(|&i| {
                let d = atlas.fibers()[i].domain()[free];
                d[0] <= parameter && parameter <= d[1]
            })
            .ok_or_else(|| crate::input("Complete branch lost its original station chart"))?;
        let mut domain = atlas.fibers()[chart].domain();
        domain[free] = [parameter; 2];
        atlas
            .source()
            .certify(
                domain,
                b.free_axis,
                FiberLimits {
                    max_rectangles,
                    preconditioner: None,
                },
            )?
            .into_fiber()
            .ok_or_else(|| crate::input("Original physical branch station root remains unproved"))
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = build(self.owner.clone(), self.limits)?;
        check(
            fresh.branches == self.branches
                && fresh.trims == self.trims
                && fresh.vertices.len() == self.vertices.len()
                && fresh.vertices.iter().zip(&self.vertices).all(|(a, b)| {
                    a.branch == b.branch
                        && a.end == b.end
                        && a.chart == b.chart
                        && a.point.source().first() == b.point.source().first()
                        && a.point.source().second() == b.point.source().second()
                        && a.point.domain() == b.point.domain()
                        && a.point.free_axis() == b.point.free_axis()
                        && a.point.report() == b.point.report()
                }),
            "Original physical branch, endpoint or trim ownership changed",
        )
    }
}
pub fn assemble_physical_fiber_branches(
    owner: Arc<QualifiedDiscoveredFiberComponents>,
    limits: PhysicalFiberBranchLimits,
) -> Result<QualifiedPhysicalFiberBranches> {
    build(owner, limits)
}
