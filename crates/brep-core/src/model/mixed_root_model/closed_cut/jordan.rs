//! Sufficient Jordan admission for interior affine original SS graphs,
//! including loops crossing interior knot lines at certified knot joints.
use super::*;
#[derive(Clone, Copy)]
pub struct InteriorLoopLimits {
    pub cuts: [ClosedCutLimits; 2],
    pub joint_depth: usize,
    /// Allowance per joint search and per whole-edge neighborhood obligation.
    pub geometry: ss::OriginalSsCurveLimits,
}
impl Default for InteriorLoopLimits {
    fn default() -> Self {
        Self {
            cuts: [Default::default(); 2],
            joint_depth: 12,
            geometry: Default::default(),
        }
    }
}
pub struct QualifiedInteriorSsLoop {
    cuts: [ClosedCutCandidate; 2],
    joints: Vec<ss::QualifiedOriginalSsWireJoint>,
    cycle: Option<ss::OrderedClosedFiberCycle>,
    limits: InteriorLoopLimits,
}
impl QualifiedInteriorSsLoop {
    /// Build a paired finite-face cut from an exact closed cycle. The
    /// interior winding witnesses are searched independently in both UV charts.
    pub fn qualify_from_cycle(
        sources: [(&crate::closed_solid::QualifiedClosedSolid, usize); 2],
        cycle: &ss::OrderedClosedFiberCycle,
        limits: InteriorLoopLimits,
        seed_cells_per_support: usize,
    ) -> Result<Self> {
        if !(1..=16384).contains(&seed_cells_per_support) {
            return Err(unproved("Invalid closed cut seed search allowance"));
        }
        let wire = cycle.to_root_wire(limits.geometry)?;
        let mut seeds = [[0.; 2]; 2];
        for (support, &(source, face)) in sources.iter().enumerate() {
            seeds[support] = find_cycle_cut_seed(
                source.model(), face, &wire, support, limits.cuts[support], seed_cells_per_support,
            )?;
        }
        let [(first, first_face), (second, second_face)] = sources;
        injective(first, first_face)?;
        injective(second, second_face)?;
        // The cycle already owns the regular embedded branch and its entire
        // first-return traversal. Reuse that proof at native chart boundaries.
        let joints = wire.qualify_knot_aware_joints(limits.joint_depth, limits.geometry)?;
        let a = split_closed_contour_candidate(first, first_face, &wire, 0, seeds[0], limits.cuts[0])?;
        let b = split_closed_contour_candidate(second, second_face, &wire, 1, seeds[1], limits.cuts[1])?;
        Ok(Self { cuts: [a, b], joints, cycle: Some(cycle.clone()), limits })
    }

    pub fn cuts(&self) -> &[ClosedCutCandidate; 2] {
        &self.cuts
    }
    pub fn joints(&self) -> &[ss::QualifiedOriginalSsWireJoint] {
        &self.joints
    }
    pub fn reverify(&self) -> Result<()> {
        if let Some(cycle) = &self.cycle {
            cycle.reverify()?;
            for cut in &self.cuts {
                cut.reverify()?;
            }
            return Ok(());
        }
        let [a, b] = &self.cuts;
        a.source.reverify()?;
        b.source.reverify()?;
        qualify_paired_interior_loop(
            &a.source,
            a.face,
            &b.source,
            b.face,
            &a.wire,
            [a.seed, b.seed],
            self.limits,
        )
        .map(|_| ())
    }
}
fn find_cycle_cut_seed(
    model: &crate::Model,
    face: usize,
    wire: &ss::OriginalSsRootWire,
    support: usize,
    limits: ClosedCutLimits,
    cells: usize,
) -> Result<[f64; 2]> {
    let surface = &model.faces.get(face)
        .ok_or_else(|| unproved("Cycle cut source face is absent"))?.surface;
    let domain = crate::face_domain::FaceDomain::new(model, face, limits.tolerance_uv)?;
    let native = crate::boolean_robustness::surface_uv_domains(surface);
    let mut pending = std::collections::VecDeque::from([(native, 0)]);
    let mut domain_cells = limits.placement_domain_cells;
    for _ in 0..cells {
        let Some((mut cell, axis)) = pending.pop_front() else { break };
        let seed = cell.map(|[lo, hi]| lo * 0.5 + hi * 0.5);
        if let Ok(winding) = ss::qualify_original_ss_winding(
            wire, support, seed, limits.per_boundary_winding,
        ) {
            if matches!(winding.winding(), -1 | 1) {
                if domain_cells == 0 {
                    return Err(unproved("Cycle cut seed domain work exhausted"));
                }
                let classification = domain.classify(seed.map(|x| [x; 2]), domain_cells.min(100000))?;
                domain_cells -= classification.cells;
                if classification.location == Location::Inside {
                    return Ok(seed);
                }
            }
        }
        if cell[axis][0] < seed[axis] && seed[axis] < cell[axis][1] {
            let mut right = cell;
            cell[axis][1] = seed[axis];
            right[axis][0] = seed[axis];
            pending.extend([(cell, 1 - axis), (right, 1 - axis)]);
        }
    }
    Err(unproved(&format!("No interior cycle cut seed proved on support {support}")))
}

fn injective(body: &crate::closed_solid::QualifiedClosedSolid, face: usize) -> Result<()> {
    let f = body
        .geometry()
        .boundary
        .intersections
        .faces
        .faces
        .iter()
        .find(|f| f.face == face)
        .ok_or_else(|| unproved("Original loop parent face has no injectivity evidence"))?;
    if !f.result.as_ref().is_some_and(|r| r.proven)
        && !f.linear.as_ref().is_some_and(|r| r.certified)
    {
        return Err(unproved(
            "Interior loop needs genuine UV injectivity, not quotient aliases",
        ));
    }
    Ok(())
}
/// `knots` lists the exact (coordinate, value) of certified cross-carrier
/// knot joints at this edge's ends. Only there may the edge reach its
/// carrier's span boundary; every other knot, seam or pole is still refused.
fn neighborhood(
    edge: &ss::OriginalSsRootEdge,
    limits: ss::OriginalSsCurveLimits,
    knots_at_ends: &[(usize, f64)],
) -> Result<()> {
    let bounds = edge.enclose(limits)?;
    let original = bounds
        .uv
        .map(|d| d.map(|x| num_rational::BigRational::from_float(x).unwrap()));
    let window = edge.curve().qualify_graph_window(original)?;
    let d = edge.curve().coordinate_domain();
    let q = |x| num_rational::BigRational::from_float(x).unwrap();
    for k in 0..4 {
        let uv = bounds.uv[k];
        // Side 0/1 of this coordinate ends on a certified knot joint.
        let certified = |side: usize| knots_at_ends.iter().any(|(c, t)| *c == k && uv[side] == *t);
        let local = window.local_window()[k].clone();
        if !((q(d[k][0]) < local[0] || certified(0)) && (local[1] < q(d[k][1]) || certified(1))) {
            return Err(unproved(
                "Entire loop edge lacks an interior source graph neighborhood",
            ));
        }
        let sources = edge.curve().source();
        let s = if k < 2 {
            sources.first()
        } else {
            sources.second()
        };
        let (knots, p, n) = if k % 2 == 0 {
            (&s.knots_u, s.degree_u, s.control_points.len())
        } else {
            (&s.knots_v, s.degree_v, s.control_points[0].len())
        };
        if !(knots[p] < uv[0] && uv[1] < knots[n])
            || knots[p..=n].iter().any(|t| {
                uv[0] <= *t && *t <= uv[1] && !((*t == uv[0] && certified(0)) || (*t == uv[1] && certified(1)))
            })
        {
            return Err(unproved(
                "Loop needs explicit one-sided knot/seam/pole neighborhood admission",
            ));
        }
    }
    Ok(())
}
/// Geometry half of paired-loop admission: every literal joint is either a
/// regular single-graph joint or a certified cross-carrier knot joint, and
/// every whole edge has a compact interior graph neighbourhood except at its
/// certified knot ends. Regular joints need one interior graph neighbourhood;
/// knot joints need the two one-sided windows to cover an open neighbourhood
/// with agreeing selectors on the knot line. Seams and poles stay refused.
pub fn qualify_loop_neighborhoods(
    wire: &ss::OriginalSsRootWire,
    joint_depth: usize,
    geometry: ss::OriginalSsCurveLimits,
) -> Result<Vec<ss::QualifiedOriginalSsWireJoint>> {
    if joint_depth > 24 {
        return Err(unproved("Invalid interior-loop joint budget"));
    }
    let joints = wire.qualify_knot_aware_joints(joint_depth, geometry)?;
    let n = wire.edges().len();
    for (i, edge) in wire.edges().iter().enumerate() {
        let mut knots = Vec::new();
        for joint in [&joints[(i + n - 1) % n], &joints[i]] {
            if let Some(k) = joint.knot() {
                let value = num_traits::ToPrimitive::to_f64(k.knot())
                    .ok_or_else(|| unproved("Knot joint value overflow"))?;
                knots.push((k.coordinate(), value));
            }
        }
        neighborhood(edge, geometry, &knots)?;
    }
    Ok(joints)
}
/// Both source supports must be the exact ordered original SS pair. Compact
/// local graph neighborhoods make the root image a connected 1-manifold;
/// injective parent faces embed it in each UV plane. Opposite joint order makes
/// traversal a local covering, and unit original winding forces degree one.
/// This proves the contour simple on both faces, not full intersection census,
/// material selection or closed mixed-body admission.
pub fn qualify_paired_interior_loop(
    first: &crate::closed_solid::QualifiedClosedSolid,
    first_face: usize,
    second: &crate::closed_solid::QualifiedClosedSolid,
    second_face: usize,
    wire: &ss::OriginalSsRootWire,
    seeds: [[f64; 2]; 2],
    limits: InteriorLoopLimits,
) -> Result<QualifiedInteriorSsLoop> {
    if limits.joint_depth > 24 {
        return Err(unproved("Invalid interior-loop joint budget"));
    }
    let a = first
        .model()
        .faces
        .get(first_face)
        .ok_or_else(|| unproved("First loop face is absent"))?;
    let b = second
        .model()
        .faces
        .get(second_face)
        .ok_or_else(|| unproved("Second loop face is absent"))?;
    for edge in wire.edges() {
        if edge.curve().source().first() != &a.surface
            || edge.curve().source().second() != &b.surface
        {
            return Err(unproved(
                "Interior loop has different ordered original supports",
            ));
        }
    }
    injective(first, first_face)?;
    injective(second, second_face)?;
    let joints = qualify_loop_neighborhoods(wire, limits.joint_depth, limits.geometry)?;
    let a = split_closed_contour_candidate(first, first_face, wire, 0, seeds[0], limits.cuts[0])?;
    let b = split_closed_contour_candidate(second, second_face, wire, 1, seeds[1], limits.cuts[1])?;
    Ok(QualifiedInteriorSsLoop {
        cuts: [a, b],
        joints,
        cycle: None,
        limits,
    })
}

mod census;
pub use census::{PairedRootSolidAdmission, QualifiedPairedRootSolid, QualifiedRootBoolean, SeparatedRootBoolean,QualifiedRootShellForest,RootRelationKind,RootRelationLimits,RootRelationWork,QualifiedRootRelation,RootDisjointBodyAssembly, RootShellSubtraction, RootCavityLimits,RootCavityCoverageWork,QualifiedRootCavitySolid,RootSurfaceMaterialLimits,QualifiedRootSurfaceMaterialBox,RootMaterialBoxLimits,RootMaterialBoxWork,QualifiedRootMaterialBox,RootSolidPointLimits, RootSolidPointReport, RootSolidRayReport, RootBoundaryVolumeLimits, RootBoundaryVolumeReport, ClosedSsBooleanLimits, PairedBooleanPatches, QualifiedPairedRootBoundary, QualifiedArrangementMaterialRectangle, CensusArrangementLimits, CensusFaceArrangement, CensusRectangleReport, QualifiedCensusMaterialRectangle, InteriorCensusAnchor, InteriorCensusLimits, ClosedSsCensusLimits, QualifiedInteriorSsCensus, qualify_interior_ss_census};
