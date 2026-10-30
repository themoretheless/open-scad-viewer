//! Exact branch graphs of implicit nonlinear surface-coordinate level sets.
//! A branch certificate is not an assembled clipped solid or a rational Net.
use super::*;
mod triangle;
use crate::profile_certificate::{atlas_roots as roots, ExactRatio, ParameterBranchDecomposition};
pub use triangle::{
    certify_algebraic_triangle_pullback, verify_algebraic_triangle_pullback, AtlasExactTriangle,
};
#[derive(Clone, Debug, PartialEq)]
pub struct AtlasAlgebraicPullback {
    pub version: u32,
    pub source: String,
    pub branches: ParameterBranchDecomposition,
    /// Ordered source signs (below,above) at each open-slab root section.
    pub slab_sides: Vec<Vec<[i8; 2]>>,
}
fn choose(n: usize, k: usize) -> Q {
    (0..k).fold(Q::one(), |v, i| {
        v * integer((n - i) as i64) / integer((i + 1) as i64)
    })
}
fn relation(h: &sb::Net, axis: usize, level: &Q, work: &mut Work) -> Result<Vec<Vec<Q>>> {
    let (n, m) = (h.len() - 1, h[0].len() - 1);
    let mut out = vec![vec![Q::zero(); n + 1]; m + 1];
    for (i, row) in h.iter().enumerate() {
        for (j, h) in row.iter().enumerate() {
            let q = &h[axis] - level * &h[3];
            for a in i..=n {
                for b in j..=m {
                    work.charge(1)?;
                    let mut term = &q
                        * choose(n, i)
                        * choose(n - i, a - i)
                        * choose(m, j)
                        * choose(m - j, b - j);
                    if (a - i + b - j) % 2 != 0 {
                        term = -term;
                    }
                    out[b][a] += term;
                }
            }
        }
    }
    for row in &mut out {
        if row
            .iter()
            .any(|q| q.numer().bits() > 8192 || q.denom().bits() > 8192)
        {
            return Err(fail("implicit knot pullback coefficient budget exceeded"));
        }
        while row.last().is_some_and(Q::is_zero) {
            row.pop();
        }
    }
    while out.last().is_some_and(Vec::is_empty) {
        out.pop();
    }
    Ok(out)
}
fn input(surface: &crate::Surface, axis: u8, level: &ExactRatio) -> Result<(String, Vec<Vec<Q>>)> {
    if surface.control_points.iter().map(Vec::len).sum::<usize>() > 8192 {
        return Err(fail("implicit source control budget exceeded"));
    }
    surface.validate()?;
    if axis > 2 || !(1..=8).contains(&surface.degree_u) || !(1..=8).contains(&surface.degree_v) {
        return Err(fail(
            "implicit surface pullback axis/degree budget exceeded",
        ));
    }
    let level = roots::decode(level)?;
    let range = |knots: &[f64], degree: usize, count: usize| {
        [
            Q::from_float(knots[degree]).unwrap(),
            Q::from_float(knots[count]).unwrap(),
        ]
    };
    let h = sb::cell(
        surface,
        range(
            &surface.knots_u,
            surface.degree_u,
            surface.control_points.len(),
        ),
        range(
            &surface.knots_v,
            surface.degree_v,
            surface.control_points[0].len(),
        ),
    )?;
    let equation = relation(&h, axis as usize, &level, &mut Work::default())?;
    let mut hash = Sha256::new();
    hash.update(b"source-atlas-implicit-pullback-v1");
    hash.update([
        axis,
        u8::from(surface.periodic_u),
        u8::from(surface.periodic_v),
    ]);
    hash.update((surface.degree_u as u64).to_le_bytes());
    hash.update((surface.degree_v as u64).to_le_bytes());
    for knots in [&surface.knots_u, &surface.knots_v] {
        hash.update((knots.len() as u64).to_le_bytes());
        for x in knots {
            hash.update(x.to_bits().to_le_bytes());
        }
    }
    hash.update((surface.control_points.len() as u64).to_le_bytes());
    for row in &surface.control_points {
        hash.update((row.len() as u64).to_le_bytes());
        for p in row {
            hash.update((p.len() as u64).to_le_bytes());
            for x in p {
                hash.update(x.to_bits().to_le_bytes());
            }
        }
    }
    for row in &surface.weights {
        hash.update((row.len() as u64).to_le_bytes());
        for w in row {
            hash.update(w.to_bits().to_le_bytes());
        }
    }
    for s in roots::ratio(&level) {
        hash.update((s.len() as u64).to_le_bytes());
        hash.update(s.as_bytes());
    }
    let source = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    Ok((source, equation))
}
fn at(p: &[Q], t: &Q) -> Q {
    p.iter().rev().fold(Q::zero(), |v, c| v * t + c)
}
fn sides(
    equation: &[Vec<Q>],
    branches: &ParameterBranchDecomposition,
) -> Result<Vec<Vec<[i8; 2]>>> {
    branches
        .slabs
        .iter()
        .map(|slab| {
            let sample = roots::decode(&slab.sample)?;
            let mut fiber = equation.iter().map(|p| at(p, &sample)).collect::<Vec<_>>();
            while fiber.last().is_some_and(Q::is_zero) {
                fiber.pop();
            }
            if fiber.is_empty() {
                return Err(fail(
                    "implicit branch slab lies on a nonisolated vertical component",
                ));
            }
            slab.roots
                .iter()
                .map(|root| {
                    let interval = [roots::decode(&root[0])?, roots::decode(&root[1])?];
                    if roots::count(&fiber, &interval)? != 1 {
                        return Err(fail("implicit slab does not bind one actual source root"));
                    }
                    let mut derivative = fiber.clone();
                    for order in 1..fiber.len() {
                        derivative = derivative
                            .iter()
                            .enumerate()
                            .skip(1)
                            .map(|(i, q)| q * integer(i as i64))
                            .collect();
                        let sign = roots::sign(&derivative, &fiber, &interval)?;
                        if sign != 0 {
                            return Ok([if order % 2 == 0 { sign } else { -sign }, sign]);
                        }
                    }
                    Err(fail("implicit branch multiplicity remains unproved"))
                })
                .collect()
        })
        .collect()
}
pub fn certify_algebraic_surface_pullback(
    surface: &crate::Surface,
    axis: u8,
    level: &ExactRatio,
) -> Result<AtlasAlgebraicPullback> {
    certify_input(input(surface, axis, level)?)
}
fn certify_input((source, equation): (String, Vec<Vec<Q>>)) -> Result<AtlasAlgebraicPullback> {
    let branches = roots::branches(&equation)?;
    let slab_sides = sides(&equation, &branches)?;
    Ok(AtlasAlgebraicPullback {
        version: 1,
        source,
        branches,
        slab_sides,
    })
}
pub fn verify_algebraic_surface_pullback(
    surface: &crate::Surface,
    axis: u8,
    level: &ExactRatio,
    cert: &AtlasAlgebraicPullback,
) -> Result<()> {
    verify_input(input(surface, axis, level)?, cert)
}
fn verify_input(
    (source, equation): (String, Vec<Vec<Q>>),
    cert: &AtlasAlgebraicPullback,
) -> Result<()> {
    if cert.version != 1 || cert.source != source {
        return Err(fail("implicit pullback source/schema binding is false"));
    }
    // Reconstruct critical fibers, Sturm root census and one-sided incidences
    // from the ORIGINAL equation, never trust supplied branch connectivity.
    let branches = roots::branches(&equation)?;
    if cert.branches != branches {
        return Err(fail("implicit pullback branch census/incidence is false"));
    }
    if cert.slab_sides != sides(&equation, &branches)? {
        return Err(fail("implicit pullback source-side labels are false"));
    }
    Ok(())
}
#[cfg(test)]
mod tests;
