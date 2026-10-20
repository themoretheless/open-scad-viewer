//! Winding witnesses bound to actual left/right sides of original carriers.
use super::super::super::algebraic_intersections::{decode_polynomial, decode_ratio};
use super::*;
use std::sync::Arc;
#[derive(Clone, Debug)]
pub struct OwnedOriginalCarrierSides {
    owner: Arc<OwnedOriginalCarriers>,
    loops: Vec<Vec<Curve>>,
    carrier: usize,
    parameter: ExactRatio,
    radius: ExactRatio,
    refinements: usize,
    certificates: [PointWindingCertificate; 2],
}
fn derive(
    owner: &OwnedOriginalCarriers,
    loops: &[Vec<Curve>],
    carrier: usize,
    parameter: &ExactRatio,
    radius: &ExactRatio,
    refinements: usize,
) -> Result<[PointWindingCertificate; 2]> {
    owner.reverify_source()?;
    if loops.iter().flatten().cloned().collect::<Vec<_>>() != owner.atlas().sources()
        || carrier >= owner.carriers().len()
        || refinements > 512
    {
        return Err(refused(
            "Carrier side winding is not bound to the original wire census",
        ));
    }
    let c = &owner.carriers()[carrier];
    let t = decode_ratio(parameter)?;
    for (side, cut) in c.cuts.iter().enumerate() {
        let sign = match cut {
            OriginalCarrierCut::DomainEndpoint(q) => {
                use std::cmp::Ordering;
                match t.cmp(&decode_ratio(q)?) {
                    Ordering::Less => -1,
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                }
            }
            OriginalCarrierCut::Contact {
                parameter_group, ..
            } => {
                let r = owner.atlas().ordered_parameters()[c.source][*parameter_group][0];
                let o = owner.atlas().occurrences()[r.occurrence];
                let e = &owner.atlas().owners()[o.pair].nodes()[o.node].exact;
                let mut value = decode_polynomial(&e.parameters[r.side])?;
                if value.is_empty() {
                    value.push(-t.clone());
                } else {
                    value[0] -= &t;
                }
                -arithmetic::event_coordinates::sign_at(
                    &value,
                    &decode_polynomial(&e.primitive)?,
                    &[
                        decode_ratio(&e.isolating_interval[0])?,
                        decode_ratio(&e.isolating_interval[1])?,
                    ],
                    refinements,
                )?
            }
        };
        if sign != if side == 0 { 1 } else { -1 } {
            return Err(refused(
                "Side witness parameter is not strictly inside the owned original carrier",
            ));
        }
    }
    let mut nets = Vec::new();
    let mut selected = None;
    for (source, curve) in owner.atlas().sources().iter().enumerate() {
        for cell in cells(curve)? {
            if source == c.source && t > cell.domain[0] && t < cell.domain[1] {
                selected = Some((
                    nets.len(),
                    (&t - &cell.domain[0]) / (&cell.domain[1] - &cell.domain[0]),
                ));
            }
            nets.push(cell.net);
        }
    }
    let (selected, local) = selected.ok_or_else(|| {
        refused("Side witness needs a regular interior original knot cell parameter")
    })?;
    let queries = arithmetic::carrier_sides::probes(
        &nets,
        selected,
        &local,
        &decode_ratio(radius)?,
        refinements,
    )?;
    Ok([
        certify_rational_point_winding(loops, queries[0].clone().map(|q| ratio(&q)))?,
        certify_rational_point_winding(loops, queries[1].clone().map(|q| ratio(&q)))?,
    ])
}
impl OwnedOriginalCarrierSides {
    pub(super) fn loops(&self) -> &[Vec<Curve>] {
        &self.loops
    }
    pub fn owner(&self) -> &Arc<OwnedOriginalCarriers> {
        &self.owner
    }
    pub fn carrier(&self) -> usize {
        self.carrier
    }
    /// Left then right relative to the original increasing traversal.
    pub fn certificates(&self) -> &[PointWindingCertificate; 2] {
        &self.certificates
    }
    pub fn reverify_source(&self) -> Result<()> {
        if derive(
            &self.owner,
            &self.loops,
            self.carrier,
            &self.parameter,
            &self.radius,
            self.refinements,
        )? != self.certificates
        {
            return Err(refused("Original carrier side winding witness changed"));
        }
        Ok(())
    }
}
/// Prove a closed normal segment meets the full original source census only at
/// this regular carrier interior point, then compute exact closed-wire winding
/// on each side. A smaller radius may be needed; no proximity sample grants a
/// side label. Winding query points and source wires remain owned by the proof.
pub fn own_original_carrier_sides(
    owner: Arc<OwnedOriginalCarriers>,
    loops: &[Vec<Curve>],
    carrier: usize,
    parameter: ExactRatio,
    radius: ExactRatio,
    refinements: usize,
) -> Result<OwnedOriginalCarrierSides> {
    let certificates = derive(&owner, loops, carrier, &parameter, &radius, refinements)?;
    Ok(OwnedOriginalCarrierSides {
        owner,
        loops: loops.to_vec(),
        carrier,
        parameter,
        radius,
        refinements,
        certificates,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn side_winding_replay_rejects_stale_query_source_and_claims() {
        let points = [[0., 0.], [1., 0.], [1., 1.], [0., 1.], [0., 0.]];
        let wire = points
            .windows(2)
            .map(|p| Curve::from_polyline(vec![p[0].to_vec(), p[1].to_vec()]).unwrap())
            .collect::<Vec<_>>();
        let loops = vec![wire.clone()];
        let atlas = Arc::new(
            assemble_original_contact_atlas(&wire, 1e-12, ContactAtlasLimits::default()).unwrap(),
        );
        let owner = Arc::new(own_original_carriers(atlas).unwrap());
        let q = |a: &str, b: &str| [a.to_string(), b.to_string()];
        let s =
            own_original_carrier_sides(owner, &loops, 0, q("1", "2"), q("1", "8"), 192).unwrap();
        s.reverify_source().unwrap();
        let mut bad = s.clone();
        bad.certificates[0].windings[0] = 0;
        assert!(bad.reverify_source().is_err());
        let mut bad = s.clone();
        bad.radius = q("1", "16");
        assert!(bad.reverify_source().is_err());
        let mut bad = s.clone();
        bad.parameter = q("1", "4");
        assert!(bad.reverify_source().is_err());
        let mut bad = s.clone();
        bad.loops[0].swap(0, 1);
        assert!(bad.reverify_source().is_err());
        let mut bad = s;
        bad.refinements = 513;
        assert!(bad.reverify_source().is_err());
    }
}
