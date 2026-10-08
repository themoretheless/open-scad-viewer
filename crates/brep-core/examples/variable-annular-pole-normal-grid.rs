//! Exact whole-grid boundary factor qualification, not a wall certificate.
use cad_predicates::{
    AuthoredScalar, Limits, PredicateContext, Sign, SourceArena, ToleranceContext,
};
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
            let before = value_codec::to_string(&model).unwrap();
            for (face, reverse) in [(0, false), (10, true)] {
                let mut s = model.faces[face].surface.clone();
                if reverse {
                    s.control_points.reverse();
                    s.weights.reverse();
                }
                let nv = s.control_points[0].len();
                let values = s
                    .control_points
                    .iter()
                    .enumerate()
                    .flat_map(|(u, row)| {
                        let weights = &s.weights;
                        row.iter()
                            .enumerate()
                            .flat_map(move |(v, p)| [p[0], p[1], p[2], weights[u][v]])
                    })
                    .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
                    .collect();
                let arena =
                    SourceArena::authored("original-pole-whole-normal-grid", 1, values).unwrap();
                let leaves = (0..s.control_points.len())
                    .map(|u| {
                        (0..nv)
                            .map(|v| {
                                std::array::from_fn(|k| arena.leaf((u * nv + v) * 4 + k).unwrap())
                            })
                            .collect()
                    })
                    .collect::<Vec<_>>();
                let tolerance = ToleranceContext::default_valid();
                let mut components = Vec::new();
                let mut factor_proven = true;
                for axes in [[1, 2], [2, 0], [0, 1]] {
                    let mut ctx = PredicateContext::new(
                        &arena,
                        &tolerance,
                        Limits {
                            max_work: cad_predicates::MAX_WORK,
                            ..Limits::default()
                        },
                        None,
                    );
                    let d = cad_predicates::rational_surface_projected_jacobian_bounds(
                        &mut ctx, &leaves, axes,
                    )
                    .unwrap();
                    let mut work = d.work_used;
                    let mut grid = d.coefficient_bounds;
                    let mut signs = d.signs;
                    let mut mode = "whole";
                    if grid.is_none() {
                        mode = "rows-or-coefficients";
                        let mut net = Vec::new();
                        let mut sign_net = Vec::new();
                        let mut complete = true;
                        for u in 0..3 * s.degree_u {
                            let mut ctx = PredicateContext::new(
                                &arena,
                                &tolerance,
                                Limits {
                                    max_work: cad_predicates::MAX_WORK,
                                    ..Limits::default()
                                },
                                None,
                            );
                            let row =
                                cad_predicates::rational_surface_projected_jacobian_row_bounds(
                                    &mut ctx, &leaves, axes, u,
                                )
                                .unwrap();
                            work += row.work_used;
                            if let (Some(b), Some(s)) = (row.coefficient_bounds, row.signs) {
                                net.push(b[0].clone());
                                sign_net.push(s[0].clone());
                            } else {
                                let mut bs = Vec::new();
                                let mut ss = Vec::new();
                                for v in 0..3 * s.degree_v {
                                    if work >= 100_000_000 {
                                        complete = false;
                                        break;
                                    }
                                    let mut ctx = PredicateContext::new(
                                        &arena,
                                        &tolerance,
                                        Limits {
                                            max_work: cad_predicates::MAX_WORK,
                                            ..Limits::default()
                                        },
                                        None,
                                    );
                                    let cell = cad_predicates::rational_surface_projected_jacobian_coefficient_bounds(&mut ctx,&leaves,axes,[u,v]).unwrap();
                                    work += cell.work_used;
                                    if let (Some(b), Some(s)) =
                                        (cell.coefficient_bounds, cell.signs)
                                    {
                                        bs.push(b[0][0]);
                                        ss.push(s[0][0]);
                                    } else {
                                        complete = false;
                                        break;
                                    }
                                }
                                if !complete {
                                    break;
                                }
                                net.push(bs);
                                sign_net.push(ss);
                            }
                        }
                        if complete {
                            grid = Some(net);
                            signs = Some(sign_net);
                        }
                    }
                    let zeros = signs.as_ref().is_some_and(|net| {
                        net.len() == 3 * s.degree_u
                            && net.iter().all(|row| row.len() == 3 * s.degree_v)
                            && net[..2].iter().flatten().all(|x| *x == Sign::Zero)
                    });
                    factor_proven &= zeros && grid.is_some();
                    components.push(value_codec::json!({"axes":axes,"firstTwoRowsExactlyZero":zeros,"reason":format!("{:?}",d.reason),"work":work,"mode":mode,"bounds":grid}));
                }
                eprintln!("radii={radii:?} direction={direction} face={face} whole_factor_proven={factor_proven}");
                cases.push(value_codec::json!({"radii":radii,"direction":direction,"face":face,"reversedU":reverse,"wholeU2FactorProven":factor_proven,"components":components}));
            }
            assert_eq!(value_codec::to_string(&model).unwrap(), before);
        }
    }
    std::fs::write(output,value_codec::to_string_pretty(&value_codec::json!({"schema":"cad-original-pole-normal-grid/1","scope":"Exact original full-net U2 factor only; no wall, regular pole or G1 certificate","cases":cases})).unwrap()).unwrap();
}
