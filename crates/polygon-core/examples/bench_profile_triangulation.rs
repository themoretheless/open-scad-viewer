//! Native profile triangulation baseline, including explicit refusal samples.
//! Run --release; validation and input construction are outside measured time.
#[path = "../../planar-geometry/tests/support/profile.rs"]
mod profile;
use planar_geometry::rings::{area, planar};
use planar_geometry::triangulation::triangulate_profile;
use polygon_core::solid::{
    primitives::{cube, cylinder, join},
    section::slice,
};
use std::{hint::black_box, time::Instant};
use value_codec::json;

fn main() {
    let mut samples = vec![];
    let mut fixtures = vec![];
    for (kind, side, segments) in [
        ("synthetic", 1, 32),
        ("synthetic", 2, 32),
        ("synthetic", 4, 4),
        ("synthetic", 4, 32),
        ("synthetic", 7, 32),
        ("synthetic", 8, 32),
        ("synthetic", 10, 12),
        ("synthetic", 10, 32),
        ("prism", 4, 32),
        ("prism", 6, 32),
        ("prism", 8, 32),
        ("prism", 10, 32),
    ] {
        let (outer, holes) = if kind == "prism" {
            prism_profile(side)
        } else {
            profile::grid(side, segments)
        };
        fixtures
            .push(json!({"kind":kind,"side":side,"segments":segments,"outer":outer,"holes":holes}));
        for iteration in 0..12 {
            let start = Instant::now();
            let result = triangulate_profile(black_box(&outer), black_box(&holes));
            let wall_ms = start.elapsed().as_secs_f64() * 1000.;
            let (status, error, triangles) = match &result {
                Ok(mesh) => {
                    profile::validate(mesh, &outer, &holes);
                    ("ok", "", mesh.indices.len() / 3)
                }
                Err(error) => ("refused", error.message.as_str(), 0),
            };
            if iteration >= 3 {
                samples.push(json!({"kind":kind,"side": side, "segments": segments, "vertices": outer.len() + holes.iter().map(Vec::len).sum::<usize>(), "iteration": iteration - 3, "wallMs": wall_ms, "status": status, "error": error, "triangles": triangles}));
            }
        }
    }
    println!(
        "{}",
        json!({"schema": 1, "warmups": 3, "fixtures": fixtures,"samples": samples})
    );
}

fn prism_profile(side: usize) -> (Vec<[f64; 2]>, Vec<Vec<[f64; 2]>>) {
    let plate = cube([86., 86., 8.], true).unwrap();
    let hole = cylinder(12., 3., 3., 32, true).unwrap();
    let step = 80. / side as f64;
    let holes: Vec<_> = (0..side * side)
        .map(|i| {
            let (x, y) = (
                -40. + step / 2. + (i % side) as f64 * step,
                -40. + step / 2. + (i / side) as f64 * step,
            );
            hole.transform([
                [1., 0., 0., x],
                [0., 1., 0., y],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ])
            .unwrap()
        })
        .collect();
    let rings = planar(
        &slice(&plate, 0.).unwrap(),
        &slice(&join(&holes).unwrap(), 0.).unwrap(),
        "difference",
    )
    .unwrap();
    let outer = rings.iter().find(|r| area(r) > 0.).unwrap().clone();
    let holes = rings.into_iter().filter(|r| area(r) < 0.).collect();
    (outer, holes)
}
