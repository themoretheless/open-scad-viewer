//! Complete-rectangle normal/line angle bounds, including both sides of knots.
pub use crate::surface_boundary_secant::{OriginalBoundarySecant,boundary_secant_bounds,OriginalBoundaryDirection,boundary_secant_direction_bounds};
pub use crate::surface_weight_profile::{WeightProfile,ProfileDirection,ProfileBoundary,ProfilePathReport,ProfilePoleLimits,ProfileSection,ProfileEndTransition,ProfileTransitionRail,qualify_weight_profile};
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
mod positive_chord_cone_tests {
    use super::positive_chord_cone_excluded as excluded;
    #[test]
    fn covers_positive_combinations_and_refuses_cancellation_or_alignment() {
        let n=[[0.,0.],[0.,0.],[1.,1.]];
        let g=[[[1.,2.],[-2.,2.],[-0.01,0.01]],[[2.,3.],[-1.,1.],[-0.02,0.02]]];
        assert!(excluded(&g,n,[1.,0.,0.],1e-6).unwrap());
        assert!(!excluded(&g,n,[-1.,0.,0.],1e-6).unwrap());
        assert!(!excluded(&[[[0.,0.],[0.,0.],[1.,2.]]],n,[0.,0.,1.],1e-6).unwrap());
        assert!(!excluded(&[[[-1.,1.],[0.,0.],[0.,0.]]],n,[1.,0.,0.],1e-6).unwrap());
        assert!(!excluded(&g,[[0.,0.];3],[1.,0.,0.],1e-6).unwrap());
    }
    #[test]
    fn shared_tangent_is_free_but_transverse_cancellation_refuses() {
        let point=|p:[f64;3]|p.map(|x|[x,x]);
        let tangent=point([1.,0.,0.]);let up=point([0.,1.,0.]);let down=point([0.,-1.,0.]);let n=point([0.,0.,1.]);
        assert!(super::shared_chord_cone_excluded(tangent,[up,up],n,1e-6).unwrap());
        assert!(!super::shared_chord_cone_excluded(tangent,[up,down],n,1e-6).unwrap());
        assert!(!super::shared_chord_cone_excluded(tangent,[up,up],up,1e-6).unwrap());
        assert!(!super::shared_chord_cone_excluded(tangent,[up,up],tangent,1e-6).unwrap());
        assert!(!super::shared_chord_cone_excluded([[0.,0.];3],[up,up],n,1e-6).unwrap());
        assert!(super::shared_chord_cone_excluded(tangent,[up,up],[[f64::NAN,1.],[0.,0.],[1.,1.]],1e-6).is_err());
    }
    #[test]
    fn shared_cone_bounds_include_nonorthogonal_and_uncertain_paths() {
        let c=[[1.,1.],[-0.01,0.01],[0.,0.]];
        let a=[[0.,0.],[1.,1.],[-0.001,0.001]];let n=[[0.,0.],[0.,0.],[1.,1.]];
        assert!(super::shared_chord_cone_excluded(c,[a,a],n,1e-6).unwrap());
        let wide=[[0.,0.],[-1.,1.],[0.,0.]];
        assert!(!super::shared_chord_cone_excluded(c,[wide,a],n,1e-6).unwrap());
        assert!(!super::shared_chord_cone_excluded(c,[a,a],[[0.,0.];3],1e-6).unwrap());
    }
    #[test]
    fn proposed_bisector_covers_narrow_cone_missed_by_integer_directions() {
        let g=[[[1.,1.],[0.,0.],[0.,0.]],[[-0.995,-0.995],[0.1,0.1],[0.,0.]]];
        let normal=[[0.,0.],[0.,0.],[1.,1.]];
        for x in -1..=1 {for y in -1..=1 {for z in -1..=1 {
            assert!(!excluded(&g,normal,[x as f64,y as f64,z as f64],1e-6).unwrap());
        }}}
        let proposal=super::chord_cone_projection_proposal(&g).unwrap();
        assert!(excluded(&g,normal,proposal,1e-6).unwrap());
        let mut uncertain=g;uncertain[0][0]=[-1.,1.];
        assert!(!excluded(&uncertain,normal,proposal,1e-6).unwrap());
        assert!(super::chord_cone_projection_proposal(&[[[0.,0.];3]]).is_none());
        assert!(super::chord_cone_projection_proposal(&[[[f64::NAN,1.],[0.,0.],[0.,0.]]]).is_none());
    }
    #[test]
    fn validates_all_generators_before_any_early_refusal() {
        let g=[[[0.,0.];3],[[f64::NAN,1.],[0.,0.],[0.,0.]]];
        assert!(excluded(&g,[[0.,0.],[0.,0.],[1.,1.]],[1.,0.,0.],1e-6).is_err());
        assert!(excluded(&[],[[0.,0.];3],[1.,0.,0.],1e-6).is_err());
    }
}
type Net = Vec<Vec<[I; 4]>>;
fn restrict_net(
    net: &Net,
    degrees: [usize; 2],
    span: [[f64; 2]; 2],
    domain: [[f64; 2]; 2],
) -> Result<Net> {
    let [p, q] = degrees;
    let ku = [vec![span[0][0]; p + 1], vec![span[0][1]; p + 1]].concat();
    let kv = [vec![span[1][0]; q + 1], vec![span[1][1]; q + 1]].concat();
    let weights = [
        net.iter()
            .flatten()
            .map(|h| h[3].lo)
            .fold(f64::INFINITY, f64::min),
        net.iter()
            .flatten()
            .map(|h| h[3].hi)
            .fold(f64::NEG_INFINITY, f64::max),
    ];
    let mut result = vec![vec![[I::point(0.); 4]; q + 1]; p + 1];
    for v in 0..=q {
        let vs = (0..q)
            .map(|k| domain[1][usize::from(k >= q - v)])
            .collect::<Vec<_>>();
        let rows = net
            .iter()
            .map(|row| crate::surface_distance::interpolate(row.clone(), q, &kv, q, &vs, weights))
            .collect::<Result<Vec<_>>>()?;
        for u in 0..=p {
            let us = (0..p)
                .map(|k| domain[0][usize::from(k >= p - u)])
                .collect::<Vec<_>>();
            let h = crate::surface_distance::interpolate(rows.clone(), p, &ku, p, &us, weights)?;
            result[u][v] = h;
        }
    }
    Ok(result)
}
pub(crate) fn restrict_hull(
    net: &Net,
    degrees: [usize; 2],
    span: [[f64; 2]; 2],
    domain: [[f64; 2]; 2],
) -> Result<[I; 4]> {
    check((0..2).all(|axis| span[axis][0].is_finite() && span[axis][1].is_finite()
        && span[axis][0]<span[axis][1] && domain[axis][0].is_finite()
        && domain[axis][1].is_finite() && span[axis][0]<=domain[axis][0]
        && domain[axis][0]<=domain[axis][1] && domain[axis][1]<=span[axis][1]),
        "Restriction must lie in its original positive closed span")?;
    // The extracted coefficients use the original span's Bernstein basis.
    // Expand normalized endpoints outward, so the unit restriction contains
    // the requested original closed rectangle, including point rectangles.
    let mut normalized=[[0.;2];2];
    let mut normalize=true;
    for axis in 0..2 {
        let length=I::point(span[axis][1]).sub(I::point(span[axis][0]))?;
        if length.lo<=0. {normalize=false;break;}
        for end in 0..2 {
            normalized[axis][end]=if domain[axis][end]==span[axis][0] {0.}
                else if domain[axis][end]==span[axis][1] {1.}
                else {
                    let value=I::point(domain[axis][end]).sub(I::point(span[axis][0]))?
                        .div(length)?;
                    if end==0 {value.lo.max(0.)} else {value.hi.min(1.)}
                };
        }
    }
    let restricted=if normalize {restrict_unit_net(net,normalized)?}
        else {restrict_net(net,degrees,span,domain)?};
    Ok(std::array::from_fn(|k| I {
        lo: restricted
            .iter()
            .flatten()
            .map(|h| h[k].lo)
            .fold(f64::INFINITY, f64::min),
        hi: restricted
            .iter()
            .flatten()
            .map(|h| h[k].hi)
            .fold(f64::NEG_INFINITY, f64::max),
    }))
}
/// Restrict all coefficients together by two de Casteljau splits per axis.
/// This keeps closed endpoints and avoids a separate blossom for every cell.
fn restrict_unit_net(net: &Net, domain: [[f64; 2]; 2]) -> Result<Net> {
    check(
        domain.iter().all(|d| {
            d[0].is_finite() && d[1].is_finite() && 0. <= d[0] && d[0] <= d[1] && d[1] <= 1.
        }),
        "Closed unit polynomial rectangle",
    )?;
    fn split(row: &[[I; 4]], t: I) -> Result<(Vec<[I; 4]>, Vec<[I; 4]>)> {
        let t = t.intersect(0., 1.)?;
        let beta = I::point(1.).sub(t)?.intersect(0., 1.)?;
        let mut current = row.to_vec();
        let mut left = vec![current[0]];
        let mut right = vec![*current.last().unwrap()];
        while current.len() > 1 {
            let mut next = Vec::new();
            for pair in current.windows(2) {
                let mut h = [I::point(0.); 4];
                for k in 0..4 {
                    h[k] = pair[0][k].mul(beta)?.add(pair[1][k].mul(t)?)?.intersect(
                        pair[0][k].lo.min(pair[1][k].lo),
                        pair[0][k].hi.max(pair[1][k].hi),
                    )?;
                }
                next.push(h);
            }
            current = next;
            left.push(current[0]);
            right.push(*current.last().unwrap());
        }
        right.reverse();
        Ok((left, right))
    }
    fn row_restrict(row: &[[I; 4]], d: [f64; 2]) -> Result<Vec<[I; 4]>> {
        if d == [0., 1.] {
            return Ok(row.to_vec());
        }
        if d[0] == d[1] {
            let value = *split(row, I::point(d[0]))?.0.last().unwrap();
            return Ok(vec![value; row.len()]);
        }
        let left = if d[1] == 1. {
            row.to_vec()
        } else {
            split(row, I::point(d[1]))?.0
        };
        if d[0] == 0. {
            return Ok(left);
        }
        Ok(split(&left, I::point(d[0]).div(I::point(d[1]))?)?.1)
    }
    let mut out = net.clone();
    for v in 0..net[0].len() {
        let row = net.iter().map(|r| r[v]).collect::<Vec<_>>();
        let restricted = row_restrict(&row, domain[0])?;
        for u in 0..net.len() {
            out[u][v] = restricted[u];
        }
    }
    for row in &mut out {
        *row = row_restrict(row, domain[1])?;
    }
    Ok(out)
}
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
    let net = crate::curve_surface_composition::surface_net_on(s, indices, span)?;
    let origin = s.evaluate_validated(domain[0][0]*0.5+domain[0][1]*0.5,
        domain[1][0]*0.5+domain[1][1]*0.5)?.point;
    jacobian_net_on(net, [p,q], span, domain, &origin)
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
fn homogeneous_jet_hull(
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
    let mut net = crate::curve_surface_composition::surface_net_on(s, indices, span)?;
    let origin = &s.control_points[u - degrees[0]][v - degrees[1]];
    for row in &mut net {
        for h in row {
            for k in 0..3 {
                h[k] = h[k].sub(I::point(origin[k]).mul(h[3])?)?;
            }
        }
    }
    let mut h = [[I::point(0.); 4]; 6];
    for (i, order) in [[0, 0], [1, 0], [0, 1], [2, 0], [1, 1], [0, 2]]
        .into_iter()
        .enumerate()
    {
        h[i] = homogeneous_jet_hull(&net, degrees, span, domain, order)?;
    }
    let mut out = JetBounds {
        point: [I::point(0.); 3],
        first: [[I::point(0.); 3]; 2],
        second: [[I::point(0.); 3]; 3],
    };
    for k in 0..3 {
        let relative = h[0][k].div(h[0][3])?;
        out.point[k] = relative.add(I::point(origin[k]))?;
        for axis in 0..2 {
            out.first[axis][k] = h[axis + 1][k]
                .sub(relative.mul(h[axis + 1][3])?)?
                .div(h[0][3])?;
        }
        for (slot, a, b) in [(0, 0, 0), (1, 0, 1), (2, 1, 1)] {
            out.second[slot][k] = h[slot + 3][k]
                .sub(out.first[a][k].mul(h[b + 1][3])?)?
                .sub(out.first[b][k].mul(h[a + 1][3])?)?
                .sub(relative.mul(h[slot + 3][3])?)?
                .div(h[0][3])?;
        }
    }
    Ok(out)
}
/// All incident span-side jets over the rectangle. Their union does not prove
/// source continuity across repeated knots; root inclusion must check that separately.
pub(crate) fn jet_bounds(
    s: &Surface,
    domain: [[f64; 2]; 2],
    max_spans: usize,
) -> Result<(Option<JetBounds>, usize)> {
    validate_input(s, domain, [1., 0., 0.], 0., max_spans)?;
    let mut out: Option<JetBounds> = None;
    let mut spans = 0;
    let union = |a: &mut I, b: I| {
        a.lo = a.lo.min(b.lo);
        a.hi = a.hi.max(b.hi);
    };
    for u in s.degree_u..s.control_points.len() {
        for v in s.degree_v..s.control_points[0].len() {
            let mut section = [[0.; 2]; 2];
            let indices = [u, v];
            let knots = [&s.knots_u, &s.knots_v];
            let mut outside = false;
            for axis in 0..2 {
                let i = indices[axis];
                let lo = knots[axis][i];
                let hi = knots[axis][i + 1];
                if lo == hi || domain[axis][1] < lo || domain[axis][0] > hi {
                    outside = true;
                    break;
                }
                section[axis] = [lo.max(domain[axis][0]), hi.min(domain[axis][1])];
            }
            if outside {
                continue;
            }
            if spans == max_spans {
                return Ok((None, spans));
            }
            spans += 1;
            let next = jet_on(s, indices, section)?;
            if let Some(current) = &mut out {
                for k in 0..3 {
                    union(&mut current.point[k], next.point[k]);
                    for axis in 0..2 {
                        union(&mut current.first[axis][k], next.first[axis][k]);
                    }
                    for slot in 0..3 {
                        union(&mut current.second[slot][k], next.second[slot][k]);
                    }
                }
            } else {
                out = Some(next);
            }
        }
    }
    check(spans > 0, "Jet rectangle has no nonempty knot spans")?;
    Ok((out, spans))
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
/// Cover every intersected nonempty knot rectangle, including both one-sided
/// derivatives at knots. Reject jumps across the integrated coordinate.
pub fn derivative_path_bounds<'a>(s: &'a Surface, domain: [[f64; 2]; 2],
    axis: usize, max_spans: usize) -> Result<(Option<OriginalDerivativeBounds<'a>>, usize)> {
    check(axis < 2, "Choose an original derivative coordinate")?;
    validate_rectangle(s, domain, max_spans)?;
    let degrees = [s.degree_u, s.degree_v];
    let counts = [s.control_points.len(), s.control_points[0].len()];
    let knots = [&s.knots_u, &s.knots_v];
    if degrees[axis] == 0 { return Ok((None, 0)); }
    let mut i = degrees[axis] + 1;
    while i < counts[axis] {
        let knot = knots[axis][i];
        let mut end = i + 1;
        while end < knots[axis].len() && knots[axis][end] == knot { end += 1; }
        if domain[axis][0] <= knot && knot <= domain[axis][1] && end-i > degrees[axis] {
            return Ok((None, 0));
        }
        i = end;
    }
    let mut hull = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    let mut spans = 0;
    for u in degrees[0]..counts[0] {
        for v in degrees[1]..counts[1] {
            if knots[0][u] == knots[0][u+1] || knots[1][v] == knots[1][v+1] { continue; }
            let indices = [u,v];
            let section = std::array::from_fn(|a| [domain[a][0].max(knots[a][indices[a]]),
                domain[a][1].min(knots[a][indices[a]+1])]);
            if section.iter().any(|a| a[0] > a[1]) { continue; }
            if spans == max_spans { return Ok((None, spans)); }
            let j = jacobian_on(s, indices, section)?;
            spans += 1;
            for k in 0..3 {
                hull[k][0] = hull[k][0].min(j[k][axis].lo);
                hull[k][1] = hull[k][1].max(j[k][axis].hi);
            }
        }
    }
    if spans == 0 { return Ok((None, 0)); }
    Ok((Some(OriginalDerivativeBounds { surface:s, domain, axis, components:hull }), spans))
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
fn normal_bounds_validated(
    s: &Surface,
    domain: [[f64; 2]; 2],
    max_spans: usize,
) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
    let degrees = [s.degree_u, s.degree_v];
    let knots = [&s.knots_u, &s.knots_v];
    let counts = [s.control_points.len(), s.control_points[0].len()];
    let mut spans = 0;
    let mut hull = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for u in degrees[0]..counts[0] {
        for v in degrees[1]..counts[1] {
            let indices = [u, v];
            let mut section = [[0.; 2]; 2];
            let mut outside = false;
            for axis in 0..2 {
                let i = indices[axis];
                let lo = knots[axis][i];
                let hi = knots[axis][i + 1];
                if lo == hi || domain[axis][1] < lo || domain[axis][0] > hi {
                    outside = true;
                    break;
                }
                section[axis] = [lo.max(domain[axis][0]), hi.min(domain[axis][1])];
            }
            if outside {
                continue;
            }
            if spans == max_spans {
                return Ok((None, spans));
            }
            spans += 1;
            // Differentiate the original span first, then restrict the derivative
            // polynomial. Narrow/point rectangles never divide by their own width.
            let j = jacobian_on(s, indices, section)?;
            for k in 0..3 {
                let a = (k + 1) % 3;
                let b = (k + 2) % 3;
                let n = j[a][0].mul(j[b][1])?.sub(j[b][0].mul(j[a][1])?)?;
                hull[k][0] = hull[k][0].min(n.lo);
                hull[k][1] = hull[k][1].max(n.hi);
            }
        }
    }
    check(spans > 0, "Normal rectangle has no nonempty knot spans")?;
    Ok((Some(hull), spans))
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
    let mut out = Report {
        aligned: None,
        sine_squared_interval: None,
        normal_components: components,
        spans,
        reason: "span-limit",
    };
    let Some(hull) = components else {
        return Ok(out);
    };
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
impl<'a> OriginalNormalCache<'a> {
    pub fn new(surface:&'a Surface)->Self {
        Self{surface,rectangles:std::collections::HashMap::new(),factored_rectangles:std::collections::HashMap::new()}
    }
    pub fn inspect_product(&mut self, other:&mut Self,
        domains:[[[f64;2];2];2], sine:f64, max_cells:usize, max_spans:usize)
        ->Result<ProductReport> {
        self.inspect_product_with_factors(other,domains,sine,max_cells,max_spans,[None,None],0)
    }
    /// Closed normal-direction coverage using exact source-bound limiting factors.
    /// Factored directions do not establish a regular tangent plane at a pole.
    pub fn inspect_product_with_factors(&mut self, other:&mut Self,
        domains:[[[f64;2];2];2], sine:f64, max_cells:usize, max_spans:usize,
        factors:[Option<&crate::factored_surface_normals::Certificate<'_>>;2],
        max_factored_queries:usize)->Result<ProductReport> {
        check(max_factored_queries<=100000,"Factored normal query budget")?;
        for (surface,factor) in [self.surface,other.surface].into_iter().zip(factors) {
            if let Some(c)=factor {check(std::ptr::eq(surface,c.surface()),"Use original factored normal source")?;}
        }
        check((1..=100000).contains(&max_cells),"Normal product cell budget")?;
        validate_input(self.surface,domains[0],[1.,0.,0.],sine,max_spans.max(1))?;
        validate_input(other.surface,domains[1],[1.,0.,0.],sine,max_spans.max(1))?;
        let mut out=ProductReport{excluded:false,cells:0,spans:0,factored_queries:0,
            unresolved:vec![domains],reason:"product-cell-limit"};
        while out.cells<max_cells && !out.unresolved.is_empty() {
            let pair=out.unresolved.pop().unwrap();out.cells+=1;
            let (r,queries)=self.inspect_other_with_factors(other,pair,sine,max_spans-out.spans,factors,max_factored_queries-out.factored_queries)?;
            out.factored_queries+=queries;
            out.spans+=r.spans;
            if r.aligned==Some(false) {continue;}
            if r.aligned==Some(true) {
                out.unresolved.push(pair);out.reason="product-aligned-region";break;
            }
            if (factors.iter().any(Option::is_none) && out.spans==max_spans) ||
                (factors.iter().any(Option::is_some) && out.factored_queries==max_factored_queries && r.normal_components.iter().any(Option::is_none)) {
                out.unresolved.push(pair);out.reason="product-span-limit";break;
            }
            let mut choice=None;let mut largest=0.;
            for side in 0..2 {for axis in 0..2 {
                let full=domains[side][axis][1]-domains[side][axis][0];
                if full<=0. {continue;}
                let width=(pair[side][axis][1]-pair[side][axis][0])/full;
                if width>largest {largest=width;choice=Some((side,axis));}
            }}
            let Some((side,axis))=choice else {
                out.unresolved.push(pair);out.reason="product-resolution-limit";break;
            };
            let d=pair[side][axis];let mid=d[0]*0.5+d[1]*0.5;
            if !(d[0]<mid && mid<d[1]) {
                out.unresolved.push(pair);out.reason="product-resolution-limit";break;
            }
            let mut left=pair;let mut right=pair;
            left[side][axis][1]=mid;right[side][axis][0]=mid;
            out.unresolved.push(right);out.unresolved.push(left);
        }
        if out.unresolved.is_empty() {
            out.excluded=true;out.reason="product-normal-excluded";
        }
        Ok(out)
    }
    /// Compare complete original rectangles, charging only new source queries.
    /// Private source references and cache keys prevent rebinding paid hulls.
    pub fn inspect_other(&mut self, other: &mut Self,
        domains: [[[f64; 2]; 2]; 2], sine: f64, budget: usize) -> Result<PairReport> {
        Ok(self.inspect_other_with_factors(other,domains,sine,budget,[None,None],0)?.0)
    }
    pub fn inspect_other_with_factors(&mut self, other:&mut Self,
        domains:[[[f64;2];2];2], sine:f64, budget:usize,
        factors:[Option<&crate::factored_surface_normals::Certificate<'_>>;2],
        query_budget:usize)->Result<(PairReport,usize)> {
        check(query_budget<=100000,"Bound original factored normal queries")?;
        for (surface,factor) in [self.surface,other.surface].into_iter().zip(factors) {
            if let Some(c)=factor {check(std::ptr::eq(surface,c.surface()),"Use original factored normal source")?;}
        }
        validate_input(self.surface,domains[0],[1.,0.,0.],sine,budget.max(1))?;
        validate_input(other.surface,domains[1],[1.,0.,0.],sine,budget.max(1))?;
        let mut spent=0; let mut queries=0; let mut components=[None;2];
        for (side,cache) in [self,other].into_iter().enumerate() {
            let rect=domains[side];let key=rect.map(|axis|axis.map(f64::to_bits));
            if let Some(c)=factors[side] {
                let factor_key=(c.end(),key);
                if let Some(hull)=cache.factored_rectangles.get(&factor_key) {components[side]=Some(*hull);}
                else if queries<query_budget {
                    let hull=c.bounds(rect)?;queries+=1;
                    cache.factored_rectangles.insert(factor_key,hull);components[side]=Some(hull);
                }
            } else if let Some(hull)=cache.rectangles.get(&key) {components[side]=Some(*hull);}
            else if spent<budget {
                let (hull,used)=normal_bounds(cache.surface,rect,budget-spent)?;
                spent+=used;components[side]=hull;
                if let Some(hull)=hull {cache.rectangles.insert(key,hull);}
            }
        }
        let [Some(a),Some(b)]=components else {
            return Ok((PairReport{aligned:None,sine_squared_interval:None,
                normal_components:components,spans:spent,reason:"span-limit"},queries));
        };
        let mut report=inspect_pair_bounds([a,b],sine)?;report.spans=spent;Ok((report,queries))
    }
    pub(crate) fn inspect_pair(&mut self,pair:[[[f64;2];2];2],sine:f64,budget:usize)
        ->Result<(PairReport,Option<OriginalPairBounds<'a>>,usize)> {
        // Validate both original rectangles even if no new query is needed.
        for rect in pair {validate_input(self.surface,rect,[1.,0.,0.],sine,budget.max(1))?;}
        let mut components=[None;2];let mut spent=0;let mut hits=0;
        for (index,rect) in pair.into_iter().enumerate() {
            let key=rect.map(|axis|axis.map(f64::to_bits));
            if let Some(bounds)=self.rectangles.get(&key) {
                components[index]=Some(*bounds);hits+=1;
            } else if spent<budget {
                let (bounds,used)=normal_bounds(self.surface,rect,budget-spent)?;
                spent+=used;components[index]=bounds;
                if let Some(bounds)=bounds {self.rectangles.insert(key,bounds);}
            }
        }
        let [Some(a),Some(b)]=components else {
            return Ok((PairReport{aligned:None,sine_squared_interval:None,
                normal_components:components,spans:spent,reason:"span-limit"},None,hits));
        };
        let mut report=inspect_pair_bounds([a,b],sine)?;report.spans=spent;
        Ok((report,Some(OriginalPairBounds{surface:self.surface,pair,components:[a,b]}),hits))
    }
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
/// Overlap, zero direction and numeric uncertainty never exclude a chord.
pub fn endpoint_chord_excluded(
    points: [[[f64; 2]; 3]; 2],
    normals: [Option<[[f64; 2]; 3]>; 2],
    max_sine_squared: f64,
) -> Result<bool> {
    check(max_sine_squared.is_finite() && (0. ..1.).contains(&max_sine_squared)
        && points.iter().flatten().all(|r|r[0].is_finite() && r[1].is_finite() && r[0]<=r[1])
        && normals.iter().flatten().flatten().all(|r|r[0].is_finite() && r[1].is_finite() && r[0]<=r[1]),
        "Finite original point enclosures and endpoint angular tolerance")?;
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
mod tests {
    #[test]
    fn endpoint_chord_requires_continuous_direction_exclusion() {
        use super::endpoint_chord_excluded as excluded;
        let origin=[[0.,0.];3];
        let normal=Some([[0.,0.],[0.,0.],[1.,1.]]);
        assert!(excluded([origin,[[2.,3.],[0.,0.],[0.,0.]]],[normal,normal],1e-6).unwrap());
        assert!(!excluded([origin,[[0.,0.],[0.,0.],[2.,3.]]],[normal,normal],1e-6).unwrap());
        assert!(!excluded([origin,[[-1.,1.];3]],[normal,normal],1e-6).unwrap());
        assert!(!excluded([origin,[[2.,3.],[0.,0.],[0.,0.]]],[None,None],1e-6).unwrap());
        assert!(excluded([origin,[[2.,3.],[0.,0.],[0.,0.]]],[None,normal],1e-6).unwrap());
        assert!(excluded([origin,[[f64::NAN,3.],[0.,0.],[0.,0.]]],[normal,normal],1e-6).is_err());
    }

    #[test]
    fn chord_angle_boundary_and_degenerate_cones_remain_unproved() {
        use super::endpoint_chord_excluded as excluded;
        let origin=[[0.,0.];3];
        let normal=Some([[0.,0.],[0.,0.],[1.,1.]]);
        let diagonal=[[1.,1.],[0.,0.],[1.,1.]];
        assert!(excluded([origin,diagonal],[normal,None],0.49).unwrap());
        assert!(!excluded([origin,diagonal],[normal,None],0.5).unwrap());
        assert!(!excluded([origin,diagonal],[normal,None],0.51).unwrap());
        assert!(!excluded([origin,diagonal],[Some([[0.,0.];3]),None],0.49).unwrap());
        assert!(!excluded([origin,diagonal],[Some([[-1.,1.];3]),None],0.49).unwrap());
        let bad=Some([[f64::NAN,1.],[0.,0.],[1.,1.]]);
        assert!(excluded([origin,[[2.,2.],[0.,0.],[0.,0.]]],[normal,bad],0.49).is_err());
        for scale in [1e-90,1.,1e90] {
            let along=[[0.,0.],[0.,0.],[scale,scale]];
            let across=[[scale,scale],[0.,0.],[0.,0.]];
            let n=Some([[0.,0.],[0.,0.],[-scale,-scale]]);
            assert!(excluded([origin,across],[n,None],1e-6).unwrap());
            assert!(!excluded([origin,along],[n,None],1e-6).unwrap());
            assert!(excluded([across,origin],[n,None],1e-6).unwrap());
        }
    }

    #[test]
    fn locally_centered_rational_derivatives_enclose_independent_evaluations() {
        for offset in [0.,1_000_000.] {
            let s=Surface{degree_u:2,degree_v:2,knots_u:vec![0.,0.,0.,1.,1.,1.],knots_v:vec![0.,0.,0.,1.,1.,1.],
                control_points:(0..3).map(|u|(0..3).map(|v|vec![offset+u as f64,offset+v as f64,offset+(u*v) as f64]).collect()).collect(),
                weights:vec![vec![1.,0.7,1.],vec![0.9,0.63,0.9],vec![1.,0.7,1.]],periodic_u:false,periodic_v:false};
            for domain in [[[0.1,0.3],[0.7,0.9]],[[0.49,0.51],[0.49,0.51]]] {
                let j=super::jacobian_on(&s,[2,2],domain).unwrap();
                for u in 0..=8 {for v in 0..=8 {
                    let t=|a:[f64;2],n:usize|a[0]+(a[1]-a[0])*n as f64/8.;
                    let (du,dv)=s.evaluate(t(domain[0],u),t(domain[1],v)).unwrap().first_derivatives().unwrap();
                    for k in 0..3 {for a in 0..2 {let d=[du,dv][a][k];assert!(j[k][a].lo<=d&&d<=j[k][a].hi);}}
                }}
            }
        }
    }

    use super::*;
    #[test]
    fn original_boundary_derivative_keeps_source_and_budget() {
        let s=plane();
        let domain=[[0.2,0.8],[1.,1.]];
        let (bound,spent)=derivative_path_bounds(&s,domain,0,1).unwrap();
        let bound=bound.unwrap();
        assert!(std::ptr::eq(bound.surface(),&s));
        assert_eq!(bound.domain(),domain); assert_eq!(bound.axis(),0); assert_eq!(spent,1);
        for (r,value) in bound.components().into_iter().zip([1.,0.,0.]) {
            assert!(r[0]<=value&&value<=r[1]);
        }
        assert!(derivative_path_bounds(&s,domain,2,1).is_err());
        assert!(derivative_path_bounds(&s,[[-0.1,0.8],[1.,1.]],0,1).is_err());
        assert!(derivative_path_bounds(&s,domain,0,0).is_err());
        let mut jump=plane();jump.knots_u=vec![0.,0.,0.5,0.5,1.,1.];
        jump.control_points=vec![jump.control_points[0].clone(),jump.control_points[1].clone(),
            vec![vec![3.,0.,0.],vec![3.,1.,0.]],vec![vec![4.,0.,0.],vec![4.,1.,0.]]];
        jump.weights=vec![vec![1.;2];4];
        assert!(derivative_path_bounds(&jump,domain,0,8).is_err());
        assert!(derivative_path_bounds(&jump,[[0.2,0.5],[1.,1.]],0,8).is_err());

    }
    #[test]
    fn boundary_mean_integrates_complete_intervals_and_refuses_missing_work() {
        let mut s=plane();s.degree_v=2;s.knots_v=vec![0.,0.,0.,1.,1.,1.];
        s.control_points=vec![vec![vec![0.,0.,0.],vec![0.,0.,0.],vec![0.,1.,0.]],
            vec![vec![1.,0.,0.],vec![1.,0.,0.],vec![1.,1.,0.]]];s.weights=vec![vec![1.;3];2];
        let endpoints=[[0.2,0.8],[0.,0.]];
        let (bound,spans)=boundary_mean_bounds(&s,endpoints,1,1.,8,8).unwrap();
        let bound=bound.unwrap();assert_eq!(spans,8);assert!(std::ptr::eq(bound.surface(),&s));
        assert_eq!(bound.endpoints(),endpoints);assert_eq!(bound.axis(),1);assert_eq!(bound.boundary(),1.);
        let r=bound.components()[1];assert!(r[0]<=1.&&r[1]>=1.);assert!(r[1]-r[0]<0.26);
        assert!(boundary_mean_bounds(&s,endpoints,1,1.,8,7).unwrap().0.is_none());
        assert!(boundary_mean_bounds(&s,endpoints,1,0.5,8,8).is_err());
        assert!(boundary_mean_bounds(&s,endpoints,1,1.,17,100).is_err());
        s.weights=vec![vec![1.,0.7,1.];2];
        let rational=boundary_mean_bounds(&s,[[0.2,0.8],[0.,0.2]],1,1.,16,16).unwrap().0.unwrap();
        for i in 0..=20 {
            let v=i as f64*0.01;
            let value=v*v/((1.-v)*(1.-v)+1.4*v*(1.-v)+v*v);
            let mean=(1.-value)/(1.-v);
            let r=rational.components()[1];assert!(r[0]<=mean&&mean<=r[1]);
        }

    }
    fn plane() -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn factored_product_accounts_queries_and_rejects_rebound_sources() {
        let mut a=plane();a.degree_u=2;a.knots_u=vec![0.,0.,0.,1.,1.,1.];
        a.control_points=vec![vec![vec![0.,0.,0.];2],vec![vec![0.5,0.,0.];2],
            vec![vec![1.,0.,0.],vec![1.,1.,0.]]];a.weights=vec![vec![1.;2];3];
        let clone=a.clone();
        let proof=crate::factored_surface_normals::prepare(&a,false,100_000_000).unwrap();
        let c=proof.certificate.as_ref().unwrap();
        let cloned=crate::factored_surface_normals::prepare(&clone,false,100_000_000).unwrap();
        let mut b=plane();for row in &mut b.control_points {for p in row {p.swap(0,2);}}
        let mut ca=OriginalNormalCache::new(&a);let mut cb=OriginalNormalCache::new(&b);
        let domains=[[[0.,1.];2];2];
        let refused=ca.inspect_product_with_factors(&mut cb,domains,1e-6,1,1,[Some(c),None],0).unwrap();
        assert!(!refused.excluded);assert_eq!(refused.unresolved,vec![domains]);
        let paid=ca.inspect_product_with_factors(&mut cb,domains,1e-6,1,1,[Some(c),None],1).unwrap();
        assert!(paid.excluded);assert_eq!(paid.factored_queries,1);
        let cached=ca.inspect_product_with_factors(&mut cb,domains,1e-6,1,0,[Some(c),None],0).unwrap();
        assert!(cached.excluded);assert_eq!(cached.factored_queries,0);assert_eq!(cached.spans,0);
        assert!(ca.inspect_product_with_factors(&mut cb,domains,1e-6,1,0,
            [cloned.certificate.as_ref(),None],0).is_err());
        let raw=ca.inspect_product(&mut cb,domains,1e-6,1,0).unwrap();
        assert!(!raw.excluded);assert_eq!(raw.unresolved,vec![domains]);
        let mut direct_a=OriginalNormalCache::new(&a);let mut direct_b=OriginalNormalCache::new(&b);
        let (limited,used)=direct_a.inspect_other_with_factors(&mut direct_b,domains,1e-6,1,[Some(c),None],0).unwrap();
        assert_eq!(used,0);assert_eq!(limited.aligned,None);
        let (paid,used)=direct_a.inspect_other_with_factors(&mut direct_b,domains,1e-6,1,[Some(c),None],1).unwrap();
        assert_eq!(used,1);assert_eq!(paid.aligned,Some(false));
        let (cached,used)=direct_a.inspect_other_with_factors(&mut direct_b,domains,1e-6,0,[Some(c),None],0).unwrap();
        assert_eq!(used,0);assert_eq!(cached.spans,0);assert_eq!(cached.normal_components,paid.normal_components);
        assert!(direct_a.inspect_other_with_factors(&mut direct_b,domains,1e-6,0,[cloned.certificate.as_ref(),None],0).is_err());
        assert!(direct_a.inspect_other_with_factors(&mut direct_b,domains,1e-6,0,[Some(c),None],100001).is_err());

    }
    #[test]
    fn closed_product_excludes_only_complete_oblique_normal_coverage() {
        let a=plane();let mut b=plane();
        for row in &mut b.control_points {for p in row {p.swap(0,2);}}
        let domains=[[[0.,1.];2];2];
        let mut ca=OriginalNormalCache::new(&a);let mut cb=OriginalNormalCache::new(&b);
        let refused=ca.inspect_product(&mut cb,domains,1e-6,1,1).unwrap();
        assert!(!refused.excluded);assert_eq!(refused.unresolved,vec![domains]);
        let paid=ca.inspect_product(&mut cb,domains,1e-6,1,2).unwrap();
        assert!(paid.excluded);assert!(paid.unresolved.is_empty());
        let cached=ca.inspect_product(&mut cb,domains,1e-6,1,0).unwrap();
        assert!(cached.excluded);assert_eq!(cached.spans,0);
        let reverse=cb.inspect_product(&mut ca,domains,1e-6,1,0).unwrap();
        assert!(reverse.excluded);
        let mut same=OriginalNormalCache::new(&a);
        let aligned=ca.inspect_product(&mut same,domains,1e-6,100,2).unwrap();
        assert!(!aligned.excluded);assert_eq!(aligned.unresolved,vec![domains]);
        assert_eq!(aligned.reason,"product-aligned-region");
        assert!(ca.inspect_product(&mut cb,domains,1e-6,0,0).is_err());
        let mut invalid=domains;invalid[1][0]=[f64::NAN,1.];
        assert!(ca.inspect_product(&mut cb,invalid,1e-6,1,0).is_err());
    }
    #[test]
    fn distinct_source_cache_reuses_only_complete_original_rectangles() {
        let a=plane();let mut b=plane();
        for row in &mut b.control_points {for p in row {p.swap(0,2);}}
        let domains=[[[0.,1.],[0.,1.]];2];
        let mut ac=OriginalNormalCache::new(&a);let mut bc=OriginalNormalCache::new(&b);
        let paid=ac.inspect_other(&mut bc,domains,1e-6,2).unwrap();
        let plain=inspect_pair([&a,&b],domains,1e-6,2).unwrap();
        assert_eq!(paid.aligned,Some(false));assert_eq!(paid.spans,2);
        assert_eq!(paid.normal_components,plain.normal_components);
        assert_eq!(paid.sine_squared_interval,plain.sine_squared_interval);
        let free=ac.inspect_other(&mut bc,domains,1e-6,0).unwrap();
        assert_eq!(free.spans,0);assert_eq!(free.aligned,paid.aligned);
        assert_eq!(free.sine_squared_interval,paid.sine_squared_interval);
        let mut changed=domains;changed[1][0]=[-0.1,1.];
        assert!(ac.inspect_other(&mut bc,changed,1e-6,0).is_err());
        assert!(ac.inspect_other(&mut bc,domains,1.,0).is_err());
        let c=plane();let mut cc=OriginalNormalCache::new(&c);
        let missing=ac.inspect_other(&mut cc,domains,1e-6,0).unwrap();
        assert_eq!(missing.aligned,None);assert_eq!(missing.normal_components[1],None);
        let fresh=ac.inspect_other(&mut cc,domains,1e-6,1).unwrap();
        assert_eq!(fresh.aligned,Some(true));assert_eq!(fresh.spans,1);
    }
    #[test]
    fn incomplete_distinct_normal_queries_never_enter_the_cache() {
        let mut a=plane();a.knots_u=vec![0.,0.,0.5,1.,1.];
        a.control_points.push(vec![vec![2.,0.,1.],vec![2.,1.,1.]]);
        a.weights.push(vec![1.;2]);let b=plane();
        let mut ac=OriginalNormalCache::new(&a);let mut bc=OriginalNormalCache::new(&b);
        let domains=[[[0.,1.],[0.,1.]];2];
        for _ in 0..2 {
            let r=ac.inspect_other(&mut bc,domains,1e-6,1).unwrap();
            assert_eq!(r.spans,1);assert_eq!(r.aligned,None);
            assert_eq!(r.normal_components,[None,None]);
        }
        let full=ac.inspect_other(&mut bc,domains,1e-6,3).unwrap();
        assert_eq!(full.spans,3);assert!(full.normal_components.iter().all(Option::is_some));
        let cached=ac.inspect_other(&mut bc,domains,1e-6,0).unwrap();
        assert_eq!(cached.spans,0);assert_eq!(cached.normal_components,full.normal_components);
        let point=[[[0.5,0.5],[0.3,0.7]];2];
        let unpaid=ac.inspect_other(&mut bc,point,1e-6,0).unwrap();
        assert_eq!(unpaid.normal_components,[None,None]);
    }
    #[test]
    fn exact_factored_limits_resolve_pole_angles_without_inventing_regular_normals() {
        // Independent S=(3u,3u²v,u³) has factored normal (-u²,0,1).
        let s = Surface {
            degree_u: 3,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0usize..4)
                .map(|i| {
                    (0..2)
                        .map(|v| {
                            vec![
                                i as f64,
                                (i * i.saturating_sub(1)) as f64 / 2. * v as f64,
                                if i == 3 { 1. } else { 0. },
                            ]
                        })
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 2]; 4],
            periodic_u: false,
            periodic_v: false,
        };
        let pair = [[[0., 0.], [0., 1.]], [[1., 1.], [0., 1.]]];
        assert!(inspect_pair([&s, &s], pair, 0.1, 2)
            .unwrap()
            .aligned
            .is_none());
        let c = crate::factored_surface_normals::prepare(&s, false, 10_000_000)
            .unwrap()
            .certificate
            .unwrap();
        let r = inspect_pair_bounds(
            [c.bounds(pair[0]).unwrap(), c.bounds(pair[1]).unwrap()],
            0.1,
        )
        .unwrap();
        assert_eq!(r.aligned, Some(false));
        let bounds = r.sine_squared_interval.unwrap();
        assert!(bounds[0] <= 0.5 && bounds[1] >= 0.5);
        assert_eq!(r.spans, 0); // Pure comparison: the caller pays source queries.
        let z = [[0., 0.], [0., 0.], [1., 1.]];
        let minus_z = [[0., 0.], [0., 0.], [-1., -1.]];
        assert_eq!(
            inspect_pair_bounds([z, minus_z], 0.1).unwrap().aligned,
            Some(true)
        );
        assert!(inspect_pair_bounds([z, [[0., 0.]; 3]], 0.1)
            .unwrap()
            .aligned
            .is_none());
    }
    #[test]
    #[ignore = "native restriction microbenchmark; no production speed claim"]
    fn benchmark_closed_rational_restriction() {
        let (p, q) = (16, 8);
        let net = (0..=p)
            .map(|u| {
                (0..=q)
                    .map(|v| {
                        let w = 1. + u as f64 / p as f64 + v as f64 / q as f64;
                        [I::point(w), I::point(1.), I::point(0.), I::point(w)]
                    })
                    .collect()
            })
            .collect::<Net>();
        let domain = [[0.125, 0.875], [0.25, 0.75]];
        let repeats = 50;
        let start = std::time::Instant::now();
        for _ in 0..repeats {
            let result = restrict_net(&net, [p, q], [[0., 1.], [0., 1.]], domain).unwrap();
            for h in result.iter().flatten() {
                assert!(h[0].div(h[3]).unwrap().contains(1.));
            }
            std::hint::black_box(result);
        }
        let blossom = start.elapsed().as_secs_f64();
        let start = std::time::Instant::now();
        for _ in 0..repeats {
            let result = restrict_unit_net(&net, domain).unwrap();
            for h in result.iter().flatten() {
                assert!(h[0].div(h[3]).unwrap().contains(1.));
            }
            std::hint::black_box(result);
        }
        let splits = start.elapsed().as_secs_f64();
        let hull = rational_quotient_hull(&net, domain).unwrap();
        for (u, v) in [(0.125, 0.25), (0.5, 0.5), (0.875, 0.75)] {
            assert!(hull[0].contains(1.) && hull[1].contains(1. / (1. + u + v)));
        }
        eprintln!("restriction-benchmark degree={p},{q} repeats={repeats} blossom_ms={} split_ms={} ratio={}",
            blossom*1000.,splits*1000.,blossom/splits);
    }
    #[test]
    fn general_span_splits_enclose_independent_polynomials_and_closed_points() {
        let net:Net=(0..3).map(|u|(0..2).map(|v|
            [I::point(if u==2 {1.} else {0.}+2.*v as f64),
             I::point(u as f64/2.*v as f64),I::point(1.),I::point(1.)]).collect()).collect();
        for span in [[[0.,1.],[0.,1.]],[[1e6,1e6+8.],[-10.,-6.]]] {
            let point=|t:f64,v:f64|[span[0][0]+t*(span[0][1]-span[0][0]),
                span[1][0]+v*(span[1][1]-span[1][0])];
            let a=point(0.125,0.125);let b=point(0.875,0.875);
            let domain=[[a[0],b[0]],[a[1],b[1]]];
            let hull=restrict_hull(&net,[2,1],span,domain).unwrap();
            for t in [0.125,0.375,0.875] {for v in [0.125,0.625,0.875] {
                assert!(hull[0].contains(t*t+2.*v) && hull[1].contains(t*v));
            }}
            let p=point(0.375,0.625);
            let hull=restrict_hull(&net,[2,1],span,[[p[0];2],[p[1];2]]).unwrap();
            assert!(hull[0].contains(0.375*0.375+2.*0.625));
            assert!(hull[1].contains(0.375*0.625));
            let hull=restrict_hull(&net,[2,1],span,span).unwrap();
            assert!(hull[0].contains(0.) && hull[0].contains(3.));
            assert!(restrict_hull(&net,[2,1],span,
                [[span[0][0]-1.,span[0][1]],span[1]]).is_err());
        }
    }
    #[test]
    fn rational_coefficient_quotients_keep_shared_denominator_and_reject_zero() {
        let net = vec![
            vec![[I::point(2.), I::point(1.), I::point(0.), I::point(2.)]; 2],
            vec![[I::point(10.), I::point(1.), I::point(0.), I::point(10.)]; 2],
        ];
        let full = rational_quotient_hull(&net, [[0., 1.], [0., 1.]]).unwrap();
        assert!(full[0].contains(1.) && full[0].hi - full[0].lo < 1e-12);
        for u in [0., 0.25, 0.5, 1.] {
            assert!(full[1].contains(1. / (2. + 8. * u)));
            let point = rational_quotient_hull(&net, [[u, u], [0.3, 0.3]]).unwrap();
            assert!(point[1].contains(1. / (2. + 8. * u)));
            assert!(point[1].hi - point[1].lo < 1e-12);
        }
        let mut negative = net.clone();
        for h in negative.iter_mut().flatten() {
            for x in h {
                *x = I::point(0.).sub(*x).unwrap();
            }
        }
        let signed = signed_rational_quotient_hull(&negative, [[0., 1.], [0., 1.]])
            .unwrap()
            .unwrap();
        assert!(signed[0].contains(1.) && signed[1].contains(0.25));
        negative[0][0][3] = I::point(1.);
        assert!(
            signed_rational_quotient_hull(&negative, [[0., 1.], [0., 1.]])
                .unwrap()
                .is_none()
        );
        let mut invalid = net;
        invalid[0][0][3] = I::point(0.);
        assert!(rational_quotient_hull(&invalid, [[0., 1.], [0., 1.]]).is_err());
    }
    #[test]
    fn pair_normals_distinguish_tangency_fold_and_singular_transition() {
        let a = plane();
        let mut b = a.clone();
        for row in &mut b.control_points {
            for p in row {
                p[0] += 100.;
                p[1] -= 20.;
            }
        }
        let domains = [[[0., 1.], [0., 1.]]; 2];
        let r = inspect_pair([&a, &b], domains, 1e-6, 2).unwrap();
        assert_eq!(r.aligned, Some(true));
        assert_eq!(r.spans, 2);
        // Position is deliberately different: angular agreement is not G0/G1.
        b.control_points.reverse();
        assert_eq!(
            inspect_pair([&a, &b], domains, 1e-6, 2).unwrap().aligned,
            Some(true)
        );
        for row in &mut b.control_points {
            for p in row {
                p[2] = p[0] - 100.;
            }
        }
        let r = inspect_pair([&a, &b], domains, 1e-6, 2).unwrap();
        assert_eq!(r.aligned, Some(false));
        let d = r.sine_squared_interval.unwrap();
        assert!(d[0] <= 0.5 && d[1] >= 0.5);
        for row in &mut b.control_points {
            for p in row {
                p[1] = 0.;
                p[2] = 0.;
            }
        }
        let r = inspect_pair([&a, &b], domains, 1e-6, 2).unwrap();
        assert_eq!(r.aligned, None);
        assert_eq!(r.reason, "normal-unresolved");
        let r = inspect_pair([&a, &a], domains, 1e-6, 1).unwrap();
        assert_eq!(r.aligned, None);
        assert_eq!(r.reason, "span-limit");
        assert_eq!(r.spans, 1);
        let invalid = [domains[0], [[0., 2.], [0., 1.]]];
        assert!(inspect_pair([&a, &a], invalid, 1e-6, 1).is_err());
    }
    #[test]
    fn parallel_antiparallel_oblique_and_collapsed_normals_are_distinct() {
        let s = plane();
        let before = format!("{s:?}");
        for d in [[0., 0., 1.], [0., 0., -1.], [0., 0., f64::MAX]] {
            let r = inspect(&s, [[0.2, 0.8]; 2], d, 1e-10, 1).unwrap();
            assert_eq!(r.aligned, Some(true));
        }
        let r = inspect(&s, [[0.2, 0.8]; 2], [1., 0., 1.], 1e-10, 1).unwrap();
        assert_eq!(r.aligned, Some(false));
        let b = r.sine_squared_interval.unwrap();
        assert!(b[0] <= 0.5 && b[1] >= 0.5);
        let mut collapsed = s.clone();
        collapsed.control_points[1] = collapsed.control_points[0].clone();
        assert_eq!(
            inspect(&collapsed, [[0.2, 0.8]; 2], [0., 0., 1.], 1e-10, 1)
                .unwrap()
                .aligned,
            None
        );
        assert_eq!(format!("{s:?}"), before);
    }
    #[test]
    fn knot_sides_and_span_exhaustion_cannot_be_hidden() {
        let mut s = plane();
        s.knots_u = vec![0., 0., 0.5, 1., 1.];
        s.control_points
            .push(vec![vec![2., 0., 1.], vec![2., 1., 1.]]);
        s.weights.push(vec![1.; 2]);
        let r = inspect(&s, [[0.5, 0.5], [0.3, 0.7]], [0., 0., 1.], 1e-6, 1).unwrap();
        assert_eq!(r.aligned, None);
        assert_eq!(r.reason, "span-limit");
        assert!(r.normal_components.is_none());
        let r = inspect(&s, [[0.5, 0.5], [0.3, 0.7]], [0., 0., 1.], 1e-6, 2).unwrap();
        assert_ne!(r.aligned, Some(true));
        assert_eq!(r.spans, 2);
        for (d, t, n) in [
            ([0.; 3], 1e-6, 1),
            ([0., 0., 1.], 1., 1),
            ([0., 0., 1.], 1e-6, 0),
        ] {
            assert!(inspect(&s, [[0., 1.]; 2], d, t, n).is_err());
        }
    }
    #[test]
    fn rational_original_derivatives_stay_enclosed_on_narrow_and_point_rectangles() {
        let s = Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0.13, 0.13, 0.13, 3.17, 3.17, 3.17],
            knots_v: vec![-0.71, -0.71, -0.71, 0.29, 0.29, 0.29],
            control_points: (0..3)
                .map(|i| {
                    (0..3)
                        .map(|j| vec![i as f64, j as f64, (i * i + 2 * j * j + i * j) as f64 / 10.])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1., 0.7, 1.2], vec![0.9, 1.1, 0.8], vec![1.3, 0.6, 1.]],
            periodic_u: false,
            periodic_v: false,
        };
        let before = format!("{s:?}");
        for domain in [
            [[0.43, 0.53], [-0.23, -0.13]],
            [[0.43, 0.43_f64.next_up()], [-0.23, (-0.23_f64).next_up()]],
            [[0.43, 0.43], [-0.23, -0.23]],
        ] {
            let r = inspect(&s, domain, [0.3, -0.2, 1.], 1e-6, 1).unwrap();
            let bounds = r.normal_components.unwrap();
            let sine = r.sine_squared_interval.unwrap();
            for i in 0..=4 {
                for j in 0..=4 {
                    let u = domain[0][0] + (domain[0][1] - domain[0][0]) * i as f64 / 4.;
                    let v = domain[1][0] + (domain[1][1] - domain[1][0]) * j as f64 / 4.;
                    let (a, b) = s.evaluate(u, v).unwrap().first_derivatives().unwrap();
                    let n = std::array::from_fn::<_, 3, _>(|k| {
                        a[(k + 1) % 3] * b[(k + 2) % 3] - a[(k + 2) % 3] * b[(k + 1) % 3]
                    });
                    for k in 0..3 {
                        assert!(n[k] >= bounds[k][0] - 1e-11 && n[k] <= bounds[k][1] + 1e-11);
                    }
                    let d = [0.3, -0.2, 1.];
                    let cross = std::array::from_fn::<_, 3, _>(|k| {
                        n[(k + 1) % 3] * d[(k + 2) % 3] - n[(k + 2) % 3] * d[(k + 1) % 3]
                    });
                    let ratio = cross.iter().map(|x| x * x).sum::<f64>()
                        / (n.iter().map(|x| x * x).sum::<f64>()
                            * d.iter().map(|x| x * x).sum::<f64>());
                    assert!(ratio >= sine[0] - 1e-11 && ratio <= sine[1] + 1e-11);
                }
            }
        }
        let direction = s.evaluate(0.43, -0.23).unwrap().unit_normal().unwrap();
        let r = inspect(&s, [[0.43, 0.43], [-0.23, -0.23]], direction, 1e-10, 1).unwrap();
        assert_eq!(r.aligned, Some(true));
        assert_eq!(format!("{s:?}"), before);
    }
    #[test]
    fn general_spatial_plane_does_not_depend_on_a_cartesian_axis() {
        let mut s = plane();
        for row in &mut s.control_points {
            for p in row {
                let (u, v) = (p[0], p[1]);
                *p = vec![1024. + u, -2048. + 2. * u + v, 512. + 3. * v];
            }
        }
        let r = inspect(
            &s,
            [[0.31, 0.31_f64.next_up()], [0.67, 0.67_f64.next_up()]],
            [6., -3., 1.],
            1e-10,
            1,
        )
        .unwrap();
        assert_eq!(r.aligned, Some(true));
        let b = r.normal_components.unwrap();
        for (i, n) in [6., -3., 1.].into_iter().enumerate() {
            assert!(b[i][0] <= n && b[i][1] >= n);
        }
    }
}
