//! Source-owned polynomial graph transition. Exact geometry and rounded display
//! storage are distinct; this type grants no coordinate-B-rep or solid admission.
#[path = "original_graph_transition/curvature.rs"]
mod curvature;
#[path = "original_graph_transition/join.rs"]
mod join;
pub use join::JoinReport;
#[path = "original_graph_transition/source_contact.rs"]
mod source_contact;
pub use source_contact::SourceContact;
#[path = "original_graph_transition/source_end.rs"]
mod source_end;
pub use source_end::SourceEndContact;
#[path = "original_graph_transition/source_line.rs"]
mod source_line;
pub use source_line::SourceLine;
#[path = "original_graph_transition/polynomial_lift.rs"]
mod polynomial_lift;
pub use polynomial_lift::polynomial_surface_line;
use crate::{Result, check, curve::Curve, surface::Surface};
pub use curvature::{CurvatureDecision, CurvatureReport};
use num_rational::BigRational as Q;
use num_traits::{One, Signed, ToPrimitive, Zero};

pub struct Budget {
    used: usize,
    limit: usize,
}
impl Budget {
    pub fn new(limit: usize) -> Result<Self> {
        check(
            limit <= 10_000_000,
            "Graph construction budget exceeds limit",
        )?;
        Ok(Self { used: 0, limit })
    }
    pub fn used(&self) -> usize {
        self.used
    }
    fn charge(&mut self, n: usize) -> Result<()> {
        check(
            n <= self.limit.saturating_sub(self.used),
            "Exact graph construction budget exhausted",
        )?;
        self.used += n;
        Ok(())
    }

    fn keep(&mut self, x: Q) -> Result<Q> {
        self.charge(1)?;
        check(x.denom().is_positive(),"Invalid exact graph rational denominator")?;
        check(
            x.numer().bits() <= 8192 && x.denom().bits() <= 8192,
            "Exact graph construction coefficient exceeds bit limit",
        )?;
        Ok(x)
    }
    fn authored(&mut self, x: f64) -> Result<Q> {
        self.keep(Q::from_float(x).ok_or_else(|| crate::numeric_err("Non-finite graph input"))?)
    }
}
/// Only `build` creates this immutable original-source construction. Its exact
/// coefficient net, not `preview()`, defines the new transition geometry.
pub struct Patch {
    sources: [Surface; 2],
    width: Curve,
    coefficients: Vec<Vec<[Q; 3]>>,
    contact_jets: [bool; 2],
    profile: Vec<Q>,
}
/// Polynomial boundary data in a canonical normalized [0,1] clock. This is
/// construction data, not an independent B-rep admission certificate.
pub struct BoundaryDefinition {
    pub points: Vec<[Q;3]>,
    pub uv: [[Q;2];2],
}
impl BoundaryDefinition {

    /// Exact degree elevation in the same normalized parameter clock.
    pub fn elevated(&self,degree:usize,b:&mut Budget)->Result<Self>{
        check((2..=33).contains(&self.points.len())&&degree>=self.points.len()-1&&degree<=32,"Invalid exact boundary elevation degree")?;
        let mut columns=Vec::new();
        for k in 0..3 {
            let input=self.points.iter().map(|p|b.keep(p[k].clone())).collect::<Result<Vec<_>>>()?;
            columns.push(elevate(&input,degree,b)?);
        }
        Ok(Self{points:(0..=degree).map(|i|std::array::from_fn(|k|columns[k][i].clone())).collect(),uv:self.uv.clone()})
    }
    /// Exact restriction in the ORIGINAL normalized polynomial clock. The
    /// resulting curve has a fresh [0,1] clock; original ends remain rational.
    pub fn restricted(&self,range:[Q;2],budget:&mut Budget)->Result<Self>{
        check((2..=33).contains(&self.points.len()),"Invalid polynomial boundary degree")?;
        let lo=budget.keep(range[0].clone())?;let hi=budget.keep(range[1].clone())?;
        check(lo>=Q::zero()&&lo<hi&&hi<=Q::one(),"Choose a strict original boundary subinterval")?;
        let mut components=Vec::new();
        for k in 0..3 {
            let p=self.points.iter().map(|p|budget.keep(p[k].clone())).collect::<Result<Vec<_>>>()?;
            components.push(restrict(&p,&lo,&hi,budget)?);
        }
        let points=(0..self.points.len()).map(|i|std::array::from_fn(|k|components[k][i].clone())).collect();
        let mut uv=std::array::from_fn(|_|std::array::from_fn(|_|Q::zero()));
        for k in 0..2 {
            let a=budget.keep(self.uv[0][k].clone())?;let z=budget.keep(self.uv[1][k].clone())?;
            let delta=budget.keep(z-&a)?;
            uv[0][k]=budget.keep(&a+&delta*&lo)?;uv[1][k]=budget.keep(a+delta*&hi)?;
        }
        Ok(Self{points,uv})
    }
}
/// Complete polynomial equality with possibly different degrees, in the SAME
/// normalized clock. This is not set equality or a parameter correspondence.
pub fn polynomial_points_equal(a:&[[Q;3]],z:&[[Q;3]],b:&mut Budget)->Result<bool>{
    check((2..=33).contains(&a.len())&&(2..=33).contains(&z.len()),"Invalid polynomial identity layout")?;
    let degree=a.len().max(z.len())-1;
    let uv=[[Q::zero(),Q::zero()],[Q::one(),Q::zero()]];
    let a=BoundaryDefinition{points:a.to_vec(),uv:uv.clone()}.elevated(degree,b)?;
    let z=BoundaryDefinition{points:z.to_vec(),uv}.elevated(degree,b)?;
    Ok(a.points==z.points)
}
impl Patch {
    pub fn boundary_definition(&self,side:crate::surface_join::Boundary,budget:&mut Budget)->Result<BoundaryDefinition>{
        use crate::surface_join::Boundary::*;
        let net=&self.coefficients;
        let selected=match side {
            UMin=>net[0].iter().collect::<Vec<_>>(),
            UMax=>net.last().unwrap().iter().collect(),
            VMin=>net.iter().map(|row|&row[0]).collect(),
            VMax=>net.iter().map(|row|row.last().unwrap()).collect(),
        };
        let mut points=Vec::new();
        for p in selected {points.push([budget.keep(p[0].clone())?,budget.keep(p[1].clone())?,budget.keep(p[2].clone())?]);}
        let d=self.width.domain();let a=budget.authored(d[0])?;let b=budget.authored(d[1])?;
        let zero=budget.keep(Q::zero())?;let one=budget.keep(Q::one())?;
        let uv=match side {
            UMin=>[[a.clone(),zero],[a,one]],
            UMax=>[[b.clone(),zero],[b,one]],
            VMin=>[[a,zero.clone()],[b,zero]],
            VMax=>[[a,one.clone()],[b,one]],
        };
        Ok(BoundaryDefinition{points,uv})
    }
    pub fn sources(&self) -> [&Surface; 2] {
        [&self.sources[0], &self.sources[1]]
    }
    pub fn width(&self) -> &Curve {
        &self.width
    }
    pub fn coefficients(&self) -> &[Vec<[Q; 3]>] {
        &self.coefficients
    }
    /// Whole-polynomial positional two-jet identities in each source chart.
    /// Positive width and monotone X prove regularity of this graph patch.
    /// These identities alone do not certify curvature <=10/mm or a body.
    pub fn profile_coefficients(&self)->&[Q]{&self.profile}
    pub fn contact_jets(&self) -> [bool; 2] {
        self.contact_jets
    }
    /// Full transverse boundary identity at adjoining physical X domains.
    /// This proves positional closure only, not G2 at the inter-panel join.
    pub fn joins_position_exact(&self, next: &Patch, budget: &mut Budget) -> Result<bool> {
        if self.sources != next.sources || self.width.domain()[1] != next.width.domain()[0] {
            return Ok(false);
        }
        let a = self.coefficients.last().unwrap();
        let b = &next.coefficients[0];
        for (p, q) in a.iter().zip(b) {
            for k in 0..3 {
                if !budget.keep(&p[k] - &q[k])?.is_zero() {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    /// Rounded display surface and a conservative whole-patch positional error.
    /// Neither this surface nor the error receipt inherits exact contact authority.
    pub fn preview(&self, budget: &mut Budget) -> Result<(Surface, f64)> {
        let mut error = Q::zero();
        let mut rows = Vec::new();
        for row in &self.coefficients {
            let mut out = Vec::new();
            for p in row {
                let mut xyz = Vec::new();
                let mut delta = Q::zero();
                for q in p {
                    let f = q
                        .to_f64()
                        .filter(|x| x.is_finite())
                        .ok_or_else(|| crate::numeric_err("Graph preview overflow"))?;
                    let authored = budget.authored(f)?;
                    let d = budget.keep(q - authored)?;
                    delta = budget.keep(delta + if d < Q::zero() { -d } else { d })?;
                    xyz.push(f);
                }
                if delta > error {
                    error = delta;
                }
                out.push(xyz);
            }
            rows.push(out);
        }
        let mut upper = error
            .to_f64()
            .ok_or_else(|| crate::numeric_err("Graph preview error overflow"))?;
        for _ in 0..4 {
            if Q::from_float(upper).unwrap() >= error {
                break;
            }
            upper = upper.next_up();
        }
        check(
            upper.is_finite() && Q::from_float(upper).unwrap() >= error,
            "Cannot enclose graph preview error",
        )?;
        let d = self.width.domain();
        let n = rows.len();
        let m=rows[0].len();
        let s = Surface {
            degree_u: n - 1,
            degree_v: m-1,
            knots_u: [vec![d[0]; n], vec![d[1]; n]].concat(),
            knots_v: [vec![0.; m], vec![1.; m]].concat(),
            weights: vec![vec![1.; m]; n],
            control_points: rows,
            periodic_u: false,
            periodic_v: false,
        };
        s.validate()?;
        Ok((s, upper))
    }
}
fn choose(n: usize, k: usize) -> u64 {
    (0..k.min(n - k)).fold(1, |a, i| a * (n - i) as u64 / (i + 1) as u64)
}
fn ratio(n: u64, d: u64) -> Q {
    Q::new(n.into(), d.into())
}
fn elevate(a: &[Q], degree: usize, b: &mut Budget) -> Result<Vec<Q>> {
    let n = a.len() - 1;
    (0..=degree)
        .map(|k| {
            let mut sum = Q::zero();
            for i in 0..=n {
                if i <= k && k - i <= degree - n {
                    let x = b.keep(
                        &a[i] * ratio(choose(n, i) * choose(degree - n, k - i), choose(degree, k)),
                    )?;
                    sum = b.keep(sum + x)?;
                }
            }
            Ok(sum)
        })
        .collect()
}
fn product(a: &[Q], v: &[Q], b: &mut Budget) -> Result<Vec<Q>> {
    let n = a.len() - 1;
    let m = v.len() - 1;
    (0..=n + m)
        .map(|k| {
            let mut sum = Q::zero();
            for i in 0..=n {
                if i <= k && k - i <= m {
                    let x = b.keep(&a[i] * &v[k - i])?;
                    let x = b.keep(x * ratio(choose(n, i) * choose(m, k - i), choose(n + m, k)))?;
                    sum = b.keep(sum + x)?;
                }
            }
            Ok(sum)
        })
        .collect()
}
fn split(a: &[Q], t: &Q, b: &mut Budget) -> Result<(Vec<Q>, Vec<Q>)> {
    let mut d = a.to_vec();
    let mut left = vec![d[0].clone()];
    let mut right = vec![d.last().unwrap().clone()];
    let omt = b.keep(Q::one() - t)?;
    for count in (1..a.len()).rev() {
        for i in 0..count {
            let x = b.keep(&d[i] * &omt)?;
            let y = b.keep(&d[i + 1] * t)?;
            d[i] = b.keep(x + y)?;
        }
        left.push(d[0].clone());
        right.push(d[count - 1].clone());
    }
    right.reverse();
    Ok((left, right))
}
fn restrict(a: &[Q], lo: &Q, hi: &Q, b: &mut Budget) -> Result<Vec<Q>> {
    let (left, _) = split(a, hi, b)?;
    if lo.is_zero() {
        return Ok(left);
    }
    let t = b.keep(lo / hi)?;
    Ok(split(&left, &t, b)?.1)
}
/// Qualified class: two opposite, nonperiodic polynomial tensor graph supports,
/// Bezier U and linear V, with physical X=U, Y=+/-V and matching Z=q(U)-h(U)V.
/// The scalar width is an original positive polynomial Bezier on a subinterval.
/// Unsupported graphs refuse; no fitting, source edits or tolerance snapping.
pub fn build(sources: [&Surface; 2], width: &Curve, b: &mut Budget) -> Result<Patch> {
    build_profile(sources,width,vec![Q::one(),ratio(1,2),Q::zero(),ratio(1,2),Q::one()],b)
}
/// Alternative polynomial core with exact original two-jets and boundary G2.
/// This is not an automatic R-law constructor: actual radius/curvature still
/// require qualification with the actual source supports and width law.
pub fn build_quadratic_core_g2(sources:[&Surface;2],width:&Curve,b:&mut Budget)->Result<Patch>{
    let x=vec![-Q::one(),Q::one()];let mut power=vec![Q::one()];
    for _ in 0..20{power=product(&power,&x,b)?;}
    let cap=product(&[Q::zero(),ratio(2,1),Q::zero()],&[Q::zero(),ratio(2,1),Q::zero()],b)?;
    let correction=product(&power,&cap,b)?;
    let base=elevate(&[Q::one(),Q::zero(),Q::one()],24,b)?;
    let profile=base.iter().zip(correction).map(|(a,z)|b.keep(a-z*ratio(1,8))).collect::<Result<Vec<_>>>()?;
    build_profile(sources,width,profile,b)
}
/// Convex transverse G2 blend with zero curvature at both source contacts.
/// The prescribed bending is phi''=v*(1-v)*(56-160*v*(1-v)) >= 0.
/// At V=1/2, phi''=4 preserves the quadratic core's transverse curvature.
/// Its integral fixes endpoint slopes -2,+2; it is a variable-radius blend,
/// not a constant-radius fillet. Width still specifies ORIGINAL contact sites.
pub fn build_convex_g2(sources: [&Surface; 2], width: &Curve, b: &mut Budget) -> Result<Patch> {
    build_profile(sources, width, vec![Q::one(), ratio(2,3), ratio(1,3),
        ratio(7,15), ratio(1,3), ratio(2,3), Q::one()], b)
}
fn build_profile(sources: [&Surface; 2], width: &Curve, profile:Vec<Q>, b: &mut Budget) -> Result<Patch> {
    for s in sources {
        s.validate()?;
    }
    width.validate()?;
    let s = sources[0];
    let n = s.degree_u;
    check(
        (1..=12).contains(&n) && width.degree <= 8,
        "Graph construction degree unsupported",
    )?;
    check(
        !width.periodic
            && width.weights.iter().all(|x| *x == width.weights[0])
            && width.control_points.len() == width.degree + 1
            && width
                .control_points
                .iter()
                .all(|p| p.len() == 3 && p[0] > 0. && p[0] <= 1. && p[1] == 0. && p[2] == 0.),
        "Need a positive polynomial scalar Bezier width",
    )?;
    let d = width.domain();
    check(
        width.knots[..=width.degree].iter().all(|x| *x == d[0])
            && width.knots[width.degree + 1..].iter().all(|x| *x == d[1]),
        "Width must have original clamped Bezier knots",
    )?;
    let domain = [s.knots_u[n], s.knots_u[s.control_points.len()]];
    check(
        d[0] >= domain[0] && d[1] <= domain[1],
        "Width lies outside source chart",
    )?;
    let low = b.authored(domain[0])?;
    let high = b.authored(domain[1])?;
    let range = b.keep(&high - &low)?;
    for (side, source) in sources.into_iter().enumerate() {
        check(
            !source.periodic_u
                && !source.periodic_v
                && source.degree_u == n
                && source.degree_v == 1
                && source.control_points.len() == n + 1
                && source.control_points[0].len() == 2
                && source.knots_u == s.knots_u
                && source.knots_v == [0., 0., 1., 1.]
                && source.knots_u[..=n].iter().all(|x| *x == domain[0])
                && source.knots_u[n + 1..].iter().all(|x| *x == domain[1])
                && source
                    .weights
                    .iter()
                    .flatten()
                    .all(|x| *x == source.weights[0][0]),
            "Need matching polynomial Bezier graph supports",
        )?;
        for i in 0..=n {
            let x = b.keep(&low + &range * ratio(i as u64, n as u64))?;
            for j in 0..2 {
                let p = &source.control_points[i][j];
                check(
                    b.authored(p[0])? == x
                        && p[1]
                            == if j == 0 {
                                0.
                            } else {
                                if side == 0 { -1. } else { 1. }
                            },
                    "Original support is not the required physical graph",
                )?;
                check(
                    p[2] == s.control_points[i][j][2],
                    "Source graph heights differ",
                )?;
            }
        }
    }
    let a = b.authored(d[0])?;
    let end = b.authored(d[1])?;
    let lo = b.keep((&a - &low) / &range)?;
    let hi = b.keep((&end - &low) / &range)?;
    let q = s
        .control_points
        .iter()
        .map(|r| b.authored(r[0][2]))
        .collect::<Result<Vec<_>>>()?;
    let h = s
        .control_points
        .iter()
        .map(|r| {
            let a = b.authored(r[0][2])?;
            let c = b.authored(r[1][2])?;
            b.keep(a - c)
        })
        .collect::<Result<Vec<_>>>()?;
    let q = restrict(&q, &lo, &hi, b)?;
    let h = restrict(&h, &lo, &hi, b)?;
    let w = width
        .control_points
        .iter()
        .map(|p| b.authored(p[0]))
        .collect::<Result<Vec<_>>>()?;
    let degree = n + width.degree;
    let hw = product(&h, &w, b)?;
    let w = elevate(&w, degree, b)?;
    let q = elevate(&q, degree, b)?;
    let phi = profile;
    let v_degree=phi.len()-1;
    let mut coefficients = Vec::new();
    for i in 0..=degree {
        let x = b.keep(&a + (&end - &a) * ratio(i as u64, degree as u64))?;
        let mut row = Vec::new();
        for j in 0..=v_degree {
            let y = b.keep(&w[i] * (ratio(2*j as u64,v_degree as u64)-Q::one()))?;
            let z = b.keep(&q[i] - &hw[i] * &phi[j])?;
            row.push([x.clone(), y, z]);
        }
        coefficients.push(row);
    }
    let mut contact_jets = [true; 2];
    for side in 0..2 {
        for i in 0..=degree {
            let row = &coefficients[i];
            let (at, next, third) = if side == 0 { (0, 1, 2) } else { (v_degree, v_degree-1, v_degree-2) };
            let expected = [
                row[at][0].clone(),
                if side == 0 { -&w[i] } else { w[i].clone() },
                &q[i] - &hw[i],
            ];
            let dv = [
                Q::zero(),
                &w[i] * ratio(2, 1),
                if side == 0 {
                    &hw[i] * ratio(2, 1)
                } else {
                    -&hw[i] * ratio(2, 1)
                },
            ];
            for k in 0..3 {
                let first = b.keep(if side == 0 {
                    (&row[next][k] - &row[at][k]) * ratio(v_degree as u64, 1)
                } else {
                    (&row[at][k] - &row[next][k]) * ratio(v_degree as u64, 1)
                })?;
                let second = b.keep(
                    (&row[third][k] - &row[next][k] * ratio(2, 1) + &row[at][k]) * ratio((v_degree*(v_degree-1)) as u64, 1),
                )?;
                contact_jets[side] &=
                    row[at][k] == expected[k] && first == dv[k] && second.is_zero();
            }
        }
    }
    check(
        contact_jets == [true; 2],
        "Exact source graph contact two-jet identity failed",
    )?;
    Ok(Patch {
        sources: sources.map(Clone::clone),
        width: width.clone(),
        coefficients,
        contact_jets,
        profile:phi,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn inputs() -> ([Surface; 2], Curve) {
        let sources = std::array::from_fn(|side| Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: [0., 0., 1.]
                .into_iter()
                .enumerate()
                .map(|(i, z)| {
                    vec![
                        vec![i as f64 / 2., 0., z],
                        vec![i as f64 / 2., if side == 0 { -1. } else { 1. }, z - 0.25],
                    ]
                })
                .collect(),
            weights: vec![vec![2.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        });
        let width = Curve {
            degree: 3,
            knots: vec![0.125; 4].into_iter().chain(vec![0.875; 4]).collect(),
            control_points: vec![
                vec![0.125, 0., 0.],
                vec![0.25, 0., 0.],
                vec![0.375, 0., 0.],
                vec![0.25, 0., 0.],
            ],
            weights: vec![3.; 4],
            periodic: false,
        };
        (sources, width)
    }
    #[test]
    fn exact_original_contact_jets_survive_non_dyadic_bernstein_coefficients() {
        let (sources, width) = inputs();
        let before = (sources.clone(), width.clone());
        let mut budget = Budget::new(100_000).unwrap();
        let patch = build([&sources[0], &sources[1]], &width, &mut budget).unwrap();
        assert_eq!(patch.contact_jets(), [true; 2]);
        assert_eq!(patch.sources(), [&sources[0], &sources[1]]);
        assert_eq!(patch.width(), &width);
        let non_dyadic = patch
            .coefficients()
            .iter()
            .flatten()
            .flatten()
            .any(|q| Q::from_float(q.to_f64().unwrap()).unwrap() != *q);
        assert!(
            non_dyadic,
            "fixture must expose a coefficient that cannot be stored exactly in binary64"
        );
        let (preview, error) = patch.preview(&mut budget).unwrap();
        assert!(error > 0. && error < 1e-14);
        assert!(budget.used() <= 100_000);
        for (exact, rounded) in patch
            .coefficients()
            .iter()
            .flatten()
            .zip(preview.control_points.iter().flatten())
        {
            let residual = exact
                .iter()
                .zip(rounded)
                .map(|(q, x)| {
                    let v = q - Q::from_float(*x).unwrap();
                    if v < Q::zero() { -v } else { v }
                })
                .sum::<Q>();
            assert!(residual <= Q::from_float(error).unwrap());
        }
        assert_eq!((sources, width), before);
    }
    #[test]
    fn exact_patch_agrees_with_independent_closed_form_geometry() {
        let (sources, width) = inputs();
        let patch = build(
            [&sources[0], &sources[1]],
            &width,
            &mut Budget::new(100_000).unwrap(),
        )
        .unwrap();
        let n = patch.coefficients.len() - 1;
        for t in [ratio(0, 1), ratio(1, 3), ratio(1, 1)] {
            for v in [ratio(0, 1), ratio(1, 4), ratio(1, 1)] {
                let mut value: [Q; 3] = std::array::from_fn(|_| Q::zero());
                for (i, row) in patch.coefficients.iter().enumerate() {
                    for (j, p) in row.iter().enumerate() {
                        let factor = ratio(choose(n, i) * choose(4, j), 1)
                            * t.pow(i as i32)
                            * (Q::one() - &t).pow((n - i) as i32)
                            * v.pow(j as i32)
                            * (Q::one() - &v).pow((4 - j) as i32);
                        for k in 0..3 {
                            value[k] += &p[k] * &factor;
                        }
                    }
                }
                // Independent physical graph: z=x^2-h*w*phi, h=1/4.
                let x = ratio(1, 8) + ratio(3, 4) * &t;
                let w = ratio(1, 8) + ratio(3, 8) * &t - ratio(1, 4) * t.pow(3);
                let phi =
                    Q::one() - ratio(2, 1) * &v + ratio(4, 1) * v.pow(3) - ratio(2, 1) * v.pow(4);
                assert_eq!(
                    value,
                    [
                        x.clone(),
                        &w * (ratio(2, 1) * &v - Q::one()),
                        &x * &x - ratio(1, 4) * &w * phi
                    ]
                );
            }
        }
    }
    #[test]
    fn exact_field_curvature_requires_complete_shared_budget_coverage() {
        let (s, w) = inputs();
        let mut budget = Budget::new(10_000_000).unwrap();
        let patch = build([&s[0], &s[1]], &w, &mut budget).unwrap();
        let before = budget.used();
        let complete = patch.inspect_curvature(10., 10000, &mut budget).unwrap();
        assert_eq!(complete.decision, CurvatureDecision::Within, "{complete:?}");
        assert!(complete.absolute_curvature_upper.unwrap() <= 10.);
        assert!(complete.unresolved_regions.is_empty() && complete.covered_cells > 0);
        assert_eq!(complete.coefficient_work, budget.used() - before);
        let no_cells = patch.inspect_curvature(10., 0, &mut budget).unwrap();
        assert_eq!(no_cells.decision, CurvatureDecision::Unresolved);
        assert!(
            no_cells.absolute_curvature_upper.is_none() && !no_cells.unresolved_regions.is_empty()
        );
        let short = patch
            .inspect_curvature(10., 100, &mut Budget::new(0).unwrap())
            .unwrap();
        assert_eq!(short.decision, CurvatureDecision::Unresolved);
        assert!(short.absolute_curvature_upper.is_none());
        let too_small = patch
            .inspect_curvature(0.000001, 10000, &mut budget)
            .unwrap();
        assert_eq!(
            too_small.decision,
            CurvatureDecision::Exceeds,
            "{too_small:?}"
        );
        assert!(too_small.exceeds_region.is_some() && too_small.absolute_curvature_upper.is_none());
        assert!(budget.used() <= 10_000_000);
    }
    #[test]
    fn whole_join_g2_distinguishes_matching_rims_and_changed_cross_derivatives(){
        let(s,mut w)=inputs();w.control_points=vec![vec![0.25,0.,0.];4];
        w.knots=[vec![0.;4],vec![0.5;4]].concat();
        let mut budget=Budget::new(1_000_000).unwrap();
        let left=build([&s[0],&s[1]],&w,&mut budget).unwrap();
        w.knots=[vec![0.5;4],vec![1.;4]].concat();
        let right=build([&s[0],&s[1]],&w,&mut budget).unwrap();
        let before=budget.used();let r=left.inspect_join_g2(&right,1e-5,1e-5,&mut budget).unwrap();
        assert!(r.within&&r.position_exact,"{r:?}");
        assert!(r.normal_difference_upper.unwrap()<1e-10&&r.tensor_difference_upper.unwrap()<1e-10);
        assert_eq!(r.coefficient_work,budget.used()-before);
        w.control_points[1][0]+=0.125;
        let changed=build([&s[0],&s[1]],&w,&mut budget).unwrap();
        let r=left.inspect_join_g2(&changed,1e-5,1e-5,&mut budget).unwrap();
        assert!(r.position_exact&&!r.within,"{r:?}");
        let short=left.inspect_join_g2(&right,1e-5,1e-5,&mut Budget::new(0).unwrap()).unwrap();
        assert!(!short.within&&short.normal_difference_upper.is_none()&&short.tensor_difference_upper.is_none());
        assert_eq!(short.coefficient_work,0);
    }
    #[test]
    fn restricted_boundary_has_exact_original_non_dyadic_endpoints(){
        let(s,w)=inputs();let mut budget=Budget::new(100_000).unwrap();
        let patch=build([&s[0],&s[1]],&w,&mut budget).unwrap();
        let full=patch.boundary_definition(crate::surface_join::Boundary::UMin,&mut budget).unwrap();
        let points=full.points.clone();let range=[ratio(1,3),ratio(2,3)];
        let part=full.restricted(range.clone(),&mut budget).unwrap();
        assert_eq!(part.uv,[[ratio(1,8),range[0].clone()],[ratio(1,8),range[1].clone()]]);
        for (at,v) in [0,4].into_iter().zip(range) {
            let phi=Q::one()-ratio(2,1)*&v+ratio(4,1)*v.pow(3)-ratio(2,1)*v.pow(4);
            assert_eq!(part.points[at],[ratio(1,8),ratio(1,8)*(ratio(2,1)*v-Q::one()),ratio(1,64)-ratio(1,32)*phi]);
        }
        assert_eq!(full.points,points);
        assert!(full.restricted([Q::new_raw(1.into(),0.into()),ratio(1,1)],&mut budget).is_err());
        assert!(full.restricted([ratio(1,2),ratio(1,2)],&mut budget).is_err());
        assert!(full.restricted([ratio(0,1),ratio(1,1)],&mut Budget::new(0).unwrap()).is_err());
    }
    #[test]
    fn adjacent_exact_panels_share_the_complete_transverse_boundary() {
        let (sources, mut width) = inputs();
        width.control_points = vec![vec![0.25, 0., 0.]; 4];
        width.knots = [vec![0.; 4], vec![0.5; 4]].concat();
        let mut budget = Budget::new(100_000).unwrap();
        let left = build([&sources[0], &sources[1]], &width, &mut budget).unwrap();
        width.knots = [vec![0.5; 4], vec![1.; 4]].concat();
        let right = build([&sources[0], &sources[1]], &width, &mut budget).unwrap();
        assert!(left.joins_position_exact(&right, &mut budget).unwrap());
        assert!(!right.joins_position_exact(&left, &mut budget).unwrap());
        width.control_points[0][0] += 2f64.powi(-40);
        let moved = build([&sources[0], &sources[1]], &width, &mut budget).unwrap();
        assert!(!left.joins_position_exact(&moved, &mut budget).unwrap());
    }
    #[test]
    fn unsupported_sources_and_missing_width_do_not_construct_a_patch() {
        let (mut sources, mut width) = inputs();
        let before = sources.clone();
        width.control_points[0][0] = 0.;
        assert!(
            build(
                [&sources[0], &sources[1]],
                &width,
                &mut Budget::new(100_000).unwrap()
            )
            .is_err()
        );
        let (_, width) = inputs();
        sources[1].control_points[1][1][2] += 2f64.powi(-20);
        assert!(
            build(
                [&sources[0], &sources[1]],
                &width,
                &mut Budget::new(100_000).unwrap()
            )
            .is_err()
        );
        sources = before;
        sources[0].weights[0][0] = 1.;
        assert!(
            build(
                [&sources[0], &sources[1]],
                &width,
                &mut Budget::new(100_000).unwrap()
            )
            .is_err()
        );
    }
    #[test]
    fn one_shared_budget_is_not_reset_between_patch_and_preview() {
        let (s, w) = inputs();
        let mut reference = Budget::new(100_000).unwrap();
        let patch = build([&s[0], &s[1]], &w, &mut reference).unwrap();
        let cost = reference.used();
        let mut short = Budget::new(cost - 1).unwrap();
        assert!(build([&s[0], &s[1]], &w, &mut short).is_err());
        assert!(short.used() <= cost - 1);
        let mut exact = Budget::new(cost).unwrap();
        let owned = build([&s[0], &s[1]], &w, &mut exact).unwrap();
        assert!(owned.preview(&mut exact).is_err());
        assert_eq!(exact.used(), cost);
        assert_eq!(patch.contact_jets(), owned.contact_jets());
    }
}

#[path = "original_graph_transition/polynomial_flux.rs"]
mod polynomial_flux;
pub use polynomial_flux::{PolynomialFlux,polynomial_surface_flux,sum_fluxes};

#[path = "original_graph_transition/transverse_contact.rs"]
mod transverse_contact;

#[path = "original_graph_transition/chart_embedding.rs"]
mod chart_embedding;
pub use chart_embedding::{PolynomialChartEmbedding,polynomial_surface_embedding};

#[path = "original_graph_transition/retained_projection.rs"]
mod retained_projection;

#[path = "original_graph_transition/support_plane.rs"]
mod support_plane;
pub use support_plane::{PolynomialPlaneTrace,polynomial_supporting_trace,polynomial_trim_trace_covered};

#[path = "original_graph_transition/normal_radius.rs"]
mod normal_radius;
pub use normal_radius::{NormalRadiusDecision,NormalRadiusReport};

#[path = "original_graph_transition/correlated_radius.rs"]
mod correlated_radius;

#[path = "original_graph_transition/source_curvature_law.rs"]
mod source_curvature_law;
pub use source_curvature_law::{SourceContactCurvatureLaw,SourceCurvatureLawReport,SourceCurvatureLawCoverage,CurvatureLawDecision};

#[path = "original_graph_transition/support_frame.rs"]
mod support_frame;
pub use support_frame::{SupportFrame,polynomial_support_frame,opposed_normal_chords_excluded,frame_distance_at_least,polynomial_plane_point,convex_uv_strict_contains};

#[path="original_graph_transition/edge_display.rs"]
mod edge_display;
pub use edge_display::{PolynomialEdgeDisplay,sample_polynomial_edge};
