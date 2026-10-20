//! Transfer opposite natural plane fibers through unchanged owned source uses.
use super::*;
pub struct ProfilePairedContact<'x, 'b> {
    owners: [&'x ShellFace; 2],
    source: crate::source_paired_fiber_contact::Certificate,
    image: crate::source_profile_boundary_identity::PlaneImage<'x, 'b>,
    shared: Vec<[Address; 2]>,
}
impl ProfilePairedContact<'_, '_> {
    pub fn owners(&self) -> [&ShellFace; 2] {
        self.owners
    }
    pub fn source(&self) -> &crate::source_paired_fiber_contact::Certificate {
        &self.source
    }
    pub fn image(&self) -> &crate::source_profile_boundary_identity::PlaneImage<'_, '_> {
        &self.image
    }
    pub fn shared(&self) -> &[[Address; 2]] {
        &self.shared
    }
}
pub struct ProfilePairedReport<'x, 'b> {
    pub certificate: Option<ProfilePairedContact<'x, 'b>>,
    pub exact_work: u64,
    pub driver_cells: usize,
    pub reason: &'static str,
}
impl<'x, 'o, 'i, 'f, 'a, 's, 'p, 'b> ExpressionShell<'x, 'o, 'i, 'f, 'a, 's, 'p, 'b> {
    pub fn qualify_profile_paired_contact(
        &self,
        faces: [usize; 2],
        max_work: u64,
        max_driver: usize,
    ) -> Result<ProfilePairedReport<'_, 'b>> {
        if faces[0] == faces[1]
            || faces.iter().any(|&i| i >= self.faces.len())
            || max_work > 100000000
            || max_driver > 100000
        {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Bound paired profile owners and native work",
            ));
        }
        let mut out = ProfilePairedReport {
            certificate: None,
            exact_work: 0,
            driver_cells: 0,
            reason: "profile-paired-face-kind-unproven",
        };
        let (original, side) = match faces.map(|i| self.faces[i].kind()) {
            [ShellFaceKind::Retained(_), ShellFaceKind::Original(a)] => (a, 0),
            [ShellFaceKind::Original(a), ShellFaceKind::Retained(_)] => (a, 1),
            _ => return Ok(out),
        };
        if self.equivalent_original_face(faces[1 - side]) != Some(original) {
            out.reason = "profile-paired-original-contour-unproven";
            return Ok(out);
        }
        let root = self.assembly.patch.rails.face.boundary.rooted[0];
        let shell = root.restricted.proposal.body.geometry().shell();
        let profile = root.restricted.proposal.profile(root.face).unwrap();
        let mut ids = [root.face; 2];
        ids[1 - side] = original;
        let pairs = self
            .pairs
            .iter()
            .filter(|p| p.uses.iter().all(|a| faces.contains(&a.face)))
            .collect::<Vec<_>>();
        if pairs.is_empty() {
            out.reason = "profile-paired-current-edge-unproven";
            return Ok(out);
        }
        if max_work == 0 || max_driver == 0 {
            out.reason = "profile-paired-work-limit";
            return Ok(out);
        }
        let report = crate::source_paired_fiber_contact::certify(shell, ids, max_work, max_driver)?;
        out.exact_work = report.exact_work;
        out.driver_cells = report.driver_cells;
        out.reason = report.reason;
        let Some(source) = report.certificate else {
            return Ok(out);
        };
        if source.fibers()[side].surface() != profile.source() {
            out.reason = "profile-paired-source-controls-unproven";
            return Ok(out);
        }
        let mut shared = Vec::new();
        let mut original_edges = Vec::new();
        for pair in pairs {
            let ShellPairKind::Original(index) = pair.kind else {
                out.reason = "profile-paired-root-ended-cover-unproven";
                return Ok(out);
            };
            let edge = &shell.edges()[index];
            // The native source certificate below must be covered by exact
            // current edge definitions and both unchanged source fragments.
            // Canonical carrier tails do not enlarge either retained use;
            // requiring the whole canonical carrier would reject valid
            // affine partial charts without adding contact authority.
            let definitions = edge.uses().each_ref().map(|f| f.definition());
            for (slot, address) in pair.uses.iter().copied().enumerate() {
                let ShellUse::Source(fragment) =
                    &self.faces[address.face].loops[address.wire][address.edge]
                else {
                    out.reason = "profile-paired-source-use-unproven";
                    return Ok(out);
                };
                if definitions[slot] != fragment.definition() {
                    out.reason = "profile-paired-current-source-use-unproven";
                    return Ok(out);
                }
            }
            original_edges.push(edge.definition());
            shared.push(pair.uses);
        }
        if source
            .edges()
            .iter()
            .any(|edge| !original_edges.contains(&edge.definition()))
        {
            out.reason = "profile-paired-complete-contact-edge-cover-unproven";
            return Ok(out);
        }
        if out.exact_work == max_work {
            out.reason = "profile-paired-work-limit";
            return Ok(out);
        }
        let fiber = &source.fibers()[side];
        let (axis, upper) = fiber.boundary();
        let report = crate::source_profile_boundary_identity::certify_plane_image(
            profile,
            fiber.plane(),
            axis,
            upper,
            max_work - out.exact_work,
        )?;
        out.exact_work += report.exact_work;
        out.reason = report.reason;
        let Some(image) = report.certificate else {
            return Ok(out);
        };
        if !image.image().poles().is_empty() || image.image().side() != fiber.side() {
            out.reason = "profile-paired-extra-plane-image-unproven";
            return Ok(out);
        }
        // Opposite native sides confine cross-face contacts to the common
        // natural plane fibers. Positive factors preserve strict interior
        // signs; exact boundary identity preserves the source shared curve.
        // Every source contact edge is fully retained and currently owned.
        out.certificate = Some(ProfilePairedContact {
            owners: faces.map(|i| &self.faces[i]),
            source,
            image,
            shared,
        });
        out.reason = "profile-owned-opposite-plane-fiber-contact-qualified";
        Ok(out)
    }
}
