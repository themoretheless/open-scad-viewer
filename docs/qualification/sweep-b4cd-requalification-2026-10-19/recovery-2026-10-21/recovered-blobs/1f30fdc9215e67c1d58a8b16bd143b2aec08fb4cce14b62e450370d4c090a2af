//! Native tolerant-parser controls. Build both revisions before timing either one.
use gcode_core::{MAX_OUTPUT_BYTES, parse_foreign};
use std::{fmt::Write, hint::black_box, time::Instant};

fn fixture(moves: usize, style: &str) -> String {
    let mut text = String::from("G90\nM83\nG1 X0 Y0 Z0.2 F600\n");
    for i in 0..moves {
        let x = (i + 1) % 2;
        match style {
            "spaced" => writeln!(text, "G1 X{x} Y0 E0.01 F600").unwrap(),
            "compact" => writeln!(text, "G1X{x}Y0E0.01F600").unwrap(),
            "comments" => writeln!(text, "G1 X{x} (move) Y0 E0.01 F600 ; print").unwrap(),
            _ => unreachable!(),
        }
    }
    assert!(text.len() <= MAX_OUTPUT_BYTES);
    text
}

fn main() {
    println!(
        "{{\"scope\":\"native parse_foreign including output allocation and drop; no WASM/UI\",\"os\":\"{}\",\"arch\":\"{}\",\"warmups\":10,\"samples\":31,\"results\":[",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    let mut first = true;
    for moves in [100, 10_000, 80_000] {
        for style in ["spaced", "compact", "comments"] {
            let text = fixture(moves, style);
            let result = parse_foreign(&text).unwrap();
            assert_eq!(result.moves.len(), moves + 1);
            assert_eq!(result.layers, 1);
            assert!((result.extrusion_mm - moves as f64 * 0.01).abs() < 1e-7);
            assert_eq!(result.print_distance_mm, moves as f64);
            assert!((result.estimated_time_s - moves as f64 / 10.0).abs() < 1e-7);
            drop(result);
            for _ in 0..10 {
                black_box(parse_foreign(black_box(&text)).unwrap());
            }
            let mut samples = Vec::with_capacity(31);
            for _ in 0..31 {
                let start = Instant::now();
                black_box(parse_foreign(black_box(&text)).unwrap());
                samples.push(start.elapsed().as_secs_f64() * 1_000.0);
            }
            let mut sorted = samples.clone();
            sorted.sort_by(f64::total_cmp);
            if !first {
                println!(",");
            }
            first = false;
            print!(
                "{{\"moves\":{moves},\"style\":\"{style}\",\"inputBytes\":{},\"p50Ms\":{},\"p95Ms\":{},\"samplesMs\":{samples:?}}}",
                text.len(),
                sorted[15],
                sorted[29]
            );
        }
    }
    println!("]}}");
}
