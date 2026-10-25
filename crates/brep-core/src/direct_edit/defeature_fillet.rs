//! Best-effort and atomic batch suppression of recognized fillet chains.
//!
//! Every mutation is delegated to [`suppress_feature`], so each staged result
//! keeps the existing validation and solid-audit contour. Chains are resolved
//! again after every successful edit through persistent face ids; face indices
//! from the input model are never reused as indices into a later model.
//!
//! Tangentially connected chains are one corner neighborhood. Removing only
//! one of them would leave an unresolved junction, so this iteration refuses
//! every chain in that neighborhood with [`DefeatureFilletRefusal::CornerConflict`]
//! instead of guessing a partial corner reconstruction.

use super::*;
use crate::aag::{Aag, DihedralClass};
use crate::aag_fillets::{FilletKind, find_fillet_chains};
use crate::{Model, Result, TopoId};
use nurbs_core::foundation::guards::Budget;

/// An atomic batch was refused; all staged removals are discarded.
pub const DEFEATURE_FILLET_ATOMIC_REFUSED: &str = "BREP_DEFEATURE_FILLET_ATOMIC_REFUSED";
/// The final model did not pass the solid audit.
pub const DEFEATURE_FILLET_AUDIT_FAILED: &str = "BREP_DEFEATURE_FILLET_AUDIT_FAILED";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DefeatureFilletOptions {
    /// Refuse the whole batch on the first per-chain refusal and discard all
    /// staged edits. The default is best-effort: refuse one chain, keep going.
    pub atomic: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefeatureFilletRefusal {
    DivergentSupports,
    FreeformSupports,
    CornerConflict,
    UnsupportedKind,
    StaleChain,
    AuditFailure,
    UnsupportedNeighborhood,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefeatureFilletStatus {
    Removed,
    Refused(DefeatureFilletRefusal),
}

#[derive(Clone, Debug)]
pub struct DefeatureFilletOutcome {
    /// The requested chain, with face indices in the original input model.
    pub chain: FilletChain,
    /// Persistent identities of the requested chain faces in the input model.
    pub face_ids: Vec<TopoId>,
    pub status: DefeatureFilletStatus,
    pub detail: String,
}

#[derive(Clone, Debug)]
pub struct DefeatureFilletReport {
    pub model: Model,
    pub outcomes: Vec<DefeatureFilletOutcome>,
    pub removed_chains: usize,
    pub refused_chains: usize,
    pub new_sharp_edges: usize,
    pub audit: SolidAuditCertificate,
}

struct WorkItem {
    requested: FilletChain,
    initial_chain: Option<usize>,
    face_ids: Vec<TopoId>,
}

fn recognized_chains(model: &Model, budget: &Budget) -> Result<(Aag, Vec<FilletChain>)> {
    let mut aag = Aag::build(model, budget)?;
    aag.attach_face_attrs(model, budget)?;
    let chains = find_fillet_chains(model, &aag, budget)?;
    Ok((aag, chains))
}

fn face_ids(model: &Model, chain: &FilletChain) -> Vec<TopoId> {
    chain
        .faces
        .iter()
        .filter_map(|&face| model.1.faces.get(face).cloned())
        .collect()
}

/// Mark both sides of every tangential edge joining distinct chains. This
/// covers the complete corner group, including chains that do not own the
/// spherical junction face after recognition partitions the faces.
fn corner_conflicts(aag: &Aag, chains: &[FilletChain], face_count: usize) -> BTreeSet<usize> {
    let mut owner = vec![None; face_count];
    for (index, chain) in chains.iter().enumerate() {
        for &face in &chain.faces {
            if let Some(slot) = owner.get_mut(face) {
                *slot = Some(index);
            }
        }
    }
    let mut conflicts = BTreeSet::new();
    for edge in &aag.edges {
        if !matches!(edge.class, DihedralClass::Smooth | DihedralClass::Tangent) {
            continue;
        }
        let owners: BTreeSet<usize> = edge.uses.iter().filter_map(|u| owner[u.face]).collect();
        if owners.len() > 1 {
            conflicts.extend(owners);
        }
    }
    conflicts
}

fn refuse(
    outcomes: &mut Vec<DefeatureFilletOutcome>,
    item: &WorkItem,
    reason: DefeatureFilletRefusal,
    detail: String,
    atomic: bool,
) -> Result<()> {
    if atomic {
        let staged = outcomes
            .iter()
            .filter(|outcome| outcome.status == DefeatureFilletStatus::Removed)
            .count();
        return Err(Error::new(
            DEFEATURE_FILLET_ATOMIC_REFUSED,
            format!(
                "Atomic fillet defeature refused after {staged} staged removal(s); all edits were discarded: {detail}"
            ),
        ));
    }
    outcomes.push(DefeatureFilletOutcome {
        chain: item.requested.clone(),
        face_ids: item.face_ids.clone(),
        status: DefeatureFilletStatus::Refused(reason),
        detail,
    });
    Ok(())
}

fn suppression_refusal(error: &Error) -> DefeatureFilletRefusal {
    let message = error.message.to_ascii_lowercase();
    match error.code {
        SUPPRESS_DIVERGENT => DefeatureFilletRefusal::DivergentSupports,
        SUPPRESS_INVALID => DefeatureFilletRefusal::StaleChain,
        SUPPRESS_UNSUPPORTED
            if message.contains("planar")
                || message.contains("freeform")
                || message.contains("curved") =>
        {
            DefeatureFilletRefusal::FreeformSupports
        }
        SUPPRESS_UNSUPPORTED => DefeatureFilletRefusal::UnsupportedNeighborhood,
        code if code.contains("STALE") => DefeatureFilletRefusal::StaleChain,
        code if code.contains("AUDIT") || code.contains("INVALID_TOPOLOGY") => {
            DefeatureFilletRefusal::AuditFailure
        }
        _ => DefeatureFilletRefusal::UnsupportedNeighborhood,
    }
}

fn is_resource_error(error: &Error) -> bool {
    error.code.contains("RESOURCE") || error.code.contains("BUDGET")
}

/// Find and suppress all recognized fillet chains.
pub fn defeature_fillets(
    model: &Model,
    options: DefeatureFilletOptions,
    budget: &Budget,
) -> Result<DefeatureFilletReport> {
    let (_, chains) = recognized_chains(model, budget)?;
    defeature_fillet_batch(model, &chains, options, budget)
}

/// Suppress an explicit list of chains recognized on `model`.
///
/// The explicit chains are matched to a fresh recognition pass by their exact
/// original face sets. This both rejects foreign/stale input and preserves the
/// corner-conflict diagnosis when only one member of a corner group is passed.
pub fn defeature_fillet_batch(
    model: &Model,
    chains: &[FilletChain],
    options: DefeatureFilletOptions,
    budget: &Budget,
) -> Result<DefeatureFilletReport> {
    let (initial_aag, initial_chains) = recognized_chains(model, budget)?;
    let conflicts = corner_conflicts(&initial_aag, &initial_chains, model.faces.len());
    let work: Vec<WorkItem> = chains
        .iter()
        .map(|requested| WorkItem {
            requested: requested.clone(),
            initial_chain: initial_chains
                .iter()
                .position(|candidate| candidate.faces == requested.faces),
            face_ids: face_ids(model, requested),
        })
        .collect();

    let mut current = model.clone();
    let mut outcomes = Vec::with_capacity(work.len());
    let mut new_sharp_edges = 0;

    for item in &work {
        let Some(initial) = item.initial_chain else {
            refuse(
                &mut outcomes,
                item,
                DefeatureFilletRefusal::StaleChain,
                "The requested face set is not a fillet chain recognized on the input model".into(),
                options.atomic,
            )?;
            continue;
        };
        if item.requested.kind != FilletKind::ConstantRadius {
            refuse(
                &mut outcomes,
                item,
                DefeatureFilletRefusal::UnsupportedKind,
                format!(
                    "Only constant-radius chains are supported; requested kind is {:?}",
                    item.requested.kind
                ),
                options.atomic,
            )?;
            continue;
        }
        if conflicts.contains(&initial) {
            refuse(
                &mut outcomes,
                item,
                DefeatureFilletRefusal::CornerConflict,
                "The chain is tangentially connected to another fillet chain; partial removal of a corner neighborhood is refused".into(),
                options.atomic,
            )?;
            continue;
        }

        // Re-recognize on the current staged model and resolve exclusively by
        // persistent ids. Zero or multiple matches are equally ambiguous.
        let (_, current_chains) = recognized_chains(&current, budget)?;
        let wanted: BTreeSet<TopoId> = item.face_ids.iter().cloned().collect();
        let matches: Vec<&FilletChain> = current_chains
            .iter()
            .filter(|candidate| {
                face_ids(&current, candidate)
                    .iter()
                    .any(|identity| wanted.contains(identity))
            })
            .collect();
        if matches.len() != 1 {
            refuse(
                &mut outcomes,
                item,
                DefeatureFilletRefusal::StaleChain,
                format!(
                    "Persistent face ids resolve to {} current chains; exactly one is required",
                    matches.len()
                ),
                options.atomic,
            )?;
            continue;
        }

        match suppress_feature(
            &current,
            &FeatureRef::FilletChain(matches[0].clone()),
            budget,
        ) {
            Ok(report) => {
                new_sharp_edges += report.new_sharp_edges;
                outcomes.push(DefeatureFilletOutcome {
                    chain: item.requested.clone(),
                    face_ids: item.face_ids.clone(),
                    status: DefeatureFilletStatus::Removed,
                    detail: format!(
                        "Chain suppressed; {} new sharp edge(s)",
                        report.new_sharp_edges
                    ),
                });
                current = report.model;
            }
            Err(error) if is_resource_error(&error) => return Err(error),
            Err(error) => {
                let reason = suppression_refusal(&error);
                refuse(
                    &mut outcomes,
                    item,
                    reason,
                    format!("{}: {}", error.code, error.message),
                    options.atomic,
                )?;
            }
        }
    }

    let audit = crate::solid_audit::audit_solid(&current).map_err(|error| {
        Error::new(
            DEFEATURE_FILLET_AUDIT_FAILED,
            format!("Final fillet-defeature audit failed: {error}"),
        )
    })?;
    if !audit.ok {
        return Err(Error::new(
            DEFEATURE_FILLET_AUDIT_FAILED,
            format!("Final fillet-defeature audit is not green: {audit:?}"),
        ));
    }
    let removed_chains = outcomes
        .iter()
        .filter(|outcome| outcome.status == DefeatureFilletStatus::Removed)
        .count();
    let refused_chains = outcomes.len() - removed_chains;
    Ok(DefeatureFilletReport {
        model: current,
        outcomes,
        removed_chains,
        refused_chains,
        new_sharp_edges,
        audit,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analytic_features::{analytic_fillet, exact_valence3_corner_blend};

    fn budget() -> Budget {
        Budget::new(10_000_000, 8, 60_000).unwrap()
    }

    fn vertical_edges(model: &Model) -> Vec<usize> {
        model
            .edges
            .iter()
            .enumerate()
            .filter_map(|(index, edge)| {
                let a = model.vertices[edge.vertices[0]].point;
                let b = model.vertices[edge.vertices[1]].point;
                ((a[0] - b[0]).abs() <= 1e-12 && (a[1] - b[1]).abs() <= 1e-12).then_some(index)
            })
            .collect()
    }

    fn rounded_cuboid() -> Model {
        let sharp = crate::cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
        let edges = vertical_edges(&sharp);
        analytic_fillet(&sharp, edges[0], 1.).unwrap().0
    }

    fn one_chain(model: &Model) -> FilletChain {
        let (_, chains) = recognized_chains(model, &budget()).unwrap();
        assert_eq!(chains.len(), 1);
        chains[0].clone()
    }

    fn corner_model() -> Model {
        let source = crate::cuboid([0., 0., 0.], [10., 8., 6.]).unwrap();
        let max = [10., 8., 6.];
        let edges: Vec<usize> = source
            .edges
            .iter()
            .enumerate()
            .filter_map(|(index, edge)| {
                let a = source.vertices[edge.vertices[0]].point;
                let b = source.vertices[edge.vertices[1]].point;
                let mid = [
                    0.5 * (a[0] + b[0]),
                    0.5 * (a[1] + b[1]),
                    0.5 * (a[2] + b[2]),
                ];
                let touches = edge.vertices.iter().any(|&vertex| {
                    let point = source.vertices[vertex].point;
                    (0..3).all(|axis| (point[axis] - max[axis]).abs() <= 1e-9)
                });
                let on_pair = |i: usize, j: usize| {
                    (mid[i] - max[i]).abs() <= 1e-9 && (mid[j] - max[j]).abs() <= 1e-9
                };
                (touches && (on_pair(0, 1) || on_pair(0, 2) || on_pair(1, 2))).then_some(index)
            })
            .collect();
        assert_eq!(edges.len(), 3);
        exact_valence3_corner_blend(&source, &edges, 1.)
            .unwrap()
            .model
    }

    #[test]
    fn defeature_fillet_single_chain_is_removed() {
        let model = rounded_cuboid();
        let report =
            defeature_fillets(&model, DefeatureFilletOptions::default(), &budget()).unwrap();
        assert_eq!(report.removed_chains, 1);
        assert_eq!(report.refused_chains, 0);
        assert!(report.new_sharp_edges >= 1);
        assert!(report.audit.ok, "audit: {:?}", report.audit);
        let (_, remaining) = recognized_chains(&report.model, &budget()).unwrap();
        assert!(remaining.is_empty(), "remaining chains: {remaining:?}");
    }

    #[test]
    fn defeature_fillet_valence3_corner_refuses_entire_group() {
        let model = corner_model();
        let before = direct_edit_model_hash(&model);
        let report =
            defeature_fillets(&model, DefeatureFilletOptions::default(), &budget()).unwrap();
        assert_eq!(report.outcomes.len(), 3);
        assert_eq!(report.removed_chains, 0);
        assert_eq!(report.refused_chains, 3);
        assert!(report.outcomes.iter().all(|outcome| outcome.status
            == DefeatureFilletStatus::Refused(DefeatureFilletRefusal::CornerConflict)));
        assert_eq!(direct_edit_model_hash(&report.model), before);
        assert_eq!(report.model, model);
        assert!(report.audit.ok, "audit: {:?}", report.audit);
    }

    #[test]
    fn defeature_fillet_mixed_batch_continues_after_refusal() {
        let model = rounded_cuboid();
        let chain = one_chain(&model);
        let mut unsupported = chain.clone();
        unsupported.kind = FilletKind::VariableRadius;
        let report = defeature_fillet_batch(
            &model,
            &[chain, unsupported],
            DefeatureFilletOptions::default(),
            &budget(),
        )
        .unwrap();
        assert_eq!(report.removed_chains, 1);
        assert_eq!(report.refused_chains, 1);
        assert_eq!(report.outcomes.len(), 2);
        assert_eq!(report.outcomes[0].status, DefeatureFilletStatus::Removed);
        assert_eq!(
            report.outcomes[1].status,
            DefeatureFilletStatus::Refused(DefeatureFilletRefusal::UnsupportedKind)
        );
        assert!(report.audit.ok, "audit: {:?}", report.audit);
    }

    #[test]
    fn defeature_fillet_atomic_batch_rolls_back_staged_removal() {
        let model = rounded_cuboid();
        let before = direct_edit_model_hash(&model);
        let chain = one_chain(&model);
        let mut unsupported = chain.clone();
        unsupported.kind = FilletKind::VariableRadius;
        let error = defeature_fillet_batch(
            &model,
            &[chain, unsupported],
            DefeatureFilletOptions { atomic: true },
            &budget(),
        )
        .unwrap_err();
        assert_eq!(error.code, DEFEATURE_FILLET_ATOMIC_REFUSED);
        assert!(
            error.message.contains("after 1 staged removal"),
            "the ordinary chain must be staged before the refusal: {error}"
        );
        assert_eq!(direct_edit_model_hash(&model), before);
    }
}
