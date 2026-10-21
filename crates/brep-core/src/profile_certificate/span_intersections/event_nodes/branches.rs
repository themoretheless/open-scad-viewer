//! Actual incoming/outgoing source cells and exact leading branch rays.
use super::super::super::algebraic_intersections::{decode_polynomial, decode_ratio};
use super::*;
use arithmetic::{branch_germ, event_coordinates as exact};
use std::sync::Arc;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeadingBranchRay {
    pub primitive: ExactPolynomial,
    pub isolating_interval: [ExactRatio; 2],
    /// Scaled derivative numerators; the omitted common denominator is positive.
    /// These prove the leading RAY, not a parameter-invariant higher-order angle.
    pub numerator: [ExactPolynomial; 2],
    pub signs: [i8; 2],
    pub order: usize,
}
#[derive(Clone, Debug)]
pub struct OwnedEventBranch {
    owner: Arc<CurveEventNodes>,
    reference: EventParameterRef,
    direction: i8,
    max_refinements: usize,
    cell: usize,
    ray: LeadingBranchRay,
}
fn derive(
    owner: &CurveEventNodes,
    r: EventParameterRef,
    direction: i8,
    max_refinements: usize,
) -> Result<Option<(usize, LeadingBranchRay)>> {
    if r.source > 1
        || r.node >= owner.nodes.len()
        || !matches!(direction, -1 | 1)
        || max_refinements > 512
    {
        return Err(refused(
            "Invalid original event branch reference or allowance",
        ));
    }
    owner.reverify_source()?;
    let node = &owner.nodes[r.node];
    let e = &node.exact;
    let f = decode_polynomial(&e.primitive)?;
    let parameter = decode_polynomial(&e.parameters[r.source])?;
    let root = [
        decode_ratio(&e.isolating_interval[0])?,
        decode_ratio(&e.isolating_interval[1])?,
    ];
    let source = cells(&owner.sources[r.source])?;
    let mut i = node.source_cells[r.source];
    let at = |value: &Q| {
        let mut p = parameter.clone();
        if p.is_empty() {
            p.push(-value.clone());
        } else {
            p[0] -= value;
        }
        exact::zero_at(&p, &f, &root)
    };
    if direction < 0 && at(&source[i].domain[0])? {
        if i == 0 {
            return Ok(None);
        }
        i -= 1;
    } else if direction > 0 && at(&source[i].domain[1])? {
        if i + 1 == source.len() {
            return Ok(None);
        }
        i += 1;
    }
    let (order, numerator, signs) = branch_germ::leading(
        &source[i].net,
        &source[i].domain,
        &parameter,
        &f,
        &root,
        direction,
        max_refinements,
    )?;
    Ok(Some((
        i,
        LeadingBranchRay {
            primitive: e.primitive.clone(),
            isolating_interval: e.isolating_interval.clone(),
            numerator: numerator.map(|p| polynomial(&p)),
            signs,
            order,
        },
    )))
}
impl OwnedEventBranch {
    pub fn owner(&self) -> &Arc<CurveEventNodes> {
        &self.owner
    }
    pub fn reference(&self) -> EventParameterRef {
        self.reference
    }
    pub fn direction(&self) -> i8 {
        self.direction
    }
    pub fn source_cell(&self) -> usize {
        self.cell
    }
    pub fn ray(&self) -> &LeadingBranchRay {
        &self.ray
    }
    pub fn reverify_source(&self) -> Result<()> {
        let derived = derive(
            &self.owner,
            self.reference,
            self.direction,
            self.max_refinements,
        )?;
        let (cell, ray) = derived
            .ok_or_else(|| refused("Original owned branch disappeared from its natural domain"))?;
        if cell != self.cell || ray != self.ray {
            return Err(refused(
                "Original branch cell or leading germ receipt changed",
            ));
        }
        Ok(())
    }
}
/// Own an actual source branch at a certified original contact. Incoming at an
/// internal knot uses the previous actual cell, outgoing the next/right cell;
/// outside-domain branches refuse. Stationary/cusp jets are not replaced by an
/// arbitrary tangent. Equal leading rays still need intrinsic tangent-branch
/// ordering before cyclic contact topology can be claimed.
pub fn own_event_branch(
    owner: Arc<CurveEventNodes>,
    reference: EventParameterRef,
    direction: i8,
    max_refinements: usize,
) -> Result<OwnedEventBranch> {
    let (cell, ray) = derive(&owner, reference, direction, max_refinements)?
        .ok_or_else(|| refused("Requested branch lies outside the original natural domain"))?;
    Ok(OwnedEventBranch {
        owner,
        reference,
        direction,
        max_refinements,
        cell,
        ray,
    })
}

/// None is published only after exact proof that the requested side is outside
/// the actual natural domain; budget/proof failures remain Err.
pub fn try_own_event_branch(
    owner: Arc<CurveEventNodes>,
    reference: EventParameterRef,
    direction: i8,
    max_refinements: usize,
) -> Result<Option<OwnedEventBranch>> {
    Ok(
        derive(&owner, reference, direction, max_refinements)?.map(|(cell, ray)| {
            OwnedEventBranch {
                owner,
                reference,
                direction,
                max_refinements,
                cell,
                ray,
            }
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_branch_replay_rejects_changed_cell_sign_order_and_direction() {
        let a = Curve::from_polyline(vec![vec![-1., 0.], vec![0., 0.], vec![0., 1.]]).unwrap();
        let b = Curve::from_polyline(vec![vec![-1., -1.], vec![1., 1.]]).unwrap();
        let c = certify_span_intersections(&a, &b, false, 1e-12).unwrap();
        let nodes = Arc::new(own_curve_event_nodes(&a, &b, &c).unwrap());
        let owner =
            own_event_branch(nodes, EventParameterRef { node: 0, source: 0 }, -1, 192).unwrap();
        let mut bad = owner.clone();
        bad.cell = 1;
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.ray.signs = [1, 0];
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.ray.order = 2;
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.direction = 1;
        assert!(bad.reverify_source().is_err());
        let mut bad = owner;
        bad.ray.numerator[0].clear();
        assert!(bad.reverify_source().is_err());
    }
}
