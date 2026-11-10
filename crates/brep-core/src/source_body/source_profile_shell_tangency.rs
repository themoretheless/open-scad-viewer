//! G1 at the four actual joins of an owned assembled transition patch.
//! Fresh original-neighbor angle covers compose with exact Hermite first jets.
use super::*;

pub struct TransitionNeighborTangency<'r, 'x, 'o, 'i, 'f, 'a, 's, 'p, 'b> {
    shell: &'r ExpressionShell<'x, 'o, 'i, 'f, 'a, 's, 'p, 'b>,
    upper: [f64; 4],
}
impl TransitionNeighborTangency<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn shell(&self) -> &ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
        self.shell
    }
    /// Section low/high, then rail low/high. Sections inherit exact first jets.
    pub fn upper_sine_squared(&self) -> [f64; 4] {
        self.upper
    }
}
pub struct TransitionNeighborTangencyReport<'r, 'x, 'o, 'i, 'f, 'a, 's, 'p, 'b> {
    pub certificate: Option<TransitionNeighborTangency<'r, 'x, 'o, 'i, 'f, 'a, 's, 'p, 'b>>,
    pub cells: usize,
    pub spans: usize,
    pub reason: &'static str,
}
impl<'x, 'o, 'i, 'f, 'a, 's, 'p, 'b> ExpressionShell<'x, 'o, 'i, 'f, 'a, 's, 'p, 'b> {
    /// Require the actual shell's four opposite uses and full regular support
    /// joins. Recompute root-ended angles against the actual original neighbor,
    /// even when the proposal also contains a changed profile for that face.
    /// This does not qualify every old shell edge or global solid embedding.
    pub fn qualify_transition_neighbor_tangency<'r>(
        &'r self,
        threshold: f64,
        controls: usize,
        cells: usize,
        spans: usize,
    ) -> Result<TransitionNeighborTangencyReport<'r, 'x, 'o, 'i, 'f, 'a, 's, 'p, 'b>> {
        if !threshold.is_finite() || !(0. ..1.).contains(&threshold) {
            return Err(Error::new(
                "BREP_PROFILE_TANGENCY",
                "Choose finite neighbor angular tolerance",
            ));
        }
        let mut out = TransitionNeighborTangencyReport {
            certificate: None,
            cells: 0,
            spans: 0,
            reason: "profile-transition-neighbor-incidence-unproven",
        };
        let face = self.assembly.patch.rails.face;
        let roots = &face.boundary.rooted;
        let mut seen = [false; 4];
        let mut upper = [0.; 4];
        for pair in &self.pairs {
            let (index, neighbor) = match pair.kind {
                ShellPairKind::Section(end) if end < 2 => (end, ShellFaceKind::Retained(end)),
                ShellPairKind::Rail(rail) if rail < 2 => {
                    let seam = &roots[rail].restricted;

                    let faces = seam.faces();
                    let neighbor = faces
                        .into_iter()
                        .find(|id| *id != roots[rail].face)
                        .unwrap();
                    (rail + 2, ShellFaceKind::Original(neighbor))
                }
                ShellPairKind::Section(_) | ShellPairKind::Rail(_) => return Ok(out),
                _ => continue,
            };
            if seen[index] {
                return Ok(out);
            }
            let kinds = pair.uses.map(|address| self.faces[address.face].kind);
            if !(kinds == [ShellFaceKind::Transition, neighbor]
                || kinds == [neighbor, ShellFaceKind::Transition])
            {
                return Ok(out);
            }
            seen[index] = true;
        }
        if !seen.into_iter().all(|v| v) {
            return Ok(out);
        }
        if cells > 100000 || spans > 1000000 {
            return Err(Error::new(
                "BREP_PROFILE_TANGENCY",
                "Bound neighbor tangency work",
            ));
        }
        if controls == 0 || cells == 0 || spans == 0 {
            out.reason = "profile-transition-neighbor-budget-exhausted";
            return Ok(out);
        }
        for rail in 0..2 {
            let root = &roots[rail];
            let seam = &root.restricted;
            let neighbor = seam
                .faces()
                .into_iter()
                .find(|id| *id != root.face)
                .unwrap();
            let remaining_cells = cells.saturating_sub(out.cells).min(100000);
            let remaining_spans = spans.saturating_sub(out.spans).min(100000);
            if remaining_cells == 0 || remaining_spans == 0 {
                out.reason = "profile-transition-neighbor-budget-exhausted";
                return Ok(out);
            }
            let angles = seam.proposal().inspect_original_neighbor_interval(
                root.face,
                neighbor,
                seam.edge(),
                threshold,
                remaining_cells,
                remaining_spans,
                seam.canonical_interval(),
            )?;
            out.cells += angles.cells;
            out.spans += angles.spans;
            let Some(bound) = angles.upper_sine_squared else {
                out.reason = "profile-transition-neighbor-angle-unproven";
                return Ok(out);
            };
            if angles.aligned != Some(true)
                || !angles.pending.is_empty()
                || !bound.is_finite()
                || bound > threshold
            {
                out.reason = "profile-transition-neighbor-angle-unproven";
                return Ok(out);
            }
            upper[rail + 2] = bound;
        }
        let support = face.qualify_support_tangency(
            controls,
            cells.saturating_sub(out.cells),
            spans.saturating_sub(out.spans),
        )?;
        out.cells += support.cells;
        out.spans += support.spans;
        out.reason = support.reason;
        if support.certificate.is_none() {
            return Ok(out);
        }
        out.certificate = Some(TransitionNeighborTangency { shell: self, upper });
        out.reason = "profile-transition-neighbor-tangency-qualified";
        Ok(out)
    }
}
