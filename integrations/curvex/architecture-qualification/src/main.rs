use planar_geometry::measure::ArcLengthIndex;
use std::{hint::black_box,time::Instant};
fn linear(p:[f64;2],table:&[(f64,[f64;2])])->f64 {
 let mut best_t=0.;let mut best_d=f64::INFINITY;
 for w in table.windows(2) {let (t0,a)=w[0];let(t1,b)=w[1];let ab=[b[0]-a[0],b[1]-a[1]];let ap=[p[0]-a[0],p[1]-a[1]];let l2=ab[0]*ab[0]+ab[1]*ab[1];let u=if l2<1e-18{0.}else{((ap[0]*ab[0]+ap[1]*ab[1])/l2).clamp(0.,1.)};let q=[a[0]+ab[0]*u,a[1]+ab[1]*u];let d=(p[0]-q[0]).hypot(p[1]-q[1]);if d<best_d{best_d=d;best_t=t0+(t1-t0)*u;}}
 best_t
}
fn main(){
 let points:Vec<_>=(0..4096).map(|i|{let t=i as f64/4095.;[t*1000.,(t*40.).sin()*80.]}).collect();
 let queries:Vec<_>=(0..4096).map(|i|{let t=i as f64/4095.;[t*1000.,(t*40.).sin()*80.+1.]}).collect();
 let mut total=0.;let mut table=vec![(0.,points[0])];for w in points.windows(2){total+=(w[1][0]-w[0][0]).hypot(w[1][1]-w[0][1]);table.push((total,w[1]));}
 let start=Instant::now();let index=ArcLengthIndex::new(&points).unwrap();let build_us=start.elapsed().as_secs_f64()*1e6;
 for &p in &queries {let a=linear(p,&table);let b=index.nearest_length(p).unwrap();assert!((a-b).abs()<1e-9);}
 let mut a=vec![];let mut b=vec![];
 for i in 0..8 {for indexed in if i%2==0{[false,true,true,false]}else{[true,false,false,true]} {
  let start=Instant::now();for &p in &queries {black_box(if indexed{index.nearest_length(p).unwrap()}else{linear(p,&table)});}
  let ms=start.elapsed().as_secs_f64()*1e3;if i>0{if indexed{b.push(ms)}else{a.push(ms)}}
 }}
 a.sort_by(f64::total_cmp);b.sort_by(f64::total_cmp);
 println!("{{\"segments\":4095,\"queries\":4096,\"build_us\":{build_us},\"linear_ms\":{},\"indexed_ms\":{},\"speedup\":{},\"linear_samples_ms\":{:?},\"indexed_samples_ms\":{:?}}}",a[a.len()/2],b[b.len()/2],a[a.len()/2]/b[b.len()/2],a,b);
}
