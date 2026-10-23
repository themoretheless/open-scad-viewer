use brep_core::linear_canal::{self, End};
use value_codec::json;
fn main() {
    let centers = [[10., -7., 5.], [13., -3., 17.]];
    let direction = [1., 0., 0.];
    let mut cases = Vec::new();
    for radii in [[0.5, 1.25], [1.25, 0.5], [1., 1.], [0., 1.], [1., 0.]] {
        for sweep in [std::f64::consts::TAU, -std::f64::consts::TAU] {
            let spans = linear_canal::construct(centers, radii, direction, sweep).unwrap();
            let model = linear_canal::to_capped_region(&spans, 1e-7).unwrap();
            let mut components = Vec::new();
            for (angular, span) in spans.iter().enumerate() {
                components.push(json!({"kind":"shaft","angular":angular,"angularCount":spans.len(),"surface":span.surface()}));
                for (end, kind) in [(End::Start, "start"), (End::Finish, "finish")] {
                    let caps = span.end_caps(end).unwrap();
                    for (meridian, cap) in caps.iter().enumerate() {
                        components.push(json!({"kind":kind,"angular":angular,"angularCount":spans.len(),"meridian":meridian,"meridianCount":caps.len(),"surface":cap.surface()}));
                    }
                }
            }
            cases.push(json!({"centers":centers,"radii":radii,"direction":direction,"sweep":sweep,"components":components,"model":model}));
        }
    }
    println!("{}",value_codec::to_string_pretty(&json!({"schema":"cad-canal-caps/1","scope":"original capped support assembly; native closed-body admission unproven","cases":cases})).unwrap());
}
