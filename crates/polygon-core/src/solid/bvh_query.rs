//! Borrowed-buffer picking. Geometry remains owned by the caller; queries do
//! not copy vertex/index buffers. A transport must preserve that ownership.
use super::bvh::{LEAF_BIT, MeshBvh};
use std::collections::BTreeSet;

type V3 = [f64; 3];
#[inline(always)]
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
#[inline(always)]
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
#[inline(always)]
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
#[inline]
fn normal(a: V3) -> V3 {
    let length = a[0].hypot(a[1]).hypot(a[2]);
    if length > 0. && length.is_finite() {
        a.map(|v| v / length)
    } else {
        [0.; 3]
    }
}

#[derive(Clone, Debug)]
pub struct Hit {
    pub triangle: u32,
    pub vertex_indices: [u32; 3],
    pub t: f64,
    pub barycentric: V3,
    pub local_point: V3,
    pub world_point: V3,
    pub local_normal: V3,
    pub world_normal: V3,
    pub front_face: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryError {
    InvalidTree,
}

pub struct Query<'a> {
    pub origin: V3,
    pub direction: V3,
    pub min_t: f64,
    pub max_t: f64,
    pub local_from_world: Option<[f64; 16]>,
    pub excluded: &'a BTreeSet<u32>,
}

#[inline]
fn near(bounds: &[f32], node: usize, o: V3, d: V3, mut lo: f64, mut hi: f64) -> Option<f64> {
    for axis in 0..3 {
        let min = bounds[node * 6 + axis] as f64;
        let max = bounds[node * 6 + axis + 3] as f64;
        if min.is_nan() || max.is_nan() || min > max {
            return None;
        }
        if d[axis] == 0. {
            if o[axis] < min || o[axis] > max {
                return None;
            }
        } else {
            let mut a = (min - o[axis]) / d[axis];
            let mut b = (max - o[axis]) / d[axis];
            if a > b {
                std::mem::swap(&mut a, &mut b);
            }
            if a.is_nan() || b.is_nan() {
                return None;
            }
            lo = lo.max(a);
            hi = hi.min(b);
            if hi < lo {
                return None;
            }
        }
    }
    Some(lo)
}

fn triangle(
    vertices: &[f32],
    indices: &[u32],
    stride: usize,
    id: u32,
    o: V3,
    d: V3,
    lo: f64,
    hi: f64,
) -> Option<Hit> {
    let start = (id as usize).checked_mul(3)?;
    let ids: [u32; 3] = indices.get(start..start.checked_add(3)?)?.try_into().ok()?;
    let mut points = [[0.; 3]; 3];
    for i in 0..3 {
        let offset = (ids[i] as usize).checked_mul(stride)?;
        let p = vertices.get(offset..offset.checked_add(3)?)?;
        points[i] = [p[0] as f64, p[1] as f64, p[2] as f64];
    }
    let e1 = sub(points[1], points[0]);
    let e2 = sub(points[2], points[0]);
    let p = cross(d, e2);
    let det = dot(e1, p);
    let scale = (dot(e1, e1) * dot(e2, e2) * dot(d, d)).sqrt();
    if !det.is_finite() || det.abs() <= f64::EPSILON * 64. * scale {
        return None;
    }
    let inv = 1. / det;
    let tvec = sub(o, points[0]);
    let u = dot(tvec, p) * inv;
    if !(-1e-12..=1. + 1e-12).contains(&u) {
        return None;
    }
    let q = cross(tvec, e1);
    let v = dot(d, q) * inv;
    if v < -1e-12 || u + v > 1. + 1e-12 {
        return None;
    }
    let t = dot(e2, q) * inv;
    if !t.is_finite() || t < lo || t > hi {
        return None;
    }
    let mut barycentric = [(1. - u - v).clamp(0., 1.), u.clamp(0., 1.), v.clamp(0., 1.)];
    let sum = barycentric.iter().sum::<f64>();
    if sum > 0. {
        for b in &mut barycentric {
            *b /= sum;
        }
    }
    Some(Hit {
        triangle: id,
        vertex_indices: ids,
        t,
        barycentric,
        local_point: std::array::from_fn(|i| o[i] + d[i] * t),
        world_point: [0.; 3],
        local_normal: normal(cross(e1, e2)),
        world_normal: [0.; 3],
        front_face: det > 0.,
    })
}

/// Nearest double-sided hit, preserving unnormalised ray parameters and exact
/// distance ties. Repeated reachable nodes are refused rather than looping.
pub fn raycast(
    bvh: &MeshBvh,
    vertices: &[f32],
    indices: &[u32],
    stride: usize,
    query: &Query<'_>,
) -> Result<Option<Hit>, QueryError> {
    let count = bvh
        .node_count
        .min(bvh.bounds.len() / 6)
        .min(bvh.nodes.len() / 2);
    if count == 0 || bvh.triangles.is_empty() || stride < 3 {
        return Ok(None);
    }
    let finite = |v: V3| v.iter().all(|x| x.is_finite());
    if !finite(query.origin) || !finite(query.direction) || query.direction == [0.; 3] {
        return Ok(None);
    }
    let lo = if query.min_t.is_nan() {
        0.
    } else {
        query.min_t
    };
    let mut hi = if query.max_t.is_nan() {
        f64::INFINITY
    } else {
        query.max_t
    };
    if hi < lo {
        return Ok(None);
    }
    let (mut o, mut d) = (query.origin, query.direction);
    if let Some(m) = query.local_from_world {
        let w = m[12] * o[0] + m[13] * o[1] + m[14] * o[2] + m[15];
        let inv = if w.is_finite() && w.abs() > 1e-15 {
            1. / w
        } else {
            1.
        };
        o = std::array::from_fn(|i| {
            (m[i * 4] * query.origin[0]
                + m[i * 4 + 1] * query.origin[1]
                + m[i * 4 + 2] * query.origin[2]
                + m[i * 4 + 3])
                * inv
        });
        d = std::array::from_fn(|i| {
            m[i * 4] * query.direction[0]
                + m[i * 4 + 1] * query.direction[1]
                + m[i * 4 + 2] * query.direction[2]
        });
    }
    if !finite(o) || !finite(d) || d == [0.; 3] || near(&bvh.bounds, 0, o, d, lo, hi).is_none() {
        return Ok(None);
    }
    let mut visited = BTreeSet::new();
    let mut stack = vec![0];
    let mut hit: Option<Hit> = None;
    while let Some(node) = stack.pop() {
        if node >= count {
            continue;
        }
        if !visited.insert(node) {
            return Err(QueryError::InvalidTree);
        }
        if near(&bvh.bounds, node, o, d, lo, hi).is_none() {
            continue;
        }
        let first = bvh.nodes[node * 2] as usize;
        let metadata = bvh.nodes[node * 2 + 1];
        if metadata & LEAF_BIT != 0 {
            let end = first
                .saturating_add((metadata & !LEAF_BIT) as usize)
                .min(bvh.triangles.len());
            for slot in first..end {
                let id = bvh.triangles[slot];
                if query.excluded.contains(&id) {
                    continue;
                }
                if let Some(next) = triangle(vertices, indices, stride, id, o, d, lo, hi) {
                    if hit
                        .as_ref()
                        .is_none_or(|old| next.t < old.t || (next.t == old.t && id < old.triangle))
                    {
                        hi = next.t;
                        hit = Some(next);
                    }
                }
            }
        } else {
            let right = metadata as usize;
            let left_near = if first < count {
                near(&bvh.bounds, first, o, d, lo, hi)
            } else {
                None
            };
            let right_near = if right < count {
                near(&bvh.bounds, right, o, d, lo, hi)
            } else {
                None
            };
            match (left_near, right_near) {
                (Some(a), Some(b)) if a <= b => {
                    stack.push(right);
                    stack.push(first);
                }
                (Some(_), Some(_)) => {
                    stack.push(first);
                    stack.push(right);
                }
                (Some(_), None) => stack.push(first),
                (None, Some(_)) => stack.push(right),
                _ => {}
            }
        }
    }
    if let Some(hit) = &mut hit {
        hit.world_point = std::array::from_fn(|i| query.origin[i] + query.direction[i] * hit.t);
        hit.world_normal = if let Some(m) = query.local_from_world {
            normal(std::array::from_fn(|i| {
                m[i] * hit.local_normal[0]
                    + m[4 + i] * hit.local_normal[1]
                    + m[8 + i] * hit.local_normal[2]
            }))
        } else {
            hit.local_normal
        };
    }
    Ok(hit)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn agrees_with_frozen_host_queries() {
        use value_codec::{Value, from_str, from_value};
        let fixture: Value = from_str(include_str!(
            "../../tests/fixtures/bvh-query-parity-v1.json"
        ))
        .unwrap();
        let vertices: Vec<f32> = from_value(fixture["vertices"].clone()).unwrap();
        let indices: Vec<u32> = from_value(fixture["indices"].clone()).unwrap();
        let tree = super::super::bvh::build_mesh_bvh(&vertices, &indices, 3, 2);
        let cases = fixture["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 256);
        for case in cases {
            let excluded: BTreeSet<u32> = from_value::<Vec<u32>>(case["excluded"].clone())
                .unwrap()
                .into_iter()
                .collect();
            let query = Query {
                origin: from_value(case["origin"].clone()).unwrap(),
                direction: from_value(case["direction"].clone()).unwrap(),
                min_t: case["minT"].as_f64().unwrap(),
                max_t: case["maxT"].as_f64().unwrap(),
                local_from_world: if case["matrix"].is_null() {
                    None
                } else {
                    Some(from_value(case["matrix"].clone()).unwrap())
                },
                excluded: &excluded,
            };
            let hit = raycast(&tree, &vertices, &indices, 3, &query).unwrap();
            let expected = &case["expected"];
            if expected.is_null() {
                assert!(hit.is_none());
                continue;
            }
            let hit = hit.unwrap();
            assert_eq!(
                hit.triangle as u64,
                expected["triangleIndex"].as_u64().unwrap()
            );
            assert!((hit.t - expected["t"].as_f64().unwrap()).abs() < 1e-12);
            assert_eq!(hit.front_face, expected["frontFace"].as_bool().unwrap());
            for (key, actual) in [
                ("barycentric", hit.barycentric),
                ("localPoint", hit.local_point),
                ("worldPoint", hit.world_point),
                ("localNormal", hit.local_normal),
                ("worldNormal", hit.world_normal),
            ] {
                let reference: V3 = from_value(expected[key].clone()).unwrap();
                for axis in 0..3 {
                    assert!((actual[axis] - reference[axis]).abs() < 1e-12, "{key}");
                }
            }
        }
    }
    #[test]
    fn nearest_layers_ties_exclusions_and_unnormalised_rays() {
        let vertices = [
            -1., -1., 0., 1., -1., 0., 0., 1., 0., -1., -1., 1., 1., -1., 1., 0., 1., 1.,
        ];
        let indices = [0, 1, 2, 3, 4, 5, 3, 4, 5];
        let tree = super::super::bvh::build_mesh_bvh(&vertices, &indices, 3, 1);
        let mut excluded = BTreeSet::new();
        for expected in [1, 2, 0] {
            let query = Query {
                origin: [0., 0., 3.],
                direction: [0., 0., -2.],
                min_t: 0.,
                max_t: f64::INFINITY,
                local_from_world: None,
                excluded: &excluded,
            };
            let hit = raycast(&tree, &vertices, &indices, 3, &query)
                .unwrap()
                .unwrap();
            assert_eq!(hit.triangle, expected);
            assert_eq!(hit.t, if expected == 0 { 1.5 } else { 1. });
            assert_eq!(hit.barycentric, [0.25, 0.25, 0.5]);
            assert_eq!(hit.world_point, hit.local_point);
            assert_eq!(hit.world_normal, [0., 0., 1.]);
            assert!(hit.front_face);
            excluded.insert(expected);
        }
    }
    #[test]
    fn affine_ray_keeps_world_parameter_and_inverse_transpose_normal() {
        let vertices = [-1., -1., 0., 1., -1., 0., 0., 1., 0.];
        let indices = [0, 1, 2];
        let tree = super::super::bvh::build_mesh_bvh(&vertices, &indices, 3, 1);
        let excluded = BTreeSet::new();
        let query = Query {
            origin: [5., 0., 8.],
            direction: [0., 0., -2.],
            min_t: 0.,
            max_t: 10.,
            local_from_world: Some([
                0.5, 0., 0., -2.5, 0., 1., 0., 0., 0., 0., 0.25, -1., 0., 0., 0., 1.,
            ]),
            excluded: &excluded,
        };
        let hit = raycast(&tree, &vertices, &indices, 3, &query)
            .unwrap()
            .unwrap();
        assert_eq!(hit.t, 2.);
        assert_eq!(hit.world_point, [5., 0., 4.]);
        assert_eq!(hit.local_point, [0., 0., 0.]);
    }
    #[test]
    fn malformed_cycles_are_bounded_and_invalid_indices_do_not_panic() {
        let tree = MeshBvh {
            node_count: 1,
            bounds: vec![-1., -1., -1., 1., 1., 1.],
            nodes: vec![0, 0],
            triangles: vec![0],
        };
        let excluded = BTreeSet::new();
        let query = Query {
            origin: [0., 0., 2.],
            direction: [0., 0., -1.],
            min_t: 0.,
            max_t: 10.,
            local_from_world: None,
            excluded: &excluded,
        };
        assert!(matches!(
            raycast(&tree, &[], &[], 3, &query),
            Err(QueryError::InvalidTree)
        ));
        let leaf = MeshBvh {
            nodes: vec![0, LEAF_BIT | 1],
            ..tree
        };
        assert!(
            raycast(&leaf, &[], &[u32::MAX; 3], 3, &query)
                .unwrap()
                .is_none()
        );
    }
}
