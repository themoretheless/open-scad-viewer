//! Mesh reconstruction helpers. Exact-coordinate welding never guesses a tolerance.
use crate::{Mesh, Result, cross, error, norm, sub};
use std::collections::BTreeMap;
pub type Point = [f64; 3];
fn dot(a: Point, b: Point) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
pub fn weld_exact(mesh: &Mesh) -> Result<Mesh> {
    mesh.validate()?;
    let mut points = Vec::new();
    let mut map = BTreeMap::new();
    let mut ids = Vec::with_capacity(mesh.indices.len());
    for &i in &mesh.indices {
        let p = &mesh.positions[3 * i..3 * i + 3];
        let key = std::array::from_fn::<_, 3, _>(|i| if p[i] == 0. { 0 } else { p[i].to_bits() });
        let next = points.len() / 3;
        let id = *map.entry(key).or_insert_with(|| {
            points.extend_from_slice(p);
            next
        });
        ids.push(id);
    }
    let result = Mesh {
        positions: points,
        indices: ids,
        uv: None,
    };
    result.validate()?;
    Ok(result)
}
pub fn valid_source(mesh: &Mesh, max_triangles: usize) -> Result<Mesh> {
    let m = weld_exact(mesh)?;
    if m.indices.is_empty() || m.indices.len() / 3 > max_triangles {
        return Err(error(
            "Reconstruction source triangle budget exceeded or empty mesh",
        ));
    }
    if m.inspect()?.degenerate_triangles != 0 {
        return Err(error("Reconstruction rejects degenerate triangles"));
    }
    Ok(m)
}
pub fn closest_triangle(p: Point, a: Point, b: Point, c: Point) -> Point {
    let ab = sub(b, a);
    let ac = sub(c, a);
    let n = cross(ab, ac);
    let nn = dot(n, n);
    if nn > 0. {
        let q = std::array::from_fn(|k| p[k] - n[k] * dot(sub(p, a), n) / nn);
        let aq = sub(q, a);
        let v = dot(cross(aq, ac), n) / nn;
        let w = dot(cross(ab, aq), n) / nn;
        if v >= 0. && w >= 0. && v + w <= 1. {
            return q;
        }
    }
    let mut best = a;
    let mut distance = f64::INFINITY;
    for (a, b) in [(a, b), (b, c), (c, a)] {
        let d = sub(b, a);
        let t = if dot(d, d) > 0. {
            (dot(sub(p, a), d) / dot(d, d)).clamp(0., 1.)
        } else {
            0.
        };
        let q = std::array::from_fn(|i| a[i] + t * d[i]);
        let dist = norm(&sub(p, q));
        if dist < distance {
            best = q;
            distance = dist;
        }
    }
    best
}
/// Prepared by the caller using valid_source; avoids repeated topology scans.
pub fn closest_point(mesh: &Mesh, p: Point) -> (Point, f64) {
    let mut best = p;
    let mut distance = f64::INFINITY;
    for t in mesh.indices.chunks_exact(3) {
        let point = |i: usize| {
            [
                mesh.positions[3 * i],
                mesh.positions[3 * i + 1],
                mesh.positions[3 * i + 2],
            ]
        };
        let q = closest_triangle(p, point(t[0]), point(t[1]), point(t[2]));
        let d = norm(&sub(p, q));
        if d < distance {
            best = q;
            distance = d;
        }
    }
    (best, distance)
}
/// Negative inside based on the oriented solid-angle sum. Closed, consistently
/// oriented, non-self-intersecting boundaries are required for solid semantics.
pub fn signed_distance(mesh: &Mesh, p: Point) -> f64 {
    let (_, d) = closest_point(mesh, p);
    if d == 0. {
        return 0.;
    }
    let mut angle = 0.;
    for t in mesh.indices.chunks_exact(3) {
        let q = t.map_point(mesh, p);
        let [a, b, c] = q;
        let la = norm(&a);
        let lb = norm(&b);
        let lc = norm(&c);
        angle += 2.
            * dot(a, cross(b, c))
                .atan2(la * lb * lc + dot(a, b) * lc + dot(b, c) * la + dot(c, a) * lb);
    }
    if angle.abs() > 2. * std::f64::consts::PI {
        -d
    } else {
        d
    }
}
trait TrianglePoints {
    fn map_point(&self, m: &Mesh, p: Point) -> [Point; 3];
}
impl TrianglePoints for [usize] {
    fn map_point(&self, m: &Mesh, p: Point) -> [Point; 3] {
        std::array::from_fn(|i| std::array::from_fn(|k| m.positions[self[i] * 3 + k] - p[k]))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nearest_regions() {
        let a = [0.; 3];
        let b = [1., 0., 0.];
        let c = [0., 1., 0.];
        assert_eq!(closest_triangle([0.2, 0.3, 4.], a, b, c), [0.2, 0.3, 0.]);
        assert_eq!(closest_triangle([1., 1., 0.], a, b, c), [0.5, 0.5, 0.]);
        assert_eq!(closest_triangle([-2., -3., 0.], a, b, c), a);
    }
}

#[derive(Clone, Debug)]
pub struct Deviation {
    pub sampled_max_mm: f64,
    pub sampled_rms_mm: f64,
    pub sample_count: usize,
    pub error_bound_certified: bool,
}
impl value_codec::Serialize for Deviation {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "sampledMaxMm".into(),
            value_codec::Serialize::to_value(&self.sampled_max_mm),
        );
        object.insert(
            "sampledRmsMm".into(),
            value_codec::Serialize::to_value(&self.sampled_rms_mm),
        );
        object.insert(
            "sampleCount".into(),
            value_codec::Serialize::to_value(&self.sample_count),
        );
        object.insert(
            "errorBoundCertified".into(),
            value_codec::Serialize::to_value(&self.error_bound_certified),
        );
        value_codec::Value::Object(object)
    }
}
/// Bidirectional samples at vertices and triangle centroids, not Hausdorff proof.
pub fn sample_deviation(a: &Mesh, b: &Mesh) -> Result<Deviation> {
    a.validate()?;
    b.validate()?;
    let count = |m: &Mesh| m.positions.len() / 3 + m.indices.len() / 3;
    let work =
        count(a).saturating_mul(b.indices.len() / 3) + count(b).saturating_mul(a.indices.len() / 3);
    if a.indices.is_empty() || b.indices.is_empty() || work > 8_000_000 {
        return Err(error(
            "Deviation sampling exceeds 8000000 triangle tests or empty input",
        ));
    }
    let mut max: f64 = 0.;
    let mut sum = 0.;
    let mut samples = 0;
    for (source, target) in [(a, b), (b, a)] {
        let points = source.positions.chunks_exact(3).map(|p| [p[0], p[1], p[2]]);
        let centers = source.indices.chunks_exact(3).map(|t| {
            std::array::from_fn(|k| t.iter().map(|&i| source.positions[i * 3 + k] / 3.).sum())
        });
        for p in points.chain(centers) {
            let d = closest_point(target, p).1;
            max = max.max(d);
            sum += d * d;
            samples += 1;
        }
    }
    Ok(Deviation {
        sampled_max_mm: max,
        sampled_rms_mm: (sum / samples as f64).sqrt(),
        sample_count: samples,
        error_bound_certified: false,
    })
}
