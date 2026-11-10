//! Automatic exact interior and transverse-side witness construction.
use super::super::super::algebraic_intersections::{decode_polynomial, decode_ratio};
use super::*;
use arithmetic::branch_neighbourhood::ParameterValue;
use std::sync::Arc;
#[derive(Clone, Copy, Debug)]
pub struct OriginalBoundaryAssemblyLimits {
    pub atlas: ContactAtlasLimits,
    pub ports: ContactPortOrderLimits,
    pub parameter_refinements: usize,
    pub radius_halvings: usize,
}
impl Default for OriginalBoundaryAssemblyLimits {
    fn default() -> Self {
        Self {
            atlas: ContactAtlasLimits::default(),
            ports: ContactPortOrderLimits::default(),
            parameter_refinements: 192,
            radius_halvings: 128,
        }
    }
}
pub(super) fn bound(
    owner: &OwnedOriginalCarriers,
    source: usize,
    cut: &OriginalCarrierCut,
) -> Result<ParameterValue> {
    match cut {
        OriginalCarrierCut::DomainEndpoint(q) => {
            let value = decode_ratio(q)?;
            Ok(ParameterValue {
                primitive: vec![-value.clone(), Q::one()],
                value: vec![Q::zero(), Q::one()],
                interval: [value.clone(), value],
            })
        }
        OriginalCarrierCut::Contact {
            parameter_group, ..
        } => {
            let r = owner.atlas().ordered_parameters()[source][*parameter_group][0];
            let o = owner.atlas().occurrences()[r.occurrence];
            let e = &owner.atlas().owners()[o.pair].nodes()[o.node].exact;
            Ok(ParameterValue {
                primitive: decode_polynomial(&e.primitive)?,
                value: decode_polynomial(&e.parameters[r.side])?,
                interval: [
                    decode_ratio(&e.isolating_interval[0])?,
                    decode_ratio(&e.isolating_interval[1])?,
                ],
            })
        }
    }
}
/// Automatically produce all exact original carrier-side receipts. A trial
/// radius is only a proposal: each accepted receipt independently proves the
/// complete original transverse census and actual closed-wire winding.
pub fn own_original_carrier_sides_automatically(
    owner: Arc<OwnedOriginalCarriers>,
    loops: &[Vec<Curve>],
    limits: OriginalBoundaryAssemblyLimits,
) -> Result<Vec<OwnedOriginalCarrierSides>> {
    owner.reverify_source()?;
    crate::profile_certificate::point_winding::validate_source_wires(loops)?;
    if limits.parameter_refinements > 512
        || limits.radius_halvings > 512
        || loops.iter().flatten().cloned().collect::<Vec<_>>() != owner.atlas().sources()
    {
        return Err(refused(
            "Invalid automatic original boundary allowance or wire census",
        ));
    }
    let mut sides = Vec::new();
    for (carrier, c) in owner.carriers().iter().enumerate() {
        let original = cells(&owner.atlas().sources()[c.source])?
            .into_iter()
            .map(|cell| (cell.net, cell.domain))
            .collect::<Vec<_>>();
        let parameter = arithmetic::carrier_sides::interior(
            &original,
            [
                bound(&owner, c.source, &c.cuts[0])?,
                bound(&owner, c.source, &c.cuts[1])?,
            ],
            limits.parameter_refinements,
        )?;
        let mut radius = Q::one();
        let mut receipt = None;
        for step in 0..=limits.radius_halvings {
            if let Ok(side) = own_original_carrier_sides(
                owner.clone(),
                loops,
                carrier,
                ratio(&parameter),
                ratio(&radius),
                limits.parameter_refinements,
            ) {
                receipt = Some(side);
                break;
            }
            if step < limits.radius_halvings {
                radius /= Q::from_integer(2.into());
            }
        }
        sides.push(receipt.ok_or_else(|| {
            refused(
                "Original carrier side proof remains unresolved within automatic radius allowance",
            )
        })?);
    }
    Ok(sides)
}
/// Automatic complete isolated-contact closed-wire boundary assembly. Original
/// algebraic cuts and rational NURBS sources remain owned throughout. Common
/// components still require overlap assembly; returned boundary components do
/// not yet group holes into physical faces.
pub fn assemble_original_filled_boundaries(
    loops: &[Vec<Curve>],
    coefficients: &[i32],
    width: f64,
    limits: OriginalBoundaryAssemblyLimits,
) -> Result<OwnedOriginalFilledBoundaries> {
    if coefficients.len() != loops.len()
        || coefficients.iter().any(|c| i64::from(*c).abs() > 1_000_000)
        || limits.parameter_refinements > 512
        || limits.radius_halvings > 512
    {
        return Err(refused(
            "Invalid automatic original fill coefficients or allowance",
        ));
    }
    crate::profile_certificate::point_winding::validate_source_wires(loops)?;
    let sources = loops.iter().flatten().cloned().collect::<Vec<_>>();
    let atlas = Arc::new(assemble_original_contact_atlas(
        &sources,
        width,
        limits.atlas,
    )?);
    let owner = Arc::new(own_original_carriers(atlas)?);
    let graph = Arc::new(own_original_carrier_graph(owner.clone())?);
    let cycles = Arc::new(own_original_carrier_cycles(graph, limits.ports)?);
    let sides = own_original_carrier_sides_automatically(owner, loops, limits)?;
    own_original_filled_boundaries(cycles, sides, coefficients)
}
