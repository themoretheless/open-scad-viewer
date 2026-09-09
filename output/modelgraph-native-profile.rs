use modelgraph_schema_check::{schema,mechanical,mechanical_old};
use serde_json::{Value,json};
use std::hint::black_box;
use std::time::Instant;
fn main(){
 let doc:Value=serde_json::from_str(&std::fs::read_to_string("output/modelgraph-schema-profile-skadis.json").unwrap()).unwrap();
 let options=json!({"diameter":12,"pitch":1.5,"length":12,"internal":false,"wall":3,"clearance":0.2,"starts":1,"left_handed":false,"segments_per_turn":32});
 let funcs:[(&str,Box<dyn Fn()->usize>);3]=[
  ("schema_clone_validate",Box::new(||schema::validate(black_box(doc.clone())).unwrap()["nodes"].as_array().unwrap().len())),
  ("thread_string_keys",Box::new(||mechanical_old::thread(black_box(&options),"/test").unwrap().source.len())),
  ("thread_numeric_keys",Box::new(||mechanical::thread(black_box(&options),"/test").unwrap().source.len()))
 ];
 let mut raw=vec![vec![];3];for(_,f)in &funcs{for _ in 0..10{black_box(f());}}
 for batch in 0..8 {for k in 0..3{let k=if batch%2==0{k}else{2-k};let start=Instant::now();for _ in 0..100{black_box((funcs[k].1)());}raw[k].push(start.elapsed().as_secs_f64()*10.);}}
 let results:Vec<Value>=funcs.iter().enumerate().map(|(i,(name,_))|{let mut ordered=raw[i].clone();ordered.sort_by(f64::total_cmp);json!({"name":name,"median_ms":(ordered[3]+ordered[4])/2.,"raw_batch_ms":raw[i]})}).collect();
 println!("{}",json!({"method":"Release native; 10warmups +8 alternating batches of100; schema includes document clone, geometry serialization included, no mesh execution", "results":results}));
}
