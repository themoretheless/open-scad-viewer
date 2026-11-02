//! Exact polynomial point evaluation followed by bounded display storage.
use super::*;
pub struct PolynomialEdgeDisplay {pub points:Vec<[f64;3]>,pub sample_storage_error_upper_mm:f64}
pub fn sample_polynomial_edge(points:&[[Q;3]],segments:usize,b:&mut Budget)->Result<PolynomialEdgeDisplay>{
 check((2..=65).contains(&points.len())&&(1..=64).contains(&segments),"Polynomial edge display limits exceeded")?;
 for p in points{for x in p{b.keep(x.clone())?;}}
 let mut out=Vec::with_capacity(segments+1);let mut error=Q::zero();
 for i in 0..=segments{
  let t=b.keep(Q::new(i.into(),segments.into()))?;
  let mut sample=[0.;3];let mut delta=Q::zero();
  for k in 0..3{
   let row=points.iter().map(|p|p[k].clone()).collect::<Vec<_>>();
   let exact=split(&row,&t,b)?.0.pop().unwrap();
   let f=exact.to_f64().filter(|x|x.is_finite()).ok_or_else(||crate::numeric_err("Polynomial edge display overflow"))?;
   let stored=b.authored(f)?;let difference=b.keep(exact-stored)?.abs();delta=b.keep(delta+difference)?;sample[k]=f;
  }
  if delta>error{error=delta;}out.push(sample);
 }
 let mut upper=error.to_f64().filter(|x|x.is_finite()).ok_or_else(||crate::numeric_err("Polynomial display error overflow"))?;
 while Q::from_float(upper).unwrap()<error{upper=upper.next_up();check(upper.is_finite(),"Polynomial display error overflow")?;}
 Ok(PolynomialEdgeDisplay{points:out,sample_storage_error_upper_mm:upper})
}
