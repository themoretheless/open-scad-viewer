//! Whole-support injectivity evidence, separate from between-face contacts.
use super::*;
use nurbs_core::original_graph_transition::{
    PolynomialChartEmbedding, polynomial_surface_embedding,
};
pub struct FaceChartAudit {
    proven: Vec<(usize, PolynomialChartEmbedding)>,
    unresolved: Vec<usize>,
}
impl FaceChartAudit {
    pub fn proven(&self) -> &[(usize, PolynomialChartEmbedding)] {
        &self.proven
    }
    pub fn unresolved(&self) -> &[usize] {
        &self.unresolved
    }
}
impl ClosedShell {
    /// Exact injectivity of complete polynomial supports also covers their
    /// retained trimmed subsets. Failure to find this certificate is unknown,
    /// not evidence of a defect and never a reason to admit a solid.
    pub fn inspect_face_charts(&self, b: &mut Budget) -> Result<FaceChartAudit> {
        let top = self.topology();
        let geometry = inspect_roof_geometry(top, b)?;
        require(
            top.shells.len() == 1 && top.shells[0].closed && geometry.open_edges.is_empty(),
            "Face charts require exact closed shell",
        )?;
        let mut proven = Vec::new();
        let mut unresolved = Vec::new();
        for (i, face) in top.faces.iter().enumerate() {
            let proof = match &face.surface {
                FaceGeometry::Transition(p) => p.inspect_chart_embedding(b)?,
                FaceGeometry::Retained(r) => polynomial_surface_embedding(r.source(), b)?,
                FaceGeometry::Imported(f) => polynomial_surface_embedding(f.source(), b)?,
            };
            if let Some(p) = proof {
                proven.push((i, p));
            } else {
                unresolved.push(i);
            }
        }
        Ok(FaceChartAudit { proven, unresolved })
    }
}
