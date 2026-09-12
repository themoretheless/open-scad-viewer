//! Identical Curvex application-level benchmark for original and migrated builds.
//! Copy as examples/osv_boolean_benchmark.rs in each isolated checkout, then run
//! cargo run --release --example osv_boolean_benchmark. Output is JSON lines.
//! Geometry is built before timing. Each sample includes operation + result drop.

use curvex::boolean_ops::{
    boolean_difference_for_bench, boolean_union_for_bench, divide_paths_pure,
    shape_to_kurbo_path_for_bench,
};
use curvex::document::Document;
use curvex::shape::{Contour, FillRule, PathSegment, Shape, ShapeData, Vec2};
use serde_json::{Value, json};
use std::hint::black_box;
use std::time::Instant;

fn ellipse(x: f32, y: f32, rx: f32, ry: f32) -> Shape {
    Shape::new(
        "ellipse".into(),
        ShapeData::Ellipse {
            center: Vec2::new(x, y),
            radius_x: rx,
            radius_y: ry,
        },
    )
}
fn rectangle(x: f32, y: f32, width: f32, height: f32) -> Shape {
    Shape::new(
        "rectangle".into(),
        ShapeData::Rectangle {
            top_left: Vec2::new(x, y),
            width,
            height,
        },
    )
}
fn contours_summary(contours: &[(Vec2, Vec<PathSegment>)]) -> Value {
    let segments: usize = contours.iter().map(|(_, s)| s.len()).sum();
    let cubics: usize = contours
        .iter()
        .map(|(_, s)| {
            s.iter()
                .filter(|s| matches!(s, PathSegment::Cubic { .. }))
                .count()
        })
        .sum();
    json!({"contours": contours.len(), "segments": segments, "cubics": cubics})
}
fn measure<T>(name: &str, run: impl Fn() -> T, describe: impl Fn(&T) -> Value) {
    if let Some(only) = std::env::args().nth(1) {
        if only != name {
            return;
        }
        let output = run();
        println!("{}", json!({"workload":name,"output":describe(&output)}));
        return;
    }
    let output = run();
    let summary = describe(&output);
    assert!(
        summary
            .get("contours")
            .and_then(Value::as_u64)
            .map_or(true, |n| n > 0),
        "{name} unexpectedly produced an empty region"
    );
    assert!(
        summary
            .get("present")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        "{name} unexpectedly failed normalization"
    );
    drop(output);
    let start = Instant::now();
    for _ in 0..3 {
        black_box(run());
    }
    let per_call = start.elapsed().as_secs_f64() / 3.;
    let iterations = (0.075 / per_call.max(1e-9)).clamp(1., 10_000.) as usize;
    let mut samples = Vec::new();
    for _ in 0..7 {
        let start = Instant::now();
        for _ in 0..iterations {
            black_box(run());
        }
        samples.push(start.elapsed().as_secs_f64() * 1e6 / iterations as f64);
    }
    let mut sorted = samples.clone();
    sorted.sort_by(f64::total_cmp);
    println!(
        "{}",
        json!({
            "workload": name, "iterations_per_sample": iterations,
            "samples_us": samples, "median_us": sorted[3],
            "min_us": sorted[0], "max_us": sorted[6], "output": summary,
        })
    );
}

fn main() {
    let ellipses = [ellipse(0., 0., 60., 40.), ellipse(45., 8., 55., 35.)];
    let pair: Vec<_> = ellipses.iter().collect();
    measure(
        "ellipse_union_2",
        || boolean_union_for_bench(&pair),
        |v| contours_summary(v),
    );

    let chain: Vec<_> = (0..12)
        .map(|i| ellipse(i as f32 * 15., (i as f32 * 0.7).sin() * 8., 25., 18.))
        .collect();
    let chain_refs: Vec<_> = chain.iter().collect();
    measure(
        "ellipse_union_12",
        || boolean_union_for_bench(&chain_refs),
        |v| contours_summary(v),
    );

    let mut rounded = rectangle(0., 0., 100., 80.);
    rounded.corner_radii = vec![12., 20., 8., 15.];
    let cutter = ellipse(85., 40., 35., 25.);
    measure(
        "rounded_rectangle_difference",
        || boolean_difference_for_bench(&[&rounded, &cutter]),
        |v| contours_summary(v),
    );

    let big = rectangle(0., 0., 1000., 1000.);
    let cutters: Vec<_> = (0..50)
        .map(|i| {
            let f = i as f32;
            ellipse(60. + (f * 137.) % 880., 60. + (f * 71.) % 880., 25., 18.)
        })
        .collect();
    let mut subtract_refs = vec![&big];
    subtract_refs.extend(cutters.iter());
    measure(
        "rectangle_minus_50_ellipses",
        || boolean_difference_for_bench(&subtract_refs),
        |v| contours_summary(v),
    );

    let circles = [
        ellipse(0., 0., 20., 20.),
        ellipse(20., 0., 20., 20.),
        ellipse(10., 17., 20., 20.),
    ];
    let paths: Vec<_> = circles
        .iter()
        .map(|s| shape_to_kurbo_path_for_bench(s).unwrap())
        .collect();
    measure(
        "three_circle_arrangement_divide",
        || divide_paths_pure(&paths),
        |v| contours_summary(v),
    );

    let mut contours = Vec::new();
    for shape in [
        ellipse(0., 0., 50., 50.),
        ellipse(0., 0., 35., 35.),
        ellipse(0., 0., 10., 10.),
    ] {
        let (start, segments, _) = shape.to_path_segments().unwrap();
        contours.push(Contour { start, segments });
    }
    let mut compound = Shape::new("nested".into(), ShapeData::Compound { contours });
    compound.fill_rule = FillRule::EvenOdd;
    let document = Document::new("benchmark".into(), 1000., 1000.);
    measure(
        "compound_evenodd_normalization",
        || document.closed_kurbo_path(&compound),
        |v| json!({"present": v.is_some(), "path_elements": v.as_ref().map(|p| p.elements().len()).unwrap_or(0)}),
    );

    let self_crossing = Shape::new(
        "self crossing".into(),
        ShapeData::Compound {
            contours: vec![Contour {
                start: Vec2::new(0., 0.),
                segments: vec![
                    PathSegment::Cubic {
                        c1: Vec2::new(60., 100.),
                        c2: Vec2::new(-60., 100.),
                        to: Vec2::new(30., 0.),
                    },
                    PathSegment::Line {
                        to: Vec2::new(0., 0.),
                    },
                ],
            }],
        },
    );
    measure(
        "cubic_self_intersection_normalization",
        || document.closed_kurbo_path(&self_crossing),
        |v| json!({"present": v.is_some(), "path_elements": v.as_ref().map(|p| p.elements().len()).unwrap_or(0)}),
    );
}
