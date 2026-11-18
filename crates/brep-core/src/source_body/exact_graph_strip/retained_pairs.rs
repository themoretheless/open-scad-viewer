//! Contact isolation against the immutable complementary original-chart notch.
use super::*;
use std::collections::BTreeSet;
pub struct RetainedPairAudit {
    transverse: TransversePairAudit,
    contacts: Vec<([usize; 2], Vec<usize>)>,
    unresolved: Vec<[usize; 2]>,
}
impl RetainedPairAudit {
    pub fn transverse(&self) -> &TransversePairAudit {
        &self.transverse
    }
    pub fn contacts(&self) -> &[([usize; 2], Vec<usize>)] {
        &self.contacts
    }
    pub fn unresolved(&self) -> &[[usize; 2]] {
        &self.unresolved
    }
}
fn partition(mut ranges: Vec<[Q; 2]>, low: Q, high: Q) -> bool {
    if ranges.is_empty() {
        return false;
    }
    ranges.sort();
    let mut end = low;
    for [a, z] in ranges {
        if a != end || a >= z {
            return false;
        }
        end = z;
    }
    end == high
}
fn end_coverage(top: &RoofTopology, face: usize, edges: &[usize], u: &Q, side: usize) -> bool {
    let mut intervals = Vec::new();
    for edge in edges {
        let Some(c) = top.loops[top.faces[face].outer]
            .coedges
            .iter()
            .find(|c| c.edge == *edge)
        else {
            return false;
        };
        let FaceUseGeometry::Transition(line) = &c.pcurve else {
            return false;
        };
        let p = line.points();
        if &p[0][0] != u || &p[1][0] != u {
            return false;
        }
        intervals.push([
            p[0][1].clone().min(p[1][1].clone()),
            p[0][1].clone().max(p[1][1].clone()),
        ]);
    }
    let half = Q::new(1.into(), 2.into());
    partition(
        intervals,
        if side == 0 { Q::zero() } else { half.clone() },
        if side == 0 { half } else { Q::one() },
    )
}
fn qualify(
    top: &RoofTopology,
    face: usize,
    p: &Arc<Patch>,
    r: &RetainedRegion,
    shared: &[usize],
    b: &mut Budget,
) -> Result<bool> {
    let side = r.side();
    if r.source() != p.sources()[side] || !p.inspect_retained_projection(side, b)? {
        return Ok(false);
    }
    let domain = p.width().domain().map(|x| Q::from_float(x).unwrap());
    let mut ranges = Vec::new();
    let mut expected = BTreeSet::new();
    let mut ends: [Vec<usize>; 2] = std::array::from_fn(|_| Vec::new());
    let mut global: Option<[Q; 2]> = None;
    for (u, _) in r.uses() {
        match u {
            RetainedUse::Contact(c) => {
                let uv = c.contact().uv_points();
                let interval = [uv[0][0].clone(), uv.last().unwrap()[0].clone()];
                if let Some(g) = &mut global {
                    g[0] = g[0].clone().min(interval[0].clone());
                    g[1] = g[1].clone().max(interval[1].clone());
                } else {
                    global = Some(interval.clone());
                }
                if c.face == face {
                    if !Arc::ptr_eq(&c.panel, p) || c.side != side || !shared.contains(&c.edge) {
                        return Ok(false);
                    }
                    let curve = &top.edges[c.edge].curve;
                    if !Arc::ptr_eq(&curve.root, &c.root)
                        || curve.range != *c.contact.root_parameters()
                    {
                        return Ok(false);
                    }
                    let Some(coedge) = top.loops[top.faces[face].outer]
                        .coedges
                        .iter()
                        .find(|u| u.edge == c.edge)
                    else {
                        return Ok(false);
                    };
                    let FaceUseGeometry::Transition(line) = &coedge.pcurve else {
                        return Ok(false);
                    };
                    let pts = line.points();
                    let v = Q::from_integer(side.into());
                    if pts.iter().any(|p| p[1] != v) {
                        return Ok(false);
                    }
                    let clock = [
                        pts[0][0].clone().min(pts[1][0].clone()),
                        pts[0][0].clone().max(pts[1][0].clone()),
                    ];
                    if clock != interval {
                        return Ok(false);
                    }
                    ranges.push(interval);
                    expected.insert(c.edge);
                }
            }
            RetainedUse::End(c) if c.face == face => {
                if !Arc::ptr_eq(&c.panel, p)
                    || c.contact.side() != side
                    || !shared.contains(&c.edge)
                {
                    return Ok(false);
                }
                let edge = &top.edges[c.edge].curve;
                if !Arc::ptr_eq(&edge.root, &c.root) || edge.range != *c.contact.root_parameters() {
                    return Ok(false);
                }
                ends[usize::from(c.contact.at_end())].push(c.edge);
                expected.insert(c.edge);
            }
            _ => (),
        }
    }
    if !partition(ranges, domain[0].clone(), domain[1].clone()) {
        return Ok(false);
    }
    let Some(global) = global else {
        return Ok(false);
    };
    for end in 0..2 {
        if domain[end] == global[end] {
            if !end_coverage(top, face, &ends[end], &domain[end], side) {
                return Ok(false);
            }
        } else if !ends[end].is_empty() {
            return Ok(false);
        }
    }
    Ok(expected == shared.iter().copied().collect())
}
impl ClosedShell {
    /// Shared footprint boundaries are admitted only after complete longitudinal
    /// clock coverage, required global end-half coverage, and exact edge/root
    /// ownership on the actual retained graph-notch region are all established.
    pub fn inspect_retained_pairs(
        &self,
        max_pairs: usize,
        b: &mut Budget,
    ) -> Result<RetainedPairAudit> {
        let transverse = self.inspect_transverse_pairs(max_pairs, b)?;
        let top = self.topology();
        let mut contacts = Vec::new();
        let mut unresolved = Vec::new();
        for pair in transverse.unresolved() {
            let [i, j] = *pair;
            let candidate = match (&top.faces[i].surface, &top.faces[j].surface) {
                (FaceGeometry::Transition(p), FaceGeometry::Retained(r)) => Some((i, p, r)),
                (FaceGeometry::Retained(r), FaceGeometry::Transition(p)) => Some((j, p, r)),
                _ => None,
            };
            let hull = transverse
                .hulls()
                .pairs()
                .iter()
                .find(|p| p.faces == *pair)
                .unwrap();
            let accepted = if let Some((face, p, r)) = candidate {
                qualify(top, face, p, r, &hull.shared_edges, b)?
            } else {
                false
            };
            if accepted {
                contacts.push((*pair, hull.shared_edges.clone()));
            } else {
                unresolved.push(*pair);
            }
        }
        Ok(RetainedPairAudit {
            transverse,
            contacts,
            unresolved,
        })
    }
}
