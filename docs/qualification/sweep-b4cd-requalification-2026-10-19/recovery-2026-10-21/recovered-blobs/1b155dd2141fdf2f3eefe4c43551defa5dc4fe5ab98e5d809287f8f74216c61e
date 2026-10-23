use nurbs_core::{surface::Surface, surface_injectivity::certify};
use value_codec::json;
fn main() {
    let mut cases = Vec::new();
    for axes in [[0, 1], [0, 2], [1, 2]] {
        for shear in [0., 0.25] {
            for weighted in [false, true] {
                for shifted in [false, true] {
                    for amplitude in [1., 8., 32.] {
                        let a = [[1., shear], [-0.5, 1.]];
                        let offset = [8., -4.];
                        let axis = (0..3).find(|i| !axes.contains(i)).unwrap();
                        let mut s = Surface {
                            degree_u: 2,
                            degree_v: 2,
                            knots_u: if shifted {
                                vec![-0.1, -0.1, -0.1, 0.9, 0.9, 0.9]
                            } else {
                                vec![0., 0., 0., 1., 1., 1.]
                            },
                            knots_v: if shifted {
                                vec![0.13, 0.13, 0.13, 3.17, 3.17, 3.17]
                            } else {
                                vec![0., 0., 0., 1., 1., 1.]
                            },
                            control_points: vec![vec![vec![0.; 3]; 3]; 3],
                            weights: vec![vec![1.; 3]; 3],
                            periodic_u: false,
                            periodic_v: false,
                        };
                        for i in 0..3 {
                            for j in 0..3 {
                                let u = i as f64 / 2.;
                                let v = j as f64 / 2.;
                                for k in 0..2 {
                                    s.control_points[i][j][axes[k]] =
                                        a[k][0] * u + a[k][1] * v + offset[k];
                                }
                                s.control_points[i][j][axis] = amplitude * ((i * j) as f64 - 1.);
                                if weighted {
                                    s.weights[i][j] = (1. + i as f64 / 64.) * (1. + j as f64 / 32.);
                                }
                            }
                        }
                        let r = certify(&s, 100).unwrap();
                        cases.push(json!({"surface":s,"axes":axes,"matrix":a,"offset":offset,"proven":r.proven,
            "projection":r.projection,"contractionUpper":r.contraction_upper,"spans":r.spans,"reason":r.reason}));
                    }
                }
            }
        }
    }
    println!("{}", json!({"cases":cases}));
}
