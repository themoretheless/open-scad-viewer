//! Native solver boundary only; not a UI, material, or complete FEA benchmark.
use mechanics_core::truss::{Member, Model, solve};
use std::{hint::black_box, time::Instant};
use value_codec::json;

fn fixture(free_nodes: usize) -> Model {
    let mut model = Model {
        nodes_mm: vec![[10., 0., 0.], [0., 10., 0.], [0., 0., 0.]],
        members: Vec::new(),
        restrained: vec![[true; 3]; 3],
        forces_n: vec![[0.; 3]; 3],
    };
    for i in 0..free_nodes {
        model.nodes_mm.push([1., 2., 10. + i as f64 / 10.]);
        model.restrained.push([false; 3]);
        model.forces_n.push([1., -2., -3.]);
        for anchor in 0..3 {
            model.members.push(Member {
                nodes: [anchor, i + 3],
                young_mpa: 2000.,
                area_mm2: 2.,
            });
        }
    }
    model
}

fn main() {
    let mut rows = Vec::new();
    for free_nodes in [1, 40, 122] {
        let model = fixture(free_nodes);
        for _ in 0..20 {
            black_box(solve(black_box(&model)).unwrap());
        }
        let mut samples = Vec::new();
        let mut residual = 0f64;
        for _ in 0..31 {
            let start = Instant::now();
            let result = solve(black_box(&model)).unwrap();
            samples.push(start.elapsed().as_secs_f64() * 1000.);
            assert_eq!(result.free_dofs, free_nodes * 3);
            assert!(result.max_relative_residual < 1e-12);
            residual = residual.max(result.max_relative_residual);
            black_box(result);
        }
        let mut ordered = samples.clone();
        ordered.sort_by(f64::total_cmp);
        rows.push(
            json!({"nodes":model.nodes_mm.len(),"members":model.members.len(),
            "freeDofs":free_nodes*3,"p50Ms":ordered[15],"p95Ms":ordered[29],
            "maxRelativeResidual":residual,"samplesMs":samples}),
        );
    }
    println!(
        "{}",
        json!({"arch":std::env::consts::ARCH,"os":std::env::consts::OS,
        "solver":"nalgebra 0.35.0 Cholesky with diagonal equilibration",
        "scope":"native validation, assembly, solve, residuals and reactions; no WASM transport or UI",
        "warmups":20,"samples":31,"results":rows})
    );
}
