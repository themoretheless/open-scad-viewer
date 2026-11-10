//! Exact planar hypotheses for ambient isotopies of signed prismatic solids.
//! No 3D topology is inferred from sampled genus or an inclusion sandwich.
use super::*;
use offset_regions::arrangement::{self, Point, Rings};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) type Polygon = Vec<Point>;

fn forest(rings: &Rings) -> Result<Vec<Option<usize>>> {
    let mut contains = vec![vec![false; rings.len()]; rings.len()];
    for (i, r) in rings.iter().enumerate() {
        for (j, s) in rings.iter().enumerate() {
            if i != j {
                contains[i][j] = polygon_winding(&s.iter().collect::<Vec<_>>(), &r[0])? != 0;
            }
        }
    }
    let depth = contains
        .iter()
        .map(|r| r.iter().filter(|b| **b).count())
        .collect::<Vec<_>>();
    (0..rings.len())
        .map(|i| {
            let mut parents =
                (0..rings.len()).filter(|&j| contains[i][j] && depth[j] + 1 == depth[i]);
            let p = parents.next();
            if parents.next().is_some() || (depth[i] > 0 && p.is_none()) {
                return Err(refused("Prismatic planar containment is not a forest"));
            }
            Ok(p)
        })
        .collect()
}

/// A simultaneous exact profile certificate proves the entire original family
/// is Jordan and disjoint. Its checked polygon homotopy preserves *all* pair
/// containments, not only each wire's nesting depth.
pub(crate) fn original_forest(loops: &[Vec<Curve>]) -> Result<Vec<Option<usize>>> {
    let c = producer::certify_geometry(loops, 1e-7)?;
    verify_geometry(loops, 1e-7, &c)?;
    let p = pieces(loops, c.subdivision_depth)?;
    let rings = (0..loops.len())
        .map(|w| {
            p.iter()
                .filter(|p| p.wire == w)
                .map(|p| p.points[0].clone())
                .collect()
        })
        .collect();
    forest(&rings)
}
pub(crate) fn original_wires(loops: &[Vec<Curve>]) -> Result<(Vec<i8>, Vec<usize>)> {
    let c = producer::certify_geometry(loops, 1e-7)?;
    let (v, orientation) = verify_geometry(loops, 1e-7, &c)?;
    Ok((orientation, v.nesting_depth().to_vec()))
}

pub(crate) fn polygon_forest(rings: &Rings) -> Result<Vec<Option<usize>>> {
    if rings.is_empty() || rings.len() > 32 || rings.iter().map(Vec::len).sum::<usize>() > 1024 {
        return Err(refused("Prismatic planar family exceeds its size budget"));
    }
    let mut points = BTreeSet::new();
    for r in rings {
        if r.len() < 3
            || arrangement::area(r).is_zero()
            || r.iter().any(|p| !points.insert(p.clone()))
        {
            return Err(refused("Prismatic planar boundary collapses or touches"));
        }
    }
    // With globally unique vertices, the only permitted common endpoints are
    // consecutive edges of the same wire. Crossings and overlaps refuse.
    arrangement::check_rounding(rings, rings)?;
    forest(rings)
}

pub(crate) fn polygon_union(rings: &Rings) -> Result<Rings> {
    arrangement::union(rings)
}
/// Canonical 3D boundary triangulations already split every shared edge.
/// Cancel those exact simplices before asking any planar topology question;
/// a second all-pairs arrangement of hundreds of coplanar triangles is neither
/// needed nor evidence about the source surface.
pub(crate) fn triangle_boundary(triangles: &Rings) -> Result<Rings> {
    let mut edges = BTreeMap::<(Point, Point), i32>::new();
    for t in triangles {
        if t.len() != 3 || arrangement::area(t) <= Q::zero() {
            return Err(refused("Prismatic top simplex orientation is false"));
        }
        for i in 0..3 {
            let (a, b) = (t[i].clone(), t[(i + 1) % 3].clone());
            let (key, sign) = if a < b { ((a, b), 1) } else { ((b, a), -1) };
            *edges.entry(key).or_default() += sign;
        }
    }
    let mut next = BTreeMap::new();
    let mut incoming = BTreeSet::new();
    for ((a, b), n) in edges {
        if n == 0 {
            continue;
        }
        if n.abs() != 1 {
            return Err(refused("Prismatic cap has repeated boundary simplices"));
        }
        let (a, b) = if n > 0 { (a, b) } else { (b, a) };
        if !incoming.insert(b.clone()) || next.insert(a, b).is_some() {
            return Err(refused(
                "Prismatic cap boundary is not a disjoint cycle family",
            ));
        }
    }
    let mut rings = Vec::new();
    while let Some(start) = next.keys().next().cloned() {
        let mut ring = Vec::new();
        let mut p = start.clone();
        loop {
            ring.push(p.clone());
            p = next
                .remove(&p)
                .ok_or_else(|| refused("Prismatic cap has an open boundary"))?;
            if p == start {
                break;
            }
        }
        rings.push(ring);
    }
    polygon_forest(&rings)?;
    Ok(rings)
}
pub(crate) fn groups(rings: &Rings) -> Result<Vec<usize>> {
    polygon_forest(rings)?;
    let mut n = arrangement::groups(rings)?
        .iter()
        .map(|g| g.len() - 1)
        .collect::<Vec<_>>();
    n.sort();
    Ok(n)
}
pub(crate) fn area(r: &[Point]) -> Q {
    arrangement::area(r)
}
pub(crate) fn hull(p: Polygon) -> Polygon {
    minkowski::topology_hull(p)
}
pub(crate) fn erosion(a: &Polygon, kernel: &Polygon) -> Result<Polygon> {
    minkowski::topology_erode(a, kernel)
}

fn regular(net: &Vec<[Q; 3]>, depth: u8) -> bool {
    let t = arithmetic::tangent_numerators(net);
    if (0..2).all(|k| t[k][0].is_zero()) || (0..2).all(|k| t[k].last().unwrap().is_zero()) {
        return false;
    }
    if (0..2).any(|k| arithmetic::no_stationary(net, k)) {
        return true;
    }
    if depth == 6 {
        return false;
    }
    let (a, b) = split(net);
    regular(&a, depth + 1) && regular(&b, depth + 1)
}

/// A simple regular closed curve with nonnegative signed curvature and
/// nonnegative ordinary corner turns bounds a convex region. All polynomial
/// sign checks cover complete original rational spans, including flat pieces.
pub(crate) fn convex_wire(wire: &[Curve]) -> Result<()> {
    use arithmetic::{derivative, power, product, sub};
    let loops = vec![wire.to_vec()];
    let c = producer::certify_geometry(&loops, 1e-7)?;
    verify_geometry(&loops, 1e-7, &c)?;
    let orientation = Q::from_integer(c.orientation[0].into());
    let nets = extraction::nets(&loops)?
        .into_iter()
        .map(|(_, n)| n)
        .collect::<Vec<_>>();
    for n in &nets {
        if !regular(n, 0) {
            return Err(refused("Prismatic convex curve tangent is unproved"));
        }
        let w = power(n, 2);
        let d: Vec<_> = (0..2)
            .map(|k| {
                let p = power(n, k);
                sub(&product(&derivative(&p), &w), &product(&p, &derivative(&w)))
            })
            .collect();
        let k = sub(
            &product(&d[0], &derivative(&d[1])),
            &product(&d[1], &derivative(&d[0])),
        )
        .into_iter()
        .map(|q| q * &orientation)
        .collect::<Vec<_>>();
        if !exact_polynomial_nonnegative(&k)? {
            return Err(refused("Prismatic source boundary is not convex"));
        }
    }
    for i in 0..nets.len() {
        let a = arithmetic::tangent_numerators(&nets[i]);
        let b = arithmetic::tangent_numerators(&nets[(i + 1) % nets.len()]);
        let turn =
            (a[0].last().unwrap() * &b[1][0] - a[1].last().unwrap() * &b[0][0]) * &orientation;
        let dot = a[0].last().unwrap() * &b[0][0] + a[1].last().unwrap() * &b[1][0];
        if turn < Q::zero() || (turn.is_zero() && dot <= Q::zero()) {
            return Err(refused("Prismatic convex boundary corner is unproved"));
        }
    }
    Ok(())
}

/// Boundary samples lie in a convex filled source, and positive rational
/// Bernstein hulls contain its entire boundary. Convex hulls give independent
/// inner/outer bounds without fitting a circle or rounding a source point.
pub(crate) fn convex_bounds(wire: &[Curve], depth: u8) -> Result<(Polygon, Polygon)> {
    if depth > 6 {
        return Err(refused("Prismatic convex bound depth exceeded"));
    }
    let mut nets = extraction::nets(&[wire.to_vec()])?
        .into_iter()
        .map(|(_, n)| n)
        .collect::<Vec<_>>();
    for _ in 0..depth {
        if nets.len() > 256 {
            return Err(refused("Prismatic convex hull census exceeded"));
        }
        nets = nets
            .into_iter()
            .flat_map(|n| {
                let (a, b) = split(&n);
                [a, b]
            })
            .collect();
    }
    let point = |h: &[Q; 3]| [&h[0] / &h[2], &h[1] / &h[2]];
    let inner = hull(
        nets.iter()
            .flat_map(|n| [point(&n[0]), point(n.last().unwrap())])
            .collect(),
    );
    let outer = hull(nets.iter().flatten().map(point).collect());
    if inner.len() < 3 || outer.len() < 3 {
        return Err(refused("Prismatic convex bound has no interior"));
    }
    Ok((inner, outer))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rect(x: i64, y: i64, w: i64, h: i64) -> Polygon {
        [[x, y], [x + w, y], [x + w, y + h], [x, y + h]]
            .map(|p| p.map(|q| Q::from_integer(q.into())))
            .to_vec()
    }
    #[test]
    fn full_labeled_parent_forest_detects_reparenting_at_the_same_depth() {
        let a = vec![rect(0, 0, 4, 4), rect(8, 0, 4, 4), rect(1, 1, 1, 1)];
        let b = vec![a[0].clone(), a[1].clone(), rect(9, 1, 1, 1)];
        assert_eq!(polygon_forest(&a).unwrap(), vec![None, None, Some(0)]);
        assert_eq!(polygon_forest(&b).unwrap(), vec![None, None, Some(1)]);
    }
    #[test]
    fn touching_nested_profiles_and_crossing_edges_are_not_jordan_families() {
        assert!(polygon_forest(&vec![rect(0, 0, 4, 4), rect(0, 1, 1, 1)]).is_err());
        assert!(polygon_forest(&vec![rect(0, 0, 4, 4), rect(3, 1, 2, 1)]).is_err());
    }
}
