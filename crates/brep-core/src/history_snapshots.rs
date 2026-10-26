//! Rollback snapshots with face-level copy-on-write history storage.
//!
//! `Model` currently owns plain `Vec<Face>` values, so a restored model still
//! clones its face array. The memory saving applies to the retained history:
//! the base model is shared through `Arc`, and a compatible delta owns only
//! changed `Arc<Face>` payloads plus changed persistent IDs. Non-face topology
//! edits and face loop rewiring use an explicit full-model fallback.
//!
//! Memory accounting is deterministic owned-payload accounting, not RSS:
//! inline `size_of` payloads and actual `Vec`/`String` capacities are included,
//! recursively through surfaces, curves, topology IDs, lineage and change sets.
//! `Arc` pointee payloads are included; allocator headers, `Arc` bookkeeping,
//! and `BTreeMap` node headers are excluded (the stored key/value payloads are
//! included). A full-history baseline contains the base and every captured
//! state as an independent `Arc<Model>` payload.

use crate::{ChangeProvenance, Error, Face, Model, Result, TopologyChange, TopologyIds};
use nurbs_core::{curve::Curve, surface::Surface};
use std::mem::size_of;
use std::sync::Arc;

pub const HISTORY_SNAPSHOT_INVALID: &str = "BREP_HISTORY_SNAPSHOT_INVALID";
pub const HISTORY_SNAPSHOT_OUT_OF_RANGE: &str = "BREP_HISTORY_SNAPSHOT_OUT_OF_RANGE";
pub const HISTORY_SNAPSHOT_PREVIOUS_MISMATCH: &str =
    "BREP_HISTORY_SNAPSHOT_PREVIOUS_MISMATCH";

fn error(code: &'static str, message: impl Into<String>) -> Error {
    Error::new(code, message)
}

#[derive(Clone, Debug)]
enum DeltaStorage {
    Faces {
        changed: Vec<(usize, Arc<Face>)>,
        topology_ids: Option<Arc<TopologyIds>>,
    },
    Full(Arc<Model>),
}

#[derive(Clone, Debug)]
struct HistoryDelta {
    feature_id: String,
    /// Size of this state in the independent-full-snapshot baseline.
    snapshot_bytes: usize,
    storage: DeltaStorage,
}

/// Base state plus one delta per captured feature.
#[derive(Clone, Debug)]
pub struct HistorySnapshots {
    base: Arc<Model>,
    deltas: Vec<HistoryDelta>,
}

/// Owned-payload memory comparison with an independent full snapshot per state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HistoryMemoryReport {
    pub base_bytes: usize,
    pub delta_bytes: usize,
    pub history_bytes: usize,
    pub full_history_bytes: usize,
    /// `full_history_bytes / history_bytes`; greater is better.
    pub memory_ratio: f64,
    pub face_deltas: usize,
    pub full_model_fallbacks: usize,
    pub changed_faces: usize,
}

impl HistorySnapshots {
    pub fn new(base: Model) -> Self {
        Self {
            base: Arc::new(base),
            deltas: Vec::new(),
        }
    }

    /// Number of captured feature deltas; the base is state zero.
    pub fn len(&self) -> usize {
        self.deltas.len()
    }

    pub fn is_empty(&self) -> bool {
        self.deltas.is_empty()
    }

    /// Append a transition, refusing a `previous` state that is not exactly
    /// the current tip. This prevents silently assembling an incoherent chain.
    pub fn capture(
        &mut self,
        feature_id: impl Into<String>,
        previous: &Model,
        next: &Model,
    ) -> Result<()> {
        let feature_id = feature_id.into();
        if feature_id.is_empty() {
            return Err(error(
                HISTORY_SNAPSHOT_INVALID,
                "Snapshot feature id must be non-empty",
            ));
        }
        let current = self.restore(self.len())?;
        if previous != current.as_ref() {
            return Err(error(
                HISTORY_SNAPSHOT_PREVIOUS_MISMATCH,
                format!(
                    "Feature \"{feature_id}\" previous model does not match snapshot state {}",
                    self.len()
                ),
            ));
        }

        let storage = if face_delta_compatible(previous, next) {
            let changed = previous
                .faces
                .iter()
                .zip(&next.faces)
                .enumerate()
                .filter(|(_, (before, after))| before != after)
                .map(|(index, (_, face))| (index, Arc::new(face.clone())))
                .collect::<Vec<_>>();
            let topology_ids = (previous.1 != next.1)
                .then(|| Arc::new(next.1.clone()));
            DeltaStorage::Faces {
                changed,
                topology_ids,
            }
        } else {
            DeltaStorage::Full(Arc::new(next.clone()))
        };
        self.deltas.push(HistoryDelta {
            feature_id,
            snapshot_bytes: model_bytes(next),
            storage,
        });
        Ok(())
    }

    /// Restore state zero (base) or the state after the Nth captured feature.
    pub fn restore(&self, after_feature: usize) -> Result<Arc<Model>> {
        if after_feature > self.len() {
            return Err(error(
                HISTORY_SNAPSHOT_OUT_OF_RANGE,
                format!(
                    "Snapshot state {after_feature} is out of range 0..={}",
                    self.len()
                ),
            ));
        }
        if after_feature == 0 {
            return Ok(Arc::clone(&self.base));
        }
        let mut restored = self.base.as_ref().clone();
        for delta in &self.deltas[..after_feature] {
            match &delta.storage {
                DeltaStorage::Full(model) => restored = model.as_ref().clone(),
                DeltaStorage::Faces {
                    changed,
                    topology_ids,
                } => {
                    for (index, face) in changed {
                        restored.faces[*index] = face.as_ref().clone();
                    }
                    if let Some(ids) = topology_ids {
                        restored.1 = ids.as_ref().clone();
                    }
                }
            }
        }
        Ok(Arc::new(restored))
    }

    pub fn memory_report(&self) -> HistoryMemoryReport {
        let base_bytes = model_bytes(&self.base);
        let delta_container_bytes = self.deltas.capacity() * size_of::<HistoryDelta>();
        let delta_payload_bytes = self
            .deltas
            .iter()
            .map(delta_extra_bytes)
            .sum::<usize>();
        let delta_bytes = delta_container_bytes + delta_payload_bytes;
        let history_bytes = size_of::<Self>() + base_bytes + delta_bytes;
        let full_history_bytes = size_of::<Self>()
            + base_bytes
            + self.deltas.capacity() * size_of::<Arc<Model>>()
            + self
                .deltas
                .iter()
                .map(|delta| delta.snapshot_bytes)
                .sum::<usize>();
        let face_deltas = self
            .deltas
            .iter()
            .filter(|delta| matches!(delta.storage, DeltaStorage::Faces { .. }))
            .count();
        let full_model_fallbacks = self.len() - face_deltas;
        let changed_faces = self
            .deltas
            .iter()
            .map(|delta| match &delta.storage {
                DeltaStorage::Faces { changed, .. } => changed.len(),
                DeltaStorage::Full(_) => 0,
            })
            .sum();
        HistoryMemoryReport {
            base_bytes,
            delta_bytes,
            history_bytes,
            full_history_bytes,
            memory_ratio: full_history_bytes as f64 / history_bytes as f64,
            face_deltas,
            full_model_fallbacks,
            changed_faces,
        }
    }
}

fn face_delta_compatible(previous: &Model, next: &Model) -> bool {
    previous.vertices == next.vertices
        && previous.edges == next.edges
        && previous.loops == next.loops
        && previous.shells == next.shells
        && previous.bodies == next.bodies
        && previous.tolerance_mm == next.tolerance_mm
        && previous.faces.len() == next.faces.len()
        && previous
            .faces
            .iter()
            .zip(&next.faces)
            .all(|(before, after)| before.outer == after.outer && before.holes == after.holes)
}

fn vec_allocation<T>(values: &Vec<T>) -> usize {
    values.capacity() * size_of::<T>()
}

fn string_allocation(value: &String) -> usize {
    value.capacity()
}

fn curve_extra_bytes(curve: &Curve) -> usize {
    vec_allocation(&curve.knots)
        + vec_allocation(&curve.control_points)
        + curve
            .control_points
            .iter()
            .map(vec_allocation)
            .sum::<usize>()
        + vec_allocation(&curve.weights)
}

fn surface_extra_bytes(surface: &Surface) -> usize {
    vec_allocation(&surface.knots_u)
        + vec_allocation(&surface.knots_v)
        + vec_allocation(&surface.control_points)
        + surface
            .control_points
            .iter()
            .map(|row| vec_allocation(row) + row.iter().map(vec_allocation).sum::<usize>())
            .sum::<usize>()
        + vec_allocation(&surface.weights)
        + surface.weights.iter().map(vec_allocation).sum::<usize>()
}

fn face_extra_bytes(face: &Face) -> usize {
    surface_extra_bytes(&face.surface) + vec_allocation(&face.holes)
}

fn face_bytes(face: &Face) -> usize {
    size_of::<Face>() + face_extra_bytes(face)
}

fn provenance_extra_bytes(provenance: &ChangeProvenance) -> usize {
    string_allocation(&provenance.operation)
        + provenance
            .operand
            .as_ref()
            .map_or(0, string_allocation)
        + string_allocation(&provenance.occurrence)
}

fn change_extra_bytes(change: &TopologyChange) -> usize {
    vec_allocation(&change.parents)
        + vec_allocation(&change.children)
        + provenance_extra_bytes(&change.provenance)
        + string_allocation(&change.role)
        + change.anchor.as_ref().map_or(0, string_allocation)
}

fn topology_ids_extra_bytes(ids: &TopologyIds) -> usize {
    vec_allocation(&ids.vertices)
        + vec_allocation(&ids.edges)
        + vec_allocation(&ids.loops)
        + vec_allocation(&ids.faces)
        + vec_allocation(&ids.shells)
        + vec_allocation(&ids.bodies)
        + vec_allocation(&ids.lineage)
        + ids
            .lineage
            .iter()
            .map(|record| {
                string_allocation(&record.operation)
                    + string_allocation(&record.entity_kind)
                    + vec_allocation(&record.parents)
                    + vec_allocation(&record.children)
            })
            .sum::<usize>()
        // BTreeMap node headers are excluded; retained key/value payloads are not.
        + ids.change_set.nodes.len()
            * (size_of::<crate::TopoId>() + size_of::<crate::TopoKind>())
        + vec_allocation(&ids.change_set.changes)
        + ids
            .change_set
            .changes
            .iter()
            .map(change_extra_bytes)
            .sum::<usize>()
}

fn topology_ids_bytes(ids: &TopologyIds) -> usize {
    size_of::<TopologyIds>() + topology_ids_extra_bytes(ids)
}

fn model_bytes(model: &Model) -> usize {
    size_of::<Model>()
        + vec_allocation(&model.vertices)
        + vec_allocation(&model.edges)
        + model
            .edges
            .iter()
            .map(|edge| curve_extra_bytes(&edge.curve))
            .sum::<usize>()
        + vec_allocation(&model.loops)
        + model
            .loops
            .iter()
            .map(|ring| {
                vec_allocation(&ring.coedges)
                    + ring
                        .coedges
                        .iter()
                        .map(|coedge| curve_extra_bytes(&coedge.pcurve))
                        .sum::<usize>()
            })
            .sum::<usize>()
        + vec_allocation(&model.faces)
        + model.faces.iter().map(face_extra_bytes).sum::<usize>()
        + vec_allocation(&model.shells)
        + model
            .shells
            .iter()
            .map(|shell| vec_allocation(&shell.faces))
            .sum::<usize>()
        + vec_allocation(&model.bodies)
        + model
            .bodies
            .iter()
            .map(|body| vec_allocation(&body.inner_shells))
            .sum::<usize>()
        + topology_ids_extra_bytes(&model.1)
}

fn delta_extra_bytes(delta: &HistoryDelta) -> usize {
    string_allocation(&delta.feature_id)
        + match &delta.storage {
            DeltaStorage::Faces {
                changed,
                topology_ids,
            } => {
                vec_allocation(changed)
                    + changed
                        .iter()
                        .map(|(_, face)| face_bytes(face))
                        .sum::<usize>()
                    + topology_ids.as_ref().map_or(0, |ids| topology_ids_bytes(ids))
            }
            DeltaStorage::Full(model) => model_bytes(model),
        }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::direct_edit::direct_edit_model_hash;
    use crate::faceted_cylinder;

    fn changed_face(model: &Model, face: usize, step: usize) -> Model {
        let mut next = model.clone();
        // Uniform positive weight scaling leaves a rational NURBS surface
        // geometrically unchanged, so every memory-fixture state stays valid
        // while its stored face payload still changes.
        let factor = 1. + step as f64;
        for row in &mut next.faces[face].surface.weights {
            for weight in row {
                *weight *= factor;
            }
        }
        next
    }

    #[test]
    fn restores_every_intermediate_hash_and_counts_full_fallbacks() {
        let base = faceted_cylinder(4., 8., 12).unwrap();
        let mut snapshots = HistorySnapshots::new(base.clone());
        let mut references = vec![base.clone()];
        for step in 1..=8 {
            let previous = references.last().unwrap();
            let next = if step == 4 {
                let mut changed = previous.clone();
                changed.vertices[0].point[0] += 0.25;
                changed
            } else {
                changed_face(previous, step % previous.faces.len(), step)
            };
            snapshots
                .capture(format!("feature_{step}"), previous, &next)
                .unwrap();
            references.push(next);
        }

        assert_eq!(snapshots.len(), 8);
        for (state, reference) in references.iter().enumerate() {
            let restored = snapshots.restore(state).unwrap();
            assert_eq!(restored.as_ref(), reference, "state {state}");
            assert_eq!(
                direct_edit_model_hash(restored.as_ref()),
                direct_edit_model_hash(reference),
                "state {state}"
            );
        }
        let report = snapshots.memory_report();
        assert_eq!(report.face_deltas, 7);
        assert_eq!(report.full_model_fallbacks, 1);
        assert_eq!(report.changed_faces, 7);
        let len = snapshots.len();
        let mismatch = snapshots
            .capture("incoherent", &base, &references[1])
            .unwrap_err();
        assert_eq!(mismatch.code, HISTORY_SNAPSHOT_PREVIOUS_MISMATCH);
        assert_eq!(snapshots.len(), len);
        assert_eq!(
            snapshots.restore(len + 1).unwrap_err().code,
            HISTORY_SNAPSHOT_OUT_OF_RANGE
        );
    }

    #[test]
    fn fifty_face_deltas_use_less_than_one_third_of_full_history() {
        let base = faceted_cylinder(10., 20., 98).unwrap();
        assert_eq!(base.faces.len(), 100);
        base.validate().unwrap();
        let mut snapshots = HistorySnapshots::new(base.clone());
        let mut previous = base;
        for step in 1..=50 {
            let next = changed_face(&previous, step % previous.faces.len(), step);
            next.validate().unwrap();
            snapshots
                .capture(format!("feature_{step:02}"), &previous, &next)
                .unwrap();
            previous = next;
        }
        let report = snapshots.memory_report();
        assert_eq!(report.face_deltas, 50);
        assert_eq!(report.full_model_fallbacks, 0);
        assert_eq!(report.changed_faces, 50);
        println!(
            "history memory: base={} B, deltas={} B, delta history={} B, full history={} B, ratio={:.3}x",
            report.base_bytes,
            report.delta_bytes,
            report.history_bytes,
            report.full_history_bytes,
            report.memory_ratio
        );
        assert!(
            report.memory_ratio >= 3.,
            "measured memory ratio is only {:.3}x: {:?}",
            report.memory_ratio,
            report
        );
    }
}
