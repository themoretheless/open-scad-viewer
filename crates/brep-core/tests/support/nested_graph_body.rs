//! Independently authored exact quartic graph, with two concentric zero sets.
use num_rational::BigRational as Q;
use num_traits::{ToPrimitive, Zero};
fn q(x: f64) -> Q {
    Q::from_float(x).unwrap()
}
fn mul(a: &[[Q; 5]; 5], b: &[[Q; 5]; 5]) -> [[Q; 5]; 5] {
    let mut out = std::array::from_fn(|_| std::array::from_fn(|_| Q::zero()));
    for i in 0..5 {
        for j in 0..5 {
            for k in 0..5 - i {
                for l in 0..5 - j {
                    out[i + k][j + l] += &a[i][j] * &b[k][l];
                }
            }
        }
    }
    out
}
fn control(o: &[f64], u: [f64; 3], v: [f64; 3]) -> Vec<Vec<Vec<f64>>> {
    let affine = |k: usize| {
        let mut p = std::array::from_fn(|_| std::array::from_fn(|_| Q::zero()));
        p[0][0] = q(o[k] - 0.5);
        p[1][0] = q(u[k]);
        p[0][1] = q(v[k]);
        p
    };
    let x = affine(0);
    let y = affine(1);
    let mut a = mul(&x, &x);
    let yy = mul(&y, &y);
    for i in 0..5 {
        for j in 0..5 {
            a[i][j] += &yy[i][j];
        }
    }
    let mut b = a.clone();
    a[0][0] -= q(0.3125_f64.powi(2));
    b[0][0] -= q(0.15625_f64.powi(2));
    let mut p = mul(&a, &b);
    for row in &mut p {
        for x in row {
            *x *= q(36.);
        }
    }
    p[0][0] += q(o[2]);
    p[1][0] += q(u[2]);
    p[0][1] += q(v[2]);
    let choose = |n: usize, k: usize| -> i64 {
        if k > n {
            return 0;
        }
        let mut a = 1;
        for i in 0..k {
            a = a * (n - i) as i64 / (i + 1) as i64;
        }
        a
    };
    (0..5)
        .map(|i| {
            (0..5)
                .map(|j| {
                    let mut z = Q::zero();
                    for a in 0..=i {
                        for b in 0..=j {
                            z += &p[a][b]
                                * Q::new(choose(i, a).into(), choose(4, a).into())
                                * Q::new(choose(j, b).into(), choose(4, b).into());
                        }
                    }
                    let value = z.to_f64().unwrap();
                    assert_eq!(q(value), z, "Fixture must remain exactly dyadic");
                    vec![
                        o[0] + u[0] * i as f64 / 4. + v[0] * j as f64 / 4.,
                        o[1] + u[1] * i as f64 / 4. + v[1] * j as f64 / 4.,
                        value,
                    ]
                })
                .collect()
        })
        .collect()
}
pub fn body() -> brep_core::Model {
    let mut m = brep_core::cuboid([0., 0., -10.], [1., 1., 0.]).unwrap();
    let knots = || vec![0., 0., 0., 0., 0., 1., 1., 1., 1., 1.];
    for v in &mut m.vertices {
        let r = (v.point[0] - 0.5).powi(2) + (v.point[1] - 0.5).powi(2);
        v.point[2] += 36. * (r - 0.3125_f64.powi(2)) * (r - 0.15625_f64.powi(2));
    }
    for e in &mut m.edges {
        let p = e.curve.control_points[0].clone();
        let end = &e.curve.control_points[1];
        let u = std::array::from_fn(|k| end[k] - p[k]);
        let net = control(&p, u, [0.; 3]);
        e.curve = nurbs_core::curve::Curve {
            degree: 4,
            knots: knots(),
            control_points: net.into_iter().map(|row| row[0].clone()).collect(),
            weights: vec![1.; 5],
            periodic: false,
        };
    }
    for f in &mut m.faces {
        let o = &f.surface.control_points[0][0];
        let u = std::array::from_fn(|k| f.surface.control_points[1][0][k] - o[k]);
        let v = std::array::from_fn(|k| f.surface.control_points[0][1][k] - o[k]);
        f.surface = nurbs_core::surface::Surface {
            degree_u: 4,
            degree_v: 4,
            knots_u: knots(),
            knots_v: knots(),
            control_points: control(o, u, v),
            weights: vec![vec![1.; 5]; 5],
            periodic_u: false,
            periodic_v: false,
        };
    }
    m.rebuild_topology_ids();
    m.validate().unwrap();
    m
}
