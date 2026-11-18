//! Re-admit original body data through incidence, embedding and signed volume.
use super::*;
fn embedding(l: &crate::source_shell_geometry::Limits) -> crate::source_shell_geometry::Limits {
    crate::source_shell_geometry::Limits {
        tolerance_uv: l.tolerance_uv,
        corners: l.corners,
        spans: l.spans,
        linear_cells: l.linear_cells,
        pairs: l.pairs,
        exact_work: l.exact_work,
        driver_cells: l.driver_cells,
    }
}
fn volume(l: &Limits) -> Limits {
    Limits {
        axis: l.axis,
        origin: l.origin,
        absolute_error: l.absolute_error,
        tolerance_uv: l.tolerance_uv,
        cells: l.cells,
        spans: l.spans,
        domain_cells: l.domain_cells,
    }
}
impl Body {
    /// Cached incidence, embedding/volume claims and orientation are not reused.
    /// Only original inverse-shear inputs select a freshly checked inverse path.
    pub fn requalify(
        &self,
        limits: &crate::source_body_restore::Limits,
    ) -> crate::Result<crate::linear_canal_body::Report> {
        let incidence = self
            .geometry
            .shell()
            .requalify(&limits.shell)
            .map_err(|e| crate::Error::new(e.code, e.message))?;
        match self.geometry.inverse_shear() {
            None => crate::linear_canal_body::from_incidence(
                incidence,
                embedding(&limits.embedding),
                volume(&limits.volume),
            ),
            Some(raw) => crate::linear_canal_body::from_incidence_using(
                incidence,
                |shell| {
                    crate::source_shell_geometry::qualify_with_inverse_shear(
                        shell,
                        embedding(&limits.embedding),
                        raw.axes(),
                        raw.coefficient(),
                        &limits.shell.regions,
                    )
                },
                volume(&limits.volume),
            ),
        }
    }
}
impl Body<crate::source_projective_edge::Edge> {
    /// Original rational shell maps are rechecked before all contact and volume gates.
    pub fn requalify(
        &self,
        limits: &crate::source_body_restore::Limits,
    ) -> Result<crate::source_projective_body_restore::Report> {
        let mut incidence = self.geometry.shell().requalify(&limits.shell)?;
        let shell = incidence.shell.take();
        let mut out = crate::source_projective_body_restore::Report {
            incidence,
            embedding: None,
            volume: None,
        };
        let Some(shell) = shell else {
            return Ok(out);
        };
        let mut geometry =
            crate::source_shell_geometry::qualify_projective(shell, embedding(&limits.embedding))?;
        let admitted = geometry.geometry.take();
        out.embedding = Some(geometry);
        if let Some(admitted) = admitted {
            out.volume = Some(super::qualify(admitted, volume(&limits.volume))?);
        }
        Ok(out)
    }
}
#[cfg(test)]
fn poison<E>(body: &mut Body<E>) {
    body.signed_volume = [-1e100, 1e100];
    body.volume = [999., 1000.];
    body.reverse_orientation = !body.reverse_orientation;
}
#[cfg(test)]
impl Body {
    pub(crate) fn assert_native_body_cache_revalidation(
        &self,
        limits: &crate::source_body_restore::Limits,
    ) {
        let mut report = self.requalify(limits).unwrap();
        let mut body = report.volume.as_mut().unwrap().body.take().unwrap();
        let expected = (body.signed_volume, body.volume, body.reverse_orientation);
        poison(&mut body);
        let fresh = body.requalify(limits).unwrap();
        let fresh = fresh.body().expect(fresh.reason());
        assert_eq!(
            (fresh.signed_volume, fresh.volume, fresh.reverse_orientation),
            expected
        );
    }
}
#[cfg(test)]
impl Body<crate::source_projective_edge::Edge> {
    pub(crate) fn assert_native_body_cache_revalidation(
        &self,
        limits: &crate::source_body_restore::Limits,
    ) {
        let mut report = self.requalify(limits).unwrap();
        let mut body = report.volume.as_mut().unwrap().body.take().unwrap();
        let expected = (body.signed_volume, body.volume, body.reverse_orientation);
        poison(&mut body);
        let fresh = body.requalify(limits).unwrap();
        let fresh = fresh.body().expect(fresh.reason());
        assert_eq!(
            (fresh.signed_volume, fresh.volume, fresh.reverse_orientation),
            expected
        );
    }
}
