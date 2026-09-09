use super::*;
use crate::{response_bytes, response_value};
use photogrammetry_kernel::{camera::Camera, Point};

fn generic_surface(mesh: &Surface, diagnostics: Option<&Value>) -> Vec<u8> {
    let value = if let Some(diagnostics) = diagnostics {
        json!({"positions": mesh.positions, "colors": mesh.colors,
            "triangles": mesh.triangles, "denseDiagnostics": diagnostics})
    } else {
        json!({"positions": mesh.positions, "colors": mesh.colors, "triangles": mesh.triangles})
    };
    response_bytes(Ok(value))
}

#[test]
fn borrowed_surface_preserves_every_wire_byte_and_decoded_value() {
    let mesh = Surface {
        positions: vec![
            [0., -0., f64::MIN_POSITIVE],
            [f64::MAX, -1.23, 42.],
            [f64::NAN, f64::INFINITY, f64::NEG_INFINITY],
        ],
        colors: vec![[0, 128, 255], [1, 2, 3], [253, 254, 255]],
        triangles: vec![[0, 1, 2], [u32::MAX, 0, 128]],
    };
    let diagnostics = json!({"source": "Измерения", "counter": 4_294_967_307u64,
        "viewReports": [{"image": 0, "sourceImages": [2, 4]}], "empty": null});
    for mesh in [&mesh, &Surface::default()] {
        for diagnostics in [None, Some(&diagnostics)] {
            let actual = surface(mesh, diagnostics).unwrap();
            let expected = generic_surface(mesh, diagnostics);
            assert_eq!(actual, expected);
            assert_eq!(
                value_codec::decode_binary(&actual).unwrap(),
                value_codec::decode_binary(&expected).unwrap()
            );
        }
    }
}

#[test]
fn borrowed_sparse_preserves_missing_cameras_point_order_and_metadata() {
    let reconstruction = Reconstruction {
        cameras: vec![
            Some(Camera::identity(700., 320., 240.)),
            None,
            Some(Camera::identity(900., 160., 120.)),
        ],
        points: vec![Point {
            position: [-0., 2., -3.],
            color: [200, 10, 0],
            observations: vec![(0, 4), (2, 9)],
        }],
        input_images: 3,
        reprojection_rmse: 0.312345,
    };
    let diagnostics =
        json!({"calibrations": [{"mode": "measured-brown"}], "warnings": ["Нет опоры"]});
    let cameras =
        reconstruction
            .cameras
            .iter()
            .enumerate()
            .filter_map(|(image, camera)| {
                camera.as_ref().map(|camera| json!({
            "image": image, "rotation": camera.rotation, "translation": camera.translation,
            "focal": camera.focal, "cx": camera.cx, "cy": camera.cy,
        }))
            })
            .collect::<Vec<_>>();
    let expected = response_bytes(Ok(json!({
        "positions": reconstruction.points.iter().map(|p| p.position).collect::<Vec<_>>(),
        "colors": reconstruction.points.iter().map(|p| p.color).collect::<Vec<_>>(),
        "triangles": Vec::<[u32; 3]>::new(), "cameras": cameras,
        "inputImages": reconstruction.input_images, "reprojectionRmse": reconstruction.reprojection_rmse,
        "diagnostics": diagnostics,
    })));
    assert_eq!(sparse(&reconstruction, &diagnostics).unwrap(), expected);
}

#[test]
fn borrowed_metadata_uses_the_same_complete_envelope_limits() {
    let metadata = json!({"boolean": false, "text": "\"\\\n測定", "integer": -42i64,
        "array": Value::Array(vec![Value::Null, json!(1.), json!(true)])});
    assert_eq!(value(&metadata).unwrap(), response_bytes(Ok(metadata)));
    let mut nested = Value::Null;
    for _ in 0..DEPTH_LIMIT - 1 {
        nested = Value::Array(vec![nested]);
    }
    assert!(value(&nested).is_ok());
    let nested = Value::Array(vec![nested]);
    assert_eq!(value(&nested).unwrap_err(), TRANSPORT_ERROR);
    assert_eq!(
        value_codec::decode_binary(&response_bytes(Ok(nested))).unwrap()["ok"],
        json!(false)
    );
}

#[test]
fn geometry_budget_counts_keys_arrays_numbers_and_diagnostics_exactly() {
    let fields = [(
        "positions",
        Field::Positions(&[[0., f64::NAN, 2.], [3., 4., 5.]]),
    )];
    let object = Field::Object(&fields);
    let mut optimized = ResponseBudget { bytes: 4, items: 0 };
    object.measure(&mut optimized, 0).unwrap();
    let mut reference = ResponseBudget { bytes: 4, items: 0 };
    reference
        .visit(&json!({"positions": [[0., f64::NAN, 2.], [3., 4., 5.]]}), 0)
        .unwrap();
    assert_eq!(
        (optimized.bytes, optimized.items),
        (reference.bytes, reference.items)
    );
    let mut exact = ResponseBudget {
        bytes: LIMIT - 32,
        items: ITEM_LIMIT - 4,
    };
    reserve_many(&mut exact, DEPTH_LIMIT, 4, 32).unwrap();
    assert_eq!((exact.bytes, exact.items), (LIMIT, ITEM_LIMIT));
    assert!(reserve_many(&mut exact, 0, 1, 0).is_err());
    assert!(reserve_many(&mut exact, 0, 0, 1).is_err());
    assert!(reserve_many(&mut ResponseBudget { bytes: 4, items: 0 }, 0, usize::MAX, 0).is_err());
    assert!(triples_budget(&mut ResponseBudget { bytes: 4, items: 0 }, 0, usize::MAX, 0).is_err());
}

#[test]
fn oversized_geometry_is_rejected_before_allocating_an_intermediate_value_tree() {
    // Wire bytes fit 32 MiB, but four million numeric/array items plus all keys do not.
    let mesh = Surface {
        positions: vec![[0.; 3]; 500_000],
        colors: vec![[0; 3]; 500_000],
        triangles: vec![],
    };
    assert_eq!(surface(&mesh, None).unwrap_err(), TRANSPORT_ERROR);
    let envelope = response_value(Err(surface(&mesh, None).unwrap_err()));
    assert_eq!(envelope["message"], json!(TRANSPORT_ERROR));
}

// The ignored benchmark runs alone in a fresh process (one test thread). Tracking
// is enabled only around serialization; process peak RSS is measured externally.
mod allocation {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};
    pub struct Tracked;
    static ENABLED: AtomicBool = AtomicBool::new(false);
    static COUNT: AtomicUsize = AtomicUsize::new(0);
    static BYTES: AtomicUsize = AtomicUsize::new(0);
    static LIVE: AtomicUsize = AtomicUsize::new(0);
    static PEAK: AtomicUsize = AtomicUsize::new(0);
    fn added(bytes: usize) {
        COUNT.fetch_add(1, Relaxed);
        BYTES.fetch_add(bytes, Relaxed);
        let live = LIVE.fetch_add(bytes, Relaxed) + bytes;
        PEAK.fetch_max(live, Relaxed);
    }
    unsafe impl GlobalAlloc for Tracked {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let ptr = System.alloc(layout);
            if !ptr.is_null() && ENABLED.load(Relaxed) {
                added(layout.size());
            }
            ptr
        }
        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            let ptr = System.alloc_zeroed(layout);
            if !ptr.is_null() && ENABLED.load(Relaxed) {
                added(layout.size());
            }
            ptr
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            if ENABLED.load(Relaxed) {
                LIVE.fetch_sub(layout.size(), Relaxed);
            }
            System.dealloc(ptr, layout);
        }
        unsafe fn realloc(&self, ptr: *mut u8, old: Layout, size: usize) -> *mut u8 {
            let next = System.realloc(ptr, old, size);
            if !next.is_null() && ENABLED.load(Relaxed) {
                LIVE.fetch_sub(old.size(), Relaxed);
                added(size);
            }
            next
        }
    }
    pub fn start() {
        for metric in [&COUNT, &BYTES, &LIVE, &PEAK] {
            metric.store(0, Relaxed);
        }
        ENABLED.store(true, Relaxed);
    }
    pub fn stop() -> [usize; 3] {
        ENABLED.store(false, Relaxed);
        [COUNT.load(Relaxed), BYTES.load(Relaxed), PEAK.load(Relaxed)]
    }
}
#[global_allocator]
static ALLOCATOR: allocation::Tracked = allocation::Tracked;

fn fixture() -> Surface {
    if let Ok(path) = std::env::var("PHOTO_TRANSPORT_FIXTURE") {
        let text = std::fs::read_to_string(path).unwrap();
        let mut lines = text.lines();
        assert_eq!(lines.next(), Some("ply"));
        assert_eq!(lines.next(), Some("format ascii 1.0"));
        let mut vertices = 0;
        let mut faces = 0;
        for line in lines.by_ref() {
            if let Some(count) = line.strip_prefix("element vertex ") {
                vertices = count.parse().unwrap();
            }
            if let Some(count) = line.strip_prefix("element face ") {
                faces = count.parse().unwrap();
            }
            if line == "end_header" {
                break;
            }
        }
        let mut mesh = Surface::default();
        for line in lines.by_ref().take(vertices) {
            let mut fields = line.split_whitespace();
            mesh.positions.push(std::array::from_fn(|_| {
                fields.next().unwrap().parse().unwrap()
            }));
            mesh.colors.push(std::array::from_fn(|_| {
                fields.next().unwrap().parse().unwrap()
            }));
        }
        for line in lines.take(faces) {
            let mut fields = line.split_whitespace();
            assert_eq!(fields.next(), Some("3"));
            mesh.triangles.push(std::array::from_fn(|_| {
                fields.next().unwrap().parse().unwrap()
            }));
        }
        mesh
    } else {
        let vertices = std::env::var("PHOTO_TRANSPORT_VERTICES")
            .ok()
            .map(|v| v.parse().unwrap())
            .unwrap_or(125_000usize);
        Surface {
            positions: (0..vertices)
                .map(|i| {
                    [
                        (i % 1000) as f64 * 0.001,
                        (i / 1000) as f64 * 0.002,
                        (i as f64).sin(),
                    ]
                })
                .collect(),
            colors: (0..vertices)
                .map(|i| [(i % 251) as u8, (i % 199) as u8, 200])
                .collect(),
            triangles: (0..2 * vertices)
                .map(|i| {
                    [
                        (i % vertices) as u32,
                        ((i + 1) % vertices) as u32,
                        ((i + 100) % vertices) as u32,
                    ]
                })
                .collect(),
        }
    }
}

#[test]
#[ignore = "Comparative benchmark; run alone with PHOTO_TRANSPORT_MODE and --test-threads=1"]
fn benchmark_surface_transport() {
    let mesh = fixture();
    let diagnostics = json!({"vertices": mesh.positions.len(), "triangles": mesh.triangles.len(),
        "preset": "baseline", "sampledSourcePixels": 4_294_967_307u64,
        "viewReports": [{"image": 0, "sourceImages": [1, 2, 3]}]});
    let mode = std::env::var("PHOTO_TRANSPORT_MODE").unwrap_or("verify".into());
    if mode == "verify" {
        let direct = surface(&mesh, Some(&diagnostics)).unwrap();
        let baseline = generic_surface(&mesh, Some(&diagnostics));
        assert_eq!(direct, baseline);
        assert_eq!(
            value_codec::decode_binary(&direct).unwrap(),
            value_codec::decode_binary(&baseline).unwrap()
        );
        println!("{{\"mode\":\"verify\",\"vertices\":{},\"faces\":{},\"bytes\":{},\"exactBytes\":true,\"exactDecoded\":true}}", mesh.positions.len(), mesh.triangles.len(), direct.len());
        return;
    }
    assert!(mode == "direct" || mode == "baseline");
    let iterations: usize = std::env::var("PHOTO_TRANSPORT_ITERATIONS")
        .ok()
        .map(|v| v.parse().unwrap())
        .unwrap_or(6);
    for iteration in 0..iterations {
        let tracked = iteration == 0;
        if tracked {
            allocation::start();
        }
        let start = std::time::Instant::now();
        let bytes = if mode == "direct" {
            surface(&mesh, Some(&diagnostics)).unwrap()
        } else {
            generic_surface(&mesh, Some(&diagnostics))
        };
        let elapsed = start.elapsed().as_secs_f64() * 1000.;
        let [allocations, allocated, peak] = if tracked { allocation::stop() } else { [0; 3] };
        println!("{{\"mode\":\"{}\",\"allocationTracking\":{},\"iteration\":{},\"vertices\":{},\"faces\":{},\"bytes\":{},\"elapsedMs\":{:.6},\"allocations\":{},\"allocatedBytes\":{},\"peakLiveBytes\":{}}}", mode, tracked, iteration, mesh.positions.len(), mesh.triangles.len(), bytes.len(), elapsed, allocations, allocated, peak);
        if iteration + 1 == iterations {
            if let Ok(path) = std::env::var("PHOTO_TRANSPORT_OUTPUT") {
                std::fs::write(path, &bytes).unwrap();
            }
        }
        std::hint::black_box(bytes);
    }
}
