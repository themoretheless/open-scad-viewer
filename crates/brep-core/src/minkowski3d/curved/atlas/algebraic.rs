//! Source-bound algebraic boundary events. No root midpoint is a parameter.
//! These certificates do not construct rational control nets at irrational cuts.
use super::*;
use crate::profile_certificate::{atlas_roots as roots, ExactPolynomial, ExactRatio};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AtlasAlgebraicRoot {
    pub polynomial: ExactPolynomial,
    pub interval: [ExactRatio; 2],
    /// Source signs on either side of an isolated irrational event.
    /// An exact rational singleton uses [0, 0].
    pub sides: [i8; 2],
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AtlasAlgebraicParameters {
    pub version: u32,
    pub source: String,
    pub roots: Vec<AtlasAlgebraicRoot>,
}
fn input(curve: &crate::Curve) -> Result<Vec<sb::H>> {
    curve.validate()?;
    let [a, b] = curve.domain().map(|v| Q::from_float(v).unwrap());
    sb::curve(curve, &a, &b)
}
fn power(h: &[sb::H], axis: usize) -> Result<Vec<Q>> {
    nonlinear_cancel::power(
        &h.iter().map(|p| p[axis].clone()).collect::<Vec<_>>(),
        &mut Work::default(),
    )
}
fn at(p: &[Q], t: &Q) -> Q {
    p.iter().rev().fold(Q::zero(), |v, c| v * t + c)
}
fn binding(h: &[sb::H], equations: &[Vec<Q>], mode: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(b"source-atlas-algebraic-parameters-v1");
    hash.update(mode.as_bytes());
    // Length-prefixed canonical rationals make this independent of JSON/F64 output.
    for p in h {
        for q in p {
            for s in roots::ratio(q) {
                hash.update((s.len() as u64).to_le_bytes());
                hash.update(s.as_bytes());
            }
        }
    }
    hash.update((equations.len() as u64).to_le_bytes());
    for p in equations {
        hash.update((p.len() as u64).to_le_bytes());
        for q in p {
            for s in roots::ratio(q) {
                hash.update((s.len() as u64).to_le_bytes());
                hash.update(s.as_bytes());
            }
        }
    }
    hash.finalize().iter().map(|v| format!("{v:02x}")).collect()
}
fn common(equations: &[Vec<Q>]) -> Result<Vec<Q>> {
    let mut g = Vec::new();
    for p in equations {
        g = roots::gcd(&g, p)?;
    }
    if g.len() < 2 {
        return Err(fail("algebraic seam has no isolated parameter event"));
    }
    Ok(g)
}
fn produce(h: &[sb::H], equations: &[Vec<Q>], mode: &str) -> Result<AtlasAlgebraicParameters> {
    let g = common(equations)?;
    let first = equations
        .iter()
        .find(|p| p.iter().any(|q| !q.is_zero()))
        .unwrap();
    let mut intervals = Vec::new();
    for r in roots::roots(first)? {
        if roots::count(&g, &r)? == 1 {
            intervals.push(r);
        }
    }
    if intervals.is_empty() {
        return Err(fail(
            "algebraic seam point is absent from the active domain",
        ));
    }
    let roots = intervals
        .into_iter()
        .map(|r| {
            // Sides apply to the first nonzero original equation, not an arbitrary
            // monic gcd whose normalization can flip the original source sign.
            let p = equations
                .iter()
                .find(|p| p.iter().any(|q| !q.is_zero()))
                .unwrap();
            let sides = if r[0] == r[1] {
                [0, 0]
            } else {
                [
                    at(p, &r[0]).signum().to_integer().try_into().unwrap(),
                    at(p, &r[1]).signum().to_integer().try_into().unwrap(),
                ]
            };
            AtlasAlgebraicRoot {
                polynomial: roots::polynomial(&g),
                interval: r.map(|q| roots::ratio(&q)),
                sides,
            }
        })
        .collect();
    let cert = AtlasAlgebraicParameters {
        version: 1,
        source: binding(h, equations, mode),
        roots,
    };
    verify(h, equations, mode, &cert)?;
    Ok(cert)
}
fn verify(
    h: &[sb::H],
    equations: &[Vec<Q>],
    mode: &str,
    cert: &AtlasAlgebraicParameters,
) -> Result<()> {
    if cert.version != 1 || cert.source != binding(h, equations, mode) || cert.roots.len() > 64 {
        return Err(fail("algebraic seam source/schema binding is false"));
    }
    let g = common(equations)?;
    let mut previous = None;
    let mut interior = 0usize;
    let mut endpoints = [false; 2];
    let w = power(h, 3)?;
    for root in &cert.roots {
        let f = roots::decode_polynomial(&root.polynomial)?;
        if f.len() < 2 {
            return Err(fail("constant algebraic event polynomial"));
        }
        let r = [
            roots::decode(&root.interval[0])?,
            roots::decode(&root.interval[1])?,
        ];
        if r[0] < Q::zero()
            || r[1] > Q::one()
            || r[0] > r[1]
            || previous.as_ref().is_some_and(|p: &Q| *p >= r[0])
        {
            return Err(fail("unordered/out-of-domain algebraic root intervals"));
        }
        previous = Some(r[1].clone());
        if roots::count(&f, &r)? != 1 || roots::count(&g, &r)? != 1 {
            return Err(fail(
                "algebraic interval does not select one actual source root",
            ));
        }
        if r[0] != r[1]
            && (at(&f, &r[0]).is_zero()
                || at(&f, &r[1]).is_zero()
                || at(&g, &r[0]).is_zero()
                || at(&g, &r[1]).is_zero())
        {
            return Err(fail("algebraic open interval has a root endpoint"));
        }
        // Prove f and every original equation refer to the SAME root, not merely
        // different roots lying inside an overlapping isolating interval.
        for equation in equations {
            if !roots::zero(equation, &f, &r)? {
                return Err(fail(
                    "selected algebraic root does not satisfy all source coordinates",
                ));
            }
        }
        if roots::sign(&w, &f, &r)? <= 0 {
            return Err(fail("selected algebraic seam denominator is not positive"));
        }
        let p = equations
            .iter()
            .find(|p| p.iter().any(|q| !q.is_zero()))
            .unwrap();
        let sign = |q: Q| {
            if q > Q::zero() {
                1
            } else if q < Q::zero() {
                -1
            } else {
                0
            }
        };
        if r[0] != r[1]
            && (roots::count(p, &r)? != 1 || at(p, &r[0]).is_zero() || at(p, &r[1]).is_zero())
        {
            return Err(fail(
                "algebraic side interval contains an additional source zero",
            ));
        }
        let expected = if r[0] == r[1] {
            [0, 0]
        } else {
            [sign(at(p, &r[0])), sign(at(p, &r[1]))]
        };
        if root.sides != expected {
            return Err(fail("algebraic source side signs are false"));
        }
        if r[0] == Q::zero() && r[1] == Q::zero() {
            endpoints[0] = true;
        } else if r[0] == Q::one() && r[1] == Q::one() {
            endpoints[1] = true;
        } else {
            interior += 1;
        }
    }
    if interior != roots::count(&g, &[Q::zero(), Q::one()])?
        || endpoints != [at(&g, &Q::zero()).is_zero(), at(&g, &Q::one()).is_zero()]
    {
        return Err(fail("algebraic parameter census omits a source branch"));
    }
    Ok(())
}
fn mode(curve: &crate::Curve, kind: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(kind.as_bytes());
    hash.update((curve.degree as u64).to_le_bytes());
    hash.update([u8::from(curve.periodic)]);
    hash.update((curve.control_points.len() as u64).to_le_bytes());
    for p in &curve.control_points {
        hash.update((p.len() as u64).to_le_bytes());
        for v in p {
            hash.update(v.to_bits().to_le_bytes());
        }
    }
    for values in [&curve.weights, &curve.knots] {
        hash.update((values.len() as u64).to_le_bytes());
        for v in values {
            hash.update(v.to_bits().to_le_bytes());
        }
    }
    format!(
        "{kind}/{}",
        hash.finalize()
            .iter()
            .map(|v| format!("{v:02x}"))
            .collect::<String>()
    )
}
fn seam(curve: &crate::Curve, point: &[ExactRatio; 3]) -> Result<(Vec<sb::H>, Vec<Vec<Q>>)> {
    if curve.control_points.first().is_none_or(|p| p.len() != 3) {
        return Err(fail("algebraic seam needs three source coordinates"));
    }
    let h = input(curve)?;
    let point = point
        .iter()
        .map(roots::decode)
        .collect::<Result<Vec<_>>>()?;
    let mut equations = Vec::new();
    for axis in 0..3 {
        equations.push(nonlinear_cancel::power(
            &h.iter()
                .map(|p| &p[axis] - &point[axis] * &p[3])
                .collect::<Vec<_>>(),
            &mut Work::default(),
        )?);
    }
    Ok((h, equations))
}
pub fn certify_algebraic_seam_parameters(
    curve: &crate::Curve,
    point: &[ExactRatio; 3],
) -> Result<AtlasAlgebraicParameters> {
    let (h, e) = seam(curve, point)?;
    produce(&h, &e, &mode(curve, "seam"))
}
pub fn verify_algebraic_seam_parameters(
    curve: &crate::Curve,
    point: &[ExactRatio; 3],
    cert: &AtlasAlgebraicParameters,
) -> Result<()> {
    let (h, e) = seam(curve, point)?;
    verify(&h, &e, &mode(curve, "seam"), cert)
}
fn knot(curve: &crate::Curve, axis: u8, level: &ExactRatio) -> Result<(Vec<sb::H>, Vec<Vec<Q>>)> {
    if axis > 2
        || curve
            .control_points
            .first()
            .is_none_or(|p| axis as usize >= p.len())
    {
        return Err(fail("invalid algebraic knot axis"));
    }
    let h = input(curve)?;
    let level = roots::decode(level)?;
    let equation = nonlinear_cancel::power(
        &h.iter()
            .map(|p| &p[axis as usize] - &level * &p[3])
            .collect::<Vec<_>>(),
        &mut Work::default(),
    )?;
    Ok((h, vec![equation]))
}
pub fn certify_algebraic_knot_parameters(
    curve: &crate::Curve,
    axis: u8,
    level: &ExactRatio,
) -> Result<AtlasAlgebraicParameters> {
    let (h, e) = knot(curve, axis, level)?;
    produce(&h, &e, &mode(curve, &format!("knot/{axis}/{level:?}")))
}
pub fn verify_algebraic_knot_parameters(
    curve: &crate::Curve,
    axis: u8,
    level: &ExactRatio,
    cert: &AtlasAlgebraicParameters,
) -> Result<()> {
    let (h, e) = knot(curve, axis, level)?;
    verify(
        &h,
        &e,
        &mode(curve, &format!("knot/{axis}/{level:?}")),
        cert,
    )
}

#[cfg(test)]
mod tests;

// Exact membership of a rational spatial point in the open source edge. Used
// only to propose a graph reparameterization; its full image proof is separate.
pub(super) fn contains_point(h: &[sb::H], point: &P, work: &mut Work) -> Result<bool> {
    if h.iter().any(|p| p[3] < Q::zero()) || !h.iter().any(|p| p[3] > Q::zero()) {
        return Ok(false);
    }
    let mut g = Vec::new();
    for axis in 0..3 {
        let p = nonlinear_cancel::power(
            &h.iter()
                .map(|p| &p[axis] - &point[axis] * &p[3])
                .collect::<Vec<_>>(),
            work,
        )?;
        work.charge(p.len() * p.len())?;
        g = roots::gcd(&g, &p)?;
        if g.len() == 1 {
            return Ok(false);
        }
    }
    if g.len() < 2 {
        return Ok(false);
    }
    work.charge(8192)?;
    Ok(roots::count(&g, &[Q::zero(), Q::one()])? > 0)
}
