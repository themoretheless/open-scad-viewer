//! Bounded display-mesh clearance; not an analytic B-rep distance certificate.
use super::{Result, Value, encode, field, input};
use polygon_core::{
    Mesh,
    solid::{
        boolean::{Operation, Options, boolean},
        proximity::closest_triangle,
    },
};
type P = [f64; 3];
fn sub(a: P, b: P) -> P {
    std::array::from_fn(|k| a[k] - b[k])
}
fn dot(a: P, b: P) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn norm(a: P) -> f64 {
    a[0].hypot(a[1]).hypot(a[2])
}
fn cross(a: P, b: P) -> P {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn segments(a: P, b: P, c: P, d: P) -> f64 {
    let u = sub(b, a);
    let v = sub(d, c);
    let w = sub(a, c);
    if !u.iter().chain(&v).chain(&w).all(|x| x.is_finite()) {
        return f64::NAN;
    }
    let scale = u
        .iter()
        .chain(&v)
        .chain(&w)
        .map(|x| x.abs())
        .fold(0., f64::max);
    if scale == 0. {
        return 0.;
    }
    let u = u.map(|x| x / scale);
    let v = v.map(|x| x / scale);
    let w = w.map(|x| x / scale);
    let endpoint = |p: P, d: P| {
        let dd = dot(d, d);
        let t = if dd > 0. {
            (dot(p, d) / dd).clamp(0., 1.)
        } else {
            0.
        };
        norm(std::array::from_fn(|k| p[k] - t * d[k]))
    };
    // If the constrained minimum lies on the boundary of the parameter square,
    // at least one endpoint attains it. This also handles parallel/zero edges.
    let mut best = endpoint(w, v)
        .min(endpoint(std::array::from_fn(|k| w[k] + u[k]), v))
        .min(endpoint(w.map(|x| -x), u))
        .min(endpoint(sub(v, w), u));
    let n = cross(u, v);
    let magnitude = n.iter().map(|x| x.abs()).fold(0., f64::max);
    if magnitude > 0. {
        // Cross-product formulation avoids subtracting nearly equal squared dots.
        // Normalize the normal before squaring to preserve tiny crossing angles.
        let n = n.map(|x| x / magnitude);
        let denominator = dot(n, n);
        let s = (dot(cross(v, w), n) / magnitude) / denominator;
        let t = (dot(cross(u, w), n) / magnitude) / denominator;
        if (0. ..=1.).contains(&s) && (0. ..=1.).contains(&t) {
            best = best.min(norm(std::array::from_fn(|k| w[k] + s * u[k] - t * v[k])));
        }
    }
    best * scale
}
fn consider(gap: &mut f64, distance: f64) -> Result<()> {
    if !distance.is_finite() {
        return Err(input("Clearance arithmetic exceeds finite numeric range."));
    }
    *gap = gap.min(distance);
    Ok(())
}
pub fn inspect(v: Value) -> Result<Value> {
    let bodies: Vec<Value> = field(&v, "bodies")?;
    if bodies.len() < 2 {
        return Err(input("Select at least two bodies."));
    }
    let meshes = bodies
        .iter()
        .map(|b| {
            let m: Mesh = field(b, "mesh")?;
            m.validate()?;
            Ok(m)
        })
        .collect::<Result<Vec<_>>>()?;
    let mut budget = 0usize;
    for i in 0..meshes.len() {
        for j in i + 1..meshes.len() {
            budget = budget.saturating_add(
                (meshes[i].indices.len() / 3).saturating_mul(meshes[j].indices.len() / 3),
            );
        }
    }
    if budget > 2_000_000 {
        return Err(input(
            "Clearance check exceeds two million triangle pairs; select fewer/simpler bodies.",
        ));
    }
    let triangles = meshes
        .iter()
        .map(|m| {
            m.indices
                .chunks_exact(3)
                .map(|t| {
                    std::array::from_fn::<_, 3, _>(|i| {
                        std::array::from_fn::<_, 3, _>(|k| m.positions[t[i] * 3 + k])
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut reports = Vec::new();
    for i in 0..meshes.len() {
        for j in i + 1..meshes.len() {
            let volume = boolean(
                &meshes[i],
                &meshes[j],
                Operation::Intersection,
                &Options::default(),
            )?
            .report
            .signed_volume_mm3
            .abs();
            let mut gap = if volume > 1e-9 { 0. } else { f64::INFINITY };
            if gap != 0. {
                for a in &triangles[i] {
                    for b in &triangles[j] {
                        for &p in a {
                            consider(
                                &mut gap,
                                norm(sub(p, closest_triangle(p, b[0], b[1], b[2]))),
                            )?;
                        }
                        for &p in b {
                            consider(
                                &mut gap,
                                norm(sub(p, closest_triangle(p, a[0], a[1], a[2]))),
                            )?;
                        }
                        for x in 0..3 {
                            for y in 0..3 {
                                consider(
                                    &mut gap,
                                    segments(a[x], a[(x + 1) % 3], b[y], b[(y + 1) % 3]),
                                )?;
                            }
                        }
                    }
                }
            }
            if !gap.is_finite() || !volume.is_finite() {
                return Err(input(
                    "Clearance calculation requires finite nonempty mesh geometry.",
                ));
            }
            let a: String = field(&bodies[i], "name")?;
            let b: String = field(&bodies[j], "name")?;
            reports.push(value_codec::json!({"a":a,"b":b,"overlapMm3":volume,"gapMm":gap}));
        }
    }
    encode(reports)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn segment_distance_is_scale_covariant() {
        for scale in [1e-100, 1e-6, 1., 1e100] {
            let p = |v: P| v.map(|x| x * scale);
            let a = p([-1., 0., 0.]);
            let b = p([1., 0., 0.]);
            let c = p([0., -1., 3.]);
            let d = p([0., 1., 3.]);
            for [a, b, c, d] in [[a, b, c, d], [b, a, c, d], [a, b, d, c], [c, d, a, b]] {
                let distance = segments(a, b, c, d);
                assert!(
                    (distance / scale - 3.).abs() < 1e-12,
                    "scale {scale}: distance {distance}"
                );
            }
        }
    }
    #[test]
    fn nearly_parallel_crossing_has_zero_distance() {
        for epsilon in [1e-12, 1e-100, 1e-200] {
            assert!(
                segments(
                    [-1., 0., 0.],
                    [1., 0., 0.],
                    [-1., -epsilon, 0.],
                    [1., epsilon, 0.]
                ) < epsilon * 1e-8
            );
        }
    }
    #[test]
    fn segment_distance_covers_interior_parallel_and_collapsed_cases() {
        assert_eq!(
            segments([-1., 0., 0.], [1., 0., 0.], [0., -1., 3.], [0., 1., 3.]),
            3.
        );
        assert_eq!(
            segments([0., 0., 0.], [1., 0., 0.], [0., 2., 0.], [1., 2., 0.]),
            2.
        );
        assert_eq!(segments([0.; 3], [0.; 3], [0., -1., 3.], [0., 1., 3.]), 3.);
    }
}
