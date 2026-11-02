//! Wrapped phase specialization of the shared original-clock residual owner.
use super::*;
pub(super) fn verify(
    a: &Curve,
    b: &Curve,
    tolerance: f64,
    max_cells: usize,
) -> Result<Option<Verification>> {
    Ok(
        crate::curve_partition_agreement::mapped::wrapped(a, b, tolerance, max_cells)?.map(|r| {
            Verification {
                error_upper: r.error_upper,
                accepted: r.accepted,
                cells: r.cells,
                reason: r.reason,
            }
        }),
    )
}
