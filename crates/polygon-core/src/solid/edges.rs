//! Semantic edge extraction, a 1:1 port of the former TypeScript module
//! `services/meshTopology.ts`. Returns boundary, crease and non-manifold
//! edges while removing coplanar triangulation diagonals.
//!
//! Numeric contract: positions arrive as f32 and are widened to f64; face
//! normals are rounded through f32 (the reference stores them in a
//! Float32Array); `Math.hypot(x, y, z)` is reproduced with the scale-down
//! algorithm rather than a hypot chain. Inputs are pre-validated by the host
//! (stride, index ranges, merge-array pairing, crease-angle finiteness).

pub const VERTEX_STRIDE: usize = 6;
const LARGE_WELD_VERTEX_THRESHOLD: usize = 65_536;
const RADIX_BITS: u32 = 16;
const RADIX_SIZE: usize = 1 << RADIX_BITS;
const RADIX_MASK: u32 = (RADIX_SIZE - 1) as u32;
const SMALL_EDGE_SORT_THRESHOLD: usize = 65_536;
/// f64::EPSILON * 16, as in the reference (`Number.EPSILON * 16`).
const AREA_EPSILON_FACTOR: f64 = f64::EPSILON * 16.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticEdgeDiagnostics {
    pub boundary: u32,
    pub crease: u32,
    pub non_manifold: u32,
    pub degenerate: u32,
}

#[derive(Debug, Clone)]
pub struct SemanticEdges {
    /// Pairs of vertex indices suitable for a GPU line-list index buffer.
    pub indices: Vec<u32>,
    pub diagnostics: SemanticEdgeDiagnostics,
}

/// `Math.hypot(x, y, z)` with V8/fdlibm scaling semantics: infinite arguments
/// yield infinity, NaN propagates, and the scaled sum is computed in argument
/// order. A naive `x.hypot(y).hypot(z)` chain has different rounding.
fn hypot3(x: f64, y: f64, z: f64) -> f64 {
    if x.is_nan() || y.is_nan() || z.is_nan() {
        if x.is_infinite() || y.is_infinite() || z.is_infinite() {
            return f64::INFINITY;
        }
        return f64::NAN;
    }
    if x.is_infinite() || y.is_infinite() || z.is_infinite() {
        return f64::INFINITY;
    }
    let ax = x.abs();
    let ay = y.abs();
    let az = z.abs();
    let m = ax.max(ay).max(az);
    if m == 0.0 {
        return 0.0;
    }
    let sx = ax / m;
    let sy = ay / m;
    let sz = az / m;
    m * (sx * sx + sy * sy + sz * sz).sqrt()
}

fn find(parent: &mut [u32], mut value: usize) -> usize {
    let mut root = value;
    while parent[root] as usize != root {
        root = parent[root] as usize;
    }
    while parent[value] as usize != value {
        let next = parent[value] as usize;
        parent[value] = root as u32;
        value = next;
    }
    root
}

fn union(parent: &mut [u32], first: usize, second: usize) {
    let first_root = find(parent, first);
    let second_root = find(parent, second);
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

fn canonical_float_bits(value: u32) -> u32 {
    if value & 0x7fff_ffff == 0 {
        0
    } else {
        value
    }
}

fn same_finite_position(vertices: &[f32], first: usize, second: usize) -> bool {
    let first_offset = first * VERTEX_STRIDE;
    let second_offset = second * VERTEX_STRIDE;
    let x = vertices[first_offset];
    let y = vertices[first_offset + 1];
    let z = vertices[first_offset + 2];
    x.is_finite()
        && y.is_finite()
        && z.is_finite()
        && x == vertices[second_offset]
        && y == vertices[second_offset + 1]
        && z == vertices[second_offset + 2]
}

/// Exact-position welding. Six stable 16-bit radix passes group identical
/// Float32 xyz bits; -0 is canonicalized to +0 and non-finite vertices remain
/// unwelded. With min-root union and full root compression this yields the
/// same groups as the reference's small-mesh map path, so only the radix
/// variant is ported.
fn weld_exact_positions(vertices: &[f32], parent: &mut [u32]) {
    let vertex_count = parent.len();
    if vertex_count == 0 {
        return;
    }
    let mut current: Vec<u32> = (0..vertex_count as u32).collect();
    let mut next: Vec<u32> = vec![0; vertex_count];
    let mut counts = vec![0u32; RADIX_SIZE];

    let bits = |vertex: u32, coordinate: usize| -> u32 {
        vertices[vertex as usize * VERTEX_STRIDE + coordinate].to_bits()
    };
    // z, y, x makes x the primary key after stable LSD sorting.
    for coordinate in [2usize, 1, 0] {
        let mut shift = 0;
        while shift < 32 {
            counts.fill(0);
            for &vertex in &current {
                let value = canonical_float_bits(bits(vertex, coordinate));
                counts[((value >> shift) & RADIX_MASK) as usize] += 1;
            }
            let mut offset = 0u32;
            for digit in counts.iter_mut() {
                let count = *digit;
                *digit = offset;
                offset += count;
            }
            for &vertex in &current {
                let value = canonical_float_bits(bits(vertex, coordinate));
                let digit = ((value >> shift) & RADIX_MASK) as usize;
                next[counts[digit] as usize] = vertex;
                counts[digit] += 1;
            }
            std::mem::swap(&mut current, &mut next);
            shift += RADIX_BITS;
        }
    }

    let mut first = current[0];
    for &candidate in &current[1..] {
        if same_finite_position(vertices, first as usize, candidate as usize) {
            union(parent, first as usize, candidate as usize);
        } else {
            first = candidate;
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EdgeKind {
    Hidden = 0,
    Boundary = 1,
    Crease = 2,
    NonManifold = 3,
}

/// Stable LSD radix sort by the (a, b) Uint32 pair.
fn radix_sort_edge_occurrences(
    edge_a: &[u32],
    edge_b: &[u32],
    occurrence_order: &[u32],
    edge_count: usize,
) -> Vec<u32> {
    let mut current: Vec<u32> = occurrence_order[..edge_count].to_vec();
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
            counts.fill(0);
            for &occurrence in &current {
                counts[((values[occurrence as usize] >> shift) & radix_mask) as usize] += 1;
            }
            let mut offset = 0u32;
            for digit in counts.iter_mut() {
                let count = *digit;
                *digit = offset;
                offset += count;
            }
            for &occurrence in &current {
                let digit = ((values[occurrence as usize] >> shift) & radix_mask) as usize;
                next[counts[digit] as usize] = occurrence;
                counts[digit] += 1;
            }
            std::mem::swap(&mut current, &mut next);
            shift += radix_bits;
        }
    }
    current
}

fn classify_edge_group(
    sorted_occurrences: &[u32],
    group_start: usize,
    group_end: usize,
    face_normals: &[f32],
    crease_dot_threshold: f64,
) -> EdgeKind {
    let incident_faces = group_end - group_start;
    if incident_faces == 1 {
        return EdgeKind::Boundary;
    }
    if incident_faces > 2 {
        return EdgeKind::NonManifold;
    }

    let first = sorted_occurrences[group_start] as usize;
    let second = sorted_occurrences[group_start + 1] as usize;
    let first_face = first - (first % 3);
    let second_face = second - (second % 3);
    let dot = (face_normals[first_face] as f64 * face_normals[second_face] as f64
        + face_normals[first_face + 1] as f64 * face_normals[second_face + 1] as f64
        + face_normals[first_face + 2] as f64 * face_normals[second_face + 2] as f64)
        .clamp(-1.0, 1.0);
    // A tiny margin prevents Float32 noise from turning a nominally coplanar
    // triangulation diagonal into an edge.
    if dot < crease_dot_threshold - 1e-7 {
        EdgeKind::Crease
    } else {
        EdgeKind::Hidden
    }
}

/// Extract semantic edges. `crease_dot_threshold` is computed by the host
/// (Math.cos of the clamped angle) so the comparison constant matches the
/// JavaScript evaluation bit-for-bit.
pub fn extract_semantic_edges(
    vertices: &[f32],
    triangle_indices: &[u32],
    merge_from: &[u32],
    merge_to: &[u32],
    weld_coincident: bool,
    crease_dot_threshold: f64,
) -> SemanticEdges {
    let vertex_count = vertices.len() / VERTEX_STRIDE;

    let mut parent: Vec<u32> = (0..vertex_count as u32).collect();
    for i in 0..merge_from.len().min(merge_to.len()) {
        union(&mut parent, merge_from[i] as usize, merge_to[i] as usize);
    }

    if weld_coincident && vertex_count > 0 {
        let _ = LARGE_WELD_VERTEX_THRESHOLD; // single radix path covers all sizes
        weld_exact_positions(vertices, &mut parent);
    }

    // Fully compress roots so representative IDs and output ordering are stable.
    for i in 0..vertex_count {
        let root = find(&mut parent, i);
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
        radix_sort_edge_occurrences(&edge_a, &edge_b, &occurrence_order, edge_count);

    let mut diagnostics = SemanticEdgeDiagnostics {
        boundary: 0,
        crease: 0,
        non_manifold: 0,
        degenerate,
    };

    let mut group_start = 0usize;
    while group_start < edge_count {
        let first_occurrence = sorted_occurrences[group_start] as usize;
        let a = edge_a[first_occurrence];
        let b = edge_b[first_occurrence];
        let mut group_end = group_start + 1;
        while group_end < edge_count {
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
            EdgeKind::Boundary => diagnostics.boundary += 1,
            EdgeKind::Crease => diagnostics.crease += 1,
            EdgeKind::NonManifold => diagnostics.non_manifold += 1,
        }
        group_start = group_end;
    }

    let output_edge_count = diagnostics.boundary + diagnostics.crease + diagnostics.non_manifold;
    let mut output = Vec::with_capacity(output_edge_count as usize * 2);
    let mut group_start = 0usize;
    while group_start < edge_count {
        let first_occurrence = sorted_occurrences[group_start] as usize;
        let a = edge_a[first_occurrence];
        let b = edge_b[first_occurrence];
        let mut group_end = group_start + 1;
        while group_end < edge_count {
            let occurrence = sorted_occurrences[group_end] as usize;
            if edge_a[occurrence] != a || edge_b[occurrence] != b {
                break;
            }
            group_end += 1;
        }
        if classify_edge_group(
            &sorted_occurrences,
            group_start,
            group_end,
            &face_normals,
            crease_dot_threshold,
        ) != EdgeKind::Hidden
        {
            output.push(a);
            output.push(b);
        }
        group_start = group_end;
    }

    SemanticEdges {
        indices: output,
        diagnostics,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vertices(positions: &[[f32; 3]]) -> Vec<f32> {
        positions
            .iter()
            .flat_map(|&[x, y, z]| [x, y, z, 0.0, 0.0, 1.0])
            .collect()
    }

    fn cos_degrees(degrees: f64) -> f64 {
        (degrees * std::f64::consts::PI / 180.0).cos()
    }

    #[test]
    fn removes_coplanar_diagonal_from_triangulated_quad() {
        let result = extract_semantic_edges(
            &vertices(&[
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ]),
            &[0, 1, 2, 0, 2, 3],
            &[],
            &[],
            true,
            cos_degrees(30.0),
        );
        assert_eq!(result.indices, vec![0, 1, 0, 3, 1, 2, 2, 3]);
        assert_eq!(
            result.diagnostics,
            SemanticEdgeDiagnostics {
                boundary: 4,
                crease: 0,
                non_manifold: 0,
                degenerate: 0,
            }
        );
    }

    #[test]
    fn keeps_twelve_creases_of_a_cube() {
        let cube_vertices = vertices(&[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
        ]);
        let cube_triangles = [
            0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7,
            6, 3, 0, 4, 3, 4, 7,
        ];
        let result = extract_semantic_edges(
            &cube_vertices,
            &cube_triangles,
            &[],
            &[],
            true,
            cos_degrees(30.0),
        );
        assert_eq!(result.indices.len(), 12 * 2);
        assert_eq!(result.diagnostics.crease, 12);
        assert_eq!(result.diagnostics.boundary, 0);
    }

    #[test]
    fn reconstructs_topology_from_duplicated_property_vertices() {
        let duplicated = vertices(&[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ]);
        let triangles = [0, 1, 2, 3, 4, 5];
        let exact_weld =
            extract_semantic_edges(&duplicated, &triangles, &[], &[], true, cos_degrees(30.0));
        let manifold_merge = extract_semantic_edges(
            &duplicated,
            &triangles,
            &[3, 4],
            &[0, 2],
            false,
            cos_degrees(30.0),
        );
        assert_eq!(exact_weld.indices, vec![0, 1, 0, 5, 1, 2, 2, 5]);
        assert_eq!(manifold_merge.indices, exact_weld.indices);
        assert_eq!(manifold_merge.diagnostics, exact_weld.diagnostics);
    }

    #[test]
    fn ignores_degenerate_triangles() {
        let result = extract_semantic_edges(
            &vertices(&[
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [2.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ]),
            &[0, 0, 1, 0, 1, 2, 0, 1, 3],
            &[],
            &[],
            true,
            cos_degrees(30.0),
        );
        assert_eq!(result.indices, vec![0, 1, 0, 3, 1, 3]);
        assert_eq!(
            result.diagnostics,
            SemanticEdgeDiagnostics {
                boundary: 3,
                crease: 0,
                non_manifold: 0,
                degenerate: 2,
            }
        );
    }

    #[test]
    fn crease_threshold_is_strict_and_deterministic() {
        let folded = vertices(&[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ]);
        let forward = [0u32, 1, 2, 1, 0, 3];
        let reversed = [1u32, 0, 3, 0, 1, 2];
        let visible = extract_semantic_edges(&folded, &forward, &[], &[], true, cos_degrees(45.0));
        let hidden = extract_semantic_edges(&folded, &forward, &[], &[], true, cos_degrees(100.0));
        let reordered =
            extract_semantic_edges(&folded, &reversed, &[], &[], true, cos_degrees(45.0));
        assert_eq!(visible.diagnostics.crease, 1);
        assert_eq!(hidden.diagnostics.crease, 0);
        assert_eq!(hidden.indices.len(), 4 * 2);
        assert_eq!(reordered.indices, visible.indices);
        assert_eq!(reordered.diagnostics, visible.diagnostics);
    }

    #[test]
    fn welds_minus_zero_to_plus_zero() {
        let duplicated = vertices(&[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [-0.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ]);
        let result = extract_semantic_edges(
            &duplicated,
            &[0, 1, 2, 3, 4, 5],
            &[],
            &[],
            true,
            cos_degrees(30.0),
        );
        assert_eq!(result.indices, vec![0, 1, 0, 5, 1, 2, 2, 5]);
    }

    #[test]
    fn hypot3_matches_reference_values() {
        assert_eq!(hypot3(0.0, 0.0, 0.0), 0.0);
        assert_eq!(hypot3(3.0, 4.0, 12.0), 13.0);
        assert_eq!(hypot3(f64::INFINITY, f64::NAN, 1.0), f64::INFINITY);
        assert!(hypot3(f64::NAN, 1.0, 2.0).is_nan());
        assert_eq!(hypot3(1e300, 1e300, 1e300), 1e300 * 3.0f64.sqrt());
    }
}
