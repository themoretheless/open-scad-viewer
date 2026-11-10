//! Rebuild material roles from a proved embedded shell forest. Alternating
//! containment defines even-odd material. No surface/curve/vertex is moved.
use super::*;
#[derive(Clone, Copy)]
pub struct ShellRepairPolicy {
    /// Each stage has its own explicit bounded work allowance.
    pub establish_forest: AdmissionPolicy,
    pub establish_orientation: AdmissionPolicy,
    pub readmit: AdmissionPolicy,
}
pub struct ShellRepair {
    original: Model,
    solid: Option<QualifiedClosedSolid>,
    attempts: Vec<Arc<volume_validity::Report>>,
    flipped: Vec<usize>,
    parents: Option<Vec<Option<usize>>>,
    refusal: Option<Error>,
}
impl ShellRepair {
    pub fn solid(&self) -> Option<&QualifiedClosedSolid> {
        self.solid.as_ref()
    }
    pub fn model(&self) -> &Model {
        self.solid
            .as_ref()
            .map(|s| s.model())
            .unwrap_or(&self.original)
    }
    pub fn attempts(&self) -> &[Arc<volume_validity::Report>] {
        &self.attempts
    }
    pub fn flipped_shells(&self) -> &[usize] {
        if self.solid.is_some() {
            &self.flipped
        } else {
            &[]
        }
    }
    pub fn attempted_flips(&self) -> &[usize] {
        &self.flipped
    }
    pub fn parents(&self) -> Option<&[Option<usize>]> {
        self.parents.as_deref()
    }
    pub fn refusal(&self) -> Option<&Error> {
        self.refusal.as_ref()
    }
}
fn refused(message: &str) -> Error {
    Error::new("BREP_SHELL_REPAIR_UNPROVED", message)
}
fn run(out: &mut ShellRepair, policy: ShellRepairPolicy) -> Result<()> {
    let initial = qualify(out.original.clone(), policy.establish_forest)?;
    let parents = initial
        .geometry
        .nesting
        .as_ref()
        .and_then(|n| n.parents.clone());
    out.attempts.push(initial.geometry.clone());
    if let Some(solid) = initial.solid {
        out.solid = Some(solid);
        return Ok(());
    }
    if !initial.geometry.boundary.proven {
        return Err(refused(
            "Shell repair requires a complete embedded original boundary",
        ));
    }
    let parents = parents.ok_or_else(|| refused("Shell containment forest is unresolved"))?;
    let n = out.original.shells.len();
    if parents.len() != n {
        return Err(refused("Shell parent count differs from original shells"));
    }
    let mut depths = vec![0usize; n];
    for i in 0..n {
        let mut p = parents[i];
        let mut seen = std::collections::BTreeSet::new();
        seen.insert(i);
        while let Some(j) = p {
            if j >= n || !seen.insert(j) {
                return Err(refused("Shell containment forest is cyclic or foreign"));
            }
            depths[i] += 1;
            p = parents[j];
        }
    }
    let mut candidate = out.original.clone();
    candidate.bodies = (0..n)
        .filter(|&i| depths[i] % 2 == 0)
        .map(|i| crate::Body {
            outer_shell: i,
            inner_shells: (0..n).filter(|&j| parents[j] == Some(i)).collect(),
        })
        .collect();
    candidate.rebuild_topology_ids();
    out.parents = Some(parents);
    let orientation = qualify(candidate.clone(), policy.establish_orientation)?;
    out.attempts.push(orientation.geometry.clone());
    if !orientation.geometry.boundary.proven
        || orientation
            .geometry
            .nesting
            .as_ref()
            .and_then(|n| n.roles_consistent)
            != Some(true)
        || orientation
            .geometry
            .orientations
            .iter()
            .any(|o| o.outward.is_none())
    {
        return Err(refused(
            "Rebuilt material shells have unresolved embedding or orientation",
        ));
    }
    for o in &orientation.geometry.orientations {
        if o.outward != Some(o.expected_outward) {
            for face in &mut candidate.shells[o.shell].faces {
                face.reversed = !face.reversed;
            }
            out.flipped.push(o.shell);
        }
    }
    candidate.rebuild_topology_ids();
    let readmitted = qualify(candidate, policy.readmit)?;
    out.attempts.push(readmitted.geometry.clone());
    out.solid = readmitted.solid;
    if out.solid.is_none() {
        return Err(refused(
            "Repaired material forest failed independent final admission",
        ));
    }
    Ok(())
}
pub fn rebuild_even_odd_shells(model: &Model, policy: ShellRepairPolicy) -> ShellRepair {
    let mut out = ShellRepair {
        original: model.clone(),
        solid: None,
        attempts: Vec::new(),
        flipped: Vec::new(),
        parents: None,
        refusal: None,
    };
    if let Err(error) = run(&mut out, policy) {
        out.refusal = Some(error);
    }
    out
}
