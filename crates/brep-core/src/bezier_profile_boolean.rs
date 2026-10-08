//! Native transverse polynomial profile Boolean. Trims retain source controls;
//! only shared endpoint roundoff is conformed within the requested tolerance.
use super::{Result, controls, point_inside, refused};
use crate::Curve;
use nurbs_core::intersection::{
    ContactClass, CurveCurveComponentKind, intersect_curve_curve_report,
};
fn material(op: &str, a: bool, b: bool) -> bool {
    match op {
        "union" => a || b,
        "intersection" => a && b,
        "difference" => a && !b,
        "xor" => a != b,
        _ => false,
    }
}
pub(super) fn lift(c: &Curve) -> Curve {
    let mut c = c.clone();
    for p in &mut c.control_points {
        p.push(0.);
    }
    c
}
fn same(a: &Curve, b: &Curve) -> Option<bool> {
    if a.degree != b.degree || a.weights != b.weights {
        return None;
    }
    if a.control_points == b.control_points {
        return Some(true);
    }
    if a.control_points.iter().eq(b.control_points.iter().rev()) {
        return Some(false);
    }
    None
}
fn contains(loops: &[Vec<Curve>], p: [f64; 2], tol: f64) -> Result<bool> {
    let mut inside = false;
    for wire in loops {
        inside ^= point_inside(wire, p, tol)?;
    }
    Ok(inside)
}
pub fn boolean(
    a: &[Vec<Curve>],
    b: &[Vec<Curve>],
    op: &str,
    tolerance: f64,
) -> Result<Vec<Vec<Curve>>> {
    if !matches!(op, "union" | "intersection" | "difference" | "xor") {
        return Err(refused("Invalid Bézier Boolean operation"));
    }
    let a = super::orient_even_odd(a, tolerance)?;
    let b = super::orient_even_odd(b, tolerance)?;
    let av: Vec<_> = a.iter().flatten().cloned().collect();
    let bv: Vec<_> = b.iter().flatten().cloned().collect();
    if av.len() + bv.len() > 1022 {
        return Err(refused("Bézier Boolean exceeds 1022 source spans"));
    }
    let mut cuts: Vec<Vec<f64>> = av.iter().chain(&bv).map(|c| c.domain().to_vec()).collect();
    let mut aliases: Vec<usize> = (0..cuts.len()).collect();
    let mut shared = Vec::new();
    let mut work = 0;
    for (i, ca) in av.iter().enumerate() {
        for (j, cb) in bv.iter().enumerate() {
            if let Some(forward) = same(ca, cb) {
                let old = aliases[av.len() + j];
                let new = aliases[i];
                for alias in &mut aliases {
                    if *alias == old {
                        *alias = new;
                    }
                }
                shared.push((i, j, forward, ca.domain(), cb.domain()));
                continue;
            }
            let pa = controls(ca)?;
            let pb = controls(cb)?;
            if (0..2).any(|k| {
                let min = |p: &Vec<[f64; 2]>| p.iter().map(|q| q[k]).fold(f64::INFINITY, f64::min);
                let max =
                    |p: &Vec<[f64; 2]>| p.iter().map(|q| q[k]).fold(f64::NEG_INFINITY, f64::max);
                max(&pa) + 8. * tolerance < min(&pb) || max(&pb) + 8. * tolerance < min(&pa)
            }) {
                continue;
            }
            // A supporting coordinate plane can contain endpoint tangencies.
            // Positive Bernstein weights and one strictly interior control on
            // each side prove that the open spans never reach that plane.
            if super::supporting_hulls_separate(&pa, &pb) {
                continue;
            }
            let shared_endpoint = [pa[0], *pa.last().unwrap()]
                .into_iter()
                .find(|p| *p == pb[0] || *p == *pb.last().unwrap());
            if shared_endpoint.is_some_and(|p| super::shared_hulls_separate(&pa, &pb, p)) {
                continue;
            }
            work += 1;
            if work > 8192 {
                return Err(refused("Bézier Boolean pair budget exceeded"));
            }
            let report = intersect_curve_curve_report(&lift(ca), &lift(cb), None)?;
            if !report.report.unresolved.is_empty() {
                return Err(refused("Bézier Boolean intersection is unresolved"));
            }
            for event in report.report.components {
                if event.kind == CurveCurveComponentKind::Overlap {
                    let x = ca.trim(event.first_interval[0], event.first_interval[1])?;
                    let y = cb.trim(event.second_interval[0], event.second_interval[1])?;
                    let forward = same(&x, &y)
                        .or_else(|| {
                            // Interval intersection proposes an overlap; retained
                            // polynomial Bernstein coefficients independently admit it.
                            if x.degree != y.degree || x.weights != y.weights {
                                return None;
                            }
                            for forward in [true, false] {
                                let yy = if forward {
                                    y.clone()
                                } else {
                                    y.reverse().ok()?
                                };
                                if x.control_points
                                    .iter()
                                    .zip(&yy.control_points)
                                    .all(|(p, q)| {
                                        p.iter().zip(q).all(|(a, b)| {
                                            (a - b).abs()
                                                <= 256.
                                                    * f64::EPSILON
                                                    * a.abs().max(b.abs()).max(1.)
                                        })
                                    })
                                {
                                    return Some(forward);
                                }
                            }
                            None
                        })
                        .ok_or_else(|| refused("Bézier overlap carrier identity is unresolved"))?;
                    cuts[i].extend(event.first_interval);
                    cuts[av.len() + j].extend(event.second_interval);
                    let old = aliases[av.len() + j];
                    let new = aliases[i];
                    for alias in &mut aliases {
                        if *alias == old {
                            *alias = new;
                        }
                    }
                    shared.push((i, j, forward, event.first_interval, event.second_interval));
                    continue;
                }
                if !matches!(
                    event.contact,
                    ContactClass::Transverse
                        | ContactClass::Boundary
                        | ContactClass::OddTangency
                        | ContactClass::EvenTangency
                ) {
                    return Err(refused("Bézier Boolean contact is unresolved"));
                }
                if event.residual > tolerance {
                    return Err(refused("Bézier Boolean event residual exceeds tolerance"));
                }
                cuts[i].push(event.first);
                cuts[av.len() + j].push(event.second);
            }
        }
    }
    let mut selected: Vec<(Curve, usize)> = Vec::new();
    for (index, source) in av.iter().chain(&bv).enumerate() {
        let from_a = index < av.len();
        let local = if from_a { index } else { index - av.len() };
        let other = if from_a { &b } else { &a };
        cuts[index].sort_by(f64::total_cmp);
        cuts[index].dedup_by(|a, b| (*a - *b).abs() < 1e-12);
        for interval in cuts[index].windows(2) {
            if interval[1] - interval[0] <= 1e-12 {
                continue;
            }
            let mut piece = source.trim(interval[0], interval[1])?;
            let p = piece.evaluate((interval[0] + interval[1]) * 0.5)?.point;
            let neighbor = shared.iter().find_map(|(i, j, same, ar, br)| {
                let range = if from_a { ar } else { br };
                let mid = (interval[0] + interval[1]) * 0.5;
                if ((from_a && *i == local) || (!from_a && *j == local))
                    && range[0] < mid
                    && mid < range[1]
                {
                    Some(*same)
                } else {
                    None
                }
            });
            let (ol, or) = if let Some(forward) = neighbor {
                (forward, !forward)
            } else {
                let inside = contains(other, [p[0], p[1]], tolerance)?;
                (inside, inside)
            };
            let left = if from_a {
                material(op, true, ol)
            } else {
                material(op, ol, true)
            };
            let right = if from_a {
                material(op, false, or)
            } else {
                material(op, or, false)
            };
            if left == right {
                continue;
            }
            if !left {
                piece = piece.reverse()?;
            }
            let alias = aliases[index];
            if selected.iter().any(|(c, id)| {
                c.degree == piece.degree
                    && c.control_points.len() == piece.control_points.len()
                    && c.weights.iter().zip(&piece.weights).all(|(x, y)| {
                        (x / c.weights[0] - y / piece.weights[0]).abs() <= 256. * f64::EPSILON
                    })
                    && c.control_points
                        .iter()
                        .zip(&piece.control_points)
                        .all(|(p, q)| {
                            p.iter().zip(q).all(|(x, y)| {
                                (x - y).abs()
                                    <= if *id == alias {
                                        8. * tolerance
                                    } else {
                                        256. * f64::EPSILON * x.abs().max(y.abs()).max(1.)
                                    }
                            })
                        })
            }) {
                continue;
            }
            selected.push((piece, alias));
            if selected.len() > 2048 {
                return Err(refused("Bézier Boolean fragment budget exceeded"));
            }
        }
    }
    assemble(selected, tolerance, op == "union")
}
pub(super) fn assemble(
    mut selected: Vec<(Curve, usize)>,
    tolerance: f64,
    contacts: bool,
) -> Result<Vec<Vec<Curve>>> {
    let mut vertices: Vec<Vec<f64>> = Vec::new();
    let mut links = Vec::new();
    for (curve, _) in &mut selected {
        let mut pair = [0usize; 2];
        let last = curve.control_points.len() - 1;
        for (side, index) in [0, last].into_iter().enumerate() {
            let p = &curve.control_points[index];
            let nearby: Vec<_> = vertices
                .iter()
                .enumerate()
                .filter(|(_, q)| (p[0] - q[0]).hypot(p[1] - q[1]) <= 8. * tolerance)
                .map(|(i, _)| i)
                .collect();
            if nearby.len() > 1 {
                return Err(refused("Bézier Boolean vertex clustering is ambiguous"));
            }
            pair[side] = if let Some(i) = nearby.first() {
                *i
            } else {
                vertices.push(p.clone());
                vertices.len() - 1
            };
            curve.control_points[index] = vertices[pair[side]].clone();
        }
        if pair[0] == pair[1] {
            return Err(refused(
                "Bézier Boolean fragment collapses at the vertex tolerance",
            ));
        }
        links.push(pair);
    }
    let mut outgoing = vec![Vec::new(); vertices.len()];
    let mut incoming = vec![0usize; vertices.len()];
    for (i, p) in links.iter().enumerate() {
        outgoing[p[0]].push(i);
        incoming[p[1]] += 1;
    }
    if incoming
        .iter()
        .zip(&outgoing)
        .any(|(n, out)| *n != out.len() || (!contacts && *n != 1))
    {
        return Err(refused("Bézier Boolean has a nonmanifold boundary contact"));
    }
    let mut next = vec![0usize; selected.len()];
    let mut predecessor = vec![0usize; selected.len()];
    for (i, link) in links.iter().enumerate() {
        let curve = &selected[i].0;
        let d = curve
            .evaluate(curve.domain()[1])?
            .d1
            .ok_or_else(|| refused("Boundary tangent is unresolved"))?;
        let back = (-d[1]).atan2(-d[0]);
        let mut candidates = Vec::new();
        for &j in &outgoing[link[1]] {
            let c = &selected[j].0;
            let d = c
                .evaluate(c.domain()[0])?
                .d1
                .ok_or_else(|| refused("Boundary tangent is unresolved"))?;
            if d[0].hypot(d[1]) <= tolerance {
                return Err(refused("Boundary cusp is unresolved"));
            }
            candidates.push((
                (back - d[1].atan2(d[0])).rem_euclid(std::f64::consts::TAU),
                j,
            ));
        }
        if contacts && candidates.len() > 1 {
            candidates.retain(|c| c.0 > 1e-10);
        }
        if candidates.is_empty() {
            return Err(refused(
                "Boundary has only zero-angle tangent continuations",
            ));
        }
        candidates.sort_by(|a, b| a.0.total_cmp(&b.0));
        if candidates.len() > 1 && (candidates[0].0 - candidates[1].0).abs() < 1e-10 {
            return Err(refused("Boundary tangent ordering is ambiguous"));
        }
        next[i] = candidates[0].1;
        predecessor[next[i]] += 1;
    }
    if predecessor.iter().any(|n| *n != 1) {
        return Err(refused("Boundary traversal is nonmanifold"));
    }
    let mut used = vec![false; selected.len()];
    let mut loops = Vec::new();
    for first in 0..selected.len() {
        if used[first] {
            continue;
        }
        let mut wire = Vec::new();
        let mut current = first;
        loop {
            if used[current] {
                if current != first {
                    return Err(refused("Bézier Boolean boundary traversal is ambiguous"));
                }
                break;
            }
            used[current] = true;
            wire.push(selected[current].0.clone());
            current = next[current];
        }
        loops.push(wire);
    }
    if !loops.is_empty() {
        super::classify_components(&loops, tolerance)?;
    }
    Ok(loops)
}
