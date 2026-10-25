//! Whole original meridian seam angular bounds between variable support patches.
#[allow(dead_code)]
#[path = "variable-annular-cylinder-rails.rs"]
mod bounds;
use nurbs_core::surface::Surface;
fn meridian(s: &Surface, start: bool) -> Surface {
    let mut out = s.clone();
    std::mem::swap(&mut out.degree_u, &mut out.degree_v);
    std::mem::swap(&mut out.knots_u, &mut out.knots_v);
    std::mem::swap(&mut out.periodic_u, &mut out.periodic_v);
    out.control_points = (0..s.control_points[0].len())
        .map(|v| {
            (0..s.control_points.len())
                .map(|u| {
                    s.control_points[if start {
                        s.control_points.len() - 1 - u
                    } else {
                        u
                    }][v]
                        .clone()
                })
                .collect()
        })
        .collect();
    out.weights = (0..s.weights[0].len())
        .map(|v| {
            (0..s.weights.len())
                .map(|u| s.weights[if start { s.weights.len() - 1 - u } else { u }][v])
                .collect()
        })
        .collect();
    // Reversing a single clamped span preserves its normalized domain.
    out
}
fn main() {
    let output = std::env::args().nth(1).expect("output JSON path");
    let mut cases = Vec::new();
    for radii in [[0.5, 1.25], [1.25, 0.5]] {
        for direction in [1., -1.] {
            let model = brep_core::circular_blend::partial_annular_variable_arc(
                20.,
                5.,
                6.,
                radii,
                [0.3, 0.7, 0.4].map(|x| x * direction),
                1e-7,
            )
            .unwrap();
            for (left, right) in [(0, 5), (5, 10)] {
                let a = &model.faces[left].surface;
                let b = &model.faces[right].surface;
                assert_eq!(a.control_points.last().unwrap(), &b.control_points[0]);
                assert_eq!(a.weights.last().unwrap(), &b.weights[0]);
                let a = bounds::normals(&meridian(a, false), false, false);
                let b = bounds::normals(&meridian(b, true), false, false);
                let mut queue = vec![(a, b, [0., 1.])];
                let mut cells = 0;
                let mut accepted = Vec::new();
                let mut unresolved = Vec::new();
                while let Some((a, b, t)) = queue.pop() {
                    cells += 1;
                    let bound = bounds::angle(&a, &b);
                    if bound.is_some_and(|x| x <= 1e-4) {
                        accepted.push(value_codec::json!({"interval":t,"sineSquaredUpper":bound}));
                    } else if cells + queue.len() >= 10000 || t[1] - t[0] < 1e-10 {
                        unresolved
                            .push(value_codec::json!({"interval":t,"sineSquaredUpper":bound}));
                    } else {
                        let (a0, a1) = bounds::split(&a);
                        let (b0, b1) = bounds::split(&b);
                        let m = (t[0] + t[1]) * 0.5;
                        queue.push((a1, b1, [m, t[1]]));
                        queue.push((a0, b0, [t[0], m]));
                    }
                }
                cases.push(value_codec::json!({"radii":radii,"direction":direction,"faces":[left,right],"originalBoundaryIdentical":true,"proven":unresolved.is_empty(),"cells":cells,"accepted":accepted,"unresolved":unresolved}));
            }
        }
    }
    std::fs::write(output,value_codec::to_string_pretty(&value_codec::json!({"schema":"cad-annular-patch-junctions/1","scope":"Whole original meridian positional identity and angular tolerance; no exact G1/G2 or body admission","maxSineSquared":1e-4,"cases":cases})).unwrap()).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_positions_do_not_hide_a_crease_after_chart_transposition() {
        let source = |origin: f64, slope: f64| Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![2., 2., 7., 7.],
            knots_v: vec![3., 3., 5., 5.],
            control_points: vec![
                vec![vec![origin, 0., 0.], vec![origin, 1., 0.]],
                vec![vec![origin + 1., 0., slope], vec![origin + 1., 1., slope]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let left = source(0., 0.);
        let right = source(1., 1.);
        assert_eq!(left.control_points[1], right.control_points[0]);
        let a = bounds::normals(&meridian(&left, false), false, false);
        let b = bounds::normals(&meridian(&right, true), false, false);
        assert!(bounds::angle(&a, &b).unwrap() > 0.49);
        let flat = source(1., 0.);
        let b = bounds::normals(&meridian(&flat, true), false, false);
        assert!(bounds::angle(&a, &b).unwrap() < 1e-4);
    }
}
