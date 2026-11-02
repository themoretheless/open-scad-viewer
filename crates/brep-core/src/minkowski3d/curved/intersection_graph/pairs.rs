//! Proper physical arcs split by a complete original-boundary root census.
use super::*;
mod pieces;
mod stitch;
pub(super) use stitch::join;
pub(super) fn construct(
    build: &mut Build<'_>,
    faces: [usize; 2],
    a: &[[f64; 2]; 3],
    b: &[[f64; 2]; 3],
) -> Result<OriginalFacePairGraph> {
    let separated = (0..3).find(|&k| a[k][1] < b[k][0] || b[k][1] < a[k][0]);
    if separated.is_some() {
        return Ok(OriginalFacePairGraph {
            faces,
            separating_axis: separated,
            source_order_swapped: false,
            branches: vec![],
            events: vec![],
            pieces: vec![],
        });
    }
    let physical = physical::construct(build, faces)?;
    let events = if physical.native.branches().is_empty() {
        vec![]
    } else {
        events::collect(build, faces, &physical)?
    };
    let pieces = pieces::partition(build, faces, &physical, &events)?;
    Ok(OriginalFacePairGraph {
        faces,
        separating_axis: None,
        branches: physical.branches(),
        source_order_swapped: physical.swapped,
        events,
        pieces,
    })
}
