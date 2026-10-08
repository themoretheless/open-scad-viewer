//! Fresh original planar contact authority tied to unchanged expression faces.
use super::*;
pub struct OriginalPlanarContact<'x> {
    faces: [usize; 2],
    owners: [&'x ShellFace; 2],
    source: crate::source_allowed_contact::Certificate,
}
impl OriginalPlanarContact<'_> {
    pub fn faces(&self) -> [usize; 2] {
        self.faces
    }
    pub fn owners(&self) -> [&ShellFace; 2] {
        self.owners
    }
    pub fn source(&self) -> &crate::source_allowed_contact::Certificate {
        &self.source
    }
}
pub struct OriginalContactReport<'x> {
    pub certificate: Option<OriginalPlanarContact<'x>>,
    pub exact_work: u64,
    pub spans: usize,
    pub reason: &'static str,
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    /// Fresh proof is transferred only when both final contours are identical
    /// to their owned source region. Subdivided/retained faces remain pending.
    pub fn qualify_original_planar_contact(
        &self,
        faces: [usize; 2],
        max_work: u64,
        max_spans: usize,
    ) -> Result<OriginalContactReport<'_>> {
        if faces[0] == faces[1]
            || faces.iter().any(|i| *i >= self.faces.len())
            || max_work > 100000000
            || max_spans > 100000
        {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Require distinct owned faces and bounded contact work",
            ));
        }
        let mut out = OriginalContactReport {
            certificate: None,
            exact_work: 0,
            spans: 0,
            reason: "expression-original-contact-face-changed",
        };
        let owners = faces.map(|i| &self.faces[i]);
        let [ShellFaceKind::Original(a), ShellFaceKind::Original(b)] = owners.map(|f| f.kind)
        else {
            return Ok(out);
        };
        let root = self.assembly.patch.rails.face.boundary.rooted[0];
        let shell = root.restricted.proposal.body.geometry().shell();
        for (face, original) in owners.iter().zip([a, b]) {
            let expected = shell.regions().unwrap()[original].loops();
            if face.loops.len() != expected.len() {
                return Ok(out);
            }
            for (wire, source) in face.loops.iter().zip(expected) {
                if wire.len()!=source.len() || wire.iter().zip(source).any(|(use_,f)|
                    !matches!(use_,ShellUse::Source(actual) if actual.definition()==f.definition())) {return Ok(out);}
            }
        }
        if max_work == 0 || max_spans == 0 {
            out.reason = "expression-original-contact-work-limit";
            return Ok(out);
        }
        let report = crate::source_allowed_contact::certify(shell, [a, b], max_work, max_spans)?;
        out.exact_work = report.exact_work;
        out.spans = report.spans;
        out.reason = report.reason;
        let Some(source) = report.certificate else {
            return Ok(out);
        };
        out.certificate = Some(OriginalPlanarContact {
            faces,
            owners,
            source,
        });
        out.reason = "expression-unchanged-planar-contact-qualified";
        Ok(out)
    }
}
