use super::*;
use crate::{Limits,SourceArena,ToleranceContext};
fn arena()->SourceArena{SourceArena::authored("exact-tensor-transform",0,vec![AuthoredScalar::Binary64Bits(0f64.to_bits())]).unwrap()}
fn naive(a:&Net,ctx:&mut PredicateContext<'_>)->Result<Net,Reason>{
 let(p,q)=(a.len()-1,a[0].len()-1);let mut out=vec![vec![Expansion::scalar(0.);q+1];p+1];
 for i in 0..=p{for j in 0..=q{for u in i..=p{for v in j..=q{let n=choose(p,i)*choose(p-i,u-i)*choose(q,j)*choose(q-j,v-j);let f=Expansion::integer(n,ctx)?;let t=a[i][j].mul(&f,ctx)?;out[u][v]=if(u-i+v-j)%2==0{out[u][v].add(&t,ctx)?}else{out[u][v].sub(&t,ctx)?};}}}}Ok(out)
}
#[test]
fn separable_basis_change_equals_independent_tensor_expansion_for_all_degrees(){
 let a=arena();let t=ToleranceContext::default_valid();
 for p in 1..=8{for q in 1..=8{
  let net:Net=(0..=p).map(|i|(0..=q).map(|j|Expansion::scalar(((i*i+3*j+2*i*j)%11)as f64-5.)).collect()).collect();
  let mut ctx=PredicateContext::new(&a,&t,Limits::default(),None);let actual=power(&net,&mut ctx).unwrap();let used=ctx.work_used();
  let mut ctx=PredicateContext::new(&a,&t,Limits::default(),None);let expected=naive(&net,&mut ctx).unwrap();if p==4&&q==4{assert!(used<ctx.work_used());}
  for i in 0..=p{for j in 0..=q{assert_eq!(actual[i][j].sub(&expected[i][j],&mut ctx).unwrap().sign(),Sign::Zero);}}
 }}
}
#[test]
fn asymmetric_quadric_entries_are_combined_exactly_without_rounding(){
 let a=arena();let t=ToleranceContext::default_valid();
 let nets=[vec![vec![0.,0.],vec![1.,1.]],vec![vec![0.,1.],vec![0.,1.]],vec![vec![0.,0.],vec![0.,1.]],vec![vec![1.,1.],vec![1.,1.]]];
 for drift in [false,true]{let mut ctx=PredicateContext::new(&a,&t,Limits::default(),None);let h=nets.iter().map(|n|power(&n.iter().map(|row|row.iter().map(|x|Expansion::scalar(*x)).collect()).collect(),&mut ctx).unwrap()).collect::<Vec<_>>();let mut q=vec![Expansion::scalar(0.);16];q[1]=Expansion::scalar(4.);q[4]=Expansion::scalar(-3.);q[11]=Expansion::scalar(if drift{f64::from_bits(11f64.to_bits()+1)}else{11.});q[14]=Expansion::scalar(-12.);
 assert_eq!(identity(&h,&q,&mut ctx).unwrap(),if drift{BezierIdentity::Different}else{BezierIdentity::Equal});}
}
