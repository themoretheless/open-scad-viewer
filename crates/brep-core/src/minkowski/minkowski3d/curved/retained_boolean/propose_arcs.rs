//! Bounded polynomial and positive rational proposals. Exact lift identities, not interpolation
//! residuals or root-box proximity, are the eventual admission predicate.
use super::*;
use exact::{float, q};
mod rational;
pub(super) struct ArcProposal {
    pub curve: Curve,
    pub pc: [Curve; 2],
}
pub(super) fn dyadic(b: [f64; 2]) -> Result<f64> {
    for n in 0..=40 {
        let scale = Q::from_integer(num_bigint::BigInt::from(1u64 << n));
        let x = (&q(b[0]) * &scale).ceil();
        if x <= &q(b[1]) * &scale {
            return float(&(x / scale));
        }
    }
    Err(fail("root endpoint has no bounded exact dyadic proposal"))
}
fn interpolate(points: &[Vec<Q>], nodes: &[Q]) -> Result<Curve> {
    let n = points.len() - 1;
    let dim = points[0].len();
    let choose = |n: usize, k: usize| -> Q {
        (0..k).fold(Q::one(), |a, i| {
            a * integer((n - i) as i64) / integer((i + 1) as i64)
        })
    };
    let mut rows = nodes
        .iter()
        .zip(points)
        .map(|(t, p)| {
            let mut r = (0..=n)
                .map(|i| choose(n, i) * t.pow(i as i32) * (Q::one() - t).pow((n - i) as i32))
                .collect::<Vec<_>>();
            r.extend_from_slice(p);
            r
        })
        .collect::<Vec<_>>();
    for c in 0..=n {
        let pivot = (c..=n)
            .find(|&i| !rows[i][c].is_zero())
            .ok_or_else(|| fail("singular polynomial proposal"))?;
        rows.swap(c, pivot);
        let s = rows[c][c].clone();
        for x in &mut rows[c] {
            *x /= &s;
        }
        for i in 0..=n {
            if i != c {
                let s = rows[i][c].clone();
                for j in c..=n + dim {
                    let v = &s * &rows[c][j];
                    rows[i][j] -= v;
                }
            }
        }
    }
    let mut controls = rows.iter().map(|r| r[n + 1..].to_vec()).collect::<Vec<_>>();
    let mut degree = n;
    // Reduce exact polynomial degree BEFORE storing binary64 poles. An affine
    // pcurve's elevated i/5 controls need not be binary64 even though its exact
    // original degree-one definition is representable.
    while degree > 1 {
        let mut reduced = vec![controls[0].clone()];
        for i in 1..degree {
            reduced.push(
                (0..dim)
                    .map(|k| {
                        (integer(degree as i64) * &controls[i][k]
                            - integer(i as i64) * &reduced[i - 1][k])
                            / integer((degree - i) as i64)
                    })
                    .collect(),
            );
        }
        if reduced.last() != controls.last() {
            break;
        }
        controls = reduced;
        degree -= 1;
    }
    if degree > 8 {
        return Err(fail("polynomial saved trace exceeds degree eight"));
    }
    Ok(Curve {
        degree,
        knots: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
        control_points: controls
            .iter()
            .map(|p| p.iter().map(float).collect())
            .collect::<Result<_>>()?,
        weights: vec![1.; degree + 1],
        periodic: false,
    })
}
fn endpoint(
    pair: &OriginalFacePairGraph,
    piece: &OriginalGraphPiece,
    end: usize,
) -> Result<[f64; 4]> {
    let GraphEndpoint::Boundary(ref key) = piece.endpoints[end] else {
        return Err(fail("natural arc endpoint cannot author a closed Model"));
    };
    let event = pair
        .events
        .iter()
        .find(|e| e.key == *key)
        .ok_or_else(|| fail("missing original boundary event"))?;
    let values = event.parameter_box.map(dyadic);
    Ok([
        values[0].clone()?,
        values[1].clone()?,
        values[2].clone()?,
        values[3].clone()?,
    ])
}
pub(super) fn construct(
    models: [&Model; 2],
    graph: &QualifiedNurbsIntersectionGraph,
) -> Result<Vec<ArcProposal>> {
    let g = graph.certificate();
    let mut proposals = Vec::new();
    if g.arcs.len() > 128 {
        return Err(fail("exact polynomial author arc budget exceeded"));
    }
    for arc in &g.arcs {
        let pair = &g.pairs[arc.pair];
        let piece = &pair.pieces[arc.piece];
        let branch = &pair.branches[piece.branch];
        let free = match branch.free_axis {
            nurbs_core::ss_intersection::FiberAxis::FirstU => 0,
            nurbs_core::ss_intersection::FiberAxis::FirstV => 1,
            nurbs_core::ss_intersection::FiberAxis::SecondU => 2,
            nurbs_core::ss_intersection::FiberAxis::SecondV => 3,
        };
        let ends = [endpoint(pair, piece, 0)?, endpoint(pair, piece, 1)?];
        for end in &ends {
            let point = |side: usize| -> Result<P> {
                let u = q(end[2 * side]);
                let v = q(end[2 * side + 1]);
                let h = sb::cell(
                    &models[side].faces[pair.faces[side]].surface,
                    [u.clone(), u],
                    [v.clone(), v],
                )?;
                Ok(sb::point(&h[0][0]))
            };
            if point(0)? != point(1)? {
                return Err(fail(
                    "ordinary Model endpoint is not exactly representable on BOTH original carriers",
                ));
            }
        }
        if ends[0][free] >= ends[1][free] {
            return Err(fail("exact arc endpoint clock collapsed or reversed"));
        }
        let mut accepted = None;
        let mut cache = BTreeMap::new();
        for n in 1usize..=8 {
            for rational_fit in [false, true] {
                let count = if rational_fit { 2 * n } else { n };
                let den = count.next_power_of_two();
                let nodes = (0..=count)
                    .map(|i| {
                        if i == count {
                            Q::one()
                        } else {
                            Q::new(i.into(), den.into())
                        }
                    })
                    .collect::<Vec<_>>();
                let points = joint_samples(
                    models,
                    graph,
                    ArcSampleContext {
                        arc,
                        pair,
                        piece,
                        ends: &ends,
                        free,
                    },
                    &nodes,
                    &mut cache,
                )?;
                let attempt = (|| -> Result<ArcProposal> {
                    let p = if rational_fit {
                        rational::fit(&points, &nodes, n)?
                    } else {
                        ArcProposal {
                            curve: interpolate(
                                &points.iter().map(|p| p[..3].to_vec()).collect::<Vec<_>>(),
                                &nodes,
                            )?,
                            pc: [
                                interpolate(
                                    &points.iter().map(|p| p[3..5].to_vec()).collect::<Vec<_>>(),
                                    &nodes,
                                )?,
                                interpolate(
                                    &points.iter().map(|p| p[5..7].to_vec()).collect::<Vec<_>>(),
                                    &nodes,
                                )?,
                            ],
                        }
                    };
                    for side in 0..2 {
                        exact::lift(
                            &p.curve,
                            &p.pc[side],
                            &models[side].faces[pair.faces[side]].surface,
                            false,
                        )?;
                    }
                    Ok(p)
                })();
                if let Ok(p) = attempt {
                    accepted = Some(p);
                    break;
                }
            }
            if accepted.is_some() {
                break;
            }
        }
        proposals.push(accepted.ok_or_else(|| {
            fail("original intersection has no admitted EXACT polynomial/rational saved trace")
        })?);
    }
    Ok(proposals)
}

struct ArcSampleContext<'a> {
    arc: &'a OriginalIntersectionArc,
    pair: &'a OriginalFacePairGraph,
    piece: &'a OriginalGraphPiece,
    ends: &'a [[f64; 4]; 2],
    free: usize,
}

fn joint_samples(
    models: [&Model; 2],
    graph: &QualifiedNurbsIntersectionGraph,
    ctx: ArcSampleContext<'_>,
    nodes: &[Q],
    cache: &mut BTreeMap<u64, Vec<Q>>,
) -> Result<Vec<Vec<Q>>> {
    let ArcSampleContext {
        arc,
        pair,
        piece,
        ends,
        free,
    } = ctx;
    let mut requests = Vec::new();
    let mut missing = Vec::new();
    for node in nodes {
        let key = float(node)?.to_bits();
        if cache.contains_key(&key) {
            continue;
        }
        if node.is_zero() || node.is_one() {
            let uv = ends[usize::from(node.is_one())].map(q).to_vec();
            cache.insert(key, joint_point(models, pair, uv)?);
        } else {
            let station =
                float(&(q(ends[0][free]) + (q(ends[1][free]) - q(ends[0][free])) * node))?;
            missing.push(key);
            requests.push((piece.branch, station));
        }
    }
    let boxes = intersection_graph::proposal_samples(models, graph, arc.pair, &requests)?;
    for (key, b) in missing.into_iter().zip(boxes) {
        let uv = b
            .into_iter()
            .map(rational::reconstruct)
            .collect::<Result<Vec<_>>>()?;
        cache.insert(key, joint_point(models, pair, uv)?);
    }
    nodes
        .iter()
        .map(|n| Ok(cache[&float(n)?.to_bits()].clone()))
        .collect()
}
fn joint_point(models: [&Model; 2], pair: &OriginalFacePairGraph, uv: Vec<Q>) -> Result<Vec<Q>> {
    let h = sb::cell(
        &models[0].faces[pair.faces[0]].surface,
        [uv[0].clone(), uv[0].clone()],
        [uv[1].clone(), uv[1].clone()],
    )?;
    let mut p = sb::point(&h[0][0]).to_vec();
    p.extend(uv);
    Ok(p)
}
