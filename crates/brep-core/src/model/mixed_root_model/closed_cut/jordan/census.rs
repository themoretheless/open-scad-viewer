//! Complete original-root coverage bound to distinct admitted components.
use super::*;
use std::sync::Arc;
#[derive(Clone, Copy, Debug)]
pub struct InteriorCensusAnchor {
    pub component: usize,
    pub edge: usize,
    /// Free clock of the corresponding coverage fiber, not a fitted point.
    pub parameter: f64,
    pub phase: [i32; 4],
}
#[derive(Clone, Copy)]
pub struct InteriorCensusLimits {
    pub separation_queries: usize,
    /// Explicit per-query allowance, shared query count bounds total work.
    pub geometry: ss::OriginalSsCurveLimits,
}
impl Default for InteriorCensusLimits {
    fn default() -> Self {
        Self {
            separation_queries: 4096,
            geometry: Default::default(),
        }
    }
}
#[derive(Clone, Copy)]
pub struct ClosedSsCensusLimits {
    pub discovery: ss::ClosedSSBuildLimits,
    pub cycle: ss::ClosedFiberCycleLimits,
    pub cuts: InteriorLoopLimits,
    pub coverage: ss::CoverageLimits,
    pub census: InteriorCensusLimits,
    pub seed_cells_per_support: usize,
    pub max_bind_attempts: usize,
}

impl Default for ClosedSsCensusLimits {
    fn default() -> Self {
        Self {
            discovery: Default::default(),
            cycle: Default::default(),
            cuts: Default::default(),
            coverage: Default::default(),
            census: Default::default(),
            seed_cells_per_support: 64,
            max_bind_attempts: 4096,
        }
    }
}

pub struct QualifiedInteriorSsCensus {
    loops: Vec<Arc<QualifiedInteriorSsLoop>>,
    coverage: ss::QualifiedQuotientCoverage,
    anchors: Vec<InteriorCensusAnchor>,
    limits: InteriorCensusLimits,
    separation_queries: usize,
}
impl QualifiedInteriorSsCensus {
    /// Find a face pair with a complete closed interior intersection census.
    /// The selected pair still requires the Boolean admission checks on the
    /// remaining faces before its census may be used to publish a body.
    pub fn discover_closed_branches_between(
        solids: [&crate::closed_solid::QualifiedClosedSolid; 2],
        limits: ClosedSsCensusLimits,
        max_pair_attempts: usize,
    ) -> Result<Self> {
        let mut last = None;
        for [first, second] in overlapping_face_pairs(solids, max_pair_attempts)? {
            match Self::discover_closed_branches(
                [(solids[0], first), (solids[1], second)],
                limits,
            ) {
                Ok(census) => return Ok(census),
                Err(error) => last = Some(error),
            }
        }
        Err(last.unwrap_or_else(|| unproved("No overlapping face pair qualified for closed SS discovery")))
    }

    pub fn components(&self) -> &[Arc<QualifiedInteriorSsLoop>] {
        &self.loops
    }
    /// Original operand face indices in the same order as the coverage domain.
    pub fn face_indices(&self) -> [usize; 2] {
        self.loops[0].cuts.each_ref().map(|cut| cut.face)
    }

    pub fn coverage(&self) -> &ss::QualifiedQuotientCoverage {
        &self.coverage
    }
    pub fn anchors(&self) -> &[InteriorCensusAnchor] {
        &self.anchors
    }
    pub fn separation_queries(&self) -> usize {
        self.separation_queries
    }
    /// Discover all closed branches of a selected pair of original faces and
    /// build the finite BRep cuts and their complete binding certificate.
    pub fn discover_closed_branches(
        sources: [(&crate::closed_solid::QualifiedClosedSolid, usize); 2],
        limits: ClosedSsCensusLimits,
    ) -> Result<Self> {
        let [(a, fa), (b, fb)] = sources;
        let first = a.model().faces.get(fa)
            .ok_or_else(|| unproved("First closed SS discovery face index is out of range"))?;
        let second = b.model().faces.get(fb)
            .ok_or_else(|| unproved("Second closed SS discovery face index is out of range"))?;
        let pair = ss::FiberSources::new(&first.surface, &second.surface)?;
        let domain = face_product(sources)?;
        let natural = [first, second].map(|face| crate::boolean_robustness::surface_uv_domains(&face.surface));
        let [a, b] = natural;
        let branches = if domain == [a[0], a[1], b[0], b[1]] {
            ss::construct_qualified_closed_ss_branches(&pair, limits.discovery)?
        } else {
            ss::construct_qualified_closed_ss_branches_in_box(&pair, domain, limits.discovery)?
        };
        Self::qualify_from_closed_branches(
            sources,
            &branches,
            limits.cycle,
            limits.cuts,
            limits.coverage,
            limits.census,
            limits.seed_cells_per_support,
            limits.max_bind_attempts,
        )
    }

    /// Turn the complete closed-branch proof into exact finite cuts and
    /// qualify all of their chart-to-contour bindings for Boolean assembly.
    pub fn qualify_from_closed_branches(
        sources: [(&crate::closed_solid::QualifiedClosedSolid, usize); 2],
        owner: &ss::QualifiedClosedFiberBranches,
        cycle_limits: ss::ClosedFiberCycleLimits,
        cut_limits: InteriorLoopLimits,
        coverage_limits: ss::CoverageLimits,
        census_limits: InteriorCensusLimits,
        seed_cells_per_support: usize,
        max_bind_attempts: usize,
    ) -> Result<Self> {
        let native = owner.owner().proof().coverage();
        if !(1..=64).contains(&owner.branches().len())
            || !native.covers_entire_surfaces()
                && !product_contains(native.domain(), face_product(sources)?)
        {
            return Err(unproved("Solid Boolean needs a complete closed SS branch proof"));
        }
        let mut loops = Vec::with_capacity(owner.branches().len());
        for branch in 0..owner.branches().len() {
            let cycle = ss::order_closed_fiber_cycle(owner, branch, cycle_limits)?;
            loops.push(Arc::new(QualifiedInteriorSsLoop::qualify_from_cycle(
                sources, &cycle, cut_limits, seed_cells_per_support,
            )?));
        }
        let mut quotient = Vec::with_capacity(native.fibers().len());
        for (index, chart) in native.fibers().iter().enumerate() {
            let result = chart.source().certify_quotient(
                chart.domain(),
                chart.free_axis(),
                ss::FiberLimits {
                    preconditioner: chart.report().operator.as_ref().map(|operator| operator.preconditioner()),
                    ..Default::default()
                },
            )?;
            let reason = result.report().reason;
            quotient.push(result.into_fiber().ok_or_else(|| {
                unproved(&format!("Closed branch chart {index} could not qualify on the original quotient: {reason}"))
            })?);
        }
        let source = quotient[0].source();
        let coverage = if [source.first(), source.second()]
            .into_iter()
            .all(|surface| !surface.periodic_u && !surface.periodic_v)
        {
            ss::QualifiedQuotientCoverage::from_nonperiodic_mixed(
                native,
                quotient,
                coverage_limits,
            )?
        } else {
            ss::cover_quotient_fiber_box(&quotient, native.domain(), coverage_limits)?
                .into_coverage()
                .ok_or_else(|| unproved("Closed branches lack complete original quotient coverage"))?
        };
        Self::qualify_with_automatic_anchors(loops, coverage, census_limits, max_bind_attempts)
    }

    /// Match each coverage chart to an exact interior point on a retained
    /// contour, inferring an exact native phase when a support is periodic.
    pub fn qualify_with_automatic_anchors(
        loops: Vec<Arc<QualifiedInteriorSsLoop>>,
        coverage: ss::QualifiedQuotientCoverage,
        limits: InteriorCensusLimits,
        max_bind_attempts: usize,
    ) -> Result<Self> {
        if !(1..=64).contains(&loops.len())
            || !(1..=1_000_000).contains(&max_bind_attempts)
        {
            return Err(unproved("Invalid automatic interior SS anchor inputs"));
        }
        verify_face_coverage(&loops, &coverage)?;
        let mut candidates = Vec::new();
        for (component, contour) in loops.iter().enumerate() {
            for (edge, root) in contour.cuts[0].wire.edges().iter().enumerate() {
                if candidates.len() == max_bind_attempts {
                    return Err(unproved("Automatic SS anchor edge allowance exhausted"));
                }
                candidates.push((component, edge, root, root.enclose(limits.geometry)?.uv));
            }
        }
        let source = coverage.fibers()[0].source();
        let periodic = [
            source.first().periodic_u, source.first().periodic_v,
            source.second().periodic_u, source.second().periodic_v,
        ];
        let infer_phase = periodic.iter().any(|&value| value);
        let mut remaining = max_bind_attempts;
        let mut anchors = Vec::with_capacity(coverage.fibers().len());
        for (chart, fiber) in coverage.fibers().iter().enumerate() {
            let curve = Arc::new(ss::OriginalSsRootCurve::from_quotient(fiber.clone()));
            let [lo, hi] = curve.parameter_domain();
            let mut anchor = None;
            'witness: for fraction in [0.5, 0.25, 0.75] {
                let parameter = lo * (1. - fraction) + hi * fraction;
                if !(lo < parameter && parameter < hi) {
                    continue;
                }
                let point = Arc::new(ss::OriginalSsRootPoint::new(curve.clone(), parameter)?);
                let uv = point.enclose(limits.geometry)?.uv;
                for &(component, edge, root, bounds) in &candidates {
                    if (0..4).any(|k| !periodic[k] && (uv[k][1] < bounds[k][0] || bounds[k][1] < uv[k][0])) {
                        continue;
                    }
                    remaining = remaining.checked_sub(1)
                        .ok_or_else(|| unproved("Automatic SS anchor bind allowance exhausted"))?;
                    let port = if infer_phase {
                        ss::QualifiedOriginalSsPort::bind_with_inferred_phase(
                            point.clone(), root.curve().clone(), limits.geometry,
                        )
                    } else {
                        ss::bind_original_ss_port(point.clone(), root.curve().clone(), [0; 4], limits.geometry)
                    };
                    if let Ok(port) = port {
                        if require_interior_anchor(&port, root).is_ok() {
                            anchor = Some(InteriorCensusAnchor {
                                component, edge, parameter, phase: port.phase(),
                            });
                            break 'witness;
                        }
                    }
                }
            }
            anchors.push(anchor.ok_or_else(|| {
                unproved(&format!("No proved contour anchor for SS coverage chart {chart}"))
            })?);
        }
        qualify_interior_ss_census(loops, coverage, &anchors, limits)
    }

    pub fn reverify(&self) -> Result<()> {
        for component in &self.loops {
            component.reverify()?;
        }
        self.coverage.reverify()?;
        let fresh = qualify_interior_ss_census(
            self.loops.clone(),
            self.coverage.clone(),
            &self.anchors,
            self.limits,
        )?;
        if fresh.separation_queries != self.separation_queries {
            return Err(unproved("SS component separation work changed"));
        }
        Ok(())
    }
}
fn separate(
    a: &QualifiedInteriorSsLoop,
    b: &QualifiedInteriorSsLoop,
    limits: InteriorCensusLimits,
    used: &mut usize,
) -> Result<()> {
    if *used == limits.separation_queries {
        return Err(unproved("SS component separation budget exhausted"));
    }
    *used += 1;
    let point = a.cuts[0].wire.edges()[0]
        .start()
        .point()
        .enclose(limits.geometry)?
        .uv;
    for edge in b.cuts[0].wire.edges() {
        let d = edge.parameter_bounds();
        let natural = edge.curve().parameter_domain();
        let range = [
            clock(&d[0], true)?.max(natural[0]),
            clock(&d[1], false)?.min(natural[1]),
        ];
        let mut pending = vec![(range, 0usize)];
        while let Some((range, depth)) = pending.pop() {
            if *used == limits.separation_queries {
                return Err(unproved("SS component separation budget exhausted"));
            }
            *used += 1;
            let uv = edge.curve().enclose(range, limits.geometry)?.uv;
            if (0..4).any(|k| point[k][1] < uv[k][0] || uv[k][1] < point[k][0]) {
                continue;
            }
            let middle = range[0] * 0.5 + range[1] * 0.5;
            if depth == 32 || !(range[0] < middle && middle < range[1]) {
                return Err(unproved(
                    "SS loops are duplicated or distinct components remain unproved",
                ));
            }
            pending.push(([range[0], middle], depth + 1));
            pending.push(([middle, range[1]], depth + 1));
        }
    }
    Ok(())
}
fn bind_anchor(
    point: Arc<ss::OriginalSsRootPoint>,
    edge: &ss::OriginalSsRootEdge,
    phase: [i32; 4],
    geometry: ss::OriginalSsCurveLimits,
) -> Result<()> {
    let port = ss::bind_original_ss_port(point, edge.curve().clone(), phase, geometry)?;
    require_interior_anchor(&port, edge)
}

fn require_interior_anchor(
    port: &ss::QualifiedOriginalSsPort,
    edge: &ss::OriginalSsRootEdge,
) -> Result<()> {
    let p = port.parameter_bounds();
    let (lo, hi) = if edge.forward() {
        (edge.start().parameter_bounds(), edge.end().parameter_bounds())
    } else {
        (edge.end().parameter_bounds(), edge.start().parameter_bounds())
    };
    if !(lo[1] < p[0] && p[1] < hi[0]) {
        return Err(unproved("SS census anchor is not strictly on the retained root edge"));
    }
    Ok(())
}

/// All roots in the complete finite face product are assigned to admitted Jordan
/// components. Connected graph fibers with actual root anchors cannot leave
/// their relatively open/closed component. Pairwise separation of one exact
/// selector from the entire other loop proves the component images distinct.
/// This does not assemble Boolean material or admit a mixed closed solid.
pub fn qualify_interior_ss_census(
    loops: Vec<Arc<QualifiedInteriorSsLoop>>,
    coverage: ss::QualifiedQuotientCoverage,
    anchors: &[InteriorCensusAnchor],
    limits: InteriorCensusLimits,
) -> Result<QualifiedInteriorSsCensus> {
    if !(1..=64).contains(&loops.len())
        || anchors.len() != coverage.fibers().len()
        || !(1..=1000000).contains(&limits.separation_queries)
    {
        return Err(unproved(
            "SS census requires complete coverage and one anchor per source graph",
        ));
    }
    verify_face_coverage(&loops, &coverage)?;
    let source = coverage.fibers()[0].source();
    for component in &loops {
        let s = component.cuts[0].wire.edges()[0].curve().source();
        if s.first() != source.first() || s.second() != source.second() {
            return Err(unproved(
                "SS census component has foreign original supports",
            ));
        }
    }
    let mut used = 0;
    for i in 0..loops.len() {
        for j in i + 1..loops.len() {
            separate(&loops[i], &loops[j], limits, &mut used)?;
        }
    }
    let mut owned = vec![false; loops.len()];
    for (fiber, anchor) in coverage.fibers().iter().zip(anchors) {
        let component = loops
            .get(anchor.component)
            .ok_or_else(|| unproved("SS census anchor component is absent"))?;
        let edge = component.cuts[0]
            .wire
            .edges()
            .get(anchor.edge)
            .ok_or_else(|| unproved("SS census anchor edge is absent"))?;
        let curve = Arc::new(ss::OriginalSsRootCurve::from_quotient(fiber.clone()));
        let point = Arc::new(ss::OriginalSsRootPoint::new(curve, anchor.parameter)?);
        bind_anchor(point, edge, anchor.phase, limits.geometry)?;
        owned[anchor.component] = true;
    }
    if owned.iter().any(|x| !*x) {
        return Err(unproved("SS census has an unowned supplied component"));
    }
    Ok(QualifiedInteriorSsCensus {
        loops,
        coverage,
        anchors: anchors.to_vec(),
        limits,
        separation_queries: used,
    })
}

fn verify_face_coverage(
    loops: &[Arc<QualifiedInteriorSsLoop>],
    coverage: &ss::QualifiedQuotientCoverage,
) -> Result<()> {
    if coverage.covers_entire_surfaces() {
        return Ok(());
    }
    for component in loops {
        let [a, b] = &component.cuts;
        let domain = face_product([(&a.source, a.face), (&b.source, b.face)])?;
        if !product_contains(coverage.domain(), domain) {
            return Err(unproved("SS census does not cover the complete finite face product"));
        }
    }
    Ok(())
}

fn product_contains(outer: [[f64; 2]; 4], inner: [[f64; 2]; 4]) -> bool {
    outer.into_iter().zip(inner).all(|([a, b], [c, d])| a <= c && d <= b)
}

fn face_product(
    sources: [(&crate::closed_solid::QualifiedClosedSolid, usize); 2],
) -> Result<[[f64; 2]; 4]> {
    let [[au, av], [bu, bv]] = [
        finite_face_cover(sources[0].0.model(), sources[0].1)?,
        finite_face_cover(sources[1].0.model(), sources[1].1)?,
    ];
    Ok([au, av, bu, bv])
}

fn finite_face_cover(
    model: &crate::Model,
    index: usize,
) -> Result<[[f64; 2]; 2]> {
    let face = model.faces.get(index)
        .ok_or_else(|| unproved("Finite SS discovery face index is out of range"))?;
    let natural = crate::boolean_robustness::surface_uv_domains(&face.surface);
    let periodic = [face.surface.periodic_u, face.surface.periodic_v];
    let outer = &model.loops[face.outer].coedges;
    if periodic.into_iter().any(|flag| flag) {
        // A winding belt can have two open rim paths. The first path's hull
        // need not enclose any of the material between those rims.
        if !(1..=256).contains(&outer.len())
            || outer.iter().any(|c| c.trim().is_some_and(|t| t.pcurve_parameter != c.pcurve.domain()))
        {
            return Ok(natural);
        }
        let literal = outer.iter().map(|c| c.pcurve.clone()).collect::<Vec<_>>();
        if nurbs_core::trim_domain::exact_loop_joins(&literal)? != Some(true) {
            return Ok(natural);
        }
    }
    // The outer pcurve hull encloses the material. Periodic projection only
    // changes a periodic coordinate. When the authored hull leaves a native
    // period, its projected coverage can reach either end of that axis.
    let mut hull = [[f64::INFINITY, f64::NEG_INFINITY]; 2];
    for coedge in outer {
        for point in &coedge.pcurve.control_points {
            for k in 0..2 {
                hull[k][0] = hull[k][0].min(point[k]);
                hull[k][1] = hull[k][1].max(point[k]);
            }
        }
    }
    let cover = std::array::from_fn(|k| {
        let [lo, hi] = natural[k];
        if periodic[k] && (hull[k][0] < lo || hull[k][1] > hi) {
            natural[k]
        } else {
            [hull[k][0].max(lo), hull[k][1].min(hi)]
        }
    });
    Ok(if cover.iter().all(|[lo, hi]| lo < hi) { cover } else { natural })
}

#[cfg(test)]
mod finite_cover_tests {
    use super::*;

    #[test]
    fn finite_ss_cover_keeps_the_transverse_trim_of_a_wrapped_periodic_face() {
        let mut model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let face = &mut model.0.faces[0];
        let profile = nurbs_core::curve::Curve {
            degree: 1,
            knots: vec![-0.25, 0., 0.25, 0.5, 0.75, 1., 1.25],
            control_points: [[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.], [0., 0., 0.]]
                .map(Vec::from).to_vec(),
            weights: vec![1.; 5],
            periodic: true,
        };
        face.surface = nurbs_core::surface::extrude(&profile, [0., 0., 1.]).unwrap();
        let wire = face.outer;
        let original = model.loops[wire].clone();
        for (start, end, expected_u) in [
            (0.25, 0.75, [0.25, 0.75]),
            (0.75, 1.25, [0., 1.]),
            (-0.25, 0.25, [0., 1.]),
        ] {
            model.loops[wire] = original.clone();
            for coedge in &mut model.loops[wire].coedges {
                for p in &mut coedge.pcurve.control_points {
                    p[0] = start + (end - start) * p[0];
                    p[1] = 0.25 + 0.5 * p[1];
                }
            }
            assert_eq!(finite_face_cover(&model, 0).unwrap(), [expected_u, [0.25, 0.75]]);
        }
        model.loops[wire].coedges.pop();
        assert_eq!(finite_face_cover(&model, 0).unwrap(), [[0., 1.]; 2]);
    }
}

fn bounds_separated(a: &[[f64; 2]; 3], b: &[[f64; 2]; 3]) -> bool {
    a.iter().zip(b).any(|(a, b)| a[1] < b[0] || b[1] < a[0])
}

fn carrier_bounds<'a>(surfaces: impl Iterator<Item = &'a crate::Surface>) -> [[f64; 2]; 3] {
    let mut bounds = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    // Positive-weight NURBS carriers lie in their control hulls.
    for point in surfaces.flat_map(|surface| surface.control_points.iter().flatten()) {
        for (bound, &coordinate) in bounds.iter_mut().zip(point) {
            bound[0] = bound[0].min(coordinate);
            bound[1] = bound[1].max(coordinate);
        }
    }
    bounds
}

/// A cut's authored UV cover can discard parts of a widened carrier before
/// candidate ranking. Keep the control hull if interval evaluation refuses.
fn finite_face_world_bounds(model: &crate::Model, face: usize) -> [[f64; 2]; 3] {
    use nurbs_core::interval_eval::{Interval, evaluate_surface_interval};

    let surface = &model.faces[face].surface;
    let hull = carrier_bounds(std::iter::once(surface));
    let natural = crate::boolean_robustness::surface_uv_domains(surface);
    let Ok([u, v]) = finite_face_cover(model, face) else {
        return hull;
    };
    if [u, v] == natural {
        return hull;
    }
    let enclosure = Interval::new(u[0], u[1]).and_then(|u| {
        evaluate_surface_interval(surface, u, Interval::new(v[0], v[1])?)
    });
    let Ok(values) = enclosure else {
        return hull;
    };
    let [x, y, z] = values.as_slice() else {
        return hull;
    };
    let tightened = std::array::from_fn(|axis| {
        let range = [x, y, z][axis];
        [range.lo.max(hull[axis][0]), range.hi.min(hull[axis][1])]
    });
    if tightened.iter().all(|[lo, hi]| lo <= hi) { tightened } else { hull }
}

fn overlapping_face_pairs(
    solids: [&crate::closed_solid::QualifiedClosedSolid; 2],
    max_pair_attempts: usize,
) -> Result<Vec<[usize; 2]>> {
    if max_pair_attempts == 0 {
        return Err(unproved("Automatic closed SS discovery needs a face-pair budget"));
    }
    let bounds = solids.map(|solid| {
        (0..solid.model().faces.len())
            .map(|face| finite_face_world_bounds(solid.model(), face))
            .collect::<Vec<_>>()
    });
    use std::{cmp::Reverse, collections::BinaryHeap};
    let mut pairs = BinaryHeap::new();
    for (first, a) in bounds[0].iter().enumerate() {
        for (second, b) in bounds[1].iter().enumerate() {
            let overlap: [f64; 3] = std::array::from_fn(|axis| {
                a[axis][1].min(b[axis][1]) - a[axis][0].max(b[axis][0])
            });
            if overlap.iter().all(|width| *width >= 0.) {
                // Rank by relative overlap. A long side wall with only a
                // narrow common strip should not outrank two matching caps.
                let mut relative: [f64; 3] = std::array::from_fn(|axis| {
                    let span = (a[axis][1] - a[axis][0]).max(b[axis][1] - b[axis][0]);
                    if span.is_finite() && span > 0. {
                        overlap[axis].max(0.) / span
                    } else {
                        0.
                    }
                });
                relative.sort_by(f64::total_cmp);
                let score = (relative[2] + relative[1]).to_bits();
                pairs.push(Reverse((score, Reverse([first, second]))));
                if pairs.len() > max_pair_attempts {
                    pairs.pop();
                }
            }
        }
    }
    Ok(pairs.into_sorted_vec().into_iter().map(|Reverse((_, Reverse(pair)))| pair).collect())
}

mod regions;
pub use regions::{CensusRectangleReport, QualifiedCensusMaterialRectangle};

mod arrangement;
pub use arrangement::{QualifiedArrangementMaterialRectangle, CensusArrangementLimits, CensusFaceArrangement};

mod sew;
pub use sew::{PairedRootSolidAdmission, QualifiedPairedRootSolid, QualifiedRootBoolean, SeparatedRootBoolean,QualifiedRootShellForest,RootRelationKind,RootRelationLimits,RootRelationWork,QualifiedRootRelation,RootDisjointBodyAssembly, RootShellSubtraction, RootCavityLimits,RootCavityCoverageWork,QualifiedRootCavitySolid,RootSurfaceMaterialLimits,QualifiedRootSurfaceMaterialBox,RootMaterialBoxLimits,RootMaterialBoxWork,QualifiedRootMaterialBox,RootSolidPointLimits, RootSolidPointReport, RootSolidRayReport, RootBoundaryVolumeLimits, RootBoundaryVolumeReport, ClosedSsBooleanLimits, PairedBooleanPatches, QualifiedPairedRootBoundary};
