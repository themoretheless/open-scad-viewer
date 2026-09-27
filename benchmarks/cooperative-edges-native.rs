//! Standalone differential and diagnostic timing, no external dependencies.
//! rustc --edition 2024 -O benchmarks/cooperative-edges-native.rs -o /tmp/edges-parity
#[path = "../crates/polygon-core/src/solid/edges.rs"]
mod edges;
use std::{
    future::Future,
    task::{Context, Poll, Waker},
    time::Instant,
};
fn main() {
    let mut seed = 7u32;
    let mut cases = 0;
    let mut max_polls = 0;
    let mut largest = (0., 0.);
    for count in [0, 1, 3, 100, 1_000, 21_845, 21_846, 30_000, 750_000] {
        let mut vertices = Vec::with_capacity(count * 18);
        let mut indices = Vec::with_capacity(count * 3);
        for i in 0..count {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let x = (seed % 1000) as f32;
            let y = (i % 331) as f32;
            let z = (i % 7) as f32;
            for p in [[x, y, z], [x + 1., y, z], [x, y + 1., z]] {
                vertices.extend_from_slice(&[p[0], p[1], p[2], 0., 0., 1.]);
                indices.push(indices.len() as u32);
            }
        }
        if count > 3 {
            vertices[0] = f32::NAN;
            vertices[18] = -0.;
            indices[6] = indices[7];
        }
        for weld in [false, true] {
            let start = Instant::now();
            let expected = edges::extract_semantic_edges(&vertices, &indices, &[], &[], weld, 0.8);
            let sync_ms = start.elapsed().as_secs_f64() * 1000.;
            let start = Instant::now();
            let mut future = std::pin::pin!(edges::extract_semantic_edges_cooperative(
                &vertices,
                &indices,
                &[],
                &[],
                weld,
                0.8
            ));
            let mut cx = Context::from_waker(Waker::noop());
            let mut polls = 0;
            loop {
                polls += 1;
                match future.as_mut().poll(&mut cx) {
                    Poll::Pending => (),
                    Poll::Ready(actual) => {
                        assert_eq!(
                            actual.indices, expected.indices,
                            "count={count}, weld={weld}"
                        );
                        assert_eq!(actual.diagnostics, expected.diagnostics);
                        break;
                    }
                }
            }
            let coop_ms = start.elapsed().as_secs_f64() * 1000.;
            cases += 1;
            max_polls = max_polls.max(polls);
            if count == 750_000 && !weld {
                largest = (sync_ms, coop_ms);
            }
        }
    }
    println!(
        "{{\"cases\":{cases},\"byteParity\":true,\"largestTriangles\":750000,\"maxPolls\":{max_polls},\"largestNoWeldSingleSampleMs\":{{\"sync\":{},\"cooperative\":{}}}}}",
        largest.0, largest.1
    );
}
