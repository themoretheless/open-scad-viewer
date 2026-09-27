//! Resumable traversal for semantic edges. Keep synchronous traversal free of
//! suspension overhead; numeric primitives and group classification are shared
//! with the synchronous builder. Differential tests enforce identical ordering.
use super::{
    AREA_EPSILON_FACTOR, EdgeKind, LARGE_WELD_VERTEX_THRESHOLD, RADIX_BITS, RADIX_MASK, RADIX_SIZE,
    SMALL_EDGE_SORT_THRESHOLD, SemanticEdgeDiagnostics, SemanticEdges, VERTEX_STRIDE,
    canonical_float_bits, classify_edge_group, hypot3, same_finite_position,
};

struct Work {
    remaining: usize,
}
impl Work {
    async fn tick(&mut self) {
        self.remaining -= 1;
        if self.remaining == 0 {
            self.remaining = 4096;
            // Manually polled by the analysis job owner after a host checkpoint.
            let mut yielded = false;
            std::future::poll_fn(|_| {
                if std::mem::replace(&mut yielded, true) {
                    std::task::Poll::Ready(())
                } else {
                    std::task::Poll::Pending
                }
            })
            .await;
        }
    }
}

async fn find(parent: &mut [u32], mut value: usize, work: &mut Work) -> usize {
    let mut root = value;
    while parent[root] as usize != root {
        work.tick().await;
        root = parent[root] as usize;
    }
    while parent[value] as usize != value {
        work.tick().await;
        let next = parent[value] as usize;
        parent[value] = root as u32;
        value = next;
    }
    root
}

#[inline]
async fn union(parent: &mut [u32], first: usize, second: usize, work: &mut Work) {
    let first_root = find(parent, first, work).await;
    let second_root = find(parent, second, work).await;
    if first_root == second_root {
        return;
    }
    // Keeping the smallest property index as root makes line indices stable.
    if first_root < second_root {
        parent[second_root] = first_root as u32;
    } else {
        parent[first_root] = second_root as u32;
    }
}

async fn counting_pass(
    current: &[u32],
    next: &mut [u32],
    counts: &mut [u32],
    radix_mask: u32,
    digit_of: impl Fn(u32) -> u32,
    work: &mut Work,
) {
    assert!(counts.len() == radix_mask as usize + 1);
    for count in counts.iter_mut() {
        work.tick().await;
        *count = 0;
    }
    for &value in current {
        work.tick().await;
        let digit = (digit_of(value) & radix_mask) as usize;
        unsafe { *counts.get_unchecked_mut(digit) += 1 };
    }
    let mut offset = 0u32;
    for digit in counts.iter_mut() {
        work.tick().await;
        let count = *digit;
        *digit = offset;
        offset += count;
    }
    for &value in current {
        work.tick().await;
        let digit = (digit_of(value) & radix_mask) as usize;
        let slot = unsafe { counts.get_unchecked_mut(digit) };
        next[*slot as usize] = value;
        *slot += 1;
    }
}

async fn weld_exact_positions(vertices: &[f32], parent: &mut [u32], work: &mut Work) {
    let vertex_count = parent.len();
    if vertex_count == 0 {
        return;
    }
    // Every vertex id in `current` is below `vertex_count`, so the xyz reads
    // stay inside `vertices`; checked once here instead of per element.
    assert!(vertex_count * VERTEX_STRIDE <= vertices.len());
    let mut current = Vec::with_capacity(vertex_count);
    for i in 0..vertex_count {
        work.tick().await;
        current.push(i as u32);
    }
    let mut next: Vec<u32> = vec![0; vertex_count];
    let mut counts = vec![0u32; RADIX_SIZE];

    // z, y, x makes x the primary key after stable LSD sorting.
    for coordinate in [2usize, 1, 0] {
        let mut shift = 0;
        while shift < 32 {
            counting_pass(
                &current,
                &mut next,
                &mut counts,
                RADIX_MASK,
                |vertex| {
                    let i = vertex as usize * VERTEX_STRIDE + coordinate;
                    debug_assert!(i < vertices.len());
                    canonical_float_bits(unsafe { vertices.get_unchecked(i) }.to_bits()) >> shift
                },
                work,
            )
            .await;
            std::mem::swap(&mut current, &mut next);
            shift += RADIX_BITS;
        }
    }

    let mut first = current[0];
    for &candidate in &current[1..] {
        work.tick().await;
        if same_finite_position(vertices, first as usize, candidate as usize) {
            union(parent, first as usize, candidate as usize, work).await;
        } else {
            first = candidate;
        }
    }
}

async fn radix_sort_edge_occurrences(
    edge_a: &[u32],
    edge_b: &[u32],
    occurrence_order: &[u32],
    edge_count: usize,
    work: &mut Work,
) -> Vec<u32> {
    let mut current = Vec::with_capacity(edge_count);
    for &v in &occurrence_order[..edge_count] {
        work.tick().await;
        current.push(v);
    }
    if edge_count < 2 {
        return current;
    }
    let mut next: Vec<u32> = vec![0; edge_count];
    // Tiny bodies otherwise allocate and scan 65,536 buckets for just a
    // handful of edges; keep the eight-bit path for small occurrence streams.
    let radix_bits = if edge_count < SMALL_EDGE_SORT_THRESHOLD {
        8
    } else {
        RADIX_BITS
    };
    let radix_size = 1usize << radix_bits;
    let radix_mask = (radix_size - 1) as u32;
    let mut counts = vec![0u32; radix_size];

    // b is the secondary key and therefore sorted first.
    for values in [edge_b, edge_a] {
        let mut shift = 0;
        while shift < 32 {
            counting_pass(
                &current,
                &mut next,
                &mut counts,
                radix_mask,
                |occurrence| values[occurrence as usize] >> shift,
                work,
            )
            .await;
            std::mem::swap(&mut current, &mut next);
            shift += radix_bits;
        }
    }
    current
}

pub async fn extract_semantic_edges_cooperative(
    vertices: &[f32],
    triangle_indices: &[u32],
    merge_from: &[u32],
    merge_to: &[u32],
    weld_coincident: bool,
    crease_dot_threshold: f64,
) -> SemanticEdges {
    let mut work = Work { remaining: 4096 };
    let vertex_count = vertices.len() / VERTEX_STRIDE;

    let mut parent = Vec::with_capacity(vertex_count);
    for i in 0..vertex_count {
        work.tick().await;
        parent.push(i as u32);
    }
    for i in 0..merge_from.len().min(merge_to.len()) {
        work.tick().await;
        union(
            &mut parent,
            merge_from[i] as usize,
            merge_to[i] as usize,
            &mut work,
        )
        .await;
    }

    if weld_coincident && vertex_count > 0 {
        let _ = LARGE_WELD_VERTEX_THRESHOLD; // single radix path covers all sizes
        weld_exact_positions(vertices, &mut parent, &mut work).await;
    }

    // Fully compress roots so representative IDs and output ordering are stable.
    for i in 0..vertex_count {
        work.tick().await;
        let root = find(&mut parent, i, &mut work).await;
        parent[i] = root as u32;
    }

    let max_edge_occurrences = triangle_indices.len();
    let mut edge_a = vec![0u32; max_edge_occurrences];
    let mut edge_b = vec![0u32; max_edge_occurrences];
    // f32 storage matches the Float32Array rounding of the reference.
    let mut face_normals = vec![0f32; triangle_indices.len()];
    let mut occurrence_order = vec![0u32; max_edge_occurrences];
    let mut edge_count = 0usize;
    let mut degenerate = 0u32;

    let add_occurrence = |edge_a: &mut [u32],
                          edge_b: &mut [u32],
                          occurrence_order: &mut [u32],
                          edge_count: usize,
                          occurrence: usize,
                          first: u32,
                          second: u32| {
        edge_a[occurrence] = first.min(second);
        edge_b[occurrence] = first.max(second);
        occurrence_order[edge_count] = occurrence as u32;
        edge_count + 1
    };

    let mut i = 0usize;
    while i < triangle_indices.len() {
        work.tick().await;
        let i0 = triangle_indices[i] as usize;
        let i1 = triangle_indices[i + 1] as usize;
        let i2 = triangle_indices[i + 2] as usize;
        let v0 = parent[i0];
        let v1 = parent[i1];
        let v2 = parent[i2];

        if v0 == v1 || v1 == v2 || v2 == v0 {
            degenerate += 1;
            i += 3;
            continue;
        }

        let p0 = i0 * VERTEX_STRIDE;
        let p1 = i1 * VERTEX_STRIDE;
        let p2 = i2 * VERTEX_STRIDE;
        let e10x = vertices[p1] as f64 - vertices[p0] as f64;
        let e10y = vertices[p1 + 1] as f64 - vertices[p0 + 1] as f64;
        let e10z = vertices[p1 + 2] as f64 - vertices[p0 + 2] as f64;
        let e20x = vertices[p2] as f64 - vertices[p0] as f64;
        let e20y = vertices[p2 + 1] as f64 - vertices[p0 + 1] as f64;
        let e20z = vertices[p2 + 2] as f64 - vertices[p0 + 2] as f64;
        let cross_x = e10y * e20z - e10z * e20y;
        let cross_y = e10z * e20x - e10x * e20z;
        let cross_z = e10x * e20y - e10y * e20x;
        let twice_area = hypot3(cross_x, cross_y, cross_z);
        let edge_scale =
            e10x * e10x + e10y * e10y + e10z * e10z + e20x * e20x + e20y * e20y + e20z * e20z;
        let area_epsilon = edge_scale * AREA_EPSILON_FACTOR;

        if !twice_area.is_finite() || twice_area <= area_epsilon {
            degenerate += 1;
            i += 3;
            continue;
        }

        face_normals[i] = (cross_x / twice_area) as f32;
        face_normals[i + 1] = (cross_y / twice_area) as f32;
        face_normals[i + 2] = (cross_z / twice_area) as f32;
        edge_count = add_occurrence(
            &mut edge_a,
            &mut edge_b,
            &mut occurrence_order,
            edge_count,
            i,
            v0,
            v1,
        );
        edge_count = add_occurrence(
            &mut edge_a,
            &mut edge_b,
            &mut occurrence_order,
            edge_count,
            i + 1,
            v1,
            v2,
        );
        edge_count = add_occurrence(
            &mut edge_a,
            &mut edge_b,
            &mut occurrence_order,
            edge_count,
            i + 2,
            v2,
            v0,
        );
        i += 3;
    }

    let sorted_occurrences =
        radix_sort_edge_occurrences(&edge_a, &edge_b, &occurrence_order, edge_count, &mut work)
            .await;

    let mut diagnostics = SemanticEdgeDiagnostics {
        boundary: 0,
        crease: 0,
        non_manifold: 0,
        degenerate,
    };

    // Single classification pass: diagnostics and the emitted edge pairs are
    // recorded together, so groups are never classified twice.
    let mut emitted: Vec<(u32, u32)> = Vec::with_capacity(edge_count);
    let mut group_start = 0usize;
    while group_start < edge_count {
        work.tick().await;
        let first_occurrence = sorted_occurrences[group_start] as usize;
        let a = edge_a[first_occurrence];
        let b = edge_b[first_occurrence];
        let mut group_end = group_start + 1;
        while group_end < edge_count {
            work.tick().await;
            let occurrence = sorted_occurrences[group_end] as usize;
            if edge_a[occurrence] != a || edge_b[occurrence] != b {
                break;
            }
            group_end += 1;
        }
        match classify_edge_group(
            &sorted_occurrences,
            group_start,
            group_end,
            &face_normals,
            crease_dot_threshold,
        ) {
            EdgeKind::Hidden => {}
            kind => {
                match kind {
                    EdgeKind::Boundary => diagnostics.boundary += 1,
                    EdgeKind::Crease => diagnostics.crease += 1,
                    EdgeKind::NonManifold => diagnostics.non_manifold += 1,
                    EdgeKind::Hidden => unreachable!(),
                }
                emitted.push((a, b));
            }
        }
        group_start = group_end;
    }

    let output_edge_count = diagnostics.boundary + diagnostics.crease + diagnostics.non_manifold;
    let mut indices = Vec::with_capacity(emitted.len() * 2);
    for (a, b) in emitted {
        work.tick().await;
        indices.push(a);
        indices.push(b);
    }
    debug_assert_eq!(indices.len(), output_edge_count as usize * 2);

    SemanticEdges {
        indices,
        diagnostics,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use std::task::{Context, Poll, Waker};

    fn finish(future: impl Future<Output = SemanticEdges>) -> (SemanticEdges, usize) {
        let mut future = std::pin::pin!(future);
        let mut cx = Context::from_waker(Waker::noop());
        let mut pending = 0;
        loop {
            match future.as_mut().poll(&mut cx) {
                Poll::Ready(result) => return (result, pending),
                Poll::Pending => pending += 1,
            }
        }
    }

    fn mesh(triangles: usize) -> (Vec<f32>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for i in 0..triangles {
            let x = (i % 113) as f32;
            let y = (i / 113) as f32;
            for point in [[x, y, 0.], [x + 1., y, 0.], [x, y + 1., 0.]] {
                vertices.extend_from_slice(&[point[0], point[1], point[2], 0., 0., 1.]);
                indices.push(indices.len() as u32);
            }
        }
        (vertices, indices)
    }

    #[test]
    fn matches_sync_across_radix_thresholds_welding_and_degeneracy() {
        for count in [0, 1, 100, 21_845, 21_846, 30_000] {
            let (mut vertices, mut indices) = mesh(count);
            if count > 100 {
                vertices[0] = -0.;
                vertices[6] = f32::NAN;
                indices[6] = indices[7];
            }
            for weld in [false, true] {
                let expected =
                    super::super::extract_semantic_edges(&vertices, &indices, &[], &[], weld, 0.8);
                let (actual, pending) = finish(extract_semantic_edges_cooperative(
                    &vertices,
                    &indices,
                    &[],
                    &[],
                    weld,
                    0.8,
                ));
                assert_eq!(
                    actual.indices, expected.indices,
                    "count={count}, weld={weld}"
                );
                assert_eq!(actual.diagnostics, expected.diagnostics);
                if count > 100 {
                    assert!(pending > 10);
                }
            }
        }
    }

    #[test]
    fn long_merge_chains_and_nonmanifold_groups_yield() {
        let (vertices, _) = mesh(10_000);
        let from: Vec<u32> = (1..30_000).rev().collect();
        let to: Vec<u32> = from.iter().map(|v| v - 1).collect();
        let indices = vec![0, 1, 2].repeat(20_000);
        for merges in [false, true] {
            let (a, b) = if merges {
                (from.as_slice(), to.as_slice())
            } else {
                (&[][..], &[][..])
            };
            let expected =
                super::super::extract_semantic_edges(&vertices, &indices, a, b, false, 0.8);
            let (actual, pending) = finish(extract_semantic_edges_cooperative(
                &vertices, &indices, a, b, false, 0.8,
            ));
            assert_eq!(actual.indices, expected.indices);
            assert_eq!(actual.diagnostics, expected.diagnostics);
            assert!(pending > 10);
        }
    }

    #[test]
    fn dropping_at_different_steps_preserves_inputs_and_later_results() {
        let (vertices, indices) = mesh(30_000);
        let expected =
            super::super::extract_semantic_edges(&vertices, &indices, &[], &[], true, 0.8);
        for steps in [1, 10, 100, 300] {
            let mut future = Box::pin(extract_semantic_edges_cooperative(
                &vertices,
                &indices,
                &[],
                &[],
                true,
                0.8,
            ));
            let mut cx = Context::from_waker(Waker::noop());
            for _ in 0..steps {
                assert!(future.as_mut().poll(&mut cx).is_pending());
            }
            drop(future);
        }
        let (actual, _) = finish(extract_semantic_edges_cooperative(
            &vertices,
            &indices,
            &[],
            &[],
            true,
            0.8,
        ));
        assert_eq!(actual.indices, expected.indices);
        assert_eq!(actual.diagnostics, expected.diagnostics);
    }
}
