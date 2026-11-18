//! Constructive axis-aligned corner proposals. Fresh geometric guards required.
use nurbs_core::{Error, Result, surface::Surface};
use std::collections::BTreeMap;
const N: usize = 7;
pub struct CornerPatchNetwork {
    pub caps: Vec<Surface>,
    /// Strip i runs along coordinate axis i. U is the cyclic cross-section.
    pub strips: Vec<Surface>,
    pub cap_nodes: [[usize; 4]; 3],
    pub barycentric_nodes: [[f64; 3]; 7],
}
fn check(ok: bool, message: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(Error::new("BREP_CORNER_PATCH_PROPOSAL", message))
    }
}
fn choose(n: usize, k: usize) -> f64 {
    if k > n {
        return 0.;
    }
    (0..k.min(n - k)).fold(1., |a, i| a * (n - i) as f64 / (i + 1) as f64)
}
fn elevate(mut p: Vec<f64>, n: usize) -> Vec<f64> {
    while p.len() <= n {
        let d = p.len();
        let mut q = vec![0.; d + 1];
        q[0] = p[0];
        q[d] = p[d - 1];
        for i in 1..d {
            let a = i as f64 / d as f64;
            q[i] = a * p[i - 1] + (1. - a) * p[i];
        }
        p = q;
    }
    p
}
fn power_coefficients(p: &[f64], n: usize) -> Vec<f64> {
    (0..=n)
        .map(|i| {
            (0..=i.min(p.len() - 1))
                .map(|k| p[k] * choose(i, k) / choose(n, k))
                .sum()
        })
        .collect()
}
fn product(a: &[Vec<f64>], b: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let p = a.len() - 1;
    let q = b.len() - 1;
    let mut out = vec![vec![0.; p + q + 1]; p + q + 1];
    for i in 0..=p {
        for j in 0..=p {
            for k in 0..=q {
                for l in 0..=q {
                    out[i + k][j + l] += a[i][j] * b[k][l] * choose(p, i) * choose(q, k)
                        / choose(p + q, i + k)
                        * choose(p, j)
                        * choose(q, l)
                        / choose(p + q, j + l);
                }
            }
        }
    }
    out
}
fn bezier(control_points: Vec<Vec<Vec<f64>>>) -> Surface {
    let u = control_points.len() - 1;
    let v = control_points[0].len() - 1;
    Surface {
        degree_u: u,
        degree_v: v,
        knots_u: vec![0.; u + 1].into_iter().chain(vec![1.; u + 1]).collect(),
        knots_v: vec![0.; v + 1].into_iter().chain(vec![1.; v + 1]).collect(),
        weights: vec![vec![1.; v + 1]; u + 1],
        control_points,
        periodic_u: false,
        periodic_v: false,
    }
}
/// Seven-degree triangular boundary-jet construction, represented by three
/// regular bilinear quadrilateral charts. The three tapered quintic strips
/// share the common cap profile and have flat first/second radius derivatives
/// at both axial ends. No solver, proximity weld, or inherited receipt.
/// This is an explicit coordinate-plane family, not a general NURBS fillet.
pub fn propose_corner_patch_network(
    radius: f64,
    far_radii: [f64; 3],
    stop: f64,
    handle: f64,
) -> Result<CornerPatchNetwork> {
    check(
        radius.is_finite()
            && radius > 0.
            && radius <= 1e6
            && stop.is_finite()
            && stop > radius
            && stop <= 1e6
            && far_radii
                .iter()
                .all(|r| r.is_finite() && *r > 0. && *r <= radius)
            && handle.is_finite()
            && (0.1..=0.3).contains(&handle),
        "Invalid corner dimensions or handle",
    )?;
    let h = handle * 2f64.sqrt() * radius;
    let g = 5. * h;
    let f = vec![0., 0., 0., radius - 2. * h, radius - h, radius];
    let mut net = BTreeMap::<[usize; 3], [f64; 3]>::new();
    for missing in 0..3 {
        for layer in 0..=2 {
            let d = N - layer;
            let y = elevate(f.clone(), d);
            let x = y.iter().copied().rev().collect::<Vec<_>>();
            let lambda = power_coefficients(&[g, g, g, -4. * g, 2. * g], d);
            let mu = power_coefficients(&[0., 2. * g, 4. * g, -12. * g, 6. * g], d);
            for i in 0..=d {
                let mut role = [0; 3];
                role[missing] = layer;
                role[(missing + 1) % 3] = i;
                role[(missing + 2) % 3] = d - i;
                let z = match layer {
                    0 => radius,
                    1 => radius - lambda[i] / N as f64,
                    _ => radius - 2. * lambda[i] / N as f64 + mu[i] / (N * (N - 1)) as f64,
                };
                let mut point = [0.; 3];
                point[missing] = z;
                point[(missing + 1) % 3] = x[i];
                point[(missing + 2) % 3] = y[i];
                if let Some(old) = net.get(&role) {
                    check(
                        (0..3).all(|k| (old[k] - point[k]).abs() <= radius * 1e-12),
                        "Triangular boundary jets conflict",
                    )?;
                } else {
                    net.insert(role, point);
                }
            }
        }
    }
    check(
        net.len() == (N + 1) * (N + 2) / 2,
        "Incomplete triangular control ownership",
    )?;
    let nodes = [
        [1., 0., 0.],
        [0., 1., 0.],
        [0., 0., 1.],
        [0., 0.5, 0.5],
        [0.5, 0., 0.5],
        [0.5, 0.5, 0.],
        [0.25, 0.375, 0.375],
    ];
    let roles = [[0, 5, 6, 4], [1, 3, 6, 5], [2, 4, 6, 3]];
    let mut caps = Vec::new();
    for quad in roles {
        let maps = (0..3)
            .map(|k| {
                vec![
                    vec![nodes[quad[0]][k], nodes[quad[3]][k]],
                    vec![nodes[quad[1]][k], nodes[quad[2]][k]],
                ]
            })
            .collect::<Vec<_>>();
        let mut cp = vec![vec![vec![0.; 3]; N + 1]; N + 1];
        for (&exponent, &point) in &net {
            let mut polynomial = vec![vec![1.]];
            for axis in 0..3 {
                for _ in 0..exponent[axis] {
                    polynomial = product(&polynomial, &maps[axis]);
                }
            }
            let factor = choose(N, exponent[0]) * choose(N - exponent[0], exponent[1]);
            for i in 0..=N {
                for j in 0..=N {
                    for k in 0..3 {
                        cp[i][j][k] += factor * polynomial[i][j] * point[k];
                    }
                }
            }
        }
        let s = bezier(cp);
        s.validate()?;
        caps.push(s);
    }
    let unit = [
        0.,
        0.,
        0.,
        1. - 2. * handle * 2f64.sqrt(),
        1. - handle * 2f64.sqrt(),
        1.,
    ];
    let mut strips = Vec::new();
    for axis in 0..3 {
        let radii = [
            radius,
            radius,
            radius,
            far_radii[axis],
            far_radii[axis],
            far_radii[axis],
        ];
        let mut cp = vec![vec![vec![0.; 3]; 6]; 6];
        for i in 0..6 {
            for j in 0..6 {
                cp[i][j][axis] = radius + (stop - radius) * j as f64 / 5.;
                cp[i][j][(axis + 1) % 3] = unit[5 - i] * radii[j];
                cp[i][j][(axis + 2) % 3] = unit[i] * radii[j];
            }
        }
        let s = bezier(cp);
        s.validate()?;
        strips.push(s);
    }
    Ok(CornerPatchNetwork {
        caps,
        strips,
        cap_nodes: roles,
        barycentric_nodes: nodes,
    })
}

mod exact_coefficients;
/// Binary-compatible variant of the same bounded family. Inputs are explicitly
/// quantised to 2205 * 2^-32 mm; the authored cap/strip coefficients then have
/// exact common boundary polynomials despite different Bernstein degrees.
/// Quantisation is a geometry change, not a tolerance or certificate upgrade.
/// Geometry admission, including curvature, must be repeated on this output.
pub fn propose_binary_corner_patch_network(
    radius: f64,
    far_radii: [f64; 3],
    stop: f64,
    handle: f64,
) -> Result<CornerPatchNetwork> {
    let mut out = propose_corner_patch_network(radius, far_radii, stop, handle)?;
    check(
        radius <= 100. && stop <= 100.,
        "Binary-compatible family is limited to 100 mm",
    )?;
    let quant = |v: f64| -> i64 { (v * 4294967296. / 2205.).round() as i64 };
    let r = quant(radius);
    let h = quant(radius * handle * 2f64.sqrt());
    check(
        r > 0 && h > 0 && 2 * h < r,
        "Quantisation collapsed the central profile",
    )?;
    for f in 0..3 {
        for i in 0..8 {
            for j in 0..8 {
                for k in 0..3 {
                    let [a, b] = exact_coefficients::COEFFICIENTS[f][i][j][k];
                    let numerator = a as i128 * r as i128 + b as i128 * h as i128;
                    check(
                        numerator.abs() <= 9007199254740992,
                        "Binary cap exceeds exact integer allowance",
                    )?;
                    out.caps[f].control_points[i][j][k] =
                        numerator as f64 * (1. / 2251799813685248.);
                }
            }
        }
    }
    let stop_i = 5 * (stop * 4294967296. / 5.).round() as i64;
    let r_i = 2205 * r;
    let h_i = 2205 * h;
    check(stop_i > r_i, "Quantisation collapsed the axial strip")?;
    for axis in 0..3 {
        let far_i = 2205 * quant(far_radii[axis]);
        let far_h = 2205 * quant(far_radii[axis] * handle * 2f64.sqrt());
        check(
            far_i > 0 && far_h > 0 && 2 * far_h < far_i,
            "Quantisation collapsed a far profile",
        )?;
        for u in 0..6 {
            for v in 0..6 {
                let (rr, hh) = if v < 3 { (r_i, h_i) } else { (far_i, far_h) };
                let profile = [0, 0, 0, rr - 2 * hh, rr - hh, rr];
                let p = &mut out.strips[axis].control_points[u][v];
                p[axis] = ((r_i * (5 - v as i64) + stop_i * v as i64) / 5) as f64 / 4294967296.;
                p[(axis + 1) % 3] = profile[5 - u] as f64 / 4294967296.;
                p[(axis + 2) % 3] = profile[u] as f64 / 4294967296.;
            }
        }
    }
    for s in out.caps.iter().chain(&out.strips) {
        s.validate()?;
    }
    Ok(out)
}
