//! Display-only diagonal selection for an already admitted planar face.
//! No vertex removal, snapping, carrier editing, or relaxed mesh inspection.
use super::FaceMesh;
use cad_predicates::{AuthoredScalar, Limits, Outcome, PredicateContext, Sign, SourceArena, ToleranceContext, orient2d};
use math_core::{cross, norm, sub};

fn degenerate(points: &[[f64; 3]], t: [usize; 3]) -> bool {
    let ab = sub(points[t[1]], points[t[0]]);
    let ac = sub(points[t[2]], points[t[0]]);
    // Exactly the existing MeshView::inspect acceptance criterion.
    norm(cross(ab, ac)) <= f64::EPSILON * (norm(ab) * norm(ac)).max(1.)
}

fn admissible_diagonal(uv: [[f64; 2]; 4]) -> bool {
    let Ok(source) = SourceArena::authored("display-planar-diagonal", 1,
        uv.into_iter().flatten().map(|v| AuthoredScalar::Binary64Bits(v.to_bits())).collect()) else { return false; };
    let tolerance = ToleranceContext::default_valid();
    let mut context = PredicateContext::new(&source, &tolerance, Limits::default(), None);
    let refs = std::array::from_fn::<_, 4, _>(|i| [source.leaf(2*i).unwrap(), source.leaf(2*i+1).unwrap()]);
    let mut sign = |t: [usize; 3]| orient2d(&mut context, refs[t[0]], refs[t[1]], refs[t[2]])
        .ok().and_then(|r| match r.outcome { Outcome::Sign(s) => Some(s), _ => None });
    let Some(winding) = sign([0, 1, 2]) else { return false; };
    if winding == Sign::Zero { return false; }
    // Both original and replacement pairs must have the same exact UV
    // winding. This proves a convex quadrilateral and the same covered region.
    [[1, 0, 3], [2, 3, 1], [3, 2, 0]].into_iter().all(|t| sign(t) == Some(winding))
}

pub(super) fn repair(mesh: &mut FaceMesh, points: &[[f64; 3]]) {
    loop {
        let mut replacement = None;
        'search: for (i, &t) in mesh.triangles.iter().enumerate() {
            if !degenerate(points, t) { continue; }
            for k in 0..3 {
                let [a, b, c] = [t[k], t[(k+1)%3], t[(k+2)%3]];
                for (j, &neighbor) in mesh.triangles.iter().enumerate() {
                    if i == j { continue; }
                    let Some(l) = (0..3).find(|&l| neighbor[l] == b && neighbor[(l+1)%3] == a) else { continue; };
                    let d = neighbor[(l+2)%3];
                    if d == c || mesh.triangles.iter().enumerate().any(|(n, v)| n != i && n != j
                        && (0..3).any(|l| (v[l] == c && v[(l+1)%3] == d) || (v[l] == d && v[(l+1)%3] == c))) { continue; }
                    let first = [c, d, b];
                    let second = [d, c, a];
                    if degenerate(points, first) || degenerate(points, second)
                        || !admissible_diagonal([mesh.uv[a], mesh.uv[b], mesh.uv[c], mesh.uv[d]]) { continue; }
                    replacement = Some((i, j, first, second));
                    break 'search;
                }
            }
        }
        let Some((i, j, first, second)) = replacement else { return; };
        mesh.triangles[i] = first;
        mesh.triangles[j] = second;
        // At least one degenerate triangle disappears and none is added.
        // Termination is bounded by the original triangle count.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    fn boundary(triangles: &[[usize; 3]]) -> BTreeMap<[usize; 2], i32> {
        let mut result = BTreeMap::new();
        for t in triangles { for k in 0..3 { let (a,b)=(t[k],t[(k+1)%3]);
            *result.entry([a.min(b), a.max(b)]).or_default() += if a < b {1} else {-1};
        }}
        result.retain(|_, n| *n != 0); result
    }
    #[test]
    fn represented_sliver_changes_only_the_interior_diagonal() {
        let uv = vec![[0.,0.9999999999999998], [4.347360804527367e-18,0.6666666666666671], [9.781561810186596e-18,0.2500000000000004], [1.,1.]];
        let points = [[-0.723230863929924,1.8646546912120687,0.],[-0.6026923866082701,1.553878909343391,0.],[-0.4520192899562025,1.165409182007543,0.],[-0.7232308639299239,1.8646546912120687,2.]];
        let mut mesh = FaceMesh {uv, shared: vec![Some(26),Some(28),Some(30),Some(69)], triangles: vec![[0,1,2],[2,3,0]]};
        assert!(degenerate(&points, mesh.triangles[0]));
        let before = boundary(&mesh.triangles); let uv = mesh.uv.clone(); let shared = mesh.shared.clone();
        repair(&mut mesh, &points);
        assert_eq!(mesh.triangles.len(), 2); assert_eq!(boundary(&mesh.triangles), before);
        assert_eq!(mesh.uv, uv); assert_eq!(mesh.shared, shared);
        assert!(mesh.triangles.iter().all(|&t| !degenerate(&points,t)));
        let triangles = mesh.triangles.clone(); repair(&mut mesh, &points); assert_eq!(mesh.triangles,triangles);
    }
    #[test]
    fn refuses_concave_zero_or_unresolved_uv_replacement() {
        for uv in [ [[0.,0.],[1.,0.],[0.,1.],[0.25,0.25]], [[0.,0.],[1.,0.],[2.,0.],[0.,1.]], [[f64::NAN,0.],[1.,0.],[0.,1.],[1.,1.]] ] {
            assert!(!admissible_diagonal(uv));
        }
    }
}
