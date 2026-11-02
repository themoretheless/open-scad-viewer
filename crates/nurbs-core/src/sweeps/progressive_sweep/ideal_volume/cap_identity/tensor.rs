//! Full original tensor equality; never a rounded extracted/elevated candidate.
use super::*;
pub(super) fn certify(
    caps: [&Surface; 2],
    out: &mut IdealVolumeReport,
    max_cells: usize,
    max_work: u64,
) -> bool {
    query(&caps, None, out, max_cells, max_work)
}
pub(super) fn continuity(
    cap: &Surface,
    order: usize,
    out: &mut IdealVolumeReport,
    max_cells: usize,
    max_work: u64,
) -> bool {
    query(&[cap], Some(order), out, max_cells, max_work)
}
fn query(
    caps: &[&Surface],
    order: Option<usize>,
    out: &mut IdealVolumeReport,
    max_cells: usize,
    max_work: u64,
) -> bool {
    let mut work = crate::analysis::surface_tensor_identity::Work {
        cells: out.cells,
        exact_work: out.exact_work,
    };
    let proven = crate::analysis::surface_tensor_identity::query(
        caps, order, &mut work, max_cells, max_work,
    );
    out.cells = work.cells;
    out.exact_work = work.exact_work;
    proven
}
