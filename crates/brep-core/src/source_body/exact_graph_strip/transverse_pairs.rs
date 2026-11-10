//! Narrow-phase ownership for exact adjacent transverse graph-patch contacts.
use super::*;
pub struct TransversePairContact {
    faces: [usize; 2],
    edges: Vec<usize>,
}
impl TransversePairContact {
    pub fn faces(&self) -> [usize; 2] {
        self.faces
    }
    pub fn shared_edges(&self) -> &[usize] {
        &self.edges
    }
}
pub struct TransversePairAudit {
    hulls: PairHullAudit,
    contacts: Vec<TransversePairContact>,
    unresolved: Vec<[usize; 2]>,
}
impl TransversePairAudit {
    pub fn hulls(&self) -> &PairHullAudit {
        &self.hulls
    }
    pub fn contacts(&self) -> &[TransversePairContact] {
        &self.contacts
    }
    pub fn unresolved(&self) -> &[[usize; 2]] {
        &self.unresolved
    }
}
fn coverage(top: &RoofTopology, face: usize, edges: &[usize], u: &Q) -> bool {
    if edges.is_empty() {
        return false;
    }
    let cs = &top.loops[top.faces[face].outer].coedges;
    let mut intervals = Vec::new();
    for c in cs.iter().filter(|c| edges.contains(&c.edge)) {
        let FaceUseGeometry::Transition(line) = &c.pcurve else {
            return false;
        };
        let p = line.points();
        if &p[0][0] != u || &p[1][0] != u {
            return false;
        }
        let low = p[0][1].clone().min(p[1][1].clone());
        let high = p[0][1].clone().max(p[1][1].clone());
        if low < Q::zero() || high > Q::one() || low >= high {
            return false;
        }
        intervals.push((low, high));
    }
    if intervals.len() != edges.len() {
        return false;
    }
    intervals.sort();
    let mut end = Q::zero();
    for (low, high) in intervals {
        if low != end {
            return false;
        }
        end = high;
    }
    end == Q::one()
}
impl ClosedShell {
    /// Fresh all-pair broad phase plus exact transverse-contact isolation.
    /// Every admitted seam must be covered exactly once by actual shared edges
    /// on BOTH neighboring faces. No admission follows merely from incidence.
    pub fn inspect_transverse_pairs(
        &self,
        max_pairs: usize,
        b: &mut Budget,
    ) -> Result<TransversePairAudit> {
        let hulls = self.inspect_pair_hulls(max_pairs, b)?;
        let top = self.topology();
        let mut contacts = Vec::new();
        let mut unresolved = Vec::new();
        for pair in hulls.pairs() {
            if matches!(pair.status, PairHullStatus::Separated { .. }) {
                continue;
            }
            let [i, j] = pair.faces;
            let qualified = if let (FaceGeometry::Transition(a), FaceGeometry::Transition(z)) =
                (&top.faces[i].surface, &top.faces[j].surface)
            {
                let forward = a.width().domain()[1] == z.width().domain()[0];
                let backward = z.width().domain()[1] == a.width().domain()[0];
                let (a, z) = if backward { (z, a) } else { (a, z) };
                if (forward || backward) && a.transverse_contact_only(z, b)? {
                    let u = Q::from_float(a.width().domain()[1]).unwrap();
                    coverage(top, i, &pair.shared_edges, &u)
                        && coverage(top, j, &pair.shared_edges, &u)
                } else {
                    false
                }
            } else {
                false
            };
            if qualified {
                contacts.push(TransversePairContact {
                    faces: pair.faces,
                    edges: pair.shared_edges.clone(),
                });
            } else {
                unresolved.push(pair.faces);
            }
        }
        Ok(TransversePairAudit {
            hulls,
            contacts,
            unresolved,
        })
    }
}
