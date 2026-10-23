use brep_core::{GearSpec, gear};
use geometry_bridge::dispatch;
use sha2::{Digest, Sha256};
use std::{hint::black_box, time::Instant};
use value_codec::json;

fn hash(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn main() {
    for teeth in [12, 32, 60] {
        let model = gear(&GearSpec {
            teeth,
            herringbone: true,
            helix_angle_deg: 20.,
            bore: 3.,
            ..GearSpec::default()
        })
        .unwrap();
        let request = json!({"op":"brep_nurbs_inspect", "model":model});
        let input = value_codec::to_string(&request).unwrap();
        let expected = value_codec::to_string(&dispatch(request.clone()).unwrap()).unwrap();
        for _ in 0..3 {
            black_box(dispatch(request.clone()).unwrap());
        }
        let mut samples = Vec::new();
        for _ in 0..9 {
            let owned = request.clone();
            let start = Instant::now();
            let report = dispatch(black_box(owned)).unwrap();
            samples.push(start.elapsed().as_secs_f64() * 1000.);
            assert_eq!(value_codec::to_string(&report).unwrap(), expected);
            assert_eq!(report["topologyValid"], json!(true));
        }
        let mut sorted = samples.clone();
        sorted.sort_by(f64::total_cmp);
        println!(
            "{}",
            json!({
                "teeth":teeth, "inputBytes":input.len(), "inputSha256":hash(&input),
                "outputSha256":hash(&expected), "p50Ms":sorted[sorted.len()/2],
                "samplesMs":samples,
                "scope":"Native owned dispatch including field decode, validation, report encoding and request drop; input preparation and equality checks excluded"
            })
        );
    }
}
