//! surface injectivity algorithm: normal_direction_bounds_detailed.
use super::*;
/// Same conservative rectangle query, retaining all charged span work.
pub fn normal_direction_bounds_detailed(s: &Surface, domain: [[f64;2];2], direction: [f64;3], max_spans: usize) -> Result<NormalDirectionReport> {
    s.validate()?;
    let knots = [&s.knots_u,&s.knots_v];
    let degrees = [s.degree_u,s.degree_v];
    let counts = [s.control_points.len(),s.control_points[0].len()];
    check(max_spans > 0 && max_spans <= 100000 && direction.iter().all(|x|x.is_finite()), "Normal bounds require a finite direction and bounded positive span budget")?;
    for axis in 0..2 {
        let d=domain[axis];
        check(d.iter().all(|x|x.is_finite()) && d[0]<=d[1] && d[0]>=knots[axis][degrees[axis]] && d[1]<=knots[axis][counts[axis]], "Normal rectangle must be inside the natural surface domain")?;
    }
    let mut spans = 0;
    let mut bounds = [f64::INFINITY,f64::NEG_INFINITY];
    for u in crate::certificates::audit::nonempty_spans(knots[0],degrees[0],counts[0]) { for v in crate::certificates::audit::nonempty_spans(knots[1],degrees[1],counts[1]) {
        let indices=[u,v];
        let mut section=[[0.;2];2];
        let mut outside=false;
        for axis in 0..2 {
            let k=indices[axis];let lo=knots[axis][k];let hi=knots[axis][k+1];
            if domain[axis][1]<lo || domain[axis][0]>hi {outside=true;break;}
            section[axis]=[lo.max(domain[axis][0]),hi.min(domain[axis][1])];
            if section[axis][0]==section[axis][1] {section[axis]=[lo,hi];}
        }
        if outside {continue;}
        if spans==max_spans {return Ok(NormalDirectionReport{bounds:None,spans});}
        spans+=1;
        let j=section_jacobian(s,indices,section)?;
        let mut dot=I::point(0.);
        for axis in 0..3 {
            let b=(axis+1)%3;let c=(axis+2)%3;
            let component=j[b][0].mul(j[c][1])?.sub(j[c][0].mul(j[b][1])?)?;
            dot=dot.add(component.mul(I::point(direction[axis]))?)?;
        }
        bounds[0]=bounds[0].min(dot.lo);bounds[1]=bounds[1].max(dot.hi);
    }}
    Ok(NormalDirectionReport{bounds:Some(bounds),spans})
}
