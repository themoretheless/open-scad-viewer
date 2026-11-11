//! Correlated normal fields on matching natural Bezier boundaries.
//! Polynomial products retain one shared parameter; no sampled normals qualify.
use crate::{check, Result, Error, curve::Curve, surface::Surface, interval_eval::Interval as I};
type P=Vec<I>;
struct Work {used:usize,max:usize}
impl Work {fn step(&mut self)->Result<()> {if self.used==self.max {return Err(Error::new("NURBS_NORMAL_WORK_LIMIT","Correlated normal work exhausted"));}self.used+=1;Ok(())}}
fn choose(n:usize,k:usize)->I {
    let mut out=I::point(1.);
    for i in 0..k.min(n-k) {out=out.mul(I::point((n-i) as f64)).unwrap().div(I::point((i+1) as f64)).unwrap();}out
}
fn product(a:&P,b:&P,w:&mut Work)->Result<P> {
    let n=a.len()-1;let m=b.len()-1;let mut out=vec![I::point(0.);n+m+1];
    for i in 0..=n {for j in 0..=m {
        w.step()?;let factor=choose(n,i).mul(choose(m,j))?.div(choose(n+m,i+j))?;
        out[i+j]=out[i+j].add(a[i].mul(b[j])?.mul(factor)?)?;
    }}Ok(out)
}
fn combine(a:&P,b:&P,subtract:bool,w:&mut Work)->Result<P> {
    check(a.len()==b.len(),"Matching polynomial degrees required")?;
    a.iter().zip(b).map(|(a,b)|{w.step()?;if subtract {a.sub(*b)}else{a.add(*b)}}).collect()
}
fn cross(a:&[P;3],b:&[P;3],w:&mut Work)->Result<[P;3]> {
    let mut out:[P;3]=std::array::from_fn(|_|Vec::new());
    for k in 0..3 {let x=(k+1)%3;let y=(k+2)%3;
        out[k]=combine(&product(&a[x],&b[y],w)?,&product(&a[y],&b[x],w)?,true,w)?;
    }Ok(out)
}
fn squared(a:&[P;3],w:&mut Work)->Result<P> {
    let mut out=product(&a[0],&a[0],w)?;
    for p in &a[1..] {out=combine(&out,&product(p,p,w)?,false,w)?;}Ok(out)
}
fn hull(p:&P)->Result<I> {I::new(p.iter().map(|x|x.lo).fold(f64::INFINITY,f64::min),p.iter().map(|x|x.hi).fold(f64::NEG_INFINITY,f64::max))}
fn normal(s:&Surface,p:&Curve,reversed:bool,w:&mut Work)->Result<Option<[P;3]>> {
    let degrees=[s.degree_u,s.degree_v];let counts=[s.control_points.len(),s.control_points[0].len()];
    let knots=[&s.knots_u,&s.knots_v];
    if s.periodic_u || s.periodic_v || p.periodic || p.degree!=1 || p.control_points.len()!=2 || p.weights[0]!=p.weights[1]
        || p.weights[0]<=0. || degrees.iter().any(|d|*d==0 || *d>4) {return Ok(None);}
    for axis in 0..2 {if counts[axis]!=degrees[axis]+1 || knots[axis][..counts[axis]].iter().any(|v|*v!=knots[axis][degrees[axis]])
        || knots[axis][counts[axis]..].iter().any(|v|*v!=knots[axis][counts[axis]]) {return Ok(None);}}
    let pd=p.domain();if p.knots[..2].iter().any(|t|*t!=pd[0]) || p.knots[2..].iter().any(|t|*t!=pd[1]) {return Ok(None);}
    let domain=std::array::from_fn::<_,2,_>(|a|[knots[a][degrees[a]],knots[a][counts[a]]]);
    let points=&p.control_points;
    let Some(fixed)=(0..2).find(|&a|points[0][a]==points[1][a] && domain[a].contains(&points[0][a])) else {return Ok(None);};
    let varying=1-fixed;
    let source_reverse=if points[0][varying]==domain[varying][0] && points[1][varying]==domain[varying][1] {false}
        else if points[0][varying]==domain[varying][1] && points[1][varying]==domain[varying][0] {true} else {return Ok(None);};
    let upper=points[0][fixed]==domain[fixed][1];let row=if upper {degrees[fixed]}else{0};let neighbor=if upper {row-1}else{1};
    let origin=&s.control_points[0][0];
    let get=|r:usize,i:usize|->Result<[I;4]> {
        let [u,v]=if fixed==0 {[r,i]}else{[i,r]};let weight=I::point(s.weights[u][v]);
        let mut out=[I::point(0.);4];out[3]=weight;
        for k in 0..3 {out[k]=I::point(s.control_points[u][v][k]).sub(I::point(origin[k]))?.mul(weight)?;}Ok(out)
    };
    let mut h:Vec<[I;4]>=Vec::new();let mut transverse=Vec::new();
    for i in 0..counts[varying] {w.step()?;let a=get(row,i)?;let b=get(neighbor,i)?;
        let mut d=[I::point(0.);4];for k in 0..4 {d[k]=if upper {a[k].sub(b[k])?}else{b[k].sub(a[k])?};}h.push(a);transverse.push(d);}
    let mut along=Vec::new();for pair in h.windows(2) {let mut d=[I::point(0.);4];for k in 0..4 {d[k]=pair[1][k].sub(pair[0][k])?;}along.push(d);}
    // Positive degree/domain factors scale each whole derivative field and
    // cancel from the angle. Rational denominator powers are positive too.
    let component=|v:&Vec<[I;4]>,k:usize|->P {v.iter().map(|p|p[k]).collect()};
    let weight=component(&h,3);
    let mut a:[P;3]=std::array::from_fn(|_|Vec::new());let mut b=a.clone();
    for k in 0..3 {
        a[k]=combine(&product(&component(&along,k),&weight,w)?,&product(&component(&h,k),&component(&along,3),w)?,true,w)?;
        b[k]=combine(&product(&component(&transverse,k),&weight,w)?,&product(&component(&h,k),&component(&transverse,3),w)?,true,w)?;
    }
    let mut n=if fixed==0 {cross(&b,&a,w)?}else{cross(&a,&b,w)?};
    if source_reverse^reversed {for p in &mut n {p.reverse();}}
    Ok(Some(n))
}
// Homogeneous Bernstein substitution t=b*q/(a*(1-q)+b*q).
// The common positive denominator cancels from tangent-plane angles.
fn projective_normal(mut n:[P;3], forward:[f64;2], reversed:bool, w:&mut Work)->Result<[P;3]> {
    let factors=if reversed {forward}else{[forward[1],forward[0]]};
    for polynomial in &mut n {
        let degree=polynomial.len()-1;
        for (i,coefficient) in polynomial.iter_mut().enumerate() {
            for _ in 0..degree-i {w.step()?;*coefficient=coefficient.mul(I::point(factors[0]))?;}
            for _ in 0..i {w.step()?;*coefficient=coefficient.mul(I::point(factors[1]))?;}
        }
    }
    Ok(n)
}
pub struct Report {pub aligned:Option<bool>,pub sine_squared_bounds:Option<[f64;2]>,pub work:usize,pub reason:&'static str}
/// Complete natural-boundary parameter coverage, including both endpoints.
/// Position/edge ownership and body orientation are separate native gates.
pub fn inspect(surfaces:[&Surface;2],pcurves:[&Curve;2],reversed:[bool;2],max_sine_squared:f64,max_work:usize)->Result<Report> {
    inspect_projective(surfaces,pcurves,reversed,[[1.,1.];2],max_sine_squared,max_work)
}
/// Original forward source-to-canonical Mobius weights; source endpoints
/// must map to the complete canonical interval in the supplied orientation.
pub fn inspect_projective(surfaces:[&Surface;2],pcurves:[&Curve;2],reversed:[bool;2],weights:[[f64;2];2],max_sine_squared:f64,max_work:usize)->Result<Report> {
    check(weights.into_iter().flatten().all(|v|v.is_finite() && v>0.),"Positive finite projective map weights required")?;
    check(max_sine_squared.is_finite() && (0. ..1.).contains(&max_sine_squared) && (1..=1000000).contains(&max_work),"Bound correlated normal work and angle")?;
    for s in surfaces {s.validate()?;check(s.control_points[0][0].len()==3 && s.weights.iter().flatten().all(|w|*w>0.),"Positive XYZ surface weights required")?;}
    for p in pcurves {p.validate()?;check(p.control_points[0].len()==2,"UV curves required")?;}
    let mut w=Work{used:0,max:max_work};let mut out=Report{aligned:None,sine_squared_bounds:None,work:0,reason:"synchronized-boundary-unsupported"};
    let result=(||->Result<Option<I>> {
        let Some(a)=normal(surfaces[0],pcurves[0],reversed[0],&mut w)? else {return Ok(None);};
        let Some(b)=normal(surfaces[1],pcurves[1],reversed[1],&mut w)? else {return Ok(None);};
        let a=projective_normal(a,weights[0],reversed[0],&mut w)?;
        let b=projective_normal(b,weights[1],reversed[1],&mut w)?;
        let numerator=hull(&squared(&cross(&a,&b,&mut w)?,&mut w)?)?;
        let denominator=hull(&squared(&a,&mut w)?)?.mul(hull(&squared(&b,&mut w)?)?)?;
        if denominator.lo<=0. {out.reason="synchronized-boundary-normal-unresolved";return Ok(None);}
        Ok(Some(I::new(numerator.lo.max(0.),numerator.hi.max(0.))?.div(denominator)?))
    })();
    out.work=w.used;
    match result {Ok(Some(sine))=>{out.sine_squared_bounds=Some([sine.lo.max(0.),sine.hi.min(1.)]);out.aligned=if sine.hi<=max_sine_squared {Some(true)}else if sine.lo>max_sine_squared {Some(false)}else{None};out.reason="synchronized-boundary-angle-bounded";},Ok(None)=>{},Err(e) if e.code=="NURBS_NORMAL_WORK_LIMIT"=>out.reason="synchronized-boundary-work-limit",Err(e)=>return Err(e)}
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rational_curved_boundary_keeps_parameter_correlation_and_refuses_oblique_or_singular_normals() {
        let make=|zs:&[f64]|Surface{degree_u:zs.len()-1,degree_v:2,knots_u:[vec![0.;zs.len()],vec![1.;zs.len()]].concat(),knots_v:vec![0.,0.,0.,1.,1.,1.],
            control_points:zs.iter().map(|z|vec![vec![1.,0.,*z],vec![1.,1.,*z],vec![0.,1.,*z]]).collect(),weights:vec![vec![1.,0.5_f64.sqrt(),1.];zs.len()],periodic_u:false,periodic_v:false};
        let a=make(&[0.,1.]);let b=make(&[1.,1.5,2.]);
        let pa=Curve::from_polyline(vec![vec![1.,0.],vec![1.,1.]]).unwrap();let pb=Curve::from_polyline(vec![vec![0.,0.],vec![0.,1.]]).unwrap();
        let r=inspect([&a,&b],[&pa,&pb],[false;2],1e-10,100000).unwrap();assert_eq!(r.aligned,Some(true),"{} {:?}",r.reason,r.sine_squared_bounds);assert!(r.work<=100000);
        assert!(inspect([&a,&b],[&pa,&pb],[false;2],1e-10,1).unwrap().aligned.is_none());
        // Reversing the authored UV traversal and its correspondence must
        // preserve the common physical parameter, including both endpoints.
        let reversed_pb=Curve::from_polyline(vec![vec![0.,1.],vec![0.,0.]]).unwrap();
        let reverse=inspect([&a,&b],[&pa,&reversed_pb],[false,true],1e-10,100000).unwrap();
        assert_eq!(reverse.aligned,Some(true));
        let mut translated_a=a.clone();let mut translated_b=b.clone();
        for surface in [&mut translated_a,&mut translated_b] {
            for point in surface.control_points.iter_mut().flatten() {
                for (coordinate,shift) in point.iter_mut().zip([1024.,-2048.,4096.]) {*coordinate+=shift;}
            }
        }
        let translated=inspect([&translated_a,&translated_b],[&pa,&pb],[false;2],1e-10,100000).unwrap();
        assert_eq!(translated.aligned,Some(true));
        assert!(inspect([&a,&b],[&pa,&pb],[false;2],f64::NAN,100000).is_err());
        assert!(inspect([&a,&b],[&pa,&pb],[false;2],1e-10,0).is_err());
        let partial=Curve::from_polyline(vec![vec![0.,0.25],vec![0.,0.75]]).unwrap();
        let unsupported=inspect([&a,&b],[&pa,&partial],[false;2],1e-10,100000).unwrap();
        assert_eq!(unsupported.aligned,None);
        assert_eq!(unsupported.reason,"synchronized-boundary-unsupported");
        let mut mapped=b.clone();
        for row in &mut mapped.weights {for (i,weight) in row.iter_mut().enumerate() {*weight*=2_f64.powi(i as i32);}}
        let mapped_report=inspect_projective([&a,&mapped],[&pa,&pb],[false;2],[[1.,1.],[1.,2.]],1e-10,100000).unwrap();
        assert_eq!(mapped_report.aligned,Some(true),"{} {:?}",mapped_report.reason,mapped_report.sine_squared_bounds);
        let mapped_reverse=inspect_projective([&a,&mapped],[&pa,&reversed_pb],[false,true],[[1.,1.],[2.,1.]],1e-10,100000).unwrap();
        assert_eq!(mapped_reverse.aligned,Some(true));
        let exhausted=inspect_projective([&a,&mapped],[&pa,&pb],[false;2],[[1.,1.],[1.,2.]],1e-10,1).unwrap();
        assert_eq!(exhausted.aligned,None);assert_eq!(exhausted.work,1);
        assert_eq!(exhausted.reason,"synchronized-boundary-work-limit");
        let wrong_map=inspect_projective([&a,&mapped],[&pa,&pb],[false;2],[[1.,1.];2],1e-10,100000).unwrap();
        assert_ne!(wrong_map.aligned,Some(true));
        assert!(inspect_projective([&a,&mapped],[&pa,&pb],[false;2],[[1.,1.],[0.,2.]],1e-10,100000).is_err());
        let mut oblique=b.clone();for p in &mut oblique.control_points[1] {p[0]+=0.25;}
        let r=inspect([&a,&oblique],[&pa,&pb],[false;2],1e-10,100000).unwrap();assert_ne!(r.aligned,Some(true));
        let singular=make(&[1.,1.,1.]);assert!(inspect([&a,&singular],[&pa,&pb],[false;2],1e-10,100000).unwrap().aligned.is_none());
    }
}
