//! Complete original-curve winding about a separate point, not polygon parity.
use super::*;
use algebraic_intersections::{ExactRatio, ratio};
use arithmetic::ray_winding as ray;
pub use ray::{RayCellWitness, RayRootWitness};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PointWindingCertificate {
    pub version: u32,
    pub source: String,
    pub point: [ExactRatio; 2],
    pub cells: Vec<RayCellWitness>,
    pub windings: Vec<i64>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedPointWinding {
    windings: Vec<i64>,
    source_cells: usize,
    root_events: usize,
}
impl VerifiedPointWinding {
    pub fn windings(&self) -> &[i64] {
        &self.windings
    }
    pub fn source_cells(&self) -> usize {
        self.source_cells
    }
    pub fn root_events(&self) -> usize {
        self.root_events
    }
    pub fn weighted_winding(&self, coefficients: &[i32]) -> Result<i64> {
        if coefficients.len() != self.windings.len()
            || coefficients.iter().any(|&c| i64::from(c).abs() > 1_000_000)
        {
            return Err(refused(
                "Winding needs one bounded integer per original wire",
            ));
        }
        self.windings
            .iter()
            .zip(coefficients)
            .try_fold(0_i64, |s, (&w, &c)| {
                s.checked_add(
                    w.checked_mul(i64::from(c))
                        .ok_or_else(|| refused("Winding product overflow"))?,
                )
                .ok_or_else(|| refused("Winding total overflow"))
            })
    }
}
fn point(p: [f64; 2]) -> Result<[Q; 2]> {
    if p.iter().any(|v| !v.is_finite()) {
        return Err(refused("Winding query must be finite"));
    }
    Ok(p.map(|v| Q::from_float(v).unwrap()))
}
fn source(loops: &[Vec<Curve>], p: &[Q; 2]) -> Result<String> {
    let a = ratio(&p[0]);
    let b = ratio(&p[1]);
    Ok(format!(
        "point-winding-v1/{}/{}/{}/{}/{}",
        a[0],
        a[1],
        b[0],
        b[1],
        source_bits(loops, 1e-7)?
    ))
}
fn cells(loops: &[Vec<Curve>]) -> Result<Vec<(usize, Vec<[Q; 3]>)>> {
    let nets = extraction::nets(loops)?;
    let endpoint = |n: &Vec<[Q; 3]>, end: bool| {
        let h = if end { n.last().unwrap() } else { &n[0] };
        [&h[0] / &h[2], &h[1] / &h[2]]
    };
    for wire in 0..loops.len() {
        let n = nets
            .iter()
            .filter(|(w, _)| *w == wire)
            .map(|(_, n)| n)
            .collect::<Vec<_>>();
        if n.is_empty()
            || (0..n.len()).any(|i| endpoint(n[i], true) != endpoint(n[(i + 1) % n.len()], false))
        {
            return Err(refused(
                "Original winding wire is not exactly closed across every knot cell",
            ));
        }
    }
    Ok(nets)
}
pub fn certify_point_winding(loops: &[Vec<Curve>], p: [f64; 2]) -> Result<PointWindingCertificate> {
    let p = point(p)?;
    let key = source(loops, &p)?;
    let nets = cells(loops)?;
    let mut c = PointWindingCertificate {
        version: 1,
        source: key,
        point: p.clone().map(|q| ratio(&q)),
        cells: vec![],
        windings: vec![0; loops.len()],
    };
    for (w, net) in nets {
        let r = ray::produce(&net, &p)?;
        c.windings[w] += ray::verify(&net, &p, &r)?;
        c.cells.push(r);
    }
    verify_rational(loops, &p, &c)?;
    Ok(c)
}
/// Reconstruct all original rational knot cells and independently check the
/// complete Sturm ray-root census, algebraic side signs and half-open germs.
/// Boundary points refuse; roots are never rounded to a sample parameter.
pub fn verify_point_winding(
    loops: &[Vec<Curve>],
    p: [f64; 2],
    c: &PointWindingCertificate,
) -> Result<VerifiedPointWinding> {
    verify_rational(loops, &point(p)?, c)
}
fn verify_rational(
    loops: &[Vec<Curve>],
    p: &[Q; 2],
    c: &PointWindingCertificate,
) -> Result<VerifiedPointWinding> {
    if c.version != 1
        || c.source != source(loops, p)?
        || c.point != p.clone().map(|q| ratio(&q))
        || c.windings.len() != loops.len()
    {
        return Err(refused(
            "Point winding source/query binding or schema changed",
        ));
    }
    let nets = cells(loops)?;
    if nets.len() != c.cells.len() {
        return Err(refused("Point winding omits an original knot cell"));
    }
    let mut windings = vec![0_i64; loops.len()];
    let mut root_events = 0;
    for ((w, net), r) in nets.iter().zip(&c.cells) {
        windings[*w] += ray::verify(net, p, r)?;
        root_events += r.roots.len();
    }
    if windings != c.windings {
        return Err(refused(
            "Original winding total differs from complete ray claims",
        ));
    }
    Ok(VerifiedPointWinding {
        windings,
        source_cells: nets.len(),
        root_events,
    })
}
