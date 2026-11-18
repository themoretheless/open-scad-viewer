//! Exact oriented UV-chain cancellation, without closest-point welding.
use super::*;
fn opposite(a: &crate::Curve, b: &crate::Curve) -> bool {
    if a.periodic
        || b.periodic
        || a.degree != b.degree
        || a.domain() != b.domain()
        || a.control_points.iter().rev().ne(b.control_points.iter())
        || a.weights.iter().rev().ne(b.weights.iter())
        || a.knots.len() != b.knots.len()
    {
        return false;
    }
    let q = |x| Q::from_float(x).unwrap();
    let sum = q(a.domain()[0]) + q(a.domain()[1]);
    a.knots
        .iter()
        .rev()
        .zip(&b.knots)
        .all(|(&x, &y)| q(x) + q(y) == sum)
}
pub(super) fn owner(m: &Model, face: usize) -> Result<(usize, bool)> {
    let uses = m
        .shells
        .iter()
        .enumerate()
        .flat_map(|(s, shell)| {
            shell
                .faces
                .iter()
                .filter(move |u| u.face == face)
                .map(move |u| (s, u.reversed))
        })
        .collect::<Vec<_>>();
    if uses.len() != 1 || !m.shells[uses[0].0].closed {
        return Err(fail(
            "same-carrier face needs one closed oriented shell owner",
        ));
    }
    Ok(uses[0])
}
pub(super) fn boundary(
    m: &Model,
    selected: &[usize],
) -> Result<(Vec<Vec<crate::Coedge>>, Vec<usize>)> {
    let surface = &m.faces[selected[0]].surface;
    let owned = owner(m, selected[0])?;
    let mut incidence = BTreeMap::<usize, Vec<&crate::Coedge>>::new();
    for &i in selected {
        if &m.faces[i].surface != surface || owner(m, i)? != owned {
            return Err(fail(
                "seam cancellation requires the identical carrier and oriented owner",
            ));
        }
        for w in std::iter::once(m.faces[i].outer).chain(m.faces[i].holes.iter().copied()) {
            for c in &m.loops[w].coedges {
                if m.edges[c.edge].degenerate {
                    return Err(fail("pole seams need separate cancellation proof"));
                }
                incidence.entry(c.edge).or_default().push(c);
            }
        }
    }
    let mut rim = Vec::new();
    let mut removed = Vec::new();
    for (edge, uses) in incidence {
        match uses.as_slice() {
            [a] => rim.push((*a).clone()),
            [a, b] if a.reversed != b.reversed && opposite(&a.pcurve, &b.pcurve) => {
                removed.push(edge)
            }
            _ => return Err(fail("original seam UV maps do not cancel exactly")),
        }
    }
    if removed.is_empty() || rim.len() > 256 {
        return Err(fail(
            "same-carrier merge needs bounded shared seams and rim",
        ));
    }
    let mut outgoing = BTreeMap::new();
    let mut incoming = BTreeSet::new();
    for (i, c) in rim.iter().enumerate() {
        let e = &m.edges[c.edge];
        let from = e.vertices[usize::from(c.reversed)];
        let to = e.vertices[usize::from(!c.reversed)];
        if outgoing.insert(from, i).is_some() || !incoming.insert(to) {
            return Err(fail("merged rim branches at an original vertex"));
        }
    }
    if outgoing.keys().copied().collect::<BTreeSet<_>>() != incoming {
        return Err(fail("merged rim is open"));
    }
    let mut seen = BTreeSet::new();
    let mut loops = Vec::new();
    for start in 0..rim.len() {
        if seen.contains(&start) {
            continue;
        }
        let mut path = Vec::new();
        let mut at = start;
        loop {
            if !seen.insert(at) {
                if at != start {
                    return Err(fail("merged cycles share a rim"));
                }
                break;
            }
            let c = &rim[at];
            let v = m.edges[c.edge].vertices[usize::from(!c.reversed)];
            path.push(c.clone());
            at = outgoing[&v];
        }
        loops.push(path);
    }
    Ok((loops, removed))
}
