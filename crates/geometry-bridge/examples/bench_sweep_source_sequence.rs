//! Exact original constructor sequence: native diagnostic, unchanged budgets.
use std::{fs,time::Instant};
use value_codec::{Value,json};
fn main() {
    let args=std::env::args().skip(1).collect::<Vec<_>>();assert_eq!(args.len(),2);
    let paths:Vec<String>=value_codec::from_str(&fs::read_to_string(&args[0]).unwrap()).unwrap();
    let requests=paths.iter().map(|p|if p.ends_with(".bin") { value_codec::decode_binary(&fs::read(p).unwrap()).unwrap() } else {value_codec::from_str::<Value>(&fs::read_to_string(p).unwrap()).unwrap()}).collect::<Vec<_>>();
    let start=Instant::now();let mut reports=Vec::new();let mut times=Vec::new();
    for request in requests {
        let op=request["op"].clone();let started=Instant::now();
        reports.push(geometry_bridge::dispatch(request).unwrap());
        times.push(json!({"op":op,"milliseconds":started.elapsed().as_secs_f64()*1000.}));
    }
    let milliseconds=start.elapsed().as_secs_f64()*1000.;
    fs::write(format!("{}.reports.json",args[1]),value_codec::to_string(&reports).unwrap()).unwrap();
    fs::write(&args[1],value_codec::to_string(&json!({"milliseconds":milliseconds,"calls":times})).unwrap()).unwrap();
}
