use value_codec::json;
fn main() {
    let mut cases = Vec::new();
    for scale in [0.25, 1., 8.] {
        for shear in [0., 0.25] {
            for shift in [0., 16.] {
                for weight in [1., 2.] {
                    let mut m = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
                    let map = |p: &mut [f64]| {
                        let q = [p[0], p[1], p[2]];
                        p[0] = shift + scale * (q[0] + shear * q[1]);
                        p[1] = -shift + scale * (q[1] + shear * q[2]);
                        p[2] = shift + scale * (q[2] + shear * q[0]);
                    };
                    for v in &mut m.vertices {
                        map(&mut v.point);
                    }
                    for e in &mut m.edges {
                        for p in &mut e.curve.control_points {
                            map(p);
                        }
                        e.curve.weights.fill(weight);
                    }
                    for f in &mut m.faces {
                        for row in &mut f.surface.control_points {
                            for p in row {
                                map(p);
                            }
                        }
                        for row in &mut f.surface.weights {
                            row.fill(weight);
                        }
                    }
                    m.validate().unwrap();
                    let mut pairs = Vec::new();
                    for a in 0..6 {
                        for b in a + 1..6 {
                            if let Some(c) =
                                brep_core::shared_boundary::inspect_pair(&m, [a, b]).unwrap()
                            {
                                pairs.push(json!({"faces":[a,b],"edge":c.edge,"planarFace":c.planar_face,"sidedFace":c.sided_face}));
                            }
                        }
                    }
                    cases.push(json!({"model":m,"certificates":pairs}));
                }
            }
        }
    }
    println!("{}", json!({"cases":cases}));
}
