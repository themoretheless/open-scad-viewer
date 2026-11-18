//! Outward Green-integral enclosures for retained rational Bezier spans.
//! Simpson's signed fourth-derivative remainder encloses the whole interval,
//! including denominator/coordinate rounding; samples alone are never proof.
use super::*;
use nurbs_core::interval_eval::Interval as I;
type Net = Vec<[I; 3]>;
fn split(net: &Net) -> Result<(Net, Net)> {
    let mut q = net.clone();
    let mut a = vec![q[0]];
    let mut b = vec![*q.last().unwrap()];
    while q.len() > 1 {
        q = q
            .windows(2)
            .map(|p| {
                Ok([
                    p[0][0].add(p[1][0])?.mul(I::point(0.5))?,
                    p[0][1].add(p[1][1])?.mul(I::point(0.5))?,
                    p[0][2].add(p[1][2])?.mul(I::point(0.5))?,
                ])
            })
            .collect::<Result<Net>>()?;
        a.push(q[0]);
        b.push(*q.last().unwrap());
    }
    b.reverse();
    Ok((a, b))
}
fn jets(net: &Net, endpoint: Option<bool>) -> Result<Vec<[I; 2]>> {
    let mut q = net.clone();
    let mut h = Vec::new();
    for _ in 0..=5 {
        h.push(std::array::from_fn::<_, 3, _>(|k| {
            if let Some(end) = endpoint {
                q[if end { q.len() - 1 } else { 0 }][k]
            } else {
                I {
                    lo: q.iter().map(|p| p[k].lo).fold(f64::INFINITY, f64::min),
                    hi: q.iter().map(|p| p[k].hi).fold(f64::NEG_INFINITY, f64::max),
                }
            }
        }));
        let degree = q.len() - 1;
        q = if degree == 0 {
            vec![[I::point(0.); 3]]
        } else {
            q.windows(2)
                .map(|p| {
                    Ok([
                        p[1][0].sub(p[0][0])?.mul(I::point(degree as f64))?,
                        p[1][1].sub(p[0][1])?.mul(I::point(degree as f64))?,
                        p[1][2].sub(p[0][2])?.mul(I::point(degree as f64))?,
                    ])
                })
                .collect::<Result<Net>>()?
        };
    }
    let mut r: Vec<[I; 2]> = Vec::new();
    for k in 0..=5 {
        let mut v = [I::point(0.); 2];
        for axis in 0..2 {
            let mut n = h[k][axis];
            for j in 1..=k {
                n = n.sub(h[j][2].mul(r[k - j][axis])?.mul(I::point(choose(k, j)))?)?;
            }
            v[axis] = n.div(h[0][2])?;
        }
        r.push(v);
    }
    Ok(r)
}
fn cross(a: [I; 2], b: [I; 2]) -> Result<I> {
    a[0].mul(b[1])?.sub(a[1].mul(b[0])?)
}
fn cell(net: &Net) -> Result<I> {
    let r = jets(net, None)?;
    let f4 = cross(r[2], r[3])?
        .mul(I::point(2.))?
        .add(cross(r[1], r[4])?.mul(I::point(3.))?)?
        .add(cross(r[0], r[5])?)?
        .mul(I::point(0.5))?;
    let a = jets(net, Some(false))?;
    let b = jets(net, Some(true))?;
    let (left, _) = split(net)?;
    let m = jets(&left, Some(true))?;
    // m' is in the half-cell parameter, so its Green integrand is doubled.
    let simpson = cross(a[0], a[1])?
        .add(cross(b[0], b[1])?)?
        .add(cross(m[0], m[1])?.mul(I::point(8.))?)?
        .div(I::point(12.))?;
    simpson.sub(f4.div(I::point(2880.))?)
}
pub(super) fn integral(curve: &Curve, origin: [f64; 2], error: f64) -> Result<I> {
    let net = curve
        .control_points
        .iter()
        .zip(&curve.weights)
        .map(|(p, w)| {
            Ok([
                I::point(p[0]).sub(I::point(origin[0]))?.mul(I::point(*w))?,
                I::point(p[1]).sub(I::point(origin[1]))?.mul(I::point(*w))?,
                I::point(*w),
            ])
        })
        .collect::<Result<Net>>()?;
    let mut stack = vec![(net, error, 0u8)];
    let mut area = I::point(0.);
    let mut work = 0;
    while let Some((net, budget, depth)) = stack.pop() {
        work += 1;
        if work > 8192 {
            return Err(refused(
                "Rational Green integral exceeded its enclosure budget",
            ));
        }
        let bounds = cell(&net)?;
        if bounds.hi - bounds.lo <= 2. * budget {
            area = area.add(bounds)?;
            continue;
        }
        if depth == 24 {
            return Err(refused(
                "Rational Green integral reached its precision limit",
            ));
        }
        let (a, b) = split(&net)?;
        stack.push((b, budget * 0.5, depth + 1));
        stack.push((a, budget * 0.5, depth + 1));
    }
    Ok(area)
}
