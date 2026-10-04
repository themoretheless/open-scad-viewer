//! Bounded inter-patch separation evidence for retained sweep walls.
//! Shared boundaries require a boundary-only intersection certificate;
//! matching boundary data alone never excludes interior intersections. This never substitutes for self-patch
//! injectivity or shell containment.
use crate::{Result, check, surface::Surface};
use std::collections::BTreeSet;
#[derive(Clone, Debug)]
pub struct UnresolvedPair {
    pub patches: [usize; 2],
    pub reason: &'static str,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub all_pairs_separated: bool,
    pub all_pairs_compatible: bool,
    pub boundary_only_pairs: Vec<[usize; 2]>,
    pub separated_pairs: usize,
    pub pairs: usize,
    pub cells: usize,
    pub unresolved: Vec<UnresolvedPair>,
}
/// Matching represented endpoint curves on clamped tensor boundaries.
/// Tuple entries are (normal axis, endpoint index) for each chart.
/// Equality proves C0 only; positive-side separation is a separate obligation.
pub(crate) fn matching_boundaries(a: &Surface, b: &Surface) -> Vec<[(usize, usize); 2]> {
    fn endpoint(s: &Surface, axis: usize, end: usize) -> Option<usize> {
        let (degree, knots, count, periodic) = if axis == 0 {
            (s.degree_u, &s.knots_u, s.control_points.len(), s.periodic_u)
        } else {
            (s.degree_v, &s.knots_v, s.control_points[0].len(), s.periodic_v)
        };
        if periodic { return None; }
        let domain = crate::sweep_support::audit::natural_domain(knots, degree, count);
        let repeated = if end == 0 { &knots[..=degree] } else { &knots[count..] };
        if repeated.len() != degree + 1 || repeated.iter().any(|k| *k != domain[end]) {
            return None;
        }
        Some(if end == 0 { 0 } else { count - 1 })
    }
    let mut matches = Vec::new();
    for aa in 0..2 { for ba in 0..2 {
        let (ad, ak, ac, ap) = if aa == 0 {
            (a.degree_v, &a.knots_v, a.control_points[0].len(), a.periodic_v)
        } else { (a.degree_u, &a.knots_u, a.control_points.len(), a.periodic_u) };
        let (bd, bk, bc, bp) = if ba == 0 {
            (b.degree_v, &b.knots_v, b.control_points[0].len(), b.periodic_v)
        } else { (b.degree_u, &b.knots_u, b.control_points.len(), b.periodic_u) };
        if ad != bd || ak != bk || ac != bc || ap != bp { continue; }
        for ae in 0..2 { for be in 0..2 {
            let (Some(ai), Some(bi)) = (endpoint(a,aa,ae),endpoint(b,ba,be)) else { continue; };
            if (0..ac).all(|i| {
                let [au,av] = if aa == 0 { [ai,i] } else { [i,ai] };
                let [bu,bv] = if ba == 0 { [bi,i] } else { [i,bi] };
                a.control_points[au][av] == b.control_points[bu][bv]
                    && a.weights[au][av] == b.weights[bu][bv]
            }) { matches.push([(aa,ai),(ba,bi)]); }
        }}
    }}
    matches
}
/// A common coordinate plane and strictly opposite non-boundary control hulls
/// prove that only the complete shared boundary can intersect. Positive weights
/// and clamping make every interior basis combination leave the plane.
fn tensor_coordinate_boundary_only(a: &Surface, b: &Surface) -> bool {
    for [(aa,ai),(ba,bi)] in matching_boundaries(a,b) {
        for coordinate in 0..3 {
            let plane = if aa == 0 { a.control_points[ai][0][coordinate] }
                else { a.control_points[0][ai][coordinate] };
            let side = |s: &Surface, axis: usize, index: usize| -> Option<bool> {
                let mut below = true;
                let mut above = true;
                for (u,row) in s.control_points.iter().enumerate() {
                    for (v,point) in row.iter().enumerate() {
                        if [u,v][axis] == index {
                            if point[coordinate] != plane { return None; }
                        } else {
                            below &= point[coordinate] < plane;
                            above &= point[coordinate] > plane;
                        }
                    }
                }
                if below { Some(false) } else if above { Some(true) } else { None }
            };
            if let (Some(x),Some(y)) = (side(a,aa,ai),side(b,ba,bi)) {
                if x != y { return true; }
            }
        }
    }
    false
}
/// Sufficient exact-data certificate: equal rational boundary curves in a
/// coordinate plane, with all opposite control columns strictly on opposite
/// sides. Positive weights force every interior point off the plane.
fn coordinate_boundary_only(a: &Surface, b: &Surface) -> bool {
    if a.degree_v != 1
        || b.degree_v != 1
        || a.degree_u != b.degree_u
        || a.knots_u != b.knots_u
        || a.knots_v != vec![0., 0., 1., 1.]
        || b.knots_v != vec![0., 0., 1., 1.]
        || a.control_points.len() != b.control_points.len()
        || a.control_points
            .iter()
            .chain(&b.control_points)
            .any(|r| r.len() != 2)
    {
        return false;
    }
    for av in 0..2 {
        for bv in 0..2 {
            if !(0..a.control_points.len()).all(|i| {
                a.control_points[i][av] == b.control_points[i][bv]
                    && a.weights[i][av] == b.weights[i][bv]
            }) {
                continue;
            }
            for axis in 0..3 {
                let plane = a.control_points[0][av][axis];
                if !a.control_points.iter().all(|r| r[av][axis] == plane) {
                    continue;
                }
                let a_below = a.control_points.iter().all(|r| r[1 - av][axis] < plane);
                let a_above = a.control_points.iter().all(|r| r[1 - av][axis] > plane);
                let b_below = b.control_points.iter().all(|r| r[1 - bv][axis] < plane);
                let b_above = b.control_points.iter().all(|r| r[1 - bv][axis] > plane);
                if (a_below && b_above) || (a_above && b_below) {
                    return true;
                }
            }
        }
    }
    false
}
/// Exact dyadic orientation of represented control data. Checked i128
/// overflow yields no sign; it never falls back to rounded zero.
fn represented_orientation(a: &[f64], b: &[f64], c: &[f64], d: &[f64]) -> Option<i8> {
    fn dyadic(x: f64) -> (i128, i32) {
        let bits = x.to_bits();
        let e = ((bits >> 52) & 2047) as i32;
        let mut m = (bits & ((1_u64 << 52) - 1)) as i128;
        if e != 0 {
            m += 1_i128 << 52;
        }
        if m == 0 {
            return (0, 0);
        }
        let trailing = m.trailing_zeros();
        m >>= trailing;
        if bits >> 63 != 0 {
            m = -m;
        }
        (m, if e == 0 { -1074 } else { e - 1075 } + trailing as i32)
    }
    let values: Vec<_> = [a, b, c, d]
        .into_iter()
        .flat_map(|p| p.iter().copied())
        .map(dyadic)
        .collect();
    let exponent = values
        .iter()
        .filter(|(m, _)| *m != 0)
        .map(|(_, e)| *e)
        .min()
        .unwrap_or(0);
    let coordinates: Option<Vec<_>> = values
        .iter()
        .map(|(m, e)| {
            if *m == 0 {
                Some(0)
            } else {
                let shift = u32::try_from(*e - exponent).ok()?;
                if shift >= 127 {
                    return None;
                }
                m.checked_mul(1_i128.checked_shl(shift)?)
            }
        })
        .collect();
    let v = coordinates?;
    let difference = |i: usize| -> Option<[i128; 3]> {
        Some([
            v[i].checked_sub(v[0])?,
            v[i + 1].checked_sub(v[1])?,
            v[i + 2].checked_sub(v[2])?,
        ])
    };
    let u = difference(3)?;
    let w = difference(6)?;
    let z = difference(9)?;
    let cross = |i: usize, j: usize| w[i].checked_mul(z[j])?.checked_sub(w[j].checked_mul(z[i])?);
    let det = u[0]
        .checked_mul(cross(1, 2)?)?
        .checked_sub(u[1].checked_mul(cross(0, 2)?)?)?
        .checked_add(u[2].checked_mul(cross(0, 1)?)?)?;
    Some(det.signum() as i8)
}
/// Checked dyadic signs validate candidate separating planes against every
/// represented tensor control. Candidate construction is never evidence.
fn tensor_planar_boundary_only(a: &Surface, b: &Surface) -> bool {
    for [(aa,ai),(ba,bi)] in matching_boundaries(a,b) {
        let boundary: Vec<_> = if aa == 0 {
            a.control_points[ai].iter().collect()
        } else { a.control_points.iter().map(|row| &row[ai]).collect() };
        if !(2..=64).contains(&boundary.len()) { continue; }
        let origin = boundary[0];
        let mut candidates: Vec<(Vec<f64>,Vec<f64>)> = boundary.windows(2)
            .skip(1).map(|pair| (pair[0].clone(),pair[1].clone())).collect();
        for x in -1..=1 { for y in -1..=1 { for z in -1..=1 {
            if [x,y,z] == [0,0,0] { continue; }
            let q: Vec<_> = origin.iter().zip([x,y,z]).map(|(v,d)| *v + f64::from(d)).collect();
            if q.iter().all(|v| v.is_finite()) {
                candidates.push((boundary[1].clone(),q));
            }
        }}}
        let outside = |s: &Surface, axis: usize, index: usize| -> Option<Vec<f64>> {
            s.control_points.iter().enumerate().find_map(|(u,row)|
                row.iter().enumerate().find_map(|(v,p)|
                    ([u,v][axis] != index).then(|| p.clone())))
        };
        if let (Some(p),Some(q)) = (outside(a,aa,ai),outside(b,ba,bi)) {
            let auxiliary: Vec<_> = (0..3).map(|i| p[i]+q[i]-origin[i]).collect();
            if auxiliary.iter().all(|v| v.is_finite()) {
                candidates.push((boundary[1].clone(),auxiliary));
            }
        }
        for (p,q) in candidates {
            let side = |s: &Surface, axis: usize, index: usize| -> Option<i8> {
                let mut sign = None;
                for (u,row) in s.control_points.iter().enumerate() {
                    for (v,point) in row.iter().enumerate() {
                        let current = represented_orientation(origin,&p,&q,point)?;
                        if [u,v][axis] == index {
                            if current != 0 { return None; }
                        } else {
                            if current == 0 || sign.is_some_and(|value| value != current) { return None; }
                            sign = Some(current);
                        }
                    }
                }
                sign
            };
            if let (Some(x),Some(y)) = (side(a,aa,ai),side(b,ba,bi)) {
                if x == -y { return true; }
            }
        }
    }
    false
}
fn planar_boundary_only(a: &Surface, b: &Surface) -> bool {
    if a.degree_v != 1
        || b.degree_v != 1
        || a.degree_u != b.degree_u
        || a.knots_u != b.knots_u
        || a.knots_v != vec![0., 0., 1., 1.]
        || b.knots_v != vec![0., 0., 1., 1.]
        || a.control_points.len() != b.control_points.len()
        || !(2..=64).contains(&a.control_points.len())
        || a.control_points
            .iter()
            .chain(&b.control_points)
            .any(|r| r.len() != 2)
    {
        return false;
    }
    for av in 0..2 {
        for bv in 0..2 {
            if !(0..a.control_points.len()).all(|i| {
                a.control_points[i][av] == b.control_points[i][bv]
                    && a.weights[i][av] == b.weights[i][bv]
            }) {
                continue;
            }
            let origin = &a.control_points[0][av];
            let mut candidates: Vec<_> = (1..a.control_points.len() - 1)
                .map(|i| {
                    (
                        a.control_points[i][av].clone(),
                        a.control_points[i + 1][av].clone(),
                    )
                })
                .collect();
            // Auxiliary represented points only propose a plane. Exact signs
            // below validate every boundary and interior control against it.
            for axis in 0..3 {
                let mut q = origin.clone();
                q[axis] += 1.;
                if q.iter().all(|x| x.is_finite()) {
                    candidates.push((a.control_points[1][av].clone(), q));
                }
            }
            for (p, q) in &candidates {
                let Some(side) =
                    represented_orientation(origin, p, q, &a.control_points[0][1 - av])
                else {
                    continue;
                };
                if side == 0 {
                    continue;
                }
                if a.control_points.iter().all(|r| {
                    represented_orientation(origin, p, q, &r[av]) == Some(0)
                        && represented_orientation(origin, p, q, &r[1 - av]) == Some(side)
                }) && b
                    .control_points
                    .iter()
                    .all(|r| represented_orientation(origin, p, q, &r[1 - bv]) == Some(-side))
                {
                    return true;
                }
            }
        }
    }
    false
}
struct Node {
    ids: Vec<usize>,
    bounds: Vec<[f64; 2]>,
    children: Option<[usize; 2]>,
}
fn build(mut ids: Vec<usize>, boxes: &[Vec<[f64; 2]>], tree: &mut Vec<Node>) -> usize {
    let mut bounds = vec![[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for id in &ids {
        for k in 0..3 {
            bounds[k][0] = bounds[k][0].min(boxes[*id][k][0]);
            bounds[k][1] = bounds[k][1].max(boxes[*id][k][1]);
        }
    }
    let children = if ids.len() > 1 {
        let axis = (0..3)
            .max_by(|a, b| {
                (bounds[*a][1] - bounds[*a][0]).total_cmp(&(bounds[*b][1] - bounds[*b][0]))
            })
            .unwrap();
        // Sorting is only a scheduling heuristic, never a geometric proof.
        ids.sort_by(|a, b| {
            (boxes[*a][axis][0] * 0.5 + boxes[*a][axis][1] * 0.5)
                .total_cmp(&(boxes[*b][axis][0] * 0.5 + boxes[*b][axis][1] * 0.5))
                .then(a.cmp(b))
        });
        let mid = ids.len() / 2;
        Some([
            build(ids[..mid].to_vec(), boxes, tree),
            build(ids[mid..].to_vec(), boxes, tree),
        ])
    } else {
        None
    };
    let index = tree.len();
    tree.push(Node {
        ids,
        bounds,
        children,
    });
    index
}
fn retain_unresolved(out: &mut Report, a: &Node, b: &Node, same: bool, reason: &'static str) {
    for (i, x) in a.ids.iter().enumerate() {
        for (j, y) in b.ids.iter().enumerate() {
            if same && i >= j {
                continue;
            }
            out.unresolved.push(UnresolvedPair {
                patches: [(*x).min(*y), (*x).max(*y)],
                reason,
            });
        }
    }
}
/// Covers every distinct patch pair. Positive clearance is certified by a
/// convex-hull box first, otherwise the existing original-domain rational
/// distance hierarchy refines the pair. Exhaustion never discards a pair.
/// max_pairs counts outer BVH node-pair visits, including internal nodes.
pub fn inspect(
    patches: &[Surface],
    shared_boundaries: &[[usize; 2]],
    clearance: f64,
    distance_tolerance: f64,
    max_pairs: usize,
    max_cells: usize,
) -> Result<Report> {
    check(
        (1..=1024).contains(&patches.len()) && max_pairs <= 100000 && max_cells <= 100000,
        "Invalid sweep pair audit size/budget",
    )?;
    check(
        clearance.is_finite()
            && clearance >= 0.
            && distance_tolerance.is_finite()
            && distance_tolerance > 0.,
        "Invalid sweep pair audit tolerance",
    )?;
    let mut shared = BTreeSet::new();
    for pair in shared_boundaries {
        check(
            pair[0] < pair[1] && pair[1] < patches.len(),
            "Invalid shared boundary pair",
        )?;
        shared.insert(*pair);
    }
    let mut boxes = Vec::new();
    for patch in patches {
        patch.validate()?;
        check(
            patch.control_points.iter().flatten().all(|p| p.len() == 3),
            "Sweep walls must be 3D",
        )?;
        let mut bounds = vec![[f64::INFINITY, f64::NEG_INFINITY]; 3];
        for p in patch.control_points.iter().flatten() {
            for k in 0..3 {
                bounds[k][0] = bounds[k][0].min(p[k]);
                bounds[k][1] = bounds[k][1].max(p[k]);
            }
        }
        boxes.push(bounds);
    }
    let mut out = Report {
        all_pairs_separated: false,
        all_pairs_compatible: false,
        boundary_only_pairs: Vec::new(),
        separated_pairs: 0,
        pairs: 0,
        cells: 0,
        unresolved: Vec::new(),
    };
    let mut tree = Vec::new();
    let root = build((0..patches.len()).collect(), &boxes, &mut tree);
    let mut pending = vec![(root, root)];
    while let Some((ai, bi)) = pending.pop() {
        let a = &tree[ai];
        let b = &tree[bi];
        if ai == bi && a.children.is_none() {
            continue;
        }
        if out.pairs == max_pairs {
            retain_unresolved(&mut out, a, b, ai == bi, "pair-budget-exhausted");
            continue;
        }
        out.pairs += 1;
        let touches_shared = shared.iter().any(|p| {
            (a.ids.contains(&p[0]) && b.ids.contains(&p[1]))
                || (a.ids.contains(&p[1]) && b.ids.contains(&p[0]))
        });
        if ai != bi
            && !touches_shared
            && crate::surface_distance::enclosure_distance(&a.bounds, &b.bounds)?.0 > clearance
        {
            out.separated_pairs += a.ids.len() * b.ids.len();
            continue;
        }
        if ai == bi {
            let [left, right] = a.children.unwrap();
            pending.extend([(left, left), (left, right), (right, right)]);
            continue;
        }
        if a.children.is_some() || b.children.is_some() {
            if a.children.is_some() && (b.children.is_none() || a.ids.len() >= b.ids.len()) {
                let [left, right] = a.children.unwrap();
                pending.extend([(left, bi), (right, bi)]);
            } else {
                let [left, right] = b.children.unwrap();
                pending.extend([(ai, left), (ai, right)]);
            }
            continue;
        }
        let x = a.ids[0];
        let y = b.ids[0];
        let pair = [x.min(y), x.max(y)];
        let reason = if shared.contains(&pair) {
            if tensor_coordinate_boundary_only(&patches[x], &patches[y])
                || tensor_planar_boundary_only(&patches[x], &patches[y])
                || coordinate_boundary_only(&patches[x], &patches[y])
                || planar_boundary_only(&patches[x], &patches[y])
            {
                out.boundary_only_pairs.push(pair);
                None
            } else {
                Some("shared-boundary-interior-separation-unproved")
            }
        } else if out.cells == max_cells {
            Some("cell-budget-exhausted")
        } else {
            match crate::surface_distance::distance(
                &patches[x],
                &patches[y],
                distance_tolerance,
                max_cells - out.cells,
            ) {
                Ok(r) => {
                    out.cells += r.cells;
                    if r.distance_interval_mm[0] > clearance {
                        out.separated_pairs += 1;
                        None
                    } else {
                        Some("positive-separation-unproved")
                    }
                }
                Err(_) => {
                    out.cells = max_cells;
                    Some("numeric-or-initial-grid-unresolved")
                }
            }
        };
        if let Some(reason) = reason {
            out.unresolved.push(UnresolvedPair {
                patches: pair,
                reason,
            });
        }
    }
    out.all_pairs_compatible = out.unresolved.is_empty();
    out.all_pairs_separated = out.all_pairs_compatible && out.boundary_only_pairs.is_empty();
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn plane(z: f64) -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., z], vec![0., 1., z]],
                vec![vec![1., 0., z], vec![1., 1., z]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn every_pair_remains_covered_and_shared_boundaries_are_not_skipped() {
        let patches = [plane(0.), plane(1.), plane(2.)];
        let r = inspect(&patches, &[], 0., 1e-6, 100, 0).unwrap();
        assert!(r.all_pairs_separated && r.separated_pairs == 3 && r.cells == 0);
        let r = inspect(&patches, &[], 0., 1e-6, 1, 0).unwrap();
        assert!(!r.all_pairs_separated && r.unresolved.len() == 3);
        let r = inspect(&patches, &[[0, 1]], 0., 1e-6, 100, 10).unwrap();
        assert!(!r.all_pairs_separated && r.unresolved[0].patches == [0, 1]);
        let r = inspect(&[plane(0.), plane(0.)], &[], 0., 1e-6, 100, 31).unwrap();
        assert!(!r.all_pairs_separated && r.unresolved.len() == 1);
    }
    #[test]
    fn hierarchy_covers_each_leaf_pair_once_under_every_budget() {
        let patches: Vec<_> = (0..16).map(|i| plane(i as f64)).collect();
        for budget in [0, 1, 7, 31, 1000] {
            let r = inspect(&patches, &[], 0., 1e-6, budget, 0).unwrap();
            let unresolved: BTreeSet<_> = r.unresolved.iter().map(|p| p.patches).collect();
            assert_eq!(unresolved.len(), r.unresolved.len());
            assert_eq!(r.separated_pairs + unresolved.len(), 120);
            assert!(r.pairs <= budget);
            if budget == 1000 {
                assert!(r.all_pairs_separated && r.pairs < 120);
            }
        }
    }
    #[test]
    fn overlapping_boxes_use_rational_pair_refinement() {
        let mut a = plane(0.);
        for row in &mut a.control_points {
            for p in row {
                p[2] = p[0];
            }
        }
        let mut b = a.clone();
        for row in &mut b.control_points {
            for p in row {
                p[2] += 0.5;
            }
        }
        let r = inspect(&[a, b], &[], 0., 0.01, 100, 4095).unwrap();
        assert!(r.all_pairs_separated, "{r:?}");
        assert!(r.cells > 0 && r.cells <= 4095);
    }
    #[test]
    fn rational_tensor_profile_boundaries_require_opposite_interior_hulls() {
        let patch = |xs: [f64; 3]| Surface {
            degree_u: 2, degree_v: 2,
            knots_u: vec![0.,0.,0.,1.,1.,1.],
            knots_v: vec![0.,0.,0.,1.,1.,1.],
            control_points: xs.iter().map(|x| [0.,0.5,1.].iter()
                .map(|y| vec![*x,*y,0.]).collect()).collect(),
            weights: vec![vec![1.,2.,1.];3], periodic_u: false, periodic_v: false,
        };
        let a = patch([-1.,-0.5,0.]);
        let b = patch([0.,0.5,1.]);
        assert!(!coordinate_boundary_only(&a,&b));
        let r = inspect(&[a.clone(),b.clone()], &[[0,1]], 0., 1e-6, 100, 0).unwrap();
        assert!(r.all_pairs_compatible);
        assert_eq!(r.boundary_only_pairs,vec![[0,1]]);
        // A parameter-axis swap preserves the same geometric shared curve.
        let mut transposed = b.clone();
        for u in 0..3 { for v in 0..3 {
            transposed.control_points[u][v] = b.control_points[v][u].clone();
            transposed.weights[u][v] = b.weights[v][u];
        }}
        assert!(inspect(&[a.clone(),transposed],&[[0,1]],0.,1e-6,100,0)
            .unwrap().all_pairs_compatible);
        let mut corner_a = a.clone();
        let mut corner_b = b.clone();
        for row in &mut corner_a.control_points { for point in row {
            *point = vec![point[0],0.,point[1]];
        }}
        for row in &mut corner_b.control_points { for point in row {
            *point = vec![0.,point[0],point[1]];
        }}
        assert!(!tensor_coordinate_boundary_only(&corner_a,&corner_b));
        assert!(tensor_planar_boundary_only(&corner_a,&corner_b));
        assert!(inspect(&[corner_a.clone(),corner_b.clone()],&[[0,1]],0.,1e-6,100,0)
            .unwrap().all_pairs_compatible);
        // Every candidate remains a heuristic: a non-boundary control on the
        // opposite chart destroys strict separation and cannot be ignored.
        corner_b.control_points[2][1] = corner_a.control_points[0][1].clone();
        assert!(!tensor_planar_boundary_only(&corner_a,&corner_b));
        let folded = patch([0.,-0.5,-1.]);
        assert!(!inspect(&[a.clone(),folded],&[[0,1]],0.,1e-6,100,0)
            .unwrap().all_pairs_compatible);
        let mut changed = b.clone();
        changed.weights[0][1] = 3.;
        assert!(matching_boundaries(&a,&changed).is_empty());
        let mut unclamped = b;
        unclamped.knots_u = vec![-2.,-1.,0.,1.,2.,3.];
        assert!(matching_boundaries(&a,&unclamped).is_empty());
    }
    #[test]
    fn exact_shared_edge_is_allowed_only_with_proved_opposite_interiors() {
        let mut a = plane(0.);
        for row in &mut a.control_points {
            row[1][2] = 1.;
        }
        let mut b = a.clone();
        for row in &mut b.control_points {
            row[0][1] = 1.;
            row[0][2] = 1.;
            row[1][1] = 2.;
            row[1][2] = 2.;
        }
        let r = inspect(&[a.clone(), b.clone()], &[[0, 1]], 0., 1e-6, 100, 31).unwrap();
        assert!(r.all_pairs_compatible && !r.all_pairs_separated);
        assert_eq!(r.boundary_only_pairs, vec![[0, 1]]);
        // Fold back onto the first patch: the matching edge alone is insufficient.
        for (row, original) in b.control_points.iter_mut().zip(&a.control_points) {
            row[1] = original[0].clone();
        }
        let r = inspect(&[a, b], &[[0, 1]], 0., 1e-6, 100, 31).unwrap();
        assert!(!r.all_pairs_compatible && r.boundary_only_pairs.is_empty());
    }
    #[test]
    fn oblique_shared_plane_uses_checked_exact_orientation() {
        let boundary = vec![vec![0., 0., 0.], vec![1., 0., 1.], vec![0., 1., 1.]];
        let wall = |sign: f64| Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: boundary
                .iter()
                .map(|p| vec![p.clone(), vec![p[0], p[1], p[2] + sign]])
                .collect(),
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        let a = wall(-1.);
        let b = wall(1.);
        assert!(!coordinate_boundary_only(&a, &b));
        assert!(planar_boundary_only(&a, &b));
        let r = inspect(&[a.clone(), b], &[[0, 1]], 0., 1e-6, 100, 0).unwrap();
        assert!(r.all_pairs_compatible && r.boundary_only_pairs.len() == 1);
        assert!(!planar_boundary_only(&a, &wall(-1.)));
        assert_eq!(
            represented_orientation(
                &[0., 0., 0.],
                &[2_f64.powi(127), 0., 0.],
                &[0., 1., 0.],
                &[0., 0., 1.]
            ),
            None
        );
        assert_eq!(
            represented_orientation(
                &[0., 0., 0.],
                &[1e100, 0., 0.],
                &[0., 1e100, 0.],
                &[0., 0., 1e-100]
            ),
            None
        );
    }
    #[test]
    fn linear_oblique_boundary_requires_opposite_interiors() {
        let wall = |offset: f64| Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: [vec![0., 0., 0.], vec![1., 1., 1.]]
                .iter()
                .map(|p| vec![p.clone(), vec![p[0] + offset, p[1] - offset, p[2]]])
                .collect(),
            weights: vec![vec![1., 1.], vec![2., 2.]],
            periodic_u: false,
            periodic_v: false,
        };
        let a = wall(-1.);
        let b = wall(1.);
        assert!(!coordinate_boundary_only(&a, &b));
        assert!(planar_boundary_only(&a, &b));
        let r = inspect(&[a.clone(), b], &[[0, 1]], 0., 1e-6, 100, 0).unwrap();
        assert!(r.all_pairs_compatible);
        assert_eq!(r.boundary_only_pairs, vec![[0, 1]]);
        assert!(!planar_boundary_only(&a, &wall(-1.)));
        assert!(!planar_boundary_only(&a, &wall(0.)));
    }
}
