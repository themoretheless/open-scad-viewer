//! Paired native emit-and-hash benchmark; no printer or browser I/O.
use gcode_core::{MachineProfile, PlannedLayer, PlannedPath, emit, emit_to};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    fmt::{self, Write},
    hint::black_box,
    sync::atomic::{AtomicUsize, Ordering},
    time::Instant,
};

struct Allocator;
// Compile-time switch: release timing builds contain no allocation counter increments.
const COUNT_ALLOCATIONS: bool = option_env!("OSV_WRITER_COUNT_ALLOCATIONS").is_some();
static BYTES: AtomicUsize = AtomicUsize::new(0);
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNT_ALLOCATIONS {
            BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        if COUNT_ALLOCATIONS {
            BYTES.fetch_add(size, Ordering::Relaxed);
        }
        unsafe { System.realloc(ptr, layout, size) }
    }
}

struct Digest {
    hash: u64,
    bytes: usize,
}
impl Default for Digest {
    fn default() -> Self {
        Self {
            hash: 14695981039346656037,
            bytes: 0,
        }
    }
}
impl Write for Digest {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        for byte in text.bytes() {
            self.hash = (self.hash ^ u64::from(byte)).wrapping_mul(1099511628211);
        }
        self.bytes += text.len();
        Ok(())
    }
}
fn run(layers: &[PlannedLayer], direct: bool) -> (u64, usize) {
    let mut sink = Digest::default();
    if direct {
        emit_to(layers, &MachineProfile::default(), &mut sink).unwrap();
    } else {
        sink.write_str(&emit(layers, &MachineProfile::default()).unwrap())
            .unwrap();
    }
    black_box((sink.hash, sink.bytes))
}
fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}
fn main() {
    for count in [100, 10_000, 60_000] {
        let layers = [PlannedLayer {
            z_mm: 0.2,
            paths: vec![PlannedPath {
                points: (0..count).map(|i| [(i % 2) as f64 * 10.0, 0.0]).collect(),
                closed: false,
            }],
        }];
        // Exact byte comparison outside measured regions, not only a digest comparison.
        let expected_text = emit(&layers, &MachineProfile::default()).unwrap();
        let mut actual_text = String::new();
        emit_to(&layers, &MachineProfile::default(), &mut actual_text).unwrap();
        assert_eq!(actual_text, expected_text);
        drop((actual_text, expected_text));
        let expected = run(&layers, false);
        for _ in 0..5 {
            assert_eq!(run(&layers, false), expected);
            assert_eq!(run(&layers, true), expected);
        }
        let mut string_ms = Vec::with_capacity(31);
        let mut writer_ms = Vec::with_capacity(31);
        let mut allocation = [0; 2];
        for sample in 0..31 {
            for index in if sample % 2 == 0 { [0, 1] } else { [1, 0] } {
                if COUNT_ALLOCATIONS {
                    BYTES.store(0, Ordering::Relaxed);
                }
                let start = Instant::now();
                let result = run(&layers, index == 1);
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                if COUNT_ALLOCATIONS {
                    allocation[index] = BYTES.load(Ordering::Relaxed);
                }
                assert_eq!(result, expected);
                if index == 0 {
                    string_ms.push(elapsed);
                } else {
                    writer_ms.push(elapsed);
                }
            }
        }
        println!(
            "{{\"allocationInstrumentation\":{COUNT_ALLOCATIONS},\"points\":{count},\"outputBytes\":{},\"stringMedianMs\":{},\"writerMedianMs\":{},\"stringAllocatedBytes\":{},\"writerAllocatedBytes\":{}}}",
            expected.1,
            median(&mut string_ms),
            median(&mut writer_ms),
            if COUNT_ALLOCATIONS {
                allocation[0].to_string()
            } else {
                "null".into()
            },
            if COUNT_ALLOCATIONS {
                allocation[1].to_string()
            } else {
                "null".into()
            }
        );
    }
}
