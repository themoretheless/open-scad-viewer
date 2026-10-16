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
#[derive(Clone,Debug)]
pub struct NormalDirectionReport {pub bounds:Option<[f64;2]>,pub spans:usize}
mod boundary_normals;
pub use boundary_normals::boundary_normal_direction_bounds;
pub fn normal_direction_bounds(s: &Surface, domain: [[f64;2];2], direction: [f64;3], max_spans: usize) -> Result<Option<[f64;2]>> {
    Ok(normal_direction_bounds_detailed(s,domain,direction,max_spans)?.bounds)
}
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
#[cfg(test)]
mod tests;

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
    // A reflected/rotated parameter chart may move the useful projective
    // origin away from control[0][0]. Corner data only selects ONE proposal;
    // the complete original interval-Jacobian proof still supplies admission.
    let hint=affine_corner_projective_hint(s);
    if let Some(projection)=hint {
        if report.spans==max_spans {report.reason="work-limit";return Ok(report);}
        let(q,cells)=projective_projection_work(s,[projection[0],projection[1]],projection[2],max_spans-report.spans)?;
        report.spans+=cells;
        if let Some(q)=q {report.proven=true;report.projective_projection=Some(projection);report.contraction_upper=Some(q);report.reason="global-projective-projection-contraction";return Ok(report);}
    }
    for basis in LINEAR_PROJECTIONS {
        if report.spans==max_spans { report.reason="work-limit"; return Ok(report); }
        let (q,cells)=linear_projection_work(s,basis,max_spans-report.spans)?;
        report.spans+=cells;
        if let Some(q)=q {report.proven=true;report.linear_projection=Some(basis);report.contraction_upper=Some(q);report.reason="global-linear-projection-contraction";return Ok(report);}
    }
    for projection in projective_projections(s){
        if hint==Some(projection) {continue;}
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
pub fn polar_projection_candidates(s: &Surface) -> Result<Vec<[[f64;4];3]>> {
    s.validate()?;
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
    // Valid source degrees are at most 25. Products use degree at most 50;
    // all binomial coefficients and integer recurrence intermediates fit u64,
    // and the final coefficients are exactly representable in binary64.
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

/// Deterministic proposals from a validated source, not certificates.
pub fn projective_projection_candidates(s:&Surface)->Result<[[[f64;4];3];6]> {
    s.validate()?;
    Ok(projective_projections(s))
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

// Bounded raw-control proposal ranking. A small parallelogram residual is NOT
// an affine identity, injectivity proof or regularity claim. All rejected and
// accepted interval trials remain in the caller's original shared span ledger.
fn affine_corner_projective_hint(s:&Surface)->Option<[[f64;4];3]> {
    let last=s.control_points.len()-1;let end=s.control_points[0].len()-1;
    let corners=[&s.control_points[0][0],&s.control_points[0][end],&s.control_points[last][0],&s.control_points[last][end]];
    let mut best:Option<(f64,[[f64;4];3])>=None;
    // Interior samples only break ties between raw proposals. They cannot
    // establish an identity or admit geometry; every chosen candidate still
    // needs the whole-domain interval proof and original shared span budget.
    let domain=[[s.knots_u[s.degree_u],s.knots_u[s.control_points.len()]],
                [s.knots_v[s.degree_v],s.knots_v[s.control_points[0].len()]]];
    let samples=[(0.25,0.25),(0.75,0.25),(0.5,0.5),(0.25,0.75),(0.75,0.75)]
        .into_iter().map(|(u,v)|s.evaluate(domain[0][0]+u*(domain[0][1]-domain[0][0]),domain[1][0]+v*(domain[1][1]-domain[1][0])).ok().map(|p|(u,v,p.point)))
        .collect::<Option<Vec<_>>>()?;
    for (origin,anchor) in [(0,2),(2,0),(1,3),(3,1),(0,1),(1,0),(2,3),(3,2)] {
        for i in 0..6 {
            let axis=[2,2,0,0,1,1][i];let free=match axis {2=>[0,1],0=>[1,2],_=>[0,2]};
            let at=corners[anchor][axis];
            let extent=s.control_points.iter().flatten().map(|p|(p[axis]-at).abs()).fold(0.,f64::max);
            let mut p=[[0.;4];3];p[0][free[0]]=1.;p[1][free[1]]=1.;p[2][axis]=if i%2==0 {1.}else{-1.};p[2][3]=extent-p[2][axis]*at;
            for j in 0..2 {p[j][3]=-corners[origin][free[j]];}
            if p.iter().flatten().any(|x|!x.is_finite()) {continue;}
            let mut values=[[0.;2];4];let mut valid=true;
            for (n,c) in corners.iter().enumerate() {
                let d=p[2][axis]*c[axis]+p[2][3];
                if !d.is_finite() || d<=0. {valid=false;break;}
                for j in 0..2 {values[n][j]=(c[free[j]]+p[j][3])/d;valid&=values[n][j].is_finite();}
            }
            if !valid {continue;}
            let a=[values[2][0]-values[0][0],values[2][1]-values[0][1]];
            let b=[values[1][0]-values[0][0],values[1][1]-values[0][1]];
            let det=a[0]*b[1]-a[1]*b[0];if !det.is_finite() || det==0. {continue;}
            let scale=values.iter().flatten().map(|x|x.abs()).fold(1.,f64::max);
            let mut score=(0..2).map(|j|(values[0][j]+values[3][j]-values[1][j]-values[2][j]).abs()/scale).fold(0.,f64::max);
            if !score.is_finite() || score>1e-12 {continue;}
            for &(u,v,c) in &samples {
                let d=p[2][axis]*c[axis]+p[2][3];if !d.is_finite() || d<=0. {valid=false;break;}
                for j in 0..2 {
                    let actual=(c[free[j]]+p[j][3])/d;
                    let expected=values[0][j]+u*a[j]+v*b[j];
                    if !actual.is_finite() || !expected.is_finite() {valid=false;break;}
                    score=score.max((actual-expected).abs()/scale);
                }
            }
            if valid && score.is_finite() && best.as_ref().is_none_or(|(old,_)|score<*old) {best=Some((score,p));}
        }
    }
    best.map(|(_,p)|p)
}
