use value_codec::{Value,json};
fn main(){
 let args:Vec<_>=std::env::args().collect();
 let mut patches:Value=value_codec::from_str(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();
 for (i,p) in patches.as_array_mut().unwrap().iter_mut().enumerate(){
  let start=(i/4) as f64/16.;
  for knot in p["knotsV"].as_array_mut().unwrap(){*knot=json!((knot.as_f64().unwrap()-start)*16.);}
  p["periodicV"]=json!(false);
 }
 std::fs::write(&args[2],value_codec::to_string(&patches).unwrap()).unwrap();
}
