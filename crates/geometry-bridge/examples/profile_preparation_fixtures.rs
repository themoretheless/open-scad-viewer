use geometry_bridge::dispatch;
use value_codec::json;
fn main() {
    let mut cases = Vec::new();
    for gap in [0., 0.001, 0.125, 1.] {
        for rotation in 0..4 {
            for mask in 0..16 {
                let mut chains = vec![
                    vec![[0., 0.], [10., 0.]],
                    vec![[10., gap], [10., 10.]],
                    vec![[10., 10.], [0., 10.]],
                    vec![[0., 10.], [0., 0.]],
                ];
                chains.rotate_left(rotation);
                for (i, chain) in chains.iter_mut().enumerate() {
                    if mask & (1 << i) != 0 {
                        chain.reverse();
                    }
                }
                let tolerance = gap * 1.01;
                let result = dispatch(
                    json!({"op":"cad_prepare_profile","chains":chains,"tolerance":tolerance}),
                )
                .unwrap();
                assert_eq!(result["accepted"], json!(true));
                cases.push(json!({"chains":chains,"tolerance":tolerance,"result":result}));
            }
        }
    }
    println!("{}", json!(cases));
}
