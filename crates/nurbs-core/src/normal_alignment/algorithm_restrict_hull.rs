//! normal alignment algorithm: restrict_hull.
use super::*;
pub fn restrict_hull(
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
