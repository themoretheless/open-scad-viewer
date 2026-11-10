//! Material ownership for one embedded paired root shell, after independent
//! original-domain flux proves its global sign and requested volume accuracy.
use super::*;
pub struct PairedRootSolidAdmission {
    report: RootBoundaryVolumeReport,
    solid: Option<QualifiedPairedRootSolid>,
}
impl PairedRootSolidAdmission {
    pub fn report(&self) -> &RootBoundaryVolumeReport {
        &self.report
    }
    pub fn solid(&self) -> Option<&QualifiedPairedRootSolid> {
        self.solid.as_ref()
    }
    pub fn into_solid(self) -> Option<QualifiedPairedRootSolid> {
        self.solid
    }
}
pub struct QualifiedPairedRootSolid {
    boundary: Arc<QualifiedPairedRootBoundary>,
    limits: RootBoundaryVolumeLimits,
    report: RootBoundaryVolumeReport,
    volume: [f64; 2],
    reversed: bool,
    model: MixedBrep,
}
impl QualifiedPairedRootSolid {
    pub fn boundary(&self) -> &Arc<QualifiedPairedRootBoundary> { &self.boundary }
    pub fn model(&self) -> &MixedBrep {
        &self.model
    }
    pub fn volume(&self) -> [f64; 2] {
        self.volume
    }
    pub fn volume_proof(&self) -> &RootBoundaryVolumeReport {
        &self.report
    }
    pub fn outward_reversed(&self) -> bool {
        self.reversed
    }
    pub fn material_admitted(&self) -> bool {
        true
    }
    pub fn genus(&self) -> usize {
        self.boundary.genus()
    }
    pub fn reverify(&self) -> Result<()> {
        self.boundary.reverify()?;
        let fresh = self
            .boundary
            .admit_material(self.limits)?
            .into_solid()
            .ok_or_else(|| unproved("Paired root material cannot be re-established"))?;
        if fresh.volume != self.volume
            || fresh.reversed != self.reversed
            || fresh.report != self.report
        {
            return Err(unproved("Paired root material volume/orientation changed"));
        }
        Ok(())
    }
}
impl QualifiedPairedRootBoundary {
    /// A complete connected embedded boundary bounds one finite material region.
    /// Independent signed flux chooses the global outward sign. No caller scalar
    /// or input-body orientation can construct this immutable output owner.
    pub fn admit_material(
        self: &Arc<Self>,
        limits: RootBoundaryVolumeLimits,
    ) -> Result<PairedRootSolidAdmission> {
        let report = self.volume_bounds(limits)?;
        let signed = report.signed_bounds;
        if !report.accuracy_proven || signed.is_none_or(|b| b[0] <= 0. && b[1] >= 0.) {
            return Ok(PairedRootSolidAdmission {
                report,
                solid: None,
            });
        }
        let signed = signed.unwrap();
        let reversed = signed[1] < 0.;
        let volume = if reversed {
            [-signed[1], -signed[0]]
        } else {
            signed
        };
        let mut topology = self.model.topology().clone();
        if reversed {
            for u in topology.shells.iter_mut().flat_map(|s| &mut s.faces) {
                u.reversed = !u.reversed;
            }
        }
        let model = MixedBrep::from_topology(topology)?;
        let solid = QualifiedPairedRootSolid {
            boundary: self.clone(),
            limits,
            report: report.clone(),
            volume,
            reversed,
            model,
        };
        Ok(PairedRootSolidAdmission {
            report,
            solid: Some(solid),
        })
    }
}
