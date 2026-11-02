//! Bounded polynomial proposals. Exact lift identities, not interpolation
//! residuals or root-box proximity, are the eventual admission predicate.
use super::*;
use exact::{float, q};
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
    Ok(Curve {
        degree: n,
        knots: [vec![0.; n + 1], vec![1.; n + 1]].concat(),
        control_points: rows
            .iter()
            .map(|r| r[n + 1..].iter().map(float).collect())
            .collect::<Result<_>>()?,
        weights: vec![1.; n + 1],
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
        for n in 1..=4 {
            let den = if n <= 1 {
                1
            } else if n <= 2 {
                2
            } else {
                4
            };
            let nodes = (0..=n)
                .map(|i| {
                    if i == n {
                        Q::one()
                    } else {
                        Q::new(i.into(), den.into())
                    }
                })
                .collect::<Vec<_>>();
            let requests = nodes[1..n]
                .iter()
                .map(|t| {
                    Ok((
                        piece.branch,
                        float(&(q(ends[0][free]) + (q(ends[1][free]) - q(ends[0][free])) * t))?,
                    ))
                })
                .collect::<Result<Vec<_>>>()?;
            let boxes = intersection_graph::proposal_samples(models, graph, arc.pair, &requests)?;
            let attempt = (|| -> Result<ArcProposal> {
                let mut uv = vec![ends[0].map(q).to_vec()];
                for b in boxes {
                    uv.push(
                        b.map(dyadic)
                            .into_iter()
                            .map(|x| x.map(q))
                            .collect::<Result<Vec<_>>>()?,
                    );
                }
                uv.push(ends[1].map(q).to_vec());
                let mut world = Vec::new();
                for p in &uv {
                    let h = sb::cell(
                        &models[0].faces[pair.faces[0]].surface,
                        [p[0].clone(), p[0].clone()],
                        [p[1].clone(), p[1].clone()],
                    )?;
                    world.push(sb::point(&h[0][0]).to_vec());
                }
                let curve = interpolate(&world, &nodes)?;
                let pc = [
                    interpolate(
                        &uv.iter().map(|v| v[..2].to_vec()).collect::<Vec<_>>(),
                        &nodes,
                    )?,
                    interpolate(
                        &uv.iter().map(|v| v[2..].to_vec()).collect::<Vec<_>>(),
                        &nodes,
                    )?,
                ];
                for side in 0..2 {
                    exact::lift(
                        &curve,
                        &pc[side],
                        &models[side].faces[pair.faces[side]].surface,
                        false,
                    )?;
                }
                Ok(ArcProposal { curve, pc })
            })();
            if let Ok(p) = attempt {
                accepted = Some(p);
                break;
            }
        }
        proposals.push(accepted.ok_or_else(|| {
            fail("original intersection has no admitted EXACT polynomial saved trace")
        })?);
    }
    Ok(proposals)
}
