//! Exhaustive pairs of ORIGINAL parameter boxes, never retained-model boxes.
//!
//! For a convex hull H, every averaged Jacobian along a segment lies in the
//! entrywise interval matrix DF(H). If its projected interval determinant
//! excludes zero, ALL matrices in that box are nonsingular. Consequently
//! F(x)-F(y)=M(x-y) cannot vanish for distinct x,y in H. Pointwise determinant
//! signs, or the union of endpoint Jacobians, do NOT prove this property.
//! Piecewise C1 source maps use the same segment integral and one-sided jets;
//! the owner has independently established continuity across the source knots.
use super::{cell::Enclosed, *};
use std::collections::BTreeMap;

fn projected(columns: [V3; 3], projection: [[f64; 3]; 3]) -> Result<[V3; 3]> {
    let mut p = [[I::point(0.); 3]; 3];
    for j in 0..3 {
        for i in 0..3 {
            p[j][i] = iv::dot_tight(projection[i].map(I::point), columns[j])?;
        }
    }
    Ok(p)
}
fn regular(columns: [V3; 3], projection: [[f64; 3]; 3]) -> Result<bool> {
    let c = projected(columns, projection)?;
    let det = iv::dot_tight(c[0], iv::cross(c[1], c[2])?)?;
    if det.lo>0. || det.hi<0. {return Ok(true);}
    uniformly_invertible(c)
}
fn frobenius_upper(m:[[I;3];3])->Result<f64>{
    let values=m.iter().flatten().copied().collect::<Vec<_>>();
    Ok(crate::distance_bounds::box_distance(&values,&[I::point(0.);9])?.1)
}
/// ALL matrices A in the original whole-hull interval box satisfy
/// ||I-R A S||2 <= ||I-R Ac||F + ||R||2 ||A S-Ac||F < 1.
/// Neumann's theorem makes EVERY such matrix invertible, including any
/// averaged Jacobian. R/Ac/S are only rounded proposals; all residuals, Gram
/// bounds and perturbations are independently enclosed with outward arithmetic.
fn uniformly_invertible(columns:[V3;3])->Result<bool>{
    let Some(scale)=coordinate_scale(columns) else{return Ok(false);};
    let a=scaled(columns,scale)?;
    let Some(r)=inverse_proposal(a) else{return Ok(false);};
    let center=a.map(|col|col.map(|x|crate::distance_bounds::midpoint(x.lo,x.hi)));
    let mut perturb=[[I::point(0.);3];3];let mut residual=perturb;let mut gram=perturb;
    for i in 0..3 {for j in 0..3 {
        perturb[i][j]=a[j][i].sub(I::point(center[j][i]))?;
        let mut product=I::point(0.);let mut dot=I::point(0.);
        for k in 0..3 {
            product=product.add(I::point(r[i][k]).mul(I::point(center[j][k]))?)?;
            dot=dot.add(I::point(r[k][i]).mul(I::point(r[k][j]))?)?;
        }
        let identity=I::point(if i==j {1.}else{0.});
        residual[i][j]=identity.sub(product)?;
        gram[i][j]=dot.sub(identity)?;
    }}
    // ||R||2²=||RᵀR||2 <= 1+||RᵀR-I||F, valid for any finite R.
    let gram_upper=I::point(1.).add(I::point(frobenius_upper(gram)?))?;
    // Same outward square-root convention as distance_bounds::box_distance.
    let norm_r=I::point(gram_upper.hi.sqrt().next_up());
    let error=I::point(frobenius_upper(residual)?).add(norm_r.mul(I::point(frobenius_upper(perturb)?))?)?;
    Ok(error.hi<1.)
}
pub(crate) fn inverse_proposal(columns: [V3; 3]) -> Option<[[f64; 3]; 3]> {
    let mid = columns.map(|c| c.map(|x| crate::distance_bounds::midpoint(x.lo, x.hi)));
    let cross = |a: [f64; 3], b: [f64; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let cofactors = [
        cross(mid[1], mid[2]),
        cross(mid[2], mid[0]),
        cross(mid[0], mid[1]),
    ];
    let det = mid[0]
        .iter()
        .zip(cofactors[0])
        .map(|(a, b)| a * b)
        .sum::<f64>();
    if !det.is_finite() || det == 0. {
        return None;
    }
    let inverse = cofactors.map(|r| r.map(|x| x / det));
    inverse
        .iter()
        .flatten()
        .all(|x| x.is_finite())
        .then_some(inverse)
}
/// ONE invertible diagonal change x=S*xi on the complete joint chart.
/// A thin carrier and a long path must not make the SPD proposal depend on
/// arbitrary normalized parameter units. This never changes source jets/knots.
fn coordinate_scale(columns: [V3; 3]) -> Option<[f64; 3]> {
    let mut scale = [0.; 3];
    for j in 0..3 {
        let mid = columns[j].map(|x| crate::distance_bounds::midpoint(x.lo, x.hi));
        let norm = mid[0].hypot(mid[1]).hypot(mid[2]);
        let s = 1. / norm;
        if !s.is_finite() || s <= 0. {
            return None;
        }
        scale[j] = s;
    }
    Some(scale)
}
fn scaled(columns: [V3; 3], scale: [f64; 3]) -> Result<[V3; 3]> {
    Ok([
        iv::scale(columns[0], I::point(scale[0]))?,
        iv::scale(columns[1], I::point(scale[1]))?,
        iv::scale(columns[2], I::point(scale[2]))?,
    ])
}
/// A sufficient enclosure test for ALL symmetric matrices in sym(R DF S).
/// Leading principal minors establish SPD without requiring diagonal
/// dominance. Every entry is an outward ORIGINAL whole-piece enclosure; R
/// and S remain fixed over the complete joint hull, including a closed seam.
fn symmetric_positive(columns:[V3;3],metric:[[f64;3];3])->Result<bool>{
    if margin(columns,metric).is_ok_and(|x|x>0.) {return Ok(true);}
    let a=projected(columns,metric)?;
    let mut s=[[I::point(0.);3];3];
    for i in 0..3 {for j in 0..3 {s[i][j]=a[j][i].add(a[i][j])?.div(I::point(2.))?;}}
    if s[0][0].lo<=0. {return Ok(false);}
    let d2=s[0][0].mul(s[1][1])?.sub(s[0][1].mul(s[0][1])?)?;
    if d2.lo<=0. {return Ok(false);}
    let d3=s[0][0].mul(s[1][1].mul(s[2][2])?.sub(s[1][2].mul(s[1][2])?)?)?
        .sub(s[0][1].mul(s[0][1].mul(s[2][2])?.sub(s[1][2].mul(s[0][2])?)?)?)?
        .add(s[0][2].mul(s[0][1].mul(s[1][2])?.sub(s[1][1].mul(s[0][2])?)?)?)?;
    Ok(d3.lo>0.)
}
/// One constant metric on the ENTIRE joint convex chart. Smaller original
/// enclosures may prove its SPD condition when their entrywise matrix union
/// loses correlations. Different metrics in separate pieces are never used.
fn monotone_hull(
    pieces: &[Box3],
    columns: [V3; 3],
    projection: [[f64; 3]; 3],
    query: &mut impl FnMut(Box3, &mut IdealVolumeReport) -> Result<Enclosed>,
    out: &mut IdealVolumeReport,
    max: usize,
) -> Result<bool> {
    // The midpoint of a wide interval matrix need not represent any source
    // frame. Propose ONE metric from a fresh enclosed source-center query;
    // its rounded inverse still supplies zero admission credit. Every piece
    // must pass the same subsequent whole-field SPD proof with that metric.
    if out.cells==max {return Ok(false);}
    out.cells+=1;
    let center=pieces[0].map(|d|[crate::distance_bounds::midpoint(d[0],d[1]);2]);
    let reference=match query(center,out) {Ok(Enclosed::Value(e))=>e.columns,_=>columns};
    let root = projected(reference, projection)?;
    let Some(scale) = coordinate_scale(root) else {
        return Ok(false);
    };
    let Ok(root_scaled) = scaled(root, scale) else {
        return Ok(false);
    };
    let Some(inverse) = inverse_proposal(root_scaled) else {
        return Ok(false);
    };
    let mut pending = pieces.to_vec();
    let mut visits = 0;
    // A fixed metric on a wide turning interval may be a poor proposal even
    // when smaller joint charts are regular. Spend a short probe first and
    // let the exhaustive PAIR cover refine, rather than exhausting the ledger
    // on spatial splits that cannot repair the temporal metric. Small joint
    // charts retain the deeper bounded proposal. Neither limit admits geometry.
    let temporal_width=pieces.iter().map(|b|b[2][1]-b[2][0]).sum::<f64>();
    let visit_limit=if temporal_width>1./64. {16}else{256};
    while let Some(b) = pending.pop() {
        if visits == visit_limit || out.cells == max {
            return Ok(false);
        }
        visits += 1;
        out.cells += 1;
        let e = query(b, out)?;
        if let Enclosed::Value(e) = e {
            // The rounded inverse supplies no proof. All original one-sided
            // Jacobians on every source piece must satisfy the outward SPD
            // test for this SAME R*P, including both closed seam limits.
            // DF*S is the derivative in xi coordinates. BOTH S and the left
            // metric are fixed across every subcell/seam piece of this hull.
            if let Ok(normalized) = scaled(projected(e.columns, projection)?, scale) {
                if symmetric_positive(normalized, inverse).unwrap_or(false) {
                    continue;
                }
            }
        }
        let axis = match e {
            Enclosed::Unavailable(Some(i)) if split_axis(b,i).is_some()=>i,
            _ => match widest(b) {Some((axis,_))=>axis,None=>return Ok(false)},
        };
        let Some(children) = split_axis(b, axis) else {
            return Ok(false);
        };
        pending.extend(children);
    }
    Ok(true)
}

// Proof/search results belong to ONE immutable source context and aggregate
// ledger. Negative entries only skip a completed bounded metric proposal;
// they never classify geometry. Budget-stopped proposals are not cached.
fn cached_metric(
    pieces: &[Box3],
    columns: [V3; 3],
    projection: [[f64; 3]; 3],
    query: &mut impl FnMut(Box3, &mut IdealVolumeReport) -> Result<Enclosed>,
    out: &mut IdealVolumeReport,
    max: usize,
    cache: &mut BTreeMap<Vec<[[u64; 2]; 3]>, bool>,
) -> Result<bool> {
    let k = pieces.iter().map(|b| key(*b)).collect::<Vec<_>>();
    if let Some(proven) = cache.get(&k) {
        return Ok(*proven);
    }
    let proven = monotone_hull(pieces, columns, projection, query, out, max)?;
    if proven || out.cells < max {
        cache.insert(k, proven);
    }
    Ok(proven)
}
fn hull(a: Box3, b: Box3) -> Box3 {
    std::array::from_fn(|i| [a[i][0].min(b[i][0]), a[i][1].max(b[i][1])])
}
fn separated(a: V3, b: V3) -> bool {
    (0..3).any(|i| a[i].hi < b[i].lo || b[i].hi < a[i].lo)
}
fn key(b: Box3) -> [[u64; 2]; 3] {
    b.map(|x| x.map(f64::to_bits))
}
fn widest(b: Box3) -> Option<(usize, f64)> {
    // Temporal jet ambiguity is usually the first obstruction for a turning
    // source. This is only a refinement heuristic, never admission evidence.
    (0..3)
        .filter(|&i|split_axis(b,i).is_some())
        .map(|i| (i, (b[i][1] - b[i][0]) * if i == 2 { 32. } else { 1. }))
        .max_by(|a, b| a.1.total_cmp(&b.1))
}
pub(super) fn certify(
    context: &cell::Context<'_, '_>,
    projection: [[f64; 3]; 3],
    out: &mut IdealVolumeReport,
    max_cells: usize,
    max_exact_work: u64,
    closed: bool,
) -> Result<()> {
    let identity=[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]].map(|v|v.map(I::point));
    let tube=if regular(identity,projection)? {
        intrinsic::Tube::prepare(context,out,max_cells,max_exact_work,closed)?
    }else {None};
    if let Some(tube)=tube {return tube.certify(context,out,max_cells,closed);}
    certify_map(|b,out,max|context.enclose(b,out,max),projection,out,max_cells,closed)
}
/// Internal query engine shared by original transport and coefficient-owned
/// affine volumes. No public callback can fabricate geometry enclosures.
pub(crate) fn certify_map(
    mut enclose:impl FnMut(Box3,&mut IdealVolumeReport,usize)->Result<Enclosed>,
    projection:[[f64;3];3],
    out:&mut IdealVolumeReport,
    max_cells:usize,
    closed:bool,
)->Result<()> {
    out.reason = Some("ideal-volume-work-limit");
    let mut cache = BTreeMap::new();
    let mut metrics = BTreeMap::new();
    let mut query = |b, out: &mut IdealVolumeReport| -> Result<Enclosed> {
        if let Some(e) = cache.get(&key(b)) {
            return Ok(*e);
        }
        let e = enclose(b, out, max_cells)?;
        cache.insert(key(b), e);
        Ok(e)
    };
    let root = [[0., 1.]; 3];
    let mut pairs = vec![(root, root)];
    while let Some((a, b)) = pairs.pop() {
        // Pair inspection itself is bounded, including cache-only visits.
        if out.cells == max_cells {
            return Ok(());
        }
        out.cells += 1;
        let h = hull(a, b);
        let ea = query(a, out)?;
        let eb = if a == b { ea } else { query(b, out)? };
        // Strict separation completes this pair without an expensive joint
        // hull query. Querying an unused hull could exhaust work AFTER the
        // actual proof had finished, and made final reported work depend on
        // whether that irrelevant query was stopped by the budget.
        if let (Enclosed::Value(x), Enclosed::Value(y)) = (ea, eb) {
            if a != b && separated(x.value, y.value) {
                continue;
            }
        }
        let eh = if h == a {
            ea
        } else if h == b {
            eb
        } else {
            // Enclose the ENTIRE convex hull. Endpoint/leaf unions leave the
            // intermediate segment unproved and must never replace this query.
            query(h, out)?
        };
        for e in [ea, eb, eh] {
            if let Enclosed::Unsupported(reason) = e {
                out.reason = Some(reason);
                return Ok(());
            }
        }
        if let Enclosed::Value(e) = eh {
            if regular(e.columns, projection)? {
                continue;
            }
            if h[2][1] - h[2][0] <= 0.125
                && cached_metric(
                    &[h],
                    e.columns,
                    projection,
                    &mut query,
                    out,
                    max_cells,
                    &mut metrics,
                )?
            {
                continue;
            }
        }
        if closed && a != b {
            let (early, late) = if a[2][0] <= b[2][0] { (a, b) } else { (b, a) };
            // Disjoint temporal intervals lift to [late.start, 1+early.end].
            // Both pieces have dt/d(lift)=1. Query ORIGINAL parameters; never
            // round/translate knot vectors or pretend endpoints are distinct.
            if early[2][1] <= late[2][0] && late[2][0] > early[2][1] {
                // A wrapped proof is a separate bounded stage, even when its
                // source boxes are cached. Do not enter it after an ordinary
                // trial exhausted the aggregate ledger.
                if out.cells == max_cells {
                    return Ok(());
                }
                out.cells += 1;
                let mut tail = h;
                let mut head = h;
                tail[2] = [late[2][0], 1.];
                head[2] = [0., early[2][1]];
                let et = query(tail, out)?;
                let eh = query(head, out)?;
                for e in [et, eh] {
                    if let Enclosed::Unsupported(reason) = e {
                        out.reason = Some(reason);
                        return Ok(());
                    }
                }
                if let (Enclosed::Value(t), Enclosed::Value(h)) = (et, eh) {
                    let mut columns = t.columns;
                    for j in 0..3 {
                        for i in 0..3 {
                            columns[j][i] = I::new(
                                columns[j][i].lo.min(h.columns[j][i].lo),
                                columns[j][i].hi.max(h.columns[j][i].hi),
                            )?;
                        }
                    }
                    // ONE complete interval matrix and projection for the
                    // lifted hull, including both one-sided seam limits.
                    if regular(columns, projection)? {
                        continue;
                    }
                    if (1. - tail[2][0]) + head[2][1] <= 0.125
                        && cached_metric(
                            &[tail, head],
                            columns,
                            projection,
                            &mut query,
                            out,
                            max_cells,
                            &mut metrics,
                        )?
                    {
                        continue;
                    }
                }
            }
        }
        if out.cells == max_cells {
            return Ok(());
        }
        let choice=match (widest(a),if a!=b {widest(b)} else {None}) {
            (Some(x),Some(y)) if y.1>x.1 =>Some((y.0,true)),
            (Some(x),_)=>Some((x.0,false)),
            (None,Some(y))=>Some((y.0,true)),
            (None,None)=>None,
        };
        let Some((mut axis,mut split_b))=choice else {
            out.reason=Some("ideal-volume-atlas-precision-limit");return Ok(());
        };
        let e = if split_b { eb } else { ea };
        let preferred = match e {
            Enclosed::Unavailable(Some(axis)) => Some(axis),
            _ => match eh {
                Enclosed::Unavailable(Some(axis)) => Some(axis),
                _ => None,
            },
        };
        // A wide JOINT hull may suggest an axis that neither narrow endpoint
        // box can split. Keep a splittable axis instead of prematurely refusing
        // an otherwise resolvable pair. This changes only subdivision proposals.
        if let Some(preferred) = preferred.filter(|&k|split_axis(a,k).is_some()
            || a!=b && split_axis(b,k).is_some()) {
            axis = preferred;
            split_b = a != b && split_axis(b,axis).is_some()
                && (split_axis(a,axis).is_none() || b[axis][1]-b[axis][0]>a[axis][1]-a[axis][0]);
        }
        let Some(children) = split_axis(if split_b { b } else { a }, axis) else {
            out.reason = Some("ideal-volume-atlas-precision-limit");
            return Ok(());
        };
        if a == b {
            // All unordered pairs in this diagonal square, including both
            // child diagonals. Symmetry supplies the reversed cross pair.
            pairs.push((children[0], children[1]));
            pairs.push((children[1], children[1]));
            pairs.push((children[0], children[0]));
        } else if split_b {
            pairs.push((a, children[1]));
            pairs.push((a, children[0]));
        } else {
            pairs.push((children[1], b));
            pairs.push((children[0], b));
        }
    }
    // No partial-cover flags: every diagonal and distinct pair was resolved.
    out.embedding_certified = true;
    out.regularity_certified = true;
    out.parameter_topology_preserved = true;
    out.reason = None;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn regular_interval_matrix_is_stronger_than_endpoint_determinant_signs() {
        // A and B both have det=11, but their convex average is singular
        // somewhere. A pair of positive pointwise signs supplies no admission.
        let a = [[1., -1., 0.], [10., 1., 0.], [0., 0., 1.]];
        let b = [[1., 10., 0.], [-1., 1., 0.], [0., 0., 1.]];
        let p = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
        for m in [a, b] {
            assert!(regular(m.map(|v| v.map(I::point)), p).unwrap());
        }
        let enclosed = std::array::from_fn(|j| {
            std::array::from_fn(|i| I::new(a[j][i].min(b[j][i]), a[j][i].max(b[j][i])).unwrap())
        });
        assert!(!regular(enclosed, p).unwrap());
        let mut singular = p;
        singular[2] = [0.; 3];
        assert!(!regular(a.map(|v| v.map(I::point)), singular).unwrap());
        assert!(!separated([I::point(0.); 3], [I::point(0.); 3]));
    }
    #[test]
    fn one_fixed_coordinate_scale_balances_thin_carrier_and_long_traversal() {
        let r = I::point(1. / 128.);
        let w = I::point(16.);
        let c = I::new(0.99, 1.).unwrap();
        let s = I::new(-0.05, 0.05).unwrap();
        let z = I::point(0.);
        let columns = [
            [r.mul(c).unwrap(), z, r.mul(s).unwrap()],
            [z, r, z],
            [w.mul(s).unwrap(), z, w.mul(c).unwrap()],
        ];
        let inverse = inverse_proposal(columns).unwrap();
        assert!(margin(columns, inverse).unwrap() < 0.);
        let scale = coordinate_scale(columns).unwrap();
        assert!(scale.iter().all(|x| x.is_finite() && *x > 0.));
        let normalized = scaled(columns, scale).unwrap();
        let inverse = inverse_proposal(normalized).unwrap();
        assert!(margin(normalized, inverse).unwrap() > 0.9);
        let singular = [[1., 0., 0.], [0., 1., 0.], [0.; 3]];
        assert!(!regular(columns, singular).unwrap());
        assert!(coordinate_scale(projected(columns, singular).unwrap()).is_none());
    }
    #[test]
    fn one_metric_spd_uses_all_interval_principal_minors_without_diagonal_dominance(){
        let identity=[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]];
        let c=[[1.,2.,0.],[2.,5.,0.],[0.,0.,1.]].map(|v|v.map(I::point));
        assert!(margin(c,identity).unwrap()<0.);assert!(symmetric_positive(c,identity).unwrap());
        let narrow=c.map(|v|v.map(|x|I::new(x.lo-1e-6,x.hi+1e-6).unwrap()));
        assert!(symmetric_positive(narrow,identity).unwrap());
        let mut wide=c;wide[0][1]=I::new(1.,3.).unwrap();wide[1][0]=wide[0][1];
        assert!(!symmetric_positive(wide,identity).unwrap());
        for bad in [[[1.,2.,0.],[2.,1.,0.],[0.,0.,1.]],[[1.,0.,0.],[0.,1.,0.],[0.,0.,0.]]] {
            assert!(!symmetric_positive(bad.map(|v|v.map(I::point)),identity).unwrap());
        }
    }

    #[test]
    fn whole_interval_neumann_bound_proves_invertibility_without_pointwise_or_determinant_credit(){
        let rotation=[[-1./3.,2./3.,2./3.],[2./3.,-1./3.,2./3.],[2./3.,2./3.,-1./3.]];
        let c=rotation.map(|col|col.map(|v|I::new(v-0.3,v+0.3).unwrap()));
        let det=iv::dot_tight(c[0],iv::cross(c[1],c[2]).unwrap()).unwrap();
        assert!(det.lo<=0. && det.hi>=0.,"{det:?}");
        assert!(uniformly_invertible(c).unwrap());
        let wide=rotation.map(|col|col.map(|v|I::new(v-0.6,v+0.6).unwrap()));
        assert!(!uniformly_invertible(wide).unwrap());
        let singular=[[I::point(1.);3];3];assert!(!uniformly_invertible(singular).unwrap());
    }

}
