//! Local x/y rational jets through total order three, on original knot cells.
use crate::{Result, distance_bounds::Interval as I, surface::Surface};
type Net = Vec<Vec<[I; 4]>>;
pub(crate) type Jets = [[[I; 3]; 4]; 4];
// Restriction encloses the same original homogeneous Bernstein polynomial.
// Jets remain derivatives in the restricted unit coordinates; the stable
// route differentiates a full original span before evaluating its query.
pub(crate) fn original_span_net(s: &Surface, span: [usize; 2], domain: [[f64; 2]; 2]) -> Result<Net> {
    match crate::curve_surface_composition::surface_net_on_bernstein(s, span, domain)? {
        Some(net) => Ok(net),
        None => crate::curve_surface_composition::surface_net_on(s, span, domain),
    }
}

fn difference(net: Net, axis: usize) -> Result<Net> {
    let degree = if axis == 0 {
        net.len() - 1
    } else {
        net[0].len() - 1
    };
    if degree == 0 {
        return Ok(vec![
            vec![
                [I::point(0.); 4];
                if axis == 0 { net[0].len() } else { 1 }
            ];
            if axis == 0 { 1 } else { net.len() }
        ]);
    }
    let nu = net.len() - usize::from(axis == 0);
    let nv = net[0].len() - usize::from(axis == 1);
    let mut out = vec![vec![[I::point(0.); 4]; nv]; nu];
    for u in 0..nu {
        for v in 0..nv {
            for k in 0..4 {
                out[u][v][k] = net[u + usize::from(axis == 0)][v + usize::from(axis == 1)][k]
                    .sub(net[u][v][k])?
                    .mul(I::point(degree as f64))?;
            }
        }
    }
    Ok(out)
}
fn choose(n: usize, k: usize) -> f64 {
    if k == 0 || k == n { 1. } else { n as f64 }
}
pub(crate) fn calculate(
    s: &Surface,
    span: [usize; 2],
    domain: [[f64; 2]; 2],
    corner: Option<[usize; 2]>,
) -> Result<Jets> {
    calculate_partial(s, span, domain, corner.map_or([None; 2], |c| c.map(Some)))
}
/// Restrict selected coordinates to an endpoint; enclose the others fully.
pub(crate) fn calculate_partial(
    s: &Surface,
    span: [usize; 2],
    domain: [[f64; 2]; 2],
    corner: [Option<usize>; 2],
) -> Result<Jets> {
    calculate_partial_up_to(s, span, domain, corner, 3)
}
fn calculate_partial_up_to(
    s: &Surface, span: [usize; 2], domain: [[f64; 2]; 2],
    corner: [Option<usize>; 2], max_order: usize,
) -> Result<Jets> {
    let mut net = original_span_net(s, span, domain)?;
    let origin = &s.control_points[span[0] - s.degree_u][span[1] - s.degree_v];
    for row in &mut net {
        for h in row {
            for k in 0..3 {
                h[k] = h[k].sub(I::point(origin[k]).mul(h[3])?)?;
            }
        }
    }
    let mut h = [[[I::point(0.); 4]; 4]; 4];
    for order in 0..=max_order {
        for a in 0..=order {
            let b = order - a;
            let mut d = net.clone();
            for _ in 0..a {
                d = difference(d, 0)?;
            }
            for _ in 0..b {
                d = difference(d, 1)?;
            }
            for k in 0..4 {
                let rows = d.len();
                let points: Vec<_> = d
                    .iter()
                    .enumerate()
                    .flat_map(|(u, row)| {
                        row.iter().enumerate().filter_map(move |(v, p)| {
                            (corner[0].is_none_or(|c| u == c * (rows - 1))
                                && corner[1].is_none_or(|c| v == c * (row.len() - 1)))
                            .then_some(p[k])
                        })
                    })
                    .collect();
                h[a][b][k] = I::new(
                    points.iter().map(|p| p.lo).fold(f64::INFINITY, f64::min),
                    points
                        .iter()
                        .map(|p| p.hi)
                        .fold(f64::NEG_INFINITY, f64::max),
                )?;
            }
        }
    }
    quotient(h, max_order)
}
fn quotient(h: [[[I; 4]; 4]; 4], max_order: usize) -> Result<Jets> {
    let mut jets = [[[I::point(0.); 3]; 4]; 4];
    for order in 0..=max_order {
        for a in 0..=order {
            let b = order - a;
            for k in 0..3 {
                let mut numerator = h[a][b][k];
                for i in 0..=a {
                    for j in 0..=b {
                        if i + j == 0 {
                            continue;
                        }
                        numerator = numerator.sub(
                            h[i][j][3]
                                .mul(jets[a - i][b - j][k])?
                                .mul(I::point(choose(a, i) * choose(b, j)))?,
                        )?;
                    }
                }
                jets[a][b][k] = numerator.div(h[0][0][3])?;
            }
        }
    }
    Ok(jets)
}

fn evaluate_bernstein(mut points: Vec<[I; 4]>, t: I) -> Result<[I; 4]> {
    crate::check(!points.is_empty() && t.lo >= 0. && t.hi <= 1.,
        "Bernstein jet evaluation needs nonempty coefficients and a unit fraction")?;
    let beta = I::point(1.).sub(t)?.intersect(0., 1.)?;
    for count in (1..points.len()).rev() {
        for i in 0..count {
            for k in 0..4 {
                // t is an original-span fraction in [0,1]. Each de Casteljau
                // step is a convex combination even though interval t and
                // 1-t are evaluated independently. Preserve that correlation
                // before the next tensor axis: a constant row must not widen.
                let a = points[i][k];
                let b = points[i + 1][k];
                points[i][k] = a.mul(beta)?.add(b.mul(t)?)?
                    .intersect(a.lo.min(b.lo), a.hi.max(b.hi))?;
            }
        }
    }
    Ok(points[0])
}
/// Derivatives in ORIGINAL span coordinates, evaluated over a restricted
/// rectangle. Differentiate before restriction so tiny query intervals do not
/// subtract nearly equal restricted coefficients to reconstruct a derivative.
pub(crate) fn calculate_partial_stable(
    s: &Surface,
    span: [usize; 2],
    domain: [[f64; 2]; 2],
    corner: [Option<usize>; 2],
) -> Result<Jets> {
    calculate_partial_stable_up_to(s, span, domain, corner, 3)
}
pub(crate) fn calculate_partial_stable_up_to(
    s: &Surface, span: [usize; 2], domain: [[f64; 2]; 2],
    corner: [Option<usize>; 2], max_order: usize,
) -> Result<Jets> {
    calculate_partial_stable_projected(s,span,domain,corner,max_order,None)
}
/// Preserve correlated height coefficients before differentiation. The binary
/// plane coefficients are proposals only; all projection arithmetic is enclosed.
pub(crate) fn calculate_partial_stable_graph(s:&Surface,span:[usize;2],domain:[[f64;2];2],corner:[Option<usize>;2],frame:([usize;3],[f64;2]))->Result<Jets> {
    calculate_partial_stable_projected(s,span,domain,corner,2,Some(frame))
}
fn calculate_partial_stable_projected(s:&Surface,span:[usize;2],domain:[[f64;2];2],corner:[Option<usize>;2],max_order:usize,frame:Option<([usize;3],[f64;2])>)->Result<Jets> {
    let full = [
        [s.knots_u[span[0]], s.knots_u[span[0] + 1]],
        [s.knots_v[span[1]], s.knots_v[span[1] + 1]],
    ];
    let mut parameter = [I::point(0.); 2];
    for k in 0..2 {
        let query = if let Some(c) = corner[k] {
            I::point(domain[k][c])
        } else {
            I::new(domain[k][0], domain[k][1])?
        };
        parameter[k] = query
            .sub(I::point(full[k][0]))?
            .div(I::point(full[k][1]).sub(I::point(full[k][0]))?)?
            .intersect(0., 1.)?;
    }
    let mut net = original_span_net(s, span, full)?;
    let origin = &s.control_points[span[0] - s.degree_u][span[1] - s.degree_v];
    for row in &mut net {
        for h in row {
            for k in 0..3 {
                h[k] = h[k].sub(I::point(origin[k]).mul(h[3])?)?;
            }
        }
    }
    if let Some((axes,plane))=frame {
        for row in &mut net {for h in row {
            let p=*h;
            h[0]=p[axes[0]];h[1]=p[axes[1]];
            h[2]=p[axes[2]].sub(p[axes[0]].mul(I::point(plane[0]))?)?.sub(p[axes[1]].mul(I::point(plane[1]))?)?;
        }}
    }
    let mut h = [[[I::point(0.); 4]; 4]; 4];
    for order in 0..=max_order {
        for a in 0..=order {
            let b = order - a;
            let mut d = net.clone();
            for _ in 0..a {
                d = difference(d, 0)?;
            }
            for _ in 0..b {
                d = difference(d, 1)?;
            }
            let mut rows = Vec::new();
            for row in &d {
                rows.push(evaluate_bernstein(row.clone(), parameter[1])?);
            }
            let value = evaluate_bernstein(rows, parameter[0])?;
            for k in 0..4 {
                // Every derivative polynomial retains the Bernstein convex
                // hull property. In particular, positive weight lower bounds
                // survive wide interval parameters without dependency collapse.
                let lo = d
                    .iter()
                    .flatten()
                    .map(|p| p[k].lo)
                    .fold(f64::INFINITY, f64::min);
                let hi = d
                    .iter()
                    .flatten()
                    .map(|p| p[k].hi)
                    .fold(f64::NEG_INFINITY, f64::max);
                h[a][b][k] = value[k].intersect(lo, hi)?;
            }
        }
    }
    quotient(h, max_order)
}


/// Value, du, dv only. Injectivity needs the first Jacobian, not curvature
/// or third derivatives. Each retained entry follows the original recurrence.
pub(crate) type FirstJets = [[I; 3]; 3];
fn first(j: Jets) -> FirstJets { [j[0][0], j[1][0], j[0][1]] }
pub(crate) fn calculate_first(s: &Surface, span: [usize; 2], domain: [[f64; 2]; 2]) -> Result<FirstJets> {
    calculate_partial_up_to(s, span, domain, [None; 2], 1).map(first)
}
pub(crate) fn calculate_first_stable(s: &Surface, span: [usize; 2], domain: [[f64; 2]; 2]) -> Result<FirstJets> {
    calculate_partial_stable_up_to(s, span, domain, [None; 2], 1).map(first)
}


/// One immutable original span at a time. DFS subdivision reuses its exact
/// homogeneous derivative nets; replacing the span bounds retained memory.
/// Certificate cell charges and query interval arithmetic remain unchanged.
pub(crate) struct FirstStableSpanCache<'a> {
    source: &'a Surface,
    prepared: Option<PreparedFirstStableSpan>,
}
struct PreparedFirstStableSpan {
    span: [usize; 2],
    full: [[f64; 2]; 2],
    nets: [Net; 3], // value, dv, du: original total-order iteration order
    hulls: [[I; 4]; 3],
}
impl<'a> FirstStableSpanCache<'a> {
    pub(crate) fn new(source: &'a Surface) -> Self { Self { source, prepared: None } }
    pub(crate) fn calculate(&mut self, span: [usize; 2], domain: [[f64; 2]; 2]) -> Result<FirstJets> {
        if self.prepared.as_ref().is_none_or(|p| p.span != span) {
            // Evict before preparing, so retained storage never grows with spans.
            self.prepared = None;
            self.prepared = Some(PreparedFirstStableSpan::new(self.source, span)?);
        }
        self.prepared.as_ref().unwrap().calculate(domain)
    }
}
impl PreparedFirstStableSpan {
    fn new(s: &Surface, span: [usize; 2]) -> Result<Self> {
        let full = [[s.knots_u[span[0]], s.knots_u[span[0]+1]],
            [s.knots_v[span[1]], s.knots_v[span[1]+1]]];
        let mut net = original_span_net(s, span, full)?;
        let origin = &s.control_points[span[0]-s.degree_u][span[1]-s.degree_v];
        for row in &mut net { for h in row { for k in 0..3 {
            h[k] = h[k].sub(I::point(origin[k]).mul(h[3])?)?;
        }}}
        let dv = difference(net.clone(), 1)?;
        let du = difference(net.clone(), 0)?;
        let nets = [net, dv, du];
        let mut hulls = [[I::point(0.);4];3];
        for (i,d) in nets.iter().enumerate() { for k in 0..4 {
            let lo=d.iter().flatten().map(|p|p[k].lo).fold(f64::INFINITY,f64::min);
            let hi=d.iter().flatten().map(|p|p[k].hi).fold(f64::NEG_INFINITY,f64::max);
            hulls[i][k]=I::new(lo,hi)?;
        }}
        Ok(Self {span, full, nets, hulls})
    }
    fn calculate(&self, domain: [[f64; 2]; 2]) -> Result<FirstJets> {
        let mut parameter=[I::point(0.);2];
        for k in 0..2 {
            parameter[k]=I::new(domain[k][0],domain[k][1])?
                .sub(I::point(self.full[k][0]))?
                .div(I::point(self.full[k][1]).sub(I::point(self.full[k][0]))?)?
                .intersect(0.,1.)?;
        }
        let mut h=[[[I::point(0.);4];4];4];
        for (index,(a,b)) in [(0,0),(0,1),(1,0)].into_iter().enumerate() {
            let mut rows=Vec::new();
            for row in &self.nets[index] {
                rows.push(evaluate_bernstein(row.clone(),parameter[1])?);
            }
            let value=evaluate_bernstein(rows,parameter[0])?;
            for k in 0..4 { h[a][b][k]=value[k].intersect(self.hulls[index][k].lo,self.hulls[index][k].hi)?; }
        }
        quotient(h,1).map(first)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn contains(i: I, value: f64) {
        assert!(i.lo <= value && value <= i.hi, "{value} outside {i:?}");
    }
    #[test]
    fn stable_jets_on_ulp_wide_queries_match_original_rational_derivatives() {
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 4., 0.]],
                vec![vec![3., 0., 0.], vec![3., 4., 0.]],
            ],
            weights: vec![vec![1., 3.], vec![2., 6.]],
            periodic_u: false,
            periodic_v: false,
        };
        let u = 0.5_f64;
        let v = 0.25_f64;
        let d = [[u.next_down(), u.next_up()], [v.next_down(), v.next_up()]];
        let j = calculate_partial_stable(&s, [1, 1], d, [None; 2]).unwrap();
        contains(j[1][0][0], 6. / (1. + u).powi(2));
        contains(j[2][0][0], -12. / (1. + u).powi(3));
        contains(j[3][0][0], 36. / (1. + u).powi(4));
        contains(j[0][1][1], 12. / (1. + 2. * v).powi(2));
        contains(j[0][2][1], -48. / (1. + 2. * v).powi(3));
        contains(j[0][3][1], 288. / (1. + 2. * v).powi(4));
        assert!(j[1][0][0].hi - j[1][0][0].lo < 1e-8);
        assert!(j[0][1][1].hi - j[0][1][1].lo < 1e-8);
        contains(j[1][1][0], 0.);
        contains(j[1][1][1], 0.);
    }
    #[test]
    fn mixed_polynomial_jets_and_corners_match_independent_equations() {
        let s = crate::polynomial::graph(
            [0., 1., 0., 1.],
            &[vec![0., 0., 0.], vec![0., 0., 1.], vec![0., 1., 0.]],
        )
        .unwrap();
        let domain = [[0.25, 0.75], [0.125, 0.875]];
        let all = calculate(&s, [2, 2], domain, None).unwrap();
        for (corner, u, v) in [
            (Some([0, 0]), 0.25, 0.125),
            (Some([0, 1]), 0.25, 0.875),
            (Some([1, 0]), 0.75, 0.125),
            (Some([1, 1]), 0.75, 0.875),
        ] {
            let c = calculate(&s, [2, 2], domain, corner).unwrap();
            for jets in [&all, &c] {
                contains(jets[0][0][2], u * u * v + u * v * v);
                contains(jets[1][0][2], (2. * u * v + v * v) * 0.5);
                contains(jets[0][1][2], (u * u + 2. * u * v) * 0.75);
                contains(jets[2][0][2], 2. * v * 0.5_f64.powi(2));
                contains(jets[1][1][2], (2. * u + 2. * v) * 0.5 * 0.75);
                contains(jets[2][1][2], 2. * 0.5_f64.powi(2) * 0.75);
                contains(jets[1][2][2], 2. * 0.5 * 0.75_f64.powi(2));
                contains(jets[3][0][2], 0.);
                contains(jets[0][3][2], 0.);
            }
        }
    }
    #[test]
    fn rational_third_jets_match_separable_weighted_chart() {
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 4., 0.]],
                vec![vec![3., 0., 0.], vec![3., 4., 0.]],
            ],
            weights: vec![vec![1., 3.], vec![2., 6.]],
            periodic_u: false,
            periodic_v: false,
        };
        let domain = [[0.25, 0.75], [0.125, 0.875]];
        let all = calculate(&s, [1, 1], domain, None).unwrap();
        for (corner, u, v) in [(Some([0, 0]), 0.25, 0.125), (Some([1, 1]), 0.75, 0.875)] {
            let c = calculate(&s, [1, 1], domain, corner).unwrap();
            for j in [&all, &c] {
                contains(j[1][0][0], 6. * 0.5 / (1_f64 + u).powi(2));
                contains(j[2][0][0], -12. * 0.5_f64.powi(2) / (1_f64 + u).powi(3));
                contains(j[3][0][0], 36. * 0.5_f64.powi(3) / (1_f64 + u).powi(4));
                contains(j[0][1][1], 12. * 0.75 / (1_f64 + 2. * v).powi(2));
                contains(
                    j[0][2][1],
                    -48. * 0.75_f64.powi(2) / (1_f64 + 2. * v).powi(3),
                );
                contains(
                    j[0][3][1],
                    288. * 0.75_f64.powi(3) / (1_f64 + 2. * v).powi(4),
                );
                contains(j[1][1][0], 0.);
                contains(j[1][2][1], 0.);
            }
        }
    }
}

#[cfg(test)]
mod first_jacobian_tests {
    use super::*;
    #[test]
    fn first_jacobian_matches_full_third_order_bits_on_rational_nets() {
        let mut checked = 0;
        for pu in 0..=6 { for pv in 0..=6 {
            let s = Surface {
                degree_u: pu, degree_v: pv,
                knots_u: [vec![0.; pu + 1], vec![1.; pu + 1]].concat(),
                knots_v: [vec![0.; pv + 1], vec![1.; pv + 1]].concat(),
                control_points: (0..=pu).map(|u| (0..=pv).map(|v|
                    vec![1e6 + u as f64 * 0.125, -1e6 + v as f64 * 0.25,
                        (u * u + 3 * v) as f64 * 0.0625]).collect()).collect(),
                weights: (0..=pu).map(|u| (0..=pv).map(|v|
                    1. + ((u + 2 * v) % 3) as f64 * 0.125).collect()).collect(),
                periodic_u: false, periodic_v: false,
            };
            let mut prepared=FirstStableSpanCache::new(&s);
            for d in [[[0.,1.],[0.,1.]], [[0.125,0.875],[0.25,0.75]],
                [[0.5,0.5f64.next_up()],[0.5,0.5f64.next_up()]]] {
                for (full, limited) in [
                    (calculate(&s, [pu,pv], d, None).unwrap(), calculate_first(&s, [pu,pv], d).unwrap()),
                    (calculate_partial_stable(&s, [pu,pv], d, [None;2]).unwrap(), calculate_first_stable(&s, [pu,pv], d).unwrap()),
                    (calculate_partial_stable(&s, [pu,pv], d, [None;2]).unwrap(), prepared.calculate([pu,pv],d).unwrap()),
                ] {
                    for (a,b) in first(full).iter().flatten().zip(limited.iter().flatten()) {
                        assert_eq!(a.lo.to_bits(),b.lo.to_bits(),"degree={pu},{pv} domain={d:?}");
                        assert_eq!(a.hi.to_bits(),b.hi.to_bits(),"degree={pu},{pv} domain={d:?}");
                    }
                    checked += 1;
                }
            }
        }}
        assert_eq!(checked,441);
    }
}

#[cfg(test)]
mod stable_span_cache_tests {
 use super::*;
 #[test]
 fn original_span_cache_replacement_matches_uncached_jets_and_source_identity() {
  let s=Surface {degree_u:2,degree_v:2,
   knots_u:vec![0.,0.,0.,0.375,1.,1.,1.],knots_v:vec![2.,2.,2.,3.,5.,5.,5.],
   control_points:(0..4).map(|u|(0..4).map(|v|vec![1e6+u as f64*0.125,-1e6+v as f64*0.25,(u*u+3*v) as f64*0.0625]).collect()).collect(),
   weights:(0..4).map(|u|(0..4).map(|v|1.+((u+2*v)%3) as f64*0.125).collect()).collect(),periodic_u:false,periodic_v:false};
  let mut other=s.clone();for row in &mut other.control_points {for p in row {p[0]+=16.;p[2]*=2.;}}
  let mut first_cache=FirstStableSpanCache::new(&s);let mut other_cache=FirstStableSpanCache::new(&other);
  let mut checked=0;
  for span in [[2,2],[3,2],[3,3],[2,3],[2,2]] {
   let full=[[s.knots_u[span[0]],s.knots_u[span[0]+1]],[s.knots_v[span[1]],s.knots_v[span[1]+1]]];
   let middle=full.map(|[a,b]|a*0.5+b*0.5);
   for d in [full,full.map(|[a,b]|[a*0.75+b*0.25,a*0.25+b*0.75]),middle.map(|m|[m,m.next_up()])] {
    for (source,cache) in [(&s,&mut first_cache),(&other,&mut other_cache)] {
     let expected=calculate_first_stable(source,span,d).unwrap();let actual=cache.calculate(span,d).unwrap();
     for (a,b) in expected.iter().flatten().zip(actual.iter().flatten()) {
      assert_eq!(a.lo.to_bits(),b.lo.to_bits(),"span={span:?} domain={d:?}");
      assert_eq!(a.hi.to_bits(),b.hi.to_bits(),"span={span:?} domain={d:?}");
     }
     checked+=1;
    }
   }
  }
  assert_eq!(checked,30);
 }
}

#[cfg(test)]
mod bernstein_consumer_tests {
    use super::*;
    fn monomial(p: usize, q: usize) -> Surface {
        let mut poles=vec![vec![vec![0.;3];q+1];p+1];
        poles[p][q]=vec![1.,2.,-1.];
        Surface {degree_u:p,degree_v:q,
            knots_u:[vec![2.;p+1],vec![6.;p+1]].concat(),
            knots_v:[vec![-2.;q+1],vec![2.;q+1]].concat(),
            control_points:poles,weights:vec![vec![1.;q+1];p+1],
            periodic_u:false,periodic_v:false}
    }
    fn derivative(n: usize, order: usize, t: f64, width: f64) -> f64 {
        if n<order { return 0.; }
        let coefficient=(0..order).fold(1.,|x,k|x*(n-k) as f64);
        coefficient*t.powi((n-order) as i32)*width.powi(order as i32)
    }
    #[test]
    fn complete_bernstein_jets_enclose_exact_original_tensor_derivatives() {
        let ranges=[[0.,1.],[0.25,0.75],[0.,0.5],[0.5,1.],[0.25,0.25],[1.,1.]];
        let corners=[[None,None],[Some(0),None],[Some(1),None],[None,Some(0)],[Some(1),Some(1)]];
        let mut checked=0;
        for (p,q) in [(1,1),(2,3),(2,13),(13,2),(13,13)] {
            let s=monomial(p,q);s.validate().unwrap();
            for u in ranges { for v in ranges {
                let domain=[u.map(|x|2.+4.*x),v.map(|x|-2.+4.*x)];
                for corner in corners {
                    let restricted=calculate_partial(&s,[p,q],domain,corner).unwrap();
                    let stable=calculate_partial_stable(&s,[p,q],domain,corner).unwrap();
                    let us:Vec<_>=corner[0].map_or_else(||u.to_vec(),|i|vec![u[i]]);
                    let vs:Vec<_>=corner[1].map_or_else(||v.to_vec(),|i|vec![v[i]]);
                    for x in us { for &y in &vs {
                        for order in 0..=3 { for a in 0..=order {
                            let b=order-a;
                            for (jets,width) in [(&restricted,[u[1]-u[0],v[1]-v[0]]),(&stable,[1.,1.])] {
                                let scalar=derivative(p,a,x,width[0])*derivative(q,b,y,width[1]);
                                for (k,factor) in [1.,2.,-1.].into_iter().enumerate() {
                                    // Every oracle operation is exact: integer falling
                                    // factors and quarter endpoints require <=53 bits.
                                    let exact=scalar*factor;let bound=jets[a][b][k];
                                    assert!(bound.lo<=exact && exact<=bound.hi,
                                        "{p}x{q} {domain:?} corner={corner:?} order={a},{b} coordinate={k}: {exact} outside {bound:?}");
                                    checked+=1;
                                }
                            }
                        }}
                    }}
                }
            }}
        }
        assert_eq!(checked,118800);
    }
    #[test]
    fn incomplete_original_spans_preserve_general_blossom_bits() {
        let s=Surface {degree_u:2,degree_v:1,
            knots_u:vec![0.,0.,0.,1.,2.,2.,2.],knots_v:vec![-2.,-2.,2.,2.],
            control_points:(0..4).map(|i|(0..2).map(|j|vec![i as f64,j as f64,(i*j) as f64]).collect()).collect(),
            weights:(0..4).map(|i|vec![1.+i as f64,2.+i as f64]).collect(),periodic_u:false,periodic_v:false};
        s.validate().unwrap();
        for span in [[2,1],[3,1]] {
            let lo=s.knots_u[span[0]];let hi=s.knots_u[span[0]+1];
            for domain in [[[lo,hi],[-2.,2.]],[[lo+0.25,hi-0.25],[-1.,1.]],[[lo,lo],[-2.,-2.]]] {
                assert!(crate::curve_surface_composition::surface_net_on_bernstein(&s,span,domain).unwrap().is_none());
                let expected=crate::curve_surface_composition::surface_net_on(&s,span,domain).unwrap();
                let actual=original_span_net(&s,span,domain).unwrap();
                for (x,y) in actual.iter().flatten().flatten().zip(expected.iter().flatten().flatten()) {
                    assert_eq!([x.lo.to_bits(),x.hi.to_bits()],[y.lo.to_bits(),y.hi.to_bits()]);
                }
            }
        }
    }
}

#[cfg(test)]
mod convex_evaluation_tests {
    use super::*;
    #[test]
    fn full_interval_constant_bernstein_rows_remain_exactly_constant() {
        let value=[2.,-3.,0.,1.].map(I::point);
        let result=evaluate_bernstein(vec![value;6],I::new(0.,1.).unwrap()).unwrap();
        for k in 0..4 {assert_eq!([result[k].lo,result[k].hi],[value[k].lo,value[k].hi]);}
        assert!(evaluate_bernstein(vec![value;2],I::new(-0.1,0.5).unwrap()).is_err());
    }
    #[test]
    fn nonconstant_rows_enclose_the_polynomial_on_each_restricted_interval() {
        let points=[0.,1.,4.].map(|x|[I::point(x);4]).to_vec();
        for range in [[0.,1.],[0.,0.25],[0.25,0.75],[0.75,1.]] {
            let result=evaluate_bernstein(points.clone(),I::new(range[0],range[1]).unwrap()).unwrap();
            for t in [range[0],(range[0]+range[1])*0.5,range[1]] {
                let exact=2.*t+2.*t*t;
                assert!(result.iter().all(|b|b.lo<=exact&&exact<=b.hi));
            }
            assert!(result.iter().all(|b|0.<=b.lo&&b.hi<=4.));
        }
    }
}
