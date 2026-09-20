//! Connected selection patches. No handle registry, caching or buffer publication.
use crate::mesh_render::js_hypot3;
use crate::{Result, input};
use std::collections::HashMap;

#[derive(Clone, Copy)]
struct GroupEdge {
    triangle: u32,
    from: u32,
    to: u32,
    other: i32,
    count: u32,
}

fn point_key(x: f32, y: f32, z: f32) -> [u32; 3] {
    let key = |v: f32| if v == 0. { 0 } else { v.to_bits() };
    [key(x), key(y), key(z)]
}

fn find(parents: &mut [u32], mut i: u32) -> u32 {
    while parents[i as usize] != i {
        let parent = parents[i as usize];
        parents[i as usize] = parents[parent as usize];
        i = parents[i as usize];
    }
    i
}

/// Connected smooth patches, matching the legacy host `inferSurfaceIds`
/// semantics: exact-coordinate welds, sharp/boundary/degenerate/non-manifold
/// barriers, and first-seen compact ids.
pub fn surface_group_ids(
    stride: usize,
    vertices: &[f32],
    indices: &[u32],
    angle_degrees: f64,
) -> Result<Vec<u32>> {
    let count = indices.len() / 3;
    if !indices.len().is_multiple_of(3)
        || !(3..=64).contains(&stride)
        || !vertices.len().is_multiple_of(stride)
        || count > 100_000
        || !angle_degrees.is_finite()
        || !(0. ..=60.).contains(&angle_degrees)
    {
        return Err(input("Invalid surface grouping input/budget"));
    }

    let mut parents: Vec<u32> = (0..count as u32).collect();
    let mut normals = vec![[0_f64; 3]; count];
    let vertex_count = vertices.len() / stride;
    let mut canonical = vec![0_u32; vertex_count];
    let mut points = HashMap::<[u32; 3], u32>::new();
    let sparse = vertex_count > indices.len();
    if sparse {
        for &index in indices {
            let slot = canonical
                .get_mut(index as usize)
                .ok_or_else(|| input("Invalid triangle index"))?;
            *slot = 1;
        }
    }
    for i in 0..vertex_count {
        let offset = i * stride;
        let x = vertices[offset];
        let y = vertices[offset + 1];
        let z = vertices[offset + 2];
        if !x.is_finite() || !y.is_finite() || !z.is_finite() {
            return Err(input("Nonfinite mesh position"));
        }
        if sparse && canonical[i] == 0 {
            continue;
        }
        let key = point_key(x, y, z);
        let next = points.len() as u32;
        canonical[i] = *points.entry(key).or_insert(next);
    }

    let mut edges = HashMap::<(u32, u32), GroupEdge>::new();
    for t in 0..count {
        let v = [indices[t * 3], indices[t * 3 + 1], indices[t * 3 + 2]];
        if v.iter().any(|&i| i as usize >= canonical.len()) {
            return Err(input("Invalid triangle index"));
        }
        let p = |i: u32, k: usize| vertices[i as usize * stride + k] as f64;
        let a = [
            p(v[1], 0) - p(v[0], 0),
            p(v[1], 1) - p(v[0], 1),
            p(v[1], 2) - p(v[0], 2),
        ];
        let b = [
            p(v[2], 0) - p(v[0], 0),
            p(v[2], 1) - p(v[0], 1),
            p(v[2], 2) - p(v[0], 2),
        ];
        let n = [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ];
        let length = js_hypot3(n);
        if length != 0. {
            normals[t] = n.map(|x| x / length);
        }
        for i in 0..3 {
            let from = canonical[v[i] as usize];
            let to = canonical[v[(i + 1) % 3] as usize];
            let key = (from.min(to), from.max(to));
            if let Some(edge) = edges.get_mut(&key) {
                edge.count += 1;
                if edge.from == to && edge.to == from {
                    edge.other = t as i32;
                }
            } else {
                edges.insert(
                    key,
                    GroupEdge {
                        triangle: t as u32,
                        from,
                        to,
                        other: -1,
                        count: 1,
                    },
                );
            }
        }
    }

    let threshold = (angle_degrees * std::f64::consts::PI / 180.).cos();
    // Every union keeps the minimum triangle root. Edge traversal order cannot
    // change components or the first-triangle numbering assigned below.
    for edge in edges.values() {
        if edge.count == 2 && edge.other >= 0 {
            let a = edge.triangle;
            let b = edge.other as u32;
            let dot = normals[a as usize][0] * normals[b as usize][0]
                + normals[a as usize][1] * normals[b as usize][1]
                + normals[a as usize][2] * normals[b as usize][2];
            if dot >= threshold - 1e-12 {
                let x = find(&mut parents, a);
                let y = find(&mut parents, b);
                parents[x.max(y) as usize] = x.min(y);
            }
        }
    }

    let mut ids = HashMap::<u32, u32>::new();
    let mut result = Vec::with_capacity(count);
    for i in 0..count as u32 {
        let root = find(&mut parents, i);
        let next = ids.len() as u32;
        result.push(*ids.entry(root).or_insert(next));
    }
    Ok(result)
}

#[cfg(test)]
mod surface_group_tests {
    use super::surface_group_ids;

    fn stride6(points: &[[f32; 3]]) -> Vec<f32> {
        points
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 0., 0., 1.])
            .collect()
    }

    #[test]
    fn groups_connected_smooth_strip() {
        let vertices = stride6(&[[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]]);
        let ids = surface_group_ids(6, &vertices, &[0, 1, 2, 0, 2, 3], 30.).unwrap();
        assert_eq!(ids, vec![0, 0]);
    }

    #[test]
    fn exact_welds_preserve_winding_and_signed_zero() {
        let vertices = stride6(&[
            [0., 0., 0.],
            [1., 0., 0.],
            [0., 1., 0.],
            [-0., 1., 0.],
            [1., -0., 0.],
            [1., 1., 0.],
        ]);
        assert_eq!(
            surface_group_ids(6, &vertices, &[0, 1, 2, 3, 4, 5], 30.).unwrap(),
            [0, 0]
        );
        assert_eq!(
            surface_group_ids(6, &vertices, &[0, 1, 2, 4, 3, 5], 30.).unwrap(),
            [0, 1]
        );
    }

    #[test]
    fn degenerate_triangles_are_barriers_and_empty_mesh_is_supported() {
        let vertices = stride6(&[[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]]);
        assert_eq!(
            surface_group_ids(6, &vertices, &[0, 1, 2, 1, 0, 0], 30.).unwrap(),
            [0, 1]
        );
        assert!(surface_group_ids(6, &[], &[], 30.).unwrap().is_empty());
    }

    #[test]
    fn validates_unused_positions_and_sparse_indices() {
        let mut vertices = stride6(&[[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [9., 9., 9.]]);
        assert_eq!(
            surface_group_ids(6, &vertices, &[0, 1, 2], 30.).unwrap(),
            [0]
        );
        assert!(surface_group_ids(6, &vertices, &[0, 1, 4], 30.).is_err());
        vertices[18] = f32::INFINITY;
        assert!(surface_group_ids(6, &vertices, &[0, 1, 2], 30.).is_err());
        assert!(surface_group_ids(6, &vertices, &[], 30.).is_err());
    }

    #[test]
    fn refuses_invalid_layout_angles_and_triangle_budget() {
        for stride in [0, 2, 65] {
            assert!(surface_group_ids(stride, &[], &[], 30.).is_err());
        }
        assert!(surface_group_ids(6, &[0.; 5], &[], 30.).is_err());
        assert!(surface_group_ids(6, &[], &[0], 30.).is_err());
        for angle in [-1., 61., f64::NAN, f64::INFINITY] {
            assert!(surface_group_ids(6, &[], &[], angle).is_err());
        }
        assert!(surface_group_ids(6, &[0.; 18], &vec![0; 300_003], 30.).is_err());
    }

    #[test]
    fn stride_three_and_stride_six_have_identical_groups() {
        let points = [[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]];
        let indices = [0, 1, 2, 0, 2, 3];
        assert_eq!(
            surface_group_ids(3, points.as_flattened(), &indices, 30.).unwrap(),
            surface_group_ids(6, &stride6(&points), &indices, 30.).unwrap(),
        );
    }

    #[test]
    fn keeps_disconnected_and_non_manifold_triangles_separate() {
        let vertices = stride6(&[
            [0., 0., 0.],
            [1., 0., 0.],
            [0., 1., 0.],
            [0., -1., 0.],
            [1., 1., 0.],
        ]);
        let ids = surface_group_ids(6, &vertices, &[0, 1, 2, 1, 0, 3, 0, 1, 4], 30.).unwrap();
        assert_eq!(ids, vec![0, 1, 2]);
    }
}
