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
    pub projective_projection: Option<[[f64;4];3]>,
    pub polar_projection: Option<[[f64;4];3]>,
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
    homogeneous_section_jacobian(&net,[p,q],domain)
}
fn homogeneous_section_jacobian(net:&[Vec<[I;4]>],degrees:[usize;2],domain:[[f64;2];2])->Result<[[I;2];3]>{
    let [p,q]=degrees;
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
pub fn certify_contraction(s: &Surface, max_spans: usize) -> Result<Report> {
    s.validate()?;
    check(
        max_spans > 0 && max_spans <= 100_000,
        "Injectivity span budget must be in 1..100000",
    )?;
    let mut report = Report {
        proven: false,
        projection: None,
        linear_projection: None,
        projective_projection: None,
        polar_projection: None,
        contraction_upper: None,
        spans: 0,
        reason: "projection-not-proven",
    };
    if s.periodic_u || s.periodic_v {
        report.reason = "periodic-domain";
        return Ok(report);
    }
    // A clamped boundary with identical Euclidean controls maps a whole
    // parameter edge to one point, even with different positive weights.
    // Strict rectangular-chart injectivity cannot certify that pole. A future
    // proof on the quotient domain must handle it explicitly.
    let collapsed_u = [0, s.control_points.len()-1].into_iter().any(|i| {
        let knots = if i == 0 { &s.knots_u[..=s.degree_u] }
            else { &s.knots_u[s.control_points.len()..] };
        knots.iter().all(|k| *k == knots[0]) &&
            s.control_points[i].iter().all(|p| *p == s.control_points[i][0])
    });
    let collapsed_v = [0, s.control_points[0].len()-1].into_iter().any(|j| {
        let knots = if j == 0 { &s.knots_v[..=s.degree_v] }
            else { &s.knots_v[s.control_points[0].len()..] };
        knots.iter().all(|k| *k == knots[0]) &&
            s.control_points.iter().all(|row| row[j] == s.control_points[0][j])
    });
    if collapsed_u || collapsed_v {
        report.reason = "collapsed-boundary-requires-quotient-proof";
        return Ok(report);
    }
    let mut j = [[I {
        lo: f64::INFINITY,
        hi: f64::NEG_INFINITY,
    }; 2]; 3];
    let us = crate::certificates::audit::nonempty_spans(&s.knots_u, s.degree_u, s.control_points.len());
    let vs = crate::certificates::audit::nonempty_spans(&s.knots_v, s.degree_v, s.control_points[0].len());
    for &u in &us {
        for &v in &vs {
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
    Ok(report)
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
    #[test]
    fn polar_jacobian_bounds_contain_independent_rational_coordinate_derivatives() {
        // x=2, y=2u/(1+u), z=-3v/(1+2v), from separable weights.
        let s = Surface {
            degree_u: 1, degree_v: 1,
            knots_u: vec![0.,0.,1.,1.], knots_v: vec![0.,0.,1.,1.],
            control_points: (0..2).map(|i| (0..2).map(|j| vec![2.,i as f64,-(j as f64)]).collect()).collect(),
            weights: vec![vec![1.,3.],vec![2.,6.]], periodic_u: false, periodic_v: false,
        };
        let coordinates=[[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.]];
        for i in 0..4 { for j in 0..4 {
            let domain=[[i as f64/4.,(i+1) as f64/4.],[j as f64/4.,(j+1) as f64/4.]];
            let bounds=polar_section_jacobian(&s,[1,1],domain,coordinates).unwrap().unwrap();
            for u in domain[0] { for v in domain[1] {
                let y=2.*u/(1.+u);let dy=2./(1.+u).powi(2);
                let expected=[[2.*dy/(4.+y*y),0.],[y*dy/(4.+y*y).sqrt(),3./(1.+2.*v).powi(2)]];
                for row in 0..2 { for axis in 0..2 {
                    let b=bounds[row][axis];let value=expected[row][axis];
                    assert!(b.lo<=value && value<=b.hi,"{row}/{axis}: {b:?} excludes {value}");
                }}
            }}
        }}
    }
    #[test]
    fn polar_projection_requires_global_coverage_continuity_and_two_independent_parameters() {
        let mut s = Surface {
            degree_u: 1, degree_v: 1,
            knots_u: vec![0.,0.,1.,1.], knots_v: vec![0.,0.,1.,1.],
            control_points: (0..2).map(|i| (0..2).map(|j| vec![2.,i as f64,-(j as f64)]).collect()).collect(),
            weights: vec![vec![1.;2];2], periodic_u: false, periodic_v: false,
        };
        let coordinates = [[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.]];
        let before = format!("{s:?}");
        assert!(certify_polar_projection(&s,coordinates,4,16).unwrap().is_some());
        assert!(certify_polar_projection(&s,coordinates,4,15).unwrap().is_none());
        assert!(certify_polar_projection(&s,coordinates,0,16).is_err());
        assert!(certify_polar_projection(&s,coordinates,4,0).is_err());
        let mut invalid = coordinates; invalid[0][0] = f64::NAN;
        assert!(certify_polar_projection(&s,invalid,4,16).is_err());
        let mut wrong_branch = coordinates; wrong_branch[0][0] = -1.;
        assert!(certify_polar_projection(&s,wrong_branch,4,16).unwrap().is_none());
        assert_eq!(format!("{s:?}"),before);
        s.control_points[1] = s.control_points[0].clone();
        assert!(certify_polar_projection(&s,coordinates,4,16).unwrap().is_none());
        let row = |y| vec![vec![2.,y,0.],vec![2.,y,-1.]];
        s.control_points = vec![row(0.),row(1.),row(0.)];
        s.weights = vec![vec![1.;2];3]; s.knots_u = vec![0.,0.,0.5,1.,1.];
        // Locally invertible halves overlap. No collection of local proofs
        // may be promoted to a proof of this complete folded chart.
        assert!(certify_polar_projection(&s,coordinates,4,32).unwrap().is_none());
        s.control_points = vec![row(0.),row(1.),row(0.),row(1.)];
        s.weights = vec![vec![1.;2];4]; s.knots_u = vec![0.,0.,0.5,0.5,1.,1.];
        assert!(certify_polar_projection(&s,coordinates,4,32).is_err());
    }

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
        for row in &mut s.control_points {
            for p in row {
                p[0] = 0.;
            }
        }
        assert!(!certify(&s, 10).unwrap().proven);
    }
    #[test]
    fn clamped_pole_reports_missing_quotient_proof_without_spending_budget() {
        let mut s = graph();
        let pole = s.control_points[0][0].clone();
        for p in &mut s.control_points[0] { *p = pole.clone(); }
        for (j, w) in s.weights[0].iter_mut().enumerate() { *w = 1. + j as f64; }
        let result = certify(&s, 10).unwrap();
        assert!(!result.proven);
        assert_eq!(result.spans, 0);
        assert_eq!(result.reason, "collapsed-boundary-requires-quotient-proof");
        // Close but distinct controls cannot be classified as an exact pole.
        s.control_points[0][1][0] += 1e-10;
        assert_ne!(certify(&s, 10).unwrap().reason,
            "collapsed-boundary-requires-quotient-proof");
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

/// Compatibility entry point: every contraction and refinement cell consumes
/// the original shared span budget. Explicit sweep stages use contraction only.
pub fn certify(s:&Surface,max_spans:usize)->Result<Report> {
    let mut report=certify_contraction(s,max_spans)?;
    if report.proven || report.reason=="collapsed-boundary-requires-quotient-proof" || report.reason=="periodic-domain" || report.reason=="work-limit" {return Ok(report);}
    if s.control_points.iter().flatten().all(|p|p==&s.control_points[0][0]) {return Ok(report);}
    for basis in LINEAR_PROJECTIONS {
        if report.spans==max_spans { report.reason="work-limit"; return Ok(report); }
        let (q,cells)=linear_projection_work(s,basis,max_spans-report.spans)?;
        report.spans+=cells;
        if let Some(q)=q {report.proven=true;report.linear_projection=Some(basis);report.contraction_upper=Some(q);report.reason="global-linear-projection-contraction";return Ok(report);}
    }
    for projection in projective_projections(s){
        if report.spans==max_spans{report.reason="work-limit";return Ok(report);}
        let (q,cells)=projective_projection_work(s,[projection[0],projection[1]],projection[2],max_spans-report.spans)?;
        report.spans+=cells;
        if let Some(q)=q{
            report.proven=true;report.projective_projection=Some(projection);report.contraction_upper=Some(q);
            report.reason="global-projective-projection-contraction";return Ok(report);
        }
    }
    let cells = polar_cells(s,16);
    if s.degree_u <= 8 && s.degree_v <= 8 && cells <= max_spans-report.spans {
        for coordinates in polar_projection_candidates(s)? {
            if cells > max_spans-report.spans { break; }
            let proof = certify_polar_projection(s,coordinates,16,cells)?;
            // Reserve the complete coverage cost even if a candidate is
            // rejected early. The shared budget never understates work.
            report.spans += cells;
            if let Some(q) = proof {
                report.proven=true;report.polar_projection=Some(coordinates);report.contraction_upper=Some(q);
                report.reason="global-polar-projection-contraction";return Ok(report);
            }
        }
    }
    if report.spans==max_spans {report.reason="work-limit";}
    Ok(report)
}

pub fn certify_projective_projection(s:&Surface,numerators:[[f64;4];2],denominator:[f64;4],max_cells:usize)->Result<Option<f64>>{
    Ok(projective_projection_work(s,numerators,denominator,max_cells)?.0)
}
/// A nonlinear projection F=(atan2(y,x), sqrt(x*x+y*y)-z), with x,y,z
/// supplied as affine functions of the original position. A strictly positive
/// x hull establishes one continuous angle branch. One common Jacobian hull
/// and contraction over the entire chart prove global injectivity; successful
/// local cells alone are insufficient. Collapsed boundaries are not quotiented.
pub fn certify_polar_projection(
    s: &Surface,
    coordinates: [[f64; 4]; 3],
    subdivisions: usize,
    max_cells: usize,
) -> Result<Option<f64>> {
    s.validate()?;
    check(
        subdivisions > 0 && subdivisions <= 64 && max_cells > 0 && max_cells <= 100_000
            && coordinates.iter().flatten().all(|x| x.is_finite()),
        "Polar projection requires finite coordinates, 1..64 subdivisions and 1..100000 cells",
    )?;
    if s.periodic_u || s.periodic_v || s.degree_u > 8 || s.degree_v > 8 { return Ok(None); }
    // Surface validation rejects disconnected full-multiplicity internal knots.
    let required = polar_cells(s,subdivisions);
    if required > max_cells { return Ok(None); }
    let mut global = [[I { lo: f64::INFINITY, hi: f64::NEG_INFINITY }; 2]; 2];
    let mut cells = 0;
    for u in s.degree_u..s.control_points.len() {
        for v in s.degree_v..s.control_points[0].len() {
            let ranges = [[s.knots_u[u], s.knots_u[u+1]], [s.knots_v[v], s.knots_v[v+1]]];
            if ranges.iter().any(|r| r[0] >= r[1]) { continue; }
            for i in 0..subdivisions { for j in 0..subdivisions {
                if cells == max_cells { return Ok(None); }
                let domain = std::array::from_fn(|k| {
                    let n = [i,j][k]; let [lo,hi] = ranges[k];
                    [if n == 0 { lo } else { lo+(hi-lo)*(n as f64/subdivisions as f64) },
                     if n+1 == subdivisions { hi } else { lo+(hi-lo)*((n+1) as f64/subdivisions as f64) }]
                });
                cells += 1;
                let Some(jac) = polar_section_jacobian(s, [u,v], domain, coordinates)? else { return Ok(None); };
                for row in 0..2 { for axis in 0..2 {
                    global[row][axis] = hull([global[row][axis],jac[row][axis]].into_iter());
                }}
            }}
        }
    }
    Ok(contraction(global)?.filter(|q| *q < 1.))
}
fn polar_cells(s: &Surface, subdivisions: usize) -> usize {
    let active = |knots: &[f64],degree: usize,count: usize|
        (degree..count).filter(|&k| knots[k] < knots[k+1]).count();
    active(&s.knots_u,s.degree_u,s.control_points.len())
        * active(&s.knots_v,s.degree_v,s.control_points[0].len()) * subdivisions * subdivisions
}
fn polar_projection_candidates(s: &Surface) -> Result<Vec<[[f64;4];3]>> {
    // Restrict automatic frame guesses to one clamped quadratic iso-curve.
    // The public verifier also accepts other degrees and explicit frames.
    if s.degree_u != 2 || s.control_points.len() != 3
        || !s.knots_u[..3].iter().all(|k| *k == s.knots_u[2])
        || !s.knots_u[3..].iter().all(|k| *k == s.knots_u[3])
        || !s.knots_v[..=s.degree_v].iter().all(|k| *k == s.knots_v[s.degree_v]) {
        return Ok(Vec::new());
    }
    let first: [f64;3]=s.control_points[0][0].as_slice().try_into().unwrap();
    let last: [f64;3]=s.control_points[2][0].as_slice().try_into().unwrap();
    let weights=[s.weights[0][0],2.*s.weights[1][0],s.weights[2][0]];
    let denominator=weights[0]+weights[1]+weights[2];
    let middle: [f64;3]=std::array::from_fn(|k|(first[k]*weights[0]+s.control_points[1][0][k]*weights[1]+last[k]*weights[2])/denominator);
    let a=std::array::from_fn(|k|middle[k]-first[k]);let b=std::array::from_fn(|k|last[k]-first[k]);
    let n=math_core::cross(a,b);let squared=math_core::dot(n,n);
    if !squared.is_finite() || squared==0. { return Ok(Vec::new()); }
    let an=math_core::dot(a,a);let bn=math_core::dot(b,b);
    let bx=math_core::cross(b,n);let nx=math_core::cross(n,a);
    let center: [f64;3]=std::array::from_fn(|k|first[k]+(an*bx[k]+bn*nx[k])/(2.*squared));
    let radial=std::array::from_fn(|k|middle[k]-center[k]);let radius=math_core::norm(radial);let normal=math_core::norm(n);
    if center.iter().any(|x|!x.is_finite()) || !radius.is_finite() || radius==0. || !normal.is_finite() || normal==0. { return Ok(Vec::new()); }
    let x=radial.map(|v|v/radius);let z=n.map(|v|v/normal);let y=math_core::cross(z,x);
    let affine=|direction: [f64;3]| [direction[0],direction[1],direction[2],-math_core::dot(direction,center)];
    let candidates=[1.,-1.].map(|sign|[affine(x),affine(y),affine(z.map(|v|v*sign))]);
    Ok(candidates.into_iter().filter(|c|c.iter().flatten().all(|v|v.is_finite())).collect())
}
fn polar_section_jacobian(s: &Surface, span: [usize;2], domain: [[f64;2];2], coordinates: [[f64;4];3]) -> Result<Option<[[I;2];2]>> {
    let source = crate::curve_surface_composition::surface_net_on(s, span, domain)?;
    let mut net = Vec::new();
    for row in source {
        let mut projected = Vec::new();
        for h in row {
            let mut out = [I::point(0.);4]; out[3] = h[3];
            for k in 0..3 { for d in 0..4 {
                out[k] = out[k].add(h[d].mul(I::point(coordinates[k][d]))?)?;
            }}
            projected.push(out);
        }
        net.push(projected);
    }
    let mut position = [I { lo: f64::INFINITY, hi: f64::NEG_INFINITY };3];
    for h in net.iter().flatten() {
        if h[3].lo <= 0. { return Ok(None); }
        for k in 0..3 { position[k] = hull([position[k],h[k].div(h[3])?].into_iter()); }
    }
    let [x,y,_] = position;
    if x.lo <= 0. { return Ok(None); }
    // Squaring a signed interval is not the independent product I*I.
    let square = |a: I| -> Result<I> {
        let minimum = if a.lo <= 0. && a.hi >= 0. { 0. } else { a.lo.abs().min(a.hi.abs()) };
        let maximum = a.lo.abs().max(a.hi.abs());
        I::new((minimum*minimum).next_down().max(0.),(maximum*maximum).next_up())
    };
    let radial_squared = square(x)?.add(square(y)?)?;
    if radial_squared.lo <= 0. { return Ok(None); }
    let components: [PolarPoly;4] = std::array::from_fn(|k| net.iter().map(|row| row.iter().map(|h| h[k]).collect()).collect());
    let [nx,ny,nz,w] = &components;
    let squared = polar_poly_add(&polar_poly_mul(nx,nx)?, &polar_poly_mul(ny,ny)?, 1.)?;
    let squared_hull = hull(squared.iter().flatten().copied());
    if squared_hull.lo <= 0. { return Ok(None); }
    let sqrt_squared = I::new(squared_hull.lo.sqrt().next_down(), squared_hull.hi.sqrt().next_up())?;
    let weight = hull(w.iter().flatten().copied());
    let weight_squared = weight.mul(weight)?;
    let radial_denominator = weight_squared.mul(sqrt_squared)?.mul(I::point(2.))?;
    let mut result = [[I::point(0.);2];2];
    for axis in 0..2 {
        let dx = polar_poly_derivative(nx,axis,domain)?;
        let dy = polar_poly_derivative(ny,axis,domain)?;
        let dz = polar_poly_derivative(nz,axis,domain)?;
        let dw = polar_poly_derivative(w,axis,domain)?;
        let ds = polar_poly_derivative(&squared,axis,domain)?;
        let angular = polar_poly_add(&polar_poly_mul(nx,&dy)?, &polar_poly_mul(ny,&dx)?, -1.)?;
        let radial = polar_poly_add(&polar_poly_mul(w,&ds)?, &polar_poly_mul(&squared,&dw)?, -2.)?;
        let vertical = polar_poly_add(&polar_poly_mul(&dz,w)?, &polar_poly_mul(nz,&dw)?, -1.)?;
        result[0][axis] = hull(angular.iter().flatten().copied()).div(squared_hull)?;
        result[1][axis] = hull(radial.iter().flatten().copied()).div(radial_denominator)?
            .sub(hull(vertical.iter().flatten().copied()).div(weight_squared)?)?;
    }
    Ok(Some(result))
}
pub(crate) type PolarPoly = Vec<Vec<I>>;
pub(crate) fn polar_poly_add(a: &PolarPoly,b: &PolarPoly,scale: f64) -> Result<PolarPoly> {
    a.iter().zip(b).map(|(ra,rb)| ra.iter().zip(rb).map(|(x,y)| x.add(y.mul(I::point(scale))?)).collect()).collect()
}
pub(crate) fn polar_poly_derivative(a: &PolarPoly,axis: usize,domain: [[f64;2];2]) -> Result<PolarPoly> {
    let [p,q] = [a.len()-1,a[0].len()-1];
    let degrees = [p,q];
    let width = I::point(domain[axis][1]).sub(I::point(domain[axis][0]))?;
    (0..=p-usize::from(axis==0)).map(|i| (0..=q-usize::from(axis==1)).map(|j|
        a[i+usize::from(axis==0)][j+usize::from(axis==1)].sub(a[i][j])?
            .mul(I::point(degrees[axis] as f64))?.div(width)
    ).collect()).collect()
}
pub(crate) fn polar_poly_mul(a: &PolarPoly,b: &PolarPoly) -> Result<PolarPoly> {
    let [p,q,r,t] = [a.len()-1,a[0].len()-1,b.len()-1,b[0].len()-1];
    // Current callers cap each source degree at eight, hence all binomial
    // coefficients here (degree <=24) are exactly representable integers.
    let choose = |n: usize,k: usize| {
        let mut value=1u64;
        for i in 0..k.min(n-k) { value=value*(n-i) as u64/(i+1) as u64; }
        value as f64
    };
    let coefficient = |n: usize,m: usize,i: usize,j: usize| -> Result<I> {
        I::point(choose(n,i)).mul(I::point(choose(m,j)))?.div(I::point(choose(n+m,i+j)))
    };
    let mut out=vec![vec![I::point(0.);q+t+1];p+r+1];
    for i in 0..=p { for j in 0..=q { for k in 0..=r { for l in 0..=t {
        let scale=coefficient(p,r,i,k)?.mul(coefficient(q,t,j,l)?)?;
        out[i+k][j+l]=out[i+k][j+l].add(a[i][j].mul(b[k][l])?.mul(scale)?)?;
    }}}}
    Ok(out)
}

fn projective_projection_work(s:&Surface,numerators:[[f64;4];2],denominator:[f64;4],max_cells:usize)->Result<(Option<f64>,usize)>{
    s.validate()?;
    check(max_cells>0&&max_cells<=100000&&numerators.iter().flatten().chain(denominator.iter()).all(|x|x.is_finite()),"Projective projection requires finite coefficients and 1..100000 cells")?;
    if s.periodic_u||s.periodic_v{return Ok((None,0));}
    let mut global=[[I{lo:f64::INFINITY,hi:f64::NEG_INFINITY};2];2];
    let mut cells=0;
    let mut all_positive=true;
    for u in s.degree_u..s.control_points.len(){for v in s.degree_v..s.control_points[0].len(){
        let ranges=[[s.knots_u[u],s.knots_u[u+1]],[s.knots_v[v],s.knots_v[v+1]]];
        if ranges.iter().any(|r|r[0]>=r[1]){continue;}
        for i in 0..4{for j in 0..4{
            if cells==max_cells{return Ok((None,cells));}
            let section=std::array::from_fn(|k|{let n=[i,j][k];let [lo,hi]=ranges[k];
                [if n==0{lo}else{lo+(hi-lo)*(n as f64/4.)},if n==3{hi}else{lo+(hi-lo)*((n+1) as f64/4.)}]});
            cells+=1;
            let Some(jac)=projective_section_jacobian(s,[u,v],section,numerators,denominator)? else{all_positive=false;continue;};
            for row in 0..2{for axis in 0..2{global[row][axis]=hull([global[row][axis],jac[row][axis]].into_iter());}}
        }}
    }}
    Ok((if all_positive{contraction(global)?.filter(|q|*q<1.)}else{None},cells))
}
fn projective_section_jacobian(s:&Surface,span:[usize;2],domain:[[f64;2];2],numerators:[[f64;4];2],denominator:[f64;4])->Result<Option<[[I;2];2]>>{
    let source=crate::curve_surface_composition::surface_net_on(s,span,domain)?;
    let dot=|h:[I;4],coeff:[f64;4]|->Result<I>{let mut sum=I::point(0.);for k in 0..4{sum=sum.add(h[k].mul(I::point(coeff[k]))?)?;}Ok(sum)};
    let mut net=Vec::new();
    for row in source{let mut out=Vec::new();for h in row{out.push([dot(h,numerators[0])?,dot(h,numerators[1])?,I::point(0.),dot(h,denominator)?]);}net.push(out);}
    if hull(net.iter().flatten().map(|h|h[3])).lo<=0.{return Ok(None);}
    let result=homogeneous_section_jacobian(&net,[s.degree_u,s.degree_v],domain)?;
    Ok(Some([result[0],result[1]]))
}

fn projective_projections(s:&Surface)->[[[f64;4];3];6]{
    std::array::from_fn(|i|{
        let axis=[2,2,0,0,1,1][i];let free=match axis{2=>[0,1],0=>[1,2],_=>[0,2]};
        let anchor=s.control_points[s.control_points.len()-1][0][axis];
        let extent=s.control_points.iter().flatten().map(|p|(p[axis]-anchor).abs()).fold(0.,f64::max);
        let mut rows=[[0.;4];3];rows[0][free[0]]=1.;rows[1][free[1]]=1.;
        for j in 0..2{let origin=s.control_points[0][0][free[j]];rows[j][3]=if origin==0.{0.}else{-origin};}
        rows[2][axis]=if i%2==0{1.}else{-1.};rows[2][3]=extent-rows[2][axis]*anchor;rows
    })
}
