//! Sufficient interval proof of nonzero normals on original knot rectangles.
use crate::{Result, check, resource, surface::Surface, surface_measure::jets};
#[derive(Clone, Debug)]
pub struct UnresolvedCell {
    pub domain: [[f64; 2]; 2],
}
#[derive(Clone, Debug)]
pub struct Report {
    /// Every original span has a nonzero one-sided normal over its closure.
    pub spanwise_regular: bool,
    /// Internal knot multiplicities guarantee at least C1 basis continuity.
    /// Periodic closure continuity is not certified by this flag.
    pub interior_basis_c1: bool,
    pub unresolved: Vec<UnresolvedCell>,
    pub cells: usize,
    pub proven_cells: usize,
}
/// Prove regularity by separating at least one component of Su cross Sv from
/// zero. Failure is unresolved, never a declaration of a singularity.
/// `max_cells` counts all created rectangles, including the original knot grid.
pub fn inspect(surface: &Surface, max_cells: usize) -> Result<Report> {
    surface.validate()?;
    check(
        (1..=100000).contains(&max_cells),
        "Regularity needs 1..100000 cells",
    )?;
    let mut pending = Vec::new();
    for u in surface.degree_u..surface.control_points.len() {
        if surface.knots_u[u] == surface.knots_u[u + 1] {
            continue;
        }
        for v in surface.degree_v..surface.control_points[0].len() {
            if surface.knots_v[v] == surface.knots_v[v + 1] {
                continue;
            }
            pending.push((
                [u, v],
                [
                    [surface.knots_u[u], surface.knots_u[u + 1]],
                    [surface.knots_v[v], surface.knots_v[v + 1]],
                ],
                0usize,
            ));
        }
    }
    if pending.len() > max_cells {
        return Err(resource("Initial regularity grid exceeds the cell budget"));
    }
    let mut cells = pending.len();
    let mut proven_cells = 0;
    let mut unresolved = Vec::new();
    while let Some((span, domain, depth)) = pending.pop() {
        let j = jets::calculate(surface, span, domain, None)?;
        let u = j[1][0];
        let v = j[0][1];
        let mut separated = false;
        for k in 0..3 {
            let a = (k + 1) % 3;
            let b = (k + 2) % 3;
            let n = u[a].mul(v[b])?.sub(u[b].mul(v[a])?)?;
            separated |= n.lo > 0. || n.hi < 0.;
        }
        if separated {
            proven_cells += 1;
            continue;
        }
        let axis = depth % 2;
        let d = domain[axis];
        let m = d[0] * 0.5 + d[1] * 0.5;
        if cells + 2 > max_cells || !(d[0] < m && m < d[1]) {
            unresolved.push(UnresolvedCell { domain });
            continue;
        }
        for sub in [[d[0], m], [m, d[1]]] {
            let mut next = domain;
            next[axis] = sub;
            pending.push((span, next, depth + 1));
            cells += 1;
        }
    }
    let c1 = |degree: usize, knots: &[f64], count: usize| {
        let [a, b] = [knots[degree], knots[count]];
        knots
            .iter()
            .filter(|&&t| a < t && t < b)
            .all(|t| knots.iter().filter(|&k| k == t).count() < degree)
    };
    Ok(Report {
        spanwise_regular: unresolved.is_empty(),
        interior_basis_c1: c1(
            surface.degree_u,
            &surface.knots_u,
            surface.control_points.len(),
        ) && c1(
            surface.degree_v,
            &surface.knots_v,
            surface.control_points[0].len(),
        ),
        unresolved,
        cells,
        proven_cells,
    })
}
