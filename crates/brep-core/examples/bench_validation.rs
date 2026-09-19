use brep_core::{GearSpec, gear};
use std::{hint::black_box, time::Instant};

fn main() {
    for teeth in [12, 32, 60] {
        let model = gear(&GearSpec {
            teeth,
            herringbone: true,
            helix_angle_deg: 20.,
            bore: 3.,
            ..GearSpec::default()
        })
        .expect("valid gear fixture");
        for _ in 0..3 {
            black_box(model.validate().unwrap());
        }
        let mut samples = Vec::new();
        for _ in 0..9 {
            let start = Instant::now();
            let report = black_box(&model).validate().unwrap();
            samples.push(start.elapsed().as_secs_f64() * 1000.);
            assert!(report.topology_valid);
            assert_eq!(report.boundary_edge_count, 0);
        }
        let mut sorted = samples.clone();
        sorted.sort_by(f64::total_cmp);
        println!(
            "{{\"teeth\":{teeth},\"faces\":{},\"p50Ms\":{},\"samplesMs\":{samples:?}}}",
            model.faces.len(),
            sorted[sorted.len() / 2]
        );
    }
}
