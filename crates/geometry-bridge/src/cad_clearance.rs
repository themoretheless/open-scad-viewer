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
fn segment_pair(a: P, b: P, c: P, d: P) -> (f64, [P;2]) {
    let u = sub(b, a);
    let v = sub(d, c);
    let w = sub(a, c);
    if !u.iter().chain(&v).chain(&w).all(|x| x.is_finite()) {
        return (f64::NAN,[a,c]);
    }
    let scale = u
        .iter()
        .chain(&v)
        .chain(&w)
        .map(|x| x.abs())
        .fold(0., f64::max);
    if scale == 0. {
        return (0.,[a,c]);
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
        (norm(std::array::from_fn(|k| p[k] - t * d[k])), t)
    };
    // If the constrained minimum lies on the boundary of the parameter square,
    // at least one endpoint attains it. This also handles parallel/zero edges.
    let e0=endpoint(w,v);
    let e1=endpoint(std::array::from_fn(|k|w[k]+u[k]),v);
    let e2=endpoint(w.map(|x|-x),u);
    let e3=endpoint(sub(v,w),u);
    let mut best=(e0.0,0.,e0.1);
    for candidate in [(e1.0,1.,e1.1),(e2.0,e2.1,0.),(e3.0,e3.1,1.)] {
        if candidate.0<best.0 {best=candidate;}
    }
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
            let distance=norm(std::array::from_fn(|k| w[k] + s * u[k] - t * v[k]));
            if distance<best.0 {best=(distance,s,t);}
        }
    }
    (best.0 * scale,[std::array::from_fn(|k|a[k]+best.1*(b[k]-a[k])),std::array::from_fn(|k|c[k]+best.2*(d[k]-c[k]))])
}
#[cfg(test)]
fn segments(a:P,b:P,c:P,d:P)->f64 {segment_pair(a,b,c,d).0}
fn consider(gap: &mut f64, witness:&mut Option<[P;2]>, distance: f64, points:[P;2]) -> Result<()> {
    if !distance.is_finite() || !points.iter().flatten().all(|x|x.is_finite()) {
        return Err(input("Clearance arithmetic exceeds finite numeric range."));
    }
    if distance<*gap {*gap=distance;*witness=Some(points);}
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
                .as_chunks::<3>().0.iter()
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
            // Any positive intersection volume means the solids overlap, regardless of units.
            let mut gap = if volume > 0. { 0. } else { f64::INFINITY };
            let mut witness=None;
            if gap != 0. {
                for a in &triangles[i] {
                    for b in &triangles[j] {
                        for &p in a {
                            let q=closest_triangle(p,b[0],b[1],b[2]);
                            consider(&mut gap,&mut witness,norm(sub(p,q)),[p,q])?;
                        }
                        for &p in b {
                            let q=closest_triangle(p,a[0],a[1],a[2]);
                            consider(&mut gap,&mut witness,norm(sub(p,q)),[q,p])?;
                        }
                        for x in 0..3 {
                            for y in 0..3 {
                                let (distance,points)=segment_pair(a[x],a[(x+1)%3],b[y],b[(y+1)%3]);
                                consider(&mut gap,&mut witness,distance,points)?;
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
            reports.push(value_codec::json!({"a":a,"b":b,"overlapMm3":volume,"gapMm":gap,"closestPoints":witness,"displayMeshOnly":true}));
        }
    }
    encode(reports)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn closest_segment_points_include_interior_and_endpoint_minima() {
        let (distance,points)=segment_pair([-1.,0.,0.],[1.,0.,0.],[0.,-1.,3.],[0.,1.,3.]);
        assert_eq!(distance,3.);assert_eq!(points,[[0.,0.,0.],[0.,0.,3.]]);
        let (distance,points)=segment_pair([0.,0.,0.],[1.,0.,0.],[2.,2.,0.],[3.,2.,0.]);
        assert_eq!(points,[[1.,0.,0.],[2.,2.,0.]]);
        assert_eq!(distance,norm(sub(points[0],points[1])));
    }
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
