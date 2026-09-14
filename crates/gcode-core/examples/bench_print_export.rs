//! Print-export microbenchmarks via rbench (release build).
//!
//! ```sh
//! cargo run --release -p gcode-core --example bench_print_export -- --profile quick
//! cargo rbench run --program target/release/examples/bench_print_export --protocol \
//!   --repetitions 8 -o .rbench/print-export -- --profile quick --json
//! ```
use gcode_core::{
    emit_gcode_3mf_job, emit_job, parse_job, JobProfile, MeshBody, PlannedLayer, PlannedPath,
};
use rbench::{DropPolicy, Suite};

fn layers(n: usize) -> Vec<PlannedLayer> {
    (0..n)
        .map(|i| PlannedLayer {
            z_mm: 0.2 * (i as f64 + 1.0),
            paths: vec![
                PlannedPath {
                    points: vec![[0.0, 0.0], [40.0, 0.0], [40.0, 40.0], [0.0, 40.0]],
                    closed: true,
                },
                PlannedPath {
                    points: vec![[50.0, 0.0], [60.0, 0.0], [60.0, 10.0], [50.0, 10.0]],
                    closed: true,
                },
            ],
        })
        .collect()
}

fn cube() -> MeshBody {
    MeshBody {
        positions: vec![
            0., 0., 0., 1., 0., 0., 1., 1., 0., 0., 1., 0., 0., 0., 1., 1., 0., 1., 1., 1., 1., 0.,
            1., 1.,
        ],
        indices: vec![
            0, 1, 2, 0, 2, 3, 4, 6, 5, 4, 7, 6, 0, 4, 5, 0, 5, 1, 1, 5, 6, 1, 6, 2, 2, 6, 7, 2, 7,
            3, 3, 7, 4, 3, 4, 0,
        ],
    }
}

fn main() -> rbench::Result<()> {
    let mut suite = Suite::new("print-export");
    suite
        .bench_with_input(
            "job/emit_gcode/layers_8",
            || (layers(8), JobProfile::default()),
            |(layers, job)| emit_job(layers, job).unwrap().len(),
            DropPolicy::InsideTiming,
        )
        .parameter("layers", 8)
        .parameter("paths_per_layer", 2);
    suite
        .bench_with_input(
            "job/emit_gcode_3mf/layers_8",
            || (layers(8), JobProfile::default(), cube()),
            |(layers, job, mesh)| emit_gcode_3mf_job(layers, job, Some(mesh)).unwrap().len(),
            DropPolicy::InsideTiming,
        )
        .parameter("layers", 8);
    suite
        .bench_with_input(
            "job/parse_gcode/layers_8",
            || emit_job(&layers(8), &JobProfile::default()).unwrap(),
            |gcode| parse_job(gcode).unwrap().layers,
            DropPolicy::InsideTiming,
        )
        .parameter("layers", 8);
    suite.main()
}
