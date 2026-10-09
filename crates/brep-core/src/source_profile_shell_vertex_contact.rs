//! Fresh single-point contact for an exact positive-weight retained profile.
use super::*;
pub struct ProfileVertexContact<'x> {
    owners: [&'x ShellFace; 2],
    source: crate::source_vertex_contact::Certificate,
    image: crate::source_corner_plane_image::Certificate,
}
impl ProfileVertexContact<'_> {
    pub fn owners(&self) -> [&ShellFace; 2] {
        self.owners
    }
    pub fn source(&self) -> &crate::source_vertex_contact::Certificate {
        &self.source
    }
    pub fn image(&self) -> &crate::source_corner_plane_image::Certificate {
        &self.image
    }
}
pub struct ProfileVertexReport<'x> {
    pub certificate: Option<ProfileVertexContact<'x>>,
    pub exact_work: u64,
    pub reason: &'static str,
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn qualify_profile_vertex_contact(
        &self,
        faces: [usize; 2],
        max_work: u64,
    ) -> Result<ProfileVertexReport<'_>> {
        if faces[0] == faces[1]
            || faces.iter().any(|&i| i >= self.faces.len())
            || max_work > 100000000
        {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Bound owned profile vertex work",
            ));
        }
        let mut out = ProfileVertexReport {
            certificate: None,
            exact_work: 0,
            reason: "profile-vertex-face-kind-unproven",
        };
        let root = self.assembly.patch.rails.face.boundary.rooted[0];
        let (original, profile_side) = match faces.map(|i| self.faces[i].kind()) {
            [ShellFaceKind::Retained(_), ShellFaceKind::Original(a)] => (a, 0),
            [ShellFaceKind::Original(a), ShellFaceKind::Retained(_)] => (a, 1),
            _ => return Ok(out),
        };
        if self.equivalent_original_face(faces[1 - profile_side]) != Some(original) {
            out.reason = "profile-vertex-original-contour-unproven";
            return Ok(out);
        }
        let shell = root.restricted.proposal.body.geometry().shell();
        let profile = root.restricted.proposal.profile(root.face).unwrap();
        let regions = shell.regions().unwrap();
        if profile.source() != regions[root.face].loops()[0][0].surface() {
            out.reason = "profile-vertex-source-controls-unproven";
            return Ok(out);
        }
        let (rows, columns, pivot) = profile.factors();
        if !pivot.is_finite()
            || pivot <= 0.
            || rows
                .iter()
                .chain(columns)
                .any(|v| !v.is_finite() || *v <= 0.)
        {
            out.reason = "profile-vertex-positive-weights-unproven";
            return Ok(out);
        }
        if max_work == 0 {
            out.reason = "profile-vertex-work-limit";
            return Ok(out);
        }
        let mut ids = [root.face; 2];
        ids[1 - profile_side] = original;
        let report = crate::source_vertex_contact::certify(shell, ids, max_work)?;
        out.exact_work = report.exact_work;
        out.reason = report.reason;
        let Some(source) = report.certificate else {
            return Ok(out);
        };
        // A material hull of the old selected region cannot bound a changed
        // profile. Only the complete raw support's corner plane is transferred.
        if source.material_hulls()[profile_side].is_some() {
            out.reason = "profile-vertex-old-material-hull-unproven";
            return Ok(out);
        }
        let address = source.uses()[profile_side].0;
        let original_fragment = &regions[root.face].loops()[address.wire][address.edge];
        let kept = &self.faces[faces[profile_side]];
        let owns = kept.loops.iter().flatten().any(|u| match u {
            ShellUse::Source(f)
                if f.surface() == profile.source() && f.curve() == original_fragment.curve() =>
            {
                (0..2)
                    .any(|end| crate::source_vertex_contact::corner(f, end) == Some(source.point()))
            }
            _ => false,
        });
        if !owns {
            out.reason = "profile-vertex-retained-corner-unproven";
            return Ok(out);
        }
        if out.exact_work == max_work {
            out.reason = "profile-vertex-work-limit";
            return Ok(out);
        }
        let report = crate::source_corner_plane_image::certify(
            profile.source(),
            source.plane(),
            source.point(),
            max_work - out.exact_work,
        )?;
        out.exact_work += report.exact_work;
        let Some(image) = report.certificate else {
            out.reason = "profile-vertex-single-point-image-unproven";
            return Ok(out);
        };
        // This fresh single-Bezier proof uses only exact control-plane signs,
        // positive Bernstein bases and collapsed zero edges. Exact positive
        // factorized weights retain all three properties and the same corner.
        out.certificate = Some(ProfileVertexContact {
            owners: faces.map(|i| &self.faces[i]),
            source,
            image,
        });
        out.reason = "profile-owned-single-point-contact-qualified";
        Ok(out)
    }
}
