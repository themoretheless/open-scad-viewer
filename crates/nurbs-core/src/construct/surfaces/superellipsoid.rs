//! Full signed-power superellipsoids as bounded-size bilinear NURBS patches.
//! The analytic interpolation estimate assumes exact samples; libm pow/sin/cos
//! and control rounding are deliberately not presented as a certified bound.
use crate::{Result, check, surface::Surface};
#[derive(Clone, Copy, Debug)]
pub struct Request {
    pub center: [f64; 3],
    pub radii: [f64; 3],
    /// Positive latitude and longitude signed-power exponents.
    pub exponents: [f64; 2],
    pub tolerance: f64,
    /// Total bilinear cells across all eight octants, 8..131072.
    pub max_cells: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop { AnalyticEstimateReached, CellBudget }
#[derive(Clone, Debug)]
pub struct Patch {
    pub surface: Surface,
    pub signs: [i8; 3],
    /// Original normalized positive-octant latitude/longitude rectangle.
    pub domain: [[f64; 2]; 2],
}
#[derive(Clone, Debug)]
pub struct Report {
    pub patches: Vec<Patch>,
    pub cells: usize,
    pub subdivisions: usize,
    /// Continuous interpolation estimate in real arithmetic with exact samples.
    /// Does not bound libm or binary64 control/evaluation error.
    pub analytic_interpolation_estimate: f64,
    pub stop: Stop,
}
fn modulus(e: f64, h: f64) -> f64 {
    // On [0,1], x^e is Holder(e,1) for e<=1, Lipschitz(e) otherwise.
    if e <= 1. { h.powf(e) } else { e*h }
}
fn estimate(r: &Request, n: usize) -> f64 {
    let h = std::f64::consts::FRAC_PI_2 / n as f64;
    let a = modulus(r.exponents[0], h.min(1.));
    let b = modulus(r.exponents[1], h.min(1.));
    // Products of factors in [0,1]: |ab-cd| <= |a-c|+|b-d|.
    (r.radii[0]*(a+b)).hypot(r.radii[1]*(a+b)).hypot(r.radii[2]*a)
}
fn trig(i: usize, n: usize, e: f64) -> [f64;2] {
    if i==0 { return [1.,0.]; }
    if i==n { return [0.,1.]; }
    let (s,c)=(std::f64::consts::FRAC_PI_2*(i as f64/n as f64)).sin_cos();
    [c.powf(e),s.powf(e)]
}
fn knots(start: usize, end: usize, n: usize) -> Vec<f64> {
    let mut k=vec![start as f64/n as f64];
    k.extend((start..=end).map(|i|i as f64/n as f64));
    k.push(end as f64/n as f64); k
}
/// Approximate all eight octants. Shared coordinate tables give bit-identical
/// seam controls, including exact axis zeros and poles. Returns surfaces, not
/// a topological solid; pole singularities and octant normal ownership remain
/// explicit. A budget stop returns the full coarse shape, never partial octants.
pub fn approximate(r: Request) -> Result<Report> {
    check(r.center.iter().all(|v|v.is_finite()) && r.radii.iter().all(|v|v.is_finite()&&*v>0.)
        && r.exponents.iter().all(|v|v.is_finite()&&*v>0.)
        && r.tolerance.is_finite()&&r.tolerance>0.
        && (8..=131072).contains(&r.max_cells), "Invalid superellipsoid dimensions, exponents, tolerance or budget")?;
    check((0..3).all(|i|(r.center[i]+r.radii[i]).is_finite() && (r.center[i]-r.radii[i]).is_finite()
        && r.center[i]+r.radii[i]>r.center[i] && r.center[i]-r.radii[i]<r.center[i]),
        "Superellipsoid dimensions overflow or collapse at coordinate precision")?;
    let mut n=1;
    while estimate(&r,n)>r.tolerance && 8*(n*2)*(n*2)<=r.max_cells {n*=2;}
    let bound=estimate(&r,n);
    check(bound.is_finite(),"Superellipsoid interpolation estimate overflow")?;
    let lat:Vec<_>=(0..=n).map(|i|trig(i,n,r.exponents[0])).collect();
    let lon:Vec<_>=(0..=n).map(|i|trig(i,n,r.exponents[1])).collect();
    let mut patches=Vec::new();
    for sx in [-1,1] {for sy in [-1,1] {for sz in [-1,1] {
        let signs=[sx,sy,sz];
        for a in (0..n).step_by(31) {for b in (0..n).step_by(31) {
            let ae=(a+31).min(n);let be=(b+31).min(n);
            let controls=(a..=ae).map(|i|(b..=be).map(|j| {
                let p=[lat[i][0]*lon[j][0],lat[i][0]*lon[j][1],lat[i][1]];
                (0..3).map(|k|r.center[k]+signs[k] as f64*r.radii[k]*p[k]).collect()
            }).collect()).collect();
            let s=Surface {degree_u:1,degree_v:1,knots_u:knots(a,ae,n),knots_v:knots(b,be,n),
                control_points:controls,weights:vec![vec![1.;be-b+1];ae-a+1],periodic_u:false,periodic_v:false};
            s.validate()?;
            patches.push(Patch {surface:s,signs,domain:[[a as f64/n as f64,ae as f64/n as f64],[b as f64/n as f64,be as f64/n as f64]]});
        }}
    }}}
    Ok(Report {patches,cells:8*n*n,subdivisions:n,analytic_interpolation_estimate:bound,
        stop:if bound<=r.tolerance {Stop::AnalyticEstimateReached} else {Stop::CellBudget}})
}
