//! Inspect source-derived projection bounds before attempting a two-chart proof.
use value_codec::json;
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let model =
        brep_core::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
    let before = format!("{model:?}");
    let mut cases = Vec::new();
    for (face, end, axis) in [(0, 0, 0), (10, 1, 1)] {
        for tilt in [0.25, 0.5, 1., 2., 4.] {
            let mut projection = [[0.; 3]; 2];
            projection[0][1 - axis] = 1.;
            projection[1][axis] = tilt;
            projection[1][2] = -1.;
            let r = nurbs_core::surface_quotient_injectivity::certify(
                &model.faces[face].surface,
                end,
                projection,
                16,
                256,
            )
            .unwrap();
            println!(
                "face={face} tilt={tilt} proven={} bounds={:?} margin={:?}",
                r.proven, r.weighted_bounds, r.dominance_margin_lower
            );
            let wall = if face == 0 { 2 } else { 12 };
            let joined = nurbs_core::surface_quotient_injectivity::certify_ruled_join(
                &model.faces[face].surface,
                &model.faces[wall].surface,
                end,
                projection,
                16,
                512,
            )
            .unwrap();
            println!(
                "joined={}/{} proven={} bounds={:?} margin={:?}",
                face, wall, joined.proven, joined.weighted_bounds, joined.dominance_margin_lower
            );
            cases.push(json!({"joinedProven":joined.proven,"joinedReason":joined.reason,"joinedBounds":joined.weighted_bounds,"joinedMargin":joined.dominance_margin_lower,"face":face,"collapsedEnd":end,"projection":projection,"proven":r.proven,"reason":r.reason,
   "cells":r.cells,"weightedBounds":r.weighted_bounds,"dominanceMarginLower":r.dominance_margin_lower,"bandMarginsLower":r.band_margins_lower}));
        }
    }
    assert_eq!(format!("{model:?}"), before);
    std::fs::write(path,value_codec::to_string(&json!({"scope":"Individual projected charts and normalized joined projection; trimmed ownership is audited separately","sourceModel":model,"cases":cases})).unwrap()).unwrap();
}
