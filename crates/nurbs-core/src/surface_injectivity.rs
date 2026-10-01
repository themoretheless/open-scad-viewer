//! Sufficient global injectivity certificate for a complete NURBS chart.
//! If a fixed projection F and constant Y satisfy ||I-Y DF||_inf < 1 over
//! the entire convex parameter rectangle, F(x)=F(y) implies x=y. Every knot
//! rectangle contributes to the same Jacobian hull; local success is not enough.
use crate::{Result, check, distance_bounds::Interval as I, surface::Surface};
#[derive(Clone, Debug)]
pub struct Report {
    pub proven: bool,
    pub projection: Option<[usize; 2]>,
    pub linear_projection: Option<[[f64;3];2]>,
    pub contraction_upper: Option<f64>,
    pub spans: usize,
    pub reason: &'static str,
}
fn hull(values: impl Iterator<Item = I>) -> I {
    values.fold(
        I {
            lo: f64::INFINITY,
            hi: f64::NEG_INFINITY,
        },
        |a, b| I {
            lo: a.lo.min(b.lo),
            hi: a.hi.max(b.hi),
        },
    )
}
fn jacobian(s: &Surface, span: [usize; 2]) -> Result<[[I; 2]; 3]> {
    section_jacobian(
        s,
        span,
        [
            [s.knots_u[span[0]], s.knots_u[span[0] + 1]],
            [s.knots_v[span[1]], s.knots_v[span[1] + 1]],
        ],
    )
}
/// Derivatives of the restricted section. A collapsed parameter axis is fixed
/// and its column is zero; it is not a bound on transverse surface derivatives.
pub(crate) fn section_jacobian(
    s: &Surface,
    span: [usize; 2],
    domain: [[f64; 2]; 2],
) -> Result<[[I; 2]; 3]> {
    let (p, q) = (s.degree_u, s.degree_v);
    let mut net = crate::curve_surface_composition::surface_net_on(s, span, domain)?;
    // Translation does not change derivatives. Reducing the numerator's
    // magnitude improves the rational quotient bound without changing geometry.
    let origin = &s.control_points[span[0] - p][span[1] - q];
    for row in &mut net {
        for h in row {
            for k in 0..3 {
                h[k] = h[k].sub(I::point(origin[k]).mul(h[3])?)?;
            }
        }
    }
    let values: [I; 4] = std::array::from_fn(|k| hull(net.iter().flatten().map(|h| h[k])));
    let denominator = values[3].mul(values[3])?;
    let mut result = [[I::point(0.); 2]; 3];
    for axis in 0..2 {
        let degree = [p, q][axis];
        if domain[axis][0] == domain[axis][1] {
            continue;
        }
        let width = I::point(domain[axis][1]).sub(I::point(domain[axis][0]))?;
        let mut diffs = Vec::new();
        for i in 0..=p {
            for j in 0..=q {
                if (axis == 0 && i == p) || (axis == 1 && j == q) {
                    continue;
                }
                let a = net[i][j];
                let b = net[i + usize::from(axis == 0)][j + usize::from(axis == 1)];
                let mut d = [I::point(0.); 4];
                for k in 0..4 {
                    d[k] = b[k].sub(a[k])?.mul(I::point(degree as f64))?.div(width)?;
                }
                diffs.push(d);
            }
        }
        let d: [I; 4] = std::array::from_fn(|k| hull(diffs.iter().map(|h| h[k])));
        for k in 0..3 {
            result[k][axis] = d[k]
                .mul(values[3])?
                .sub(values[k].mul(d[3])?)?
                .div(denominator)?;
        }
    }
    Ok(result)
}
fn contraction(j: [[I; 2]; 2]) -> Result<Option<f64>> {
    let a = j.map(|row| row.map(|v| v.lo * 0.5 + v.hi * 0.5));
    let det = a[0][0] * a[1][1] - a[0][1] * a[1][0];
    if !det.is_finite() || det == 0. {
        return Ok(None);
    }
    let y = [
        [a[1][1] / det, -a[0][1] / det],
        [-a[1][0] / det, a[0][0] / det],
    ];
    if y.iter().flatten().any(|x| !x.is_finite()) {
        return Ok(None);
    }
    let mut maximum = 0_f64;
    for i in 0..2 {
        let mut row = I::point(0.);
        for k in 0..2 {
            let product = I::point(y[i][0])
                .mul(j[0][k])?
                .add(I::point(y[i][1]).mul(j[1][k])?)?;
            let residual = I::point(if i == k { 1. } else { 0. }).sub(product)?;
            row = row.add(I::point(residual.lo.abs().max(residual.hi.abs())))?;
        }
        maximum = maximum.max(row.hi);
    }
    Ok(Some(maximum))
}
/// Failure of this sufficient condition is unresolved, never evidence of an
/// intersection. This certifies the whole natural chart, hence any trimmed
/// subset, but does not compare it with other faces or certify a whole solid.
pub fn certify(s: &Surface, max_spans: usize) -> Result<Report> {
    s.validate()?;
    check(
        max_spans > 0 && max_spans <= 100_000,
        "Injectivity span budget must be in 1..100000",
    )?;
    let mut report = Report {
        proven: false,
        projection: None,
        linear_projection: None,
        contraction_upper: None,
        spans: 0,
        reason: "projection-not-proven",
    };
    if s.periodic_u || s.periodic_v {
        report.reason = "periodic-domain";
        return Ok(report);
    }
    let mut j = [[I {
        lo: f64::INFINITY,
        hi: f64::NEG_INFINITY,
    }; 2]; 3];
    for u in s.degree_u..s.control_points.len() {
        for v in s.degree_v..s.control_points[0].len() {
            if s.knots_u[u] >= s.knots_u[u + 1] || s.knots_v[v] >= s.knots_v[v + 1] {
                continue;
            }
            if report.spans == max_spans {
                report.reason = "work-limit";
                return Ok(report);
            }
            let next = jacobian(s, [u, v])?;
            report.spans += 1;
            for k in 0..3 {
                for axis in 0..2 {
                    j[k][axis] = hull([j[k][axis], next[k][axis]].into_iter());
                }
            }
        }
    }
    for axes in [[0, 1], [0, 2], [1, 2]] {
        if let Some(q) = contraction([j[axes[0]], j[axes[1]]])? {
            if q < 1. {
                report.proven = true;
                report.projection = Some(axes);
                report.contraction_upper = Some(q);
                report.reason = "global-projection-contraction";
                return Ok(report);
            }
        }
    }
    for basis in LINEAR_PROJECTIONS{
        if report.spans==max_spans{report.reason="work-limit";return Ok(report);}
        let (q,cells)=linear_projection_work(s,basis,max_spans-report.spans)?;
        report.spans+=cells;
        if let Some(q)=q{
            report.proven=true;report.linear_projection=Some(basis);report.contraction_upper=Some(q);
            report.reason="global-linear-projection-contraction";return Ok(report);
        }
    }
    if report.spans==max_spans{report.reason="work-limit";}
    Ok(report)
}
pub const LINEAR_PROJECTIONS: [[[f64;3];2];6]=[
    [[1.,1.,0.],[0.,0.,1.]],[[1.,-1.,0.],[0.,0.,1.]],
    [[1.,0.,1.],[0.,1.,0.]],[[1.,0.,-1.],[0.,1.,0.]],
    [[0.,1.,1.],[1.,0.,0.]],[[0.,1.,-1.],[1.,0.,0.]],
];
/// Certify a fixed linear projection using a common Jacobian hull over every
/// knot rectangle. Subdivision tightens bounds; it never replaces the global
/// contraction with unrelated local certificates. None includes budget exhaustion.
pub fn certify_linear_projection(s:&Surface,basis:[[f64;3];2],max_cells:usize)->Result<Option<f64>> {
    Ok(linear_projection_work(s,basis,max_cells)?.0)
}
/// A fixed perspective projection (two affine numerators divided by one
/// affine denominator). All coefficients refer to [x,y,z,1]. The denominator
/// must stay strictly positive over every covered section. A single global
/// Jacobian contraction proves injectivity of the original chart; no rounded
/// transformed control points or sampled denominator admission are used.
pub fn certify_projective_projection(s:&Surface,numerators:[[f64;4];2],denominator:[f64;4],max_cells:usize)->Result<Option<f64>>{
    s.validate()?;
    check(max_cells>0&&max_cells<=100000&&numerators.iter().flatten().chain(denominator.iter()).all(|x|x.is_finite()),"Projective projection requires finite coefficients and 1..100000 cells")?;
    if s.periodic_u||s.periodic_v{return Ok(None);}
    let mut global=[[I{lo:f64::INFINITY,hi:f64::NEG_INFINITY};2];2];
    let mut cells=0;
    for u in s.degree_u..s.control_points.len(){for v in s.degree_v..s.control_points[0].len(){
        let ranges=[[s.knots_u[u],s.knots_u[u+1]],[s.knots_v[v],s.knots_v[v+1]]];
        if ranges.iter().any(|r|r[0]>=r[1]){continue;}
        for i in 0..4{for j in 0..4{
            if cells==max_cells{return Ok(None);}
            let section=std::array::from_fn(|k|{let n=[i,j][k];let [lo,hi]=ranges[k];
                [if n==0{lo}else{lo+(hi-lo)*(n as f64/4.)},if n==3{hi}else{lo+(hi-lo)*((n+1) as f64/4.)}]});
            cells+=1;
            let Some(jac)=projective_section_jacobian(s,[u,v],section,numerators,denominator)? else{return Ok(None);};
            for row in 0..2{for axis in 0..2{global[row][axis]=hull([global[row][axis],jac[row][axis]].into_iter());}}
        }}
    }}
    Ok(contraction(global)?.filter(|q|*q<1.))
}
fn projective_section_jacobian(s:&Surface,span:[usize;2],domain:[[f64;2];2],numerators:[[f64;4];2],denominator:[f64;4])->Result<Option<[[I;2];2]>>{
    let source=crate::curve_surface_composition::surface_net_on(s,span,domain)?;
    let dot=|h:[I;4],coeff:[f64;4]|->Result<I>{let mut sum=I::point(0.);for k in 0..4{sum=sum.add(h[k].mul(I::point(coeff[k]))?)?;}Ok(sum)};
    let mut net=Vec::new();
    for row in source{let mut out=Vec::new();for h in row{out.push([dot(h,numerators[0])?,dot(h,numerators[1])?,dot(h,denominator)?]);}net.push(out);}
    let values:[I;3]=std::array::from_fn(|k|hull(net.iter().flatten().map(|h|h[k])));
    if values[2].lo<=0.{return Ok(None);}
    let square=values[2].mul(values[2])?;
    let degrees=[s.degree_u,s.degree_v];
    let mut result=[[I::point(0.);2];2];
    for axis in 0..2{
        let width=I::point(domain[axis][1]).sub(I::point(domain[axis][0]))?;
        let mut diffs=Vec::new();
        for i in 0..=degrees[0]{for j in 0..=degrees[1]{
            if (axis==0&&i==degrees[0])||(axis==1&&j==degrees[1]){continue;}
            let a=net[i][j];let b=net[i+usize::from(axis==0)][j+usize::from(axis==1)];
            let mut d=[I::point(0.);3];
            for k in 0..3{d[k]=b[k].sub(a[k])?.mul(I::point(degrees[axis] as f64))?.div(width)?;}diffs.push(d);
        }}
        let d:[I;3]=std::array::from_fn(|k|hull(diffs.iter().map(|h|h[k])));
        for row in 0..2{result[row][axis]=d[row].mul(values[2])?.sub(values[row].mul(d[2])?)?.div(square)?;}
    }
    Ok(Some(result))
}
fn linear_projection_work(s:&Surface,basis:[[f64;3];2],max_cells:usize)->Result<(Option<f64>,usize)> {
    s.validate()?;
    check(max_cells>0&&max_cells<=100000&&basis.iter().flatten().all(|x|x.is_finite()),"Linear projection requires finite coefficients and 1..100000 cells")?;
    if s.periodic_u||s.periodic_v{return Ok((None,0));}
    let mut global=[[I{lo:f64::INFINITY,hi:f64::NEG_INFINITY};2];2];
    let mut cells=0;
    for u in s.degree_u..s.control_points.len(){for v in s.degree_v..s.control_points[0].len(){
        let ranges=[[s.knots_u[u],s.knots_u[u+1]],[s.knots_v[v],s.knots_v[v+1]]];
        if ranges.iter().any(|r|r[0]>=r[1]){continue;}
        for i in 0..4{for j in 0..4{
            if cells==max_cells{return Ok((None,cells));}
            let section=std::array::from_fn(|k|{
                let n=[i,j][k];let [lo,hi]=ranges[k];
                [if n==0{lo}else{lo+(hi-lo)*(n as f64/4.)},if n==3{hi}else{lo+(hi-lo)*((n+1) as f64/4.)}]
            });
            let jac=section_jacobian(s,[u,v],section)?;cells+=1;
            for row in 0..2{for axis in 0..2{
                let mut value=I::point(0.);
                for k in 0..3{value=value.add(I::point(basis[row][k]).mul(jac[k][axis])?)?;}
                global[row][axis]=hull([global[row][axis],value].into_iter());
            }}
        }}
    }}
    Ok((contraction(global)?.filter(|q|*q<1.),cells))
}
/// Enclose (dS/du cross dS/dv) dot direction over the requested rectangle.
/// At a knot, include both one-sided derivatives. A point rectangle uses a
/// non-collapsed knot span for derivative bounds, never a zero column.
/// None means the span budget did not cover the entire rectangle.
pub fn normal_direction_bounds(s: &Surface, domain: [[f64;2];2], direction: [f64;3], max_spans: usize) -> Result<Option<[f64;2]>> {
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
    for u in degrees[0]..counts[0] { for v in degrees[1]..counts[1] {
        let indices=[u,v];
        let mut section=[[0.;2];2];
        let mut outside=false;
        for axis in 0..2 {
            let k=indices[axis];let lo=knots[axis][k];let hi=knots[axis][k+1];
            if lo==hi || domain[axis][1]<lo || domain[axis][0]>hi {outside=true;break;}
            section[axis]=[lo.max(domain[axis][0]),hi.min(domain[axis][1])];
            if section[axis][0]==section[axis][1] {section[axis]=[lo,hi];}
        }
        if outside {continue;}
        if spans==max_spans {return Ok(None);}
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
    Ok(Some(bounds))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn graph() -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: (0..3)
                .map(|i| {
                    (0..3)
                        .map(|j| vec![i as f64 / 2., j as f64 / 2., (i * j) as f64])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn perspective_projection_requires_a_nonzero_denominator_and_full_global_coverage(){
        let basis=[[1.,0.,0.,0.],[0.,1.,0.,0.]];
        let mut s=graph();let before=format!("{s:?}");
        assert!(certify_projective_projection(&s,basis,[0.,0.,0.,1.],16).unwrap().is_some());
        assert!(certify_projective_projection(&s,basis,[0.,0.,0.,1.],15).unwrap().is_none());
        for d in [[0.,0.,0.,0.],[0.,0.,0.,-1.],[1.,0.,0.,-0.5],[1.,0.,0.,0.]]{
            assert!(certify_projective_projection(&s,basis,d,16).unwrap().is_none());
        }
        assert_eq!(format!("{s:?}"),before);
        assert!(certify_projective_projection(&s,basis,[0.,0.,0.,1.],0).is_err());
        assert!(certify_projective_projection(&s,[[f64::NAN,0.,0.,0.],basis[1]],[0.,0.,0.,1.],16).is_err());
        s.degree_u=1;s.knots_u=vec![0.,0.,0.5,1.,1.];
        assert!(certify_projective_projection(&s,basis,[0.,0.,0.,1.],32).unwrap().is_some());
        assert!(certify_projective_projection(&s,basis,[0.,0.,0.,1.],31).unwrap().is_none());
        for p in &mut s.control_points[2]{p[0]=0.;}
        // Each side is locally invertible, but the complete folded chart isn't.
        assert!(certify_projective_projection(&s,basis,[0.,0.,0.,1.],32).unwrap().is_none());
        s.periodic_u=true;
        s.knots_u=vec![-1.,0.,1.,2.,3.];
        s.control_points[2]=s.control_points[0].clone();s.weights[2]=s.weights[0].clone();
        assert!(certify_projective_projection(&s,basis,[0.,0.,0.,1.],32).unwrap().is_none());
    }
    #[test]
    fn diagonal_projection_certifies_quarter_cylinder_and_refuses_partial_coverage(){
        let mut s=Surface{degree_u:2,degree_v:1,knots_u:vec![0.,0.,0.,1.,1.,1.],knots_v:vec![0.,0.,1.,1.],
            control_points:vec![vec![vec![2.,0.,0.],vec![2.,0.,4.]],vec![vec![2.,2.,0.],vec![2.,2.,4.]],vec![vec![0.,2.,0.],vec![0.,2.,4.]]],
            weights:vec![vec![1.;2],vec![std::f64::consts::FRAC_1_SQRT_2;2],vec![1.;2]],periodic_u:false,periodic_v:false};
        let basis=[[-1.,1.,0.],[0.,0.,1.]];
        assert!(!certify(&s,10).unwrap().proven);
        let q=certify_linear_projection(&s,basis,16).unwrap();assert!(q.is_some(),"{q:?}");
        assert!(certify_linear_projection(&s,basis,15).unwrap().is_none());
        for row in &mut s.control_points{for p in row{p[2]=0.;}}
        assert!(certify_linear_projection(&s,basis,16).unwrap().is_none());
    }
    #[test]
    fn normal_bounds_cover_graph_derivatives_and_do_not_collapse_at_points() {
        // S(u,v)=(u,v,4uv), hence (Su cross Sv).d = -4v*dx-4u*dy+dz.
        let s=graph();
        for domain in [[[0.,1.],[0.,1.]],[[0.2,0.4],[0.3,0.7]],[[0.25,0.25],[0.5,0.5]]] {
            for direction in [[0.,0.,1.],[0.3,-0.2,0.7],[0.,0.,-1.]] {
                let b=normal_direction_bounds(&s,domain,direction,10).unwrap().unwrap();
                for u in domain[0] {for v in domain[1] {
                    let value=-4.*v*direction[0]-4.*u*direction[1]+direction[2];
                    assert!(b[0]<=value && value<=b[1], "{b:?}: {value}");
                }}
                if direction[2]==1. {assert!(b[0]>0.);}
                if direction[2]==-1. {assert!(b[1]<0.);}
            }
        }
    }
    #[test]
    fn rational_normal_bounds_contain_an_independent_separable_formula() {
        let s=Surface {degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
            control_points:vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]],vec![vec![1.,0.,0.],vec![1.,1.,0.]]],
            weights:vec![vec![1.,3.],vec![2.,6.]],periodic_u:false,periodic_v:false};
        // x=2u/(1+u), y=3v/(1+2v); normal Z is dx/du * dy/dv.
        for i in 0..10 {for j in 0..10 {
            let domain=[[i as f64/10.,(i+1) as f64/10.],[j as f64/10.,(j+1) as f64/10.]];
            let b=normal_direction_bounds(&s,domain,[0.,0.,1.],1).unwrap().unwrap();
            for u in domain[0] {for v in domain[1] {
                let z=6./((1.+u).powi(2)*(1.+2.*v).powi(2));
                assert!(b[0]<=z && z<=b[1]);
            }}
        }}
    }
    #[test]
    fn normal_bounds_keep_both_knot_sides_and_refuse_partial_coverage() {
        let mut s=graph();s.degree_u=1;s.knots_u=vec![0.,0.,0.5,1.,1.];
        for p in &mut s.control_points[2] {p[0]=0.;}
        let d=[[0.5,0.5],[0.3,0.7]];
        assert!(normal_direction_bounds(&s,d,[0.,0.,1.],1).unwrap().is_none());
        let b=normal_direction_bounds(&s,d,[0.,0.,1.],2).unwrap().unwrap();
        assert!(b[0]<0. && b[1]>0.);
        assert!(normal_direction_bounds(&s,[[-1.,0.],[0.,1.]],[0.,0.,1.],2).is_err());
    }
    #[test]
    fn nonlinear_graph_and_vertical_rotation_are_globally_injective() {
        let mut s = graph();
        let r = certify(&s, 10).unwrap();
        assert!(r.proven);
        assert_eq!(r.projection, Some([0, 1]));
        assert!(r.contraction_upper.unwrap() < 1e-10);
        for row in &mut s.control_points {
            for p in row {
                p.swap(1, 2);
            }
        }
        assert!(certify(&s, 10).unwrap().proven);
    }
    #[test]
    fn folded_and_collapsed_charts_never_claim_injectivity() {
        let mut s = graph();
        for (i, row) in s.control_points.iter_mut().enumerate() {
            for p in row {
                p[0] = [1., -1., 1.][i];
                p[2] = 0.;
            }
        }
        assert!(!certify(&s, 10).unwrap().proven);
        assert!(certify_linear_projection(&s,[[1.,0.,0.],[0.,1.,0.]],100).unwrap().is_none());
        for row in &mut s.control_points {
            for p in row {
                p[0] = 0.;
            }
        }
        assert!(!certify(&s, 10).unwrap().proven);
    }
    #[test]
    fn all_spans_must_pass_the_same_global_test() {
        let mut s = graph();
        s.degree_u = 1;
        s.knots_u = vec![0., 0., 0.3, 1., 1.];
        let partial = certify(&s, 1).unwrap();
        assert!(!partial.proven);
        assert_eq!(partial.reason, "work-limit");
        let r = certify(&s, 2).unwrap();
        assert!(r.proven);
        assert_eq!(r.spans, 2);
        // Both individual spans are regular; their union folds back on itself.
        for p in &mut s.control_points[2] {
            p[0] = 0.;
            p[2] = 0.;
        }
        for p in &mut s.control_points[1] {
            p[2] = 0.;
        }
        assert!(!certify(&s, 2).unwrap().proven);
    }
}
