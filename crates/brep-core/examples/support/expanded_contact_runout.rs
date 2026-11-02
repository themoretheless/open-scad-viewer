//! Constructive polynomial-control proposal. Every geometric gate is separate.
use brep_core::fillet_contact::Contact;
use num_rational::BigRational as Q;
use num_traits::{FromPrimitive, One, ToPrimitive, Zero};
use nurbs_core::{
    curve::Curve,
    surface::{Axis, Surface},
};
fn bin(n: usize, k: usize) -> i64 {
    if k > n {
        0
    } else {
        (0..k.min(n - k)).fold(1, |v, i| v * (n - i) as i64 / (i + 1) as i64)
    }
}
fn q(x: f64) -> Q {
    Q::from_f64(x).unwrap()
}
fn power_bernstein(power: &[Q], d: usize, a: f64, b: f64) -> Vec<Q> {
    let a = q(a);
    let step = q(b) - &a;
    let local = (0..=d)
        .map(|i| {
            (i..power.len())
                .map(|k| {
                    &power[k]
                        * Q::from_integer(bin(k, i).into())
                        * a.pow((k - i) as i32)
                        * step.pow(i as i32)
                })
                .sum::<Q>()
        })
        .collect::<Vec<_>>();
    (0..=d)
        .map(|i| {
            (0..=i)
                .map(|k| &local[k] * Q::new(bin(i, k).into(), bin(d, k).into()))
                .sum()
        })
        .collect()
}
fn elevate(c: &[Q], d: usize) -> Vec<Q> {
    let n = c.len() - 1;
    (0..=d)
        .map(|k| {
            (0..=n)
                .filter(|&i| i <= k && k - i <= d - n)
                .map(|i| &c[i] * Q::new((bin(n, i) * bin(d - n, k - i)).into(), bin(d, k).into()))
                .sum()
        })
        .collect()
}
fn product(a: &[Q], b: &[Q]) -> Vec<Q> {
    let n = a.len() - 1;
    let m = b.len() - 1;
    (0..=n + m)
        .map(|k| {
            (0..=n)
                .filter(|&i| i <= k && k - i <= m)
                .map(|i| {
                    &a[i]
                        * &b[k - i]
                        * Q::new((bin(n, i) * bin(m, k - i)).into(), bin(n + m, k).into())
                })
                .sum()
        })
        .collect()
}
fn height(x: f64) -> (f64, f64, f64) {
    let h = 280. / 262144. * x.powi(4) * (2. - x).powi(4);
    let log = 4. / x - 4. / (2. - x);
    (
        h,
        h * log,
        h * (log * log - 4. / (x * x) - 4. / ((2. - x) * (2. - x))),
    )
}
// Numerical proposals only. Original-support radius inspection is mandatory.
fn width(x: f64) -> (f64, f64) {
    let (h, hp, hpp) = height(x);
    let qp = 7. * x / 32.;
    let qpp = 7. / 32.;
    let mut lo = 0.;
    let mut hi = h / (1. + h * h).sqrt();
    for _ in 0..80 {
        let w = (lo + hi) * 0.5;
        if w == lo || w == hi {
            break;
        }
        let f = w * (1. + h * h + (qp - hp * w).powi(2)).sqrt() - h;
        if f > 0. {
            hi = w;
        } else {
            lo = w;
        }
    }
    let w = (lo + hi) * 0.5;
    let a = qp - hp * w;
    let d = 1. + h * h + a * a;
    let fw = 2. * w * d - 2. * hp * a * w * w;
    let fx = w * w * (2. * h * hp + 2. * a * (qpp - hpp * w)) - 2. * h * hp;
    (w, -fx / fw)
}
fn end_width(value: f64, first: f64, second: f64) -> Curve {
    let l = 0.125;
    let cp = [
        0.25,
        0.25,
        0.05,
        0.005,
        value - l * first / 2. + l * l * second / 12.,
        value - l * first / 4.,
        value,
    ];
    assert!(cp.iter().all(|&v| v > 0.));
    let c = Curve {
        degree: 4,
        knots: [vec![0.; 5], vec![0.375; 2], vec![0.5; 5]].concat(),
        control_points: cp.map(|v| vec![v, 0., 0.]).to_vec(),
        weights: vec![1.; 7],
        periodic: false,
    };
    c.validate().unwrap();
    c
}
pub struct Proposal {
    pub skin: Surface,
    pub storage_error: f64,
    pub pieces: usize,
    pub width_segments: Vec<Curve>,
}
pub fn build() -> Proposal {
    let xs = (0..=16).map(|i| 0.5 + i as f64 / 16.).collect::<Vec<_>>();
    let points = xs.iter().map(|&x| [width(x).0, 0., 0.]).collect::<Vec<_>>();
    let middle = nurbs_core::natural_spline::clamped(
        &points,
        &xs,
        [width(0.5).1, 0., 0.],
        [width(1.5).1, 0., 0.],
    )
    .unwrap();
    let left_jet = middle.evaluate(0.).unwrap();
    let right_jet = middle.evaluate(1.).unwrap();
    let left = end_width(
        left_jet.point[0],
        left_jet.d1.unwrap()[0],
        left_jet.d2.unwrap()[0],
    );
    let right = end_width(
        right_jet.point[0],
        -right_jet.d1.unwrap()[0],
        right_jet.d2.unwrap()[0],
    )
    .reverse()
    .unwrap();
    let mut pieces = Vec::new();
    for (c, offset) in [(&left, 0.), (&middle, 0.5), (&right, 1.5)] {
        for segment in c.decompose().unwrap() {
            let d = segment.domain();
            pieces.push((
                [offset + d[0], offset + d[1]],
                segment
                    .definition()
                    .control_points
                    .iter()
                    .map(|p| q(p[0]))
                    .collect::<Vec<_>>(),
            ));
        }
    }
    let mut hp = vec![Q::zero(); 9];
    for (i, c) in [16, -32, 24, -8, 1].into_iter().enumerate() {
        hp[i + 4] = Q::new((c * 280).into(), 262144.into());
    }
    let qp = vec![Q::zero(), Q::zero(), Q::new(7.into(), 64.into())];
    let phi = [
        Q::one(),
        Q::new(1.into(), 2.into()),
        Q::zero(),
        Q::new(1.into(), 2.into()),
        Q::one(),
    ];
    let mut rows = Vec::new();
    let mut knots = vec![0.; 13];
    for (bounds, w) in &pieces {
        let h = power_bernstein(&hp, 8, bounds[0], bounds[1]);
        let h_w = elevate(&product(&h, w), 12);
        let w = elevate(w, 12);
        let z = power_bernstein(&qp, 12, bounds[0], bounds[1]);
        let x = power_bernstein(&[q(bounds[0]), q(bounds[1]) - q(bounds[0])], 12, 0., 1.);
        let panel = (0..=12)
            .map(|i| {
                (0..=4)
                    .map(|j| {
                        vec![
                            x[i].to_f64().unwrap(),
                            (&w[i] * Q::new((j as i64 - 2).into(), 2.into()))
                                .to_f64()
                                .unwrap(),
                            (&z[i] - &h_w[i] * &phi[j]).to_f64().unwrap(),
                        ]
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        if let Some(last) = rows.last() {
            assert_eq!(last, &panel[0]);
        }
        rows.extend(panel.into_iter().skip(usize::from(!rows.is_empty())));
        knots.extend(std::iter::repeat_n(bounds[1], 12));
    }
    knots.push(2.);
    let raw = Surface {
        degree_u: 12,
        degree_v: 4,
        knots_u: knots,
        knots_v: [vec![0.; 5], vec![1.; 5]].concat(),
        weights: vec![vec![1.; 5]; rows.len()],
        control_points: rows,
        periodic_u: false,
        periodic_v: false,
    };
    raw.validate().unwrap();
    let mut target = vec![0.; 13];
    for (d, _) in pieces.iter().take(pieces.len() - 1) {
        target.extend(std::iter::repeat_n(d[1], 10));
    }
    target.extend(vec![2.; 13]);
    let compact = nurbs_core::foundation::approximate_edits::remove_surface_knots_report(
        &raw,
        Axis::U,
        target,
        2.5e-8,
        None,
    )
    .unwrap();
    assert!(
        compact.certificate.accepted,
        "C2 storage error {}",
        compact.certificate.error_upper
    );
    // Generate polynomial storage, then certify its ACTUAL coefficients against
    // the raw construction. The rational refit's receipt is not reused here.
    let mut polynomial=compact.surface;
    for row in &mut polynomial.weights { for weight in row {*weight=1.;} }
    let storage_error=nurbs_core::continuity::deviation::positional_upper(&raw,&polynomial).unwrap();
    assert!(storage_error<=2.5e-8,"Polynomial C2 storage error {storage_error}");
    let width_segments = pieces.iter().map(|(d,w)| Curve {
        degree:w.len()-1,
        knots:[vec![d[0];w.len()],vec![d[1];w.len()]].concat(),
        control_points:w.iter().map(|x|vec![x.to_f64().unwrap(),0.,0.]).collect(),
        weights:vec![1.;w.len()],periodic:false,
    }).collect();
    Proposal {
        width_segments,
        skin: polynomial,
        storage_error,
        pieces: pieces.len(),
    }
}
pub fn contacts(skin: &Surface, supports: [&Surface; 2]) -> [Contact; 2] {
    std::array::from_fn(|side| {
        let curve = skin.iso(Axis::V, side as f64).unwrap();
        let support_pcurve = Curve {
            control_points: curve
                .control_points
                .iter()
                .map(|p| vec![p[0], if side == 0 { -p[1] } else { p[1] }])
                .collect(),
            ..curve.clone()
        };
        let fillet_pcurve = Curve {
            degree: 1,
            knots: vec![0., 0., 2., 2.],
            control_points: vec![vec![0., side as f64], vec![2., side as f64]],
            weights: vec![1.; 2],
            periodic: false,
        };
        let positions = nurbs_core::surface_g2_pcurves::verify_positions(
            &curve,
            [&support_pcurve, &fillet_pcurve],
            [supports[side], skin],
            [false; 2],
            1e-7,
            100000,
            1000000,
        )
        .unwrap();
        assert!(positions.proven, "source contact {side} remains unresolved");
        Contact {
            curve,
            support_pcurve,
            fillet_pcurve,
            positions,
        }
    })
}
