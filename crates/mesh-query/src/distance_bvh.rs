//! Binary64 BVH for repeated surface distances and parity-based inside tests.
//! Separate from the float32 picking BVH to retain sampled-field precision.
use crate::proximity::closest_triangle;
use crate::{MeshView, Result};
use math_core::{cross, dot, sub};
pub type P = [f64; 3];
/// Build owned triangle records once after validating the source mesh.
pub fn distance_triangles(mesh: MeshView<'_>) -> Result<Vec<DistanceTriangle>> {
    mesh.validate()?;
    Ok(mesh
        .indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|ids| {
            let p =
                std::array::from_fn(|i| std::array::from_fn(|k| mesh.positions[ids[i] * 3 + k]));
            DistanceTriangle {
                min: std::array::from_fn(|k| p.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min)),
                max: std::array::from_fn(|k| {
                    p.iter().map(|p| p[k]).fold(f64::NEG_INFINITY, f64::max)
                }),
                p,
            }
        })
        .collect())
}
#[derive(Clone)]
pub struct DistanceTriangle {
    pub p: [P; 3],
    pub min: P,
    pub max: P,
}
pub struct DistanceBvh {
    pub min: P,
    pub max: P,
    pub triangles: Vec<DistanceTriangle>,
    pub children: Option<Box<[DistanceBvh; 2]>>,
}
impl DistanceBvh {
    pub fn build(mut triangles: Vec<DistanceTriangle>) -> Self {
        let min = std::array::from_fn(|k| {
            triangles
                .iter()
                .map(|t| t.min[k])
                .fold(f64::INFINITY, f64::min)
        });
        let max = std::array::from_fn(|k| {
            triangles
                .iter()
                .map(|t| t.max[k])
                .fold(f64::NEG_INFINITY, f64::max)
        });
        if triangles.len() <= 8 {
            return Self {
                min,
                max,
                triangles,
                children: None,
            };
        }
        let axis = (0..3)
            .max_by(|&a, &b| (max[a] - min[a]).total_cmp(&(max[b] - min[b])))
            .unwrap();
        triangles
            .sort_by(|a, b| (a.min[axis] + a.max[axis]).total_cmp(&(b.min[axis] + b.max[axis])));
        let right = triangles.split_off(triangles.len() / 2);
        Self {
            min,
            max,
            triangles: vec![],
            children: Some(Box::new([Self::build(triangles), Self::build(right)])),
        }
    }
    fn bound(&self, p: P) -> f64 {
        (0..3)
            .map(|k| (self.min[k] - p[k]).max(p[k] - self.max[k]).max(0.).powi(2))
            .sum()
    }
    fn nearest(&self, p: P, best: &mut f64) {
        if self.bound(p) > *best {
            return;
        }
        if let Some(children) = &self.children {
            let first = usize::from(children[1].bound(p) < children[0].bound(p));
            children[first].nearest(p, best);
            children[1 - first].nearest(p, best)
        } else {
            for t in &self.triangles {
                let q = closest_triangle(p, t.p[0], t.p[1], t.p[2]);
                *best = best.min(dot(sub(p, q), sub(p, q)))
            }
        }
    }
    pub fn distance(&self, p: P) -> f64 {
        let mut best = f64::INFINITY;
        self.nearest(p, &mut best);
        best.sqrt()
    }
    fn hits(&self, p: P, d: P, hits: &mut Vec<f64>) {
        let mut low: f64 = 0.;
        let mut high = f64::INFINITY;
        for k in 0..3 {
            let a = (self.min[k] - p[k]) / d[k];
            let b = (self.max[k] - p[k]) / d[k];
            low = low.max(a.min(b));
            high = high.min(a.max(b));
        }
        if high < low {
            return;
        }
        if let Some(children) = &self.children {
            children[0].hits(p, d, hits);
            children[1].hits(p, d, hits)
        } else {
            for t in &self.triangles {
                let e1 = sub(t.p[1], t.p[0]);
                let e2 = sub(t.p[2], t.p[0]);
                let h = cross(d, e2);
                let det = dot(e1, h);
                if det.abs() < 1e-13 {
                    continue;
                }
                let s = sub(p, t.p[0]);
                let u = dot(s, h) / det;
                if !(-1e-10..=1. + 1e-10).contains(&u) {
                    continue;
                }
                let q = cross(s, e1);
                let v = dot(d, q) / det;
                if v < -1e-10 || u + v > 1. + 1e-10 {
                    continue;
                }
                let along = dot(e2, q) / det;
                if along > 1e-10 {
                    hits.push(along)
                }
            }
        }
    }
    pub fn signed_distance(&self, p: P) -> f64 {
        let distance = self.distance(p);
        if distance < 1e-12 {
            return 0.;
        }
        let mut hits = vec![];
        self.hits(p, [1., 0.3713906763541037, 0.127831], &mut hits);
        hits.sort_by(f64::total_cmp);
        hits.dedup_by(|a, b| (*a - *b).abs() < 1e-8 * (1. + a.abs().max(b.abs())));
        if hits.len() % 2 == 1 {
            -distance
        } else {
            distance
        }
    }
}

/// Preorder flattening of the recursive BVH: min/max (6 f32), left/right
/// (i32, -1 for leaves), triangle window (start, count as u32 bits). Shared
/// by the wgpu and CUDA lattice kernels.
pub fn flatten_distance_bvh(root: &DistanceBvh) -> (Vec<f32>, Vec<f32>) {
    let mut nodes: Vec<f32> = Vec::new();
    let mut triangles: Vec<f32> = Vec::new();
    fn emit(node: &DistanceBvh, nodes: &mut Vec<f32>, triangles: &mut Vec<f32>) -> u32 {
        let index = (nodes.len() / 10) as u32;
        nodes.resize(nodes.len() + 10, 0.);
        for k in 0..3 {
            nodes[index as usize * 10 + k] = node.min[k] as f32;
            nodes[index as usize * 10 + 3 + k] = node.max[k] as f32;
        }
        if let Some(children) = &node.children {
            let left = emit(&children[0], nodes, triangles);
            let right = emit(&children[1], nodes, triangles);
            nodes[index as usize * 10 + 6] = f32::from_bits(left);
            nodes[index as usize * 10 + 7] = f32::from_bits(right);
        } else {
            nodes[index as usize * 10 + 6] = f32::from_bits(u32::MAX);
            nodes[index as usize * 10 + 7] = f32::from_bits(u32::MAX);
            let start = (triangles.len() / 9) as u32;
            for t in &node.triangles {
                for point in t.p {
                    for coordinate in point {
                        triangles.push(coordinate as f32);
                    }
                }
            }
            nodes[index as usize * 10 + 8] = f32::from_bits(start);
            nodes[index as usize * 10 + 9] = f32::from_bits(triangles.len() as u32 / 9 - start);
        }
        index
    }
    emit(root, &mut nodes, &mut triangles);
    (nodes, triangles)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn distances_match_direct_queries_across_internal_nodes() {
        let mut positions = Vec::new();
        let mut indices = Vec::new();
        for i in 0..20 {
            let start = positions.len() / 3;
            positions.extend([i as f64, 0., 0., i as f64, 1., 0., i as f64, 0., 1.]);
            indices.extend([start, start + 1, start + 2]);
        }
        let mesh = MeshView::new(&positions, &indices);
        let bvh = DistanceBvh::build(distance_triangles(mesh).unwrap());
        for p in [[-2., 0.2, 0.2], [7.2, 0.4, 0.4], [30., 2., 2.]] {
            let expected = crate::proximity::closest_point(&mesh, p).1;
            assert!((bvh.distance(p) - expected).abs() < 1e-12);
        }
        let (nodes, triangles) = flatten_distance_bvh(&bvh);
        assert_eq!(nodes.len() % 10, 0);
        assert_eq!(triangles.len(), 20 * 9);
        for record in nodes.as_chunks::<10>().0 {
            let left = record[6].to_bits();
            if left == u32::MAX {
                let start = record[8].to_bits() as usize;
                let count = record[9].to_bits() as usize;
                assert!(start + count <= 20);
                assert!(count <= 8);
            } else {
                assert!((left as usize) < nodes.len() / 10);
                assert!((record[7].to_bits() as usize) < nodes.len() / 10);
            }
        }
    }
    #[test]
    fn empty_and_invalid_meshes_have_defined_admission() {
        let empty = DistanceBvh::build(distance_triangles(MeshView::new(&[], &[])).unwrap());
        assert_eq!(empty.distance([0.; 3]), f64::INFINITY);
        assert!(distance_triangles(MeshView::new(&[0.; 3], &[0, 1, 2])).is_err());
    }
}
