use brep_core::{GearSpec, Model, gear};
use rbench::{DropPolicy, Suite};
use sha2::{Digest, Sha256};

fn main() -> rbench::Result<()> {
    let mut suite = Suite::new("brep-core/decode");
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
        let input = value.clone();
        let expected = canonical.clone();
        suite
            .bench_with_input(
                Box::leak(format!("decode/{teeth}-teeth").into_boxed_str()),
                move || input.clone(),
                move |input| {
                    let decoded = value_codec::from_value::<Model>(input.clone()).unwrap();
                    assert_eq!(value_codec::to_string(&decoded).unwrap(), expected);
                    decoded.faces.len()
                },
                DropPolicy::InsideTiming,
            )
            .parameter("teeth", teeth as f64)
            .parameter("bytes", canonical.len() as f64)
            .parameter("sha256_bytes", sha256.len() as f64);
    }
    suite.main()
}
