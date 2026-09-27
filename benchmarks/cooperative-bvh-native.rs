#![allow(dead_code)]
#[path = "../crates/polygon-core/src/solid/bvh.rs"]
mod after;
#[path = "../tests/fixtures/bvh-before-cooperative.rs"]
mod before;
use std::{
    future::Future,
    task::{Context, Poll, Waker},
    time::Instant,
};
fn main() {
    let mut seed = 0x193ab02fu32;
    let mut random = || {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        seed
    };
    let mut max_polls = 0;
    let mut big = (0., 0., 0.);
    for case in 0..122 {
        let n = match case {
            120 => 100_000,
            121 => 750_000,
            _ => case * 31,
        };
        let mut vertices = Vec::with_capacity(n * 18);
        let mut indices = Vec::with_capacity(n * 3);
        for t in 0..n {
            for _ in 0..3 {
                for _ in 0..3 {
                    let x = (random() as i32 % 10000) as f32 / 128.;
                    vertices.push(if case % 7 == 0 { x.round() } else { x });
                }
                vertices.extend_from_slice(&[0., 0., 1.]);
            }
            indices.extend_from_slice(&[(t * 3) as u32, (t * 3 + 1) as u32, (t * 3 + 2) as u32]);
            if case % 3 == 0 && t % 5 == 0 {
                indices[t * 3 + 1] = indices[t * 3];
            }
            if case % 5 == 0 && t % 13 == 0 {
                vertices[t * 18] = f32::NAN;
            }
            if case % 11 == 0 && t % 7 == 0 {
                indices[t * 3] = u32::MAX;
            }
        }
        let leaf = [1, 2, 8, 16, 64][case % 5];
        let t = Instant::now();
        let expected = before::build_mesh_bvh(&vertices, &indices, 6, leaf);
        let old = t.elapsed().as_secs_f64() * 1000.;
        let t = Instant::now();
        let sync = after::build_mesh_bvh(&vertices, &indices, 6, leaf);
        let new = t.elapsed().as_secs_f64() * 1000.;
        let mut cx = Context::from_waker(Waker::noop());
        let mut future = Box::pin(after::build_mesh_bvh_cooperative(
            &vertices, &indices, 6, leaf,
        ));
        let t = Instant::now();
        let mut polls = 0;
        let actual = loop {
            polls += 1;
            assert!(polls < 100_000);
            if let Poll::Ready(x) = future.as_mut().poll(&mut cx) {
                break x;
            }
        };
        let cooperative = t.elapsed().as_secs_f64() * 1000.;
        max_polls = max_polls.max(polls);
        assert_eq!(expected.node_count, actual.node_count, "case {case}");
        assert_eq!(
            expected
                .bounds
                .iter()
                .map(|x| x.to_bits())
                .collect::<Vec<_>>(),
            actual
                .bounds
                .iter()
                .map(|x| x.to_bits())
                .collect::<Vec<_>>(),
            "case {case}"
        );
        assert_eq!(expected.nodes, actual.nodes, "case {case}");
        assert_eq!(expected.triangles, actual.triangles, "case {case}");
        assert_eq!(
            expected
                .bounds
                .iter()
                .map(|x| x.to_bits())
                .collect::<Vec<_>>(),
            sync.bounds.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
            "sync case {case}"
        );
        assert_eq!(expected.nodes, sync.nodes);
        assert_eq!(expected.triangles, sync.triangles);
        if case == 121 {
            big = (old, new, cooperative);
        }
    }
    println!(
        "{{\"cases\":122,\"byteParity\":true,\"maxPolls\":{max_polls},\"largestTriangles\":750000,\"largestSingleSampleMs\":{{\"before\":{},\"sync\":{},\"cooperative\":{}}}}}",
        big.0, big.1, big.2
    );
}
