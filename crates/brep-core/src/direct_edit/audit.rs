use super::*;
use crate::watertight::PatchSetReport;
use sha2::{Digest, Sha256};

pub const DIRECT_EDIT_AUDIT_REFUSED: &str = "BREP_DIRECT_EDIT_AUDIT_REFUSED";

#[derive(Clone, Debug)]
pub struct ShellEulerCheck {
    pub shell: usize,
    pub before: Option<i64>,
    pub after: Option<i64>,
}

impl ShellEulerCheck {
    pub fn ok(&self) -> bool {
        self.before == self.after
    }
}

/// Bounded post-edit evidence over the dirty core and its one-face seam.
/// This is not a replacement for the operation's global solid certificate.
#[derive(Clone, Debug)]
pub struct DirectEditValidationReport {
    pub dirty: DirtyRegion,
    pub seam_faces: BTreeSet<usize>,
    /// Non-degenerate core edges whose use count in core + seam is not two.
    pub non_manifold_edges: Vec<usize>,
    pub euler: Vec<ShellEulerCheck>,
    pub seam: Option<PatchSetReport>,
    pub seam_error: Option<String>,
    /// Local face-pair contacts, mapped back to result-model indices.
    pub interior_contacts: Vec<[usize; 2]>,
    pub self_intersection_proven: bool,
    pub self_intersection_error: Option<String>,
    pub local_geometry_error: Option<String>,
    pub checked_face_pairs: usize,
}

impl DirectEditValidationReport {
    pub fn seam_complete(&self) -> bool {
        match &self.seam {
            None => self.seam_error.is_none(),
            Some(report) => {
                let matched: BTreeSet<usize> = report.matches.iter().flatten().copied().collect();
                report
                    .boundaries
                    .iter()
                    .enumerate()
                    .filter(|(_, boundary)| boundary.model == 0)
                    .all(|(index, _)| matched.contains(&index))
            }
        }
    }

    /// Existing operations may publish with an explicitly unresolved local
    /// self-intersection proof, matching the prior global-audit admission
    /// contour. A detected contact, structural defect or diagnostic error is
    /// always refused. Use `ok()` when a complete local proof is required.
    pub fn publishable(&self) -> bool {
        self.non_manifold_edges.is_empty()
            && self.euler.iter().all(ShellEulerCheck::ok)
            && self.seam_complete()
            && self.interior_contacts.is_empty()
            && self.self_intersection_error.is_none()
            && self.local_geometry_error.is_none()
    }

    /// Strict success: every local check, including absence of intersections,
    /// is proved rather than merely free of a detected defect.
    pub fn ok(&self) -> bool {
        self.publishable() && self.self_intersection_proven
    }
}

fn index(ids: BTreeSet<usize>) -> BTreeMap<usize, usize> {
    ids.into_iter()
        .enumerate()
        .map(|(new, old)| (old, new))
        .collect()
}

/// Extract an open patch while preserving persistent ids and the authoritative
/// naming history needed by `LocallyValidatedModel`.
fn extract_patch(model: &Model, faces: &BTreeSet<usize>) -> Result<(Model, Vec<usize>)> {
    let face_map = index(faces.clone());
    let face_indices = face_map.keys().copied().collect::<Vec<_>>();
    let loops = index(
        face_indices
            .iter()
            .flat_map(|&face| {
                let face = &model.faces[face];
                std::iter::once(face.outer).chain(face.holes.iter().copied())
            })
            .collect(),
    );
    let edges = index(
        loops
            .keys()
            .flat_map(|&wire| model.loops[wire].coedges.iter().map(|c| c.edge))
            .collect(),
    );
    let vertices = index(
        edges
            .keys()
            .flat_map(|&edge| model.edges[edge].vertices)
            .collect(),
    );
    let shells = index(
        model
            .shells
            .iter()
            .enumerate()
            .filter(|(_, shell)| shell.faces.iter().any(|u| face_map.contains_key(&u.face)))
            .map(|(shell, _)| shell)
            .collect(),
    );
    if shells.len()
        != model
            .shells
            .iter()
            .filter(|shell| shell.faces.iter().any(|u| face_map.contains_key(&u.face)))
            .count()
    {
        return Err(error(
            DIRECT_EDIT_AUDIT_REFUSED,
            "Dirty patch has no owning shell",
        ));
    }

    let mut out = Model(
        brep_topology::Model {
            vertices: vertices
                .keys()
                .map(|&vertex| model.vertices[vertex].clone())
                .collect(),
            edges: edges
                .keys()
                .map(|&edge| {
                    let mut edge = model.edges[edge].clone();
                    edge.vertices = edge.vertices.map(|vertex| vertices[&vertex]);
                    edge
                })
                .collect(),
            loops: loops
                .keys()
                .map(|&wire| {
                    let mut wire = model.loops[wire].clone();
                    for coedge in &mut wire.coedges {
                        coedge.edge = edges[&coedge.edge];
                    }
                    wire
                })
                .collect(),
            faces: face_indices
                .iter()
                .map(|&face| {
                    let mut face = model.faces[face].clone();
                    face.outer = loops[&face.outer];
                    for hole in &mut face.holes {
                        *hole = loops[hole];
                    }
                    face
                })
                .collect(),
            shells: shells
                .keys()
                .map(|&shell| {
                    let mut patch = model.shells[shell].clone();
                    patch.closed = false;
                    patch.faces = patch
                        .faces
                        .iter()
                        .filter_map(|u| {
                            face_map.get(&u.face).map(|&face| brep_topology::FaceUse {
                                face,
                                reversed: u.reversed,
                            })
                        })
                        .collect();
                    patch
                })
                .collect(),
            bodies: vec![],
            tolerance_mm: model.tolerance_mm,
        },
        crate::TopologyIds {
            vertices: vertices.keys().map(|&id| model.1.vertices[id]).collect(),
            edges: edges.keys().map(|&id| model.1.edges[id]).collect(),
            loops: loops.keys().map(|&id| model.1.loops[id]).collect(),
            faces: face_indices.iter().map(|&id| model.1.faces[id]).collect(),
            shells: shells.keys().map(|&id| model.1.shells[id]).collect(),
            bodies: vec![],
            lineage: model.1.lineage.clone(),
            change_set: Default::default(),
            local_tolerances_mm: Default::default(),
            sewed_edges: Default::default(),
        },
    );
    out.refresh_change_set(&[model]);
    Ok((out, face_indices))
}

fn core_edges(model: &Model, core: &BTreeSet<usize>) -> BTreeSet<usize> {
    core.iter()
        .flat_map(|&face| {
            let face = &model.faces[face];
            std::iter::once(face.outer).chain(face.holes.iter().copied())
        })
        .flat_map(|wire| model.loops[wire].coedges.iter().map(|c| c.edge))
        .collect()
}

fn seam_faces(model: &Model, core: &BTreeSet<usize>) -> BTreeSet<usize> {
    let edges = core_edges(model, core);
    (0..model.faces.len())
        .filter(|face| !core.contains(face))
        .filter(|&face| {
            let face = &model.faces[face];
            std::iter::once(face.outer)
                .chain(face.holes.iter().copied())
                .flat_map(|wire| &model.loops[wire].coedges)
                .any(|coedge| edges.contains(&coedge.edge))
        })
        .collect()
}

fn affected_shells(model: &Model, faces: &BTreeSet<usize>) -> BTreeSet<usize> {
    model
        .shells
        .iter()
        .enumerate()
        .filter(|(_, shell)| shell.faces.iter().any(|u| faces.contains(&u.face)))
        .map(|(shell, _)| shell)
        .collect()
}

/// Validate the changed region before publication. Closure is checked on every
/// core edge with its complete seam present; Euler-Poincare is compared only on
/// affected shells. The local face-pair proof is stronger than checking only
/// new-edge/old-face contacts and therefore covers that required subset.
pub fn validate_direct_edit(
    source: &Model,
    result: &Model,
    seeds: &[usize],
    budget: &Budget,
) -> Result<DirectEditValidationReport> {
    let dirty = dirty_region_for_result(source, result, seeds, budget)?;
    let source_seam = seam_faces(source, &dirty.faces);
    let result_seam = seam_faces(result, &dirty.result_faces);
    let source_region: BTreeSet<usize> = dirty.faces.union(&source_seam).copied().collect();
    let result_region: BTreeSet<usize> = dirty.result_faces.union(&result_seam).copied().collect();

    let core = core_edges(result, &dirty.result_faces);
    let mut uses = BTreeMap::<usize, usize>::new();
    for &face in &result_region {
        let face = &result.faces[face];
        for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            for coedge in &result.loops[wire].coedges {
                *uses.entry(coedge.edge).or_default() += 1;
            }
        }
    }
    let non_manifold_edges = core
        .iter()
        .copied()
        .filter(|&edge| !result.edges[edge].degenerate && uses.get(&edge) != Some(&2))
        .collect::<Vec<_>>();

    let before_shells = affected_shells(source, &source_region);
    let after_shells = affected_shells(result, &result_region);
    let euler = before_shells
        .union(&after_shells)
        .map(|&shell| ShellEulerCheck {
            shell,
            before: before_shells
                .contains(&shell)
                .then(|| source.shell_euler_characteristic(shell))
                .flatten(),
            after: after_shells
                .contains(&shell)
                .then(|| result.shell_euler_characteristic(shell))
                .flatten(),
        })
        .collect::<Vec<_>>();

    let (region_model, local_faces) = extract_patch(result, &result_region)?;
    let local_geometry_error = crate::solid_audit::LocallyValidatedModel::new(region_model.clone())
        .err()
        .map(|error| format!("{}: {}", error.code, error.message));

    let (seam, seam_error) = if result_seam.is_empty() {
        (None, None)
    } else {
        match extract_patch(result, &dirty.result_faces)
            .and_then(|(core, _)| extract_patch(result, &result_seam).map(|(seam, _)| (core, seam)))
            .and_then(|(core, seam)| {
                crate::watertight::inspect_patch_set(&[core, seam], 4096, 100_000)
            }) {
            Ok(report) => (Some(report), None),
            Err(error) => (None, Some(format!("{}: {}", error.code, error.message))),
        }
    };

    let pair_budget = region_model.faces.len() * region_model.faces.len().saturating_sub(1) / 2;
    let intersection = crate::self_intersection::inspect_qualified(
        &region_model,
        1e-8,
        64,
        crate::face_contacts::Limits {
            pairs: pair_budget.max(1),
            cells: 100_000,
            domain_cells: 1_000_000,
            cells_per_pair: 256,
            domain_cells_per_pair: 4096,
        },
        100_000,
    );
    let mut interior_contacts = Vec::new();
    let mut self_intersection_proven = false;
    let mut self_intersection_error = None;
    let mut checked_face_pairs = 0;
    match intersection {
        Ok(report) => {
            checked_face_pairs = report.pairs.pairs.len();
            self_intersection_proven = report.absence_proven;
            interior_contacts = report
                .interior_contacts
                .iter()
                .map(|pair| [local_faces[pair[0]], local_faces[pair[1]]])
                .collect();
        }
        Err(error) => {
            self_intersection_error = Some(format!("{}: {}", error.code, error.message));
        }
    }

    Ok(DirectEditValidationReport {
        dirty,
        seam_faces: result_seam,
        non_manifold_edges,
        euler,
        seam,
        seam_error,
        interior_contacts,
        self_intersection_proven,
        self_intersection_error,
        local_geometry_error,
        checked_face_pairs,
    })
}

pub(super) fn validate_staged(
    source: &Model,
    staged: &Model,
    seeds: &[usize],
    budget: &Budget,
) -> Result<DirectEditValidationReport> {
    let report = validate_direct_edit(source, staged, seeds, budget)?;
    if !report.publishable() {
        return Err(error(
            DIRECT_EDIT_AUDIT_REFUSED,
            format!(
                "Local direct-edit audit failed: {} non-manifold edges, {} Euler mismatches, {} interior contacts; seam_complete={}, self_intersection_proven={}, seam_error={:?}, self_intersection_error={:?}, local_geometry_error={:?}",
                report.non_manifold_edges.len(),
                report.euler.iter().filter(|check| !check.ok()).count(),
                report.interior_contacts.len(),
                report.seam_complete(),
                report.self_intersection_proven,
                report.seam_error,
                report.self_intersection_error,
                report.local_geometry_error,
            ),
        ));
    }
    Ok(report)
}

/// Stable rollback identity over geometry, topology, naming and change history.
pub fn direct_edit_model_hash(model: &Model) -> [u8; 32] {
    Sha256::digest(format!("{model:?}").as_bytes()).into()
}

#[derive(Clone, Debug)]
pub struct DirectEditUndoRecord {
    checkpoint: Model,
    checkpoint_hash: [u8; 32],
    pub validation: DirectEditValidationReport,
}

impl DirectEditUndoRecord {
    pub fn checkpoint_hash(&self) -> [u8; 32] {
        self.checkpoint_hash
    }

    pub fn rollback(&self) -> Result<Model> {
        if direct_edit_model_hash(&self.checkpoint) != self.checkpoint_hash {
            return Err(error(
                DIRECT_EDIT_AUDIT_REFUSED,
                "Undo checkpoint hash does not match its retained model",
            ));
        }
        Ok(self.checkpoint.clone())
    }
}

#[derive(Clone, Debug)]
pub struct DirectEditCommit {
    pub model: Model,
    pub undo: DirectEditUndoRecord,
}

/// Explicit staged transaction: validation failure retains both the report and
/// the untouched checkpoint, so `rollback` remains available before commit.
#[derive(Clone, Debug)]
pub struct DirectEditTransaction {
    checkpoint: Model,
    checkpoint_hash: [u8; 32],
    staged: Option<Model>,
    pub validation: Option<DirectEditValidationReport>,
}

impl DirectEditTransaction {
    pub fn stage(checkpoint: &Model, staged: Model) -> Self {
        Self {
            checkpoint: checkpoint.clone(),
            checkpoint_hash: direct_edit_model_hash(checkpoint),
            staged: Some(staged),
            validation: None,
        }
    }

    pub fn validate(
        &mut self,
        seeds: &[usize],
        budget: &Budget,
    ) -> Result<&DirectEditValidationReport> {
        let staged = self.staged.as_ref().ok_or_else(|| {
            error(
                DIRECT_EDIT_AUDIT_REFUSED,
                "Direct-edit transaction has no staged model",
            )
        })?;
        let report = validate_direct_edit(&self.checkpoint, staged, seeds, budget)?;
        let ok = report.ok();
        self.validation = Some(report);
        if !ok {
            return Err(error(
                DIRECT_EDIT_AUDIT_REFUSED,
                "Direct-edit validation refused publication; rollback checkpoint retained",
            ));
        }
        Ok(self.validation.as_ref().expect("validation stored"))
    }

    pub fn rollback(&self) -> Result<Model> {
        if direct_edit_model_hash(&self.checkpoint) != self.checkpoint_hash {
            return Err(error(
                DIRECT_EDIT_AUDIT_REFUSED,
                "Undo checkpoint hash does not match its retained model",
            ));
        }
        Ok(self.checkpoint.clone())
    }

    pub fn commit(self) -> Result<DirectEditCommit> {
        let validation = self.validation.ok_or_else(|| {
            error(
                DIRECT_EDIT_AUDIT_REFUSED,
                "Direct-edit commit requires validation",
            )
        })?;
        if !validation.ok() {
            return Err(error(
                DIRECT_EDIT_AUDIT_REFUSED,
                "Failed direct-edit validation cannot be committed",
            ));
        }
        Ok(DirectEditCommit {
            model: self.staged.ok_or_else(|| {
                error(
                    DIRECT_EDIT_AUDIT_REFUSED,
                    "Direct-edit transaction has no staged model",
                )
            })?,
            undo: DirectEditUndoRecord {
                checkpoint: self.checkpoint,
                checkpoint_hash: self.checkpoint_hash,
                validation,
            },
        })
    }
}
