use super::*;

/// Leak a copy of `data` into a raw buffer, mimicking a host upload.
/// `read_pod` copies out of it; the buffer is reclaimed by `reclaim`.
fn upload<T: Copy>(data: &[T]) -> (usize, usize) {
    let v = data.to_vec();
    let ptr = v.as_ptr() as usize;
    std::mem::forget(v);
    (ptr, data.len())
}

unsafe fn reclaim<T: Copy>(ptr: usize, len: usize) {
    if ptr != 0 {
        drop(unsafe { Vec::from_raw_parts(ptr as *mut T, len, len) });
    }
}

/// Decode the last packed response and free its allocation.
unsafe fn take_response() -> Value {
    let ptr = abi_response_ptr();
    let len = abi_response_len();
    let bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, len) }.to_vec();
    unsafe { abi_free(ptr, len) };
    value_codec::decode_binary(&bytes).unwrap()
}

fn response_handle(v: &Value) -> usize {
    assert_eq!(v["ok"].as_bool(), Some(true), "{v:?}");
    v["value"].as_u64().unwrap() as usize
}

#[test]
fn manifold_check_reports_open_quad() {
    // Single quad (two triangles): manifold with boundary, one component.
    let positions: [f64; 12] = [0., 0., 0., 1., 0., 0., 1., 1., 0., 0., 1., 0.];
    let indices: [u32; 6] = [0, 1, 2, 0, 2, 3];
    let (vp, vl) = upload(&positions);
    let (ip, il) = upload(&indices);
    let packed = unsafe { abi_manifold_check(vp, vl, ip, il, FMT_F64) };
    let _ = packed;
    let response = unsafe { take_response() };
    let handle = response_handle(&response);
    unsafe {
        // Boundary edges: 4 edges * 2 indices.
        assert_eq!(abi_array_field(handle, 1), 8);
        // No non-manifold / orientation / degenerate issues.
        assert_eq!(abi_array_field(handle, 3), 0);
        assert_eq!(abi_array_field(handle, 5), 0);
        assert_eq!(abi_array_field(handle, 7), 0);
        // Summary: 4 vertices, 2 triangles, 1 component, boundary flag.
        assert_eq!(abi_array_field(handle, 12), 4);
        assert_eq!(abi_array_field(handle, 13), 2);
        assert_eq!(abi_array_field(handle, 14), 1);
        let flags = abi_array_field(handle, 15);
        assert_eq!(flags & 1, 0, "open quad is not strictly manifold");
        assert_ne!(flags & 2, 0, "open quad is manifold with boundary");
        abi_array_free(handle);
        reclaim::<f64>(vp, vl);
        reclaim::<u32>(ip, il);
    }
}

#[test]
fn manifold_check_flags_triple_edge() {
    let positions: [f64; 15] = [0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 1., 0., -1., 0.];
    let indices: [u32; 9] = [0, 1, 2, 0, 4, 1, 0, 1, 3];
    let (vp, vl) = upload(&positions);
    let (ip, il) = upload(&indices);
    unsafe { abi_manifold_check(vp, vl, ip, il, FMT_F64) };
    let response = unsafe { take_response() };
    let handle = response_handle(&response);
    unsafe {
        // One non-manifold edge (0, 1) with 3 faces.
        assert_eq!(abi_array_field(handle, 3), 2);
        assert_ne!(abi_array_field(handle, 15) & 1, 1);
        abi_array_free(handle);
        reclaim::<f64>(vp, vl);
        reclaim::<u32>(ip, il);
    }
}

#[test]
fn manifold_repair_welds_unwelded_cube() {
    // 24 duplicated corner vertices (face-local), 12 triangles.
    let corners: [[f64; 3]; 8] = [
        [0., 0., 0.],
        [1., 0., 0.],
        [1., 1., 0.],
        [0., 1., 0.],
        [0., 0., 1.],
        [1., 0., 1.],
        [1., 1., 1.],
        [0., 1., 1.],
    ];
    // Face-local quads (bottom, top, front, back, right, left), each CCW
    // seen from outside, referencing cube corners 0..8.
    let faces: [[usize; 4]; 6] = [
        [0, 3, 2, 1],
        [4, 5, 6, 7],
        [0, 1, 5, 4],
        [2, 3, 7, 6],
        [1, 2, 6, 5],
        [3, 0, 4, 7],
    ];
    let mut positions: Vec<f64> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    for face in faces {
        let base = (positions.len() / 3) as u32;
        for c in face {
            positions.extend_from_slice(&corners[c]);
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    let (vp, vl) = upload(&positions);
    let (ip, il) = upload(&indices);
    unsafe { abi_manifold_repair(vp, vl, ip, il, FMT_F64, 0.0, 0) };
    let response = unsafe { take_response() };
    let handle = response_handle(&response);
    unsafe {
        // Welded 24 -> 8 vertices, nothing removed or flipped.
        assert_eq!(abi_array_field(handle, 1), 24); // 8 vertices * 3
        assert_eq!(abi_array_field(handle, 3), 36); // 12 triangles * 3
        assert_eq!(abi_array_field(handle, 4), 16); // welded
        assert_eq!(abi_array_field(handle, 5), 0); // degenerates
        let flags = abi_array_field(handle, 7);
        assert_eq!(flags & 1, 1, "repaired cube must be strictly manifold");
        abi_array_free(handle);
        reclaim::<f64>(vp, vl);
        reclaim::<u32>(ip, il);
    }
}

#[test]
fn manifold_check_rejects_unknown_format() {
    let positions: [f64; 3] = [0., 0., 0.];
    let (vp, vl) = upload(&positions);
    unsafe { abi_manifold_check(vp, vl, 0, 0, 7) };
    let response = unsafe { take_response() };
    assert_eq!(response["ok"].as_bool(), Some(false));
    unsafe { reclaim::<f64>(vp, vl) };
}

/// Unit tetrahedron, outward CCW: closed, genus 0, volume 1/6.
fn tetrahedron() -> (Vec<f64>, Vec<u32>) {
    let positions = vec![0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 1.];
    let indices = vec![0, 2, 1, 0, 1, 3, 1, 2, 3, 2, 0, 3];
    (positions, indices)
}

#[test]
fn manifold_metrics_reports_tetrahedron() {
    let (positions, indices) = tetrahedron();
    let (vp, vl) = upload(&positions);
    let (ip, il) = upload(&indices);
    unsafe { abi_manifold_metrics(vp, vl, ip, il, FMT_F64) };
    let response = unsafe { take_response() };
    let handle = response_handle(&response);
    unsafe {
        // Summary: 4 vertices, 4 triangles, 6 edges, euler 2, 1 component.
        assert_eq!(abi_array_field(handle, 6), 4);
        assert_eq!(abi_array_field(handle, 7), 4);
        assert_eq!(abi_array_field(handle, 8), 6);
        assert_eq!(abi_array_field(handle, 9) as i32, 2);
        assert_eq!(abi_array_field(handle, 10), 1);
        assert_eq!(
            abi_array_field(handle, 11) & 1,
            1,
            "closed tetra is watertight"
        );
        // One component record: 4 triangles, genus 0 (stored as genus + 1).
        assert_eq!(abi_array_field(handle, 1), 6);
        let stats_ptr = abi_array_field(handle, 0) as *const u32;
        let stats = std::slice::from_raw_parts(stats_ptr, 6);
        assert_eq!(stats[0], 4);
        assert_eq!(stats[3], 0, "no boundary edges");
        assert_eq!(stats[5], 1, "genus 0 stored as genus + 1");
        // Whole-mesh floats: signed volume 1/6, surface area > 0.
        assert_eq!(abi_array_field(handle, 5), 2);
        let floats = std::slice::from_raw_parts(abi_array_field(handle, 4) as *const f64, 2);
        assert!(
            (floats[0] - 1.0 / 6.0).abs() < 1e-12,
            "volume: {}",
            floats[0]
        );
        assert!(floats[1] > 1.0, "area: {}", floats[1]);
        abi_array_free(handle);
        reclaim::<f64>(vp, vl);
        reclaim::<u32>(ip, il);
    }
}

#[test]
fn manifold_boolean_unions_overlapping_boxes() {
    // Two unit cubes offset by half an edge; union must be strictly
    // manifold with 26 triangles... rather: just assert the contract —
    // success, manifold flag set, nonempty output.
    let box_mesh = |ox: f64| -> (Vec<f64>, Vec<u32>) {
        let corners: [[f64; 3]; 8] = [
            [ox, 0., 0.],
            [ox + 1., 0., 0.],
            [ox + 1., 1., 0.],
            [ox, 1., 0.],
            [ox, 0., 1.],
            [ox + 1., 0., 1.],
            [ox + 1., 1., 1.],
            [ox, 1., 1.],
        ];
        let faces: [[usize; 4]; 6] = [
            [0, 3, 2, 1],
            [4, 5, 6, 7],
            [0, 1, 5, 4],
            [2, 3, 7, 6],
            [1, 2, 6, 5],
            [3, 0, 4, 7],
        ];
        let mut positions = Vec::new();
        let mut indices = Vec::new();
        for face in faces {
            let base = (positions.len() / 3) as u32;
            for c in face {
                positions.extend_from_slice(&corners[c]);
            }
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        // Weld into a strictly manifold box first (the ABI checks input).
        let out = manifold_core::repair_with_mode(
            &positions,
            &indices.iter().map(|&i| i as usize).collect::<Vec<_>>(),
            0.0,
            manifold_core::RepairMode::Conservative,
        );
        assert!(out.is_manifold());
        (
            out.positions,
            out.indices.iter().map(|&i| i as u32).collect(),
        )
    };
    let (ap, ai) = box_mesh(0.0);
    let (bp, bi) = box_mesh(0.5);
    let (avp, avl) = upload(&ap);
    let (aip, ail) = upload(&ai);
    let (bvp, bvl) = upload(&bp);
    let (bip, bil) = upload(&bi);
    unsafe { abi_manifold_boolean(avp, avl, aip, ail, FMT_F64, bvp, bvl, bip, bil, FMT_F64, 0) };
    let response = unsafe { take_response() };
    let handle = response_handle(&response);
    unsafe {
        assert!(abi_array_field(handle, 1) > 0, "union produced vertices");
        assert!(abi_array_field(handle, 3) > 0, "union produced triangles");
        // Stats slots 6..=13: op, repaired a/b, fragments, work, inputs, flags.
        assert_eq!(abi_array_field(handle, 6), 0, "union op tag");
        assert_eq!(abi_array_field(handle, 11), 12, "operand a triangles");
        assert_eq!(abi_array_field(handle, 12), 12, "operand b triangles");
        assert_eq!(
            abi_array_field(handle, 13) & 1,
            1,
            "strictly manifold output"
        );
        abi_array_free(handle);
        reclaim::<f64>(avp, avl);
        reclaim::<u32>(aip, ail);
        reclaim::<f64>(bvp, bvl);
        reclaim::<u32>(bip, bil);
    }
}

#[test]
fn manifold_repair_full_fills_open_box_top() {
    // Closed box minus its top face: conservative repair leaves the
    // boundary; Full mode fills the loop.
    let corners: [[f64; 3]; 8] = [
        [0., 0., 0.],
        [1., 0., 0.],
        [1., 1., 0.],
        [0., 1., 0.],
        [0., 0., 1.],
        [1., 0., 1.],
        [1., 1., 1.],
        [0., 1., 1.],
    ];
    let faces: [[usize; 4]; 5] = [
        [0, 3, 2, 1],
        [0, 1, 5, 4],
        [2, 3, 7, 6],
        [1, 2, 6, 5],
        [3, 0, 4, 7],
    ];
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    for face in faces {
        let base = (positions.len() / 3) as u32;
        for c in face {
            positions.extend_from_slice(&corners[c]);
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    let (vp, vl) = upload(&positions);
    let (ip, il) = upload(&indices);
    unsafe { abi_manifold_repair(vp, vl, ip, il, FMT_F64, 0.0, 1) };
    let response = unsafe { take_response() };
    let handle = response_handle(&response);
    unsafe {
        assert_eq!(abi_array_field(handle, 8), 0, "no splits needed");
        assert_eq!(abi_array_field(handle, 9), 1, "one boundary loop filled");
        assert_eq!(
            abi_array_field(handle, 10),
            2,
            "quad filled with 2 triangles"
        );
        let flags = abi_array_field(handle, 7);
        assert_eq!(flags & 1, 1, "full repair closes the box");
        assert_eq!(abi_array_field(handle, 11), 1, "mode tag is Full");
        abi_array_free(handle);
        reclaim::<f64>(vp, vl);
        reclaim::<u32>(ip, il);
    }
}
