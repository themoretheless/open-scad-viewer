//! Atomic parameter carriers retain the original curve and algebraic cuts.
use super::super::super::algebraic_intersections::{decode_polynomial, decode_ratio};
use super::*;
use std::sync::Arc;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OriginalCarrierCut {
    DomainEndpoint(ExactRatio),
    Contact {
        parameter_group: usize,
        point_group: usize,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalAtomicCarrier {
    pub source: usize,
    pub cuts: [OriginalCarrierCut; 2],
}
#[derive(Clone, Debug)]
pub struct OwnedOriginalCarriers {
    atlas: Arc<OriginalContactAtlas>,
    carriers: Vec<OriginalAtomicCarrier>,
}
fn derive(atlas: &OriginalContactAtlas) -> Result<Vec<OriginalAtomicCarrier>> {
    atlas.reverify_source()?;
    let mut out = Vec::new();
    for (source, curve) in atlas.sources().iter().enumerate() {
        let original = cells(curve)?;
        let lo = original
            .first()
            .ok_or_else(|| refused("Empty original carrier source"))?
            .domain[0]
            .clone();
        let hi = original.last().unwrap().domain[1].clone();
        let mut cuts = vec![OriginalCarrierCut::DomainEndpoint(ratio(&lo))];
        let mut at_upper = false;
        for (parameter_group, group) in atlas.ordered_parameters()[source].iter().enumerate() {
            let r = group[0];
            let occurrence = atlas.occurrences()[r.occurrence];
            let node = &atlas.owners()[occurrence.pair].nodes()[occurrence.node];
            let exact = &node.exact;
            let primitive = decode_polynomial(&exact.primitive)?;
            let value = decode_polynomial(&exact.parameters[r.side])?;
            let interval = [
                decode_ratio(&exact.isolating_interval[0])?,
                decode_ratio(&exact.isolating_interval[1])?,
            ];
            let at = |endpoint: &Q| -> Result<bool> {
                let mut p = value.clone();
                if p.is_empty() {
                    p.push(-endpoint.clone());
                } else {
                    p[0] -= endpoint;
                }
                arithmetic::event_coordinates::zero_at(&p, &primitive, &interval)
            };
            let lower = at(&lo)?;
            let upper = at(&hi)?;
            if at_upper
                || (lower && parameter_group != 0)
                || (upper && parameter_group + 1 != atlas.ordered_parameters()[source].len())
            {
                return Err(refused(
                    "Original endpoint conflicts with complete parameter order",
                ));
            }
            let point_group = atlas
                .groups()
                .iter()
                .position(|g| g.contains(&r.occurrence))
                .ok_or_else(|| refused("Original contact has no owned point group"))?;
            let cut = OriginalCarrierCut::Contact {
                parameter_group,
                point_group,
            };
            if lower {
                cuts[0] = cut;
            } else {
                cuts.push(cut);
            }
            at_upper = upper;
        }
        if !at_upper {
            cuts.push(OriginalCarrierCut::DomainEndpoint(ratio(&hi)));
        }
        for pair in cuts.windows(2) {
            out.push(OriginalAtomicCarrier {
                source,
                cuts: [pair[0].clone(), pair[1].clone()],
            });
        }
    }
    Ok(out)
}
impl OwnedOriginalCarriers {
    pub fn atlas(&self) -> &Arc<OriginalContactAtlas> {
        &self.atlas
    }
    pub fn carriers(&self) -> &[OriginalAtomicCarrier] {
        &self.carriers
    }
    pub fn reverify_source(&self) -> Result<()> {
        if derive(&self.atlas)? != self.carriers {
            return Err(refused(
                "Original carrier coverage or algebraic cut ownership changed",
            ));
        }
        Ok(())
    }
}
/// Partition each complete original source traversal at every isolated contact
/// and stationary event. Endpoint contacts replace domain cuts exactly; no
/// rounded extraction or reconstructed curve gains authority. These carriers
/// are not yet filled-region topology or overlap assembly.
pub fn own_original_carriers(atlas: Arc<OriginalContactAtlas>) -> Result<OwnedOriginalCarriers> {
    let carriers = derive(&atlas)?;
    Ok(OwnedOriginalCarriers { atlas, carriers })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_rejects_missing_reversed_and_rebound_carriers() {
        let a = Curve::from_polyline(vec![vec![-1., -1.], vec![1., 1.]]).unwrap();
        let b = Curve::from_polyline(vec![vec![-1., 1.], vec![1., -1.]]).unwrap();
        let atlas = Arc::new(
            assemble_original_contact_atlas(&[a, b], 1e-12, ContactAtlasLimits::default()).unwrap(),
        );
        let owner = own_original_carriers(atlas).unwrap();
        assert_eq!(owner.carriers.len(), 4);
        owner.reverify_source().unwrap();
        let mut bad = owner.clone();
        bad.carriers.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.carriers[0].cuts.swap(0, 1);
        assert!(bad.reverify_source().is_err());
        let mut bad = owner;
        bad.carriers[0].source = 1;
        assert!(bad.reverify_source().is_err());
    }
}
