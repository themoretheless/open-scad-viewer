//! Intrinsic cyclic order on the unified graph, after common-branch deduplication.
use super::super::super::super::algebraic_intersections::{decode_polynomial, decode_ratio};
use super::*;
use arithmetic::{branch_neighbourhood::ParameterValue, branch_slice::SliceBranch};
use std::cmp::Ordering;
#[derive(Clone, Debug)]
pub struct OwnedOriginalUnifiedRotation {
    owner: Arc<OwnedOriginalUnifiedPorts>,
    limits: ContactPortOrderLimits,
    rotations: Vec<Vec<usize>>,
    comparisons: usize,
}
fn slice(
    owner: &OwnedOriginalUnifiedPorts,
    p: &OriginalUnifiedPort,
    axis: usize,
    sign: i8,
    l: ContactPortOrderLimits,
) -> Result<SliceBranch> {
    let c = &owner.graph().owner().carriers()[p.carrier];
    let values = &owner.graph().owner().cuts()[p.source];
    let contact = &values[p.cut].value;
    let other = if c.cuts[0] == p.cut {
        c.cuts[1]
    } else {
        c.cuts[0]
    };
    let decode = |v: &OriginalSourceParameter| -> Result<ParameterValue> {
        Ok(ParameterValue {
            primitive: decode_polynomial(&v.primitive)?,
            value: decode_polynomial(&v.parameter)?,
            interval: [decode_ratio(&v.interval[0])?, decode_ratio(&v.interval[1])?],
        })
    };
    let contact = decode(contact)?;
    let neighbour = decode(&values[other].value)?;
    let actual = cells(&owner.graph().owner().dictionary().sources()[p.source])?;
    let cell = &actual[p.cell];
    // Complete unified source cuts guarantee no other original contact before
    // neighbour. The source-cell critical census independently proves monotonicity.
    let proof = arithmetic::branch_neighbourhood::prove(
        &cell.net,
        &cell.domain,
        contact.clone(),
        Some(neighbour),
        p.direction,
        axis,
        sign,
        l.max_neighbourhood_steps,
    )?;
    Ok(SliceBranch {
        net: cell.net.clone(),
        domain: cell.domain.clone(),
        contact: ParameterValue {
            interval: proof.root,
            ..contact
        },
        cut: proof.cut,
        cut_point: proof.cut_point,
        direction: p.direction,
    })
}
fn compare(
    owner: &OwnedOriginalUnifiedPorts,
    a: usize,
    b: usize,
    l: ContactPortOrderLimits,
) -> Result<Ordering> {
    let a = &owner.ports()[a];
    let b = &owner.ports()[b];
    let leading = super::super::port_order::compare(&a.ray, &b.ray, l.algebraic)?;
    if leading != Ordering::Equal {
        return Ok(leading);
    }
    let axis = usize::from(a.ray.signs[0] == 0);
    let sign = a.ray.signs[axis];
    if sign == 0 || b.ray.signs[axis] != sign {
        return Err(refused("Unified tangent port directions disagree"));
    }
    arithmetic::branch_slice::compare(
        slice(owner, a, axis, sign, l)?,
        slice(owner, b, axis, sign, l)?,
        axis,
        sign,
        l.algebraic.max_refinements,
        l.algebraic.max_resultant_size,
    )
}
fn derive(
    owner: &OwnedOriginalUnifiedPorts,
    l: ContactPortOrderLimits,
) -> Result<(Vec<Vec<usize>>, usize)> {
    if l.max_ports == 0
        || l.max_ports > 4096
        || l.max_comparisons > 65536
        || l.max_neighbourhood_steps == 0
        || l.max_neighbourhood_steps > 512
        || l.algebraic.max_refinements > 512
        || !(1..=128).contains(&l.algebraic.max_resultant_size)
    {
        return Err(refused("Invalid unified original cyclic ordering limits"));
    }
    owner.reverify_source()?;
    let mut comparisons = 0;
    let mut rotations = Vec::new();
    for members in owner.vertices() {
        if members.len() > l.max_ports {
            return Err(refused("Unified original vertex port allowance exceeded"));
        }
        let mut ordered = Vec::new();
        for &candidate in members {
            let (mut lo, mut hi) = (0, ordered.len());
            while lo < hi {
                if comparisons >= l.max_comparisons {
                    return Err(refused(
                        "Unified original cyclic comparison allowance exceeded",
                    ));
                }
                comparisons += 1;
                let mid = lo + (hi - lo) / 2;
                match compare(owner, candidate, ordered[mid], l)? {
                    Ordering::Less => hi = mid,
                    Ordering::Greater => lo = mid + 1,
                    Ordering::Equal => {
                        return Err(refused(
                            "Unified original branches are not intrinsically separated",
                        ));
                    }
                }
            }
            ordered.insert(lo, candidate);
        }
        let below = ordered
            .iter()
            .take_while(|&&p| {
                owner.ports()[p].ray.signs == [1, 0] && owner.ports()[p].component_signs[1] < 0
            })
            .count();
        ordered.rotate_left(below);
        rotations.push(ordered);
    }
    Ok((rotations, comparisons))
}
impl OwnedOriginalUnifiedRotation {
    pub fn owner(&self) -> &Arc<OwnedOriginalUnifiedPorts> {
        &self.owner
    }
    /// CCW from +X, with exact beyond-leading signs at the angular cut.
    pub fn rotations(&self) -> &[Vec<usize>] {
        &self.rotations
    }
    pub fn comparisons_used(&self) -> usize {
        self.comparisons
    }
    pub fn reverify_source(&self) -> Result<()> {
        let (rotations, comparisons) = derive(&self.owner, self.limits)?;
        if rotations != self.rotations || comparisons != self.comparisons {
            return Err(refused(
                "Unified original cyclic ordering or proof work changed",
            ));
        }
        Ok(())
    }
}
pub fn own_original_unified_rotation(
    owner: Arc<OwnedOriginalUnifiedPorts>,
    limits: ContactPortOrderLimits,
) -> Result<OwnedOriginalUnifiedRotation> {
    let (rotations, comparisons) = derive(&owner, limits)?;
    Ok(OwnedOriginalUnifiedRotation {
        owner,
        limits,
        rotations,
        comparisons,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_rejects_reversed_cyclic_order_and_changed_work() {
        let a = Curve::from_polyline(vec![vec![0., 0.], vec![2., 2.]]).unwrap();
        let b = Curve::from_polyline(vec![vec![0., 2.], vec![2., 0.]]).unwrap();
        let d = Arc::new(
            own_original_common_dictionary(&[a, b], 1e-12, ParameterOrderLimits::default())
                .unwrap(),
        );
        let c = Arc::new(own_original_source_carriers(d, ParameterOrderLimits::default()).unwrap());
        let g = Arc::new(own_original_unified_graph(c).unwrap());
        let p = Arc::new(own_original_unified_ports(g, 192).unwrap());
        let x = own_original_unified_rotation(p, ContactPortOrderLimits::default()).unwrap();
        x.reverify_source().unwrap();
        let mut bad = x.clone();
        bad.rotations
            .iter_mut()
            .find(|v| v.len() == 4)
            .unwrap()
            .reverse();
        assert!(bad.reverify_source().is_err());
        let mut bad = x.clone();
        bad.comparisons += 1;
        assert!(bad.reverify_source().is_err());
    }
}
