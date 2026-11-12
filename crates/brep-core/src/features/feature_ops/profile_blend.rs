//! Closed prism blends on ORIGINAL curved rational profile supports.
//! Not a mesh bevel or an open skin: profile trim, new face and both cap trims
//! are authored together by the owning exact-extrusion constructor.
use crate::{Error, Model, Result, solid_audit, watertight};
use nurbs_core::{
    curve::Curve, offset::profile_corner, surface_g1_pcurves, surface_join::Decision,
};
fn refused(message: &str) -> Error {
    Error::new("BREP_CURVED_PROFILE_BLEND_REFUSED", message)
}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub normal_tolerance: f64,
    /// Shared length-inversion and downstream radius/seam/closure cells.
    /// Recognition and profile construction keep their existing owner bounds.
    pub max_cells: usize,
    pub max_exact: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            normal_tolerance: 1e-5,
            max_cells: 100000,
            max_exact: 1000000,
        }
    }
}
pub struct Feature {
    model: Model,
    face: usize,
    corner: profile_corner::Corner,
    joins: [usize; 2],
    seams: Vec<surface_g1_pcurves::Report>,
    radius: Option<nurbs_core::moving_radius::Certificate>,
    closure: watertight::QualifiedReport,
    audit: solid_audit::SolidAuditCertificate,
    cells: usize,
    exact_work: u64,
}
impl Feature {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn into_model(self) -> Model {
        self.model
    }
    pub fn blend_face(&self) -> usize {
        self.face
    }
    pub fn join_edges(&self) -> [usize; 2] {
        self.joins
    }
    pub fn corner(&self) -> &profile_corner::Corner {
        &self.corner
    }
    pub fn tangent_seams(&self) -> &[surface_g1_pcurves::Report] {
        &self.seams
    }
    pub fn radius_certificate(&self) -> Option<&nurbs_core::moving_radius::Certificate> {
        self.radius.as_ref()
    }
    pub fn closure(&self) -> &watertight::QualifiedReport {
        &self.closure
    }
    pub fn audit(&self) -> &solid_audit::SolidAuditCertificate {
        &self.audit
    }
    pub fn proof_work(&self) -> (usize, u64) {
        (self.cells, self.exact_work)
    }
}
fn profile(
    source: &Model,
    loop_id: usize,
    preceding: usize,
    limits: Limits,
) -> Result<crate::prism::ProfilePrism> {
    source.validate()?;
    if !limits.normal_tolerance.is_finite()
        || limits.normal_tolerance <= 0.
        || limits.normal_tolerance > 2.
        || !(256..=100000).contains(&limits.max_cells)
        || !(1..=1000000).contains(&limits.max_exact)
    {
        return Err(refused("Invalid normal tolerance or proof work"));
    }
    let p = crate::prism::recognize(source)?
        .ok_or_else(|| refused("Need a retained rational XY prism"))?;
    if loop_id >= p.loops.len() || preceding >= p.loops[loop_id].len() || p.loops[loop_id].len() < 3
    {
        return Err(refused(
            "Need an original profile junction and at least three source curves",
        ));
    }
    Ok(p)
}
/// Constant-radius convex round on two arbitrary retained rational profile
/// curves of an XY prism. Original supports are trimmed at solved contacts;
/// the circular side and planar end-cap loops are reauthored as a CLOSED body.
/// Actual G1 joins and whole-side radius must freshly qualify. Concave corners,
/// non-prismatic support pairs and intersecting multi-edge networks refuse.
pub fn round_corner(
    source: &Model,
    loop_id: usize,
    preceding: usize,
    radius: f64,
    limits: Limits,
) -> Result<Feature> {
    let p = profile(source, loop_id, preceding, limits)?;
    let next = (preceding + 1) % p.loops[loop_id].len();
    let corner = profile_corner::round(
        &p.loops[loop_id][preceding],
        &p.loops[loop_id][next],
        radius,
        source.tolerance_mm / 32.,
    )?;
    assemble(source, p, loop_id, preceding, corner, limits)
}
/// Distance/distance chamfer with setbacks measured on the ORIGINAL curved
/// support profiles, not along their control polygons or display polylines.
pub fn chamfer_corner(
    source: &Model,
    loop_id: usize,
    preceding: usize,
    distances: [f64; 2],
    limits: Limits,
) -> Result<Feature> {
    let p = profile(source, loop_id, preceding, limits)?;
    let next = (preceding + 1) % p.loops[loop_id].len();
    let corner = profile_corner::chamfer(
        &p.loops[loop_id][preceding],
        &p.loops[loop_id][next],
        distances,
        source.tolerance_mm / 4.,
        limits.max_cells / 2,
    )?;
    assemble(source, p, loop_id, preceding, corner, limits)
}
fn assemble(
    source: &Model,
    mut p: crate::prism::ProfilePrism,
    loop_id: usize,
    preceding: usize,
    corner: profile_corner::Corner,
    limits: Limits,
) -> Result<Feature> {
    let next = (preceding + 1) % p.loops[loop_id].len();
    let mut wire = Vec::new();
    for (i, c) in p.loops[loop_id].iter().enumerate() {
        if i == preceding {
            wire.push(corner.incoming().clone());
            wire.push(corner.bridge().clone());
        } else if i == next {
            wire.push(corner.outgoing().clone());
        } else {
            wire.push(c.clone());
        }
    }
    p.loops[loop_id] = wire;
    let mut model = crate::prism::extrude(&p.loops, p.z_min, p.z_max)?;
    model.tolerance_mm = source.tolerance_mm;
    model.inherit_topology_ids(&[source]);
    model.validate()?;
    let mut joins = Vec::new();
    let mut sides = Vec::<Vec<(usize, usize, usize)>>::new();
    for contact in corner.contacts() {
        let candidates = model
            .edges
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                let a = model.vertices[e.vertices[0]].point;
                let b = model.vertices[e.vertices[1]].point;
                (a[2] - b[2]).abs() > (p.z_max - p.z_min) / 2.
                    && [a, b].iter().all(|v| {
                        (v[0] - contact[0]).hypot(v[1] - contact[1]) <= source.tolerance_mm
                    })
            })
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        if candidates.len() != 1 {
            return Err(refused(
                "Blend contact does not own exactly one longitudinal shared edge",
            ));
        }
        let edge = candidates[0];
        joins.push(edge);
        let mut owners = Vec::new();
        for (fi, f) in model.faces.iter().enumerate() {
            for w in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
                for (ci, c) in model.loops[w].coedges.iter().enumerate() {
                    if c.edge == edge {
                        owners.push((fi, w, ci));
                    }
                }
            }
        }
        if owners.len() != 2 {
            return Err(refused("Blend join must have two actual face uses"));
        }
        sides.push(owners);
    }
    let common = sides[0]
        .iter()
        .map(|p| p.0)
        .filter(|f| sides[1].iter().any(|p| p.0 == *f))
        .collect::<Vec<_>>();
    if common.len() != 1 {
        return Err(refused("Cannot identify one authored blend side"));
    }
    let face = common[0];
    let mut cells = corner.length_cells();
    let mut exact_work = 0;
    let mut radius = None;
    let mut seams = Vec::new();
    if let (Some(center), Some(r)) = (corner.center(), corner.radius()) {
        let original = &model.faces[face].surface;
        let proof_surface = nurbs_core::surface_edit::transpose(original)?;
        let centers = Curve::from_polyline(vec![
            vec![center[0], center[1], p.z_min],
            vec![center[0], center[1], p.z_max],
        ])?;
        let law = Curve::from_polyline(vec![vec![r, 0.], vec![r, 0.]])?;
        let mut proof = nurbs_core::moving_radius::qualify(
            &proof_surface,
            &centers,
            &law,
            source.tolerance_mm,
            (limits.max_cells - cells) / 4,
            limits.max_exact,
        )?;
        cells += proof.cells;
        exact_work += proof.work;
        radius = Some(
            proof
                .certificate
                .take()
                .ok_or_else(|| refused("Actual whole blend side radius remains unproven"))?,
        );
        for i in 0..2 {
            if cells >= limits.max_cells || exact_work >= limits.max_exact {
                return Err(refused("Blend tangent proof allowance exhausted"));
            }
            let uses = &sides[i];
            let (fa, wa, ia) = uses[0];
            let (fb, wb, ib) = uses[1];
            let ca = &model.loops[wa].coedges[ia];
            let cb = &model.loops[wb].coedges[ib];
            let proof = surface_g1_pcurves::inspect(
                surface_g1_pcurves::PcurveSeam {
                    curve: &model.edges[joins[i]].curve,
                    pcurves: [&ca.pcurve, &cb.pcurve],
                    surfaces: [&model.faces[fa].surface, &model.faces[fb].surface],
                    reversed: [ca.reversed, cb.reversed],
                },
                source.tolerance_mm,
                limits.normal_tolerance,
                (limits.max_cells - cells) / (3 - i),
                limits.max_exact - exact_work,
            )?;
            cells += proof.cells;
            exact_work += proof.work();
            if !proof.positions.proven
                || proof.decision != Decision::WithinTolerance
                || !proof.regularity_proven
            {
                return Err(refused(
                    "Actual original-profile/blend G1 join remains unproven",
                ));
            }
            seams.push(proof);
        }
    }
    if cells >= limits.max_cells || exact_work >= limits.max_exact {
        return Err(refused(
            "No work remains for full closed boundary agreement",
        ));
    }
    let closure = watertight::inspect_with_exact(
        &model,
        limits.max_cells - cells,
        limits.max_exact - exact_work,
    )?;
    cells += closure.closure.agreement.cells;
    exact_work += closure.exact.work;
    if closure.closure.status != watertight::Closure::WithinTolerance {
        return Err(refused("Rounded prism is not continuously closed"));
    }
    let audit = solid_audit::audit_solid(&model)?;
    Ok(Feature {
        model,
        face,
        corner,
        joins: [joins[0], joins[1]],
        seams,
        radius,
        closure,
        audit,
        cells,
        exact_work,
    })
}
