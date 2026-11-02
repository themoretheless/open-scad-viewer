//! Ordered CAD root sections and critical fibers form an exact oriented cell
//! complex. Root ranks are symbolic selectors, never approximate coordinates.
use super::*;
fn diagonal(flags: &[bool]) -> Result<usize> {
    if flags.iter().filter(|v| **v).count() != 1 {
        return Err(fail(
            "algebraic domain has no unique triangle boundary root",
        ));
    }
    Ok(flags.iter().position(|v| *v).unwrap())
}
fn vertex(vertices: &mut Vec<DomainVertex>, cut: usize, rank: usize) -> u32 {
    let id = vertices.len() as u32;
    vertices.push(DomainVertex { cut, rank });
    id
}
fn edge(edges: &mut Vec<DomainEdge>, kind: DomainEdgeKind, a: u32, b: u32) -> usize {
    let id = edges.len();
    edges.push(DomainEdge {
        kind,
        endpoints: [a, b],
    });
    id
}
fn oriented(id: usize, positive: bool) -> i32 {
    if positive {
        (id + 1) as i32
    } else {
        -((id + 1) as i32)
    }
}
fn ends(e: &DomainEdge, positive: bool) -> [u32; 2] {
    if positive {
        e.endpoints
    } else {
        [e.endpoints[1], e.endpoints[0]]
    }
}
fn cycle(boundary: &[i32], edges: &[DomainEdge]) -> Result<()> {
    if boundary.len() < 2 {
        return Err(fail("algebraic cell boundary has fewer than two arcs"));
    }
    let mut first = None;
    let mut previous = None;
    for oriented in boundary {
        if *oriented == 0 {
            return Err(fail("zero algebraic boundary edge"));
        }
        let id = oriented.unsigned_abs() as usize - 1;
        let e = edges
            .get(id)
            .ok_or_else(|| fail("algebraic boundary edge is absent"))?;
        let [a, b] = ends(e, *oriented > 0);
        if previous.is_some_and(|v| v != a) {
            return Err(fail("algebraic cell boundary chain is disconnected"));
        }
        if first.is_none() {
            first = Some(a);
        }
        previous = Some(b);
    }
    if first != previous {
        return Err(fail("algebraic cell boundary chain is open"));
    }
    Ok(())
}
pub(super) fn assemble(
    cuts: &[DomainPolynomial],
    decoded: &[Bi],
    arrangement: ParameterBranchDecomposition,
) -> Result<DomainPartition> {
    let count = arrangement.cuts.len();
    if arrangement.dimension != 1 || count < 2 || arrangement.slabs.len() + 1 != count {
        return Err(fail("algebraic domain projection census is incomplete"));
    }
    let q0 = roots::ratio(&Q::zero());
    let q1 = roots::ratio(&Q::one());
    if arrangement.cuts[0].parameter != [q0.clone(), q0]
        || arrangement.cuts[count - 1].parameter != [q1.clone(), q1]
    {
        return Err(fail("algebraic domain omits a parameter endpoint"));
    }
    let mut vertices = Vec::new();
    let mut edges = Vec::new();
    let mut faces = Vec::new();
    let mut nodes = Vec::new();
    let mut fibers = Vec::new();
    let mut diagonals = Vec::new();
    for (k, cut) in arrangement.cuts.iter().enumerate() {
        if cut.excluded_nodes.len() != cut.fibers.len() {
            return Err(fail("algebraic critical fiber boundary census is false"));
        }
        let d = diagonal(&cut.excluded_nodes)?;
        diagonals.push(d);
        let ids = (0..=d)
            .map(|rank| vertex(&mut vertices, k, rank))
            .collect::<Vec<_>>();
        let e = (0..d)
            .map(|rank| {
                edge(
                    &mut edges,
                    DomainEdgeKind::Fiber { cut: k, rank },
                    ids[rank],
                    ids[rank + 1],
                )
            })
            .collect::<Vec<_>>();
        nodes.push(ids);
        fibers.push(e);
    }
    let mut sections = Vec::new();
    for (k, slab) in arrangement.slabs.iter().enumerate() {
        if slab.diagonal_side.len() != slab.roots.len() {
            return Err(fail("algebraic open slab boundary census is false"));
        }
        let flags = slab
            .diagonal_side
            .iter()
            .map(|v| *v == 0)
            .collect::<Vec<_>>();
        let d = diagonal(&flags)?;
        if d == 0 {
            return Err(fail(
                "algebraic triangle has an empty open vertical section",
            ));
        }
        let (left, right) = (&arrangement.cuts[k].right, &arrangement.cuts[k + 1].left);
        if left.len() != slab.roots.len() || right.len() != slab.roots.len() {
            return Err(fail("algebraic one-sided incidence omits a branch"));
        }
        if left[0] != 0 || right[0] != 0 || left[d] != diagonals[k] || right[d] != diagonals[k + 1]
        {
            return Err(fail(
                "algebraic slab does not attach to the triangle corners",
            ));
        }
        if left[..=d].windows(2).any(|p| p[0] > p[1]) || right[..=d].windows(2).any(|p| p[0] > p[1])
        {
            return Err(fail("algebraic branch order reverses at a critical fiber"));
        }
        let mut ids = Vec::new();
        for rank in 0..=d {
            let a = *nodes[k]
                .get(left[rank])
                .ok_or_else(|| fail("algebraic left endpoint lies outside triangle"))?;
            let b = *nodes[k + 1]
                .get(right[rank])
                .ok_or_else(|| fail("algebraic right endpoint lies outside triangle"))?;
            ids.push(edge(
                &mut edges,
                DomainEdgeKind::Section { slab: k, rank },
                a,
                b,
            ));
        }
        for rank in 0..d {
            let mut boundary = vec![oriented(ids[rank], true)];
            for r in right[rank]..right[rank + 1] {
                boundary.push(oriented(fibers[k + 1][r], true));
            }
            boundary.push(oriented(ids[rank + 1], false));
            for r in (left[rank]..left[rank + 1]).rev() {
                boundary.push(oriented(fibers[k][r], false));
            }
            cycle(&boundary, &edges)?;
            let u = roots::decode(&slab.sample)?;
            let lower = roots::decode(&slab.roots[rank][1])?;
            let upper = roots::decode(&slab.roots[rank + 1][0])?;
            if lower > upper {
                return Err(fail(
                    "algebraic section intervals overlap without a proved open band",
                ));
            }
            let v = (lower + upper) / integer(2);
            if u <= Q::zero() || u >= Q::one() || v <= Q::zero() || &u + &v >= Q::one() {
                return Err(fail(
                    "algebraic band sign witness is outside the open triangle",
                ));
            }
            let mut signs = Vec::new();
            for p in decoded {
                let value = algebra::at(p, &u, &v);
                if value.is_zero() && !p.is_empty() {
                    return Err(fail("algebraic open band contains an unaccounted cut"));
                }
                signs.push(if value > Q::zero() {
                    1
                } else if value < Q::zero() {
                    -1
                } else {
                    0
                });
            }
            faces.push(DomainFace {
                slab: k,
                rank,
                boundary,
                signs,
            });
        }
        sections.push(ids);
        if vertices.len() + edges.len() + faces.len() > 16384 {
            return Err(fail("algebraic parameter complex cell budget exceeded"));
        }
    }
    if vertices.len() + faces.len() != edges.len() + 1 {
        return Err(fail("algebraic parameter complex is not a disk"));
    }
    // Exact cancellation must leave precisely the original oriented triangle.
    let mut residual = vec![0i32; edges.len()];
    for face in &faces {
        for e in &face.boundary {
            residual[e.unsigned_abs() as usize - 1] += if *e > 0 { 1 } else { -1 };
        }
    }
    let mut outer = Vec::new();
    for s in &sections {
        outer.push(oriented(s[0], true));
    }
    for s in sections.iter().rev() {
        outer.push(oriented(*s.last().unwrap(), false));
    }
    for e in fibers[0].iter().rev() {
        outer.push(oriented(*e, false));
    }
    cycle(&outer, &edges)?;
    let mut expected = vec![0i32; edges.len()];
    for e in outer {
        expected[e.unsigned_abs() as usize - 1] += if e > 0 { 1 } else { -1 };
    }
    if residual != expected {
        return Err(fail(
            "algebraic cell boundary cancellation leaves a gap or duplicate region",
        ));
    }
    Ok(DomainPartition {
        cuts: cuts.to_vec(),
        arrangement,
        vertices,
        edges,
        faces,
    })
}
