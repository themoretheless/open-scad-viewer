use brep_core::{analysis, operations, prism, Model};
fn main() {
    let model: Model = value_codec::from_str(include_str!(
        "../../../docs/qualification/cad-roadmap-2026-09-28/parts-history/imported-flange.json"
    ))
    .unwrap();
    assert!(prism::recognize(&model).unwrap().is_some());
    let top = model
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == 7.)
        })
        .unwrap();
    let result = operations::push_planar_face(&model, top, 1.).unwrap();
    let volume = analysis::mass_properties(&result, 1e-7, 200_000)
        .unwrap()
        .signed_volume_mm3;
    assert!((volume - 3000. * std::f64::consts::PI).abs() < 1e-5);
    println!("volume={volume}");
}
