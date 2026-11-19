//! Exact compression of selected trimmed face carriers in one transaction.
use crate::{Model, Result, invalid, require};
use nurbs_core::editing::exact_surface_knot_removal::compress_tensor_surface_batch;
use std::collections::BTreeSet;

impl Model {
    /// Simplify clamped nonperiodic face surfaces while preserving their UV
    /// maps, trim curves and topology identities. On budget exhaustion return
    /// None without publishing partial edits. Duplicate face indices are invalid.
    pub fn compress_face_surfaces(&self, faces: &[usize], max_work: u64) -> Result<Option<Self>> {
        require(
            !faces.is_empty() && faces.len() <= self.faces.len(),
            "Invalid face selection",
        )?;
        require(
            (1..=cad_predicates::MAX_WORK).contains(&max_work),
            "Invalid compression work limit",
        )?;
        let mut selected = BTreeSet::new();
        for &index in faces {
            require(
                index < self.faces.len() && selected.insert(index),
                "Invalid or repeated face index",
            )?;
        }
        self.validate()?;
        let mut remaining = max_work;
        let mut replacements = Vec::new();
        for &index in faces {
            if remaining == 0 {
                return Ok(None);
            }
            let source = &self.faces[index].surface;
            let batch = compress_tensor_surface_batch(std::slice::from_ref(source), remaining)?;
            remaining -= batch.work_used;
            let Some(mut patches) = batch.patches else {
                return Ok(None);
            };
            let patch = patches
                .pop()
                .ok_or_else(|| invalid("Face compression produced no chart"))?;
            replacements.push((index, patch.surface));
        }
        let mut model = self.clone();
        for (index, surface) in replacements {
            model.faces[index].surface = surface;
        }
        model.validate()?;
        Ok(Some(model))
    }
}
