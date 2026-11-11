//! Complete support-product coverage for aligned chords. Same-face products
//! are included; unresolved diagonal neighborhoods never disappear.
use super::*;
/// Owner-bound lower bound for endpoint-normal-aligned chords only.
/// This is not an unconstrained minimum thickness or an attained witness.
pub struct ExpressionAlignedChordMinimum<'v, 'g, 'x, 'o, 'i, 'f, 'a, 's, 'p, 'b> {
    geometry: &'v EmbeddedExpressionGeometry<'g, 'x, 'o, 'i, 'f, 'a, 's, 'p, 'b>,
    minimum_mm: f64,
    max_sine_squared: f64,
}
impl ExpressionAlignedChordMinimum<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn geometry(&self) -> &EmbeddedExpressionGeometry<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
        self.geometry
    }
    pub fn minimum_mm(&self) -> f64 {
        self.minimum_mm
    }
    pub fn max_sine_squared(&self) -> f64 {
        self.max_sine_squared
    }
}
pub struct ExpressionWallQualification<'v, 'g, 'x, 'o, 'i, 'f, 'a, 's, 'p, 'b> {
    pub report: ExpressionWallReport,
    pub certificate: Option<ExpressionAlignedChordMinimum<'v, 'g, 'x, 'o, 'i, 'f, 'a, 's, 'p, 'b>>,
}
impl<'g, 'x, 'o, 'i, 'f, 'a, 's, 'p, 'b>
    EmbeddedExpressionGeometry<'g, 'x, 'o, 'i, 'f, 'a, 's, 'p, 'b>
{
    /// Fresh coverage is mandatory; mutable caller reports never admit bounds.
    pub fn qualify_minimum_aligned_chords<'v>(
        &'v self,
        minimum_mm: f64,
        max_sine_squared: f64,
        tolerance_uv: f64,
        controls: usize,
        max_cells: usize,
        max_spans: usize,
    ) -> Result<ExpressionWallQualification<'v, 'g, 'x, 'o, 'i, 'f, 'a, 's, 'p, 'b>> {
        self.qualify_trimmed_minimum_aligned_chords(
            minimum_mm,
            max_sine_squared,
            tolerance_uv,
            controls,
            max_cells,
            max_spans,
            0,
        )
    }
    /// Fresh whole-face coverage including proven outside-trim exclusions.
    pub fn qualify_trimmed_minimum_aligned_chords<'v>(
        &'v self,
        minimum_mm: f64,
        max_sine_squared: f64,
        tolerance_uv: f64,
        controls: usize,
        max_cells: usize,
        max_spans: usize,
        max_domain_cells: usize,
    ) -> Result<ExpressionWallQualification<'v, 'g, 'x, 'o, 'i, 'f, 'a, 's, 'p, 'b>> {
        let report = self.inspect_trimmed_minimum_aligned_chords(
            minimum_mm,
            max_sine_squared,
            tolerance_uv,
            controls,
            max_cells,
            max_spans,
            max_domain_cells,
        )?;
        let certificate = if report.complete && report.pending.is_empty() {
            Some(ExpressionAlignedChordMinimum {
                geometry: self,
                minimum_mm,
                max_sine_squared,
            })
        } else {
            None
        };
        Ok(ExpressionWallQualification {
            report,
            certificate,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpressionWallPendingReason {
    Budget,
    Enclosure,
    Resolution,
}
impl ExpressionWallPendingReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Budget => "budget",
            Self::Enclosure => "enclosure",
            Self::Resolution => "resolution",
        }
    }
}
pub struct ExpressionWallCell {
    pub faces: [usize; 2],
    pub domains: [[[f64; 2]; 2]; 2],
    pub pending_reason: Option<ExpressionWallPendingReason>,
}
#[cfg(feature = "codec")]
impl value_codec::Serialize for ExpressionWallPendingReason {
    fn to_value(&self) -> value_codec::Value {
        value_codec::Serialize::to_value(self.as_str())
    }
}
#[cfg(feature = "codec")]
impl value_codec::Serialize for ExpressionWallCell {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({"faces":self.faces,"domains":self.domains,"pending_reason":self.pending_reason})
    }
}
fn product_key(cell: &ExpressionWallCell) -> ([usize; 2], [[[u64; 2]; 2]; 2]) {
    let mut domains = cell.domains.map(|uv| uv.map(|d| d.map(f64::to_bits)));
    if cell.faces[0] == cell.faces[1] && domains[1] < domains[0] {
        domains.swap(0, 1);
    }
    (cell.faces, domains)
}
// A whole-rectangle exclusion covers every pair of endpoints in any
// contained rectangle. Both certificates belong to this immutable face and
// this inspection's angular policy; incomplete/negative results never enter.
fn rectangle_contains(outer: [[f64; 2]; 2], inner: [[f64; 2]; 2]) -> bool {
    (0..2).all(|axis| {
        outer[axis]
            .iter()
            .chain(inner[axis].iter())
            .all(|v| v.is_finite())
            && outer[axis][0] <= inner[axis][0]
            && inner[axis][0] <= inner[axis][1]
            && inner[axis][1] <= outer[axis][1]
    })
}
#[cfg(test)]
#[path="source_profile_shell_wall/tests/product_tests.rs"]
mod product_tests;
#[derive(Default)]
pub struct ExpressionWallSpanStages {
    pub position: usize,
    pub normal: usize,
    pub shared_corner: usize,
    pub original_pair: usize,
    pub retained_pair: usize,
    pub corrected_pair: usize,
}
/// Profile-transition products: checked by the transition test, excluded, inconclusive, unresolved,
/// and the spans the test visited.
#[derive(Default)]
pub struct ExpressionWallTransitions {
    pub checked: usize,
    pub excluded: usize,
    pub inconclusive: usize,
    pub unresolved: usize,
    pub spans: usize,
}
/// Products settled or left open by the intrinsic (same-surface) test, and the spans it visited.
#[derive(Default)]
pub struct ExpressionWallIntrinsic {
    pub checked: usize,
    pub excluded: usize,
    pub unresolved: usize,
    pub spans: usize,
}
/// Why pending products stayed pending.
#[derive(Default)]
pub struct ExpressionWallPendingCauses {
    /// Left by global cell/span exhaustion.
    pub budget: usize,
    /// Without complete position enclosures.
    pub enclosure: usize,
    /// At the requested or representable UV resolution.
    pub resolution: usize,
}
pub struct ExpressionWallReport {
    pub minimum_mm: f64,
    pub max_sine_squared: f64,
    pub tolerance_uv: f64,
    pub complete: bool,
    pub pending: Vec<ExpressionWallCell>,
    pub cells: usize,
    pub spans: usize,
    pub span_stages: ExpressionWallSpanStages,
    pub distance_excluded: usize,
    pub angle_excluded: usize,
    pub planar_excluded: usize,
    pub plane_controls: usize,
    pub cache_hits: usize,
    pub total_pairs: usize,
    pub initial_pairs_checked: usize,
    pub symmetry_reused: usize,
    pub trim_excluded: usize,
    pub domain_cells: usize,
    pub profile_transition: ExpressionWallTransitions,
    pub intrinsic: ExpressionWallIntrinsic,
    pub pending_causes: ExpressionWallPendingCauses,
}
/// Diagnostic transport only. The geometry-owned qualification certificate
/// deliberately has no wire decoder or serialization-based admission path.
#[cfg(feature = "codec")]
impl value_codec::Serialize for ExpressionWallReport {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({
            "method":"expression-support-aligned-chord-coverage",
            "scope":"original-retained-transition-support-products",
            "minimum_mm":self.minimum_mm,"max_sine_squared":self.max_sine_squared,
            "tolerance_uv":self.tolerance_uv,
            "complete":self.complete,"pending":self.pending,
            "cells":self.cells,"spans":self.spans,
            "span_stages":{
                "position":self.span_stages.position,"normal":self.span_stages.normal,
                "shared_corner":self.span_stages.shared_corner,"original_pair":self.span_stages.original_pair,
                "retained_pair":self.span_stages.retained_pair,"corrected_pair":self.span_stages.corrected_pair,
            },
            "distance_excluded":self.distance_excluded,"angle_excluded":self.angle_excluded,
            "planar_excluded":self.planar_excluded,"intrinsic_excluded":self.intrinsic.excluded,
            "intrinsic_checked":self.intrinsic.checked,"intrinsic_spans":self.intrinsic.spans,
            "intrinsic_unresolved":self.intrinsic.unresolved,
            "profile_transition_checked":self.profile_transition.checked,
            "profile_transition_excluded":self.profile_transition.excluded,
            "profile_transition_inconclusive":self.profile_transition.inconclusive,
            "profile_transition_unresolved":self.profile_transition.unresolved,
            "profile_transition_spans":self.profile_transition.spans,
            "plane_controls":self.plane_controls,"cache_hits":self.cache_hits,
            "total_pairs":self.total_pairs,"initial_pairs_checked":self.initial_pairs_checked,
            "symmetry_reused":self.symmetry_reused,"trim_excluded":self.trim_excluded,
            "domain_cells":self.domain_cells,"budget_pending":self.pending_causes.budget,
            "enclosure_pending":self.pending_causes.enclosure,"resolution_pending":self.pending_causes.resolution
        })
    }
}


#[path="source_profile_shell_wall/inspect_minimum_aligned_chords.rs"]
mod inspect_minimum_aligned_chords;
#[path="source_profile_shell_wall/inspect_trimmed_minimum_aligned_chords.rs"]
mod inspect_trimmed_minimum_aligned_chords;
