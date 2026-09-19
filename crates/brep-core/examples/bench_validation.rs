use brep_core::{GearSpec, gear};
use rbench::{DropPolicy, Suite};
use std::hint::black_box;

fn main() -> rbench::Result<()> {
    let mut suite = Suite::new("brep-core/validation");
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
        let input = model.clone();
        suite
            .bench_with_input(
                Box::leak(format!("validate/{teeth}-teeth").into_boxed_str()),
                move || input.clone(),
                |model| {
                    let report = model.validate().unwrap();
                    assert!(report.topology_valid);
                    assert_eq!(report.boundary_edge_count, 0);
                    report.face_count
                },
                DropPolicy::InsideTiming,
            )
            .parameter("teeth", teeth as f64)
            .parameter("faces", model.faces.len() as f64);
    }
    suite.main()
}
