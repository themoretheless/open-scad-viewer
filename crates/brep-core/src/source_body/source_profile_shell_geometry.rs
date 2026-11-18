//! Join immutable face admission, closed manifold incidence and complete
//! cross-face contact proofs. Material orientation and volume remain separate.
use super::*;

pub struct EmbeddedExpressionGeometry<'g, 'x, 'o, 'i, 'f, 'a, 's, 'p, 'b> {
    shell: &'g ExpressionShell<'x, 'o, 'i, 'f, 'a, 's, 'p, 'b>,
    contacts: &'g ShellContactAudit<'g>,
}
impl EmbeddedExpressionGeometry<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn shell(&self) -> &ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
        self.shell
    }
    pub fn contacts(&self) -> &ShellContactAudit<'_> {
        self.contacts
    }
}
impl<'x, 'o, 'i, 'f, 'a, 's, 'p, 'b> ExpressionShell<'x, 'o, 'i, 'f, 'a, 's, 'p, 'b> {
    /// The assembly owns admitted retained charts and an admitted transition
    /// chart, exact shared edges and checked manifold vertex links. Original
    /// charts may inherit source admission only after complete contour identity.
    /// Diagnostics, a pending-count value or an audit from another shell cannot
    /// establish embedding. This certificate does not admit a solid Body.
    pub fn qualify_embedding<'g>(
        &'g self,
        contacts: &'g ShellContactAudit<'g>,
    ) -> Option<EmbeddedExpressionGeometry<'g, 'x, 'o, 'i, 'f, 'a, 's, 'p, 'b>> {
        if !std::ptr::eq(self.faces.as_slice(), contacts.owners())
            || !contacts.complete_cross_face_contacts()
        {
            return None;
        }
        for (index, face) in self.faces.iter().enumerate() {
            if let ShellFaceKind::Original(source) = face.kind() {
                if self.equivalent_original_face(index) != Some(source) {
                    return None;
                }
            }
        }
        Some(EmbeddedExpressionGeometry { shell: self, contacts })
    }
}
