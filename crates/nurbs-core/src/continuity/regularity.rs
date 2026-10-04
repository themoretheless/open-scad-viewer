//! Outward-rounded Bernstein enclosure of the rational boundary normal.
//! A nonzero component over every subinterval proves regularity of the seam.
use super::Boundary;
use crate::{check,intersection::{next_down,next_up},surface::Surface,Result};
use value_codec::{Value,json};
#[derive(Clone,Copy,Debug)]pub(super) struct I{pub(super) lo:f64,pub(super) hi:f64}
impl I {
 pub(super) fn point(x:f64)->Self{Self{lo:x,hi:x}}
 pub(super) fn add(self,b:Self)->Self{Self{lo:next_down(self.lo+b.lo),hi:next_up(self.hi+b.hi)}}
 pub(super) fn sub(self,b:Self)->Self{Self{lo:next_down(self.lo-b.hi),hi:next_up(self.hi-b.lo)}}
 pub(super) fn mul(self,b:Self)->Self{let v=[self.lo*b.lo,self.lo*b.hi,self.hi*b.lo,self.hi*b.hi];Self{lo:next_down(v.iter().copied().fold(f64::INFINITY,f64::min)),hi:next_up(v.iter().copied().fold(f64::NEG_INFINITY,f64::max))}}
 pub(super) fn div_positive(self,b:Self)->Result<Self>{check(b.lo>0.&&b.hi.is_finite(),"Seam certificate denominator is not separated from zero")?;Ok(self.mul(Self{lo:next_down(1./b.hi),hi:next_up(1./b.lo)}))}
 pub(super) fn finite(self)->bool{self.lo.is_finite()&&self.hi.is_finite()}
}
type Pair=[I;8];
pub(super) fn insert<const N:usize>(knots:&mut Vec<f64>,controls:&mut Vec<[I;N]>,p:usize,u:f64)->Result<()>{
 check(controls.len()<8192,"Seam certificate knot refinement exceeds 8192 controls")?;
 let n=controls.len()-1;let s=knots.iter().filter(|&&k|k==u).count();
 let mut k=p;while k+1<knots.len()&&knots[k+1]<=u{k+=1;}
 check(s<=p&&k>=p&&k-s<=n,"Seam certificate knot insertion is invalid")?;
 let mut next=vec![[I::point(0.);N];n+2];next[..=k-p].copy_from_slice(&controls[..=k-p]);next[k-s+1..n+2].copy_from_slice(&controls[k-s..n+1]);
 for i in k-p+1..=k-s{
  let alpha=I::point(u).sub(I::point(knots[i])).div_positive(I::point(knots[i+p]).sub(I::point(knots[i])))?;
  let beta=I::point(1.).sub(alpha);
  next[i]=std::array::from_fn(|a|alpha.mul(controls[i][a]).add(beta.mul(controls[i-1][a])));
 }
 knots.insert(k+1,u);*controls=next;Ok(())
}
pub(super) fn choose(n:usize,k:usize)->I{let mut x=I::point(1.);for i in 0..k.min(n-k){x=x.mul(I::point((n-i)as f64)).div_positive(I::point((i+1)as f64)).unwrap();}x}
fn product(a:&[I],b:&[I])->Vec<I>{
 let m=a.len()-1;let n=b.len()-1;
 (0..=m+n).map(|k|{
  let mut x=I::point(0.);for i in k.saturating_sub(n)..=k.min(m){
   let factor=choose(m,i).mul(choose(n,k-i)).div_positive(choose(m+n,k)).unwrap();
   x=x.add(factor.mul(a[i]).mul(b[k-i]));
  }x
 }).collect()
}
fn difference(a:Vec<I>,b:Vec<I>)->Vec<I>{a.into_iter().zip(b).map(|(x,y)|x.sub(y)).collect()}
fn normal_net(points:&[Pair])->[Vec<I>;3]{
 let p=points.len()-1;
 let component=|index:usize|points.iter().map(|h|h[index]).collect::<Vec<_>>();
 let derivative=|index:usize|points.windows(2).map(|h|h[1][index].sub(h[0][index]).mul(I::point(p as f64))).collect::<Vec<_>>();
 let w=component(3);let dw=derivative(3);let inward_w=component(7);
 let along:Vec<Vec<I>>=(0..3).map(|a|difference(product(&derivative(a),&w),product(&component(a),&dw))).collect();
 let cross:Vec<Vec<I>>=(0..3).map(|a|difference(product(&component(4+a),&w),product(&component(a),&inward_w))).collect();
 std::array::from_fn(|a|{let b=(a+1)%3;let c=(a+2)%3;difference(product(&cross[b],&along[c]),product(&cross[c],&along[b]))})
}
fn split(a:&[I])->(Vec<I>,Vec<I>){
 let mut row=a.to_vec();let mut left=vec![row[0]];let mut right=vec![*row.last().unwrap()];
 while row.len()>1{row=row.windows(2).map(|p|p[0].add(p[1]).mul(I::point(0.5))).collect();left.push(row[0]);right.push(*row.last().unwrap());}
 right.reverse();(left,right)
}
pub(super) fn certify(surface:&Surface,b:Boundary)->Result<Value>{
 certify_impl(surface,b,false)
}
// Exact along-knot jets must be proven by the caller before bypassing the basis gate.
pub(super) fn certify_after_exact_along_jets(surface:&Surface,b:Boundary)->Result<Value>{
 certify_impl(surface,b,true)
}
fn certify_impl(surface:&Surface,b:Boundary,exact_along_jets:bool)->Result<Value>{
 let(p,n,k,_)=b.along(surface);let a=k[p];let z=k[n];let mut breaks=k[p..=n].to_vec();breaks.dedup();
 for &u in &breaks{if !exact_along_jets&&u>a&&u<z&&k.iter().filter(|&&x|x==u).count()>=p{
  return Ok(json!({"certified":false,"reason":"seam-basis-is-not-C1","unresolvedIntervals":[[a,z]]}));
 }}
 let mut controls:Vec<Pair>=(0..n).map(|i|std::array::from_fn(|c|{
  let(u,v)=b.index(surface,i,c/4);let axis=c%4;let w=I::point(surface.weights[u][v]);
  if axis==3{w}else{I::point(surface.control_points[u][v][axis]).mul(w)}
 })).collect();let mut knots=k.to_vec();
 for &u in &breaks{let target=if u==a||u==z{p+1}else{p};while knots.iter().filter(|&&x|x==u).count()<target{insert(&mut knots,&mut controls,p,u)?;}}
 let start=knots.iter().position(|&x|x==a).unwrap();let end=knots.iter().rposition(|&x|x==z).unwrap();
 knots=knots[start..=end].to_vec();let count=knots.len()-p-1;controls=controls[start..start+count].to_vec();
 let mut pending=Vec::new();let mut spans=0;
 for i in p..count{if knots[i]<knots[i+1]{pending.push((normal_net(&controls[i-p..=i]),[knots[i],knots[i+1]],0));spans+=1;}}
 let mut inspected=0;let mut accepted=0;let mut unresolved=Vec::new();let mut resource=false;
 while let Some((net,domain,depth))=pending.pop(){
  inspected+=1;if inspected>4096{resource=true;unresolved.push(domain);break;}
  let separated=net.iter().any(|v|v.iter().all(|x|x.finite()&&x.lo>0.)||v.iter().all(|x|x.finite()&&x.hi<0.));
  if separated{accepted+=1;continue;}
  let mid=domain[0]+(domain[1]-domain[0])*0.5;
  if depth>=12||mid==domain[0]||mid==domain[1]||net.iter().flatten().any(|x|!x.finite()){unresolved.push(domain);continue;}
  let split:Vec<_>=net.iter().map(|v|split(v)).collect();
  pending.push((std::array::from_fn(|i|split[i].1.clone()),[mid,domain[1]],depth+1));
  pending.push((std::array::from_fn(|i|split[i].0.clone()),[domain[0],mid],depth+1));
 }
 let unresolved_count=unresolved.len()+pending.len();unresolved.extend(pending.into_iter().map(|(_,domain,_)|domain));unresolved.truncate(32);
 Ok(json!({"certified":unresolved_count==0&&!resource,"method":"outward-Bernstein-normal-component-separation",
  "domain":[a,z],"spans":spans,"inspectedCells":inspected,"acceptedCells":accepted,
  "unresolvedCount":unresolved_count,"unresolvedIntervals":unresolved,"resourceLimitReached":resource,
  "maxDepth":12,"maxCells":4096}))
}
