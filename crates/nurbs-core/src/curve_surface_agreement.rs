//! Bounded, conservative verification of C(t) = S(P(t)) over a full interval.
//! Positive rational hulls cover every knot span. Exhaustion is inconclusive.
use crate::distance_bounds::{Interval, box_distance};
use crate::{Result, check, curve::Curve, surface::Surface};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    WithinTolerance,
    Mismatch,
    Unresolved,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub status: Status,
    pub cells: usize,
    /// Parameter in the normalized, forward pcurve traversal.
    pub witness: Option<f64>,
    /// Certified distance enclosure at the witness (only for Mismatch).
    pub witness_distance: Option<[f64; 2]>,
}

pub(crate) fn curve_bounds(c: &Curve, t: Interval) -> Result<Vec<Interval>> {
    let mut bounds = vec![[f64::INFINITY, f64::NEG_INFINITY]; c.control_points[0].len()];
    for span in c.degree..c.control_points.len() {
        if c.knots[span] >= c.knots[span + 1] {
            continue;
        }
        let lo = t.lo.max(c.knots[span]);
        let hi = t.hi.min(c.knots[span + 1]);
        if lo > hi {
            continue;
        }
        for (out, b) in bounds.iter_mut().zip(crate::curve_distance::enclosure(
            c,
            span,
            Interval::new(lo, hi)?,
        )?) {
            out[0] = out[0].min(b.lo);
            out[1] = out[1].max(b.hi);
        }
    }
    bounds
        .into_iter()
        .map(|[lo, hi]| Interval::new(lo, hi))
        .collect()
}

pub(crate) fn mapped(c: &Curve, t: Interval, reversed: bool) -> Result<Interval> {
    mapped_range(c.domain(), t, reversed)
}
pub(crate) fn mapped_range(domain: [f64;2], t: Interval, reversed: bool) -> Result<Interval> {
    let [a,b] = domain;
    let t = if reversed {
        Interval::point(1.).sub(t)?.intersect(0., 1.)?
    } else {
        t
    };
    if a == 0. && b == 1. {
        return Ok(t);
    }
    if t.lo == 0. && t.hi == 0. {
        return Ok(Interval::point(a));
    }
    if t.lo == 1. && t.hi == 1. {
        return Ok(Interval::point(b));
    }
    Interval::point(a)
        .add(Interval::point(b).sub(Interval::point(a))?.mul(t)?)?
        .intersect(a, b)
}

fn distance(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    t: Interval,
    reversed: bool,
    source_interval: [f64;2],
) -> Result<Option<(f64, f64)>> {
    let edge = curve_bounds(c, mapped(c, t, reversed)?)?;
    let uv = curve_bounds(p, mapped_range(source_interval, t, false)?)?;
    let domain = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    let Some(us) = crate::periodic_chart::charts(uv[0], domain[0], s.periodic_u)? else {
        return Ok(None);
    };
    let Some(vs) = crate::periodic_chart::charts(uv[1], domain[1], s.periodic_v)? else {
        return Ok(None);
    };
    let mut bounds = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for u in &us {
        for v in &vs {
            let patch = crate::surface_distance::rectangle_bounds(
                s,
                [[u.range.lo, u.range.hi], [v.range.lo, v.range.hi]],
            )?;
            for k in 0..3 {
                bounds[k][0] = bounds[k][0].min(patch[k][0]);
                bounds[k][1] = bounds[k][1].max(patch[k][1]);
            }
        }
    }
    let surface = bounds
        .into_iter()
        .map(|[lo, hi]| Interval::new(lo, hi))
        .collect::<Result<Vec<_>>>()?;
    Ok(Some(box_distance(&edge, &surface)?))
}

/// Covers the natural domains of both curves using a common normalized
/// parameter. Reversal applies to the 3D curve, as for a reversed coedge.
/// Periodic surface axes use their periodic extension for lifted UV values.
/// Both chart images remain covered when a parameter interval crosses a seam.
pub fn verify(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    reversed: bool,
    tolerance: f64,
    max_cells: usize,
) -> Result<Report> {
    p.validate()?;
    verify_on(c,p,s,reversed,p.domain(),tolerance,max_cells)
}
/// Full normalized world-curve traversal against an unchanged original pcurve
/// restricted only by an explicit source parameter interval. No trim proposal
/// or rounded extracted pcurve replaces the source definition.
pub fn verify_on(
    c: &Curve, p: &Curve, s: &Surface, reversed: bool,
    source_interval: [f64;2], tolerance: f64, max_cells: usize,
) -> Result<Report> {
    c.validate()?;
    p.validate()?;
    s.validate()?;
    check(
        c.control_points[0].len() == 3 && p.control_points[0].len() == 2,
        "Agreement needs a 3D curve and 2D pcurve",
    )?;
    check(
        tolerance.is_finite() && tolerance > 0.,
        "Agreement tolerance must be positive",
    )?;
    check(
        max_cells > 0 && max_cells <= 100_000,
        "Agreement cell budget must be in 1..100000",
    )?;
    check(source_interval.iter().all(|x|x.is_finite()) && source_interval[0]<source_interval[1]
        && source_interval[0]>=p.domain()[0] && source_interval[1]<=p.domain()[1],
        "Source pcurve interval must have positive width inside its original domain")?;
    let mut report = Report {
        status: Status::Unresolved,
        cells: 0,
        witness: None,
        witness_distance: None,
    };
    // Keep the common parameter correlated before trying Cartesian boxes.
    // An unavailable or numerically inconclusive optional bound falls back to
    // the full-span interval traversal; it never turns into a success.
    let complete_bound = if source_interval==p.domain() {
        crate::curve_surface_composition::upper(c,p,s,reversed)
    } else {
        crate::curve_surface_composition::upper_cell_on(c,p,s,reversed,[0.,1.],source_interval)
    };
    if let Ok(Some(upper)) = complete_bound {
        if upper <= tolerance {
            report.status = Status::WithinTolerance;
            report.cells = 1;
            return Ok(report);
        }
    }
    let mut stack = vec![[0., 1.]];
    while let Some([lo, hi]) = stack.pop() {
        if report.cells == max_cells {
            return Ok(report);
        }
        report.cells += 1;
        let cell_bound = if source_interval==p.domain() {
            crate::curve_surface_composition::upper_cell(c,p,s,reversed,[lo,hi])
        } else {
            crate::curve_surface_composition::upper_cell_on(c,p,s,reversed,[lo,hi],source_interval)
        };
        if let Ok(Some(upper)) = cell_bound {
            if upper <= tolerance {
                continue;
            }
        }
        if let Some((_, upper)) = distance(c, p, s, Interval::new(lo, hi)?, reversed, source_interval)? {
            if upper <= tolerance {
                continue;
            }
        }
        let mid = lo + (hi - lo) * 0.5;
        if let Some((lower, upper)) = distance(c, p, s, Interval::point(mid), reversed, source_interval)? {
            if lower > tolerance {
                report.status = Status::Mismatch;
                report.witness = Some(mid);
                report.witness_distance = Some([lower, upper]);
                return Ok(report);
            }
        }
        if mid == lo || mid == hi {
            return Ok(report);
        }
        stack.push([mid, hi]);
        stack.push([lo, mid]);
    }
    report.status = Status::WithinTolerance;
    Ok(report)
}

/// Exact identity on one Bezier chart, over normalized curve traversal.
/// None means this representation or its chart-domain inclusion is unproven.
/// A returned decision distinguishes exact equality, difference and work limits.
pub fn verify_exact(c: &Curve, p: &Curve, s: &Surface, reversed: bool, max_work: u64)
    -> Result<Option<cad_predicates::BezierIdentityDecision>> {
    verify_exact_impl(c,p,s,reversed,max_work,true)
}
/// Homogeneous cross-product identity of a formal Bezier composition. This
/// report proves neither denominator positivity nor source chart membership.
/// A geometry caller must independently prove the actual restriction in-chart.
pub struct AlgebraicCompositionIdentity {
    pub outcome: cad_predicates::BezierIdentity,
    pub work_used: u64,
    pub context: cad_predicates::ContextIdentity,
}
pub fn verify_exact_algebraic(c:&Curve,p:&Curve,s:&Surface,reversed:bool,max_work:u64)
    -> Result<Option<AlgebraicCompositionIdentity>> {
    Ok(verify_exact_impl(c,p,s,reversed,max_work,false)?.map(|r|AlgebraicCompositionIdentity {
        outcome:r.outcome,work_used:r.work_used,context:r.context,
    }))
}
fn verify_exact_impl(c:&Curve,p:&Curve,s:&Surface,reversed:bool,max_work:u64,require_chart:bool)
    -> Result<Option<cad_predicates::BezierIdentityDecision>> {
    use cad_predicates::{AuthoredScalar, Limits, PredicateContext, SourceArena, ToleranceContext};
    c.validate()?;p.validate()?;s.validate()?;
    check(c.control_points[0].len()==3 && p.control_points[0].len()==2,"Agreement needs a 3D curve and 2D pcurve")?;
    let bezier=|knots:&[f64],degree:usize,n:usize| n==degree+1
        && knots[..=degree].iter().all(|x|*x==knots[degree])
        && knots[n..].iter().all(|x|*x==knots[n]);
    if !bezier(&c.knots,c.degree,c.control_points.len())
        || !bezier(&s.knots_u,s.degree_u,s.control_points.len())
        || !bezier(&s.knots_v,s.degree_v,s.control_points[0].len()) {
        if let Some(identity)=exact_natural_nurbs_boundary(c,p,s,reversed,max_work)? {return Ok(Some(identity));}
    }
    if c.periodic || p.periodic || s.periodic_u || s.periodic_v
        || c.degree>32 || p.degree>8 || s.degree_u>8 || s.degree_v>8
        || !bezier(&c.knots,c.degree,c.control_points.len())
        || !bezier(&p.knots,p.degree,p.control_points.len())
        || !bezier(&s.knots_u,s.degree_u,s.control_points.len())
        || !bezier(&s.knots_v,s.degree_v,s.control_points[0].len()) {return Ok(None);}
    let domain=[[s.knots_u[s.degree_u],s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v],s.knots_v[s.control_points[0].len()]]];
    // Positive rational weights put the entire pcurve in its control hull.
    if require_chart && p.control_points.iter().any(|v|(0..2).any(|k|v[k]<domain[k][0]||v[k]>domain[k][1])) {return Ok(None);}
    if p.degree==1&&p.weights[0]==p.weights[1] {
        for fixed in 0..2 {for end in 0..2 {
            let free=1-fixed;let a=&p.control_points[0];let b=&p.control_points[1];
            if a[fixed]!=domain[fixed][end]||b[fixed]!=a[fixed]{continue;}
            let forward=a[free]==domain[free][0]&&b[free]==domain[free][1];
            let backward=a[free]==domain[free][1]&&b[free]==domain[free][0];
            if !forward&&!backward{continue;}
            let size=[s.control_points.len(),s.control_points[0].len()];
            let row=if end==0{0}else{size[fixed]-1};let count=size[free];
            let mut values=Vec::new();
            for i in 0..c.control_points.len(){let i=if reversed{c.control_points.len()-1-i}else{i};values.extend(c.control_points[i].iter().copied());values.push(c.weights[i]);}
            for i in 0..count {
                let i=if backward{count-1-i}else{i};let (u,v)=if fixed==0{(row,i)}else{(i,row)};
                values.extend(s.control_points[u][v].iter().copied());values.push(s.weights[u][v]);
            }
            let source=SourceArena::authored("natural-bezier-boundary",1,values.into_iter().map(|v|AuthoredScalar::Binary64Bits(v.to_bits())).collect()).map_err(|_|crate::Error::new("NURBS_INVALID_INPUT","Invalid natural boundary source"))?;
            let mut index=0;let mut leaf=||{let r=source.leaf(index).unwrap();index+=1;r};
            let first:Vec<_>=(0..c.control_points.len()).map(|_|std::array::from_fn(|_|leaf())).collect();
            let second:Vec<_>=(0..count).map(|_|std::array::from_fn(|_|leaf())).collect();
            let tolerance=ToleranceContext::default_valid();let mut ctx=PredicateContext::new(&source,&tolerance,Limits{max_work,..Limits::default()},None);
            return cad_predicates::rational_bezier_identity(&mut ctx,&first,&second).map(Some).map_err(|_|crate::Error::new("NURBS_INVALID_INPUT","Invalid natural boundary identity request"));
        }}
    }
    // A coordinate identity plane needs no polynomial surface composition.
    // Recognize the exact authored chart, then lift pcurve controls by copying
    // original numbers only. No fitted plane or rounded affine inverse is used.
    if s.degree_u==1&&s.degree_v==1&&s.weights.iter().flatten().all(|w|*w==s.weights[0][0]) {
        for axes in [[0,1],[0,2],[1,0],[1,2],[2,0],[2,1]] {
            let fixed=3-axes[0]-axes[1];let height=s.control_points[0][0][fixed];
            let identity=(0..2).all(|u|(0..2).all(|v| {
                let point=&s.control_points[u][v];
                point[axes[0]]==domain[0][u]&&point[axes[1]]==domain[1][v]&&point[fixed]==height
            }));
            if !identity{continue;}
            let mut values=Vec::new();
            for i in 0..c.control_points.len(){let i=if reversed{c.control_points.len()-1-i}else{i};values.extend(c.control_points[i].iter().copied());values.push(c.weights[i]);}
            for (point,weight) in p.control_points.iter().zip(&p.weights){
                let mut lifted=[height;3];lifted[axes[0]]=point[0];lifted[axes[1]]=point[1];values.extend(lifted);values.push(*weight);
            }
            let source=SourceArena::authored("identity-plane-boundary",1,values.into_iter().map(|v|AuthoredScalar::Binary64Bits(v.to_bits())).collect()).map_err(|_|crate::Error::new("NURBS_INVALID_INPUT","Invalid identity plane source"))?;
            let mut index=0;let mut leaf=||{let r=source.leaf(index).unwrap();index+=1;r};
            let first:Vec<_>=(0..c.control_points.len()).map(|_|std::array::from_fn(|_|leaf())).collect();
            let second:Vec<_>=(0..p.control_points.len()).map(|_|std::array::from_fn(|_|leaf())).collect();
            let tolerance=ToleranceContext::default_valid();let mut ctx=PredicateContext::new(&source,&tolerance,Limits{max_work,..Limits::default()},None);
            return cad_predicates::rational_bezier_identity(&mut ctx,&first,&second).map(Some).map_err(|_|crate::Error::new("NURBS_INVALID_INPUT","Invalid identity plane boundary request"));
        }
    }
    let mut values=Vec::new();
    for i in 0..c.control_points.len() {
        let i=if reversed {c.control_points.len()-1-i}else{i};
        values.extend(c.control_points[i].iter().copied());values.push(c.weights[i]);
    }
    for (v,w) in p.control_points.iter().zip(&p.weights){values.extend(v.iter().copied());values.push(*w);}
    for (row,weights) in s.control_points.iter().zip(&s.weights){for (v,w) in row.iter().zip(weights){values.extend(v.iter().copied());values.push(*w);}}
    values.extend(domain.into_iter().flatten());
    let source=SourceArena::authored("curve-surface-agreement",1,values.into_iter().map(|v|AuthoredScalar::Binary64Bits(v.to_bits())).collect())
        .map_err(|_|crate::Error::new("NURBS_INVALID_INPUT","Invalid authored composition source"))?;
    let mut index=0;let mut leaf=||{let r=source.leaf(index).unwrap();index+=1;r};
    let cc:Vec<_>=(0..c.control_points.len()).map(|_|std::array::from_fn(|_|leaf())).collect();
    let pp:Vec<_>=(0..p.control_points.len()).map(|_|std::array::from_fn(|_|leaf())).collect();
    let ss:Vec<Vec<_>>=s.control_points.iter().map(|row|row.iter().map(|_|std::array::from_fn(|_|leaf())).collect()).collect();
    let dd=std::array::from_fn(|_|std::array::from_fn(|_|leaf()));
    let tolerance=ToleranceContext::default_valid();
    let mut ctx=PredicateContext::new(&source,&tolerance,Limits{max_work,..Limits::default()},None);
    // A constant-weight linear pcurve spanning one full tensor boundary is
    // exactly normalized affine traversal of that boundary Bezier curve.
    // Keep the complete immutable source arena/context and shared work limit;
    // compare authored boundary controls, never a rounded extracted curve.
    if p.degree==1 && p.weights[0]==p.weights[1] {
        let a=&p.control_points[0];let b=&p.control_points[1];
        for axis in 0..2 {
            let fixed=1-axis;
            if a[fixed]!=b[fixed] {continue;}
            let at=if a[fixed]==domain[fixed][0] {0}
                else if a[fixed]==domain[fixed][1] {if fixed==0 {ss.len()-1} else {ss[0].len()-1}}
                else {continue;};
            let forward=a[axis]==domain[axis][0] && b[axis]==domain[axis][1];
            let backward=a[axis]==domain[axis][1] && b[axis]==domain[axis][0];
            let degree=if axis==0 {s.degree_u} else {s.degree_v};
            if (!forward && !backward) || c.degree!=degree {continue;}
            let mut boundary=if axis==0 {ss.iter().map(|row|row[at]).collect::<Vec<_>>()}
                else {ss[at].clone()};
            if backward {boundary.reverse();}
            return cad_predicates::rational_bezier_identity(&mut ctx,&cc,&boundary)
                .map(Some).map_err(|_|crate::Error::new("NURBS_INVALID_INPUT","Invalid tensor boundary identity request"));
        }
    }
    cad_predicates::rational_bezier_composition_identity(&mut ctx,&cc,&pp,&ss,dd)
        .map(Some).map_err(|_|crate::Error::new("NURBS_INVALID_INPUT","Invalid composition identity request"))
}
// A clamped tensor endpoint restricts exactly to its original control row.
// Matching the complete univariate knot basis and authored controls/weights
// proves a multi-span trace, without rounded knot insertion or composition.
// The subsequent predicate sees identical coefficient sequences: its Bezier
// identity is used only to mint the immutable source/context and charge exact
// work, never to equate different NURBS coefficient sequences.
fn exact_natural_nurbs_boundary(c:&Curve,p:&Curve,s:&Surface,reversed:bool,max_work:u64)
    -> Result<Option<cad_predicates::BezierIdentityDecision>> {
    use cad_predicates::{AuthoredScalar,Limits,PredicateContext,SourceArena,ToleranceContext};
    let clamped=|k:&[f64],d:usize,n:usize|k[..=d].iter().all(|x|*x==k[d])
        && k[n..].iter().all(|x|*x==k[n])&&k[d+1]>k[d]&&k[n-1]<k[n];
    let continuous=|k:&[f64],d:usize,n:usize|{
        let mut i=d+1;
        while i<n {let mut end=i+1;while end<n&&k[end]==k[i]{end+=1;}
            if k[i]>k[d]&&k[i]<k[n]&&end-i>d{return false;}i=end;}
        true
    };
    if c.periodic||p.periodic||s.periodic_u||s.periodic_v||p.degree!=1
        ||p.control_points.len()!=2||p.weights[0]!=p.weights[1]
        ||p.knots!=[0.,0.,1.,1.]||c.domain()!=[0.,1.]
        ||c.control_points.len()>33||s.degree_u>8||s.degree_v>8
        ||!clamped(&c.knots,c.degree,c.control_points.len())
        ||!clamped(&s.knots_u,s.degree_u,s.control_points.len())
        ||!clamped(&s.knots_v,s.degree_v,s.control_points[0].len())
        ||!continuous(&s.knots_u,s.degree_u,s.control_points.len())
        ||!continuous(&s.knots_v,s.degree_v,s.control_points[0].len()) {return Ok(None);}
    let sizes=[s.control_points.len(),s.control_points[0].len()];
    let degrees=[s.degree_u,s.degree_v];let knots=[&s.knots_u,&s.knots_v];
    for fixed in 0..2 {for end in 0..2 {
        let free=1-fixed;let a=&p.control_points[0];let b=&p.control_points[1];
        if a[fixed]!=knots[fixed][if end==0{degrees[fixed]}else{sizes[fixed]}]
            ||b[fixed]!=a[fixed]||knots[free][degrees[free]]!=0.||knots[free][sizes[free]]!=1.
            ||c.degree!=degrees[free]||c.control_points.len()!=sizes[free] {continue;}
        let backward=if a[free]==0.&&b[free]==1.{false}
            else if a[free]==1.&&b[free]==0.{true}else{continue};
        let mirror=reversed!=backward;
        if c.knots.len()!=knots[free].len() {continue;}
        let same_basis=c.knots.iter().enumerate().all(|(i,&x)|{
            if !mirror{return x==knots[free][i];}
            let y=knots[free][c.knots.len()-1-i];
            // TwoSum residual: rounded x+y==1 alone is not exact mirroring.
            let sum=x+y;let vy=sum-x;
            sum==1.&&(x-(sum-vy))+(y-vy)==0.
        });
        if !same_basis {continue;}
        let row=if end==0{0}else{sizes[fixed]-1};
        let indices=|i|if fixed==0{(row,i)}else{(i,row)};
        let same_controls=(0..sizes[free]).all(|i|{
            let ci=if mirror{sizes[free]-1-i}else{i};let(u,v)=indices(i);
            c.control_points[ci]==s.control_points[u][v]&&c.weights[ci]==s.weights[u][v]
        });
        if !same_controls {continue;}
        let mut values=Vec::new();
        for i in 0..sizes[free] {let ci=if mirror{sizes[free]-1-i}else{i};
            values.extend(c.control_points[ci].iter().copied());values.push(c.weights[ci]);}
        for i in 0..sizes[free] {let(u,v)=indices(i);values.extend(s.control_points[u][v].iter().copied());values.push(s.weights[u][v]);}
        // Bind the complete basis and traversal to the proof context as well.
        values.extend(c.knots.iter().copied());values.extend(s.knots_u.iter().copied());values.extend(s.knots_v.iter().copied());
        values.extend(p.knots.iter().copied());values.extend(p.control_points.iter().flatten().copied());values.extend(p.weights.iter().copied());
        values.extend([c.degree as f64,s.degree_u as f64,s.degree_v as f64,reversed as u8 as f64]);
        let source=SourceArena::authored("natural-multispan-nurbs-boundary",1,values.into_iter().map(|v|AuthoredScalar::Binary64Bits(v.to_bits())).collect())
            .map_err(|_|crate::Error::new("NURBS_INVALID_INPUT","Invalid NURBS boundary source"))?;
        let mut index=0;let mut leaf=||{let r=source.leaf(index).unwrap();index+=1;r};
        let first:Vec<_>=(0..sizes[free]).map(|_|std::array::from_fn(|_|leaf())).collect();
        let second:Vec<_>=(0..sizes[free]).map(|_|std::array::from_fn(|_|leaf())).collect();
        let tolerance=ToleranceContext::default_valid();let mut ctx=PredicateContext::new(&source,&tolerance,Limits{max_work,..Limits::default()},None);
        return cad_predicates::rational_bezier_identity(&mut ctx,&first,&second).map(Some)
            .map_err(|_|crate::Error::new("NURBS_INVALID_INPUT","Invalid NURBS boundary identity request"));
    }}
    Ok(None)
}
#[cfg(test)]
#[path="tests/curve_surface_agreement_tests.rs"]
mod tests;
