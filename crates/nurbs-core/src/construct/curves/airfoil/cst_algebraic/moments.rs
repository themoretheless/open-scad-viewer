//! Original-source area moments; no integration of a fitted control polygon.
use super::*;

/// Area moments in the canonical chord XY frame. `xx` integrates y²,
/// `yy` integrates x², `xy` integrates xy (positive product convention).
/// The second row is about the enclosed centroid. Units are chord units^4.
#[derive(Clone, Debug)]
pub struct CstSectionMoments {
    pub about_origin: [[f64; 2]; 3],
    pub about_centroid: [[f64; 2]; 3],
    pub work_used: u64,
}

// Unlike the degree-50 first-moment helper, these products reach degree75.
// Enclose binomials by interval recurrence: no u64 overflow or rounded large
// integer may be treated as an exact combinatorial coefficient.
fn choose(n: usize, k: usize, work: &mut Work) -> Result<Interval> {
    let mut value = Interval::point(1.);
    for j in 0..k.min(n-k) {
        value = work.mul(value, Interval::point((n-j) as f64))?;
        value = work.div(value, Interval::point((j+1) as f64))?;
    }
    Ok(value)
}
fn multiply(a: &[Interval], b: &[Interval], work: &mut Work) -> Result<Vec<Interval>> {
    let (n,m)=(a.len()-1,b.len()-1);
    let mut out=vec![Interval::point(0.);n+m+1];
    for (i,&x) in a.iter().enumerate() { for (j,&y) in b.iter().enumerate() {
        let ca=choose(n,i,work)?;let cb=choose(m,j,work)?;
        let denominator=choose(n+m,i+j,work)?;
        let numerator=work.mul(ca,cb)?;let ratio=work.div(numerator,denominator)?;
        let xy=work.mul(x,y)?;let term=work.mul(xy,ratio)?;
        out[i+j]=work.add(out[i+j],term)?;
    }}
    Ok(out)
}

/// Continuous enclosures of original CST second area moments. Same algebraic
/// subset and sufficient coefficientwise ordering as section properties.
/// One caller arithmetic ledger covers all construction and integration.
/// No fitted-NURBS, frame transform, solid inertia or aerodynamic authority.
pub fn cst_section_moments(model: &AlgebraicCst, max_work: u64) -> Result<CstSectionMoments> {
    check((1..=cad_predicates::MAX_WORK).contains(&max_work), "CST moment work must be in 1..=1000000")?;
    let profile=&model.source;
    check(profile.upper.iter().zip(&profile.lower).all(|(a,b)|a>=b)
        && profile.upper.iter().zip(&profile.lower).any(|(a,b)|a>b),
        "CST moments require coefficientwise ordered, nonzero thickness")?;
    let mut work=Work{used:0,limit:max_work};
    let power=model.parameter_power;let d=model.curve.degree;
    let m=(profile.n1*power as f64) as usize;let n=profile.n2 as usize;
    let k=(d-m)/power-n;
    let mut differences=Vec::new();let mut sums=Vec::new();
    for (&a,&b) in profile.upper.iter().zip(&profile.lower).take(k+1) {
        work.charge()?;differences.push(Interval::point(a).sub(Interval::point(b))?);
        sums.push(work.add(Interval::point(a),Interval::point(b))?);
    }
    let h=ordinate_controls(&differences,m,n,k,power,d,&mut work)?;
    let s=ordinate_controls(&sums,m,n,k,power,d,&mut work)?;
    let hs=multiply(&h,&s,&mut work)?;
    let h2=multiply(&h,&h,&mut work)?;let s2=multiply(&s,&s,&mut work)?;
    let mut q=Vec::new();
    for (a,b) in h2.into_iter().zip(s2) {
        let three_s2=work.mul(Interval::point(3.),b)?;
        q.push(work.add(a,three_s2)?);
    }
    let cube=multiply(&h,&q,&mut work)?;
    let area=weighted_integral(&h,power-1,&mut work)?;
    crate::numeric(area.lo>0., "CST moment area not separated from zero")?;
    let mx=weighted_integral(&h,2*power-1,&mut work)?;
    let my=weighted_integral(&hs,power-1,&mut work)?;
    let my=work.mul(my,Interval::point(0.5))?;
    let xx=weighted_integral(&cube,power-1,&mut work)?;
    let xx=work.div(xx,Interval::point(12.))?;
    let yy=weighted_integral(&h,3*power-1,&mut work)?;
    let xy=weighted_integral(&hs,2*power-1,&mut work)?;
    let xy=work.mul(xy,Interval::point(0.5))?;
    let chord=Interval::point(model.chord);let c2=work.mul(chord,chord)?;
    let c4=work.mul(c2,c2)?;let scale=work.mul(c4,Interval::point(power as f64))?;
    crate::numeric(scale.lo>0., "CST moment physical scale underflow")?;
    let raw=[xx,yy,xy];let factors=[(my,my),(mx,mx),(mx,my)];
    let mut origin=[[0.;2];3];let mut central=[[0.;2];3];
    for i in 0..3 {
        let first=work.mul(factors[i].0,factors[i].1)?;
        let correction=work.div(first,area)?;
        work.charge()?;let centered=raw[i].sub(correction)?;
        let value=work.mul(raw[i],scale)?;origin[i]=[value.lo,value.hi];
        let value=work.mul(centered,scale)?;central[i]=[value.lo,value.hi];
    }
    Ok(CstSectionMoments{about_origin:origin,about_centroid:central,work_used:work.used})
}
