//! Complete-rectangle normal/line angle bounds, including both sides of knots.
pub use crate::surface_boundary_secant::{OriginalBoundarySecant,boundary_secant_bounds,OriginalBoundaryDirection,boundary_secant_direction_bounds};
pub use crate::surface_weight_profile::{WeightProfile,ProfileDirection,ProfileBoundary,ProfilePathReport,ProfilePoleLimits,ProfilePoleNormal,ProfilePoleCover,ProfileSection,ProfileEndTransition,ProfileTransitionRail,TransitionRegularityReport,qualify_weight_profile};
pub use crate::profile_section_bridge::{ProfileSectionBridge,ProfileBridgeRail,RailConstrainedBridge,ProfileBridgeChordCoverage};
use crate::{check, distance_bounds::Interval as I, surface::Surface, Result};

#[derive(Debug)]
pub struct Report {
    pub aligned: Option<bool>,
    pub sine_squared_interval: Option<[f64; 2]>,
    pub normal_components: Option<[[f64; 2]; 3]>,
    pub spans: usize,
    pub reason: &'static str,
}
fn square(x: I) -> Result<I> {
    let lo = if x.lo > 0. {
        x.lo
    } else if x.hi < 0. {
        -x.hi
    } else {
        0.
    };
    let hi = x.lo.abs().max(x.hi.abs());
    I::new((lo * lo).next_down().max(0.), (hi * hi).next_up())
}
fn norm_squared(x: [I; 3]) -> Result<I> {
    let mut out = I::point(0.);
    for v in x {
        out = out.add(square(v)?)?;
    }
    I::new(out.lo.max(0.), out.hi)
}

/// Exclude normal alignment for every nonzero positive combination of the
/// supplied direction enclosures. The caller must prove its original chords
/// belong to this cone; this algebraic predicate does not certify a surface.
/// A strict positive projection prevents cancellation between generators.
pub fn positive_chord_cone_excluded(
    generators: &[[[f64; 2]; 3]], normal: [[f64; 2]; 3],
    projection: [f64; 3], max_sine_squared: f64,
) -> Result<bool> {
    check(!generators.is_empty() && generators.len()<=16
        && projection.iter().all(|x|x.is_finite())
        && max_sine_squared.is_finite() && (0. ..1.).contains(&max_sine_squared),
        "Choose bounded chord generators, a finite projection and an angle below one")?;
    let n=[I::new(normal[0][0],normal[0][1])?,I::new(normal[1][0],normal[1][1])?,I::new(normal[2][0],normal[2][1])?];
    let p=projection.map(I::point);
    let directions:Vec<_>=generators.iter().map(|g|Ok([
        I::new(g[0][0],g[0][1])?,I::new(g[1][0],g[1][1])?,I::new(g[2][0],g[2][1])?
    ])).collect::<Result<_>>()?;
    let n2=norm_squared(n)?;let p2=norm_squared(p)?;
    if n2.lo<=0. || p2.lo<=0. {return Ok(false);}
    let mut ratio=0_f64;
    for g in directions {
        let mut projected=I::point(0.);let mut normal_dot=I::point(0.);
        for k in 0..3 {
            projected=projected.add(g[k].mul(p[k])?)?;
            normal_dot=normal_dot.add(g[k].mul(n[k])?)?;
        }
        if projected.lo<=0. {return Ok(false);}
        let absolute=normal_dot.lo.abs().max(normal_dot.hi.abs());
        ratio=ratio.max(I::point(absolute).div(I::point(projected.lo))?.hi);
    }
    let cosine_upper=square(I::point(ratio))?.mul(p2)?.div(I::point(n2.lo))?.hi;
    Ok(cosine_upper<I::point(1.).sub(I::point(max_sine_squared))?.lo)
}

/// Exclude normal alignment for t*c + a*g0 + b*g1, where t is free and
/// a,b are nonnegative. The caller must prove the shared-curve decomposition
/// on the original sources. This algebraic decision alone admits no geometry.
pub fn shared_chord_cone_excluded(shared:[[f64;2];3],transverse:[[[f64;2];3];2],
    normal:[[f64;2];3],max_sine_squared:f64)->Result<bool> {
    check(max_sine_squared.is_finite()&&(0. ..1.).contains(&max_sine_squared),
        "Choose a finite shared-chord angle below one")?;
    let vectors=[shared,transverse[0],transverse[1]].into_iter().map(|g|Ok([
        I::new(g[0][0],g[0][1])?,I::new(g[1][0],g[1][1])?,I::new(g[2][0],g[2][1])?
    ])).collect::<Result<Vec<_>>>()?;
    let n=[I::new(normal[0][0],normal[0][1])?,I::new(normal[1][0],normal[1][1])?,I::new(normal[2][0],normal[2][1])?];
    let n2=norm_squared(n)?;if n2.lo<=0. {return Ok(false);}
    let dot=|a:[I;3],b:[I;3]|->Result<I>{let mut sum=I::point(0.);for k in 0..3 {sum=sum.add(a[k].mul(b[k])?)?;}Ok(sum)};
    let nd=vectors.iter().map(|g|dot(*g,n)).collect::<Result<Vec<_>>>()?;
    let weight=I::point(1.).sub(I::point(max_sine_squared))?.mul(n2)?;
    let mut q=[[I::point(0.);3];3];
    for a in 0..3 {for b in 0..3 {
        let gram=if a==b {norm_squared(vectors[a])?}else{dot(vectors[a],vectors[b])?};
        q[a][b]=weight.mul(gram)?.sub(nd[a].mul(nd[b])?)?;
    }}
    if q[0][0].lo<=0. {return Ok(false);}
    // Eliminate the unrestricted coefficient by a Schur complement. Strict
    // copositivity of its 2x2 block covers all nonnegative transverse pairs,
    // including equal directions where a full 3x3 rank test would refuse.
    let schur=|a:usize,b:usize|q[a][b].sub(q[a][0].mul(q[0][b])?.div(q[0][0])?);
    let a=schur(1,1)?;let b=schur(2,2)?;let off=schur(1,2)?;
    if a.lo<=0.||b.lo<=0. {return Ok(false);}
    if off.lo>=0. {return Ok(true);}
    Ok(I::point(a.lo).mul(I::point(b.lo))?.sub(square(I::point(off.lo))?)?.lo>0.)
}

/// A rounded direction proposal only. The interval cone predicate must
/// independently verify it; midpoint vectors never authorize an exclusion.
pub(crate) fn chord_cone_projection_proposal(generators:&[[[f64;2];3]])->Option<[f64;3]> {
    if generators.is_empty()||generators.len()>16 {return None;}
    let mut sum=[0.;3];
    for g in generators {
        if g.iter().any(|r|!r[0].is_finite()||!r[1].is_finite()||r[0]>r[1]) {return None;}
        let mid=g.map(|r|r[0]*0.5+r[1]*0.5);
        let scale=mid.iter().map(|x|x.abs()).fold(0_f64,f64::max);
        if scale<f64::MIN_POSITIVE {return None;}
        let scaled=mid.map(|x|x/scale);
        let norm=scaled.iter().map(|x|x*x).sum::<f64>().sqrt();
        for k in 0..3 {sum[k]+=scaled[k]/norm;}
    }
    (sum.iter().all(|x|x.is_finite())&&sum.iter().any(|x|*x!=0.)).then_some(sum)
}

#[cfg(test)]
#[path="normal_alignment/tests/positive_chord_cone_tests.rs"]
mod positive_chord_cone_tests;
type Net = Vec<Vec<[I; 4]>>;



/// Positive denominator Bernstein coefficients make a rational polynomial a
/// convex combination of coefficient quotients, including after restriction.
pub(crate) fn rational_quotient_hull(net: &Net, domain: [[f64; 2]; 2]) -> Result<[I; 3]> {
    let restricted = restrict_unit_net(net, domain)?;
    let mut out = [I {
        lo: f64::INFINITY,
        hi: f64::NEG_INFINITY,
    }; 3];
    for h in restricted.iter().flatten() {
        for k in 0..3 {
            let value = h[k].div(h[3])?;
            out[k].lo = out[k].lo.min(value.lo);
            out[k].hi = out[k].hi.max(value.hi);
        }
    }
    Ok(out)
}
/// Signed denominators are allowed only when all restricted coefficients have
/// the same separated sign, so their rational convex weights remain positive.
pub(crate) fn signed_rational_quotient_hull(
    net: &Net,
    domain: [[f64; 2]; 2],
) -> Result<Option<[I; 3]>> {
    let r = restrict_unit_net(net, domain)?;
    let positive = r.iter().flatten().all(|h| h[3].lo > 0.);
    let negative = r.iter().flatten().all(|h| h[3].hi < 0.);
    if !positive && !negative {
        return Ok(None);
    }
    let mut out = [I {
        lo: f64::INFINITY,
        hi: f64::NEG_INFINITY,
    }; 3];
    for h in r.iter().flatten() {
        for k in 0..3 {
            let x = h[k].div_signed(h[3])?;
            out[k].lo = out[k].lo.min(x.lo);
            out[k].hi = out[k].hi.max(x.hi);
        }
    }
    Ok(Some(out))
}
pub(crate) fn jacobian_on(
    s: &Surface,
    indices: [usize; 2],
    domain: [[f64; 2]; 2],
) -> Result<[[I; 2]; 3]> {
    let [u, v] = indices;
    let (p, q) = (s.degree_u, s.degree_v);
    let span = [
        [s.knots_u[u], s.knots_u[u + 1]],
        [s.knots_v[v], s.knots_v[v + 1]],
    ];
    let origin = s.evaluate_validated(domain[0][0]*0.5+domain[0][1]*0.5,
        domain[1][0]*0.5+domain[1][1]*0.5)?.point;
    // Translate original controls before homogeneous multiplication/extraction.
    // The finite origin is only a numerical reference; interval subtraction
    // encloses the exact translated expression and its unchanged derivatives.
    let net = crate::curve_surface_composition::surface_net_on_with_weights_centered(
        s,indices,span,|i,j|Ok(I::point(s.weights[i][j])),
        std::array::from_fn(|k|origin[k]),
    )?;
    jacobian_net_on(net, [p,q], span, domain, &[0.;3])
}
/// Any finite translation cancels from the quotient derivative. The supplied
/// homogeneous coefficients must describe the actual surface being qualified.
pub(crate) fn jacobian_net_on(mut net: Net, [p,q]: [usize;2],
    span:[[f64;2];2], domain:[[f64;2];2], origin:&[f64]) -> Result<[[I;2];3]> {
    for row in &mut net {
        for h in row {
            for k in 0..3 {
                h[k] = h[k].sub(I::point(origin[k]).mul(h[3])?)?;
            }
        }
    }
    let values = restrict_hull(&net, [p, q], span, domain)?;
    let denominator = values[3].mul(values[3])?;
    let mut result = [[I::point(0.); 2]; 3];
    for axis in 0..2 {
        let degree = [p, q][axis];
        if degree == 0 {
            continue;
        }
        let derivatives = differentiate_net(&net, [p, q], axis, span)?;
        let d = restrict_hull(
            &derivatives,
            [p - usize::from(axis == 0), q - usize::from(axis == 1)],
            span,
            domain,
        )?;
        for k in 0..3 {
            result[k][axis] = d[k]
                .mul(values[3])?
                .sub(values[k].mul(d[3])?)?
                .div(denominator)?;
        }
    }
    Ok(result)
}
#[derive(Clone, Copy)]
pub(crate) struct JetBounds {
    pub point: [I; 3],
    pub first: [[I; 3]; 2],
    pub second: [[I; 3]; 3],
}
pub(crate) fn differentiate_net(
    net: &Net,
    degrees: [usize; 2],
    axis: usize,
    span: [[f64; 2]; 2],
) -> Result<Net> {
    let degree = degrees[axis];
    let mut out = vec![
        vec![[I::point(0.); 4]; degrees[1] + 1 - usize::from(axis == 1)];
        degrees[0] + 1 - usize::from(axis == 0)
    ];
    let width = I::point(span[axis][1]).sub(I::point(span[axis][0]))?;
    for i in 0..out.len() {
        for j in 0..out[0].len() {
            for k in 0..4 {
                out[i][j][k] = net[i + usize::from(axis == 0)][j + usize::from(axis == 1)][k]
                    .sub(net[i][j][k])?
                    .mul(I::point(degree as f64))?
                    .div(width)?;
            }
        }
    }
    Ok(out)
}
pub(crate) fn homogeneous_jet_hull(
    net: &Net,
    degrees: [usize; 2],
    span: [[f64; 2]; 2],
    domain: [[f64; 2]; 2],
    order: [usize; 2],
) -> Result<[I; 4]> {
    if (0..2).any(|axis| order[axis] > degrees[axis]) {
        return Ok([I::point(0.); 4]);
    }
    let mut derivative = net.clone();
    let mut degree = degrees;
    for axis in 0..2 {
        for _ in 0..order[axis] {
            derivative = differentiate_net(&derivative, degree, axis, span)?;
            degree[axis] -= 1;
        }
    }
    restrict_hull(&derivative, degree, span, domain)
}
fn jet_on(s: &Surface, indices: [usize; 2], domain: [[f64; 2]; 2]) -> Result<JetBounds> {
    let [u, v] = indices;
    let degrees = [s.degree_u, s.degree_v];
    let span = [
        [s.knots_u[u], s.knots_u[u + 1]],
        [s.knots_v[v], s.knots_v[v + 1]],
    ];
    let origin = &s.control_points[u - degrees[0]][v - degrees[1]];
    let net = crate::curve_surface_composition::surface_net_on_with_weights_centered(
        s,indices,span,|i,j|Ok(I::point(s.weights[i][j])),
        std::array::from_fn(|k|origin[k]),
    )?;
    let mut jet=jet_net_on(net,degrees,span,domain,&[0.;3])?;
    // Derivatives are translation-invariant, but callers still receive the
    // actual original world-position enclosure.
    for k in 0..3 {jet.point[k]=jet.point[k].add(I::point(origin[k]))?;}
    Ok(jet)
}


fn validate_input(
    s: &Surface,
    domain: [[f64; 2]; 2],
    direction: [f64; 3],
    max_sine_squared: f64,
    max_spans: usize,
) -> Result<()> {
    s.validate()?;
    check(
        direction.iter().all(|x| x.is_finite())
            && direction.iter().any(|x| *x != 0.)
            && max_sine_squared.is_finite()
            && (0. ..1.).contains(&max_sine_squared)
            && (1..=100000).contains(&max_spans),
        "Normal alignment requires a finite nonzero direction, squared-sine tolerance in [0,1) and bounded positive spans",
    )?;
    let knots = [&s.knots_u, &s.knots_v];
    let degrees = [s.degree_u, s.degree_v];
    let counts = [s.control_points.len(), s.control_points[0].len()];
    for axis in 0..2 {
        check(
            domain[axis].iter().all(|x| x.is_finite())
                && domain[axis][0] <= domain[axis][1]
                && domain[axis][0] >= knots[axis][degrees[axis]]
                && domain[axis][1] <= knots[axis][counts[axis]],
            "Normal rectangle must lie inside the natural surface domain",
        )?;
    }
    Ok(())
}
/// Encloses unnormalized source normals over every intersecting knot span.
/// None means the span budget did not cover the requested rectangle.
pub(crate) fn validate_rectangle(
    s: &Surface,
    domain: [[f64; 2]; 2],
    max_spans: usize,
) -> Result<()> {
    validate_input(s, domain, [1., 0., 0.], 0., max_spans)
}
/// A derivative enclosure tied to its original source and covered parameter path.
/// Degenerate rectangles allow a boundary line. This is not a body certificate.
pub struct OriginalDerivativeBounds<'a> {
    surface: &'a Surface,
    domain: [[f64; 2]; 2],
    axis: usize,
    components: [[f64; 2]; 3],
}
impl OriginalDerivativeBounds<'_> {
    pub fn surface(&self) -> &Surface { self.surface }
    pub fn domain(&self) -> [[f64; 2]; 2] { self.domain }
    pub fn axis(&self) -> usize { self.axis }
    pub fn components(&self) -> [[f64; 2]; 3] { self.components }
}

/// Original mean derivative along a straight coordinate path from a natural
/// chart boundary to every point of an endpoint rectangle. Private fields keep
/// averaged derivatives distinct from pointwise derivative bounds.
pub struct OriginalBoundaryMean<'a> {
    surface:&'a Surface, endpoints:[[f64;2];2],axis:usize,boundary:f64,
    components:[[f64;2];3],
}
impl OriginalBoundaryMean<'_> {
    pub fn surface(&self)->&Surface {self.surface}
    pub fn endpoints(&self)->[[f64;2];2] {self.endpoints}
    pub fn axis(&self)->usize {self.axis}
    pub fn boundary(&self)->f64 {self.boundary}
    pub fn components(&self)->[[f64;2];3] {self.components}
}
/// Interval integration in path parameter t; this is not sampled quadrature.
/// Every t subinterval encloses its full original derivative and contributes
/// its exact interval width. All intersected source spans count against budget.
pub fn boundary_mean_bounds<'a>(s:&'a Surface,endpoints:[[f64;2];2],axis:usize,
    boundary:f64,pieces:usize,max_spans:usize)->Result<(Option<OriginalBoundaryMean<'a>>,usize)> {
    check(axis<2&&(1..=16).contains(&pieces),"Bound original coordinate mean integration")?;
    validate_rectangle(s,endpoints,max_spans)?;
    let knots=[&s.knots_u,&s.knots_v];let degrees=[s.degree_u,s.degree_v];
    let counts=[s.control_points.len(),s.control_points[0].len()];
    let low=knots[axis][degrees[axis]];let high=knots[axis][counts[axis]];
    check(boundary==low||boundary==high,"Start the mean path at an original natural boundary")?;
    let delta=I::new(endpoints[axis][0],endpoints[axis][1])?.sub(I::point(boundary))?;
    let mut mean=[I::point(0.);3];let mut spans=0;
    for piece in 0..pieces {
        if spans==max_spans {return Ok((None,spans));}
        // Integer division uses outward arithmetic; t coverage has no gaps.
        let a=I::point(piece as f64).div(I::point(pieces as f64))?;
        let b=I::point((piece+1) as f64).div(I::point(pieces as f64))?;
        let t=I::new(a.lo.max(0.),b.hi.min(1.))?;
        let coordinate=I::point(boundary).add(delta.mul(t)?)?.intersect(low,high)?;
        let mut path=endpoints;path[axis]=[coordinate.lo,coordinate.hi];
        let (proof,used)=derivative_path_bounds(s,path,axis,max_spans-spans)?;spans+=used;
        let Some(proof)=proof else{return Ok((None,spans));};
        let width=I::point(1.).div(I::point(pieces as f64))?;
        for (k,r) in proof.components().into_iter().enumerate(){
            mean[k]=mean[k].add(I::new(r[0],r[1])?.mul(width)?)?;
        }
    }
    Ok((Some(OriginalBoundaryMean{surface:s,endpoints,axis,boundary,
        components:mean.map(|r|[r.lo,r.hi])}),spans))
}
pub fn normal_bounds(
    s: &Surface,
    domain: [[f64; 2]; 2],
    max_spans: usize,
) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
    validate_input(s, domain, [1., 0., 0.], 0., max_spans)?;
    normal_bounds_validated(s, domain, max_spans)
}

pub fn inspect(
    s: &Surface,
    domain: [[f64; 2]; 2],
    direction: [f64; 3],
    max_sine_squared: f64,
    max_spans: usize,
) -> Result<Report> {
    validate_input(s, domain, direction, max_sine_squared, max_spans)?;
    let (components, spans) = normal_bounds_validated(s, domain, max_spans)?;
    let out = Report {
        aligned: None,
        sine_squared_interval: None,
        normal_components: components,
        spans,
        reason: "span-limit",
    };
    let Some(hull) = components else {
        return Ok(out);
    };
    let mut report=inspect_normal_enclosure(hull,direction,max_sine_squared)?;
    report.spans=spans;
    Ok(report)
}

/// Angle bounds from a caller-proven normal enclosure. This algebraic report
/// does not admit a surface, material chord or thickness certificate.
pub fn inspect_normal_enclosure(hull:[[f64;2];3],direction:[f64;3],max_sine_squared:f64)->Result<Report> {
    check(hull.iter().all(|r|r.iter().all(|x|x.is_finite()) && r[0]<=r[1])
        && direction.iter().all(|x|x.is_finite()) && direction.iter().any(|x|*x!=0.)
        && max_sine_squared.is_finite() && (0. ..1.).contains(&max_sine_squared),
        "Choose finite normal enclosures, nonzero direction and angle tolerance below one")?;
    let mut out=Report {aligned:None,sine_squared_interval:None,normal_components:Some(hull),spans:0,
        reason:"normal-unresolved"};
    if hull.iter().all(|r| r[0] <= 0. && r[1] >= 0.) {
        out.reason = "normal-unresolved";
        return Ok(out);
    }
    let scale = hull.iter().flatten().map(|x| x.abs()).fold(0_f64, f64::max);
    if scale == 0. {
        out.reason = "normal-unresolved";
        return Ok(out);
    }
    let mut normal = [I::point(0.); 3];
    let mut d = [I::point(0.); 3];
    let ds = direction.iter().map(|x| x.abs()).fold(0_f64, f64::max);
    for k in 0..3 {
        normal[k] = I::new(hull[k][0], hull[k][1])?.div(I::point(scale))?;
        d[k] = I::point(direction[k]).div(I::point(ds))?;
    }
    let denominator = norm_squared(normal)?.mul(norm_squared(d)?)?;
    if denominator.lo <= 0. {
        out.reason = "normal-unresolved";
        return Ok(out);
    }
    let mut cross = [I::point(0.); 3];
    for k in 0..3 {
        let a = (k + 1) % 3;
        let b = (k + 2) % 3;
        cross[k] = normal[a].mul(d[b])?.sub(normal[b].mul(d[a])?)?;
    }
    let sine = norm_squared(cross)?.div(denominator)?;
    let bounds = [sine.lo.max(0.), sine.hi.min(1.)];
    I::new(bounds[0], bounds[1])?;
    out.sine_squared_interval = Some(bounds);
    out.aligned = if bounds[1] <= max_sine_squared {
        Some(true)
    } else if bounds[0] > max_sine_squared {
        Some(false)
    } else {
        None
    };
    out.reason = match out.aligned {
        Some(true) => "angular-tolerance",
        Some(false) => "oblique",
        None => "angular-unresolved",
    };
    Ok(out)
}

/// Angular compatibility over both complete original UV rectangles. This
/// checks normal directions, not positional coincidence or a shared seam map.
#[derive(Debug)]
pub struct PairReport {
    pub aligned: Option<bool>,
    pub sine_squared_interval: Option<[f64; 2]>,
    pub normal_components: [Option<[[f64; 2]; 3]>; 2],
    pub spans: usize,
    pub reason: &'static str,
}
/// Paid complete original normal boxes, bound to one immutable source and pair.
pub(crate) struct OriginalPairBounds<'a> {
    surface: &'a Surface,
    pair: [[[f64; 2]; 2]; 2],
    components: [[[f64; 2]; 3]; 2],
}
impl OriginalPairBounds<'_> {
    pub(crate) fn components(&self, surface: &Surface, pair: [[[f64; 2]; 2]; 2])
        -> Result<[[[f64; 2]; 3]; 2]> {
        check(std::ptr::eq(self.surface,surface) && self.pair==pair,
            "Use original paid normal source and endpoint pair")?;
        Ok(self.components)
    }
}
pub struct OriginalNormalCache<'a> {
    surface: &'a Surface,
    rectangles: std::collections::HashMap<[[u64;2];2],[[f64;2];3]>,
    factored_rectangles: std::collections::HashMap<(bool,[[u64;2];2]),[[f64;2];3]>,
}
/// Complete closed product traversal. An unresolved cell is always retained;
/// angular alignment alone is neither a wall witness nor a thickness proof.
#[derive(Debug)]
pub struct ProductReport {
    pub factored_queries: usize,
    pub excluded: bool,
    pub cells: usize,
    pub spans: usize,
    pub unresolved: Vec<[[[f64;2];2];2]>,
    pub reason: &'static str,
}

#[cfg(test)]
pub(crate) fn inspect_original_pair_cached(
    surface: &Surface, pair: [[[f64; 2]; 2]; 2], sine: f64, budget: usize,
) -> Result<(PairReport, Option<OriginalPairBounds<'_>>)> {
    let (report,cache,_)=OriginalNormalCache::new(surface).inspect_pair(pair,sine,budget)?;
    Ok((report,cache))
}
pub fn inspect_pair(
    surfaces: [&Surface; 2],
    domains: [[[f64; 2]; 2]; 2],
    max_sine_squared: f64,
    max_spans: usize,
) -> Result<PairReport> {
    // Validate both inputs before a budget refusal can hide invalid geometry.
    for side in 0..2 {
        validate_input(
            surfaces[side],
            domains[side],
            [1., 0., 0.],
            max_sine_squared,
            max_spans,
        )?;
    }
    let first = inspect(
        surfaces[0],
        domains[0],
        [1., 0., 0.],
        max_sine_squared,
        max_spans,
    )?;
    let mut out = PairReport {
        aligned: None,
        sine_squared_interval: None,
        normal_components: [first.normal_components, None],
        spans: first.spans,
        reason: "normal-unresolved",
    };
    if first.spans == max_spans {
        out.reason = "span-limit";
        return Ok(out);
    }
    let second = inspect(
        surfaces[1],
        domains[1],
        [1., 0., 0.],
        max_sine_squared,
        max_spans - first.spans,
    )?;
    out.spans += second.spans;
    out.normal_components[1] = second.normal_components;
    if first.reason == "span-limit" || second.reason == "span-limit" {
        out.reason = "span-limit";
        return Ok(out);
    }
    let (Some(a), Some(b)) = (out.normal_components[0], out.normal_components[1]) else {
        return Ok(out);
    };
    let mut bounded = inspect_pair_bounds([a, b], max_sine_squared)?;
    bounded.spans = out.spans;
    Ok(bounded)
}
/// Compare already proved normal enclosures. This performs no source queries
/// and does not itself certify ownership or positional agreement.
pub(crate) fn inspect_pair_bounds(
    components: [[[f64; 2]; 3]; 2],
    max_sine_squared: f64,
) -> Result<PairReport> {
    check(
        max_sine_squared.is_finite()
            && (0. ..1.).contains(&max_sine_squared)
            && components
                .iter()
                .flatten()
                .all(|x| x[0].is_finite() && x[1].is_finite() && x[0] <= x[1]),
        "Finite ordered normal enclosures and squared-sine threshold",
    )?;
    let mut out = PairReport {
        aligned: None,
        sine_squared_interval: None,
        normal_components: [Some(components[0]), Some(components[1])],
        spans: 0,
        reason: "normal-unresolved",
    };
    let mut normals = [[I::point(0.); 3]; 2];
    for side in 0..2 {
        let Some(hull) = out.normal_components[side] else {
            return Ok(out);
        };
        if hull.iter().all(|r| r[0] <= 0. && r[1] >= 0.) {
            return Ok(out);
        }
        let scale = hull.iter().flatten().map(|x| x.abs()).fold(0_f64, f64::max);
        if scale < f64::MIN_POSITIVE {
            return Ok(out);
        }
        for k in 0..3 {
            normals[side][k] = I::new(hull[k][0], hull[k][1])?.div(I::point(scale))?;
        }
    }
    let denominator = norm_squared(normals[0])?.mul(norm_squared(normals[1])?)?;
    if denominator.lo <= 0. {
        return Ok(out);
    }
    let mut cross = [I::point(0.); 3];
    for k in 0..3 {
        let a = (k + 1) % 3;
        let b = (k + 2) % 3;
        cross[k] = normals[0][a]
            .mul(normals[1][b])?
            .sub(normals[0][b].mul(normals[1][a])?)?;
    }
    let sine = norm_squared(cross)?.div(denominator)?;
    let bounds = [sine.lo.max(0.), sine.hi.min(1.)];
    I::new(bounds[0], bounds[1])?;
    out.sine_squared_interval = Some(bounds);
    out.aligned = if bounds[1] <= max_sine_squared {
        Some(true)
    } else if bounds[0] > max_sine_squared {
        Some(false)
    } else {
        None
    };
    out.reason = match out.aligned {
        Some(true) => "angular-tolerance",
        Some(false) => "oblique",
        None => "angular-unresolved",
    };
    Ok(out)
}


/// Exclude every chord in two point enclosures from the required endpoint
/// normal angle. Callers must bind these enclosures to their original domains.
/// Coordinate overlap alone never excludes a chord. Incompatible endpoint
/// normal cones can exclude all nonzero directions despite such overlap.
pub fn endpoint_chord_excluded(
    points: [[[f64; 2]; 3]; 2],
    normals: [Option<[[f64; 2]; 3]>; 2],
    max_sine_squared: f64,
) -> Result<bool> {
    check(max_sine_squared.is_finite() && (0. ..1.).contains(&max_sine_squared)
        && points.iter().flatten().all(|r|r[0].is_finite() && r[1].is_finite() && r[0]<=r[1])
        && normals.iter().flatten().flatten().all(|r|r[0].is_finite() && r[1].is_finite() && r[0]<=r[1]),
        "Finite original point enclosures and endpoint angular tolerance")?;
    if max_sine_squared < 0.5 {
        if let [Some(a),Some(b)] = normals {
            // Two unoriented normals within alpha of one line must lie within
            // 2*alpha of each other; sin^2(2*alpha)=4*s*(1-s).
            let s=I::point(max_sine_squared);
            let threshold=I::point(4.).mul(s)?.mul(I::point(1.).sub(s)?)?.hi;
            if threshold < 1. {
                match inspect_pair_bounds([a,b],threshold) {
                    Ok(report) if report.aligned==Some(false)=>return Ok(true),
                    Ok(_)=>{},
                    Err(error) if error.code=="NURBS_NUMERIC_ERROR"=>{},
                    Err(error)=>return Err(error),
                }
            }
        }
    }
    let mut chord=[[0.;2];3];
    for k in 0..3 {
        let difference=I::new(points[1][k][0],points[1][k][1])?
            .sub(I::new(points[0][k][0],points[0][k][1])?)?;
        chord[k]=[difference.lo,difference.hi];
    }
    for normal in normals.into_iter().flatten() {
        match inspect_pair_bounds([normal,chord],max_sine_squared) {
            Ok(report) if report.aligned==Some(false)=>return Ok(true),
            Ok(_)=>{},
            Err(error) if error.code=="NURBS_NUMERIC_ERROR"=>{},
            Err(error)=>return Err(error),
        }
    }
    Ok(false)
}

#[cfg(test)]
#[path = "normal_alignment/tests/mod.rs"]
mod tests;

#[path="normal_alignment/methods_OriginalNormalCache_a__new.rs"]
mod methods_OriginalNormalCache_a__new;
#[path="normal_alignment/algorithm_restrict_net.rs"]
mod algorithm_restrict_net;
use algorithm_restrict_net::restrict_net;
#[path="normal_alignment/algorithm_restrict_hull.rs"]
mod algorithm_restrict_hull;
pub(crate) use algorithm_restrict_hull::restrict_hull;
#[path="normal_alignment/algorithm_restrict_unit_net.rs"]
mod algorithm_restrict_unit_net;
use algorithm_restrict_unit_net::restrict_unit_net;
#[path="normal_alignment/algorithm_jet_net_on.rs"]
mod algorithm_jet_net_on;
pub(crate) use algorithm_jet_net_on::jet_net_on;
#[path="normal_alignment/algorithm_jet_bounds.rs"]
mod algorithm_jet_bounds;
pub(crate) use algorithm_jet_bounds::jet_bounds;
#[path="normal_alignment/algorithm_derivative_path_bounds.rs"]
mod algorithm_derivative_path_bounds;
pub use algorithm_derivative_path_bounds::derivative_path_bounds;
#[path="normal_alignment/algorithm_normal_bounds_validated.rs"]
mod algorithm_normal_bounds_validated;
use algorithm_normal_bounds_validated::normal_bounds_validated;
