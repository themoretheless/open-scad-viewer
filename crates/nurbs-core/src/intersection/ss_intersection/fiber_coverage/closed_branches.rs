//! Embedded original closed branches, not a cycle inferred from chart adjacency.
//!
//! The complete zero set of C1 F=S1-S2 on a compact product is compact. Original
//! contraction operators give rank DF=3 at every root. Strict interior root
//! enclosures exclude the product boundary. The regular-level-set theorem then
//! makes each owned connected component a compact connected one-manifold with
//! no boundary, hence a circle. Global injection of BOTH supports makes the
//! root-to-world map injective; compactness makes it a world embedding. Separate
//! support immersion proofs exclude zero world tangent. No fitted trim, loop
//! orientation, periodic quotient or volumetric role follows from this proof.
use super::*;
use crate::surface_injectivity;
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub struct ClosedFiberBranchLimits {
    pub max_branches: usize,
    pub max_injectivity_spans_per_surface: usize,
    pub injectivity_refinement_depth: usize,
}
impl Default for ClosedFiberBranchLimits {
    fn default() -> Self {
        Self {
            max_branches: 64,
            max_injectivity_spans_per_surface: 10000,
            injectivity_refinement_depth: 2,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalClosedFiberBranch {
    pub component: usize,
    pub charts: Vec<usize>,
}
/// Procedural F=0 trim projected onto an original carrier, not a NURBS pcurve.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalClosedFiberTrim {
    pub branch: usize,
    pub surface: usize,
}
#[derive(Clone, Debug)]
pub struct QualifiedClosedFiberBranches {
    owner: Arc<DiscoveredMixedFiberComponents>,
    limits: ClosedFiberBranchLimits,
    branches: Vec<OriginalClosedFiberBranch>,
    trims: Vec<OriginalClosedFiberTrim>,
    root_enclosures: Vec<Box4>,
    injectivity: [surface_injectivity::Report; 2],
}
impl QualifiedClosedFiberBranches {
    pub fn owner(&self) -> &Arc<DiscoveredMixedFiberComponents> {
        &self.owner
    }
    pub fn branches(&self) -> &[OriginalClosedFiberBranch] {
        &self.branches
    }
    pub fn trims(&self) -> &[OriginalClosedFiberTrim] {
        &self.trims
    }
    pub fn root_enclosures(&self) -> &[Box4] {
        &self.root_enclosures
    }
    pub fn support_injectivity(&self) -> &[surface_injectivity::Report; 2] {
        &self.injectivity
    }
    /// An exact original root selector on one owned chart. No global free
    /// coordinate, sampled world point or artificially inserted seam is implied.
    pub fn point_at(
        &self,
        branch: usize,
        chart: usize,
        parameter: f64,
        limits: FiberLimits,
    ) -> Result<QualifiedFiber> {
        let b = self
            .branches
            .get(branch)
            .ok_or_else(|| crate::input("Unknown closed branch"))?;
        check(
            b.charts.contains(&chart),
            "Point chart does not belong to this closed branch",
        )?;
        let f = &self.owner.proof().coverage().fibers()[chart];
        f.at(parameter, limits)?
            .into_fiber()
            .ok_or_else(|| crate::input("Owned closed-branch point remains unproved"))
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = assemble_closed_fiber_branches(self.owner.clone(), self.limits)?;
        check(
            self.branches == fresh.branches
                && self.trims == fresh.trims
                && self.root_enclosures == fresh.root_enclosures
                && self
                    .injectivity
                    .iter()
                    .zip(&fresh.injectivity)
                    .all(|(a, b)| same_injectivity(a, b)),
            "Original closed branches, interior roots or world injection receipts changed",
        )
    }
}
fn c1_source(s: &Surface) -> Result<()> {
    s.validate()?;
    check(
        !s.periodic_u && !s.periodic_v,
        "Closed branch proof does not identify periodic chart boundaries",
    )?;
    for (degree, knots, n) in [
        (s.degree_u, &s.knots_u, s.control_points.len()),
        (s.degree_v, &s.knots_v, s.control_points[0].len()),
    ] {
        check(
            degree > 0,
            "Closed branches require two-dimensional C1 carriers",
        )?;
        let low = knots[degree];
        let high = knots[n];
        check(
            knots[..=degree].iter().all(|&t| t == low) && knots[n..].iter().all(|&t| t == high),
            "Closed branch proof requires original clamped endpoint continuity",
        )?;
        let mut i = 0;
        while i < knots.len() {
            let mut j = i + 1;
            while j < knots.len() && knots[j] == knots[i] {
                j += 1;
            }
            check(
                !(low < knots[i] && knots[i] < high) || j - i < degree,
                "Original carrier lacks the source-wide C1 knot-continuity gate",
            )?;
            i = j;
        }
    }
    Ok(())
}
fn root_enclosure(f: &QualifiedFiber) -> Result<Box4> {
    let r = f.report();
    check(
        r.status == FiberStatus::UniformUniqueRoot
            && r.operator.is_some()
            && r.contraction_upper
                .is_some_and(|q| q.is_finite() && 0. <= q && q < 1.)
            && r.support_regularity_proven == [true, true],
        "Closed branch needs original rank-three operators and both support immersions",
    )?;
    let image = r
        .root_image
        .ok_or_else(|| crate::input("Missing original root enclosure"))?;
    let mut domain = f.domain();
    let mut index = 0;
    for k in 0..4 {
        if k != f.free_axis().index() {
            // Both the actual chart and its outward image enclose every root.
            domain[k] = [
                domain[k][0].max(image[index][0]),
                domain[k][1].min(image[index][1]),
            ];
            index += 1;
        }
    }
    Ok(domain)
}
/// Own every physical closed branch of a complete natural surface product.
/// All hypotheses of the manifold/embedding argument above are native gates.
/// Open arcs, incomplete coverage, insufficient smoothness, boundary contact,
/// unproved rank or support injection refuse without publishing an owner.
pub fn assemble_closed_fiber_branches(
    owner: Arc<DiscoveredMixedFiberComponents>,
    limits: ClosedFiberBranchLimits,
) -> Result<QualifiedClosedFiberBranches> {
    check(
        (1..=65536).contains(&limits.max_branches)
            && (1..=100000).contains(&limits.max_injectivity_spans_per_surface)
            && limits.injectivity_refinement_depth <= 6,
        "Invalid closed-branch proof work",
    )?;
    owner.reverify()?;
    let core = owner.proof();
    let cover = core.coverage();
    check(
        cover.covers_entire_surfaces(),
        "Closed branch census requires the entire original surface product",
    )?;
    check(
        core.components().len() <= limits.max_branches,
        "Closed branch census exceeds its component allowance",
    )?;
    let first = &cover.fibers()[0];
    let sources = first.source();
    c1_source(sources.first())?;
    c1_source(sources.second())?;
    let product = cover.domain();
    let roots = cover
        .fibers()
        .iter()
        .map(root_enclosure)
        .collect::<Result<Vec<_>>>()?;
    check(
        roots.iter().all(|b| {
            (0..4).all(|k| {
                b[k].iter().all(|x| x.is_finite())
                    && product[k][0] < b[k][0]
                    && b[k][0] <= b[k][1]
                    && b[k][1] < product[k][1]
            })
        }),
        "Closed branches need strictly interior original root enclosures; boundary arcs are separate",
    )?;
    let mut injections = vec![];
    for (s, uv) in [
        (sources.first(), [product[0], product[1]]),
        (sources.second(), [product[2], product[3]]),
    ] {
        let proof = surface_injectivity::certify_on_refined(
            s,
            uv,
            limits.max_injectivity_spans_per_surface,
            limits.injectivity_refinement_depth,
        )?;
        check(
            proof.proven,
            "Both original complete carriers need global injection for closed world branches",
        )?;
        injections.push(proof);
    }
    let branches = core
        .components()
        .iter()
        .enumerate()
        .map(|(component, charts)| OriginalClosedFiberBranch {
            component,
            charts: charts.clone(),
        })
        .collect::<Vec<_>>();
    let trims = (0..branches.len())
        .flat_map(|branch| (0..2).map(move |surface| OriginalClosedFiberTrim { branch, surface }))
        .collect();
    Ok(QualifiedClosedFiberBranches {
        owner,
        limits,
        branches,
        trims,
        root_enclosures: roots,
        injectivity: injections.try_into().unwrap(),
    })
}
fn same_injectivity(a: &surface_injectivity::Report, b: &surface_injectivity::Report) -> bool {
    let polar = match (&a.polar_p_matrix, &b.polar_p_matrix) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            a.coordinates == b.coordinates
                && a.jacobian == b.jacobian
                && a.row_signs == b.row_signs
                && a.diagonal_lower == b.diagonal_lower
                && a.determinant_lower == b.determinant_lower
        }
        _ => false,
    };
    polar
        && a.proven == b.proven
        && a.projection == b.projection
        && a.linear_projection == b.linear_projection
        && a.projective_projection == b.projective_projection
        && a.polar_projection == b.polar_projection
        && a.contraction_upper == b.contraction_upper
        && a.spans == b.spans
        && a.reason == b.reason
}
