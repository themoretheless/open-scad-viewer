//! Native exact volume admission. Fillet radius/thickness and application
//! publication are deliberately separate from geometric/material admission.
use super::*;
pub struct VolumeBounds {
    minimum: Q,
    maximum: Q,
}
impl VolumeBounds {
    pub fn new(minimum: Q, maximum: Q) -> Result<Self> {
        require(
            [&minimum, &maximum].iter().all(|x| {
                x.denom().is_positive() && x.numer().bits() <= 8192 && x.denom().bits() <= 8192
            }) && minimum >= Q::zero()
                && maximum > Q::zero()
                && minimum <= maximum,
            "Invalid exact volume bounds",
        )?;
        Ok(Self { minimum, maximum })
    }
    pub fn minimum(&self) -> &Q {
        &self.minimum
    }
    pub fn maximum(&self) -> &Q {
        &self.maximum
    }
}
pub struct VolumeQualification {
    embedding: OriginalPairAudit,
    flux: SignedFlux,
}
impl VolumeQualification {
    pub fn embedding(&self) -> &OriginalPairAudit {
        &self.embedding
    }
    pub fn flux(&self) -> &SignedFlux {
        &self.flux
    }
}
pub struct NativeVolume {
    topology: RoofTopology,
    original_shell: ClosedShell,
    qualification: VolumeQualification,
    bounds: VolumeBounds,
}
impl NativeVolume {
    pub fn topology(&self) -> &RoofTopology {
        &self.topology
    }
    pub fn original_shell(&self) -> &ClosedShell {
        &self.original_shell
    }
    pub fn qualification(&self) -> &VolumeQualification {
        &self.qualification
    }
    pub fn volume(&self) -> &Q {
        self.qualification.flux.total()
    }
    pub fn bounds(&self) -> &VolumeBounds {
        &self.bounds
    }
}
impl ClosedShell {
    /// The sign is never repaired by abs()/flipping. For an embedded connected
    /// closed oriented manifold, strictly positive z-flux gives its bounded
    /// material volume with outward orientation. Requested bounds are exact.
    pub fn qualify_volume(
        &self,
        bounds: &VolumeBounds,
        max_pairs: usize,
        b: &mut Budget,
    ) -> Result<VolumeQualification> {
        let top = self.topology();
        require(
            top.shells.len() == 1 && top.shells[0].closed && top.bodies.is_empty(),
            "Volume admission requires one unassigned closed outer shell",
        )?;
        let flux = self.signed_flux(b)?;
        require(
            flux.total() > &Q::zero(),
            "Native shell has nonpositive signed volume or inward orientation",
        )?;
        require(
            flux.total() >= bounds.minimum() && flux.total() <= bounds.maximum(),
            "Native exact volume is outside the requested bounds",
        )?;
        let embedding = self.inspect_original_pairs(max_pairs, b)?;
        require(
            embedding.globally_embedded(),
            "Native shell embedding remains unresolved",
        )?;
        let roof = self.inherited_roof();
        require(
            roof.transition_joins().iter().all(|r| r.within)
                && roof.transition_fields().iter().all(|r| {
                    r.decision == CurvatureDecision::Within
                        && r.unresolved_regions.is_empty()
                        && r.absolute_curvature_upper.is_some_and(|k| k <= 10.)
                })
                && roof
                    .source_fields()
                    .iter()
                    .chain(self.imported_fields())
                    .all(|r| r.absolute_curvature_upper <= 10.),
            "Native volume lost G2 or whole-support curvature admission",
        )?;
        Ok(VolumeQualification { embedding, flux })
    }
    /// Consume immutable source ownership and add the body role only after all
    /// geometric, orientation and exact requested-volume gates have succeeded.
    pub fn admit_volume(
        self,
        bounds: VolumeBounds,
        max_pairs: usize,
        b: &mut Budget,
    ) -> Result<NativeVolume> {
        let qualification = self.qualify_volume(&bounds, max_pairs, b)?;
        let mut topology = self.topology().clone();
        topology.bodies.push(brep_topology::Body {
            outer_shell: 0,
            inner_shells: vec![],
        });
        let geometry = inspect_roof_geometry(&topology, b)?;
        require(
            geometry.open_edges.is_empty(),
            "Native body publication changed boundary closure",
        )?;
        Ok(NativeVolume {
            topology,
            original_shell: self,
            qualification,
            bounds,
        })
    }
}
