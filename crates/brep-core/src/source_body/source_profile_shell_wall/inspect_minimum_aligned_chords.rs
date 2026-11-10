//! source profile shell wall operations: inspect_minimum_aligned_chords.
use super::*;
impl EmbeddedExpressionGeometry<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
/// Exclude all geometrical endpoint-aligned chords shorter than minimum.
    /// Whole-support supersets include chords outside material. Complete
    /// exclusion therefore bounds material chords, but never supplies a local
    /// witness or proves that this requested minimum is attained.
    pub fn inspect_minimum_aligned_chords(
        &self,
        minimum_mm: f64,
        max_sine_squared: f64,
        tolerance_uv: f64,
        controls: usize,
        max_cells: usize,
        max_spans: usize,
    ) -> Result<ExpressionWallReport> {
        self.inspect_trimmed_minimum_aligned_chords(
            minimum_mm,
            max_sine_squared,
            tolerance_uv,
            controls,
            max_cells,
            max_spans,
            0,
        )
    }
}
