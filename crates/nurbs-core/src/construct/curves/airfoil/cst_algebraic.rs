//! Direct algebraic CST conversion with an outward control-error enclosure.
use super::*;
use crate::distance_bounds::Interval;

/// Immutable construction result. The bound concerns the mathematical
/// NURBS defined by these binary64 controls, not evaluator roundoff, Gk,
/// regularity, topology, or any previously fitted curve.
#[derive(Clone, Debug)]
pub struct AlgebraicCst {
    curve: Curve,
    source: CstProfile,
    chord: f64,
    max_error: f64,
    work_used: u64,
    parameter_power: usize,
}
impl AlgebraicCst {
    pub fn curve(&self)->&Curve {&self.curve}
    pub fn source(&self)->&CstProfile {&self.source}
    pub fn chord(&self)->f64 {self.chord}
    pub fn max_error(&self)->f64 {self.max_error}
    pub fn work_used(&self)->u64 {self.work_used}
    pub fn parameter_power(&self)->usize {self.parameter_power}
}
struct Work {used:u64,limit:u64}
impl Work {
    fn charge(&mut self)->Result<()> {
        if self.used>=self.limit {return Err(crate::resource("Algebraic CST shared work limit exhausted"));}
        self.used+=1;Ok(())
    }
    fn choose(&mut self,n:usize,k:usize)->Result<u64> {
        let mut v=1u64;
        for j in 0..k.min(n-k) {self.charge()?;v=v*(n-j) as u64/(j+1) as u64;}
        Ok(v)
    }
    fn add(&mut self,a:Interval,b:Interval)->Result<Interval>{self.charge()?;a.add(b)}
    fn mul(&mut self,a:Interval,b:Interval)->Result<Interval>{self.charge()?;a.mul(b)}
    fn div(&mut self,a:Interval,b:Interval)->Result<Interval>{self.charge()?;a.div(b)}
    fn error(&mut self,p:f64,ideal:Interval)->Result<f64>{
        self.charge()?;let e=Interval::point(p).sub(ideal)?;Ok(e.lo.abs().max(e.hi.abs()))
    }
}
fn midpoint(i:Interval)->f64 {(i.lo*0.5+i.hi*0.5).clamp(i.lo,i.hi)}

/// Construct a closed two-span polynomial NURBS directly from a CST law.
/// Supports positive integer/half-integer n1 and integer n2. Uses u=t^p,
/// p=1 for integer n1 and p=2 otherwise. Degree is p*(n1+n2+shape_order)
/// and must not exceed 25; literal constant shape rows reduce to order zero.
/// The upper span reverses t; the complete curve domain is [0,2].
/// max_error encloses continuous nominal position deviation from that
/// original CST law in L1 (therefore also Euclidean) distance.
/// Work units charge each binomial step, coefficient contribution, interval
/// primitive and output control. One shared caller budget is never reset.
pub fn cst_algebraic(profile:&CstProfile,chord:f64,max_work:u64)->Result<AlgebraicCst> {
    check((1..=cad_predicates::MAX_WORK).contains(&max_work),"Algebraic CST work must be in 1..=1000000")?;
    check(chord.is_finite()&&chord>0.&&chord<=1e9,"Algebraic CST chord must respect the existing 1e9 curve-coordinate limit")?;
    let power=if profile.n1.fract()==0. {1usize}else{2usize};
    let m=profile.n1*power as f64;let n=profile.n2;
    check(m.is_finite()&&m>=1.&&m<=25.&&m.fract()==0.&&n.is_finite()&&n>=1.&&n<=24.&&n.fract()==0.,
        "Algebraic CST needs positive half-integer n1 and positive integer n2")?;
    // Reuse the checked parameter contract without evaluating sample sites.
    validate_cst_profile(profile)?;
    // Literal constant Bernstein rows represent a degree-zero shape exactly.
    // Recognize them before choosing a degree, without fitting or averaging.
    let constant=[&profile.upper,&profile.lower].iter().all(|row|row.iter().all(|a|*a==row[0]));
    let (m,n,k)=(m as usize,n as usize,if constant {0}else{profile.upper.len()-1});
    let d=m+power*(n+k);check(d<=25,"Algebraic CST degree exceeds the existing degree-25 limit")?;
    let mut work=Work{used:0,limit:max_work};let mut xs=Vec::with_capacity(d+1);
    for a in 0..=d {
        work.charge()?;
        let x=if a<power {Interval::point(0.)} else if a==d {Interval::point(chord)} else {
            let (numerator,denominator)=if power==1 {(a,d)}else{(a*(a-1),d*(d-1))};
            let ratio=work.div(Interval::point(numerator as f64),Interval::point(denominator as f64))?;
            work.mul(Interval::point(chord),ratio)?
        };xs.push(x);
    }
    let mut rows=Vec::new();let mut max_error=0_f64;
    for raw in [&profile.upper,&profile.lower] {
        let coefficients=if constant {&raw[..1]}else{raw.as_slice()};
        let mut ys=vec![Interval::point(0.);d+1];
        for (i,&a) in coefficients.iter().enumerate() {
            let q=n+k-i;let r=m+power*i;let base=work.choose(k,i)?;
            for j in 0..=if power==1 {0}else{q} {
                work.charge()?;
                if a==0. {continue;}
                // D<=25 implies these integer factors are exact binary64
                // values (and far below u64 overflow).
                let numerator=base*work.choose(q,j)?*(1u64<<j);
                let denominator=work.choose(d,r+j)?;
                let ratio=work.div(Interval::point(numerator as f64),Interval::point(denominator as f64))?;
                let term=work.mul(Interval::point(a),ratio)?;
                ys[r+j]=work.add(ys[r+j],term)?;
            }
        }
        let mut row=Vec::new();
        for a in 0..=d {
            work.charge()?;
            // Endpoint identities follow from m,n>0; preserve closure bits.
            let y=if a==0||a==d {Interval::point(0.)} else {work.mul(Interval::point(chord),ys[a])?};
            let p=[midpoint(xs[a]),midpoint(y),0.];
            let ex=work.error(p[0],xs[a])?;let ey=work.error(p[1],y)?;
            work.charge()?;let e=(ex+ey).next_up();
            crate::numeric(e.is_finite(),"Algebraic CST coefficient error overflowed")?;
            max_error=max_error.max(e);row.push(p.to_vec());
        }
        rows.push(row);
    }
    rows[0].reverse();let mut controls=rows.remove(0);controls.extend(rows.remove(0).into_iter().skip(1));
    let knots=[vec![0.;d+1],vec![1.;d],vec![2.;d+1]].concat();
    let curve=Curve{degree:d,knots,weights:vec![1.;controls.len()],control_points:controls,periodic:false};
    curve.validate()?;
    Ok(AlgebraicCst{curve,source:profile.clone(),chord,max_error,work_used:work.used,parameter_power:power})
}
