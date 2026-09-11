//! Deterministic median-split triangle BVH, a 1:1 port of the former
//! TypeScript builder in `services/meshBvh.ts`. Ray queries stay on the host;
//! this module only constructs the compact node/bounds/triangle buffers.
//!
//! Numeric contract: vertex coordinates arrive as f32, are widened to f64,
//! and every arithmetic step follows the JavaScript evaluation order. Centroid
//! and bounds buffers are f32, so computed values are rounded through f32
//! exactly like the Float32Array stores in the reference implementation.

pub const LEAF_BIT: u32 = 0x8000_0000;

/// Math.min/Math.max semantics: NaN propagates, and among equal zeros the
/// sign follows JavaScript (-0 wins min, +0 wins max). Inputs here are always
/// finite, so only the zero-sign rule is observable in the output bytes.
fn js_min(a: f64, b: f64) -> f64 {
    if b < a || (b == a && b.is_sign_negative()) {
        b
    } else {
        a
    }
}

fn js_max(a: f64, b: f64) -> f64 {
    if b > a || (b == a && b.is_sign_positive()) {
        b
    } else {
        a
    }
}

#[derive(Debug, Clone)]
pub struct MeshBvh {
    pub node_count: usize,
    pub bounds: Vec<f32>,
    pub nodes: Vec<u32>,
    pub triangles: Vec<u32>,
}

fn next_power_of_two(value: usize) -> usize {
    if value <= 1 {
        return 1;
    }
    2usize.pow((value as f64).log2().ceil() as u32)
}

struct Builder<'a> {
    triangle_bounds: &'a [f32],
    centroids: &'a [f32],
    source_triangles: &'a [u32],
    order: Vec<u32>,
    leaf_size: usize,
    bounds: Vec<f32>,
    nodes: Vec<u32>,
}

impl Builder<'_> {
    fn compare_records(&self, left: u32, right: u32, axis: usize) -> f64 {
        let difference = self.centroids[left as usize * 3 + axis] as f64
            - self.centroids[right as usize * 3 + axis] as f64;
        if difference != 0.0 {
            return difference;
        }
        // Original triangle number is a stable, deterministic tiebreaker.
        (self.source_triangles[left as usize] as f64)
            - (self.source_triangles[right as usize] as f64)
    }

    /// In-place deterministic quickselect with a three-way partition.
    fn select_nth(&mut self, start: usize, end: usize, nth: usize, axis: usize) {
        let mut low = start;
        let mut high = end;
        while high - low > 1 {
            let middle = low + ((high - low) >> 1);
            let low_record = self.order[low];
            let middle_record = self.order[middle];
            let high_record = self.order[high - 1];
            // Allocation-free median-of-three pivot selection.
            let pivot = if self.compare_records(low_record, middle_record, axis) < 0.0 {
                if self.compare_records(middle_record, high_record, axis) < 0.0 {
                    middle_record
                } else if self.compare_records(low_record, high_record, axis) < 0.0 {
                    high_record
                } else {
                    low_record
                }
            } else if self.compare_records(low_record, high_record, axis) < 0.0 {
                low_record
            } else if self.compare_records(middle_record, high_record, axis) < 0.0 {
                high_record
            } else {
                middle_record
            };

            let mut before = low;
            let mut cursor = low;
            let mut after = high;
            while cursor < after {
                let comparison = self.compare_records(self.order[cursor], pivot, axis);
                if comparison < 0.0 {
                    self.order.swap(before, cursor);
                    before += 1;
                    cursor += 1;
                } else if comparison > 0.0 {
                    after -= 1;
                    self.order.swap(cursor, after);
                } else {
                    cursor += 1;
                }
            }
            if nth < before {
                high = before;
            } else if nth >= after {
                low = after;
            } else {
                return;
            }
        }
    }

    fn build_node(&mut self, start: usize, end: usize) -> usize {
        let node = self.nodes.len() / 2;
        self.nodes.resize(self.nodes.len() + 2, 0);
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        let mut centroid_min = [f64::INFINITY; 3];
        let mut centroid_max = [f64::NEG_INFINITY; 3];

        for slot in start..end {
            let record = self.order[slot] as usize;
            let bounds_offset = record * 6;
            for axis in 0..3 {
                let lo = self.triangle_bounds[bounds_offset + axis] as f64;
                let hi = self.triangle_bounds[bounds_offset + 3 + axis] as f64;
                min[axis] = js_min(min[axis], lo);
                max[axis] = js_max(max[axis], hi);
                let c = self.centroids[record * 3 + axis] as f64;
                centroid_min[axis] = js_min(centroid_min[axis], c);
                centroid_max[axis] = js_max(centroid_max[axis], c);
            }
        }

        for axis in 0..3 {
            self.bounds.push(min[axis] as f32);
        }
        for axis in 0..3 {
            self.bounds.push(max[axis] as f32);
        }

        let count = end - start;
        if count <= self.leaf_size {
            self.nodes[node * 2] = start as u32;
            self.nodes[node * 2 + 1] = LEAF_BIT | count as u32;
            return node;
        }

        let extent_x = centroid_max[0] - centroid_min[0];
        let extent_y = centroid_max[1] - centroid_min[1];
        let extent_z = centroid_max[2] - centroid_min[2];
        // Stable tie order is X, then Y, then Z.
        let mut axis = 0;
        if extent_y > extent_x {
            axis = 1;
        }
        if extent_z > (if axis == 0 { extent_x } else { extent_y }) {
            axis = 2;
        }
        let middle = start + (count >> 1);
        self.select_nth(start, end, middle, axis);
        let left = self.build_node(start, middle);
        let right = self.build_node(middle, end);
        self.nodes[node * 2] = left as u32;
        self.nodes[node * 2 + 1] = right as u32;
        node
    }
}

/// Build the BVH. Inputs are pre-validated by the host: `vertex_stride` and
/// `leaf_size` are already clamped, and the empty-mesh case never reaches
/// this function. Degenerate and non-finite triangles are omitted, matching
/// the reference builder.
pub fn build_mesh_bvh(
    vertices: &[f32],
    indices: &[u32],
    vertex_stride: usize,
    leaf_size: usize,
) -> MeshBvh {
    let triangle_count = indices.len() / 3;
    let vertex_count = vertices.len() / vertex_stride;

    let mut source_triangles = Vec::with_capacity(triangle_count);
    let mut triangle_bounds: Vec<f32> = Vec::with_capacity(triangle_count * 6);
    let mut centroids: Vec<f32> = Vec::with_capacity(triangle_count * 3);

    for triangle in 0..triangle_count {
        let index_offset = triangle * 3;
        let ia = indices[index_offset] as usize;
        let ib = indices[index_offset + 1] as usize;
        let ic = indices[index_offset + 2] as usize;
        if ia >= vertex_count || ib >= vertex_count || ic >= vertex_count {
            continue;
        }

        let a = ia * vertex_stride;
        let b = ib * vertex_stride;
        let c = ic * vertex_stride;
        let ax = vertices[a] as f64;
        let ay = vertices[a + 1] as f64;
        let az = vertices[a + 2] as f64;
        let bx = vertices[b] as f64;
        let by = vertices[b + 1] as f64;
        let bz = vertices[b + 2] as f64;
        let cx = vertices[c] as f64;
        let cy = vertices[c + 1] as f64;
        let cz = vertices[c + 2] as f64;
        if ![ax, ay, az, bx, by, bz, cx, cy, cz]
            .iter()
            .all(|v| v.is_finite())
        {
            continue;
        }

        let e1x = bx - ax;
        let e1y = by - ay;
        let e1z = bz - az;
        let e2x = cx - ax;
        let e2y = cy - ay;
        let e2z = cz - az;
        let nx = e1y * e2z - e1z * e2y;
        let ny = e1z * e2x - e1x * e2z;
        let nz = e1x * e2y - e1y * e2x;
        let area_squared = nx * nx + ny * ny + nz * nz;
        if !(area_squared > 0.0) || !area_squared.is_finite() {
            continue;
        }

        let min_x = js_min(js_min(ax, bx), cx);
        let min_y = js_min(js_min(ay, by), cy);
        let min_z = js_min(js_min(az, bz), cz);
        let max_x = js_max(js_max(ax, bx), cx);
        let max_y = js_max(js_max(ay, by), cy);
        let max_z = js_max(js_max(az, bz), cz);
        triangle_bounds.extend_from_slice(&[
            min_x as f32,
            min_y as f32,
            min_z as f32,
            max_x as f32,
            max_y as f32,
            max_z as f32,
        ]);
        // min + half-extent avoids overflowing where (min + max) / 2 would.
        centroids.extend_from_slice(&[
            (min_x + (max_x - min_x) * 0.5) as f32,
            (min_y + (max_y - min_y) * 0.5) as f32,
            (min_z + (max_z - min_z) * 0.5) as f32,
        ]);
        source_triangles.push(triangle as u32);
    }

    let valid_count = source_triangles.len();
    if valid_count == 0 {
        return MeshBvh {
            node_count: 0,
            bounds: Vec::new(),
            nodes: Vec::new(),
            triangles: Vec::new(),
        };
    }

    // Balanced median splits produce no more than the next power-of-two number
    // of leaves, so the node buffers are pre-sized to that bound.
    let maximum_leaves = next_power_of_two(valid_count.div_ceil(leaf_size));
    let maximum_nodes = maximum_leaves * 2 - 1;
    let mut builder = Builder {
        triangle_bounds: &triangle_bounds,
        centroids: &centroids,
        source_triangles: &source_triangles,
        order: (0..valid_count as u32).collect(),
        leaf_size,
        bounds: Vec::with_capacity(maximum_nodes * 6),
        nodes: Vec::with_capacity(maximum_nodes * 2),
    };
    builder.build_node(0, valid_count);

    let node_count = builder.nodes.len() / 2;
    let triangles = builder
        .order
        .iter()
        .map(|&record| source_triangles[record as usize])
        .collect();
    MeshBvh {
        node_count,
        bounds: builder.bounds,
        nodes: builder.nodes,
        triangles,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vertex_buffer(points: &[[f32; 3]]) -> Vec<f32> {
        points
            .iter()
            .flat_map(|&[x, y, z]| [x, y, z, 0.0, 0.0, 1.0])
            .collect()
    }

    #[test]
    fn omits_invalid_and_degenerate_triangles() {
        let vertices = vertex_buffer(&[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [2.0, 0.0, 0.0],
            [3.0, 0.0, 0.0],
            [4.0, 0.0, 0.0],
        ]);
        let indices = [0, 1, 2, 3, 4, 5, 0, 1, 99];
        let bvh = build_mesh_bvh(&vertices, &indices, 6, 8);
        assert_eq!(bvh.triangles, vec![0]);
        assert_eq!(bvh.node_count, 1);
        assert_eq!(bvh.nodes.len(), 2);
        assert_eq!(bvh.nodes[1], LEAF_BIT | 1);
    }

    #[test]
    fn builds_deterministic_balanced_hierarchy() {
        let mut points = Vec::new();
        let mut indices = Vec::new();
        for x in 0..25 {
            let first = points.len() as u32;
            points.push([x as f32, 0.0, 0.0]);
            points.push([x as f32 + 0.8, 0.0, 0.0]);
            points.push([x as f32, 0.8, 0.0]);
            indices.extend_from_slice(&[first, first + 1, first + 2]);
        }
        let vertices = vertex_buffer(&points);
        let first = build_mesh_bvh(&vertices, &indices, 6, 2);
        let second = build_mesh_bvh(&vertices, &indices, 6, 2);
        assert!(first.node_count > 1);
        let mut sorted = first.triangles.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..25).collect::<Vec<u32>>());
        assert_eq!(first.nodes, second.nodes);
        assert_eq!(first.bounds, second.bounds);
        assert_eq!(first.triangles, second.triangles);
        assert_eq!(first.bounds.len(), first.node_count * 6);
        assert_eq!(first.nodes.len(), first.node_count * 2);
    }

    #[test]
    fn next_power_of_two_matches_log2_ceil() {
        assert_eq!(next_power_of_two(0), 1);
        assert_eq!(next_power_of_two(1), 1);
        assert_eq!(next_power_of_two(2), 2);
        assert_eq!(next_power_of_two(3), 4);
        assert_eq!(next_power_of_two(1024), 1024);
        assert_eq!(next_power_of_two(1025), 2048);
    }
}
