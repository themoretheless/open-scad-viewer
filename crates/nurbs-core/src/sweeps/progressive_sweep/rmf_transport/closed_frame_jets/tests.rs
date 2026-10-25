//! Independent Bernstein quotient/RK4/Simpson oracle for CLOSED holonomy jets.
//! These numerical observations never participate in source admission.
use super::*;
type F = [f64; 3];
fn add(a: F, b: F) -> F {
    std::array::from_fn(|k| a[k] + b[k])
}
fn sub(a: F, b: F) -> F {
    std::array::from_fn(|k| a[k] - b[k])
}
fn mul(a: F, s: f64) -> F {
    a.map(|x| x * s)
}
fn cross(a: F, b: F) -> F {
    math_core::cross(a, b)
}
fn dot(a: F, b: F) -> f64 {
    math_core::dot(a, b)
}
fn source(rational: bool) -> Curve {
    Curve {
        degree: 9,
        knots: [vec![-3.; 10], vec![5.; 10]].concat(),
        control_points: vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![2., 1., 0.],
            vec![3., 3., 1.],
            vec![2., 4., 6.],
            vec![-2., 4., -4.],
            vec![-3., 3., -1.],
            vec![-2., 1., 0.],
            vec![-1., 0., 0.],
            vec![0., 0., 0.],
        ],
        weights: if rational {
            vec![1., 1., 1., 1., 1.25, 1.125, 1., 1., 1., 1.]
        } else {
            vec![1.; 10]
        },
        periodic: false,
    }
}
fn jets(p: &Curve, t: f64) -> [F; 4] {
    let mut c: Vec<[f64; 4]> = p
        .control_points
        .iter()
        .zip(&p.weights)
        .map(|(p, w)| [p[0] * w, p[1] * w, p[2] * w, *w])
        .collect();
    let mut h = [[0.; 4]; 4];
    for (order, row) in h.iter_mut().enumerate() {
        let d = p.degree - order;
        let mut choose = 1.;
        for (i, v) in c.iter().enumerate() {
            if i > 0 {
                choose *= (d + 1 - i) as f64 / i as f64;
            }
            let b = choose * t.powi(i as i32) * (1. - t).powi((d - i) as i32);
            for k in 0..4 {
                row[k] += b * v[k];
            }
        }
        if order < 3 {
            c = c
                .windows(2)
                .map(|v| std::array::from_fn(|k| d as f64 * (v[1][k] - v[0][k])))
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
fn fields(p: &Curve, t: f64) -> (F, F, F, f64, f64) {
    let [_, v, a, j] = jets(p, t);
    let q = dot(v, v);
    let speed = q.sqrt();
    let omega = mul(cross(v, a), 1. / q);
    let rate = sub(mul(cross(v, j), 1. / q), mul(omega, 2. * dot(v, a) / q));
    (omega, rate, mul(v, 1. / speed), speed, dot(v, a) / speed)
}
fn oracle(p: &Curve) -> (Vec<F>, Vec<f64>, f64) {
    let count = 8192;
    let h = 1. / count as f64;
    let mut n = [0., 0., 1.];
    let mut normals = vec![n];
    let mut lengths = vec![0.];
    let mut length = 0.;
    let rhs = |t, n| cross(fields(p, t).0, n);
    for i in 0..count {
        let t = i as f64 * h;
        let k1 = rhs(t, n);
        let k2 = rhs(t + h / 2., add(n, mul(k1, h / 2.)));
        let k3 = rhs(t + h / 2., add(n, mul(k2, h / 2.)));
        let k4 = rhs(t + h, add(n, mul(k3, h)));
        n = add(
            n,
            mul(add(add(k1, mul(k2, 2.)), add(mul(k3, 2.), k4)), h / 6.),
        );
        length += h / 6. * (fields(p, t).3 + 4. * fields(p, t + h / 2.).3 + fields(p, t + h).3);
        normals.push(n);
        lengths.push(length);
    }
    let angle = dot(fields(p, 0.).2, cross(n, [0., 0., 1.])).atan2(dot(n, [0., 0., 1.]));
    (normals, lengths, angle)
}
fn contains(range: [f64; 2], value: f64) {
    assert!(
        range[0] - 1e-10 <= value && value <= range[1] + 1e-10,
        "{value} outside {range:?}"
    );
}
#[test]
fn corrected_original_closed_jets_contain_nonzero_holonomy_ode_oracles() {
    for rational in [false, true] {
        let path = source(rational);
        let before = path.clone();
        let (normals, lengths, angle) = oracle(&path);
        let total = *lengths.last().unwrap();
        assert!(
            angle.abs() > 0.05,
            "The correction must be nontrivial: {angle}"
        );
        let transport =
            certify_original_rmf_transport(&path, [0., 0., 1.], 1024, 100000, 1000000, true)
                .unwrap();
        assert_eq!(transport.status, Status::Certified, "{transport:?}");
        contains(transport.original_arc_length.unwrap(), total);
        contains(transport.closing_angle.unwrap(), angle);
        let twist = Curve {
            degree: 4,
            knots: [vec![7.; 5], vec![11.; 5]].concat(),
            control_points: [0., 0., 1. / 64., 0., 0.].map(|x| vec![x, 0., 0.]).to_vec(),
            weights: vec![1.; 5],
            periodic: false,
        };
        for i in 0..=8 {
            let t = i as f64 / 8.;
            let range = [(t - 0.0001).max(0.), (t + 0.0001).min(1.)];
            let proof = closed_frame_jets(&transport, &path, &twist, range, 10000).unwrap();
            assert_eq!(proof.status, Status::Certified, "{proof:?}");
            let (w, wd, tangent, speed, speed_d) = fields(&path, t);
            let beta = 3. / 32. * t * t * (1. - t).powi(2);
            let bd = 3. / 16. * t * (1. - t) * (1. - 2. * t);
            let bdd = 3. / 16. * (1. - 6. * t + 6. * t * t);
            let phase = angle * lengths[i * 1024] / total + beta;
            let raw = normals[i * 1024];
            let n = add(mul(raw, phase.cos()), mul(cross(tangent, raw), phase.sin()));
            let td = cross(w, tangent);
            let tdd = add(cross(wd, tangent), cross(w, td));
            let rate = angle * speed / total + bd;
            let rate_d = angle * speed_d / total + bdd;
            let spin = add(w, mul(tangent, rate));
            let spin_d = add(add(wd, mul(tangent, rate_d)), mul(td, rate));
            let nd = cross(spin, n);
            let ndd = add(cross(spin_d, n), cross(spin, nd));
            let b = cross(tangent, n);
            let b1 = add(cross(td, n), cross(tangent, nd));
            let b2 = add(
                add(cross(tdd, n), mul(cross(td, nd), 2.)),
                cross(tangent, ndd),
            );
            for (report, values) in [
                (proof.longitudinal.as_ref().unwrap(), [tangent, td, tdd]),
                (proof.transverse.as_ref().unwrap(), [n, nd, ndd]),
                (proof.binormal.as_ref().unwrap(), [b, b1, b2]),
            ] {
                for k in 0..3 {
                    for (range, value) in [report.value[k], report.first[k], report.second[k]]
                        .into_iter()
                        .zip([values[0][k], values[1][k], values[2][k]])
                    {
                        contains(range, value);
                    }
                }
            }
            let short =
                closed_frame_jets(&transport, &path, &twist, range, proof.cells - 1).unwrap();
            assert_eq!(short.status, Status::Unresolved);
            assert!(
                short.longitudinal.is_none()
                    && short.transverse.is_none()
                    && short.binormal.is_none()
            );
        }
        assert_eq!(path, before);
        eprintln!("closed independent holonomy rational={rational}: angle={angle}, length={total}");
    }
}
#[test]
fn corrected_jets_refuse_raw_partial_gapped_and_lengthless_covers() {
    let path = source(false);
    let twist = crate::sweeps::progressive_sweep::constant_vector_law([0.; 3]).unwrap();
    let full =
        certify_original_rmf_transport(&path, [0., 0., 1.], 128, 100000, 1000000, true).unwrap();
    assert_eq!(full.status, Status::Certified);
    let check = |r: &OriginalRmfTransportReport| max_refuse(r, &path, &twist);
    let raw =
        certify_original_rmf_transport(&path, [0., 0., 1.], 128, 100000, 1000000, false).unwrap();
    check(&raw);
    let mut missing = full.clone();
    missing.original_arc_length = None;
    check(&missing);
    let mut zero = full.clone();
    zero.original_arc_length = Some([0., 0.]);
    check(&zero);
    let mut partial = full.clone();
    partial.arc_corrected_normals.as_mut().unwrap().pop();
    check(&partial);
    let mut gap = full.clone();
    gap.cover.as_mut().unwrap()[64].traversal[0] += 1. / 256.;
    check(&gap);
    let short =
        certify_original_rmf_transport(&path, [0., 0., 1.], 128, 20, 1000000, true).unwrap();
    check(&short);
    assert_eq!(
        closed_frame_jets(&full, &path, &twist, [0.4, 0.6], 0)
            .unwrap()
            .cells,
        0
    );
    assert_eq!(
        super::super::frame_jets(&full, &path, &twist, [0.4, 0.6], 10000)
            .unwrap()
            .status,
        Status::Unresolved
    );
}
fn max_refuse(r: &OriginalRmfTransportReport, path: &Curve, twist: &Curve) {
    let p = closed_frame_jets(r, path, twist, [0.4, 0.6], 10000).unwrap();
    assert_eq!(p.status, Status::Unresolved);
    assert!(p.longitudinal.is_none() && p.transverse.is_none() && p.binormal.is_none());
}

use crate as nurbs;
#[allow(dead_code)]
#[path = "../../../../../tests/support/arc_bishop_source.rs"]
mod geometric_clock_case;
#[test]
fn geometric_arc_closure_matches_independent_ode_and_station_derivatives_through_second_order() {
    let path = geometric_clock_case::path(true);
    let base = geometric_clock_case::path(false);
    let (normals, lengths, angle) = oracle(&path);
    let (_, base_lengths, base_angle) = oracle(&base);
    let total = *lengths.last().unwrap();
    assert!((total - base_lengths.last().unwrap()).abs() < 1e-8);
    assert!((angle - base_angle).abs() < 1e-8 && angle.abs() > 0.05);
    for i in 0..=16 {
        let t = i as f64 / 16.;
        let a = jets(&path, t)[0];
        let b = jets(&base, 2. * t / (1. + t))[0];
        for k in 0..3 {
            assert!((a[k] - b[k]).abs() < 1e-12);
        }
    }
    let transport =
        certify_original_arc_rmf_transport(&path, [0., 0., 1.], 1024, 100000, 1000000, true)
            .unwrap();
    assert_eq!(transport.status, Status::Certified);
    contains(transport.original_arc_length.unwrap(), total);
    contains(transport.closing_angle.unwrap(), angle);
    let twist = crate::progressive_sweep::constant_vector_law([0.; 3]).unwrap();
    let mut ends = Vec::new();
    for i in 0..=8 {
        let t = i as f64 / 8.;
        let range = [(t - 0.0001).max(0.), (t + 0.0001).min(1.)];
        let proof = closed_frame_jets(&transport, &path, &twist, range, 10000).unwrap();
        assert_eq!(proof.status, Status::Certified, "{proof:?}");
        let (w, wd, tangent, speed, speed_d) = fields(&path, t);
        let phase = angle * lengths[i * 1024] / total;
        let raw = normals[i * 1024];
        let n = add(mul(raw, phase.cos()), mul(cross(tangent, raw), phase.sin()));
        let td = cross(w, tangent);
        let tdd = add(cross(wd, tangent), cross(w, td));
        let rate = angle * speed / total;
        let rate_d = angle * speed_d / total;
        let spin = add(w, mul(tangent, rate));
        let spin_d = add(add(wd, mul(tangent, rate_d)), mul(td, rate));
        let nd = cross(spin, n);
        let ndd = add(cross(spin_d, n), cross(spin, nd));
        let b = cross(tangent, n);
        let b1 = add(cross(td, n), cross(tangent, nd));
        let b2 = add(
            add(cross(tdd, n), mul(cross(td, nd), 2.)),
            cross(tangent, ndd),
        );
        let values = [[tangent, td, tdd], [n, nd, ndd], [b, b1, b2]];
        for (report, v) in [
            proof.longitudinal.as_ref().unwrap(),
            proof.transverse.as_ref().unwrap(),
            proof.binormal.as_ref().unwrap(),
        ]
        .into_iter()
        .zip(values)
        {
            for k in 0..3 {
                contains(report.value[k], v[0][k]);
                contains(report.first[k], v[1][k]);
                contains(report.second[k], v[2][k]);
            }
        }
        if i == 0 || i == 8 {
            let first = total / speed;
            let second = -total * total * speed_d / (speed * speed * speed);
            ends.push(values.map(|v| {
                [
                    v[0],
                    mul(v[1], first),
                    add(mul(v[2], first * first), mul(v[1], second)),
                ]
            }));
        }
    }
    for axis in 0..3 {
        for order in 0..3 {
            for k in 0..3 {
                assert!(
                    (ends[0][axis][order][k] - ends[1][axis][order][k]).abs() < 1e-6,
                    "axis={axis} derivative={order} coordinate={k}"
                );
            }
        }
    }
    assert!(!transport.continuous_bound && !transport.solid_certified);
}
