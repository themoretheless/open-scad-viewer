use brep_core::linear_canal;
use value_codec::json;
fn main() {
    let centers = [[10., -7., 5.], [13., -3., 17.]];
    let direction = [1., 0., 0.];
    let mut cases = Vec::new();
    for radii in [[0.5, 1.25], [1.25, 0.5], [0., 1.], [1., 0.]] {
        for sweep in [-0.7, 0.7] {
            let spans = linear_canal::construct(centers, radii, direction, sweep).unwrap();
            let model = linear_canal::to_open_region(&spans, 1e-7).unwrap();
            let surfaces = spans
                .iter()
                .map(|s| s.surface().clone())
                .collect::<Vec<_>>();
            cases.push(json!({"centers":centers,"radii":radii,"direction":direction,"sweep":sweep,"surfaces":surfaces,"model":model}));
        }
    }
    for sweep in [std::f64::consts::TAU, -4.7] {
        let radii = [0.5, 1.25];
        let spans = linear_canal::construct(centers, radii, direction, sweep).unwrap();
        let model = linear_canal::to_open_region(&spans, 1e-7).unwrap();
        let surfaces = spans
            .iter()
            .map(|s| s.surface().clone())
            .collect::<Vec<_>>();
        cases.push(json!({"centers":centers,"radii":radii,"direction":direction,"sweep":sweep,"surfaces":surfaces,"model":model}));
    }
    println!("{}",value_codec::to_string_pretty(&json!({"schema":"cad-linear-canal/1","scope":"authored rational support sheets; no closed-body or STEP admission","cases":cases})).unwrap());
}
