//! Detached display preparation. No handle registry or persistent scratch state.
use crate::CadMeshBuffer;

/// Stride-6 display vertices, property-vertex merges and authored planar face ids.
pub struct RenderMesh {
    pub(crate) vertices: Vec<f32>,
    pub(crate) indices: Vec<u32>,
    pub(crate) merge_from: Vec<u32>,
    pub(crate) merge_to: Vec<u32>,
    pub(crate) face_ids: Vec<u32>,
}

/// `Math.hypot` as V8 evaluates it (scaled Kahan summation).
pub(crate) fn js_hypot3(v: [f64; 3]) -> f64 {
    let mut max = 0_f64;
    let mut has_nan = false;
    let mut magnitudes = [0_f64; 3];
    for (slot, value) in magnitudes.iter_mut().zip(v) {
        if value.is_nan() {
            has_nan = true;
        } else {
            *slot = value.abs();
            if *slot > max {
                max = *slot;
            }
        }
    }
    if max == f64::INFINITY {
        return f64::INFINITY;
    }
    if has_nan {
        return f64::NAN;
    }
    if max == 0. {
        return 0.;
    }
    let (mut sum, mut compensation) = (0_f64, 0_f64);
    for magnitude in magnitudes {
        let n = magnitude / max;
        let summand = n * n - compensation;
        let preliminary = sum + summand;
        compensation = (preliminary - sum) - summand;
        sum = preliminary;
    }
    sum.sqrt() * max
}

/// `Math.round`: ties toward positive infinity.
fn js_round(x: f64) -> f64 {
    if !x.is_finite() || x.fract() == 0. {
        return x;
    }
    let floor = x.floor();
    if x - floor >= 0.5 {
        floor + 1.
    } else {
        floor
    }
}

fn js_normalized(v: [f64; 3], length: f64) -> [f64; 3] {
    if length == 0. || length.is_nan() {
        [0.; 3]
    } else {
        v.map(|x| x / length)
    }
}

fn point(positions: &[f64], id: u32) -> [f64; 3] {
    let i = id as usize * 3;
    [positions[i], positions[i + 1], positions[i + 2]]
}

fn face_normal(positions: &[f64], tri: &[u32; 3]) -> [f64; 3] {
    let p0 = point(positions, tri[0]);
    let p1 = point(positions, tri[1]);
    let p2 = point(positions, tri[2]);
    let a = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
    let b = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];
    let n = [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ];
    js_normalized(n, js_hypot3(n))
}

struct PropertyVertex {
    normal: [i64; 3],
    previous: u32,
}
const NONE: u32 = u32::MAX;

struct FaceAdjacency {
    offsets: Vec<usize>,
    faces: Vec<u32>,
    normals: Vec<[f64; 3]>,
}

fn build_face_adjacency(positions: &[f64], indices: &[u32]) -> FaceAdjacency {
    let vertex_count = positions.len() / 3;
    let mut offsets = vec![0usize; vertex_count + 1];
    for &id in indices {
        offsets[id as usize + 1] += 1;
    }
    for i in 0..vertex_count {
        offsets[i + 1] += offsets[i];
    }

    // CSR adjacency retains repeated incidences and their original order.
    let mut faces = vec![0u32; indices.len()];
    let mut cursors = offsets[..vertex_count].to_vec();
    let mut normals = Vec::with_capacity(indices.len() / 3);
    for (t, tri) in indices.as_chunks::<3>().0.iter().enumerate() {
        normals.push(face_normal(positions, tri));
        for &id in tri {
            let cursor = &mut cursors[id as usize];
            faces[*cursor] = t as u32;
            *cursor += 1;
        }
    }

    FaceAdjacency {
        offsets,
        faces,
        normals,
    }
}

/// Inputs are validated export snapshots. Preserve triangle-order summation,
/// first-occurrence property ids, and normals rounded to 1e-7 exactly.
pub(crate) fn render(snapshot: CadMeshBuffer, crease_cosine: f64) -> RenderMesh {
    let positions = &snapshot.positions;
    let indices = &snapshot.indices;
    let vertex_count = positions.len() / 3;
    let adjacency = build_face_adjacency(positions, indices);
    let mut vertices = Vec::<f32>::with_capacity(vertex_count * 6);
    let mut out_indices = Vec::with_capacity(indices.len());
    let mut properties = Vec::<PropertyVertex>::with_capacity(vertex_count);
    let mut head = vec![NONE; vertex_count];
    let mut first = vec![NONE; vertex_count];
    // A source vertex can produce at most one merge record per incident
    // triangle. Reserve that bounded upper shape once instead of growing both
    // transport vectors through repeated reallocations on crease-heavy meshes.
    let merge_capacity = indices.len() / 3;
    let mut merge_from = Vec::with_capacity(merge_capacity);
    let mut merge_to = Vec::with_capacity(merge_capacity);
    for (i, &id) in indices.iter().enumerate() {
        let source = id as usize;
        let face = adjacency.normals[i / 3];
        let mut normal = [0_f64; 3];
        for &t in &adjacency.faces[adjacency.offsets[source]..adjacency.offsets[source + 1]] {
            let n = adjacency.normals[t as usize];
            let alignment = n[0] * face[0] + n[1] * face[1] + n[2] * face[2];
            if alignment >= crease_cosine - 1e-10 {
                normal[0] += n[0];
                normal[1] += n[1];
                normal[2] += n[2];
            }
        }
        let normal = js_normalized(normal, js_hypot3(normal));
        let key = normal.map(|v| js_round(v * 1e7) as i64);
        // Search only properties of this source vertex, usually one smooth
        // normal or a few crease sides. The links use stable output indices.
        let mut index = head[source];
        while index != NONE && properties[index as usize].normal != key {
            index = properties[index as usize].previous;
        }
        if index == NONE {
            index = properties.len() as u32;
            properties.push(PropertyVertex {
                normal: key,
                previous: head[source],
            });
            head[source] = index;
            if first[source] == NONE {
                first[source] = index;
            } else {
                merge_from.push(index);
                merge_to.push(first[source]);
            }
            let p = point(positions, id);
            vertices.extend([
                p[0] as f32,
                p[1] as f32,
                p[2] as f32,
                normal[0] as f32,
                normal[1] as f32,
                normal[2] as f32,
            ]);
        }
        out_indices.push(index);
    }
    RenderMesh {
        vertices,
        indices: out_indices,
        merge_from,
        merge_to,
        face_ids: snapshot.face_ids,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::hint::black_box;
    use std::time::Instant;

    // Original map-of-property-keys implementation, retained only as a byte oracle.
    fn reference(snapshot: CadMeshBuffer, cosine: f64) -> RenderMesh {
        let positions = &snapshot.positions;
        let indices = &snapshot.indices;
        let mut normals = Vec::new();
        let mut adjacent = vec![Vec::new(); positions.len() / 3];
        for (t, tri) in indices.as_chunks::<3>().0.iter().enumerate() {
            normals.push(face_normal(positions, tri));
            for &id in tri {
                adjacent[id as usize].push(t);
            }
        }
        let mut vertices = Vec::new();
        let mut out_indices = Vec::new();
        let mut raw_ids = Vec::new();
        let mut unique = HashMap::new();
        for (i, &id) in indices.iter().enumerate() {
            let face = normals[i / 3];
            let mut normal = [0.; 3];
            for &t in &adjacent[id as usize] {
                let n = normals[t];
                if n[0] * face[0] + n[1] * face[1] + n[2] * face[2] >= cosine - 1e-10 {
                    for k in 0..3 {
                        normal[k] += n[k];
                    }
                }
            }
            let normal = js_normalized(normal, js_hypot3(normal));
            let key = (id, normal.map(|v| js_round(v * 1e7) as i64));
            let next = (vertices.len() / 6) as u32;
            out_indices.push(*unique.entry(key).or_insert_with(|| {
                let p = point(positions, id);
                vertices.extend(p.iter().chain(normal.iter()).map(|&v| v as f32));
                raw_ids.push(id);
                next
            }));
        }
        let mut first = HashMap::new();
        let (mut merge_from, mut merge_to) = (Vec::new(), Vec::new());
        for (i, id) in raw_ids.into_iter().enumerate() {
            if let Some(&previous) = first.get(&id) {
                merge_from.push(i as u32);
                merge_to.push(previous);
            } else {
                first.insert(id, i as u32);
            }
        }
        RenderMesh {
            vertices,
            indices: out_indices,
            merge_from,
            merge_to,
            face_ids: snapshot.face_ids,
        }
    }

    fn check(positions: Vec<f64>, indices: Vec<u32>) {
        for cosine in [-2., -1., 0., (52.5_f64.to_radians()).cos(), 1., 2.] {
            let snapshot = || CadMeshBuffer {
                positions: positions.clone(),
                indices: indices.clone(),
                face_ids: (0..indices.len() as u32 / 3).collect(),
            };
            let actual = render(snapshot(), cosine);
            let expected = reference(snapshot(), cosine);
            assert_eq!(
                actual
                    .vertices
                    .iter()
                    .map(|x| x.to_bits())
                    .collect::<Vec<_>>(),
                expected
                    .vertices
                    .iter()
                    .map(|x| x.to_bits())
                    .collect::<Vec<_>>()
            );
            assert_eq!(actual.indices, expected.indices);
            assert_eq!(actual.merge_from, expected.merge_from);
            assert_eq!(actual.merge_to, expected.merge_to);
            assert_eq!(actual.face_ids, expected.face_ids);
        }
    }

    #[test]
    fn matches_reference_for_primitives_and_dense_smooth_meshes() {
        use polygon_core::solid::primitives::*;
        for mesh in [
            cube([2., 3., 4.], true).unwrap(),
            sphere(30., 128).unwrap(),
            cylinder(8., 3., 3., 128, true).unwrap(),
        ] {
            check(
                mesh.positions,
                mesh.indices.into_iter().map(|i| i as u32).collect(),
            );
        }
    }

    #[test]
    fn preserves_repeated_incidence_degeneracy_and_unused_vertices() {
        check(vec![], vec![]);
        check(
            vec![-0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 1., 9., 9., 9.],
            vec![0, 1, 2, 0, 2, 3, 0, 0, 1, 2, 2, 2, 0, 2, 1],
        );
    }

    #[test]
    #[ignore = "explicit native benchmark; run with --ignored --nocapture"]
    fn native_render_benchmark() {
        use polygon_core::solid::primitives::{cylinder, sphere};

        for (name, mesh) in [
            ("sphere-128", sphere(30., 128).unwrap()),
            ("cylinder-128", cylinder(8., 3., 3., 128, true).unwrap()),
        ] {
            let indices: Vec<u32> = mesh.indices.into_iter().map(|index| index as u32).collect();
            let snapshot = CadMeshBuffer {
                positions: mesh.positions,
                face_ids: (0..indices.len() as u32 / 3).collect(),
                indices,
            };
            let cosine = (52.5_f64.to_radians()).cos();
            let expected = render(
                CadMeshBuffer {
                    positions: snapshot.positions.clone(),
                    indices: snapshot.indices.clone(),
                    face_ids: snapshot.face_ids.clone(),
                },
                cosine,
            );
            let signature = |mesh: &RenderMesh| {
                (
                    mesh.vertices.iter().map(|value| value.to_bits()).collect::<Vec<_>>(),
                    mesh.indices.clone(),
                    mesh.merge_from.clone(),
                    mesh.merge_to.clone(),
                )
            };
            let expected_signature = signature(&expected);
            for _ in 0..2 {
                let result = render(
                    CadMeshBuffer {
                        positions: snapshot.positions.clone(),
                        indices: snapshot.indices.clone(),
                        face_ids: snapshot.face_ids.clone(),
                    },
                    cosine,
                );
                assert_eq!(signature(&result), expected_signature);
            }
            let mut samples = Vec::with_capacity(9);
            for _ in 0..9 {
                let input = CadMeshBuffer {
                    positions: snapshot.positions.clone(),
                    indices: snapshot.indices.clone(),
                    face_ids: snapshot.face_ids.clone(),
                };
                let started = Instant::now();
                let result = black_box(render(black_box(input), cosine));
                samples.push(started.elapsed().as_secs_f64() * 1000.);
                assert_eq!(signature(&result), expected_signature);
                black_box(result);
            }
            samples.sort_by(f64::total_cmp);
            println!(
                "native-render {name}: triangles={} p50_ms={:.3} samples_ms={samples:?}",
                snapshot.indices.len() / 3,
                samples[samples.len() / 2],
            );
        }
    }

    #[test]
    fn preserves_high_valence_crease_groups_and_triangle_order() {
        let mut positions = vec![0., 0., 0.];
        let mut indices = Vec::new();
        for i in 0..257 {
            let angle = i as f64 * 2.399963229728653;
            positions.extend([angle.cos(), angle.sin(), (i % 7) as f64 * 0.25]);
        }
        for i in 1..=257 {
            indices.extend([0, i, i % 257 + 1]);
        }
        check(positions.clone(), indices.clone());
        indices.reverse();
        check(positions, indices);
    }
}
