//! cargo bench --manifest-path crates/Cargo.toml -p nurbs-core --bench kernels
//!
//! Median wall time of intersection and tessellation kernels on fixed
//! fixtures. Plain `std::time` like `surface_evaluation`: `criterion` is not
//! in the shared lock file.
use nurbs_core::curve::Curve;
use nurbs_core::intersection::intersect_curve_curve_report;
use nurbs_core::ss_intersection::intersect_surface_surface_report;
use nurbs_core::surface::Surface;
use nurbs_core::surface_tessellation::tessellate;
use nurbs_core::tessellation::shading::Orientation;
use nurbs_core::tessellation::watertight::tessellate_lods;
use std::{hint::black_box, time::Instant};

const ROUNDS: usize = 7;

fn median(mut run: impl FnMut()) -> f64 {
    run();
    let mut t: Vec<f64> = (0..ROUNDS)
        .map(|_| {
            let start = Instant::now();
            run();
            start.elapsed().as_secs_f64() * 1e6
        })
        .collect();
    t.sort_by(f64::total_cmp);
    t[ROUNDS / 2]
}

fn wave(phase: f64) -> Curve {
    let control_points = (0..12)
        .map(|i| {
            let x = i as f64 / 11.;
            vec![x, (6. * x + phase).sin() * 0.3, 0.]
        })
        .collect();
    let mut knots = vec![0.; 4];
    knots.extend((1..9).map(|i| i as f64 / 9.));
    knots.extend([1.; 4]);
    Curve {
        degree: 3,
        knots,
        control_points,
        weights: vec![1.; 12],
        periodic: false,
    }
}

fn sheet(tilt: f64) -> Surface {
    let control_points = (0..5)
        .map(|i| {
            (0..5)
                .map(|j| {
                    let (x, y) = (i as f64 / 4., j as f64 / 4.);
                    vec![x, y, 0.2 * (3. * x).sin() * (2. * y).cos() + tilt * (x - 0.5)]
                })
                .collect()
        })
        .collect();
    let knots = vec![0., 0., 0., 0., 0.5, 1., 1., 1., 1.];
    Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: knots.clone(),
        knots_v: knots,
        control_points,
        weights: vec![vec![1.; 5]; 5],
        periodic_u: false,
        periodic_v: false,
    }
}

fn main() {
    let (a, b) = (wave(0.), wave(1.3));
    let cc = median(|| {
        black_box(intersect_curve_curve_report(black_box(&a), black_box(&b), None).unwrap());
    });
    let (s, t) = (sheet(0.), sheet(0.8));
    let ss = median(|| {
        black_box(intersect_surface_surface_report(black_box(&s), black_box(&t), None).unwrap());
    });
    let adaptive = median(|| {
        black_box(tessellate(black_box(&s), 1e-3, 100000).unwrap());
    });
    let lods = median(|| {
        black_box(
            tessellate_lods(&[s.clone()], &[], &[Orientation::Natural], &[0.5, 0.1], 100000, 1000000)
                .unwrap(),
        );
    });
    for (name, micros) in [
        ("curve/curve intersection", cc),
        ("surface/surface intersection", ss),
        ("adaptive tessellation 1e-3", adaptive),
        ("watertight LOD ladder", lods),
    ] {
        println!("{name:32} {micros:>12.1} us");
    }
}
