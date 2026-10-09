use geometry_bridge::dispatch;
use value_codec::json;
fn main() {
    let mut cases = Vec::new();
    for scale in [0.001, 1., 1000.] {
        for offset in [0., 12345.] {
            for reverse in [false, true] {
                for points in [
                    vec![[0., 0.], [2., 2.], [0., 2.], [2., 0.], [0., 0.]],
                    vec![[0., 0.], [2., 0.], [1., 0.], [1., 2.], [0., 2.], [0., 0.]],
                ] {
                    let mut points: Vec<[f64; 2]> = points
                        .into_iter()
                        .map(|p| [p[0] * scale + offset, p[1] * scale + offset])
                        .collect();
                    if reverse {
                        points.reverse();
                    }
                    let result = dispatch(
                        json!({"op":"cad_prepare_profile","chains":[points],"tolerance":0.}),
                    )
                    .unwrap();
                    assert_eq!(result["accepted"], json!(false));
                    assert!(result.get("segmentDefect").is_some());
                    cases.push(json!({"points":points,"result":result}));
                }
            }
        }
    }
    println!("{}", json!(cases));
}
