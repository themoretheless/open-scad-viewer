//! cargo bench -p mesh-query --bench picking -- model.stl
use mesh_query::{Query, build_mesh_bvh, raycast};
use std::{collections::BTreeSet, hint::black_box, time::Instant};

fn load(path: &str) -> (Vec<f32>, Vec<u32>) {
    let bytes = std::fs::read(path).expect("read STL");
    let mut vertices = Vec::new();
    let binary_count = bytes
        .get(80..84)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize);
    if let Some(count) = binary_count
        .filter(|n| n.checked_mul(50).and_then(|s| s.checked_add(84)) == Some(bytes.len()))
    {
        for record in bytes[84..].as_chunks::<50>().0.iter().take(count) {
            for component in record[12..48].as_chunks::<4>().0 {
                vertices.push(f32::from_le_bytes(*component));
            }
        }
    } else {
        let text = std::str::from_utf8(&bytes).expect("ASCII or binary STL");
        for line in text.lines() {
            let mut words = line.split_whitespace();
            if words.next() == Some("vertex") {
                for _ in 0..3 {
                    vertices.push(
                        words
                            .next()
                            .expect("vertex xyz")
                            .parse()
                            .expect("finite number"),
                    );
                }
            }
        }
    }
    assert!(!vertices.is_empty() && vertices.len().is_multiple_of(9));
    assert!(vertices.iter().all(|v| v.is_finite()));
    let indices = (0..u32::try_from(vertices.len() / 3).expect("u32 vertices")).collect();
    (vertices, indices)
}

fn main() {
    let path = std::env::args().skip(1).find(|arg| !arg.starts_with('-'));
    let (vertices, indices) = if let Some(path) = path.as_deref() {
        load(path)
    } else {
        // Deterministic 128 x 128 triangulated curved height field.
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for y in 0..=128 {
            for x in 0..=128 {
                vertices.extend([
                    x as f32,
                    y as f32,
                    ((x as f32) * 0.1).sin() * ((y as f32) * 0.1).cos(),
                ]);
            }
        }
        for y in 0..128 {
            for x in 0..128 {
                let a = y * 129 + x;
                indices.extend([a, a + 1, a + 129, a + 1, a + 130, a + 129]);
            }
        }
        (vertices, indices)
    };
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for p in vertices.as_chunks::<3>().0 {
        for axis in 0..3 {
            lo[axis] = lo[axis].min(p[axis] as f64);
            hi[axis] = hi[axis].max(p[axis] as f64);
        }
    }
    let start = Instant::now();
    let tree = build_mesh_bvh(black_box(&vertices), black_box(&indices), 3, 8);
    let build_ms = start.elapsed().as_secs_f64() * 1000.;
    let excluded = BTreeSet::new();
    let count = 10_000;
    let mut timings = Vec::with_capacity(count);
    let mut hits = 0;
    for i in 0..count {
        let u = ((i * 7919) % count) as f64 / count as f64;
        let v = ((i * 3571) % count) as f64 / count as f64;
        let query = Query {
            origin: [
                lo[0] + u * (hi[0] - lo[0]),
                lo[1] + v * (hi[1] - lo[1]),
                hi[2] + 1.,
            ],
            direction: [0., 0., -1.],
            min_t: 0.,
            max_t: f64::INFINITY,
            local_from_world: None,
            excluded: &excluded,
        };
        let start = Instant::now();
        hits += usize::from(
            black_box(raycast(&tree, &vertices, &indices, 3, &query).unwrap()).is_some(),
        );
        timings.push(start.elapsed().as_nanos());
    }
    timings.sort_unstable();
    println!(
        "input={} triangles={} build_ms={build_ms:.3} rays={count} hits={hits} p50_ns={} p95_ns={} p99_ns={}",
        path.as_deref().unwrap_or("generated-height-field"),
        indices.len() / 3,
        timings[count / 2],
        timings[count * 95 / 100],
        timings[count * 99 / 100]
    );
}
