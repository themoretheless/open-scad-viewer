//! Complete original pole atlases with uniformly transverse projections.
//! A local Jacobian sign does not by itself prove a globally one-to-one
//! projection of the fan, a differentiable pole graph, or pole G2.
use super::*;
pub struct TransverseLimits {
    pub atlas: FanLimits,
    pub columns: usize,
}
pub struct TransverseBodyPoleAtlas<'a, E: Carrier> {
    atlas: BodyPoleAtlas<'a, E>,
    projections: Vec<(Address, normals::ProjectionCollar<'a>)>,
}
impl<'a, E: Carrier> TransverseBodyPoleAtlas<'a, E> {
    pub fn atlas(&self) -> &BodyPoleAtlas<'a, E> {
        &self.atlas
    }
    pub fn projections(&self) -> &[(Address, normals::ProjectionCollar<'a>)] {
        &self.projections
    }
}
pub struct TransverseReport<'a, E: Carrier> {
    pub certificate: Option<TransverseBodyPoleAtlas<'a, E>>,
    pub exact_work: u64,
    pub cells: usize,
    pub columns: usize,
    pub completed_poles: usize,
    pub uncertain_pole: Option<Address>,
    pub reason: &'static str,
}
/// Freshly recompute the entire private atlas, then prove one common world
/// projection transverse on ALL punctured collars incident at each pole.
/// Coordinate axes come from the proved oriented limit, never caller normals.
/// All columns share one allowance; unfinished proofs yield no body token.
pub fn qualify_body_transverse_pole_atlas<E: Carrier>(
    body: &Body<E>,
    limits: TransverseLimits,
) -> Result<TransverseReport<'_, E>> {
    if !(1..=100000).contains(&limits.columns) {
        return Err(Error::new(
            "BREP_SOURCE_POLE_PROJECTION",
            "Choose bounded original projection columns",
        ));
    }
    let extent = limits.atlas.inward_extent;
    let report = qualify_body_pole_atlas(body, limits.atlas)?;
    let mut out = TransverseReport {
        certificate: None,
        exact_work: report.exact_work,
        cells: report.cells,
        columns: 0,
        completed_poles: 0,
        uncertain_pole: report.uncertain_pole,
        reason: report.reason,
    };
    let Some(atlas) = report.certificate else {
        return Ok(out);
    };
    let shell = body.geometry().shell();
    let Some(regions) = shell.regions() else {
        out.reason = "source-pole-projection-material-unproven";
        return Ok(out);
    };
    let directions = atlas
        .orientations()
        .iter()
        .map(|p| (p.vertex(), (p.axis(), p.positive())))
        .collect::<BTreeMap<_, _>>();
    let mut projections = Vec::new();
    for (address, plane) in atlas.planes().poles() {
        out.uncertain_pole = Some(*address);
        if out.columns == limits.columns {
            out.reason = "source-pole-projection-column-limit";
            return Ok(out);
        }
        let vertex = shell.vertices()[address.face][address.wire][address.edge][0];
        let Some(&(axis, positive)) = directions.get(&vertex) else {
            out.reason = "source-pole-projection-owned-direction-unproven";
            return Ok(out);
        };
        let p =
            plane
                .limit()
                .certify_projection_collar(axis, extent, limits.columns - out.columns)?;
        out.columns += p.columns;
        let Some(p) = p.certificate else {
            out.reason = p.reason;
            return Ok(out);
        };
        if fan::oriented_component(
            p.component_bounds(),
            regions[address.face].chart_winding(),
            body.reverse_orientation(),
        ) != Some(positive)
        {
            out.reason = "source-pole-projection-oriented-sign-unproven";
            return Ok(out);
        }
        projections.push((*address, p));
        out.completed_poles += 1;
    }
    out.certificate = Some(TransverseBodyPoleAtlas { atlas, projections });
    out.uncertain_pole = None;
    out.reason = "source-body-complete-pole-atlas-projection-transverse";
    Ok(out)
}
