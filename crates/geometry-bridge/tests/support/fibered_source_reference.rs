//! Independent floating source-flux reference for STEP qualification fixtures.
//! This is numerical quadrature, not a certified continuous geometry bound.
use value_codec::{Value, json};
fn legendre(n: usize, z: f64) -> (f64, f64) {
    let (mut p, mut q) = (1., 0.);
    for j in 1..=n {
        let old = p;
        p = ((2 * j - 1) as f64 * z * p - (j - 1) as f64 * q) / j as f64;
        q = old;
    }
    (p, n as f64 * (z * p - q) / (z * z - 1.))
}
fn rule(n: usize) -> Vec<(f64, f64)> {
    (0..n)
        .map(|i| {
            let mut z = (std::f64::consts::PI * (i as f64 + 0.75) / (n as f64 + 0.5)).cos();
            for _ in 0..64 {
                let (p, d) = legendre(n, z);
                let next = z - p / d;
                if (next - z).abs() < 2e-15 {
                    z = next;
                    break;
                }
                z = next;
            }
            let (p, d) = legendre(n, z);
            assert!(p.abs() < 1e-12);
            ((z + 1.) / 2., 1. / ((1. - z * z) * d * d))
        })
        .collect()
}
fn casteljau(mut c: Vec<Vec<f64>>, t: f64) -> Vec<f64> {
    for n in (1..c.len()).rev() {
        for i in 0..n {
            for k in 0..c[0].len() {
                c[i][k] = (1. - t) * c[i][k] + t * c[i + 1][k];
            }
        }
    }
    c.remove(0)
}
fn rational(cp: &[Vec<f64>], w: &[f64], t: f64) -> (Vec<f64>, Vec<f64>) {
    let d = cp[0].len();
    let n = cp.len() - 1;
    let h = cp
        .iter()
        .zip(w)
        .map(|(p, w)| {
            p.iter()
                .map(|x| x * w)
                .chain(std::iter::once(*w))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let dh = h
        .windows(2)
        .map(|p| {
            p[1].iter()
                .zip(&p[0])
                .map(|(a, b)| n as f64 * (a - b))
                .collect()
        })
        .collect();
    let a = casteljau(h, t);
    let b = casteljau(dh, t);
    (
        (0..d).map(|k| a[k] / a[d]).collect(),
        (0..d)
            .map(|k| (b[k] * a[d] - a[k] * b[d]) / (a[d] * a[d]))
            .collect(),
    )
}
fn apply(p: &[f64], q: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|k| p[9 + k] + p[k] * q[0] + p[3 + k] * q[1] + p[6 + k] * q[2])
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|k| a[k] * b[k]).sum()
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn poses(piece: &Value) -> (Vec<Vec<f64>>, Vec<f64>) {
    let raw: Vec<[[f64; 3]; 4]> = value_codec::from_value(piece["controls"].clone()).unwrap();
    (
        raw.iter()
            .map(|p| p.iter().flatten().copied().collect())
            .collect(),
        value_codec::from_value(piece["weights"].clone()).unwrap(),
    )
}
fn integrate(input: &Value, n: usize) -> f64 {
    // These four fixtures have the same radius-two disk, center at origin.
    // Integral of the A'(t)Q determinant term vanishes by its zero centroid.
    let rule = rule(n);
    let mut volume = 0.;
    for piece in input["source"]["motion"].as_array().unwrap() {
        let (cp, w) = poses(piece);
        for (t, wt) in &rule {
            let (a, d) = rational(&cp, &w, *t);
            let x = [a[0], a[1], a[2]];
            let y = [a[3], a[4], a[5]];
            let dt = [d[9], d[10], d[11]];
            volume += 4. * std::f64::consts::PI * wt * dot(cross(x, y), dt);
        }
    }
    volume.abs()
}
pub fn reference(input: &Value) -> Value {
    // Prevent reuse of the disk volume formula for a different material.
    assert_eq!(
        input["source"]["profiles"],
        super::support::request()["source"]["profiles"]
    );
    let a = integrate(input, 16);
    let b = integrate(input, 32);
    let delta = (a - b).abs();
    assert!(
        b > 0. && delta < 1e-7,
        "original source quadrature: {a} {b} delta {delta}"
    );
    let profiles: Vec<Vec<nurbs_core::curve::Curve>> =
        value_codec::from_value(input["source"]["profiles"].clone()).unwrap();
    let mut samples = Vec::new();
    let pieces = input["source"]["motion"].as_array().unwrap();
    for piece in pieces {
        let (cp, w) = poses(piece);
        for c in profiles.iter().flatten() {
            for s in [0., 0.25, 0.5, 0.75, 1.] {
                let (q, _) = rational(&c.control_points, &c.weights, s);
                for t in [0., 0.125, 0.375, 0.625, 0.875, 1.] {
                    let (a, _) = rational(&cp, &w, t);
                    samples.push(apply(&a, [q[0], q[1], q[2]]));
                }
            }
        }
    }
    for (piece, t) in [(&pieces[0], 0.), (&pieces[pieces.len() - 1], 1.)] {
        let (cp, w) = poses(piece);
        let (a, _) = rational(&cp, &w, t);
        for q in [
            [0.; 3],
            [1., 0., 0.],
            [-1., 0., 0.],
            [0., 1., 0.],
            [0., -1., 0.],
        ] {
            samples.push(apply(&a, q));
        }
    }
    json!({"schema":"original-fibered-disk-numerical-reference/1","orders":[16,32],"volumeMm3":[a,b],"deltaMm3":delta,"samplesMm":samples,"passed":true,
        "scope":"Numerical original radius-two disk fiber determinant integral and original boundary points; supplementary to the separate exact native theorem"})
}
