//! Exact endpoint incidence of source-owned atomic carriers.
use super::super::super::algebraic_intersections::{decode_polynomial, decode_ratio};
use super::*;
use std::sync::Arc;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OriginalCarrierVertex {
    Contact(usize),
    RationalEndpoint([ExactRatio; 2]),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OriginalCarrierIncidence {
    pub carrier: usize,
    /// +1 leaves the lower cut; -1 leaves the upper cut.
    pub direction: i8,
}
#[derive(Clone, Debug)]
pub struct OwnedOriginalCarrierGraph {
    owner: Arc<OwnedOriginalCarriers>,
    vertices: Vec<OriginalCarrierVertex>,
    ends: Vec<[usize; 2]>,
    incidence: Vec<Vec<OriginalCarrierIncidence>>,
}
fn contact_matches(atlas: &OriginalContactAtlas, group: usize, point: &[Q; 2]) -> Result<bool> {
    let occurrence = atlas.occurrences()[atlas.groups()[group][0]];
    let e = &atlas.owners()[occurrence.pair].nodes()[occurrence.node].exact;
    let f = decode_polynomial(&e.primitive)?;
    let root = [
        decode_ratio(&e.isolating_interval[0])?,
        decode_ratio(&e.isolating_interval[1])?,
    ];
    for axis in 0..2 {
        let numerator = decode_polynomial(&e.position[axis][0])?;
        let denominator = decode_polynomial(&e.position[axis][1])?;
        let mut difference = vec![Q::zero(); numerator.len().max(denominator.len())];
        for (i, c) in numerator.iter().enumerate() {
            difference[i] += c;
        }
        for (i, c) in denominator.iter().enumerate() {
            difference[i] -= c * &point[axis];
        }
        if !arithmetic::event_coordinates::zero_at(&difference, &f, &root)? {
            return Ok(false);
        }
    }
    Ok(true)
}
type GraphData = (
    Vec<OriginalCarrierVertex>,
    Vec<[usize; 2]>,
    Vec<Vec<OriginalCarrierIncidence>>,
);
fn derive(owner: &OwnedOriginalCarriers) -> Result<GraphData> {
    owner.reverify_source()?;
    let atlas = owner.atlas();
    let mut vertices = (0..atlas.groups().len())
        .map(OriginalCarrierVertex::Contact)
        .collect::<Vec<_>>();
    let mut ends = Vec::new();
    let mut incidence = vec![Vec::new(); vertices.len()];
    for (carrier, c) in owner.carriers().iter().enumerate() {
        let mut pair = [0; 2];
        for side in 0..2 {
            let index = match &c.cuts[side] {
                OriginalCarrierCut::Contact { point_group, .. } => *point_group,
                OriginalCarrierCut::DomainEndpoint(parameter) => {
                    let source = cells(&atlas.sources()[c.source])?;
                    let parameter = decode_ratio(parameter)?;
                    let h = if parameter == source[0].domain[0] {
                        source[0].net[0].clone()
                    } else if parameter == source.last().unwrap().domain[1] {
                        source.last().unwrap().net.last().unwrap().clone()
                    } else {
                        return Err(refused("Carrier endpoint is not an actual domain endpoint"));
                    };
                    if h[2].is_zero() {
                        return Err(refused("Original endpoint denominator vanishes"));
                    }
                    let point = [&h[0] / &h[2], &h[1] / &h[2]];
                    let mut found = None;
                    for (i, v) in vertices.iter().enumerate() {
                        let equal = match v {
                            OriginalCarrierVertex::Contact(group) => {
                                contact_matches(atlas, *group, &point)?
                            }
                            OriginalCarrierVertex::RationalEndpoint(p) => {
                                [decode_ratio(&p[0])?, decode_ratio(&p[1])?] == point
                            }
                        };
                        if equal {
                            if found.is_some() {
                                return Err(refused(
                                    "Distinct original point groups coincide at an endpoint",
                                ));
                            }
                            found = Some(i);
                        }
                    }
                    if let Some(i) = found {
                        i
                    } else {
                        let i = vertices.len();
                        vertices.push(OriginalCarrierVertex::RationalEndpoint(
                            point.map(|q| ratio(&q)),
                        ));
                        incidence.push(Vec::new());
                        i
                    }
                }
            };
            pair[side] = index;
            incidence[index].push(OriginalCarrierIncidence {
                carrier,
                direction: if side == 0 { 1 } else { -1 },
            });
        }
        ends.push(pair);
    }
    Ok((vertices, ends, incidence))
}
impl OwnedOriginalCarrierGraph {
    pub fn owner(&self) -> &Arc<OwnedOriginalCarriers> {
        &self.owner
    }
    pub fn vertices(&self) -> &[OriginalCarrierVertex] {
        &self.vertices
    }
    pub fn ends(&self) -> &[[usize; 2]] {
        &self.ends
    }
    /// Incidence is deterministic source order, not geometric cyclic order.
    pub fn incidence(&self) -> &[Vec<OriginalCarrierIncidence>] {
        &self.incidence
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (vertices, ends, incidence) = derive(&self.owner)?;
        if vertices != self.vertices || ends != self.ends || incidence != self.incidence {
            return Err(refused("Original carrier endpoint incidence changed"));
        }
        Ok(())
    }
}
/// Reconcile actual endpoint coordinates exactly with complete contact groups.
/// Original traversals and loop half-edges remain distinct. No tolerance merge.
/// This owns incidence only; it does not admit face cycles or filled regions.
pub fn own_original_carrier_graph(
    owner: Arc<OwnedOriginalCarriers>,
) -> Result<OwnedOriginalCarrierGraph> {
    let (vertices, ends, incidence) = derive(&owner)?;
    Ok(OwnedOriginalCarrierGraph {
        owner,
        vertices,
        ends,
        incidence,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn graph_replay_rejects_ends_incidence_and_vertex_changes() {
        let c = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap();
        let a = Arc::new(
            assemble_original_contact_atlas(&[c], 1e-12, ContactAtlasLimits::default()).unwrap(),
        );
        let g = own_original_carrier_graph(Arc::new(own_original_carriers(a).unwrap())).unwrap();
        g.reverify_source().unwrap();
        let mut bad = g.clone();
        bad.ends[0].swap(0, 1);
        assert!(bad.reverify_source().is_err());
        let mut bad = g.clone();
        bad.incidence[0].clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = g;
        bad.vertices.pop();
        assert!(bad.reverify_source().is_err());
    }
}
