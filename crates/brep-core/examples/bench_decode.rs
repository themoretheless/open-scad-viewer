use brep_core::{GearSpec, Model, gear};
use sha2::{Digest, Sha256};
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
        let value = value_codec::to_value(&model).unwrap();
        let canonical = value_codec::to_string(&model).unwrap();
        let sha256: String = Sha256::digest(canonical.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        for _ in 0..3 {
            black_box(value_codec::from_value::<Model>(value.clone()).unwrap());
        }
        let mut samples = Vec::new();
        for _ in 0..9 {
            let input = value.clone();
            let start = Instant::now();
            let decoded = value_codec::from_value::<Model>(black_box(input)).unwrap();
            samples.push(start.elapsed().as_secs_f64() * 1000.);
            assert_eq!(value_codec::to_string(&decoded).unwrap(), canonical);
        }
        let mut sorted = samples.clone();
        sorted.sort_by(f64::total_cmp);
        println!(
            "{{\"teeth\":{teeth},\"faces\":{},\"serializedBytes\":{},\"sha256\":\"{sha256}\",\"p50Ms\":{},\"samplesMs\":{samples:?}}}",
            model.faces.len(),
            canonical.len(),
            sorted[sorted.len() / 2]
        );
    }
}
