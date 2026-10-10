// WebGPU Compute Shader: Multi-Plane Mesh Slicer & Cross-Section Contour Extractor.
//
// Intersects a triangle mesh with a stack of parallel slicing planes in parallel:
// 1. O(1) per-triangle layer range culling [layer_start..layer_end].
// 2. Exact edge-plane linear interpolation.
// 3. Canonical counter-clockwise segment winding via (plane_normal x tri_normal).
// 4. Emits 3D contour segments [q1.x, q1.y, q1.z, layer_idx, q2.x, q2.y, q2.z, tri_idx].

struct SlicerParams {
  total_triangles: u32,
  num_layers: u32,
  stride_floats: u32,
  pos_offset: u32,
  plane_nx: f32,
  plane_ny: f32,
  plane_nz: f32,
  first_layer_d: f32,
  layer_step: f32,
  max_segments: u32,
  _pad0: u32,
  _pad1: u32,
};

@group(0) @binding(0) var<uniform> params: SlicerParams;
@group(0) @binding(1) var<storage, read> vertices: array<f32>;
@group(0) @binding(2) var<storage, read> indices: array<u32>;
@group(0) @binding(3) var<storage, read_write> segment_counter: array<atomic<u32>, 1>;
// Each emitted segment is 8 floats: [p0.x, p0.y, p0.z, f32(layer_idx), p1.x, p1.y, p1.z, f32(tri_idx)]
@group(0) @binding(4) var<storage, read_write> out_segments: array<f32>;

fn get_pos(v_idx: u32) -> vec3<f32> {
  let base = v_idx * params.stride_floats + params.pos_offset;
  return vec3<f32>(
    vertices[base + 0u],
    vertices[base + 1u],
    vertices[base + 2u]
  );
}

fn interp_edge(p0: vec3<f32>, s0: f32, p1: vec3<f32>, s1: f32) -> vec3<f32> {
  let denom = s1 - s0;
  if (abs(denom) < 1e-9) {
    return (p0 + p1) * 0.5;
  }
  let t = clamp(-s0 / denom, 0.0, 1.0);
  return p0 + t * (p1 - p0);
}

const WG: u32 = 256;

@compute @workgroup_size(WG)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let tri_idx = id.x;
  if (tri_idx >= params.total_triangles) {
    return;
  }

  let i0 = indices[tri_idx * 3u + 0u];
  let i1 = indices[tri_idx * 3u + 1u];
  let i2 = indices[tri_idx * 3u + 2u];

  let p0 = get_pos(i0);
  let p1 = get_pos(i1);
  let p2 = get_pos(i2);

  let plane_n = normalize(vec3<f32>(params.plane_nx, params.plane_ny, params.plane_nz));

  let d0 = dot(plane_n, p0);
  let d1 = dot(plane_n, p1);
  let d2 = dot(plane_n, p2);

  let d_min = min(d0, min(d1, d2));
  let d_max = max(d0, max(d1, d2));

  // Compute intersecting layer range
  var l_start = 0;
  var l_end = i32(params.num_layers) - 1;

  if (params.layer_step > 1e-7) {
    l_start = max(0, i32(ceil((d_min - params.first_layer_d) / params.layer_step)));
    l_end = min(i32(params.num_layers) - 1, i32(floor((d_max - params.first_layer_d) / params.layer_step)));
  } else {
    // Single layer check
    if (params.first_layer_d < d_min || params.first_layer_d > d_max) {
      return;
    }
    l_start = 0;
    l_end = 0;
  }

  if (l_start > l_end) {
    return;
  }

  let tri_n = cross(p1 - p0, p2 - p0);
  // Counter-clockwise tangent direction in slice plane: plane_n x tri_n
  let ccw_dir = cross(plane_n, tri_n);

  let pts = array<vec3<f32>, 3>(p0, p1, p2);
  let dists = array<f32, 3>(d0, d1, d2);

  for (var layer = l_start; layer <= l_end; layer++) {
    let plane_d = params.first_layer_d + f32(layer) * params.layer_step;

    let s0 = d0 - plane_d;
    let s1 = d1 - plane_d;
    let s2 = d2 - plane_d;
    let s_arr = array<f32, 3>(s0, s1, s2);

    var neg_idx: array<u32, 3>;
    var pos_idx: array<u32, 3>;
    var neg_len = 0u;
    var pos_len = 0u;

    for (var k = 0u; k < 3u; k++) {
      if (s_arr[k] < 0.0) {
        neg_idx[neg_len] = k;
        neg_len++;
      } else {
        pos_idx[pos_len] = k;
        pos_len++;
      }
    }

    if (neg_len == 0u || pos_len == 0u) {
      continue;
    }

    var solo = 0u;
    var pair0 = 0u;
    var pair1 = 0u;

    if (neg_len == 1u) {
      solo = neg_idx[0];
      pair0 = pos_idx[0];
      pair1 = pos_idx[1];
    } else {
      solo = pos_idx[0];
      pair0 = neg_idx[0];
      pair1 = neg_idx[1];
    }

    var q1 = interp_edge(pts[solo], s_arr[solo], pts[pair0], s_arr[pair0]);
    var q2 = interp_edge(pts[solo], s_arr[solo], pts[pair1], s_arr[pair1]);

    // Enforce counter-clockwise orientation around plane_n
    if (dot(ccw_dir, q2 - q1) < 0.0) {
      let tmp = q1;
      q1 = q2;
      q2 = tmp;
    }

    // Skip degenerate zero-length segments
    if (length(q2 - q1) < 1e-7) {
      continue;
    }

    let slot = atomicAdd(&segment_counter[0], 1u);
    if (slot < params.max_segments) {
      let off = slot * 8u;
      out_segments[off + 0u] = q1.x;
      out_segments[off + 1u] = q1.y;
      out_segments[off + 2u] = q1.z;
      out_segments[off + 3u] = f32(layer);
      out_segments[off + 4u] = q2.x;
      out_segments[off + 5u] = q2.y;
      out_segments[off + 6u] = q2.z;
      out_segments[off + 7u] = f32(tri_idx);
    }
  }
}
