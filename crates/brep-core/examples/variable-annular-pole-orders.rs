//! Exact leading normal numerator orders at original collapsed chart corners.
//! This records component signs and orders, not angular or G1 admission.
use brep_core::circular_blend::partial_annular_variable_arc;
use cad_predicates::{
    AuthoredScalar, Limits, PredicateContext, Sign, SourceArena, ToleranceContext,
};
fn main() {
    let output = std::env::args().nth(1).expect("output JSON path");
    let mut cases = Vec::new();
    for radii in [[0.5, 1.25], [1.25, 0.5]] {
        for direction in [1., -1.] {
            let model = partial_annular_variable_arc(
                20.,
                5.,
                6.,
                radii,
                [0.3, 0.7, 0.4].map(|x| x * direction),
                1e-7,
            )
            .unwrap();
            for (face, end) in [(0, 0), (10, 1)] {
                let original = &model.faces[face].surface;
                let mut surface = original.clone();
                if end == 1 {
                    surface.control_points.reverse();
                    surface.weights.reverse();
                }
                let weights = &surface.weights;
                let values = surface
                    .control_points
                    .iter()
                    .enumerate()
                    .flat_map(|(u, row)| {
                        row.iter()
                            .enumerate()
                            .flat_map(move |(v, p)| [p[0], p[1], p[2], weights[u][v]])
                    })
                    .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
                    .collect();
                let arena =
                    SourceArena::authored("original-pole-normal-orders", 1, values).unwrap();
                let leaves = surface
                    .control_points
                    .iter()
                    .enumerate()
                    .map(|(u, row)| {
                        row.iter()
                            .enumerate()
                            .map(|(v, _)| {
                                std::array::from_fn(|k| {
                                    arena.leaf((u * row.len() + v) * 4 + k).unwrap()
                                })
                            })
                            .collect()
                    })
                    .collect::<Vec<_>>();
                let tolerance = ToleranceContext::default_valid();
                let mut corners = Vec::new();
                let mut leading = [[None; 3]; 2];
                let mut plane_signs = Vec::new();
                for corner in [0, 1] {
                    let column = corner * (3 * surface.degree_v - 1);
                    let mut components = Vec::new();
                    for (component, axes) in [[1, 2], [2, 0], [0, 1]].into_iter().enumerate() {
                        let mut rows = Vec::new();
                        for row in 0..if corner == 0 { 3 * surface.degree_u } else { 3 } {
                            let mut ctx = PredicateContext::new(
                                &arena,
                                &tolerance,
                                Limits {
                                    max_work: cad_predicates::MAX_WORK,
                                    ..Limits::default()
                                },
                                None,
                            );
                            let d =
                                cad_predicates::rational_surface_projected_jacobian_coefficient_bounds(
                                    &mut ctx,
                                    &leaves,
                                    axes,
                                    [row, column],
                                )
                                .unwrap();
                            let sign = d.signs.as_ref().map(|s| s[0][0]);
                            rows.push(value_codec::json!({"row":row,"sign":format!("{sign:?}"),"reason":format!("{:?}",d.reason),"work":d.work_used,"coefficientBounds":d.coefficient_bounds}));
                            if corner == 0 {
                                if component < 2 {
                                    assert_eq!(
                                        sign,
                                        Some(Sign::Zero),
                                        "Original plane rail must have exact axial normals"
                                    );
                                } else {
                                    plane_signs.push(sign);
                                }
                            }
                            if row < 2 {
                                assert_eq!(
                                    sign,
                                    Some(Sign::Zero),
                                    "Lower normal orders must vanish exactly"
                                );
                            }
                            if row == 2 {
                                leading[corner][component] = sign;
                            }
                            if corner != 0 && sign.is_some_and(|s| s != Sign::Zero) {
                                break;
                            }
                        }
                        components.push(value_codec::json!({"axes":axes,"rows":rows}));
                    }
                    corners.push(value_codec::json!({"vEnd":corner,"components":components}));
                }
                let plane_direction = plane_signs[2].unwrap();
                assert!(matches!(plane_direction, Sign::Positive | Sign::Negative));
                assert!(
                    plane_signs[2..].iter().all(|s| *s == Some(plane_direction)),
                    "Plane rail normals must stay nonzero for all u>0"
                );
                assert!(
                    model.faces[face + 1]
                        .surface
                        .control_points
                        .iter()
                        .flatten()
                        .all(|p| p[2] == 6.),
                    "Incident original face must be the authored support plane"
                );
                let nonzero = |s: Option<Sign>| {
                    s.is_some_and(|x| matches!(x, Sign::Positive | Sign::Negative))
                };
                let regular_pole_disproven = leading[0][0] == Some(Sign::Zero)
                    && leading[0][1] == Some(Sign::Zero)
                    && nonzero(leading[0][2])
                    && nonzero(leading[1][0]);
                assert!(
                    regular_pole_disproven,
                    "Two original corner paths must prove nonparallel limiting normals"
                );
                cases.push(value_codec::json!({"radii":radii,"direction":direction,"face":face,"collapsedEnd":end,"planeRailAndLimitProven":true,"regularPoleDisproven":regular_pole_disproven,"corners":corners}));
            }
        }
    }
    std::fs::write(output,value_codec::to_string_pretty(&value_codec::json!({"schema":"cad-annular-pole-normal-orders/1","scope":"Exact zero normal orders 0/1 and nonparallel leading order-two normals on two original corner paths disprove a unique regular pole tangent plane; no seam angular bound", "cases":cases})).unwrap()).unwrap();
}
