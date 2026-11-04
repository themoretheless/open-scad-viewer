//! Sufficient Jordan admission for interior affine original SS graphs.
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
    joints: Vec<ss::QualifiedOriginalSsJoint>,
    limits: InteriorLoopLimits,
}
impl QualifiedInteriorSsLoop {
    pub fn cuts(&self) -> &[ClosedCutCandidate; 2] {
        &self.cuts
    }
    pub fn joints(&self) -> &[ss::QualifiedOriginalSsJoint] {
        &self.joints
    }
    pub fn reverify(&self) -> Result<()> {
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
fn neighborhood(edge: &ss::OriginalSsRootEdge, limits: ss::OriginalSsCurveLimits) -> Result<()> {
    let bounds = edge.enclose(limits)?;
    let original = bounds
        .uv
        .map(|d| d.map(|x| num_rational::BigRational::from_float(x).unwrap()));
    let window = edge.curve().qualify_graph_window(original)?;
    let d = edge.curve().coordinate_domain();
    for k in 0..4 {
        let q = |x| num_rational::BigRational::from_float(x).unwrap();
        if !(q(d[k][0]) < window.local_window()[k][0] && window.local_window()[k][1] < q(d[k][1])) {
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
        let uv = bounds.uv[k];
        if !(knots[p] < uv[0] && uv[1] < knots[n])
            || knots[p..=n].iter().any(|t| uv[0] <= *t && *t <= uv[1])
        {
            return Err(unproved(
                "Loop needs explicit one-sided knot/seam/pole neighborhood admission",
            ));
        }
    }
    Ok(())
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
    for edge in wire.edges() {
        neighborhood(edge, limits.geometry)?;
    }
    let joints = wire.qualify_joints(limits.joint_depth, limits.geometry)?;
    let a = split_closed_contour_candidate(first, first_face, wire, 0, seeds[0], limits.cuts[0])?;
    let b = split_closed_contour_candidate(second, second_face, wire, 1, seeds[1], limits.cuts[1])?;
    Ok(QualifiedInteriorSsLoop {
        cuts: [a, b],
        joints,
        limits,
    })
}

mod census;
pub use census::{PairedRootSolidAdmission, QualifiedPairedRootSolid, RootCavityLimits,RootCavityCoverageWork,QualifiedRootCavitySolid,RootSurfaceMaterialLimits,QualifiedRootSurfaceMaterialBox,RootMaterialBoxLimits,RootMaterialBoxWork,QualifiedRootMaterialBox,RootSolidPointLimits, RootSolidPointReport, RootSolidRayReport, RootBoundaryVolumeLimits, RootBoundaryVolumeReport, QualifiedPairedRootBoundary, QualifiedArrangementMaterialRectangle, CensusArrangementLimits, CensusFaceArrangement, CensusRectangleReport, QualifiedCensusMaterialRectangle, InteriorCensusAnchor, InteriorCensusLimits, QualifiedInteriorSsCensus, qualify_interior_ss_census};
