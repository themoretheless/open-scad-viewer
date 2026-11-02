use super::*;
pub(super) fn q(x: f64) -> Q {
    Q::from_float(x).unwrap()
}
pub(super) fn float(x: &Q) -> Result<f64> {
    let f = x
        .to_f64()
        .ok_or_else(|| fail("exact parameter is not representable"))?;
    if Q::from_float(f).as_ref() != Some(x) {
        return Err(fail("exact parameter needs rounding"));
    }
    Ok(f)
}
pub(super) fn clamped(c: &Curve) -> bool {
    !c.periodic
        && c.knots[..=c.degree].iter().all(|&x| x == c.domain()[0])
        && c.knots[c.control_points.len()..]
            .iter()
            .all(|&x| x == c.domain()[1])
}
pub(super) fn line(a: Vec<f64>, b: Vec<f64>) -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a, b],
        weights: vec![1.; 2],
        periodic: false,
    }
}
/// Turn a curve into a constant-V tensor strip. The existing exact composition
/// predicate checks ORIGINAL source coefficients at an affine subinterval;
/// rounded knot insertion is never mistaken for a restriction identity.
pub(super) fn restriction(original: &Curve, saved: &Curve, range: [f64; 2]) -> Result<()> {
    let pad = |p: &Vec<f64>| {
        let mut p = p.clone();
        p.resize(3, 0.);
        p
    };
    let surface = crate::Surface {
        degree_u: original.degree,
        degree_v: 1,
        knots_u: original.knots.clone(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: original
            .control_points
            .iter()
            .map(|p| vec![pad(p); 2])
            .collect(),
        weights: original.weights.iter().map(|&w| vec![w; 2]).collect(),
        periodic_u: original.periodic,
        periodic_v: false,
    };
    let mut candidate = saved.clone();
    candidate.control_points = candidate.control_points.iter().map(pad).collect();
    lift(
        &candidate,
        &line(vec![range[0], 0.], vec![range[1], 0.]),
        &surface,
        false,
    )
}
pub(super) fn lift(curve: &Curve, pc: &Curve, s: &crate::Surface, reversed: bool) -> Result<()> {
    let r = agreement::verify_exact(curve, pc, s, reversed, 1_000_000)?
        .ok_or_else(|| fail("exact saved curve/carrier identity unavailable"))?;
    if r.outcome != cad_predicates::BezierIdentity::Equal {
        return Err(fail("saved curve is not EXACTLY on the original carrier"));
    }
    Ok(())
}
pub(super) fn same(a: &Curve, b: &Curve, reversed: bool) -> Result<()> {
    let d = a.domain();
    restriction(a, b, if reversed { [d[1], d[0]] } else { d })
}
pub(super) fn profile(m: &Model, f: usize) -> Result<()> {
    let f = &m.faces[f];
    let loops = std::iter::once(f.outer)
        .chain(f.holes.iter().copied())
        .map(|w| {
            m.loops[w]
                .coedges
                .iter()
                .map(|c| c.pcurve.clone())
                .collect()
        })
        .collect::<Vec<_>>();
    let p = crate::profile_certificate::certify(&loops, 1e-10)?;
    if p.orientation[0] != 1
        || p.nesting_depth[0] != 0
        || p.orientation.iter().skip(1).any(|&x| x != -1)
        || p.nesting_depth.iter().skip(1).any(|&x| x != 1)
    {
        return Err(fail(
            "region is not one positive connected UV domain with holes",
        ));
    }
    Ok(())
}
pub(super) fn owner(m: &Model, f: usize) -> Result<bool> {
    let uses = m
        .shells
        .iter()
        .flat_map(|s| s.faces.iter())
        .filter(|u| u.face == f)
        .collect::<Vec<_>>();
    if uses.len() != 1 {
        return Err(fail("region face has no unique oriented shell use"));
    }
    Ok(uses[0].reversed)
}
pub(super) fn uses(m: &Model, f: usize) -> impl Iterator<Item = &Coedge> {
    std::iter::once(m.faces[f].outer)
        .chain(m.faces[f].holes.iter().copied())
        .flat_map(|w| m.loops[w].coedges.iter())
}
