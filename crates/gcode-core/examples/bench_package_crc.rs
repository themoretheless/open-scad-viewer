//! Run before/after in release mode; compare the saved archives byte-for-byte.
use std::{hint::black_box, path::PathBuf, time::Instant};

fn main() {
    let output = PathBuf::from(std::env::args().nth(1).expect("output directory"));
    std::fs::create_dir_all(&output).unwrap();
    for size in [65_536, 1_048_576, 3_145_728] {
        let line = "G1 X10 Y20 E1 F1200\n";
        let source = line.repeat(size / line.len());
        let expected = gcode_core::package_gcode_3mf(&source).unwrap();
        for _ in 0..3 {
            black_box(gcode_core::package_gcode_3mf(black_box(&source)).unwrap());
        }
        let mut samples = Vec::new();
        for _ in 0..9 {
            let start = Instant::now();
            let archive = gcode_core::package_gcode_3mf(black_box(&source)).unwrap();
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            assert_eq!(archive, expected);
            samples.push(ms);
        }
        let mut sorted = samples.clone();
        sorted.sort_by(f64::total_cmp);
        std::fs::write(output.join(format!("{size}.3mf")), &expected).unwrap();
        println!("{{\"input_bytes\":{},\"output_bytes\":{},\"median_ms\":{},\"samples_ms\":{:?}}}", source.len(), expected.len(), sorted[4], samples);
    }
}
