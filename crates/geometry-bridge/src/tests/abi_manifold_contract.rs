//! Manifold report schema, ABI version and f32/f64 transport edge cases.
use super::*;

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

unsafe fn take_response() -> Value {
    let ptr = abi_response_ptr();
    let len = abi_response_len();
    let bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, len) }.to_vec();
    unsafe { abi_free(ptr, len) };
    value_codec::decode_binary(&bytes).unwrap()
}

#[test]
fn manifold_report_schema_matches_typescript_schema_file() {
    let file: Value = value_codec::from_str(include_str!(
        "../../../../src/kernel/geometry/manifoldReportSchema.json"
    ))
    .unwrap();
    let schema = MANIFOLD_REPORT_SCHEMA;
    assert_eq!(
        file["schemaVersion"].as_u64(),
        Some(schema.schema_version as u64)
    );
    assert_eq!(file["abiVersion"].as_u64(), Some(schema.abi_version as u64));
    assert_eq!(abi_manifold_abi_version(), schema.abi_version);
    assert_eq!(schema.schema_version, manifold_core::REPORT_SCHEMA_VERSION);
    let reports = file["reports"].as_object().unwrap();
    assert_eq!(reports.len(), schema.reports.len(), "report kinds differ");
    for (kind, fields) in schema.reports {
        let file_fields: Vec<&str> = reports
            .get(*kind)
            .unwrap_or_else(|| panic!("schema file lacks report {kind}"))
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(&file_fields, fields, "report {kind} fields differ");
    }
}

/// Closed tetrahedron scaled by `scale` and shifted by `offset`.
fn tetra(scale: f64, offset: f64) -> (Vec<f64>, [u32; 12]) {
    let base = [0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 1.];
    let positions = base.iter().map(|v| v * scale + offset).collect();
    (positions, [0, 2, 1, 0, 1, 3, 0, 3, 2, 1, 2, 3])
}

/// Run abi_manifold_check with `positions` encoded as `fmt`; returns the
/// decoded response and, on success, the summary slots (handle freed).
fn check_as(fmt: u32, positions: &[f64], indices: &[u32]) -> (Value, Option<[usize; 4]>) {
    let (ip, il) = upload(indices);
    let (vp, vl) = if fmt == FMT_F32 {
        upload(&positions.iter().map(|&v| v as f32).collect::<Vec<_>>())
    } else {
        upload(positions)
    };
    unsafe { abi_manifold_check(vp, vl, ip, il, fmt) };
    let response = unsafe { take_response() };
    let summary = (response["ok"].as_bool() == Some(true)).then(|| {
        let handle = response["value"].as_u64().unwrap() as usize;
        unsafe {
            let s = [12, 13, 14, 15].map(|slot| abi_array_field(handle, slot));
            abi_array_free(handle);
            s
        }
    });
    unsafe {
        if fmt == FMT_F32 {
            reclaim::<f32>(vp, vl)
        } else {
            reclaim::<f64>(vp, vl)
        }
        reclaim::<u32>(ip, il);
    }
    (response, summary)
}

/// 4 vertices, 4 triangles, 1 component, strictly manifold (flags 0b11).
const CLOSED_TETRA: Option<[usize; 4]> = Some([4, 4, 1, 3]);

#[test]
fn manifold_transport_f32_f64_edge_coordinates_have_identical_semantics() {
    let cases: [(&str, f64, f64); 6] = [
        ("unit", 1.0, 0.0),
        ("negative zero offset", 1.0, -0.0),
        ("large f32-representable", 1.0e30, 1.0e30),
        ("small normal", 1.0e-30, 0.0),
        ("f32 subnormal", 1.0e-40, 0.0),
        ("offset far from origin", 1.0, 1.0e7),
    ];
    for (name, scale, offset) in cases {
        let (positions, indices) = tetra(scale, offset);
        for fmt in [FMT_F32, FMT_F64] {
            let (response, summary) = check_as(fmt, &positions, &indices);
            assert_eq!(summary, CLOSED_TETRA, "{name} fmt={fmt}: {response:?}");
        }
    }
    // Explicit -0.0 in every zero slot is finite and accepted.
    let positions = [
        -0.0, -0.0, -0.0, 1., -0.0, -0.0, -0.0, 1., -0.0, -0.0, -0.0, 1.,
    ];
    let indices = [0, 2, 1, 0, 1, 3, 0, 3, 2, 1, 2, 3];
    for fmt in [FMT_F32, FMT_F64] {
        assert_eq!(
            check_as(fmt, &positions, &indices).1,
            CLOSED_TETRA,
            "-0.0 fmt={fmt}"
        );
    }
    // f64-only extremes (outside f32 range) analyse on the f64 path.
    for scale in [1.0e300, f64::MIN_POSITIVE, 5.0e-324] {
        let (positions, indices) = tetra(scale, 0.0);
        assert_eq!(
            check_as(FMT_F64, &positions, &indices).1,
            CLOSED_TETRA,
            "f64 scale {scale}"
        );
    }
    // A value beyond f32 range becomes infinity in f32 transport and is refused.
    let (positions, indices) = tetra(1.0e300, 0.0);
    let (response, summary) = check_as(FMT_F32, &positions, &indices);
    assert_eq!(summary, None);
    assert_eq!(response["ok"].as_bool(), Some(false));
}

#[test]
fn manifold_transport_refuses_nan_and_infinity_in_both_formats() {
    let indices = [0u32, 2, 1, 0, 1, 3, 0, 3, 2, 1, 2, 3];
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for slot in [0, 5, 11] {
            let (mut positions, _) = tetra(1.0, 0.0);
            positions[slot] = bad;
            for fmt in [FMT_F32, FMT_F64] {
                let (response, summary) = check_as(fmt, &positions, &indices);
                assert_eq!(summary, None, "{bad} slot {slot} fmt={fmt}");
                let text = format!("{response:?}");
                assert!(text.contains("GEOMETRY_INVALID_INPUT"), "{text}");
            }
            // Repair and metrics share the same transport validation.
            let (vp, vl) = upload(&positions);
            let (ip, il) = upload(&indices);
            unsafe { abi_manifold_metrics(vp, vl, ip, il, FMT_F64) };
            assert_eq!(unsafe { take_response() }["ok"].as_bool(), Some(false));
            unsafe { abi_manifold_repair(vp, vl, ip, il, FMT_F64, 0.0, 0) };
            assert_eq!(unsafe { take_response() }["ok"].as_bool(), Some(false));
            unsafe {
                reclaim::<f64>(vp, vl);
                reclaim::<u32>(ip, il);
            }
        }
    }
    // An unknown transport format is refused rather than reinterpreted.
    let (positions, _) = tetra(1.0, 0.0);
    let (response, _) = check_as(7, &positions, &indices);
    assert_eq!(response["ok"].as_bool(), Some(false));
}

/// Run abi_manifold_diagnostics on f64 positions; returns the response.
fn diagnostics(positions: &[f64], indices: &[u32], density: f64, max_work: u32) -> Value {
    let (vp, vl) = upload(positions);
    let (ip, il) = upload(indices);
    unsafe { abi_manifold_diagnostics(vp, vl, ip, il, FMT_F64, density, 0.6, 0.5, max_work) };
    let response = unsafe { take_response() };
    unsafe {
        reclaim::<f64>(vp, vl);
        reclaim::<u32>(ip, il);
    }
    response
}

unsafe fn slice<T: Copy>(handle: usize, slot: u32) -> Vec<T> {
    let (ptr, len) = unsafe {
        (
            abi_array_field(handle, slot),
            abi_array_field(handle, slot + 1),
        )
    };
    if len == 0 {
        return Vec::new();
    }
    unsafe { std::slice::from_raw_parts(ptr as *const T, len) }.to_vec()
}

#[test]
fn manifold_diagnostics_reports_intersections_thickness_clearance_and_mass() {
    // Two overlapping unit tetrahedra and a third one 1.2 beyond the second.
    let (a, ia) = tetra(1.0, 0.0);
    let (b, _) = tetra(1.0, 0.25);
    let far: Vec<f64> = a
        .chunks(3)
        .flat_map(|p| [p[0] + 2.45, p[1], p[2]])
        .collect();
    let mut positions = a.clone();
    positions.extend(&b);
    positions.extend(&far);
    let mut indices: Vec<u32> = ia.to_vec();
    indices.extend(ia.iter().map(|i| i + 4));
    indices.extend(ia.iter().map(|i| i + 8));
    let response = diagnostics(&positions, &indices, 2.0, 0);
    let handle = response["value"].as_u64().expect("diagnostics ok") as usize;
    unsafe {
        let pairs: Vec<u32> = slice(handle, 0);
        assert!(!pairs.is_empty());
        assert!(pairs.chunks(2).all(|p| p[0] < 4 && (4..8).contains(&p[1])));
        let clearance: Vec<u32> = slice(handle, 4);
        let floats: Vec<f64> = slice(handle, 6);
        // Shells 0 and 1 overlap (distance 0); shell 2 is farther than 0.5.
        assert_eq!(clearance.len(), 4);
        assert_eq!(&clearance[..2], &[0, 1]);
        assert_eq!(floats.len(), 17 + 1);
        assert_eq!(floats[17], 0.0);
        // The far tetrahedron's faces see the second one across the gap.
        assert!(floats[2] > 0.0 && abi_array_field(handle, 14) > 0);
        assert!(floats[0] > 0.0 && floats[0] <= floats[1]);
        // Three closed tetrahedra: volume 3/6, mass 2 * 0.5.
        assert_eq!(abi_array_field(handle, 9) & 2, 2);
        assert!((floats[3] - 0.5).abs() < 1e-12 && (floats[4] - 1.0).abs() < 1e-12);
        assert_eq!(abi_array_field(handle, 10), 12, "one ray per triangle");
        assert_eq!(abi_array_field(handle, 12), 3, "three shells");
        abi_array_free(handle);
    }

    // Open mesh: no mass (flag clear, NaN floats), still a valid report.
    let response = diagnostics(&a, &ia[..9], 1.0, 0);
    let handle = response["value"].as_u64().unwrap() as usize;
    unsafe {
        assert_eq!(abi_array_field(handle, 9) & 2, 0);
        let floats: Vec<f64> = slice(handle, 6);
        assert!(floats[3].is_nan());
        abi_array_free(handle);
    }

    // Budget and parameter failures are typed.
    let over = diagnostics(&positions, &indices, 1.0, 1);
    assert_eq!(
        over["error"]["code"].as_str(),
        Some("GEOMETRY_RESOURCE_LIMIT"),
        "{over:?}"
    );
    let bad = diagnostics(&a, &ia, -1.0, 0);
    assert_eq!(
        bad["error"]["code"].as_str(),
        Some("GEOMETRY_INVALID_INPUT"),
        "{bad:?}"
    );
}
