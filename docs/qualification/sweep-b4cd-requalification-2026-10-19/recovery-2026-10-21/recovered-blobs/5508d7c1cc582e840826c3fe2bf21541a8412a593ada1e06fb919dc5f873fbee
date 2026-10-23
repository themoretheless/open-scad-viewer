//! Whole-surface positional error on a common parameter domain. All homogeneous
//! conversion, knot insertion and Bernstein products use outward intervals.
use super::regularity::{I,insert,choose};
use crate::{Result,check,surface::Surface,intersection::next_up};
type H=[I;4];
struct Net {p:usize,q:usize,u:Vec<f64>,v:Vec<f64>,h:Vec<Vec<H>>}
impl Net {
 fn from(s:&Surface)->Self{Self{p:s.degree_u,q:s.degree_v,u:s.knots_u.clone(),v:s.knots_v.clone(),h:s.control_points.iter().enumerate().map(|(i,row)|row.iter().enumerate().map(|(j,p)|{
  let w=I::point(s.weights[i][j]);[I::point(p[0]).mul(w),I::point(p[1]).mul(w),I::point(p[2]).mul(w),w]
 }).collect()).collect()}}
 fn refine(&mut self,axis:bool,breaks:&[f64])->Result<()>{
  let degree=if axis{self.p}else{self.q};
  for &value in breaks{
   loop{
    let knots=if axis{&self.u}else{&self.v};
    let target=if value==breaks[0]||value==*breaks.last().unwrap(){degree+1}else{degree};
    if knots.iter().filter(|&&x|x==value).count()>=target{break;}
    check((self.h.len()+1).saturating_mul(self.h[0].len()+1)<=262144,"Surface deviation refinement exceeds control budget")?;
    if axis{
     let mut columns=Vec::new();let mut next_knots=self.u.clone();
     for j in 0..self.h[0].len(){let mut column=self.h.iter().map(|row|row[j]).collect();let mut knots=self.u.clone();insert(&mut knots,&mut column,degree,value)?;next_knots=knots;columns.push(column);}
     self.u=next_knots;self.h=(0..columns[0].len()).map(|i|columns.iter().map(|column|column[i]).collect()).collect();
    }else{
     let mut next_knots=self.v.clone();for row in &mut self.h{let mut knots=self.v.clone();insert(&mut knots,row,degree,value)?;next_knots=knots;}self.v=next_knots;
    }
   }
  }Ok(())
 }
 fn patch(&self,u:f64,v:f64)->Result<Vec<Vec<H>>>{
  let i=(self.p..self.h.len()).find(|&i|self.u[i]==u&&self.u[i+1]>u).ok_or_else(||crate::input("Missing U interval"))?;
  let j=(self.q..self.h[0].len()).find(|&j|self.v[j]==v&&self.v[j+1]>v).ok_or_else(||crate::input("Missing V interval"))?;
  Ok((i-self.p..=i).map(|i|self.h[i][j-self.q..=j].to_vec()).collect())
 }
}
fn product_factor(p:usize,q:usize,i:usize,j:usize)->Result<I>{choose(p,i).mul(choose(q,j)).div_positive(choose(p+q,i+j))}
fn patch_error(a:&[Vec<H>],b:&[Vec<H>])->Result<f64>{
 let(p,q,r,s)=(a.len()-1,a[0].len()-1,b.len()-1,b[0].len()-1);
 let mut coefficients=vec![[I::point(0.);3];(p+r+1)*(q+s+1)];
 for i in 0..=p{for j in 0..=q{for k in 0..=r{for l in 0..=s{
  let factor=product_factor(p,r,i,k)?.mul(product_factor(q,s,j,l)?);
  for axis in 0..3{let delta=a[i][j][axis].mul(b[k][l][3]).sub(b[k][l][axis].mul(a[i][j][3]));let c=&mut coefficients[(i+k)*(q+s+1)+j+l][axis];*c=c.add(factor.mul(delta));}
 }}}}
 let wa=a.iter().flatten().map(|h|h[3].lo).fold(f64::INFINITY,f64::min);
 let wb=b.iter().flatten().map(|h|h[3].lo).fold(f64::INFINITY,f64::min);
 check(wa>0.&&wb>0.,"Surface deviation denominator is not proven positive")?;
 let denominator=I::point(wa).mul(I::point(wb));let mut error=0_f64;
 for c in coefficients{let mut sum=I::point(0.);for v in c{let v=v.div_positive(denominator)?;let m=I::point(v.lo.abs().max(v.hi.abs()));sum=sum.add(m.mul(m));}error=error.max(next_up(sum.hi.sqrt()));}
 check(error.is_finite(),"Surface deviation bound overflowed")?;Ok(error)
}
/// A bound for corresponding points, hence also a Hausdorff upper bound.
/// Both surfaces must have the exact same active parameter rectangle.
pub fn positional_upper(a:&Surface,b:&Surface)->Result<f64>{
 a.validate()?;b.validate()?;
 if a.degree_u==b.degree_u&&a.degree_v==b.degree_v&&a.knots_u==b.knots_u&&a.knots_v==b.knots_v&&a.control_points==b.control_points&&a.weights==b.weights&&a.periodic_u==b.periodic_u&&a.periodic_v==b.periodic_v{return Ok(0.);}
 let domain=|s:&Surface|[s.knots_u[s.degree_u],s.knots_u[s.control_points.len()],s.knots_v[s.degree_v],s.knots_v[s.control_points[0].len()]];
 let d=domain(a);check(d==domain(b),"Surface deviation requires identical parameter domains")?;
 let breaks=|a:&[f64],b:&[f64],lo:f64,hi:f64|{let mut values=a.iter().chain(b).copied().filter(|&x|x>=lo&&x<=hi).collect::<Vec<_>>();values.sort_by(f64::total_cmp);values.dedup();values};
 let us=breaks(&a.knots_u,&b.knots_u,d[0],d[1]);let vs=breaks(&a.knots_v,&b.knots_v,d[2],d[3]);
 let work=(us.len()-1).saturating_mul(vs.len()-1).saturating_mul((a.degree_u+1)*(a.degree_v+1)).saturating_mul((b.degree_u+1)*(b.degree_v+1));
 check(work<=2_000_000,"Surface deviation exceeds coefficient work budget")?;
 let mut a=Net::from(a);let mut b=Net::from(b);
 for net in [&mut a,&mut b]{net.refine(true,&us)?;net.refine(false,&vs)?;}
 let mut error=0_f64;
 for &u in &us[..us.len()-1]{for &v in &vs[..vs.len()-1]{error=error.max(patch_error(&a.patch(u,v)?,&b.patch(u,v)?)?);}}
 Ok(error)
}
