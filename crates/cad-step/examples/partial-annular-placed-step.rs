use brep_core::{analytic_features::build_partial_annular_preview, tube};
use cad_step::export_step_v9;

fn main() {
    let directory = std::env::args().nth(1).expect("output directory");
    std::fs::create_dir_all(&directory).unwrap();
    let base = tube(20., 5., 6.).unwrap();
    let (sa, ca) = 0.37_f64.sin_cos();
    let (sb, cb) = (-0.61_f64).sin_cos();
    let identity = [
        [1., 0., 0., 0.],
        [0., 1., 0., 0.],
        [0., 0., 1., 0.],
        [0., 0., 0., 1.],
    ];
    let matrices = [
        identity,
        [
            [0., 0., 1., 7.],
            [1., 0., 0., -3.],
            [0., 1., 0., 11.],
            [0., 0., 0., 1.],
        ],
        [
            [ca * cb, -sa, ca * sb, 17.],
            [sa * cb, ca, sa * sb, -9.],
            [-sb, 0., cb, 23.],
            [0., 0., 0., 1.],
        ],
    ];
    let mut parts = Vec::new();
    for (placement, matrix) in matrices.into_iter().enumerate() {
        let source = if placement == 0 {
            base.clone()
        } else {
            brep_core::transform::affine(&base, matrix).unwrap()
        };
        for edge in [2, 6, 9, 11] {
            let result = build_partial_annular_preview(&source, edge, 1.25).unwrap();
            let file = format!("placement-{placement}-edge-{edge}.step");
            std::fs::write(
                format!("{directory}/{file}"),
                export_step_v9(&result).unwrap().0,
            )
            .unwrap();
            parts.push(
                value_codec::json!({"file":file,"placement":placement,"edge":edge,"matrix":matrix}),
            );
        }
    }
    std::fs::write(
        format!("{directory}/manifest.json"),
        value_codec::to_string_pretty(
            &value_codec::json!({"schema":"cad-partial-annular-placed/1","parts":parts}),
        )
        .unwrap(),
    )
    .unwrap();
    println!("Exported {} source-bound specimens", parts.len());
}
