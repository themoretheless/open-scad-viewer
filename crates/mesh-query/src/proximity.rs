//! Binary64 nearest points, signed distance and sampled mesh deviation.
use crate::{Result, error};
use math_core::{cross, dot, norm, sub};
use mesh_topology::MeshView;
pub type Point = [f64; 3];
pub fn closest_triangle(p: Point, a: Point, b: Point, c: Point) -> Point {
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = sub(p, a);
    if !ab.iter().chain(&ac).chain(&ap).all(|v| v.is_finite()) {
        return [f64::NAN; 3];
    }
    let scale = ab
        .iter()
        .chain(&ac)
        .chain(&ap)
        .map(|v| v.abs())
        .fold(0., f64::max);
    if scale == 0. {
        return a;
    }
    let q = closest_triangle_normalized(
        ap.map(|v| v / scale),
        [0.; 3],
        ab.map(|v| v / scale),
        ac.map(|v| v / scale),
    );
    std::array::from_fn(|k| a[k] + q[k] * scale)
}
fn closest_triangle_normalized(p: Point, a: Point, b: Point, c: Point) -> Point {
    let ab = sub(b, a);
    let ac = sub(c, a);
    let n = cross(ab, ac);
    let magnitude = n.iter().map(|v| v.abs()).fold(0., f64::max);
    if magnitude > 0. {
        let n = n.map(|v| v / magnitude);
        let nn = dot(n, n);
        let q = std::array::from_fn(|k| p[k] - n[k] * dot(sub(p, a), n) / nn);
        let aq = sub(q, a);
        let v = (dot(cross(aq, ac), n) / magnitude) / nn;
        let w = (dot(cross(ab, aq), n) / magnitude) / nn;
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
        let dist = norm(sub(p, q));
        if dist < distance {
            best = q;
            distance = dist;
        }
    }
    best
}
/// Nearest surface point for buffers admitted by MeshView::validate.
/// The caller validates once before repeated queries; malformed indices panic.
pub fn closest_point(mesh: &MeshView<'_>, p: Point) -> (Point, f64) {
    closest_point_validated(mesh, p)
}
fn closest_point_validated(mesh: &MeshView<'_>, p: Point) -> (Point, f64) {
    let mut best = p;
    let mut distance = f64::INFINITY;
    for t in mesh.indices.as_chunks::<3>().0 {
        let point = |i: usize| {
            [
                mesh.positions[3 * i],
                mesh.positions[3 * i + 1],
                mesh.positions[3 * i + 2],
            ]
        };
        let q = closest_triangle(p, point(t[0]), point(t[1]), point(t[2]));
        let d = norm(sub(p, q));
        if d < distance {
            best = q;
            distance = d;
        }
    }
    (best, distance)
}
/// Negative inside based on the oriented solid-angle sum. Closed, consistently
/// oriented, non-self-intersecting boundaries are required for solid semantics.
pub fn signed_distance(mesh: &MeshView<'_>, p: Point) -> f64 {
    let (_, d) = closest_point(mesh, p);
    if d == 0. {
        return 0.;
    }
    let mut angle = 0.;
    for t in mesh.indices.as_chunks::<3>().0 {
        let q = t.map_point(mesh, p);
        let [a, b, c] = q;
        let la = norm(a);
        let lb = norm(b);
        let lc = norm(c);
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
    fn map_point(&self, m: &MeshView<'_>, p: Point) -> [Point; 3];
}
impl TrianglePoints for [usize] {
    fn map_point(&self, m: &MeshView<'_>, p: Point) -> [Point; 3] {
        std::array::from_fn(|i| std::array::from_fn(|k| m.positions[self[i] * 3 + k] - p[k]))
    }
}
#[derive(Clone, Debug)]
pub struct Deviation {
    pub sampled_max_mm: f64,
    pub sampled_rms_mm: f64,
    pub sample_count: usize,
    pub error_bound_certified: bool,
}
/// Bidirectional samples at vertices and triangle centroids, not Hausdorff proof.
pub fn sample_deviation(a: &MeshView<'_>, b: &MeshView<'_>) -> Result<Deviation> {
    a.validate()?;
    b.validate()?;
    let count = |m: &MeshView<'_>| m.positions.len() / 3 + m.indices.len() / 3;
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
        let points = source
            .positions
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| [p[0], p[1], p[2]]);
        let centers = source.indices.as_chunks::<3>().0.iter().map(|t| {
            std::array::from_fn(|k| t.iter().map(|&i| source.positions[i * 3 + k] / 3.).sum())
        });
        for p in points.chain(centers) {
            let d = closest_point_validated(target, p).1;
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

/// Symmetric surface distance statistics between two triangle meshes evaluated via BVH.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HausdorffReport {
    pub min_mm: f64,
    pub max_mm: f64,
    pub mean_mm: f64,
    pub rms_mm: f64,
    pub sample_count: usize,
}

/// Computes symmetric approximate Hausdorff surface distance statistics (`min_mm`, `max_mm`, `mean_mm`, `rms_mm`)
/// between `mesh_a` and `mesh_b` using `DistanceBvh` acceleration over vertices, edge midpoints, and triangle centroids.
pub fn hausdorff_distance_approx(
    mesh_a: &MeshView<'_>,
    mesh_b: &MeshView<'_>,
) -> Result<HausdorffReport> {
    mesh_a.validate()?;
    mesh_b.validate()?;
    if mesh_a.indices.is_empty() || mesh_b.indices.is_empty() {
        return Err(error("Hausdorff distance requires non-empty meshes"));
    }
    let bvh_a = crate::distance_bvh::DistanceBvh::build(crate::distance_bvh::distance_triangles(*mesh_a)?);
    let bvh_b = crate::distance_bvh::DistanceBvh::build(crate::distance_bvh::distance_triangles(*mesh_b)?);

    let mut min_d = f64::INFINITY;
    let mut max_d = 0.0_f64;
    let mut sum_d = 0.0_f64;
    let mut sum_sq = 0.0_f64;
    let mut count = 0usize;

    for (source, target_bvh) in [(mesh_a, &bvh_b), (mesh_b, &bvh_a)] {
        let mut pts = Vec::with_capacity(source.positions.len() / 3 + (source.indices.len() / 3) * 4);
        for p in source.positions.as_chunks::<3>().0 {
            pts.push([p[0], p[1], p[2]]);
        }
        for t in source.indices.as_chunks::<3>().0 {
            let p0 = [
                source.positions[t[0] * 3],
                source.positions[t[0] * 3 + 1],
                source.positions[t[0] * 3 + 2],
            ];
            let p1 = [
                source.positions[t[1] * 3],
                source.positions[t[1] * 3 + 1],
                source.positions[t[1] * 3 + 2],
            ];
            let p2 = [
                source.positions[t[2] * 3],
                source.positions[t[2] * 3 + 1],
                source.positions[t[2] * 3 + 2],
            ];
            pts.push([(p0[0] + p1[0] + p2[0]) / 3.0, (p0[1] + p1[1] + p2[1]) / 3.0, (p0[2] + p1[2] + p2[2]) / 3.0]);
            pts.push([0.5 * (p0[0] + p1[0]), 0.5 * (p0[1] + p1[1]), 0.5 * (p0[2] + p1[2])]);
            pts.push([0.5 * (p1[0] + p2[0]), 0.5 * (p1[1] + p2[1]), 0.5 * (p1[2] + p2[2])]);
            pts.push([0.5 * (p2[0] + p0[0]), 0.5 * (p2[1] + p0[1]), 0.5 * (p2[2] + p0[2])]);
        }
        for p in pts {
            let d = target_bvh.distance(p);
            min_d = min_d.min(d);
            max_d = max_d.max(d);
            sum_d += d;
            sum_sq += d * d;
            count += 1;
        }
    }

    Ok(HausdorffReport {
        min_mm: if min_d.is_finite() { min_d } else { 0.0 },
        max_mm: max_d,
        mean_mm: sum_d / count.max(1) as f64,
        rms_mm: (sum_sq / count.max(1) as f64).sqrt(),
        sample_count: count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nearest_triangle_scale_covariance() {
        for scale in [1e-100, 1., 1e100] {
            let scaled = |p: Point| p.map(|v| v * scale);
            let a = [0.; 3];
            let b = scaled([1., 0., 0.]);
            let c = scaled([0., 1., 0.]);
            for [a, b, c] in [[a, b, c], [c, b, a], [b, c, a]] {
                for (p, expected) in [
                    ([0.2, 0.3, 4.], [0.2, 0.3, 0.]),
                    ([1., 1., 0.], [0.5, 0.5, 0.]),
                    ([-2., -3., 0.], [0.; 3]),
                ] {
                    let result = closest_triangle(scaled(p), a, b, c);
                    for (actual, expected) in result.into_iter().zip(expected) {
                        assert!(
                            (actual / scale - expected).abs() < 1e-12,
                            "scale {scale}: {result:?}"
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn nearest_regions() {
        let a = [0.; 3];
        let b = [1., 0., 0.];
        let c = [0., 1., 0.];
        assert_eq!(closest_triangle([0.2, 0.3, 4.], a, b, c), [0.2, 0.3, 0.]);
        assert_eq!(closest_triangle([1., 1., 0.], a, b, c), [0.5, 0.5, 0.]);
        assert_eq!(closest_triangle([-2., -3., 0.], a, b, c), a);
    }

    #[test]
    fn hausdorff_distance_approx_measures_surface_offset() {
        let pos_a = [0.0, 0.0, 0.0, 10.0, 0.0, 0.0, 0.0, 10.0, 0.0];
        let pos_b = [0.0, 0.0, 2.5, 10.0, 0.0, 2.5, 0.0, 10.0, 2.5];
        let idx = [0, 1, 2];
        let ma = MeshView::new(&pos_a, &idx);
        let mb = MeshView::new(&pos_b, &idx);
        let rep = hausdorff_distance_approx(&ma, &mb).unwrap();
        assert!((rep.min_mm - 2.5).abs() < 1e-12);
        assert!((rep.max_mm - 2.5).abs() < 1e-12);
        assert!((rep.mean_mm - 2.5).abs() < 1e-12);
        assert!((rep.rms_mm - 2.5).abs() < 1e-12);
    }
}

/// Validate and exactly weld a bounded source before reconstruction.
/// This checks triangle degeneracy, not closedness or full manifoldness.
pub fn valid_source(
    mesh: &MeshView<'_>,
    max_triangles: usize,
) -> Result<mesh_topology::weld::Welded> {
    let welded = mesh_topology::weld::exact(mesh)?;
    if welded.indices.is_empty() || welded.indices.len() / 3 > max_triangles {
        return Err(super::error(
            "Reconstruction source triangle budget exceeded or empty mesh",
        ));
    }
    if welded.view().inspect()?.degenerate_triangles != 0 {
        return Err(super::error("Reconstruction rejects degenerate triangles"));
    }
    Ok(welded)
}

#[cfg(test)]
mod source_tests {
    use super::*;
    #[test]
    fn source_admission_keeps_budget_degeneracy_and_native_error_namespaces() {
        let positions = [0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 0.];
        let indices = [3, 1, 2];
        let view = MeshView::new(&positions, &indices);
        let source = valid_source(&view, 1).unwrap();
        assert_eq!(source.indices, [0, 1, 2]);
        assert_eq!(source.positions, &positions[..9]);
        assert_eq!(
            valid_source(&view, 0).unwrap_err().code,
            "MESH_QUERY_INVALID_INPUT"
        );
        let empty = MeshView::new(&[], &[]);
        assert!(
            valid_source(&empty, 1)
                .unwrap_err()
                .message
                .contains("empty mesh")
        );
        let collapsed = MeshView::new(&positions, &[0, 3, 1]);
        assert_eq!(
            valid_source(&collapsed, 1).unwrap_err().message,
            "Reconstruction rejects degenerate triangles"
        );
        assert_eq!(
            valid_source(&MeshView::new(&positions, &[0, 1, 9]), 1)
                .unwrap_err()
                .code,
            "MESH_INVALID_INPUT"
        );
    }
}
