//! Independent cubic quotient jets and RK4 validate ODE derivative units.
//! Numerical observations never participate in the production proof.
use super::*;
type F = [f64; 3];
fn sum(a: F, b: F) -> F {
    std::array::from_fn(|k| a[k] + b[k])
}
fn diff(a: F, b: F) -> F {
    std::array::from_fn(|k| a[k] - b[k])
}
fn mul(a: F, s: f64) -> F {
    a.map(|x| x * s)
}
fn cp(a: F, b: F) -> F {
    math_core::cross(a, b)
}
fn dp(a: F, b: F) -> f64 {
    math_core::dot(a, b)
}
fn source(rational: bool) -> Curve {
    let mut p = crate::paths::bezier(
        vec![
            vec![0., 0., 0.],
            vec![0., 0., 16.],
            vec![0.125, 0., 48.],
            vec![0.25, 0.125, 64.],
        ],
        Some(if rational {
            vec![1., 1.125, 1.25, 1.]
        } else {
            vec![1.; 4]
        }),
    )
    .unwrap();
    for u in &mut p.knots {
        *u = -3. + 10. * *u;
    }
    p
}
fn jets(p: &Curve, t: f64) -> [F; 4] {
    let controls: Vec<[f64; 4]> = p
        .control_points
        .iter()
        .zip(&p.weights)
        .map(|(p, w)| [p[0] * w, p[1] * w, p[2] * w, *w])
        .collect();
    let mut h = [[0.; 4]; 4];
    let bases = [
        vec![
            (1. - t).powi(3),
            3. * t * (1. - t).powi(2),
            3. * t * t * (1. - t),
            t.powi(3),
        ],
        vec![(1. - t).powi(2), 2. * t * (1. - t), t * t],
        vec![1. - t, t],
        vec![1.],
    ];
    let mut c = controls;
    for order in 0..4 {
        for (i, p) in c.iter().enumerate() {
            for k in 0..4 {
                h[order][k] += bases[order][i] * p[k];
            }
        }
        if order < 3 {
            c = c
                .windows(2)
                .map(|p| std::array::from_fn(|k| (3 - order) as f64 * (p[1][k] - p[0][k])))
                .collect();
        }
    }
    let mut x = [[0.; 3]; 4];
    for k in 0..3 {
        x[0][k] = h[0][k] / h[0][3];
        x[1][k] = (h[1][k] - h[1][3] * x[0][k]) / h[0][3];
        x[2][k] = (h[2][k] - 2. * h[1][3] * x[1][k] - h[2][3] * x[0][k]) / h[0][3];
        x[3][k] = (h[3][k] - 3. * h[1][3] * x[2][k] - 3. * h[2][3] * x[1][k] - h[3][3] * x[0][k])
            / h[0][3];
    }
    x
}
fn omega(p: &Curve, t: f64) -> (F, F, F) {
    let [_, v, a, j] = jets(p, t);
    let q = dp(v, v);
    let w = mul(cp(v, a), 1. / q);
    let wd = diff(mul(cp(v, j), 1. / q), mul(w, 2. * dp(v, a) / q));
    (w, wd, mul(v, 1. / q.sqrt()))
}
fn oracle(p: &Curve) -> Vec<F> {
    let count = 8192;
    let h = 1. / count as f64;
    let mut n = [1., 0., 0.];
    let mut out = vec![n];
    let rhs = |t, n| cp(omega(p, t).0, n);
    for i in 0..count {
        let t = i as f64 * h;
        let k1 = rhs(t, n);
        let k2 = rhs(t + h / 2., sum(n, mul(k1, h / 2.)));
        let k3 = rhs(t + h / 2., sum(n, mul(k2, h / 2.)));
        let k4 = rhs(t + h, sum(n, mul(k3, h)));
        n = sum(
            n,
            mul(sum(sum(k1, mul(k2, 2.)), sum(mul(k3, 2.), k4)), h / 6.),
        );
        out.push(n);
    }
    out
}
fn contains(interval: [f64; 2], v: f64) {
    assert!(
        interval[0] - 1e-11 <= v && v <= interval[1] + 1e-11,
        "{v} outside {interval:?}"
    );
}
#[test]
fn original_normal_first_and_second_jets_contain_independent_rational_ode_oracles() {
    for rational in [false, true] {
        let p = source(rational);
        let before = p.clone();
        let normals = oracle(&p);
        let transport =
            certify_original_rmf_transport(&p, [1., 0., 0.], 128, 100000, 1000000, false).unwrap();
        assert_eq!(transport.status, Status::Certified);
        let mut twist = crate::primitives::line([0.; 3], [0.125, 0., 0.]).unwrap();
        twist.knots = vec![2., 2., 10., 10.];
        for i in 1..8 {
            let t = i as f64 / 8.;
            let proof =
                frame_jets(&transport, &p, &twist, [t - 0.0001, t + 0.0001], 10000).unwrap();
            assert_eq!(proof.status, Status::Certified);
            let (w, wd, tangent) = omega(&p, t);
            let raw = normals[i * 1024];
            let b = cp(tangent, raw);
            let theta = 0.125 * t;
            let n = sum(mul(raw, theta.cos()), mul(b, theta.sin()));
            let td = cp(w, tangent);
            let tdd = sum(cp(wd, tangent), cp(w, td));
            let spin = sum(w, mul(tangent, 0.125));
            let spin_d = sum(wd, mul(td, 0.125));
            let nd = cp(spin, n);
            let ndd = sum(cp(spin_d, n), cp(spin, nd));
            let binormal = cp(tangent, n);
            let bd = sum(cp(td, n), cp(tangent, nd));
            let bdd = sum(sum(cp(tdd, n), mul(cp(td, nd), 2.)), cp(tangent, ndd));
            for (report, values) in [
                (proof.longitudinal.as_ref().unwrap(), [tangent, td, tdd]),
                (proof.transverse.as_ref().unwrap(), [n, nd, ndd]),
                (proof.binormal.as_ref().unwrap(), [binormal, bd, bdd]),
            ] {
                for k in 0..3 {
                    contains(report.value[k], values[0][k]);
                    contains(report.first[k], values[1][k]);
                    contains(report.second[k], values[2][k]);
                }
            }
            let short = frame_jets(
                &transport,
                &p,
                &twist,
                [t - 0.0001, t + 0.0001],
                proof.cells - 1,
            )
            .unwrap();
            assert_eq!(short.status, Status::Unresolved);
            assert!(
                short.longitudinal.is_none()
                    && short.transverse.is_none()
                    && short.binormal.is_none()
            );
        }
        assert_eq!(p, before);
    }
}
#[test]
fn original_bishop_jets_need_a_complete_open_cover_and_available_query_work() {
    let p = source(false);
    let twist = crate::sweeps::progressive_sweep::constant_vector_law([0.; 3]).unwrap();
    let short = certify_original_rmf_transport(&p, [1., 0., 0.], 128, 10, 1000000, false).unwrap();
    assert_eq!(short.status, Status::Unresolved);
    assert_eq!(
        frame_jets(&short, &p, &twist, [0.25, 0.75], 10000)
            .unwrap()
            .status,
        Status::Unresolved
    );
    let full =
        certify_original_rmf_transport(&p, [1., 0., 0.], 128, 100000, 1000000, false).unwrap();
    let zero = frame_jets(&full, &p, &twist, [0.25, 0.75], 0).unwrap();
    assert_eq!(zero.status, Status::Unresolved);
    assert_eq!(zero.cells, 0);
    // A closing correction is a different ODE; raw open jets cannot certify it.
    let mut closed = full;
    closed.closing_angle = Some([0., 0.]);
    assert_eq!(
        frame_jets(&closed, &p, &twist, [0.25, 0.75], 10000)
            .unwrap()
            .status,
        Status::Unresolved
    );
}
