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
    // Coincident carriers are two-dimensional trim-region problems, not
    // one-dimensional marching branches. Near coincidence likewise needs an
    // interval/material arbitration proof before physical branch authorship.
    use crate::boolean_robustness::CoincidentFaceClass;
    match crate::boolean_robustness::preclassify_coincident_faces(
        build.models[0],
        faces[0],
        build.models[1],
        faces[1],
    )? {
        CoincidentFaceClass::IdenticalCarrier | CoincidentFaceClass::EquivalentNurbs(_) => {
            return Err(fail(
                "identical face carrier requires the certified same-surface UV trim Boolean",
            ));
        }
        CoincidentFaceClass::CoincidentPlane { .. }
        | CoincidentFaceClass::CoincidentCylinder { .. } => {
            return Err(fail(
                "coincident planar faces require a certified 2D trim-region Boolean",
            ));
        }
        CoincidentFaceClass::NearCoincident => {
            return Err(fail(
                "near-coincident face pair requires interval arbitration before marching",
            ));
        }
        CoincidentFaceClass::NonCoincident => {}
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
