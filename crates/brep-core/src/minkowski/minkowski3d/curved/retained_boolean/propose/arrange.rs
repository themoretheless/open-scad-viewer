//! Whole face-network proposal, including paths between outer and hole rims.
//! All resulting regions still require the independent exact partition proof.
use super::*;
use nurbs_core::trim_domain::{Location, TrimDomain};
use std::cmp::Ordering;
type Ray = [Q; 2];
fn ray(c: &Curve, at_end: bool) -> Result<Ray> {
    if !exact::clamped(c) {
        return Err(fail("DCEL tangent proposal requires clamped saved pcurves"));
    }
    let controls = if at_end {
        c.control_points.iter().rev().collect::<Vec<_>>()
    } else {
        c.control_points.iter().collect::<Vec<_>>()
    };
    let start = controls[0];
    for p in controls.iter().take(c.degree + 1).skip(1) {
        let v = [
            exact::q(p[0]) - exact::q(start[0]),
            exact::q(p[1]) - exact::q(start[1]),
        ];
        if v.iter().any(|q| !q.is_zero()) {
            return Ok(v);
        }
    }
    Err(fail("DCEL endpoint jet is not proved nonzero"))
}
fn angle(a: &Ray, b: &Ray) -> Ordering {
    let half = |v: &Ray| usize::from(v[1].is_negative() || (v[1].is_zero() && v[0].is_negative()));
    half(a)
        .cmp(&half(b))
        .then_with(|| (&b[0] * &a[1] - &b[1] * &a[0]).cmp(&Q::zero()))
}
fn reverse(c: &[Coedge]) -> Result<Vec<Coedge>> {
    c.iter()
        .rev()
        .map(|c| Ok(Coedge::new(c.edge, !c.reversed, c.pcurve.reverse()?)))
        .collect()
}
fn curves(c: &[Coedge]) -> Vec<Curve> {
    c.iter().map(|c| c.pcurve.clone()).collect()
}
fn domain(c: &[Coedge]) -> Result<TrimDomain> {
    TrimDomain::new(&[curves(c)], 1e-10)
}
fn inside_boundary(ring: &[Coedge], region: &TrimDomain) -> Result<bool> {
    // These are ownership proposals only; final positive profile, exact chain
    // coverage and original graph membership are proved afresh by partition.
    for c in ring {
        for t in [0.25, 0.5, 0.75] {
            let d = c.pcurve.domain();
            let p = c.pcurve.evaluate(d[0] + (d[1] - d[0]) * t)?.point;
            let r = region.classify([[p[0]; 2], [p[1]; 2]], 4096)?;
            if r.location == Location::Inside && r.winding == Some(1) {
                return Ok(true);
            }
            if r.location == Location::Outside {
                return Ok(false);
            }
        }
    }
    Ok(false)
}
fn regions(m: &Model, halfedges: Vec<Coedge>) -> Result<Vec<(Vec<Coedge>, Vec<Vec<Coedge>>)>> {
    if halfedges.is_empty() || halfedges.len() > 1024 {
        return Err(fail("bounded nonempty DCEL is required"));
    }
    let mut outgoing = BTreeMap::<usize, Vec<(usize, Ray)>>::new();
    for (i, c) in halfedges.iter().enumerate() {
        let from = m.edges[c.edge].vertices[usize::from(c.reversed)];
        outgoing
            .entry(from)
            .or_default()
            .push((i, ray(&c.pcurve, false)?));
    }
    for edges in outgoing.values_mut() {
        edges.sort_by(|a, b| angle(&a.1, &b.1));
        if edges
            .windows(2)
            .any(|w| angle(&w[0].1, &w[1].1) == Ordering::Equal)
        {
            return Err(fail(
                "tangent DCEL rays need a separate contact certificate",
            ));
        }
    }
    let mut next = Vec::new();
    let mut count = vec![0; halfedges.len()];
    for c in &halfedges {
        let to = m.edges[c.edge].vertices[usize::from(!c.reversed)];
        let reference = ray(&c.pcurve, true)?;
        let edges = outgoing.get(&to).ok_or_else(|| fail("open DCEL node"))?;
        let successor = edges
            .iter()
            .rev()
            .find(|(_, v)| angle(v, &reference) == Ordering::Less)
            .or_else(|| {
                edges
                    .iter()
                    .rev()
                    .find(|(_, v)| angle(v, &reference) != Ordering::Equal)
            })
            .ok_or_else(|| fail("DCEL has only a returning tangent ray"))?
            .0;
        next.push(successor);
        count[successor] += 1;
    }
    if count.iter().any(|&n| n != 1) {
        return Err(fail(
            "DCEL successor is not a complete halfedge permutation",
        ));
    }
    let mut seen = BTreeSet::new();
    let mut positive = Vec::new();
    let mut negative = Vec::new();
    for start in 0..halfedges.len() {
        if seen.contains(&start) {
            continue;
        }
        let mut at = start;
        let mut ring = Vec::new();
        loop {
            if !seen.insert(at) {
                if at != start {
                    return Err(fail("DCEL cycles share halfedges"));
                }
                break;
            }
            ring.push(halfedges[at].clone());
            at = next[at];
        }
        if crate::profile_certificate::certify(&[curves(&ring)], 1e-10).is_ok() {
            positive.push(ring);
        } else if crate::profile_certificate::certify(&[curves(&reverse(&ring)?)], 1e-10).is_ok() {
            negative.push(ring);
        } else {
            return Err(fail(
                "DCEL loop is not an independently certifiable simple oriented profile",
            ));
        }
    }
    let domains = positive
        .iter()
        .map(|r| domain(r))
        .collect::<Result<Vec<_>>>()?;
    let mut holes = vec![Vec::new(); positive.len()];
    for hole in negative {
        let candidates = domains
            .iter()
            .enumerate()
            .filter_map(|(i, d)| match inside_boundary(&hole, d) {
                Ok(true) => Some(Ok(i)),
                Ok(false) => None,
                Err(e) => Some(Err(e)),
            })
            .collect::<Result<Vec<_>>>()?;
        let mut owner = None;
        for &i in &candidates {
            let mut smallest = true;
            for &j in &candidates {
                if i != j && !inside_boundary(&positive[i], &domains[j])? {
                    smallest = false;
                    break;
                }
            }
            if smallest && owner.replace(i).is_some() {
                return Err(fail("hole has multiple innermost positive cycles"));
            }
        }
        holes[owner.ok_or_else(|| fail("hole has no uniquely contained positive outer cycle"))?]
            .push(hole);
    }
    Ok(positive.into_iter().zip(holes).collect())
}
pub(super) fn construct(
    p: &mut NurbsFacePartition,
    source: &Model,
    g: &QualifiedNurbsIntersectionGraph,
    arcs: &[ArcProposal],
    side: usize,
    mut roots: BTreeMap<usize, usize>,
) -> Result<()> {
    let graph = g.certificate();
    let mut cut_edges = Vec::new();
    for (i, a) in graph.arcs.iter().enumerate() {
        let mut vertices = [0; 2];
        for end in 0..2 {
            let root = a.vertices[end];
            let point = if end == 0 {
                &arcs[i].curve.control_points[0]
            } else {
                arcs[i].curve.control_points.last().unwrap()
            };
            vertices[end] = if let Some(&v) = roots.get(&root) {
                if p.model.vertices[v].point.as_slice() != point {
                    return Err(fail("DCEL root proposal requires rounding"));
                }
                v
            } else {
                let v = p.model.vertices.len();
                p.model.vertices.push(crate::Vertex {
                    point: point
                        .clone()
                        .try_into()
                        .map_err(|_| fail("invalid world root dimension"))?,
                });
                roots.insert(root, v);
                v
            };
        }
        let edge = p.model.edges.len();
        p.model.edges.push(crate::Edge {
            curve: arcs[i].curve.clone(),
            vertices,
            degenerate: false,
        });
        p.edge_sources.push(RegionEdgeOrigin::Intersection {
            arc: i,
            reversed: false,
        });
        cut_edges.push(edge);
    }
    let mut loops = Vec::new();
    let mut faces = Vec::new();
    let mut face_sources = Vec::new();
    let mut descendants = vec![Vec::new(); source.faces.len()];
    for original in 0..source.faces.len() {
        let mut halfedges = exact::uses(&p.model, original).cloned().collect::<Vec<_>>();
        for (i, a) in graph
            .arcs
            .iter()
            .enumerate()
            .filter(|(_, a)| graph.pairs[a.pair].faces[side] == original)
        {
            let pc = &arcs[i].pc[side];
            halfedges.push(Coedge::new(cut_edges[i], false, pc.clone()));
            halfedges.push(Coedge::new(cut_edges[i], true, pc.reverse()?));
        }
        for (outer, holes) in regions(&p.model, halfedges)? {
            let outer_id = loops.len();
            loops.push(crate::Loop { coedges: outer });
            let mut hole_ids = Vec::new();
            for h in holes {
                hole_ids.push(loops.len());
                loops.push(crate::Loop { coedges: h });
            }
            descendants[original].push(faces.len());
            faces.push(crate::Face {
                surface: source.faces[original].surface.clone(),
                outer: outer_id,
                holes: hole_ids,
            });
            face_sources.push(original);
        }
    }
    p.model.loops = loops;
    p.model.faces = faces;
    p.face_sources = face_sources;
    for s in &mut p.model.shells {
        s.faces = s
            .faces
            .iter()
            .flat_map(|u| {
                descendants[u.face].iter().map(|&face| crate::FaceUse {
                    face,
                    reversed: u.reversed,
                })
            })
            .collect();
    }
    p.model.inherit_topology_ids(&[source]);
    p.model.validate()?;
    Ok(())
}
