use nurbs_core::{surface::Surface, surface_contact_search::search};
use value_codec::json;
fn main() {
    let mut cases = Vec::new();
    for rotation in 0..3 {
        for weight in [1., 1.03125, 1.0625] {
            for shifted in [false, true] {
                for sheared in [false, true] {
                    for mode in 0..3 {
                        let separated = mode == 1;
                        let curved = mode == 2;
                        let matrix = if sheared {
                            [[1., 0.25, 0.], [0., 1., 0.125], [0.125, 0., 1.]]
                        } else {
                            [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]
                        };
                        let offset = if sheared { [16., -8., 4.] } else { [0.; 3] };
                        let make = |second: bool| {
                            let mut s = Surface {
                                degree_u: 1,
                                degree_v: 1,
                                knots_u: if shifted {
                                    vec![-0.1, -0.1, 1.3, 1.3]
                                } else {
                                    vec![0., 0., 1., 1.]
                                },
                                knots_v: if shifted {
                                    vec![0.13, 0.13, 3.17, 3.17]
                                } else {
                                    vec![0., 0., 1., 1.]
                                },
                                control_points: vec![vec![vec![0.; 3]; 2]; 2],
                                weights: vec![vec![1.; 2]; 2],
                                periodic_u: false,
                                periodic_v: false,
                            };
                            for i in 0..2 {
                                for j in 0..2 {
                                    let p = if second && curved {
                                        [i as f64, j as f64, (i * j) as f64 - 0.1875]
                                    } else if second {
                                        [
                                            i as f64,
                                            0.375,
                                            j as f64 + if separated { 2. } else { -0.625 },
                                        ]
                                    } else {
                                        [i as f64, j as f64, 0.]
                                    };
                                    for k in 0..3 {
                                        s.control_points[i][j][(k + rotation) % 3] = offset[k]
                                            + (0..3).map(|l| matrix[k][l] * p[l]).sum::<f64>();
                                    }
                                    s.weights[i][j] = (if i == 1 { weight } else { 1. })
                                        * (if j == 1 {
                                            if second { 1.125 } else { 1.0625 }
                                        } else {
                                            1.
                                        });
                                }
                            }
                            s
                        };
                        let a = make(false);
                        let b = make(true);
                        let r = search(&a, &b, 10000).unwrap();
                        let witness=r.contact.as_ref().map(|w|json!({"firstUv":w.first_uv,"secondUv":w.second_uv,"point":w.point,"contractionUpper":w.contraction_upper}));
                        cases.push(json!({"a":a,"b":b,"rotation":rotation,"matrix":matrix,"offset":offset,"separated":separated,"curved":curved,"absenceProven":r.absence_proven,"cells":r.cells,"unresolved":r.unresolved.len(),"witness":witness}));
                    }
                }
            }
        }
    }
    println!("{}", json!({"cases":cases}));
}
